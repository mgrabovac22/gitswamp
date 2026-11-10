use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{ipc::Channel, AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tauri::Manager;
use std::io::{Read, Write};

use crate::constants::{APP_USER_AGENT, GITHUB_ACCEPT_HEADER};

const RELEASES_API_URL: &str =
    "https://api.github.com/repos/mgrabovac22/gitswamp/releases?per_page=30";
const RELEASE_MANIFEST_NAME: &str = "latest.json";
const RELEASE_DOWNLOAD_FMT: &str =
    "https://github.com/mgrabovac22/gitswamp/releases/download/{}/latest.json";

#[derive(Debug, Clone, Deserialize)]
struct GithubAsset {
    name: String,
    size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    html_url: Option<String>,
    assets: Option<Vec<GithubAsset>>,
}

impl GithubRelease {
    fn has_manifest(&self) -> bool {
        self.assets
            .as_ref()
            .map(|assets| assets.iter().any(|asset| asset.name == RELEASE_MANIFEST_NAME))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub deb_installation: bool,
    pub download_size: Option<u64>,
    pub already_installed: bool,
    pub tag: String,
    pub version: String,
    pub current_version: String,
    pub name: String,
    pub notes: String,
    pub published_at: String,
    pub prerelease: bool,
    pub html_url: String,
    pub manifest_url: String,
}

#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum DownloadEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        content_length: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
        chunk_length: usize,
    },
    Finished,
}

#[derive(Default)]
pub struct PendingUpdate {
    native: Mutex<Option<Update>>,
    deb: Mutex<Option<std::path::PathBuf>>,
}

fn published_key(release: &GithubRelease) -> String {
    release.published_at.clone().unwrap_or_default()
}

fn fetch_releases() -> Result<Vec<GithubRelease>, String> {
    let response = ureq::get(RELEASES_API_URL)
        .set("Accept", GITHUB_ACCEPT_HEADER)
        .set("User-Agent", APP_USER_AGENT)
        .call()
        .map_err(|e| format!("Update check failed: {}", e))?;

    response
        .into_json::<Vec<GithubRelease>>()
        .map_err(|e| format!("Update release parse failed: {}", e))
}

fn pick_release(releases: &[GithubRelease]) -> Option<GithubRelease> {
    let mut candidates: Vec<GithubRelease> = releases
        .iter()
        .filter(|release| !release.draft)
        .filter(|release| release.has_manifest())
        .filter(|release| release.published_at.is_some())
        .cloned()
        .collect();

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|a, b| published_key(b).cmp(&published_key(a)));

    let newest_stable = candidates.iter().find(|release| !release.prerelease).cloned();
    let newest_prerelease = candidates.iter().find(|release| release.prerelease).cloned();

    match (newest_stable, newest_prerelease) {
        (Some(stable), Some(prerelease)) => {
            if published_key(&prerelease) > published_key(&stable) {
                Some(prerelease)
            } else {
                Some(stable)
            }
        }
        (stable, prerelease) => stable.or(prerelease),
    }
}

fn manifest_url(tag: &str) -> String {
    RELEASE_DOWNLOAD_FMT.replace("{}", tag)
}

#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<Option<UpdateCheckResult>, String> {
    let releases = tauri::async_runtime::spawn_blocking(fetch_releases)
        .await.map_err(|e| e.to_string())??;
    let Some(release) = pick_release(&releases) else {
        return Ok(None);
    };

    let endpoint = url::Url::parse(&manifest_url(&release.tag_name))
        .map_err(|e| format!("Invalid update endpoint: {}", e))?;

    let update = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e| format!("Invalid update endpoint: {}", e))?
        .build()
        .map_err(|e| format!("Update builder failed: {}", e))?
        .check()
        .await
        .map_err(|e| format!("Update check failed: {}", e))?;

    let Some(update) = update else {
        return Ok(None);
    };

    let deb_installation = is_deb_installation();
    let version = update.version.clone();
    let already_installed = deb_installation && tauri::async_runtime::spawn_blocking(move || deb_already_installed(&version))
        .await.map_err(|e| e.to_string())?;
    let result = UpdateCheckResult {
        already_installed,
        deb_installation,
        download_size: if deb_installation {
            deb_asset(&release, std::env::consts::ARCH).and_then(|asset| asset.size)
        } else { None },
        tag: release.tag_name.clone(),
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        name: release.name.clone().unwrap_or_else(|| release.tag_name.clone()),
        notes: release.body.clone().unwrap_or_default(),
        published_at: release.published_at.clone().unwrap_or_default(),
        prerelease: release.prerelease,
        html_url: release.html_url.clone().unwrap_or_default(),
        manifest_url: manifest_url(&release.tag_name),
    };

    let mut slot = pending
        .native
        .lock()
        .map_err(|e| format!("Pending update lock failed: {}", e))?;
    *slot = Some(update);

    Ok(Some(result))
}

fn is_deb_installation() -> bool {
    cfg!(target_os = "linux") && matches!(tauri::utils::platform::bundle_type(), Some(tauri::utils::config::BundleType::Deb))
}

const DEB_PACKAGE_NAME: &str = "git-swamp";

fn parse_installed_deb_version(output: &str) -> Option<String> {
    let (status, version) = output.trim_end().split_once('\t')?;
    if !status.starts_with("ii") || version.is_empty() { return None; }
    Some(version.to_string())
}

fn deb_already_installed(version: &str) -> bool {
    if !cfg!(target_os = "linux") { return false; }
    let output = std::process::Command::new("dpkg-query")
        .args(["--show", "--showformat=${db:Status-Abbrev}\t${Version}", DEB_PACKAGE_NAME]).output();
    let Some(installed) = output.ok().filter(|output| output.status.success())
        .and_then(|output| parse_installed_deb_version(&String::from_utf8_lossy(&output.stdout))) else { return false; };
    std::process::Command::new("dpkg").args(["--compare-versions", &installed, "ge", version])
        .output().map(|output| output.status.success()).unwrap_or(false)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebUpdateDownload {
    path: Option<String>,
    version: String,
    already_installed: bool,
}

fn validate_deb_metadata(output: &str, version: &str, architecture: &str) -> Result<(), String> {
    let fields: Vec<_> = output.lines().collect();
    let expected_arch = match architecture { "x86_64" => "amd64", "aarch64" => "arm64", _ => return Err("Unsupported Debian architecture.".to_string()) };
    if fields.len() != 3 || fields[0] != DEB_PACKAGE_NAME || fields[1] != version || fields[2] != expected_arch {
        return Err("The Debian package name, version or architecture does not match the offered update. Nothing was installed.".to_string());
    }
    Ok(())
}

fn deb_asset<'a>(release: &'a GithubRelease, architecture: &str) -> Option<&'a GithubAsset> {
    release.assets.as_ref()?.iter().find(|asset| {
        let name = asset.name.to_lowercase();
        let matching_arch = match architecture {
            "x86_64" => name.contains("amd64") || name.contains("x86_64"),
            "aarch64" => name.contains("aarch64") || name.contains("arm64"),
            _ => false,
        };
        name.ends_with(".deb") && matching_arch && !name.contains('/') && !name.contains('\\')
    })
}

#[tauri::command]
pub async fn download_deb_update(app: AppHandle, tag: String, version: String, on_event: Channel<DownloadEvent>) -> Result<DebUpdateDownload, String> {
    if !is_deb_installation() {
        return Err("Debian fallback is only available for a .deb installation.".to_string());
    }
    let directory = app.path().download_dir().map_err(|e| format!("Downloads folder unavailable: {}", e))?;
    tauri::async_runtime::spawn_blocking(move || {
        if tag.strip_prefix('v').unwrap_or(&tag) != version || !version.starts_with(|c: char| c.is_ascii_digit())
            || !version.chars().all(|c| c.is_ascii_alphanumeric() || ".-+".contains(c)) {
            return Err("Invalid release version.".to_string());
        }
        if deb_already_installed(&version) {
            return Ok(DebUpdateDownload { path: None, version, already_installed: true });
        }
        let release = fetch_releases()?.into_iter().find(|release| release.tag_name == tag && !release.draft && release.has_manifest())
            .ok_or("The offered release is no longer available. Check for updates again.")?;
        let asset = deb_asset(&release, std::env::consts::ARCH).ok_or("This release has no Debian package for your architecture.")?;
        let mut url = url::Url::parse("https://github.com/mgrabovac22/gitswamp/releases/download/").map_err(|e| e.to_string())?;
        url.path_segments_mut().map_err(|_| "Invalid release URL")?.pop_if_empty().push(&release.tag_name).push(&asset.name);
        let response = ureq::get(url.as_str()).set("User-Agent", APP_USER_AGENT).timeout(std::time::Duration::from_secs(120)).call().map_err(|e| format!("Debian package download failed: {}", e))?;
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
        let destination = directory.join(format!("{}-{}.deb", asset.name.trim_end_matches(".deb"), stamp));
        let partial = destination.with_extension("deb.part");
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&partial).map_err(|e| e.to_string())?;
        let download = (|| -> Result<(), String> {
            const LIMIT: u64 = 512 * 1024 * 1024;
            let content_length = response.header("Content-Length").and_then(|value| value.parse().ok()).or(asset.size);
            if content_length.is_some_and(|size| size > LIMIT) { return Err("Debian package exceeds the download limit.".to_string()); }
            let _ = on_event.send(DownloadEvent::Started { content_length });
            let mut reader = response.into_reader().take(LIMIT + 1);
            let mut magic = [0; 8];
            reader.read_exact(&mut magic).map_err(|e| e.to_string())?;
            if &magic != b"!<arch>\n" { return Err("Downloaded file is not a Debian archive.".to_string()); }
            file.write_all(&magic).map_err(|e| e.to_string())?;
            let mut buffer = [0; 64 * 1024];
            let mut total = magic.len() as u64;
            let mut pending_bytes = magic.len();
            let mut last_update = std::time::Instant::now();
            loop {
                let count = reader.read(&mut buffer).map_err(|e| e.to_string())?;
                if count == 0 { break; }
                total += count as u64;
                if total > LIMIT { return Err("Debian package exceeds the download limit.".to_string()); }
                file.write_all(&buffer[..count]).map_err(|e| e.to_string())?;
                pending_bytes += count;
                if last_update.elapsed() >= std::time::Duration::from_millis(150) {
                    let _ = on_event.send(DownloadEvent::Progress { chunk_length: pending_bytes });
                    pending_bytes = 0;
                    last_update = std::time::Instant::now();
                }
            }
            if pending_bytes > 0 { let _ = on_event.send(DownloadEvent::Progress { chunk_length: pending_bytes }); }
            file.sync_all().map_err(|e| e.to_string())?;
            Ok(())
        })();
        drop(file);
        if let Err(error) = download { let _ = std::fs::remove_file(&partial); return Err(error); }
        let validation = (|| -> Result<(), String> {
            let output = std::process::Command::new("dpkg-deb")
                .args(["--show", "--showformat=${Package}\n${Version}\n${Architecture}\n"])
                .arg(&partial).output().map_err(|e| format!("Could not inspect Debian package: {}", e))?;
            if !output.status.success() { return Err("Could not read Debian package metadata.".to_string()); }
            validate_deb_metadata(&String::from_utf8_lossy(&output.stdout), &version, std::env::consts::ARCH)
        })();
        if let Err(error) = validation { let _ = std::fs::remove_file(&partial); return Err(error); }
        if deb_already_installed(&version) {
            let _ = std::fs::remove_file(&partial);
            return Ok(DebUpdateDownload { path: None, version, already_installed: true });
        }
        if let Err(error) = std::fs::rename(&partial, &destination) {
            let _ = std::fs::remove_file(&partial);
            return Err(error.to_string());
        }
        *app.state::<PendingUpdate>().deb.lock().map_err(|_| "Could not retain the downloaded package.")? = Some(destination.clone());
        let _ = on_event.send(DownloadEvent::Finished);
        Ok(DebUpdateDownload { path: Some(destination.to_string_lossy().to_string()), version, already_installed: false })
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_deb_update(pending: State<'_, PendingUpdate>) -> Result<bool, String> {
    if !is_deb_installation() { return Err("No Debian update is available.".into()); }
    let path = pending.deb.lock().map_err(|_| "Could not access the downloaded package.")?
        .clone().ok_or("Download the Debian update first.")?;
    tauri::async_runtime::spawn_blocking(move || {
        if !path.is_file() { return Err("The downloaded package is no longer in Downloads.".into()); }
        let opened = std::process::Command::new("gio").arg("open").arg(&path)
            .output().map(|output| output.status.success());
        if matches!(opened, Ok(true)) || (opened.is_err() && tauri_plugin_opener::open_path(&path, None::<&str>).is_ok()) {
            return Ok(true);
        }
        let _ = tauri_plugin_opener::reveal_item_in_dir(&path);
        Ok(false)
    }).await.map_err(|_| "Could not open the package manager.".to_string())?
}

#[tauri::command]
pub async fn install_update(
    pending: State<'_, PendingUpdate>,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    if is_deb_installation() {
        return Err("Debian installations must be updated through the system package manager.".to_string());
    }
    let update = pending
        .native
        .lock()
        .map_err(|e| format!("Pending update lock failed: {}", e))?
        .take();

    let Some(update) = update else {
        return Err("No pending update.".to_string());
    };

    let mut started = false;
    let result = update
        .download_and_install(
            |chunk_length, content_length| {
                if !started {
                    started = true;
                    let _ = on_event.send(DownloadEvent::Started {
                        content_length,
                    });
                }
                let _ = on_event.send(DownloadEvent::Progress { chunk_length });
            },
            || {
                let _ = on_event.send(DownloadEvent::Finished);
            },
        )
        .await;

    if let Err(error) = result {
        let mut slot = pending
            .native
            .lock()
            .map_err(|e| format!("Pending update lock failed: {}", e))?;
        *slot = Some(update);
        return Err(format!("Update install failed: {}", error));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_fully_installed_debian_versions() {
        assert_eq!(parse_installed_deb_version("ii \t0.4.123\n").as_deref(), Some("0.4.123"));
        assert_eq!(parse_installed_deb_version("ii \t1:0.4.123-1").as_deref(), Some("1:0.4.123-1"));
        assert!(parse_installed_deb_version("rc \t0.4.123").is_none());
        assert!(parse_installed_deb_version("iU \t0.4.123").is_none());
        assert!(parse_installed_deb_version("invalid").is_none());
    }

    #[test]
    fn deb_metadata_must_match_the_offered_version_and_package() {
        assert!(validate_deb_metadata("git-swamp\n0.4.123\namd64\n", "0.4.123", "x86_64").is_ok());
        assert!(validate_deb_metadata("git-swamp\n0.4.123\narm64\n", "0.4.123", "aarch64").is_ok());
        for metadata in ["git-swamp\n0.4.0\namd64\n", "another-app\n0.4.123\namd64\n", "git-swamp\n0.4.123\narm64\n", "invalid"] {
            assert!(validate_deb_metadata(metadata, "0.4.123", "x86_64").is_err());
        }
    }

    fn release(tag: &str, published: &str, prerelease: bool, manifest: bool) -> GithubRelease {
        GithubRelease {
            tag_name: tag.to_string(),
            name: Some(tag.to_string()),
            body: None,
            draft: false,
            prerelease,
            published_at: Some(published.to_string()),
            html_url: Some(format!("https://example.com/{}", tag)),
            assets: Some(if manifest {
                vec![GithubAsset {
                    name: RELEASE_MANIFEST_NAME.to_string(),
                    size: None,
                }]
            } else {
                vec![GithubAsset {
                    name: "GitSwamp.deb".to_string(),
                    size: None,
                }]
            }),
        }
    }

    #[test]
    fn deb_fallback_selects_only_the_matching_architecture() {
        let mut selected = release("v1.0.0", "2026-10-29T00:00:00Z", false, true);
        selected.assets = Some(vec![
            GithubAsset { name: "GitSwamp_aarch64.deb".into(), size: Some(9_000_000) },
            GithubAsset { name: "GitSwamp_amd64.deb".into(), size: Some(10_000_000) },
            GithubAsset { name: "GitSwamp_amd64.AppImage".into(), size: Some(90_000_000) },
        ]);
        assert_eq!(deb_asset(&selected, "x86_64").map(|asset| asset.name.as_str()), Some("GitSwamp_amd64.deb"));
        assert_eq!(deb_asset(&selected, "x86_64").and_then(|asset| asset.size), Some(10_000_000));
        assert_eq!(deb_asset(&selected, "aarch64").and_then(|asset| asset.size), Some(9_000_000));
        assert!(deb_asset(&selected, "unsupported").is_none());
        selected.assets = Some(vec![GithubAsset { name: "../GitSwamp_amd64.deb".into(), size: None }]);
        assert!(deb_asset(&selected, "x86_64").is_none());
    }

    #[test]
    fn picks_newest_stable() {
        let releases = vec![
            release("v0.4.0", "2026-10-10T00:00:00Z", false, true),
            release("v0.3.0", "2026-09-01T00:00:00Z", false, true),
        ];
        assert_eq!(pick_release(&releases).unwrap().tag_name, "v0.4.0");
    }

    #[test]
    fn prefers_prerelease_only_when_newer() {
        let releases = vec![
            release("v0.4.0", "2026-10-10T00:00:00Z", false, true),
            release("v0.5.0-beta.1", "2026-10-12T00:00:00Z", true, true),
        ];
        assert_eq!(pick_release(&releases).unwrap().tag_name, "v0.5.0-beta.1");
    }

    #[test]
    fn ignores_older_prerelease() {
        let releases = vec![
            release("v0.4.0", "2026-10-10T00:00:00Z", false, true),
            release("v0.5.0-beta.1", "2026-10-01T00:00:00Z", true, true),
        ];
        assert_eq!(pick_release(&releases).unwrap().tag_name, "v0.4.0");
    }

    #[test]
    fn skips_releases_without_manifest() {
        let releases = vec![
            release("v0.4.0", "2026-10-10T00:00:00Z", false, false),
            release("v0.3.0", "2026-09-01T00:00:00Z", false, true),
        ];
        assert_eq!(pick_release(&releases).unwrap().tag_name, "v0.3.0");
    }

    #[test]
    fn returns_none_when_nothing_usable() {
        let releases = vec![release("v0.4.0", "2026-10-10T00:00:00Z", false, false)];
        assert!(pick_release(&releases).is_none());
    }

    #[test]
    fn builds_manifest_url_from_tag() {
        assert_eq!(
            manifest_url("v0.4.0"),
            "https://github.com/mgrabovac22/gitswamp/releases/download/v0.4.0/latest.json"
        );
    }
}

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{ipc::Channel, AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::constants::{APP_USER_AGENT, GITHUB_ACCEPT_HEADER};

const RELEASES_API_URL: &str =
    "https://api.github.com/repos/mgrabovac22/gitswamp/releases?per_page=30";
const RELEASE_MANIFEST_NAME: &str = "latest.json";
const RELEASE_DOWNLOAD_FMT: &str =
    "https://github.com/mgrabovac22/gitswamp/releases/download/{}/latest.json";

#[derive(Debug, Clone, Deserialize)]
struct GithubAsset {
    name: String,
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

pub struct PendingUpdate(pub Mutex<Option<Update>>);

impl Default for PendingUpdate {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
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

/// Picks the release that should be offered as an update.
///
/// The newest non-draft release that ships a `latest.json` manifest wins.
/// A prerelease is only preferred when it is chronologically newer than the
/// newest stable release.
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
    let releases = fetch_releases()?;
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

    let result = UpdateCheckResult {
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
        .0
        .lock()
        .map_err(|e| format!("Pending update lock failed: {}", e))?;
    *slot = Some(update);

    Ok(Some(result))
}

#[tauri::command]
pub async fn install_update(
    pending: State<'_, PendingUpdate>,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    let update = pending
        .0
        .lock()
        .map_err(|e| format!("Pending update lock failed: {}", e))?
        .take();

    let Some(update) = update else {
        return Err("No pending update.".to_string());
    };

    let mut started = false;
    update
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
        .await
        .map_err(|e| format!("Update install failed: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
                }]
            } else {
                vec![GithubAsset {
                    name: "GitSwamp.deb".to_string(),
                }]
            }),
        }
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


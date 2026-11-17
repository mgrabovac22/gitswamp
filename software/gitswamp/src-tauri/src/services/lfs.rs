use std::{ffi::OsStr, fs, io::Read, path::Path, process::Stdio, sync::Mutex};

use git2::{AttrCheckFlags, Index, Repository, RepositoryState};
use serde::{Deserialize, Serialize};

use super::secret_service::{safe_file, safe_relative, OwnedLock, SecretService};
use crate::repositories::git_repository::GitRepository as Git;

const ATTR: &str = ".gitattributes";
const METADATA_LIMIT: u64 = 4 * 1024 * 1024;
const POINTER_HEADER: &[u8] = b"version https://git-lfs.github.com/spec/v1\n";
const MAX_BATCH: usize = 100;
static EDIT: Mutex<()> = Mutex::new(());

#[derive(Deserialize, Serialize)]
struct Rule {
    pattern: String,
    source: String,
    tracked: bool,
    lockable: bool,
}

#[derive(Deserialize, Serialize)]
struct Asset {
    name: String,
    size: u64,
    downloaded: bool,
    checkout: bool,
}

#[derive(Serialize)]
pub struct Overview {
    version: Option<String>,
    configured: bool,
    rules: Vec<Rule>,
    files: Vec<Asset>,
    warnings: Vec<String>,
    large_files: Vec<super::secret_service::LargeStagedFile>,
    snapshot: Option<String>,
    remotes: Vec<String>,
}

#[derive(Deserialize)]
struct Rules {
    patterns: Vec<Rule>,
}
#[derive(Deserialize)]
struct Files {
    files: Option<Vec<Asset>>,
}

fn root(repo: &Repository) -> Result<&Path, String> {
    repo.workdir()
        .ok_or_else(|| "Open a working copy to manage Git LFS.".into())
}

fn version(path: &str) -> Option<String> {
    Git::git_cli_with_env(path, &["lfs", "version"], &[])
        .ok()
        .filter(|text| text.starts_with("git-lfs/"))
}

fn metadata<T: serde::de::DeserializeOwned>(path: &str, command: &[&str]) -> Result<T, String> {
    let mut child = Git::git_command(
        path,
        command,
        &[("GIT_LFS_TRACK_NO_INSTALL_HOOKS", OsStr::new("1"))],
    )
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .unwrap()
        .take(METADATA_LIMIT + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() as u64 > METADATA_LIMIT {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Git LFS metadata exceeds the display limit.".into());
    }
    if !child.wait().map_err(|e| e.to_string())?.success() {
        return Err("Could not read Git LFS metadata. Check that your installed Git LFS supports JSON output.".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Git LFS returned unreadable metadata.".into())
}

pub fn overview(path: &str) -> Result<Overview, String> {
    let repo = Git::open(path)?;
    let path = root(&repo)?.to_str().ok_or("Invalid working copy path.")?;
    let version = version(path);
    let configured = repo.config().is_ok_and(|config| {
        config.get_string("filter.lfs.process").is_ok()
            || config.get_string("filter.lfs.clean").is_ok()
    });
    let mut warnings = Vec::new();
    let (mut rules, mut files) = (Vec::new(), Vec::new());
    if version.is_some() {
        match metadata::<Rules>(path, &["lfs", "track", "--json"]) {
            Ok(value) => rules = value.patterns,
            Err(error) => warnings.push(error),
        }
        match metadata::<Files>(path, &["lfs", "ls-files", "--json"]) {
            Ok(value) => files = value.files.unwrap_or_default(),
            Err(error) => warnings.push(error),
        }
    }
    let (large_files, snapshot) = match SecretService::review_staged(path, false) {
        Ok(report) => {
            warnings.extend(report.skipped);
            (report.large_files, Some(report.snapshot))
        }
        Err(error) => {
            warnings.push(error);
            (Vec::new(), None)
        }
    };
    let remotes = repo
        .remotes()
        .map_err(|e| e.to_string())?
        .iter()
        .flatten()
        .map(str::to_owned)
        .collect();
    Ok(Overview {
        version,
        configured,
        rules,
        files,
        warnings,
        large_files,
        snapshot,
        remotes,
    })
}

fn ready(repo: &Repository) -> Result<(), String> {
    if repo.state() != RepositoryState::Clean
        || repo.index().map_err(|e| e.to_string())?.has_conflicts()
    {
        return Err("Finish the active Git operation before changing Git LFS tracking.".into());
    }
    Ok(())
}

fn install_local(path: &str) -> Result<(), String> {
    Git::git_cli_with_env(path, &["lfs", "install", "--local"], &[]).map(|_| ())
}

pub fn setup(path: &str) -> Result<(), String> {
    let repo = Git::open(path)?;
    ready(&repo)?;
    install_local(root(&repo)?.to_str().ok_or("Invalid working copy path.")?)
}

pub fn rule(path: &str, pattern: &str, remove: bool) -> Result<(), String> {
    let _guard = EDIT.lock().map_err(|e| e.to_string())?;
    if !safe_relative(pattern)
        || pattern.trim() != pattern
        || pattern.starts_with(['!', '#', '"'])
        || pattern
            .split('/')
            .any(|part| part.eq_ignore_ascii_case(ATTR))
    {
        return Err("Enter a valid repository-relative Git LFS pattern, such as *.psd.".into());
    }
    let repo = Git::open(path)?;
    ready(&repo)?;
    let dir = root(&repo)?;
    safe_file(dir, ATTR).map_err(|_| "The repository's .gitattributes must be a regular file.")?;
    let path = dir.to_str().ok_or("Invalid working copy path.")?;
    let command = if remove { "untrack" } else { "track" };
    if !remove {
        install_local(path)?;
    }
    Git::git_cli_with_env(path, &["lfs", command, "--", pattern], &[]).map(|_| ())
}

fn index_entry(repo: &Repository, file: &str) -> Result<git2::IndexEntry, String> {
    repo.index()
        .map_err(|e| e.to_string())?
        .get_path(Path::new(file), 0)
        .filter(|entry| matches!(entry.mode, 0o100644 | 0o100755))
        .ok_or_else(|| "The selected staged file no longer exists or is not a regular file.".into())
}

fn clean_blob(repo: &Repository, path: &str, file: &str) -> Result<git2::Oid, String> {
    let entry = index_entry(repo, file)?;
    let mut source = Git::git_command(path, &["cat-file", "blob", &entry.id.to_string()], &[])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    // Without a path, git-lfs reads the whole stream instead of sizing it from the
    // working-copy file, so the pointer always describes the exact staged blob.
    let pointer = Git::git_command(path, &["lfs", "clean"], &[])
        .stdin(source.stdout.take().unwrap())
        .output();
    let source_ok = source.wait().map_err(|e| e.to_string())?.success();
    let pointer = pointer.map_err(|e| e.to_string())?;
    if !source_ok
        || !pointer.status.success()
        || !pointer.stdout.starts_with(POINTER_HEADER)
        || pointer.stdout.len() > 1024
    {
        return Err(format!(
            "Git LFS could not convert the staged contents of {file}. No staged file was replaced."
        ));
    }
    repo.blob(&pointer.stdout).map_err(|e| e.to_string())
}

pub fn convert(path: &str, files: &[String], snapshot: &str) -> Result<(), String> {
    let _guard = EDIT.lock().map_err(|e| e.to_string())?;
    if files.is_empty() || files.len() > MAX_BATCH {
        return Err("Select between 1 and 100 files.".into());
    }
    let repo = Git::open(path)?;
    ready(&repo)?;
    let dir = root(&repo)?;
    let path = dir.to_str().ok_or("Invalid working copy path.")?;
    let report = SecretService::review_staged(path, false)?;
    if report.snapshot != snapshot {
        return Err("Staged files changed. Refresh Git LFS Manager and try again.".into());
    }
    let mut unique = std::collections::HashSet::new();
    for file in files {
        if !unique.insert(file)
            || !safe_relative(file)
            || file.split('/').any(|part| part.eq_ignore_ascii_case(ATTR))
            || !report.large_files.iter().any(|item| &item.path == file)
            || safe_file(dir, file).ok().flatten().is_none()
        {
            return Err(
                "Select regular oversized staged files that still exist in the working copy."
                    .into(),
            );
        }
    }
    safe_file(dir, ATTR).map_err(|_| "The repository's .gitattributes must be a regular file.")?;
    if repo.status_file(Path::new(ATTR)).is_ok_and(|status| {
        status.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_NEW
                | git2::Status::WT_DELETED
                | git2::Status::WT_TYPECHANGE,
        )
    }) {
        return Err("Stage your existing .gitattributes changes first so they are not staged by this conversion.".into());
    }
    install_local(path)?;

    // Preserve the original index until every pointer and the tracking file are ready.
    let index = repo.index().map_err(|e| e.to_string())?;
    let index_path = index
        .path()
        .ok_or("Could not locate the repository index.")?;
    let index_bytes = fs::read(index_path).map_err(|e| e.to_string())?;
    let lock_path = index_path.with_file_name("index.lock");
    let mut lock = OwnedLock::create(lock_path.clone(), &index_bytes)
        .map_err(|_| "Could not lock the repository index. Refresh and try again.")?;
    if SecretService::review_staged(path, false)?.snapshot != snapshot {
        return Err("Staged files changed. Refresh Git LFS Manager and try again.".into());
    }
    let mut updated = Index::open(&lock_path).map_err(|e| e.to_string())?;
    for file in files {
        let mut entry = index_entry(&repo, file)?;
        entry.id = clean_blob(&repo, path, file)?;
        entry.file_size = 0;
        updated.add(&entry).map_err(|e| e.to_string())?;
    }
    let attr_path = dir.join(ATTR);
    if fs::metadata(&attr_path).is_ok_and(|info| info.len() > METADATA_LIMIT) {
        return Err("The repository's .gitattributes exceeds the edit limit.".into());
    }
    for file in files {
        Git::git_cli_with_env(
            path,
            &["lfs", "track", "--filename", "--", file],
            &[("GIT_LFS_TRACK_NO_INSTALL_HOOKS", OsStr::new("1"))],
        )?;
    }
    let attrs = fs::read(&attr_path).map_err(|e| e.to_string())?;
    if attrs.len() as u64 > METADATA_LIMIT {
        return Err("The resulting .gitattributes exceeds the edit limit.".into());
    }
    let mut attr_entry = index
        .get_path(Path::new(ATTR), 0)
        .unwrap_or(git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: git2::Oid::zero(),
            flags: 0,
            flags_extended: 0,
            path: ATTR.as_bytes().to_vec(),
        });
    attr_entry.id = repo.blob(&attrs).map_err(|e| e.to_string())?;
    attr_entry.file_size = 0;
    updated.add(&attr_entry).map_err(|e| e.to_string())?;
    for file in files {
        let value = Git::git_output(path, &["check-attr", "-z", "filter", "--", file], &[])?.stdout;
        if value.split(|byte| *byte == 0).nth(2) != Some(b"lfs".as_slice()) {
            return Err(format!("Another .gitattributes rule overrides LFS for {file}. The original index was kept."));
        }
    }
    updated.write().map_err(|e| e.to_string())?;
    lock.install(index_path)
        .map_err(|_| "Could not replace the repository index. Refresh and try again.".into())
}

pub fn stage_if_lfs(repo: &Repository, files: &[String]) -> Result<bool, String> {
    let mut lfs = false;
    for file in files {
        if !safe_relative(file) {
            continue;
        }
        if repo
            .get_attr(Path::new(file), "filter", AttrCheckFlags::FILE_THEN_INDEX)
            .map_err(|e| e.to_string())?
            == Some("lfs")
        {
            lfs = true;
            break;
        }
    }
    if !lfs {
        return Ok(false);
    }
    let path = root(repo)?.to_str().ok_or("Invalid working copy path.")?;
    if version(path).is_none() {
        return Err("Install Git LFS before staging LFS files.".into());
    }
    // Pass exact paths to Git's streaming clean filter, including paths with glob characters.
    if files.iter().any(|file| !safe_relative(file)) {
        return Err("A selected path cannot be staged safely with Git LFS.".into());
    }
    let mut args = vec![
        "--literal-pathspecs",
        "-c",
        "filter.lfs.process=git-lfs filter-process",
        "-c",
        "filter.lfs.required=true",
        "add",
        "-A",
        "--",
    ];
    for batch in files.chunks(16) {
        let len = args.len();
        args.extend(batch.iter().map(String::as_str));
        Git::git_cli_with_env(path, &args, &[])?;
        args.truncate(len);
    }
    Ok(true)
}

pub fn configured_repository(repo: &Repository) -> bool {
    let git_dir = repo.path();
    if git_dir.join("lfs/objects").is_dir() {
        return true;
    }
    if repo.is_worktree() {
        if let Ok(common) = fs::read_to_string(git_dir.join("commondir")) {
            if git_dir.join(common.trim()).join("lfs/objects").is_dir() {
                return true;
            }
        }
    }
    repo.config()
        .is_ok_and(|config| config.get_string("lfs.storage").is_ok())
}

pub fn status(path: &str) -> Result<Vec<crate::models::FileStatusInfo>, String> {
    use crate::models::FileStatusInfo;
    let bytes = Git::git_output(path, &["status", "--porcelain=v1", "-z", "-uall"], &[])?.stdout;
    let mut files = Vec::new();
    let mut records = bytes.split(|byte| *byte == 0);
    while let Some(record) = records.next() {
        if record.len() < 4 || record[2] != b' ' {
            continue;
        }
        let [x, y] = [record[0], record[1]];
        if matches!(x, b'R' | b'C') || matches!(y, b'R' | b'C') {
            records.next();
        }
        if x == b'!' {
            continue;
        }
        let Ok(file) = std::str::from_utf8(&record[3..]) else {
            continue;
        };
        let size = fs::symlink_metadata(Path::new(path).join(file))
            .ok()
            .filter(|meta| meta.is_file())
            .map(|meta| meta.len());
        let conflicted = matches!(
            (x, y),
            (b'D', b'D')
                | (b'A', b'U')
                | (b'U', b'D')
                | (b'U', b'A')
                | (b'D', b'U')
                | (b'A', b'A')
                | (b'U', b'U')
        );
        let mut add = |status: &str, staged| {
            files.push(FileStatusInfo {
                path: file.into(),
                status: status.into(),
                staged,
                conflicted,
                file_size_bytes: size,
            })
        };
        let label = |code| match code {
            b'A' | b'?' => "new",
            b'D' => "deleted",
            b'R' => "renamed",
            b'T' => "typechange",
            _ => "modified",
        };
        if conflicted {
            add("conflicted", false);
        } else {
            if x != b' ' && x != b'?' {
                add(label(x), true);
            }
            if y != b' ' {
                add(label(y), false);
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        git_service::GitService,
        secret_service::{validate_commit, GITHUB_MAX_FILE_BYTES},
    };
    use std::io::Write;

    fn git(path: &str, args: &[&str]) -> String {
        Git::git_cli_with_env(path, args, &[]).unwrap()
    }

    #[test]
    fn converts_exact_staged_bytes_and_pushes_lfs_objects_before_refs() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        let work = tempfile::tempdir_in(&base).unwrap();
        let bare = tempfile::tempdir_in(&base).unwrap();
        let path = work.path().to_str().unwrap();
        let remote = bare.path().to_str().unwrap();
        if version(path).is_none() {
            eprintln!("Git LFS is unavailable; skipping integration test");
            return;
        }
        git(path, &["init", "-b", "main"]);
        git(remote, &["init", "--bare"]);
        git(path, &["config", "user.name", "GitSwamp Tests"]);
        git(path, &["config", "user.email", "test@example.invalid"]);
        git(path, &["config", "commit.gpgsign", "false"]);
        let name = "recording [1].bin";
        let large = work.path().join(name);
        let mut file = fs::File::create(&large).unwrap();
        file.write_all(b"staged content").unwrap();
        file.set_len(GITHUB_MAX_FILE_BYTES + 1).unwrap();
        drop(file);
        fs::write(work.path().join("unrelated.txt"), "keep staged").unwrap();
        git(
            path,
            &["--literal-pathspecs", "add", "--", name, "unrelated.txt"],
        );
        let prior = git(path, &["rev-parse", ":unrelated.txt"]);
        let report = overview(path).unwrap();
        assert_eq!(report.large_files.len(), 1);
        let snapshot = report.snapshot.unwrap();
        assert!(validate_commit(path, false, Some(&snapshot)).is_err());
        fs::write(&large, "new unstaged content").unwrap();
        assert!(convert(path, &[name.into()], "stale").is_err());
        convert(path, &[name.into()], &snapshot).unwrap();
        assert_eq!(fs::read_to_string(&large).unwrap(), "new unstaged content");
        assert_eq!(git(path, &["rev-parse", ":unrelated.txt"]), prior);
        let pointer = git(path, &["show", &format!(":{name}")]);
        assert!(pointer.ends_with(&format!("size {}", GITHUB_MAX_FILE_BYTES + 1)));
        let oid = pointer
            .lines()
            .nth(1)
            .unwrap()
            .strip_prefix("oid sha256:")
            .unwrap();
        assert_eq!(
            fs::metadata(
                work.path()
                    .join(".git/lfs/objects")
                    .join(&oid[..2])
                    .join(&oid[2..4])
                    .join(oid)
            )
            .unwrap()
            .len(),
            GITHUB_MAX_FILE_BYTES + 1
        );
        assert!(SecretService::review_staged(path, false)
            .unwrap()
            .large_files
            .is_empty());
        assert!(GitService::status(path)
            .unwrap()
            .iter()
            .any(|item| item.path == name && !item.staged));
        git(path, &["commit", "-m", "Add large asset"]);
        assert_eq!(
            overview(path).unwrap().files[0].size,
            GITHUB_MAX_FILE_BYTES + 1
        );
        git(path, &["remote", "add", "origin", remote]);
        GitService::push(path, None).unwrap();
        assert_eq!(
            git(remote, &["rev-parse", "refs/heads/main"]),
            git(path, &["rev-parse", "HEAD"])
        );
        assert!(bare
            .path()
            .join("lfs/objects")
            .join(&oid[..2])
            .join(&oid[2..4])
            .join(oid)
            .is_file());

        GitService::stage_file(path, name).unwrap();
        let changed_pointer = git(path, &["show", &format!(":{name}")]);
        assert!(changed_pointer.ends_with("size 20"), "{changed_pointer}");
        rule(path, "*.psd", false).unwrap();
        assert!(overview(path)
            .unwrap()
            .rules
            .iter()
            .any(|rule| rule.pattern == "*.psd"));
        rule(path, "*.psd", true).unwrap();
        assert!(!overview(path)
            .unwrap()
            .rules
            .iter()
            .any(|rule| rule.pattern == "*.psd"));
    }
}

pub fn upload_refs(
    repo: &Repository,
    remote: &str,
    url: &str,
    token: Option<(&str, &str)>,
    refs: &[&str],
) -> Result<(), String> {
    if !configured_repository(repo) {
        return Ok(());
    }
    let path = root(repo)?.to_str().ok_or("Invalid working copy path.")?;
    let mut args = vec!["lfs", "push", "--", remote];
    args.extend(refs.iter().copied());
    let mut config = Vec::<(String, String)>::new();
    if repo.find_remote(remote).is_err() {
        config.push((format!("remote.{remote}.url"), url.into()));
    }
    if let (Some((username, secret)), Ok(url)) = (token, url::Url::parse(url)) {
        if url.scheme() == "https" && url.username().is_empty() && url.password().is_none() {
            let key = format!(
                "http.{}{}.extraHeader",
                &url[..url::Position::BeforePath],
                url.path()
            );
            config.push((
                key,
                super::integration_service::IntegrationService::basic_auth_header(username, secret),
            ));
        }
    }
    let mut names = vec![("GIT_CONFIG_COUNT".to_owned(), config.len().to_string())];
    for (i, (key, value)) in config.into_iter().enumerate() {
        names.push((format!("GIT_CONFIG_KEY_{i}"), key));
        names.push((format!("GIT_CONFIG_VALUE_{i}"), value));
    }
    let env: Vec<_> = names
        .iter()
        .map(|(key, value)| (key.as_str(), OsStr::new(value)))
        .collect();
    Git::git_cli_with_env(path, &args, &env).map(|_| ()).map_err(|_| "Git LFS objects could not be uploaded. No Git refs were pushed. Check LFS support and credentials in Git LFS Manager.".into())
}

pub fn transfer(path: &str, remote: &str, upload: bool) -> Result<(), String> {
    let repo = Git::open(path)?;
    ready(&repo)?;
    if remote.starts_with('-') || repo.find_remote(remote).is_err() {
        return Err("Select a configured remote.".into());
    }
    let path = root(&repo)?.to_str().ok_or("Invalid working copy path.")?;
    let args = if upload {
        vec!["lfs", "push", "--", remote, "HEAD"]
    } else {
        vec!["lfs", "pull", "--", remote]
    };
    Git::git_cli_with_env(path, &args, &[])
        .map(|_| ())
        .map_err(|_| {
            "Git LFS transfer failed. Check the remote's LFS support, quota and Git credentials."
                .into()
        })
}

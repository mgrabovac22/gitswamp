//! Local, metadata-only secret review. No content, matches, or library errors leave this module.
use std::collections::HashSet;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant, UNIX_EPOCH};

use git2::{ErrorCode, Index, IndexEntry, ObjectType, Oid, Repository};
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};

const CHANGED: &str = "Staged files changed. Review and commit again.";
const FAILED: &str = "Secret review could not inspect the repository. Review and try again.";
const CONFLICTS: &str = "Resolve index conflicts before reviewing or committing staged files.";
const BLOCKED: &str = "Staged files contain possible secrets or could not be fully scanned. Review before committing.";
const IGNORE_FAILED: &str =
    "Could not safely ignore these files. Refresh and review the index and .gitignore.";
const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ENTRIES: usize = 20_000;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Serialize)]
pub struct SecretFinding {
    pub path: String,
    pub line: usize,
    pub kind: String,
    /// Present in HEAD, not merely present in the index.
    pub tracked: bool,
}

#[derive(Serialize)]
pub struct SecretScan {
    pub findings: Vec<SecretFinding>,
    pub skipped: Vec<String>,
    pub snapshot: String,
    pub large_files: Vec<LargeStagedFile>,
}

#[derive(Serialize)]
pub struct LargeStagedFile {
    pub path: String,
    pub size: u64,
}

const GITHUB_MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

pub struct SecretService;

#[derive(Clone, Copy)]
struct Limits {
    file_bytes: usize,
    total_bytes: usize,
    files: usize,
    entries: usize,
    findings: usize,
    duration: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            file_bytes: 1024 * 1024,
            total_bytes: 16 * 1024 * 1024,
            files: 2048,
            entries: MAX_ENTRIES,
            findings: 1000,
            duration: Duration::from_secs(5),
        }
    }
}

struct Budget {
    limits: Limits,
    start: Instant,
    bytes: usize,
    files: usize,
    entries: usize,
}

impl Budget {
    fn new(limits: Limits) -> Self {
        Self {
            limits,
            start: Instant::now(),
            bytes: 0,
            files: 0,
            entries: 0,
        }
    }

    // The deadline is cooperative between bounded filesystem/libgit2 operations.
    fn check(&self) -> Result<(), &'static str> {
        if self.start.elapsed() >= self.limits.duration {
            Err("Scan time limit reached; remaining files were not scanned.")
        } else if self.entries >= self.limits.entries {
            Err("Scan entry limit reached; remaining files were not scanned.")
        } else if self.files >= self.limits.files {
            Err("Scan file limit reached; remaining files were not scanned.")
        } else if self.bytes >= self.limits.total_bytes {
            Err("Scan byte limit reached; remaining files were not scanned.")
        } else {
            Ok(())
        }
    }

    fn reserve(&mut self, size: usize) -> Result<(), &'static str> {
        self.check()?;
        self.files += 1;
        if size > self.limits.file_bytes {
            return Err("File exceeds the secret scan size limit.");
        }
        if size > self.limits.total_bytes.saturating_sub(self.bytes) {
            self.bytes = self.limits.total_bytes;
            return Err("Scan byte limit reached; file was not scanned.");
        }
        self.bytes += size;
        Ok(())
    }
}

struct State {
    index: Index,
    index_path: PathBuf,
    head_tree: Option<Oid>,
    snapshot: String,
}

fn object_size(repo: &Repository, id: Oid, kind: ObjectType, max: u64) -> Result<usize, String> {
    let (size, actual) = repo
        .odb()
        .and_then(|odb| odb.read_header(id))
        .map_err(|_| FAILED)?;
    if actual != kind || size as u64 > max {
        return Err(FAILED.into());
    }
    Ok(size)
}

fn regular_metadata(path: &Path) -> Result<Option<Metadata>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !is_link(&meta) => Ok(Some(meta)),
        Ok(_) => Err(IGNORE_FAILED.into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(IGNORE_FAILED.into()),
    }
}

fn is_link(meta: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0 // Includes junctions and other reparse points.
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

impl State {
    fn read(repo: &Repository) -> Result<Self, String> {
        let expected_index = repo.path().join("index");
        if regular_metadata(&expected_index)?.is_some_and(|m| m.len() > MAX_METADATA_BYTES) {
            return Err("Repository index exceeds secret review limits.".into());
        }
        let mut index = repo.index().map_err(|_| FAILED)?;
        index.read(true).map_err(|_| FAILED)?;
        if index.has_conflicts() {
            return Err(CONFLICTS.into());
        }
        let index_path = index.path().ok_or(FAILED)?.to_path_buf();
        if index_path != expected_index {
            return Err(FAILED.into());
        }
        let tree = index.write_tree().map_err(|_| FAILED)?;
        let head_ref = repo.find_reference("HEAD").map_err(|_| FAILED)?;
        let head_name = head_ref.symbolic_target_bytes().unwrap_or(b"HEAD");
        let head = match repo.head() {
            Ok(reference) => Some(reference.target().ok_or(FAILED)?),
            Err(error) if error.code() == ErrorCode::UnbornBranch => None,
            Err(_) => return Err(FAILED.into()),
        };
        let head_tree = match head {
            Some(id) => {
                object_size(repo, id, ObjectType::Commit, MAX_METADATA_BYTES)?;
                Some(repo.find_commit(id).map_err(|_| FAILED)?.tree_id())
            }
            None => None,
        };
        let root = fs::canonicalize(repo.workdir().ok_or(FAILED)?).map_err(|_| FAILED)?;
        let git_dir = fs::canonicalize(repo.path()).map_err(|_| FAILED)?;
        let mut hash = Sha256::new();
        for part in [
            b"gitswamp-secret-review-v1".as_slice(),
            root.as_os_str().as_encoded_bytes(),
            git_dir.as_os_str().as_encoded_bytes(),
            head_name,
            head.as_ref().map(Oid::as_bytes).unwrap_or(b"unborn"),
            tree.as_bytes(),
        ] {
            hash.update((part.len() as u64).to_le_bytes());
            hash.update(part);
        }
        Ok(Self {
            index,
            index_path,
            head_tree,
            snapshot: format!("{:x}", hash.finalize()),
        })
    }

    fn head_entry(&self, repo: &Repository, path: &str) -> Result<Option<(Oid, u32)>, String> {
        let Some(mut tree_id) = self.head_tree else {
            return Ok(None);
        };
        let mut parts = path.split('/').peekable();
        while let Some(part) = parts.next() {
            object_size(repo, tree_id, ObjectType::Tree, MAX_METADATA_BYTES)?;
            let tree = repo.find_tree(tree_id).map_err(|_| FAILED)?;
            let Some(entry) = tree.get_name(part) else {
                return Ok(None);
            };
            if parts.peek().is_none() {
                return Ok(Some((entry.id(), entry.filemode() as u32)));
            }
            if entry.kind() != Some(ObjectType::Tree) {
                return Ok(None);
            }
            tree_id = entry.id();
        }
        Ok(None)
    }
}

fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_PATH_BYTES
        && !path
            .chars()
            .any(|c| c.is_control() || matches!(c, '\\' | ':'))
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != ".." && !p.eq_ignore_ascii_case(".git"))
        && path.split('/').count() <= 64
}

fn safe_file(root: &Path, relative: &str) -> Result<Option<PathBuf>, String> {
    if !safe_relative(relative) {
        return Err(IGNORE_FAILED.into());
    }
    let mut path = root.to_path_buf();
    let mut parts = relative.split('/').peekable();
    while let Some(part) = parts.next() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(meta)
                if !is_link(&meta)
                    && (if parts.peek().is_some() {
                        meta.is_dir()
                    } else {
                        meta.is_file()
                    }) => {}
            Ok(_) => return Err(IGNORE_FAILED.into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(IGNORE_FAILED.into()),
        }
    }
    Ok(Some(path))
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|_| FAILED)?;
    let metadata = file.metadata().map_err(|_| FAILED)?;
    if !metadata.is_file() || is_link(&metadata) || metadata.len() > limit as u64 {
        return Err(FAILED.into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| FAILED)?;
    if bytes.len() > limit {
        return Err(FAILED.into());
    }
    Ok(bytes)
}

impl SecretService {
    pub fn review_staged(path: &str, include_secrets: bool) -> Result<SecretScan, String> {
        let repo = Repository::open(path).map_err(|_| FAILED)?;
        let state = State::read(&repo)?;
        let mut large_files = Vec::new();
        let started = Instant::now();
        let mut incomplete = false;
        for entry in state.index.iter() {
            if started.elapsed() > Duration::from_secs(5) {
                incomplete = true;
                break;
            }
            if entry.mode == 0o160000 {
                continue;
            }
            let relative = std::str::from_utf8(&entry.path).map_err(|_| FAILED)?;
            if state.head_entry(&repo, relative)? == Some((entry.id, entry.mode)) {
                continue;
            }
            let (size, kind) = repo
                .odb()
                .and_then(|odb| odb.read_header(entry.id))
                .map_err(|_| FAILED)?;
            if kind == ObjectType::Blob && size as u64 > GITHUB_MAX_FILE_BYTES {
                large_files.push(LargeStagedFile {
                    path: relative.into(),
                    size: size as u64,
                });
            }
        }
        let mut result = if include_secrets {
            Self::scan(path, true)?
        } else {
            SecretScan {
                findings: Vec::new(),
                skipped: Vec::new(),
                snapshot: state.snapshot.clone(),
                large_files: Vec::new(),
            }
        };
        if result.snapshot != state.snapshot || State::read(&repo)?.snapshot != state.snapshot {
            return Err(CHANGED.into());
        }
        result.large_files = large_files;
        if incomplete {
            result.skipped.push(
                "Large-file review time limit reached; remaining staged sizes were not checked."
                    .into(),
            );
        }
        Ok(result)
    }

    pub fn scan_worktree_paths(path: &str, paths: &[String]) -> Result<SecretScan, String> {
        let repo = Repository::open(path).map_err(|_| FAILED)?;
        let state = State::read(&repo)?;
        let root = repo.workdir().ok_or(FAILED)?;
        let mut scan = SecretScan {
            findings: vec![],
            skipped: vec![],
            snapshot: state.snapshot.clone(),
            large_files: vec![],
        };
        let mut budget = Budget::new(Limits::default());
        for relative in paths.iter().take(MAX_ENTRIES) {
            if !continue_scan(&mut scan, &budget) {
                break;
            }
            budget.entries += 1;
            if !safe_relative(relative) {
                skip(&mut scan, relative, "Unsupported path.");
                continue;
            }
            let entry = state.index.get_path(Path::new(relative), 0);
            if entry.as_ref().is_some_and(|e| e.mode == 0o160000) {
                continue;
            }
            let tracked = state.head_entry(&repo, relative)?.is_some();
            scan_worktree(
                &repo,
                root,
                relative,
                entry.as_ref(),
                true,
                tracked,
                &mut scan,
                &mut budget,
            );
        }
        if paths.len() > MAX_ENTRIES {
            scan.skipped.push("File limit reached.".into());
        }
        Ok(scan)
    }

    pub fn scan(path: &str, staged: bool) -> Result<SecretScan, String> {
        Self::scan_with_limits(path, staged, Limits::default())
    }

    fn scan_with_limits(path: &str, staged: bool, limits: Limits) -> Result<SecretScan, String> {
        let mut budget = Budget::new(limits);
        let repo = Repository::open(path).map_err(|_| FAILED)?;
        let state = State::read(&repo)?;
        let root = repo.workdir().ok_or(FAILED)?;
        let mut scan = SecretScan {
            findings: Vec::new(),
            skipped: Vec::new(),
            snapshot: state.snapshot.clone(),
            large_files: Vec::new(),
        };
        for entry in state.index.iter() {
            if !continue_scan(&mut scan, &budget) {
                break;
            }
            budget.entries += 1;
            let relative = match std::str::from_utf8(&entry.path) {
                Ok(path) if safe_relative(path) => path,
                _ => {
                    scan.skipped
                        .push("An unsupported repository path was not scanned.".into());
                    continue;
                }
            };
            let head_entry = state.head_entry(&repo, relative)?;
            let changed = head_entry != Some((entry.id, entry.mode));
            if entry.mode == 0o160000 {
                continue;
            } // Submodules have no file content in this index.
            if staged {
                if !changed {
                    continue;
                }
                let odb = repo.odb().map_err(|_| FAILED)?;
                let (size, kind) = match odb.read_header(entry.id) {
                    Ok(header) => header,
                    Err(_) => {
                        skip(&mut scan, relative, "Staged content could not be read.");
                        continue;
                    }
                };
                if kind != ObjectType::Blob {
                    skip(&mut scan, relative, "Unsupported staged object.");
                    continue;
                }
                if let Err(reason) = budget.reserve(size) {
                    skip(&mut scan, relative, reason);
                    continue;
                }
                match odb.read(entry.id) {
                    Ok(blob) if blob.len() <= limits.file_bytes => detect(
                        blob.data(),
                        relative,
                        head_entry.is_some(),
                        &mut scan,
                        &budget,
                    ),
                    _ => skip(&mut scan, relative, "Staged content could not be read."),
                };
            } else {
                scan_worktree(
                    &repo,
                    root,
                    relative,
                    Some(&entry),
                    changed,
                    head_entry.is_some(),
                    &mut scan,
                    &mut budget,
                );
            }
        }
        if !staged && continue_scan(&mut scan, &budget) {
            walk_untracked(&repo, &state, root, root, 0, &mut scan, &mut budget)?;
        }
        if State::read(&repo)?.snapshot != scan.snapshot {
            return Err(CHANGED.into());
        }
        scan.findings
            .sort_by(|a, b| (&a.path, a.line, &a.kind).cmp(&(&b.path, b.line, &b.kind)));
        scan.findings
            .dedup_by(|a, b| a.path == b.path && a.line == b.line && a.kind == b.kind);
        scan.skipped.sort();
        scan.skipped.dedup();
        Ok(scan)
    }
}

fn continue_scan(scan: &mut SecretScan, budget: &Budget) -> bool {
    let reason = if scan.findings.len() >= budget.limits.findings {
        Some("Finding limit reached; remaining content was not scanned.")
    } else {
        budget.check().err()
    };
    if let Some(reason) = reason {
        if !scan.skipped.iter().any(|entry| entry == reason) {
            scan.skipped.push(reason.into());
        }
        false
    } else {
        true
    }
}

fn skip(scan: &mut SecretScan, path: &str, reason: &str) {
    scan.skipped.push(format!("{}: {}", path, reason));
}

#[allow(clippy::too_many_arguments)]
fn scan_worktree(
    repo: &Repository,
    root: &Path,
    relative: &str,
    entry: Option<&IndexEntry>,
    changed: bool,
    tracked: bool,
    scan: &mut SecretScan,
    budget: &mut Budget,
) {
    let full_path = match safe_file(root, relative) {
        Ok(Some(path)) => path,
        Ok(None) => return,
        Err(_) => {
            skip(
                scan,
                relative,
                "Unsafe or unreadable worktree file was not scanned.",
            );
            return;
        }
    };
    let meta = match fs::metadata(&full_path) {
        Ok(meta) => meta,
        Err(_) => {
            skip(scan, relative, "Worktree file could not be read.");
            return;
        }
    };
    if meta.len() > budget.limits.file_bytes as u64 {
        // Do not ask libgit2 to hash a huge worktree file just to determine status.
        let unchanged_stat = entry.is_some_and(|e| {
            meta.len() == e.file_size as u64
                && meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .is_some_and(|t| {
                        t.as_secs() == e.mtime.seconds() as u64
                            && t.subsec_nanos() == e.mtime.nanoseconds()
                    })
        });
        if changed || !unchanged_stat {
            skip(scan, relative, "File exceeds the secret scan size limit.");
        }
        return;
    }
    if entry.is_some() && !changed {
        match repo.status_file(Path::new(relative)) {
            Ok(status)
                if !status.is_wt_modified()
                    && !status.is_wt_new()
                    && !status.is_wt_typechange() =>
            {
                return
            }
            Ok(_) => {}
            Err(_) => {
                skip(scan, relative, "Worktree status could not be determined.");
                return;
            }
        }
    }
    if let Err(reason) = budget.reserve(meta.len() as usize) {
        skip(scan, relative, reason);
        return;
    }
    match read_bounded(&full_path, meta.len() as usize) {
        Ok(bytes) => detect(&bytes, relative, tracked, scan, budget),
        Err(_) => skip(
            scan,
            relative,
            "Worktree content changed or could not be read.",
        ),
    }
}

fn walk_untracked(
    repo: &Repository,
    state: &State,
    root: &Path,
    dir: &Path,
    depth: usize,
    scan: &mut SecretScan,
    budget: &mut Budget,
) -> Result<(), String> {
    if depth > 64 {
        scan.skipped
            .push("Directory depth limit reached; remaining files were not scanned.".into());
        return Ok(());
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => {
            scan.skipped
                .push("A worktree directory could not be read.".into());
            return Ok(());
        }
    };
    for entry in entries {
        if !continue_scan(scan, budget) {
            break;
        }
        budget.entries += 1;
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                scan.skipped
                    .push("A worktree entry could not be read.".into());
                continue;
            }
        };
        let path = entry.path();
        let relative = match path.strip_prefix(root).ok().and_then(Path::to_str) {
            Some(path) => path.replace(std::path::MAIN_SEPARATOR, "/"),
            None => {
                scan.skipped
                    .push("An unsupported repository path was not scanned.".into());
                continue;
            }
        };
        if entry.file_name().eq_ignore_ascii_case(".git") {
            continue;
        }
        if !safe_relative(&relative) {
            scan.skipped
                .push("An unsupported repository path was not scanned.".into());
            continue;
        }
        if state.index.get_path(Path::new(&relative), 0).is_some() {
            continue;
        }
        match repo.status_should_ignore(Path::new(&relative)) {
            Ok(true) => continue,
            Ok(false) => {}
            Err(_) => {
                skip(scan, &relative, "Ignore status could not be determined.");
                continue;
            }
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(_) => {
                skip(scan, &relative, "Worktree file could not be read.");
                continue;
            }
        };
        if is_link(&metadata) {
            skip(scan, &relative, "Symbolic links were not followed.");
        } else if metadata.is_dir() {
            if path.join(".git").exists() {
                skip(scan, &relative, "Nested repository was not scanned.");
            } else {
                walk_untracked(repo, state, root, &path, depth + 1, scan, budget)?;
            }
        } else if metadata.is_file() {
            let tracked = state.head_entry(repo, &relative)?.is_some();
            scan_worktree(repo, root, &relative, None, true, tracked, scan, budget);
        } else {
            skip(scan, &relative, "Non-regular file was not scanned.");
        }
    }
    Ok(())
}

struct Rules {
    tokens: Vec<(&'static str, Regex)>,
    assignment: Regex,
}

fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| Rules {
        tokens: [
            ("AWS access key ID", r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b"),
            ("Private key", r"-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY(?: BLOCK)?-----"),
            ("Provider API token", r"\b(?:gh[pousr]_[A-Za-z0-9]{20,255}|github_pat_[A-Za-z0-9_]{20,255}|glpat-[A-Za-z0-9_-]{20,255}|AIza[A-Za-z0-9_-]{35}|xox[baprs]-[A-Za-z0-9-]{10,255}|xapp-[0-9]-[A-Za-z0-9-]{10,255}|sk-(?:proj-|ant-api[0-9]+-)?[A-Za-z0-9_-]{20,255}|[rs]k_(?:live|test)_[A-Za-z0-9]{16,255}|npm_[A-Za-z0-9]{20,255}|pypi-[A-Za-z0-9_-]{30,255}|SG\.[A-Za-z0-9_-]{16,64}\.[A-Za-z0-9_-]{20,128}|dop_v1_[a-f0-9]{64})\b"),
        ].into_iter().map(|(kind, pattern)| (kind, Regex::new(pattern).expect("static secret rule"))).collect(),
        assignment: Regex::new(r#"(?im)(?:^|[^a-z0-9_])['\"]?([a-z_][a-z0-9_.-]*)['\"]?[ \t]*(?:=>|:|=)[ \t]*(?:\"((?:\\.|[^\"\\\r\n])*)\"|'((?:\\.|[^'\\\r\n])*)'|`([^`\r\n]*)`|([^\s,;}\]#]+))"#).expect("static assignment rule"),
    })
}

fn credential_kind(key: &str) -> Option<&'static str> {
    let key: String = key
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect();
    if key.contains("secretaccesskey") {
        Some("AWS secret access key")
    } else if key.contains("password") || key.contains("passwd") || key.ends_with("pwd") {
        Some("Password assignment")
    } else if [
        "apikey",
        "secretkey",
        "privatekey",
        "accesskey",
        "token",
        "secret",
        "credential",
        "credentials",
        "authorization",
        "authkey",
        "clientsecret",
    ]
    .iter()
    .any(|suffix| key.ends_with(suffix))
    {
        Some("Credential assignment")
    } else {
        None
    }
}

fn placeholder(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || matches!(value, "{" | "[" | "=" | "=>" | "|" | ">") {
        return true;
    }
    if (value.starts_with("${") && value.ends_with('}'))
        || (value.starts_with("{{") && value.ends_with("}}"))
        || (value.starts_with('<') && value.ends_with('>'))
    {
        return true;
    }
    if value.starts_with('$')
        && value[1..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return true;
    }
    let lower = value.to_ascii_lowercase();
    if [
        "process.env.",
        "import.meta.env.",
        "os.environ",
        "os.getenv(",
        "getenv(",
        "std::env::",
        "env(",
        "env.",
        "environment.getenvironmentvariable(",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
    {
        return true;
    }
    let compact: String = lower.chars().filter(char::is_ascii_alphanumeric).collect();
    if matches!(
        compact.as_str(),
        "" | "null"
            | "none"
            | "nil"
            | "undefined"
            | "true"
            | "false"
            | "changeme"
            | "replaceme"
            | "redacted"
            | "placeholder"
            | "example"
            | "examplekey"
            | "exampletoken"
            | "examplepassword"
            | "examplesecret"
            | "examplevalue"
            | "sample"
            | "dummy"
            | "todo"
            | "tbd"
            | "string"
            | "str"
            | "stringnew"
    ) {
        return true;
    }
    if [
        "yourapikeyhere",
        "yourapikey",
        "yourkeyhere",
        "yourkey",
        "yourtokenhere",
        "yourtoken",
        "yourpasswordhere",
        "yourpassword",
        "yoursecrethere",
        "yoursecret",
        "insertapikeyhere",
        "replacewithyourapikey",
    ]
    .contains(&compact.as_str())
    {
        return true;
    }
    value
        .chars()
        .all(|c| matches!(c, '*' | 'x' | 'X' | '.' | '_'))
}

fn decode_text(bytes: &[u8]) -> Option<std::borrow::Cow<'_, str>> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if bytes.len() % 2 != 0 {
            return None;
        }
        let le = bytes[0] == 0xff;
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|b| {
                if le {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect();
        Some(std::borrow::Cow::Owned(String::from_utf16_lossy(&units)))
    } else if bytes.contains(&0) {
        None
    } else {
        Some(String::from_utf8_lossy(bytes))
    }
}

fn finding(scan: &mut SecretScan, path: &str, line: usize, kind: &str, tracked: bool) {
    scan.findings.push(SecretFinding {
        path: path.into(),
        line,
        kind: kind.into(),
        tracked,
    });
}

fn detect(bytes: &[u8], path: &str, tracked: bool, scan: &mut SecretScan, budget: &Budget) {
    let Some(text) = decode_text(bytes) else {
        return;
    }; // Non-text content is not scanned.
    let rules = rules();
    for (kind, regex) in &rules.tokens {
        let mut offset = 0;
        let mut line = 1;
        for matched in regex.find_iter(&text) {
            if !continue_detection(scan, budget) {
                return;
            }
            line += text[offset..matched.start()]
                .bytes()
                .filter(|b| *b == b'\n')
                .count();
            offset = matched.start();
            finding(scan, path, line, kind, tracked);
        }
    }
    let mut offset = 0;
    let mut line = 1;
    for captures in rules.assignment.captures_iter(&text) {
        if !continue_detection(scan, budget) {
            return;
        }
        let Some(kind) = credential_kind(&captures[1]) else {
            continue;
        };
        let value = (2..=5)
            .find_map(|i| captures.get(i))
            .expect("assignment value");
        // Unquoted calls, indexing and member access are expressions, not embedded values.
        if captures.get(5).is_some()
            && (value.as_str().contains('(')
                || value.as_str().contains('[')
                || value.as_str().starts_with("self.")
                || value.as_str().starts_with("this.")
                || value.as_str().starts_with('&'))
        {
            continue;
        }
        line += text[offset..value.start()]
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
        offset = value.start();
        if matches!(value.as_str(), "|" | "|-" | "|+" | ">" | ">-" | ">+") {
            // YAML block scalars contain their value on the following indented lines.
            let key = captures.get(1).expect("assignment key");
            let line_start = text[..key.start()].rfind('\n').map_or(0, |i| i + 1);
            let indentation = text[line_start..]
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .count();
            for (n, next) in text[value.end()..].lines().enumerate().skip(1) {
                if !continue_detection(scan, budget) {
                    return;
                }
                if next.trim().is_empty() {
                    continue;
                }
                if next.chars().take_while(|c| *c == ' ' || *c == '\t').count() <= indentation {
                    break;
                }
                if !placeholder(next.trim()) {
                    finding(scan, path, line + n, kind, tracked);
                    break;
                }
            }
        } else if !placeholder(value.as_str()) {
            finding(scan, path, line, kind, tracked);
        }
    }
}

fn continue_detection(scan: &mut SecretScan, budget: &Budget) -> bool {
    if scan.findings.len() >= budget.limits.findings {
        scan.skipped
            .push("Finding limit reached; remaining content was not scanned.".into());
        false
    } else if budget.start.elapsed() >= budget.limits.duration {
        scan.skipped
            .push("Scan time limit reached; remaining content was not scanned.".into());
        false
    } else {
        true
    }
}

/// Call synchronously immediately before the real commit in the same blocking closure.
pub fn validate_commit(path: &str, enabled: bool, approval: Option<&str>) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|_| FAILED)?;
    let current = State::read(&repo)?;
    if let Some(approval) = approval {
        if current.snapshot != approval {
            return Err(CHANGED.into());
        }
        return Ok(());
    }
    let scan = SecretService::review_staged(path, enabled)?;
    if scan.snapshot != current.snapshot {
        return Err(CHANGED.into());
    }
    if !scan.findings.is_empty() || !scan.skipped.is_empty() {
        return Err(BLOCKED.into());
    }
    if !scan.large_files.is_empty() {
        return Err(
            "Staged files exceed GitHub's 100 MiB limit. Review Git LFS before committing.".into(),
        );
    }
    Ok(())
}

struct OwnedLock {
    path: PathBuf,
    active: bool,
}

impl OwnedLock {
    fn create(path: PathBuf, bytes: &[u8]) -> Result<Self, String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| IGNORE_FAILED)?;
        let lock = Self { path, active: true };
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| IGNORE_FAILED)?;
        Ok(lock)
    }

    fn install(&mut self, destination: &Path) -> Result<(), String> {
        fs::rename(&self.path, destination).map_err(|_| IGNORE_FAILED)?;
        self.active = false;
        Ok(())
    }
}

impl Drop for OwnedLock {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn ignore_literal(path: &str) -> String {
    let mut literal = String::from("/");
    for c in path.chars() {
        if matches!(c, '*' | '?' | '[' | ']' | '!' | '#' | ' ' | '\\') {
            literal.push('\\');
        }
        literal.push(c);
    }
    literal
}

fn ignore_contents(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match regular_metadata(path)? {
        Some(meta) if meta.len() <= Limits::default().file_bytes as u64 => {
            read_bounded(path, Limits::default().file_bytes).map(Some)
        }
        Some(_) => Err(IGNORE_FAILED.into()),
        None => Ok(None),
    }
}

/// Explicit UI action only: unstage newly added findings, retain worktree files, never stage .gitignore.
pub fn ignore_untracked(path: &str, paths: &[String], snapshot: &str) -> Result<(), String> {
    if paths.is_empty() || paths.len() > Limits::default().files {
        return Err(IGNORE_FAILED.into());
    }
    let scan = SecretService::scan(path, true)?;
    if scan.snapshot != snapshot {
        return Err(CHANGED.into());
    }
    let candidates: HashSet<&str> = scan
        .findings
        .iter()
        .filter(|f| !f.tracked)
        .map(|f| f.path.as_str())
        .collect();
    let mut selected = HashSet::new();
    for candidate in paths {
        if !safe_relative(candidate)
            || candidate.eq_ignore_ascii_case(".gitignore")
            || !candidates.contains(candidate.as_str())
        {
            return Err(IGNORE_FAILED.into());
        }
        selected.insert(candidate.as_str());
    }
    let repo = Repository::open(path).map_err(|_| IGNORE_FAILED)?;
    let state = State::read(&repo)?;
    if state.snapshot != snapshot {
        return Err(CHANGED.into());
    }
    let root = repo.workdir().ok_or(IGNORE_FAILED)?;
    for candidate in &selected {
        if state.head_entry(&repo, candidate)?.is_some() || safe_file(root, candidate)?.is_none() {
            return Err(IGNORE_FAILED.into());
        }
    }
    let ignore_entry = state
        .index
        .get_path(Path::new(".gitignore"), 0)
        .map(|e| (e.id, e.mode));
    if ignore_entry != state.head_entry(&repo, ".gitignore")? {
        return Err(IGNORE_FAILED.into());
    }
    let ignore_path = root.join(".gitignore");
    let old_ignore = ignore_contents(&ignore_path)?;
    let mut new_ignore = old_ignore.clone().unwrap_or_default();
    if new_ignore.contains(&0) {
        return Err(IGNORE_FAILED.into());
    }
    if !new_ignore.is_empty() && !new_ignore.ends_with(b"\n") {
        new_ignore.push(b'\n');
    }
    let mut selected: Vec<&str> = selected.into_iter().collect();
    selected.sort_unstable();
    for candidate in &selected {
        new_ignore.extend_from_slice(ignore_literal(candidate).as_bytes());
        new_ignore.push(b'\n');
    }
    if new_ignore.len() > Limits::default().file_bytes {
        return Err(IGNORE_FAILED.into());
    }

    // Own Git's conventional lock before rereading the index. Never rewrite a concurrently staged index.
    let index_lock_path = state.index_path.with_file_name("index.lock");
    let index_bytes = read_bounded(&state.index_path, MAX_METADATA_BYTES as usize)?;
    let mut index_lock = OwnedLock::create(index_lock_path, &index_bytes)?;
    if State::read(&repo)?.snapshot != snapshot {
        return Err(CHANGED.into());
    }
    // Also compare raw bytes: a concurrent stat-only index refresh must not be overwritten.
    if read_bounded(&state.index_path, MAX_METADATA_BYTES as usize)? != index_bytes {
        return Err(CHANGED.into());
    }
    let mut locked_index = Index::open(&index_lock.path).map_err(|_| IGNORE_FAILED)?;
    for candidate in selected {
        let entry = locked_index
            .get_path(Path::new(candidate), 0)
            .ok_or(IGNORE_FAILED)?;
        if !matches!(entry.mode, 0o100644 | 0o100755) {
            return Err(IGNORE_FAILED.into());
        }
        locked_index
            .remove_path(Path::new(candidate))
            .map_err(|_| IGNORE_FAILED)?;
    }
    locked_index.write().map_err(|_| IGNORE_FAILED)?;
    drop(locked_index);
    let mut ignore_lock = OwnedLock::create(root.join(".gitignore.gitswamp.lock"), &new_ignore)?;
    if ignore_contents(&ignore_path)? != old_ignore {
        return Err(IGNORE_FAILED.into());
    }
    if State::read(&repo)?.snapshot != snapshot {
        return Err(CHANGED.into());
    }
    // Install ignore rules first. A subsequent failure leaves the secret staged (fail closed).
    // Do not roll back either file over a possible concurrent edit; report partial failure instead.
    ignore_lock.install(&ignore_path)?;
    index_lock.install(&state.index_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        root: PathBuf,
        repo: Repository,
    }
    impl Fixture {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/gitswamp-tests")
                .join(format!("secret-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            let repo = Repository::init(&root).unwrap();
            Self { root, repo }
        }
        fn path(&self) -> &str {
            self.root.to_str().unwrap()
        }
        fn write(&self, path: &str, value: &[u8]) {
            fs::write(self.root.join(path), value).unwrap();
        }
        fn stage(&self, path: &str) {
            let mut index = self.repo.index().unwrap();
            index.add_path(Path::new(path)).unwrap();
            index.write().unwrap();
        }
        fn commit(&self) {
            let id = self.repo.index().unwrap().write_tree().unwrap();
            let tree = self.repo.find_tree(id).unwrap();
            let sig = git2::Signature::now("Test", "test@example.invalid").unwrap();
            let parent = self.repo.head().ok().and_then(|h| h.peel_to_commit().ok());
            self.repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    "Fixture",
                    &tree,
                    &parent.iter().collect::<Vec<_>>(),
                )
                .unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    fn inspect(text: &[u8]) -> SecretScan {
        let mut scan = SecretScan {
            findings: vec![],
            skipped: vec![],
            snapshot: String::new(),
            large_files: vec![],
        };
        detect(
            text,
            "arbitrary.data",
            false,
            &mut scan,
            &Budget::new(Limits::default()),
        );
        scan
    }
    #[test]
    fn recognizes_values_in_any_text_type_but_not_empty_examples_or_expressions() {
        let empty = b"API_KEY=\npassword=\"\"\nsecret: null\napi_key: <API_KEY>\napi_key: your_api_key_here\npassword: string\nconst token = getToken();\nsecret = process.env.SECRET\n";
        assert!(inspect(empty).findings.is_empty());
        for text in [
            "API_KEY=synthetic-value-42",
            "{\"password\":\"synthetic-pass\"}",
            "const apiKey = 'synthetic-key';",
            "client_secret: synthetic-value",
        ] {
            assert!(!inspect(text.as_bytes()).findings.is_empty(), "{text}");
        }
        assert!(!inspect(b"-----BEGIN RSA PRIVATE KEY-----")
            .findings
            .is_empty());
        let token = format!("ghp_{}", "aB34".repeat(10));
        assert!(!inspect(token.as_bytes()).findings.is_empty());
        let utf16 = [
            vec![0xff, 0xfe],
            "password=synthetic-value"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect(),
        ]
        .concat();
        assert!(!inspect(&utf16).findings.is_empty());
        let serialized = serde_json::to_string(&inspect(b"password=synthetic-value")).unwrap();
        assert!(!serialized.contains("synthetic-value"));
    }
    #[test]
    fn staged_content_not_worktree_is_authoritative_and_real_examples_are_checked() {
        let f = Fixture::new();
        f.write(".env.example", b"API_KEY=synthetic-secret-value");
        f.stage(".env.example");
        f.write(".env.example", b"API_KEY=\n");
        let scan = SecretService::scan(f.path(), true).unwrap();
        assert!(scan
            .findings
            .iter()
            .any(|finding| finding.path == ".env.example"));
        assert!(validate_commit(f.path(), true, None).is_err());
        assert!(validate_commit(f.path(), true, Some(&scan.snapshot)).is_ok());
        f.stage(".env.example");
        assert!(validate_commit(f.path(), true, Some(&scan.snapshot)).is_err());
        assert!(SecretService::scan(f.path(), true)
            .unwrap()
            .findings
            .is_empty());
        f.write("untracked.txt", b"password=synthetic-value");
        assert!(SecretService::scan(f.path(), false)
            .unwrap()
            .findings
            .iter()
            .any(|finding| finding.path == "untracked.txt"));
    }
    #[test]
    fn ignore_only_unstages_new_findings_preserving_disk_and_other_staged_work() {
        let f = Fixture::new();
        f.write("tracked.txt", b"safe\n");
        f.stage("tracked.txt");
        f.commit();
        f.write("tracked.txt", b"password=synthetic-tracked");
        f.stage("tracked.txt");
        f.write("new[1].env", b"API_KEY=synthetic-new");
        f.stage("new[1].env");
        let scan = SecretService::scan(f.path(), true).unwrap();
        assert!(ignore_untracked(f.path(), &["tracked.txt".into()], &scan.snapshot).is_err());
        ignore_untracked(f.path(), &["new[1].env".into()], &scan.snapshot).unwrap();
        assert_eq!(
            fs::read(f.root.join("new[1].env")).unwrap(),
            b"API_KEY=synthetic-new"
        );
        let mut index = f.repo.index().unwrap();
        index.read(true).unwrap();
        assert!(index.get_path(Path::new("new[1].env"), 0).is_none());
        assert!(index.get_path(Path::new("tracked.txt"), 0).is_some());
        assert!(index.get_path(Path::new(".gitignore"), 0).is_none());
        assert!(f
            .repo
            .status_should_ignore(Path::new("new[1].env"))
            .unwrap());
    }
    #[test]
    fn budget_limits_are_reported_instead_of_treated_as_clean() {
        let f = Fixture::new();
        f.write("large.txt", b"0123456789");
        f.stage("large.txt");
        let scan = SecretService::scan_with_limits(
            f.path(),
            true,
            Limits {
                file_bytes: 4,
                ..Limits::default()
            },
        )
        .unwrap();
        assert!(scan.findings.is_empty());
        assert!(!scan.skipped.is_empty());
    }
    #[test]
    fn oversized_staged_blobs_need_review_even_if_secret_detection_is_disabled() {
        let f = Fixture::new();
        f.write("large.bin", b"small working copy");
        f.stage("large.bin");
        let odb = f.repo.odb().unwrap();
        let mut writer = odb
            .writer(GITHUB_MAX_FILE_BYTES as usize + 1, ObjectType::Blob)
            .unwrap();
        let chunk = [0u8; 64 * 1024];
        for _ in 0..(GITHUB_MAX_FILE_BYTES as usize / chunk.len()) {
            writer.write_all(&chunk).unwrap();
        }
        writer.write_all(&[0]).unwrap();
        let oid = writer.finalize().unwrap();
        let mut index = f.repo.index().unwrap();
        let mut entry = index.get_path(Path::new("large.bin"), 0).unwrap();
        entry.id = oid;
        entry.file_size = GITHUB_MAX_FILE_BYTES as u32 + 1;
        index.add(&entry).unwrap();
        index.write().unwrap();
        let scan = SecretService::review_staged(f.path(), false).unwrap();
        assert_eq!(scan.large_files.len(), 1);
        assert_eq!(scan.large_files[0].size, GITHUB_MAX_FILE_BYTES + 1);
        assert!(scan.findings.is_empty());
        assert!(scan.skipped.is_empty());
        assert!(validate_commit(f.path(), false, None).is_err());
        assert!(validate_commit(f.path(), false, Some(&scan.snapshot)).is_ok());
        // An LFS pointer stages a small text blob even when the local file is huge.
        let pointer = format!(
            "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize {}\n",
            "0".repeat(64),
            GITHUB_MAX_FILE_BYTES + 1
        );
        f.write("large.bin", pointer.as_bytes());
        f.stage("large.bin");
        assert!(SecretService::review_staged(f.path(), false)
            .unwrap()
            .large_files
            .is_empty());
    }
}

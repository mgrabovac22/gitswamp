mod rules;

use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use git2::{
    Delta, DiffFindOptions, DiffOptions, ErrorCode, FileMode, Index, ObjectType, Oid, Patch,
    Repository, RepositoryState, Status, StatusOptions, Tree,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::{NamedTempFile, TempDir};

use super::secret_service::{
    safe_file, safe_relative, CompositionSecrets, OwnedLock, SecretScan, GITHUB_MAX_FILE_BYTES,
};
use crate::repositories::git_repository::GitRepository as Git;

const MAX_FILES: usize = 500;
const MAX_GROUPS: usize = 20;
const MAX_INDEX: u64 = 16 * 1024 * 1024;
const MAX_INPUT: u64 = 256 * 1024 * 1024;
const PATCH_FILE_LIMIT: usize = 128 * 1024;
const PATCH_TOTAL_LIMIT: usize = 8 * 1024 * 1024;
const STALE: &str = "Working changes or HEAD changed. Generate a new plan before committing.";
static COMPOSER: Mutex<()> = Mutex::new(());

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Composition {
    branch: String,
    snapshot: String,
    changes: Vec<Change>,
    units: Vec<Unit>,
    groups: Vec<Group>,
    review: SecretScan,
    limited: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Change {
    path: String,
    old_path: Option<String>,
    status: &'static str,
    staged: bool,
    partial: bool,
    added: usize,
    removed: usize,
    kind: rules::Kind,
    #[serde(skip)]
    new: Option<(Oid, u32)>,
    #[serde(skip)]
    old: Option<(Oid, u32)>,
    #[serde(skip)]
    hints: rules::Hints,
}

#[derive(Serialize)]
struct Unit {
    id: usize,
    changes: Vec<usize>,
}

#[derive(Serialize)]
struct Group {
    subject: String,
    description: String,
    units: Vec<usize>,
}

#[derive(Deserialize)]
pub struct Draft {
    subject: String,
    description: String,
    units: Vec<usize>,
}

fn git_err(error: git2::Error) -> String {
    error.message().to_owned()
}

fn head(repo: &Repository) -> Result<(String, Option<Oid>), String> {
    if repo.is_bare()
        || repo.state() != RepositoryState::Clean
        || super::interactive_rebase::is_active(repo)
    {
        return Err("Finish the current Git operation before composing commits.".into());
    }
    let reference = repo.find_reference("HEAD").map_err(git_err)?;
    let name = reference
        .symbolic_target()
        .filter(|name| name.starts_with("refs/heads/"))
        .ok_or("Check out a local branch before composing commits.")?
        .to_owned();
    let oid = match repo.find_reference(&name) {
        Ok(branch) => Some(branch.target().ok_or("Invalid local branch.")?),
        Err(error) if error.code() == ErrorCode::NotFound => None,
        Err(error) => return Err(git_err(error)),
    };
    Ok((name, oid))
}

fn index_bytes(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_INDEX + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_INDEX {
        return Err("This Git index is too large for the composer.".into());
    }
    Ok(Some(bytes))
}

struct Snapshot {
    temp: TempDir,
    repo: Repository,
    objects: PathBuf,
    index_path: PathBuf,
    original_index: Option<Vec<u8>>,
    branch: String,
    head: Option<Oid>,
    tree: Oid,
    changes: Vec<Change>,
    limited: bool,
    worktree_staged: bool,
}

fn relative(bytes: &[u8]) -> Result<&str, String> {
    std::str::from_utf8(bytes)
        .ok()
        .filter(|path| safe_relative(path))
        .ok_or_else(|| "The changes include a path the composer cannot safely handle.".into())
}

fn stage_paths(path: &str, target: &Path, objects: &Path, paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut list = NamedTempFile::new().map_err(|e| e.to_string())?;
    for file in paths {
        list.write_all(file.as_bytes())
            .and_then(|_| list.write_all(&[0]))
            .map_err(|e| e.to_string())?;
    }
    // Check using the same Git configuration as add; preview must not run custom clean filters.
    let mut check = Git::git_command(path, &["check-attr", "-z", "--stdin", "filter"], &[]);
    check.stdin(fs::File::open(list.path()).map_err(|e| e.to_string())?);
    let output = check.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("Could not inspect Git clean filters.".into());
    }
    let values: Vec<_> = output
        .stdout
        .strip_suffix(&[0])
        .ok_or("Invalid Git attribute result.")?
        .split(|b| *b == 0)
        .collect();
    if values.len() != paths.len() * 3 {
        return Err("Invalid Git attribute result.".into());
    }
    for item in values.chunks_exact(3) {
        if !matches!(item[2], b"unspecified" | b"set" | b"unset") {
            return Err(format!(
                "Stage {} manually first so its configured Git filter runs outside the preview.",
                relative(item[0])?
            ));
        }
    }
    let source = format!(
        "--pathspec-from-file={}",
        list.path().to_str().ok_or("Unsupported temporary path.")?
    );
    Git::git_output(
        path,
        &[
            "-c",
            "core.splitIndex=false",
            "--literal-pathspecs",
            "add",
            "-A",
            &source,
            "--pathspec-file-nul",
        ],
        &[
            ("GIT_INDEX_FILE", target.as_os_str()),
            ("GIT_OBJECT_DIRECTORY", objects.as_os_str()),
        ],
    )?;
    Ok(())
}

fn entry(file: git2::DiffFile<'_>) -> Option<(Oid, u32)> {
    (!file.id().is_zero()).then(|| (file.id(), file.mode().into()))
}

impl Snapshot {
    fn build(repo: &Repository) -> Result<Self, String> {
        let (branch, head_id) = head(repo)?;
        let root = repo
            .workdir()
            .ok_or("This repository has no working tree.")?;
        let path = root.to_str().ok_or("Unsupported working tree path.")?;
        let index_path = repo.path().join("index");
        let original_index = index_bytes(&index_path)?;
        let live_index = repo.index().map_err(git_err)?;
        if live_index.has_conflicts() || live_index.iter().any(|e| e.flags_extended & 0x2000 != 0) {
            return Err(
                "Resolve conflicts or fully stage intent-to-add files before composing.".into(),
            );
        }
        let mut options = StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_unmodified(false)
            .exclude_submodules(false);
        let statuses = repo.statuses(Some(&mut options)).map_err(git_err)?;
        if statuses.len() > MAX_FILES * 2 {
            return Err(format!(
                "Compose at most {MAX_FILES} changed files at a time."
            ));
        }
        let mut staged = HashSet::new();
        let mut partial = HashSet::new();
        let mut working = Vec::new();
        let mut total_size = 0u64;
        let work_flags = Status::WT_NEW
            | Status::WT_MODIFIED
            | Status::WT_DELETED
            | Status::WT_TYPECHANGE
            | Status::WT_RENAMED;
        for status in statuses.iter() {
            let file = relative(status.path_bytes())?;
            let flags = status.status();
            let index_changed = status.head_to_index().is_some();
            if index_changed {
                staged.insert(file.to_owned());
                if flags.intersects(work_flags) {
                    partial.insert(file.to_owned());
                }
                continue;
            }
            if !flags.intersects(work_flags) {
                continue;
            }
            if let Some(delta) = status.index_to_workdir() {
                if delta.new_file().mode() == FileMode::Commit
                    || delta.old_file().mode() == FileMode::Commit
                {
                    return Err(format!(
                        "Stage the intended submodule pointer manually: {file}"
                    ));
                }
                if delta.new_file().mode() != FileMode::Link && !flags.contains(Status::WT_DELETED)
                {
                    let full = safe_file(root, file)
                        .map_err(|_| format!("Stage this path manually: {file}"))?
                        .ok_or(STALE)?;
                    let bytes = fs::metadata(full).map_err(|e| e.to_string())?.len();
                    if bytes > GITHUB_MAX_FILE_BYTES {
                        return Err(format!(
                            "{file} exceeds 100 MiB. Stage it with Git LFS Manager first."
                        ));
                    }
                    total_size = total_size.saturating_add(bytes);
                }
            }
            working.push(file.to_owned());
        }
        drop(statuses);
        if total_size > MAX_INPUT {
            return Err(
                "The working files exceed the composer input budget. Stage some changes first."
                    .into(),
            );
        }

        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let scratch = Repository::init_opts(
            temp.path().join("repo"),
            git2::RepositoryInitOptions::new()
                .bare(true)
                .no_reinit(true)
                .external_template(false),
        )
        .map_err(git_err)?;
        let source = Git::git_cli(
            path,
            &[
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "objects",
            ],
        )?;
        if source.contains(['\r', '\n']) {
            return Err("Unsupported Git object directory.".into());
        }
        let objects = scratch.path().join("objects");
        fs::write(objects.join("info/alternates"), format!("{source}\n"))
            .map_err(|e| e.to_string())?;
        let original = temp.path().join("index");
        if let Some(bytes) = &original_index {
            fs::write(&original, bytes).map_err(|e| e.to_string())?;
        }
        stage_paths(path, &original, &objects, &working)?;
        let mut index = Index::open(&original).map_err(git_err)?;
        let tree = index.write_tree_to(&scratch).map_err(git_err)?;
        let base = head_id
            .map(|oid| scratch.find_commit(oid).and_then(|commit| commit.tree()))
            .transpose()
            .map_err(git_err)?;
        let target = scratch.find_tree(tree).map_err(git_err)?;
        let base_index_tree = {
            let mut old = live_index;
            old.write_tree_to(&scratch).map_err(git_err)?
        };
        let mut index_diff_options = DiffOptions::new();
        index_diff_options
            .include_typechange(true)
            .skip_binary_check(true);
        let index_diff = scratch
            .diff_tree_to_tree(
                base.as_ref(),
                Some(&scratch.find_tree(base_index_tree).map_err(git_err)?),
                Some(&mut index_diff_options),
            )
            .map_err(git_err)?;
        for delta in index_diff.deltas() {
            let file = relative(
                delta
                    .new_file()
                    .path_bytes()
                    .or_else(|| delta.old_file().path_bytes())
                    .ok_or(STALE)?,
            )?;
            if target
                .get_path(Path::new(file))
                .ok()
                .filter(|e| e.kind() != Some(ObjectType::Tree))
                .map(|e| (e.id(), e.filemode() as u32))
                != entry(delta.new_file())
            {
                return Err(format!("Staged and unstaged edits overlap at {file}. Commit or unstage it manually first."));
            }
        }
        let (changes, limited) = read_changes(&scratch, base.as_ref(), &target, &staged, &partial)?;
        drop((base, target, index_diff));
        if changes.is_empty() {
            return Err("There are no working changes to compose.".into());
        }
        if head(repo)? != (branch.clone(), head_id) || index_bytes(&index_path)? != original_index {
            return Err(STALE.into());
        }
        Ok(Self {
            temp,
            repo: scratch,
            objects: PathBuf::from(source),
            index_path,
            original_index,
            branch,
            head: head_id,
            tree,
            changes,
            limited,
            worktree_staged: !working.is_empty(),
        })
    }

    fn fingerprint(&self, include_secrets: bool) -> String {
        let mut hash = Sha256::new();
        hash.update(b"commit-composer-v1\0");
        hash.update(self.objects.as_os_str().as_encoded_bytes());
        hash.update([0]);
        hash.update(self.index_path.as_os_str().as_encoded_bytes());
        hash.update([0]);
        hash.update(self.branch.as_bytes());
        hash.update(self.head.unwrap_or_else(Oid::zero).as_bytes());
        hash.update(self.tree.as_bytes());
        hash.update([include_secrets as u8]);
        for change in &self.changes {
            hash.update(change.path.as_bytes());
            hash.update([0]);
            hash.update(change.old_path.as_deref().unwrap_or("").as_bytes());
            hash.update([0]);
            hash.update([change.staged as u8, change.partial as u8]);
        }
        format!("{:x}", hash.finalize())
    }

    fn review(&self, enabled: bool, fingerprint: &str) -> Result<SecretScan, String> {
        let mut review = CompositionSecrets::new(fingerprint, enabled);
        let odb = self.repo.odb().map_err(git_err)?;
        for change in &self.changes {
            if let Some((id, mode)) = change.new {
                if mode == 0o160000 {
                    continue;
                }
                let (size, kind) = odb.read_header(id).map_err(git_err)?;
                if kind != ObjectType::Blob {
                    return Err("Invalid file object in the commit plan.".into());
                }
                review.inspect(&self.repo, &change.path, id, size, change.old.is_some());
            }
        }
        Ok(review.finish())
    }
}

fn read_changes(
    repo: &Repository,
    base: Option<&Tree<'_>>,
    target: &Tree<'_>,
    staged: &HashSet<String>,
    partial: &HashSet<String>,
) -> Result<(Vec<Change>, bool), String> {
    let mut options = DiffOptions::new();
    options.skip_binary_check(true).include_typechange(true);
    let mut diff = repo
        .diff_tree_to_tree(base, Some(target), Some(&mut options))
        .map_err(git_err)?;
    if diff.deltas().len() > MAX_FILES * 2 {
        return Err(format!(
            "Compose at most {MAX_FILES} changed files at a time."
        ));
    }
    diff.find_similar(Some(
        DiffFindOptions::new().renames(true).exact_match_only(true),
    ))
    .map_err(git_err)?;
    if diff.deltas().len() > MAX_FILES {
        return Err(format!(
            "Compose at most {MAX_FILES} changed files at a time."
        ));
    }
    let mut changes = Vec::new();
    let mut budget = PATCH_TOTAL_LIMIT;
    let mut limited = false;
    let odb = repo.odb().map_err(git_err)?;
    for (i, delta) in diff.deltas().enumerate() {
        let path = relative(
            delta
                .new_file()
                .path_bytes()
                .or_else(|| delta.old_file().path_bytes())
                .ok_or(STALE)?,
        )?
        .to_owned();
        let old_path = if delta.status() == Delta::Renamed {
            Some(relative(delta.old_file().path_bytes().ok_or(STALE)?)?.to_owned())
        } else {
            None
        };
        let old = entry(delta.old_file());
        let new = entry(delta.new_file());
        let kind = rules::kind(&path, new.or(old).map(|(_, mode)| mode).unwrap_or(0));
        let mut hints = rules::Hints::default();
        let mut added = 0;
        let mut removed = 0;
        if old.map(|v| v.0) != new.map(|v| v.0) && kind.inspect_text() {
            let size = |item: Option<(Oid, u32)>| {
                item.filter(|(_, mode)| *mode != 0o160000)
                    .map_or(Ok(0), |(id, _)| odb.read_header(id).map(|(size, _)| size))
                    .map_err(git_err)
            };
            let (before, after) = (size(old)?, size(new)?);
            if before.max(after) <= PATCH_FILE_LIMIT && before + after <= budget {
                budget -= before + after;
                if let Some(patch) = Patch::from_diff(&diff, i).map_err(git_err)? {
                    (_, added, removed) = patch.line_stats().map_err(git_err)?;
                    limited |= patch.num_hunks() > 64;
                    for hunk in 0..patch.num_hunks().min(64) {
                        let (_, lines) = patch.hunk(hunk).map_err(git_err)?;
                        limited |= lines > 200;
                        for line in 0..lines.min(200) {
                            let line = patch.line_in_hunk(hunk, line).map_err(git_err)?;
                            if line.origin() == '+' || line.origin() == '-' {
                                hints.observe(&path, kind, line.content(), line.origin() == '+');
                            }
                        }
                    }
                }
            } else {
                limited = true;
            }
        }
        changes.push(Change {
            staged: staged.contains(&path) || old_path.as_ref().is_some_and(|p| staged.contains(p)),
            partial: partial.contains(&path)
                || old_path.as_ref().is_some_and(|p| partial.contains(p)),
            status: match delta.status() {
                Delta::Added => "added",
                Delta::Deleted => "deleted",
                Delta::Renamed => "renamed",
                Delta::Typechange => "typechange",
                _ => "modified",
            },
            path,
            old_path,
            old,
            new,
            added,
            removed,
            kind,
            hints,
        });
    }
    Ok((changes, limited))
}

pub fn plan(path: &str, include_secrets: bool) -> Result<Composition, String> {
    let _guard = COMPOSER
        .lock()
        .map_err(|_| "Commit composer is unavailable.")?;
    let snapshot = Snapshot::build(&Git::open(path)?)?;
    let fingerprint = snapshot.fingerprint(include_secrets);
    let review = snapshot.review(include_secrets, &fingerprint)?;
    let (units, groups) = rules::compose(&snapshot.changes);
    Ok(Composition {
        branch: snapshot.branch["refs/heads/".len()..].into(),
        snapshot: fingerprint,
        changes: snapshot.changes,
        units,
        groups,
        review,
        limited: snapshot.limited,
    })
}

fn validate_drafts(units: &[Unit], drafts: &[Draft]) -> Result<(), String> {
    let mut seen = HashSet::new();
    if drafts.is_empty()
        || drafts.len() > MAX_GROUPS
        || drafts.iter().any(|draft| {
            draft.units.is_empty()
                || draft.units.len() > units.len()
                || draft
                    .units
                    .iter()
                    .any(|id| *id >= units.len() || !seen.insert(*id))
                || draft.subject.trim().is_empty()
                || draft.subject.len() > 288
                || draft.subject.chars().any(char::is_control)
                || draft.description.len() > 32 * 1024
                || draft.description.contains('\0')
        })
        || seen.len() != units.len()
    {
        return Err(
            "Each change set must appear exactly once and each commit needs a valid subject."
                .into(),
        );
    }
    Ok(())
}

fn create_series(
    snapshot: &Snapshot,
    signature: &git2::Signature<'_>,
    units: &[Unit],
    drafts: &[Draft],
) -> Result<Vec<Oid>, String> {
    let mut index = Index::new().map_err(git_err)?;
    let mut parent = snapshot.head;
    if let Some(id) = parent {
        index
            .read_tree(
                &snapshot
                    .repo
                    .find_commit(id)
                    .and_then(|c| c.tree())
                    .map_err(git_err)?,
            )
            .map_err(git_err)?;
    }
    let mut commits = Vec::new();
    let mut tree = None;
    for draft in drafts {
        let files = draft
            .units
            .iter()
            .flat_map(|id| units[*id].changes.iter().map(|id| &snapshot.changes[*id]));
        for file in files.clone() {
            let previous = file.old_path.as_deref().unwrap_or(&file.path);
            if file.old.is_some() {
                index.remove_path(Path::new(previous)).map_err(git_err)?;
            }
        }
        for file in files {
            if let Some((id, mode)) = file.new {
                index
                    .add(&git2::IndexEntry {
                        ctime: git2::IndexTime::new(0, 0),
                        mtime: git2::IndexTime::new(0, 0),
                        dev: 0,
                        ino: 0,
                        mode,
                        uid: 0,
                        gid: 0,
                        file_size: 0,
                        id,
                        flags: 0,
                        flags_extended: 0,
                        path: file.path.as_bytes().to_vec(),
                    })
                    .map_err(git_err)?;
            }
        }
        let id = index.write_tree_to(&snapshot.repo).map_err(git_err)?;
        let commit = parent
            .map(|oid| snapshot.repo.find_commit(oid))
            .transpose()
            .map_err(git_err)?;
        if commit.as_ref().is_some_and(|commit| commit.tree_id() == id) {
            return Err("The plan contains an empty commit.".into());
        }
        let body = draft.description.trim();
        let message = if body.is_empty() {
            draft.subject.trim().to_owned()
        } else {
            format!("{}\n\n{body}", draft.subject.trim())
        };
        let commit = snapshot
            .repo
            .commit(
                None,
                signature,
                signature,
                &message,
                &snapshot.repo.find_tree(id).map_err(git_err)?,
                &commit.iter().collect::<Vec<_>>(),
            )
            .map_err(git_err)?;
        tree = Some(id);
        parent = Some(commit);
        commits.push(commit);
    }
    if tree != Some(snapshot.tree) {
        return Err(
            "The commit series does not match the reviewed Git tree. Nothing was committed.".into(),
        );
    }
    Ok(commits)
}

fn publish_objects(source: &Path, target: &Path) -> Result<(), String> {
    for dir in fs::read_dir(source).map_err(|e| e.to_string())? {
        let dir = dir.map_err(|e| e.to_string())?;
        let name = dir.file_name();
        if !name
            .to_str()
            .is_some_and(|s| s.len() == 2 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            continue;
        }
        let destination = target.join(name);
        fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
        for item in fs::read_dir(dir.path()).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let final_path = destination.join(item.file_name());
            if final_path.exists() || fs::hard_link(item.path(), &final_path).is_ok() {
                continue;
            }
            let mut output = NamedTempFile::new_in(&destination).map_err(|e| e.to_string())?;
            std::io::copy(
                &mut fs::File::open(item.path()).map_err(|e| e.to_string())?,
                &mut output,
            )
            .map_err(|e| e.to_string())?;
            output.as_file().sync_all().map_err(|e| e.to_string())?;
            if let Err(error) = output.persist_noclobber(&final_path) {
                if error.error.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(error.error.to_string());
                }
            }
        }
    }
    Ok(())
}

pub fn apply(
    path: &str,
    expected: &str,
    drafts: &[Draft],
    include_secrets: bool,
    approved: bool,
) -> Result<Vec<String>, String> {
    let _guard = COMPOSER
        .lock()
        .map_err(|_| "Commit composer is unavailable.")?;
    let repo = Git::open(path)?;
    let snapshot = Snapshot::build(&repo)?;
    if snapshot.fingerprint(include_secrets) != expected {
        return Err(STALE.into());
    }
    let (units, _) = rules::compose(&snapshot.changes);
    validate_drafts(&units, drafts)?;
    let review = snapshot.review(include_secrets, expected)?;
    if !review.large_files.is_empty() {
        return Err(
            "The plan contains files over 100 MiB. Stage them with Git LFS Manager first.".into(),
        );
    }
    if (!review.findings.is_empty() || !review.skipped.is_empty()) && !approved {
        return Err("Review the secret scan warnings before confirming.".into());
    }
    let signature = repo.signature().map_err(git_err)?;
    let commits = create_series(&snapshot, &signature, &units, drafts)?;

    let lock_path = snapshot.index_path.with_file_name("index.lock");
    let mut index_lock = OwnedLock::create(lock_path.clone(), &[])
        .map_err(|_| "The Git index is locked. Wait for the other operation and try again.")?;
    let mut refs = repo.transaction().map_err(git_err)?;
    refs.lock_ref("HEAD")
        .and_then(|_| refs.lock_ref(&snapshot.branch))
        .map_err(git_err)?;
    if head(&repo)? != (snapshot.branch.clone(), snapshot.head)
        || index_bytes(&snapshot.index_path)? != snapshot.original_index
    {
        return Err(STALE.into());
    }
    publish_objects(&snapshot.repo.path().join("objects"), &snapshot.objects)?;
    refs.set_target(
        &snapshot.branch,
        *commits.last().ok_or("Empty commit plan.")?,
        Some(&signature),
        "commit: compose commits",
    )
    .map_err(git_err)?;
    if snapshot.worktree_staged {
        fs::copy(snapshot.temp.path().join("index"), &lock_path).map_err(|e| e.to_string())?;
        fs::OpenOptions::new()
            .write(true)
            .open(&lock_path)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
        index_lock
            .install(&snapshot.index_path)
            .map_err(|_| "Could not update the Git index; branch left unchanged.")?;
    }
    if let Err(error) = refs.commit() {
        if snapshot.worktree_staged {
            let restored = (|| {
                let mut rollback = OwnedLock::create(
                    lock_path,
                    snapshot.original_index.as_deref().unwrap_or(&[]),
                )?;
                if index_bytes(&snapshot.index_path)?
                    != index_bytes(&snapshot.temp.path().join("index"))?
                {
                    return Err(STALE.into());
                }
                match snapshot.original_index {
                    Some(_) => rollback.install(&snapshot.index_path),
                    None => fs::remove_file(&snapshot.index_path).map_err(|e| e.to_string()),
                }
            })();
            if restored.is_err() {
                return Err(format!("Could not update the branch ({error}) or restore the index. Review staged files."));
            }
        }
        return Err(git_err(error));
    }
    Ok(commits.iter().map(Oid::to_string).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(TempDir);
    impl Fixture {
        fn new(initial: bool) -> Self {
            let f = Self(tempfile::tempdir().unwrap());
            f.git(&["init", "--initial-branch=main"]);
            f.git(&["config", "user.name", "Composer Test"]);
            f.git(&["config", "user.email", "composer@example.invalid"]);
            f.git(&["config", "core.autocrlf", "false"]);
            if initial {
                f.git(&["commit", "--allow-empty", "-m", "Base"]);
            }
            f
        }
        fn path(&self) -> &str {
            self.0.path().to_str().unwrap()
        }
        fn git(&self, args: &[&str]) -> String {
            Git::git_cli(self.path(), args).unwrap()
        }
        fn write(&self, path: &str, bytes: &str) {
            let path = self.0.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        fn drafts(plan: &Composition) -> Vec<Draft> {
            plan.groups
                .iter()
                .map(|g| Draft {
                    subject: g.subject.clone(),
                    description: g.description.clone(),
                    units: g.units.clone(),
                })
                .collect()
        }
        fn apply(&self, plan: &Composition) -> Vec<String> {
            apply(
                self.path(),
                &plan.snapshot,
                &Self::drafts(plan),
                false,
                false,
            )
            .unwrap()
        }
    }

    fn together(plan: &Composition, a: &str, b: &str) -> bool {
        plan.units.iter().any(|unit| {
            [a, b]
                .iter()
                .all(|path| unit.changes.iter().any(|i| plan.changes[*i].path == *path))
        })
    }

    #[test]
    fn composes_exact_snapshots_without_consuming_partial_unstaged_edits() {
        let f = Fixture::new(true);
        f.write("src/auth/token.ts", "export const token = 'base';\n");
        f.git(&["add", "--all"]);
        f.git(&["commit", "-m", "Add original file"]);
        f.write("src/auth/token.ts", "export const token = 'staged';\n");
        f.git(&["add", "--", "src/auth/token.ts"]);
        f.write("src/auth/token.ts", "export const token = 'working';\n");
        f.write(
            "tests/token.test.ts",
            "import { token } from '../src/auth/token';\n",
        );
        f.write("package.json", "{\"name\":\"fixture\"}\n");
        f.write(
            "package-lock.json",
            "{\"name\":\"fixture\",\"lockfileVersion\":3}\n",
        );
        f.write("docs/usage.md", "# Use the fixture\n");
        f.write("src/routes/[route].ts", "export function route() {}\n");
        let head_before = f.git(&["rev-parse", "HEAD"]);
        let index_before = index_bytes(&f.0.path().join(".git/index")).unwrap();
        let objects_before = f.git(&["count-objects"]);
        let plan = plan(f.path(), false).unwrap();
        assert_eq!(f.git(&["count-objects"]), objects_before);
        assert_eq!(f.git(&["rev-parse", "HEAD"]), head_before);
        assert_eq!(
            index_bytes(&f.0.path().join(".git/index")).unwrap(),
            index_before
        );
        assert!(together(&plan, "src/auth/token.ts", "tests/token.test.ts"));
        assert!(together(&plan, "package.json", "package-lock.json"));
        assert!(plan
            .changes
            .iter()
            .any(|file| file.path.ends_with("token.ts") && file.partial));
        let commits = f.apply(&plan);
        assert!(commits.len() >= 3);
        assert_eq!(
            f.git(&["rev-list", "--count", &format!("{head_before}..HEAD")]),
            commits.len().to_string()
        );
        assert_eq!(
            f.git(&["show", "HEAD:src/auth/token.ts"]),
            "export const token = 'staged';"
        );
        assert_eq!(f.git(&["status", "--short"]), "M src/auth/token.ts");
        assert_eq!(
            f.git(&["show", "HEAD:src/routes/[route].ts"]),
            "export function route() {}"
        );
        f.git(&["fsck", "--no-dangling"]);
    }

    #[test]
    fn creates_initial_commit_and_rejects_stale_changes_and_invalid_group_assignment() {
        let f = Fixture::new(false);
        f.write("readme.md", "First draft");
        let first = plan(f.path(), false).unwrap();
        f.write("readme.md", "New contents");
        assert_eq!(
            apply(
                f.path(),
                &first.snapshot,
                &Fixture::drafts(&first),
                false,
                false
            )
            .unwrap_err(),
            STALE
        );
        assert!(!f.0.path().join(".git/index").exists());
        let second = plan(f.path(), false).unwrap();
        let mut drafts = Fixture::drafts(&second);
        let duplicate = drafts[0].units[0];
        drafts[0].units.push(duplicate);
        assert!(apply(f.path(), &second.snapshot, &drafts, false, false).is_err());
        assert_eq!(f.apply(&second).len(), 1);
        assert_eq!(f.git(&["show", "HEAD:readme.md"]), "New contents");
        assert_eq!(f.git(&["symbolic-ref", "--short", "HEAD"]), "main");
    }

    #[test]
    fn groups_renames_and_path_replacements_and_preserves_index_on_cancel() {
        let f = Fixture::new(true);
        f.write("old.rs", "pub fn demo() {}\n");
        f.write("asset", "asset");
        f.git(&["add", "--all"]);
        f.git(&["commit", "-m", "Add original paths"]);
        f.git(&["mv", "old.rs", "new.rs"]);
        fs::remove_file(f.0.path().join("asset")).unwrap();
        f.write("asset/index.js", "export const path = 1;\n");
        let plan = plan(f.path(), false).unwrap();
        assert!(plan
            .changes
            .iter()
            .any(|file| file.path == "new.rs" && file.old_path.as_deref() == Some("old.rs")));
        assert!(together(&plan, "asset", "asset/index.js"));
        f.apply(&plan);
        assert_eq!(f.git(&["status", "--porcelain"]), "");
        f.git(&["fsck", "--no-dangling"]);
    }

    #[test]
    fn enforces_secret_review_and_leaves_custom_filters_to_manual_staging() {
        let f = Fixture::new(true);
        f.write("settings.js", "const api_key = 'AKIAIOSFODNN7EXAMPLE';\n");
        let p = plan(f.path(), true).unwrap();
        assert!(!p.review.findings.is_empty());
        assert!(apply(f.path(), &p.snapshot, &Fixture::drafts(&p), true, false).is_err());
        assert_eq!(f.git(&["rev-list", "--count", "HEAD"]), "1");
        apply(f.path(), &p.snapshot, &Fixture::drafts(&p), true, true).unwrap();
        f.write(".gitattributes", "*.txt filter=custom\n");
        f.write("input.txt", "Custom filter contents");
        assert!(plan(f.path(), false)
            .err()
            .unwrap()
            .contains("configured Git filter"));
    }

    #[test]
    fn groups_matching_dependency_imports_and_schema_migrations() {
        let f = Fixture::new(true);
        f.write(
            "package.json",
            "{\n  \"dependencies\": {\n    \"super-validator\": \"1.0.0\"\n  }\n}\n",
        );
        f.write("src/validation.ts", "import { check } from 'super-validator';\nexport function validate() { return check(); }\n");
        f.write("prisma/schema.prisma", "model User { id Int @id }\n");
        f.write(
            "prisma/migrations/001/migration.sql",
            "CREATE TABLE User (id INTEGER PRIMARY KEY);\n",
        );
        let plan = plan(f.path(), false).unwrap();
        assert!(together(&plan, "src/validation.ts", "package.json"));
        assert!(together(
            &plan,
            "prisma/schema.prisma",
            "prisma/migrations/001/migration.sql"
        ));
        assert!(!together(
            &plan,
            "src/validation.ts",
            "prisma/schema.prisma"
        ));
    }

    #[test]
    fn keeps_cross_module_imports_together_with_a_scope_and_specific_message() {
        let f = Fixture::new(true);
        f.write(
            "packages/cli/src/main.ts",
            "import { buildCommitSeries } from '../../core/src/index';\nbuildCommitSeries();\n",
        );
        f.write(
            "packages/core/src/index.ts",
            "export function buildCommitSeries() {}\n",
        );
        let plan = plan(f.path(), false).unwrap();
        assert!(together(
            &plan,
            "packages/cli/src/main.ts",
            "packages/core/src/index.ts"
        ));
        assert_eq!(plan.groups.len(), 1);
        assert_eq!(
            plan.groups[0].subject,
            "Feat(packages): Add build commit series"
        );
        assert!(plan.groups[0]
            .description
            .contains("add buildCommitSeries; +1/-0 lines"));
    }

    #[test]
    fn pairs_only_the_corresponding_manifest_and_lockfile() {
        let f = Fixture::new(true);
        f.write("pixi.toml", "[workspace]\nname = \"demo\"\n");
        f.write("pixi.lock", "version: 7\n");
        f.write("package.json", "{\"name\":\"demo\"}\n");
        f.write("yarn.lock", "# yarn lockfile\n");
        let plan = plan(f.path(), false).unwrap();
        assert!(together(&plan, "pixi.toml", "pixi.lock"));
        assert!(together(&plan, "package.json", "yarn.lock"));
        assert!(!together(&plan, "pixi.toml", "yarn.lock"));
        assert!(plan.groups.iter().all(|group| group.subject.contains('(')));
    }

    #[test]
    fn explains_actual_changes_without_copying_values_into_messages() {
        let f = Fixture::new(true);
        f.write("app/settings.env", "retry_count=old-value\n");
        f.write(
            "src/app.py",
            "def calculate_total(value):\n    return value\n",
        );
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Add baseline"]);
        f.write("app/settings.env", "retry_count=new-value\n");
        f.write(
            "src/app.py",
            "def calculate_total(value, fee):\n    return value + fee\n",
        );
        f.write(
            "db/migrations/001.sql",
            "ALTER TABLE accounts ADD COLUMN status TEXT;\n",
        );
        f.write("docs/guide.md", "## Configure branch protection\n");
        let plan = plan(f.path(), false).unwrap();
        let messages = plan
            .groups
            .iter()
            .map(|draft| format!("{}\n{}", draft.subject, draft.description))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(messages.contains("Update table accounts"), "{messages}");
        assert!(messages.contains("update calculate_total"), "{messages}");
        assert!(
            messages.contains("update setting retry_count"),
            "{messages}"
        );
        assert!(
            messages.contains("Configure branch protection"),
            "{messages}"
        );
        assert!(!messages.contains("old-value") && !messages.contains("new-value"));
        f.apply(&plan);
        assert_eq!(f.git(&["status", "--porcelain"]), "");
    }

    #[test]
    fn pairs_python_relative_imports_and_only_matching_schema_outputs() {
        let f = Fixture::new(true);
        f.write(
            "pylib/api/routes.py",
            "from ..domain.account import Account\n",
        );
        f.write("pylib/domain/account.py", "class Account: pass\n");
        f.write("proto/account.proto", "message Account {}\n");
        f.write("generated/account_pb2.py", "# generated\n");
        f.write("generated/report_pb2.py", "# generated\n");
        f.write(
            "stores/main/migrations/001.sql",
            "CREATE TABLE ledger (id INT);\n",
        );
        f.write(
            "stores/reporting/migrations/001.sql",
            "CREATE TABLE jobs (id INT);\n",
        );
        let plan = plan(f.path(), false).unwrap();
        assert!(together(
            &plan,
            "pylib/api/routes.py",
            "pylib/domain/account.py"
        ));
        assert!(together(
            &plan,
            "proto/account.proto",
            "generated/account_pb2.py"
        ));
        assert!(!together(
            &plan,
            "proto/account.proto",
            "generated/report_pb2.py"
        ));
        assert!(!together(
            &plan,
            "stores/main/migrations/001.sql",
            "stores/reporting/migrations/001.sql"
        ));
    }

    #[test]
    fn refuses_oversize_without_writing_an_object_or_a_commit() {
        let f = Fixture::new(true);
        fs::File::create(f.0.path().join("large.bin"))
            .unwrap()
            .set_len(GITHUB_MAX_FILE_BYTES + 1)
            .unwrap();
        let before = f.git(&["count-objects"]);
        assert!(plan(f.path(), false).err().unwrap().contains("100 MiB"));
        assert_eq!(f.git(&["count-objects"]), before);
        assert_eq!(f.git(&["rev-list", "--count", "HEAD"]), "1");
    }
}

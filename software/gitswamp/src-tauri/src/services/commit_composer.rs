mod rules;

use std::collections::{BTreeMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use git2::{Delta, DiffFindOptions, DiffOptions, ErrorCode, FileMode, Index, ObjectType, Oid, Patch, Repository, RepositoryState, Status, StatusOptions, Tree};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::{NamedTempFile, TempDir};

use super::secret_service::{safe_file, safe_relative, CompositionSecrets, OwnedLock, SecretScan, GITHUB_MAX_FILE_BYTES};
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

fn git_err(error: git2::Error) -> String { error.message().to_owned() }

fn head(repo: &Repository) -> Result<(String, Option<Oid>), String> {
    if repo.is_bare() || repo.state() != RepositoryState::Clean || super::interactive_rebase::is_active(repo) {
        return Err("Finish the current Git operation before composing commits.".into());
    }
    let reference = repo.find_reference("HEAD").map_err(git_err)?;
    let name = reference.symbolic_target().filter(|name| name.starts_with("refs/heads/"))
        .ok_or("Check out a local branch before composing commits.")?.to_owned();
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
    file.take(MAX_INDEX + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
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
    std::str::from_utf8(bytes).ok().filter(|path| safe_relative(path))
        .ok_or_else(|| "The changes include a path the composer cannot safely handle.".into())
}

fn stage_paths(path: &str, target: &Path, objects: &Path, paths: &[String]) -> Result<(), String> {
    if paths.is_empty() { return Ok(()); }
    let mut list = NamedTempFile::new().map_err(|e| e.to_string())?;
    for file in paths {
        list.write_all(file.as_bytes()).and_then(|_| list.write_all(&[0])).map_err(|e| e.to_string())?;
    }
    let source = format!("--pathspec-from-file={}", list.path().to_str().ok_or("Unsupported temporary path.")?);
    Git::git_output(
        path,
        &["-c", "core.splitIndex=false", "--literal-pathspecs", "add", "-A", &source, "--pathspec-file-nul"],
        &[("GIT_INDEX_FILE", target.as_os_str()), ("GIT_OBJECT_DIRECTORY", objects.as_os_str())],
    )?;
    Ok(())
}

fn entry(file: git2::DiffFile<'_>) -> Option<(Oid, u32)> {
    (!file.id().is_zero()).then(|| (file.id(), file.mode().into()))
}

impl Snapshot {
    fn build(repo: &Repository) -> Result<Self, String> {
        let (branch, head_id) = head(repo)?;
        let root = repo.workdir().ok_or("This repository has no working tree.")?;
        let path = root.to_str().ok_or("Unsupported working tree path.")?;
        let index_path = repo.path().join("index");
        let original_index = index_bytes(&index_path)?;
        let live_index = repo.index().map_err(git_err)?;
        if live_index.has_conflicts() || live_index.iter().any(|e| e.flags_extended & 0x2000 != 0) {
            return Err("Resolve conflicts or fully stage intent-to-add files before composing.".into());
        }
        let mut options = StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true).include_unmodified(false)
            .exclude_submodules(false);
        let statuses = repo.statuses(Some(&mut options)).map_err(git_err)?;
        if statuses.len() > MAX_FILES * 2 {
            return Err(format!("Compose at most {MAX_FILES} changed files at a time."));
        }
        let mut staged = HashSet::new();
        let mut partial = HashSet::new();
        let mut working = Vec::new();
        let mut total_size = 0u64;
        let work_flags = Status::WT_NEW | Status::WT_MODIFIED | Status::WT_DELETED | Status::WT_TYPECHANGE | Status::WT_RENAMED;
        for status in statuses.iter() {
            let file = relative(status.path_bytes())?;
            let flags = status.status();
            let index_changed = status.head_to_index().is_some();
            if index_changed {
                staged.insert(file.to_owned());
                if flags.intersects(work_flags) { partial.insert(file.to_owned()); }
                continue;
            }
            if !flags.intersects(work_flags) { continue; }
            if let Some(delta) = status.index_to_workdir() {
                if delta.new_file().mode() == FileMode::Commit || delta.old_file().mode() == FileMode::Commit {
                    return Err(format!("Stage the intended submodule pointer manually: {file}"));
                }
                if delta.new_file().mode() != FileMode::Link && !flags.contains(Status::WT_DELETED) {
                    let full = safe_file(root, file).map_err(|_| format!("Stage this path manually: {file}"))?
                        .ok_or(STALE)?;
                    let bytes = fs::metadata(full).map_err(|e| e.to_string())?.len();
                    if bytes > GITHUB_MAX_FILE_BYTES {
                        return Err(format!("{file} exceeds 100 MiB. Stage it with Git LFS Manager first."));
                    }
                    total_size = total_size.saturating_add(bytes);
                }
            }
            if repo.get_attr(file, "filter", git2::AttrCheckFlags::FILE_THEN_INDEX).map_err(git_err)?.is_some() {
                return Err(format!("Stage {file} manually first so its configured Git filter runs outside the preview."));
            }
            working.push(file.to_owned());
        }
        drop(statuses);
        if total_size > MAX_INPUT { return Err("The working files exceed the composer input budget. Stage some changes first.".into()); }

        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let scratch = Repository::init_bare(temp.path().join("repo")).map_err(git_err)?;
        let source = Git::git_cli(path, &["rev-parse", "--path-format=absolute", "--git-path", "objects"])?;
        let objects = scratch.path().join("objects");
        fs::write(objects.join("info/alternates"), format!("{source}\n")).map_err(|e| e.to_string())?;
        let original = temp.path().join("index");
        if let Some(bytes) = &original_index {
            fs::write(&original, bytes).map_err(|e| e.to_string())?;
        }
        stage_paths(path, &original, &objects, &working)?;
        let mut index = Index::open(&original).map_err(git_err)?;
        let tree = index.write_tree_to(&scratch).map_err(git_err)?;
        let base = head_id.map(|oid| scratch.find_commit(oid).and_then(|commit| commit.tree())).transpose().map_err(git_err)?;
        let target = scratch.find_tree(tree).map_err(git_err)?;
        let base_index_tree = {
            let mut old = live_index;
            old.write_tree_to(&scratch).map_err(git_err)?
        };
        let index_diff = scratch.diff_tree_to_tree(base.as_ref(), Some(&scratch.find_tree(base_index_tree).map_err(git_err)?), None).map_err(git_err)?;
        for delta in index_diff.deltas() {
            let file = relative(delta.new_file().path_bytes().or_else(|| delta.old_file().path_bytes()).ok_or(STALE)?)?;
            if target.get_path(Path::new(file)).ok().map(|e| (e.id(), e.filemode() as u32)) != entry(delta.new_file()) {
                return Err(format!("Staged and unstaged edits overlap at {file}. Commit or unstage it manually first."));
            }
        }
        let (changes, limited) = read_changes(&scratch, base.as_ref(), &target, &staged, &partial)?;
        drop((base, target, index_diff));
        if changes.is_empty() { return Err("There are no working changes to compose.".into()); }
        if head(repo)? != (branch.clone(), head_id) || index_bytes(&index_path)? != original_index {
            return Err(STALE.into());
        }
        Ok(Self { temp, repo: scratch, objects: PathBuf::from(source), index_path, original_index, branch, head: head_id, tree, changes, limited, worktree_staged: !working.is_empty() })
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
            hash.update([change.staged as u8, change.partial as u8]);
        }
        format!("{:x}", hash.finalize())
    }

    fn review(&self, enabled: bool, fingerprint: &str) -> Result<SecretScan, String> {
        let mut review = CompositionSecrets::new(fingerprint, enabled);
        let odb = self.repo.odb().map_err(git_err)?;
        for change in &self.changes {
            if let Some((id, mode)) = change.new {
                if mode == 0o160000 { continue; }
                let (size, kind) = odb.read_header(id).map_err(git_err)?;
                if kind != ObjectType::Blob { return Err("Invalid file object in the commit plan.".into()); }
                review.inspect(&self.repo, &change.path, id, size, change.old.is_some());
            }
        }
        Ok(review.finish())
    }
}

fn read_changes(repo: &Repository, base: Option<&Tree<'_>>, target: &Tree<'_>, staged: &HashSet<String>, partial: &HashSet<String>) -> Result<(Vec<Change>, bool), String> {
    let mut options = DiffOptions::new();
    options.skip_binary_check(true).include_typechange(true);
    let mut diff = repo.diff_tree_to_tree(base, Some(target), Some(&mut options)).map_err(git_err)?;
    if diff.deltas().len() > MAX_FILES * 2 { return Err(format!("Compose at most {MAX_FILES} changed files at a time.")); }
    diff.find_similar(Some(DiffFindOptions::new().renames(true).exact_match_only(true))).map_err(git_err)?;
    if diff.deltas().len() > MAX_FILES { return Err(format!("Compose at most {MAX_FILES} changed files at a time.")); }
    let mut changes = Vec::new();
    let mut budget = PATCH_TOTAL_LIMIT;
    let mut limited = false;
    let odb = repo.odb().map_err(git_err)?;
    for (i, delta) in diff.deltas().enumerate() {
        let path = relative(delta.new_file().path_bytes().or_else(|| delta.old_file().path_bytes()).ok_or(STALE)?)?.to_owned();
        let old_path = if delta.status() == Delta::Renamed {
            Some(relative(delta.old_file().path_bytes().ok_or(STALE)?)?.to_owned())
        } else { None };
        let old = entry(delta.old_file());
        let new = entry(delta.new_file());
        let kind = rules::kind(&path, new.or(old).map(|(_, mode)| mode).unwrap_or(0));
        let mut hints = rules::Hints::default();
        let mut added = 0;
        let mut removed = 0;
        if old.map(|v| v.0) != new.map(|v| v.0) && kind.inspect_text() {
            let size = |item: Option<(Oid, u32)>| item.filter(|(_, mode)| *mode != 0o160000)
                .map_or(Ok(0), |(id, _)| odb.read_header(id).map(|(size, _)| size)).map_err(git_err);
            let (before, after) = (size(old)?, size(new)?);
            if before.max(after) <= PATCH_FILE_LIMIT && before + after <= budget {
                budget -= before + after;
                if let Some(patch) = Patch::from_diff(&diff, i).map_err(git_err)? {
                    (_, added, removed) = patch.line_stats().map_err(git_err)?;
                    for hunk in 0..patch.num_hunks().min(64) {
                        let (_, lines) = patch.hunk(hunk).map_err(git_err)?;
                        for line in 0..lines.min(200) {
                            let line = patch.line_in_hunk(hunk, line).map_err(git_err)?;
                            if line.origin() == '+' || line.origin() == '-' {
                                hints.observe(&path, line.content(), line.origin() == '+');
                            }
                        }
                    }
                }
            } else { limited = true; }
        }
        changes.push(Change {
            staged: staged.contains(&path) || old_path.as_ref().is_some_and(|p| staged.contains(p)),
            partial: partial.contains(&path) || old_path.as_ref().is_some_and(|p| partial.contains(p)),
            status: match delta.status() { Delta::Added => "added", Delta::Deleted => "deleted", Delta::Renamed => "renamed", Delta::Typechange => "typechange", _ => "modified" },
            path, old_path, old, new, added, removed, kind, hints,
        });
    }
    Ok((changes, limited))
}

pub fn plan(path: &str, include_secrets: bool) -> Result<Composition, String> {
    let _guard = COMPOSER.try_lock().map_err(|_| "A commit composition is already running.")?;
    let snapshot = Snapshot::build(&Git::open(path)?)?;
    let fingerprint = snapshot.fingerprint(include_secrets);
    let review = snapshot.review(include_secrets, &fingerprint)?;
    let (units, groups) = rules::compose(&snapshot.changes);
    Ok(Composition {
        branch: snapshot.branch["refs/heads/".len()..].into(),
        snapshot: fingerprint, changes: snapshot.changes, units, groups, review, limited: snapshot.limited,
    })
}

fn validate_drafts(units: &[Unit], drafts: &[Draft]) -> Result<(), String> {
    let mut seen = HashSet::new();
    if drafts.is_empty() || drafts.len() > MAX_GROUPS || drafts.iter().any(|draft| {
        draft.units.is_empty() || draft.units.len() > units.len() ||
        draft.units.iter().any(|id| *id >= units.len() || !seen.insert(*id)) ||
        draft.subject.trim().is_empty() || draft.subject.len() > 288 ||
        draft.subject.chars().any(char::is_control) ||
        draft.description.len() > 32 * 1024 || draft.description.contains('\0')
    }) || seen.len() != units.len() {
        return Err("Each change set must appear exactly once and each commit needs a valid subject.".into());
    }
    Ok(())
}

fn create_series(snapshot: &Snapshot, signature: &git2::Signature<'_>, units: &[Unit], drafts: &[Draft]) -> Result<Vec<Oid>, String> {
    let mut index = Index::new().map_err(git_err)?;
    let mut parent = snapshot.head;
    if let Some(id) = parent {
        index.read_tree(&snapshot.repo.find_commit(id).and_then(|c| c.tree()).map_err(git_err)?).map_err(git_err)?;
    }
    let mut commits = Vec::new();
    let mut tree = None;
    for draft in drafts {
        let files = draft.units.iter().flat_map(|id| units[*id].changes.iter().map(|id| &snapshot.changes[*id]));
        for file in files.clone() {
            let previous = file.old_path.as_deref().unwrap_or(&file.path);
            if file.old.is_some() { index.remove_path(Path::new(previous)).map_err(git_err)?; }
        }
        for file in files {
            if let Some((id, mode)) = file.new {
                index.add(&git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0), mtime: git2::IndexTime::new(0, 0),
                    dev: 0, ino: 0, mode, uid: 0, gid: 0, file_size: 0, id,
                    flags: 0, flags_extended: 0, path: file.path.as_bytes().to_vec(),
                }).map_err(git_err)?;
            }
        }
        let id = index.write_tree_to(&snapshot.repo).map_err(git_err)?;
        let commit = parent.map(|oid| snapshot.repo.find_commit(oid)).transpose().map_err(git_err)?;
        if commit.as_ref().is_some_and(|commit| commit.tree_id() == id) { return Err("The plan contains an empty commit.".into()); }
        let body = draft.description.trim();
        let message = if body.is_empty() { draft.subject.trim().to_owned() } else { format!("{}\n\n{body}", draft.subject.trim()) };
        let commit = snapshot.repo.commit(None, signature, signature, &message, &snapshot.repo.find_tree(id).map_err(git_err)?, &commit.iter().collect::<Vec<_>>()).map_err(git_err)?;
        tree = Some(id);
        parent = Some(commit);
        commits.push(commit);
    }
    if tree != Some(snapshot.tree) {
        return Err("The commit series does not match the reviewed Git tree. Nothing was committed.".into());
    }
    Ok(commits)
}

fn publish_objects(source: &Path, target: &Path) -> Result<(), String> {
    for dir in fs::read_dir(source).map_err(|e| e.to_string())? {
        let dir = dir.map_err(|e| e.to_string())?;
        let name = dir.file_name();
        if !name.to_str().is_some_and(|s| s.len() == 2 && s.bytes().all(|b| b.is_ascii_hexdigit())) { continue; }
        let destination = target.join(name);
        fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
        for item in fs::read_dir(dir.path()).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let final_path = destination.join(item.file_name());
            if final_path.exists() || fs::hard_link(item.path(), &final_path).is_ok() { continue; }
            let mut output = NamedTempFile::new_in(&destination).map_err(|e| e.to_string())?;
            std::io::copy(&mut fs::File::open(item.path()).map_err(|e| e.to_string())?, &mut output).map_err(|e| e.to_string())?;
            output.as_file().sync_all().map_err(|e| e.to_string())?;
            if let Err(error) = output.persist_noclobber(&final_path) {
                if error.error.kind() != std::io::ErrorKind::AlreadyExists { return Err(error.error.to_string()); }
            }
        }
    }
    Ok(())
}

pub fn apply(path: &str, expected: &str, drafts: &[Draft], include_secrets: bool, approved: bool) -> Result<Vec<String>, String> {
    let _guard = COMPOSER.try_lock().map_err(|_| "A commit composition is already running.")?;
    let repo = Git::open(path)?;
    let snapshot = Snapshot::build(&repo)?;
    if snapshot.fingerprint(include_secrets) != expected { return Err(STALE.into()); }
    let (units, _) = rules::compose(&snapshot.changes);
    validate_drafts(&units, drafts)?;
    let review = snapshot.review(include_secrets, expected)?;
    if !review.large_files.is_empty() {
        return Err("The plan contains files over 100 MiB. Stage them with Git LFS Manager first.".into());
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
    refs.lock_ref("HEAD").and_then(|_| refs.lock_ref(&snapshot.branch)).map_err(git_err)?;
    if head(&repo)? != (snapshot.branch.clone(), snapshot.head) || index_bytes(&snapshot.index_path)? != snapshot.original_index {
        return Err(STALE.into());
    }
    publish_objects(&snapshot.repo.path().join("objects"), &snapshot.objects)?;
    refs.set_target(&snapshot.branch, *commits.last().ok_or("Empty commit plan.")?, Some(&signature), "commit: compose commits").map_err(git_err)?;
    if snapshot.worktree_staged {
        fs::copy(snapshot.temp.path().join("index"), &lock_path).map_err(|e| e.to_string())?;
        fs::File::open(&lock_path).and_then(|file| file.sync_all()).map_err(|e| e.to_string())?;
        index_lock.install(&snapshot.index_path).map_err(|_| "Could not update the Git index; branch left unchanged.")?;
    }
    if let Err(error) = refs.commit() {
        if snapshot.worktree_staged {
            let restored = match &snapshot.original_index {
                Some(bytes) => OwnedLock::create(lock_path, bytes).and_then(|mut lock| lock.install(&snapshot.index_path)),
                None => fs::remove_file(&snapshot.index_path).map_err(|e| e.to_string()),
            };
            if restored.is_err() { return Err(format!("Could not update the branch ({error}) or restore the index. Review staged files.")); }
        }
        return Err(git_err(error));
    }
    Ok(commits.iter().map(Oid::to_string).collect())
}

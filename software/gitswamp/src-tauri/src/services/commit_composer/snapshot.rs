use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use git2::{
    Delta, DiffFindOptions, DiffOptions, FileMode, Index, ObjectType, Oid, Patch, Repository,
    Status, StatusOptions, Tree,
};
use sha2::{Digest, Sha256};
use tempfile::{NamedTempFile, TempDir};

use super::super::secret_service::{
    safe_file, safe_relative, CompositionSecrets, GITHUB_MAX_FILE_BYTES,
};
use super::{
    chunks, git_err, head, rules, Change, MAX_INDEX, PATCH_FILE_LIMIT, STALE,
};
use crate::repositories::git_repository::GitRepository as Git;

const MAX_FILES: usize = 2000;
const MAX_INPUT: u64 = 256 * 1024 * 1024;

pub(super) struct Snapshot {
    pub(super) temp: TempDir,
    pub(super) repo: Repository,
    pub(super) objects: PathBuf,
    pub(super) index_path: PathBuf,
    pub(super) original_index: Option<Vec<u8>>,
    pub(super) branch: String,
    pub(super) head: Option<Oid>,
    pub(super) tree: Oid,
    pub(super) changes: Vec<Change>,
    pub(super) limited: bool,
    pub(super) worktree_staged: bool,
}

pub(super) fn relative(bytes: &[u8]) -> Result<&str, String> {
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
    pub(super) fn build(repo: &Repository, analyze_behavior: bool) -> Result<Self, String> {
        let (branch, head_id) = head(repo)?;
        let root = repo
            .workdir()
            .ok_or("This repository has no working tree.")?;
        let path = root.to_str().ok_or("Unsupported working tree path.")?;
        let index_path = repo.path().join("index");
        let original_index = Git::index_bytes(&index_path, MAX_INDEX)?;
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
        let (changes, limited) = read_changes(
            &scratch,
            base.as_ref(),
            &target,
            &staged,
            &partial,
            analyze_behavior,
        )?;
        drop((base, target, index_diff));
        if changes.is_empty() {
            return Err("There are no working changes to compose.".into());
        }
        if head(repo)? != (branch.clone(), head_id)
            || Git::index_bytes(&index_path, MAX_INDEX)? != original_index
        {
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

    pub(super) fn fingerprint(&self, include_secrets: bool) -> String {
        let mut hash = Sha256::new();
        hash.update(b"commit-composer-v2\0");
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
            if let Some(chunk) = &change.chunk {
                hash.update(chunk.old_start.to_le_bytes());
                hash.update(chunk.new_start.to_le_bytes());
            }
        }
        format!("{:x}", hash.finalize())
    }

    pub(super) fn review(&self, review: &mut CompositionSecrets) -> Result<(), String> {
        let odb = self.repo.odb().map_err(git_err)?;
        let mut seen = HashSet::new();
        for change in &self.changes {
            if let Some((id, mode)) = change.new {
                if mode == 0o160000 || !seen.insert(&change.path) {
                    continue;
                }
                let (size, kind) = odb.read_header(id).map_err(git_err)?;
                if kind != ObjectType::Blob {
                    return Err("Invalid file object in the commit plan.".into());
                }
                review.inspect(&self.repo, &change.path, id, size, change.old.is_some());
            }
        }
        Ok(())
    }
}

fn read_changes(
    repo: &Repository,
    base: Option<&Tree<'_>>,
    target: &Tree<'_>,
    staged: &HashSet<String>,
    partial: &HashSet<String>,
    analyze_behavior: bool,
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
    let mut extra_changes = chunks::MAX_EXTRA_CHANGES;
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
        let staged_file =
            staged.contains(&path) || old_path.as_ref().is_some_and(|p| staged.contains(p));
        let partial_file =
            partial.contains(&path) || old_path.as_ref().is_some_and(|p| partial.contains(p));
        let language_support = matches!(kind, rules::Kind::Source | rules::Kind::Test)
            && rules::behavior::supported(&path);
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
            if before.max(after) <= PATCH_FILE_LIMIT {
                if let Some(patch) = Patch::from_diff(&diff, i).map_err(git_err)? {
                    (_, added, removed) = patch.line_stats().map_err(git_err)?;
                    let track_behavior = analyze_behavior && language_support;
                    let split = rules::behavior::splittable(&path)
                        && matches!(kind, rules::Kind::Source | rules::Kind::Test)
                        && delta.status() == Delta::Modified
                        && patch.num_hunks() > 1
                        && patch.num_hunks() - 1 <= extra_changes
                        && before + after <= chunks::MAX_SPLIT_BYTES
                        && old.zip(new).is_some_and(|((_, a), (_, b))| {
                            a == b && matches!(a, 0o100644 | 0o100755)
                        });
                    let read_text = |entry: Option<(Oid, u32)>| {
                        entry
                            .filter(|(_, mode)| matches!(mode, 0o100644 | 0o100755))
                            .map(|(oid, _)| repo.find_blob(oid))
                            .transpose()
                            .map_err(git_err)
                    };
                    let old_text = if track_behavior || split {
                        read_text(old)?
                    } else {
                        None
                    };
                    let new_text = if track_behavior || split {
                        read_text(new)?
                    } else {
                        None
                    };
                    let mut previous = old_text
                        .as_ref()
                        .filter(|blob| !blob.is_binary())
                        .map(|blob| rules::behavior::Cursor::new(&path, blob.content()));
                    let mut next = new_text
                        .as_ref()
                        .filter(|blob| !blob.is_binary())
                        .map(|blob| rules::behavior::Cursor::new(&path, blob.content()));
                    let pieces = if split {
                        match (old_text.as_ref(), new_text.as_ref()) {
                            (Some(a), Some(b)) => {
                                chunks::identify(&path, &patch, a.content(), b.content())
                                    .map_err(git_err)?
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let start = changes.len();
                    for hunk in 0..patch.num_hunks() {
                        let (_, lines) = patch.hunk(hunk).map_err(git_err)?;
                        let mut hunk_hints = rules::Hints::default();
                        let mut hunk_added = 0;
                        let mut hunk_removed = 0;
                        for line in 0..lines {
                            let line = patch.line_in_hunk(hunk, line).map_err(git_err)?;
                            if line.origin() == '+' || line.origin() == '-' {
                                let added = line.origin() == '+';
                                if added {
                                    hunk_added += 1;
                                } else {
                                    hunk_removed += 1;
                                }
                                let hints = if pieces.is_some() {
                                    &mut hunk_hints
                                } else {
                                    &mut hints
                                };
                                hints.observe(&path, kind, line.content(), added, analyze_behavior);
                                let (cursor, number) = if added {
                                    (&mut next, line.new_lineno())
                                } else {
                                    (&mut previous, line.old_lineno())
                                };
                                if track_behavior {
                                    if let (Some(cursor), Some(number)) = (cursor, number) {
                                        hints.observe_body(cursor.line_at(number), added);
                                    }
                                }
                            }
                        }
                        if let Some(chunk) = pieces.as_ref().and_then(|pieces| pieces.get(hunk)) {
                            changes.push(Change {
                                path: path.clone(),
                                old_path: None,
                                status: "modified",
                                staged: staged_file,
                                partial: partial_file,
                                added: hunk_added,
                                removed: hunk_removed,
                                kind,
                                chunk: Some(chunk.clone()),
                                old,
                                new,
                                hints: hunk_hints,
                            });
                        }
                    }
                    if changes.len() > start {
                        extra_changes -= changes.len() - start - 1;
                        continue;
                    }
                }
            } else {
                limited = true;
            }
        }
        changes.push(Change {
            staged: staged_file,
            partial: partial_file,
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
            chunk: None,
            hints,
        });
    }
    Ok((changes, limited))
}

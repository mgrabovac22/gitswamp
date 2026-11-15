mod chunks;
mod dependencies;
mod rules;
mod verification;

use std::collections::{BTreeMap, HashSet};
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
    dependency_notice: Option<String>,
}

#[derive(Serialize)]
pub struct BuildPlan {
    groups: Vec<Group>,
    review: SecretScan,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk: Option<chunks::Chunk>,
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
    fn build(repo: &Repository, analyze_behavior: bool) -> Result<Self, String> {
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

    fn review(&self, review: &mut CompositionSecrets) -> Result<(), String> {
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
    let mut budget = PATCH_TOTAL_LIMIT;
    let mut split_bytes = chunks::MAX_SPLIT_BYTES;
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
            if before.max(after) <= PATCH_FILE_LIMIT && before + after <= budget {
                budget -= before + after;
                if let Some(patch) = Patch::from_diff(&diff, i).map_err(git_err)? {
                    (_, added, removed) = patch.line_stats().map_err(git_err)?;
                    limited |= patch.num_hunks() > 64;
                    let track_behavior = analyze_behavior && language_support;
                    let split = rules::behavior::splittable(&path)
                        && matches!(kind, rules::Kind::Source | rules::Kind::Test)
                        && delta.status() == Delta::Modified
                        && patch.num_hunks() > 1
                        && patch.num_hunks() - 1 <= extra_changes
                        && before + after <= split_bytes
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
                        split_bytes -= before + after;
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
                    for hunk in 0..patch.num_hunks().min(64) {
                        let (_, lines) = patch.hunk(hunk).map_err(git_err)?;
                        limited |= lines > 200;
                        let mut hunk_hints = rules::Hints::default();
                        let mut hunk_added = 0;
                        let mut hunk_removed = 0;
                        for line in 0..lines.min(200) {
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

pub fn plan(path: &str, include_secrets: bool) -> Result<Composition, String> {
    let _guard = COMPOSER
        .lock()
        .map_err(|_| "Commit composer is unavailable.")?;
    let snapshot = Snapshot::build(&Git::open(path)?, true)?;
    let fingerprint = snapshot.fingerprint(include_secrets);
    let (units, mut groups) = rules::compose(&snapshot.changes);
    let mut review = CompositionSecrets::new(&fingerprint, include_secrets);
    snapshot.review(&mut review)?;
    let mut dependencies = dependencies::Check::new(
        &snapshot.repo,
        snapshot.head,
        snapshot.tree,
        &snapshot.changes,
    )?;
    let target = snapshot.repo.find_tree(snapshot.tree).map_err(git_err)?;
    if let Some(problem) = dependencies.verify(&target) {
        return Err(format!(
            "The proposed final state is missing a local dependency. {problem}"
        ));
    }
    let (mut dependency_notice, mut intermediate) = arrange_groups(
        &snapshot,
        &units,
        &mut groups,
        &mut dependencies,
        include_secrets,
    )?;
    if dependencies.limited {
        dependency_notice = Some(
            "The dependency review reached its limit. The preview kept all changes in one commit."
                .into(),
        );
        groups = vec![rules::combined(
            &snapshot.changes,
            &units,
            units.iter().map(|unit| unit.id).collect(),
        )];
        intermediate.clear();
    }
    review_intermediate(&snapshot, &intermediate, &mut review);
    Ok(Composition {
        branch: snapshot.branch["refs/heads/".len()..].into(),
        snapshot: fingerprint,
        changes: snapshot.changes,
        units,
        groups,
        review: review.finish(),
        limited: snapshot.limited,
        dependency_notice,
    })
}

type Intermediate = BTreeMap<(String, Oid), usize>;

fn arrange_groups(
    snapshot: &Snapshot,
    units: &[Unit],
    groups: &mut Vec<Group>,
    dependencies: &mut dependencies::Check<'_>,
    inspect_intermediate: bool,
) -> Result<(Option<String>, Intermediate), String> {
    let mut notice = None;
    loop {
        let mut series = Series::new(snapshot)?;
        let mut pieces = Intermediate::new();
        let mut failed = None;
        let mut last = None;
        for (position, group) in groups.iter().enumerate() {
            let (id, staged) = series.advance(units, &group.units)?;
            last = Some(id);
            if let Some(problem) =
                dependencies.verify(&snapshot.repo.find_tree(id).map_err(git_err)?)
            {
                failed = Some((position, problem));
                break;
            }
            if inspect_intermediate {
                pieces.extend(staged);
            }
        }
        let Some((position, problem)) = failed else {
            if last != Some(snapshot.tree) {
                return Err(
                    "The proposed hunks do not reproduce the final reviewed Git tree.".into(),
                );
            }
            return Ok((notice, pieces));
        };
        notice.get_or_insert_with(|| format!("Grouped dependent changes. {problem}"));
        let provider = groups
            .iter()
            .enumerate()
            .skip(position + 1)
            .find(|(_, group)| {
                group
                    .units
                    .iter()
                    .flat_map(|id| &units[*id].changes)
                    .any(|id| problem.is_provider(&snapshot.changes[*id]))
            })
            .map(|(index, _)| index);
        if let Some(provider) = provider {
            let dependency = groups.remove(provider);
            let selected = groups[position]
                .units
                .iter()
                .copied()
                .chain(dependency.units)
                .collect();
            groups[position] = rules::combined_with_reason(
                &snapshot.changes,
                units,
                selected,
                "Keep these changes together so local references still resolve at this point in the series.",
            );
        } else if position + 1 < groups.len() {
            let following = groups.remove(position + 1);
            let selected = groups[position]
                .units
                .iter()
                .copied()
                .chain(following.units)
                .collect();
            groups[position] = rules::combined_with_reason(
                &snapshot.changes,
                units,
                selected,
                "Keep these changes together so local references still resolve at this point in the series.",
            );
        } else {
            return Err(format!(
                "The composed changes do not pass dependency review. {problem}"
            ));
        }
    }
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

pub fn find_buildable_groups(
    path: &str,
    expected: &str,
    drafts: &[Draft],
    include_secrets: bool,
    command: &str,
) -> Result<BuildPlan, String> {
    use verification::CheckError;

    let _guard = COMPOSER
        .lock()
        .map_err(|_| "Commit composer is unavailable.")?;
    let snapshot = Snapshot::build(&Git::open(path)?, true)?;
    if snapshot.fingerprint(include_secrets) != expected {
        return Err(STALE.into());
    }
    let units = rules::atomic_units(&snapshot.changes);
    validate_drafts(&units, drafts)?;
    let verify = verification::Verification::new(command)?;
    let mut dependencies = dependencies::Check::new(
        &snapshot.repo,
        snapshot.head,
        snapshot.tree,
        &snapshot.changes,
    )?;
    let mut state = Series::new(&snapshot)?;
    let mut groups = Vec::new();
    let mut pieces = Intermediate::new();
    let mut pending = Vec::new();
    let mut start = 0;
    let mut last = None;
    for (index, draft) in drafts.iter().enumerate() {
        pending.extend(draft.units.iter().copied());
        let (id, staged) = state.advance(&units, &draft.units)?;
        last = Some(id);
        let number = groups.len() + 1;
        let tree = snapshot.repo.find_tree(id).map_err(git_err)?;
        let problem = dependencies.verify(&tree);
        if dependencies.limited && !groups.is_empty() {
            return Err("Dependency review reached its limit. Combine the changes in one commit or stage a smaller batch.".into());
        }
        if index + 1 < drafts.len() && (problem.is_some() || dependencies.limited) {
            continue;
        }
        if let Some(problem) = problem {
            return Err(format!(
                "The final state does not pass dependency review: {problem}"
            ));
        }
        match verify.check(&snapshot.repo, &tree, number) {
            Ok(()) => {}
            Err(CheckError::Rejected(_)) if index + 1 < drafts.len() => continue,
            Err(CheckError::Rejected(message)) => {
                return Err(format!("The final state does not build with this command. Regrouping cannot fix the final tree.\n{message}"));
            }
            Err(error) => return Err(error.message()),
        }
        if include_secrets {
            // A chunk that was selected in a failed prefix may still be present in this tree.
            for file in pending.iter().flat_map(|unit| &units[*unit].changes) {
                let file = &snapshot.changes[*file];
                if file.chunk.is_some() {
                    let entry = tree.get_path(Path::new(&file.path)).map_err(git_err)?;
                    if file.new.is_some_and(|(final_id, _)| entry.id() != final_id) {
                        let (size, _) = snapshot
                            .repo
                            .odb()
                            .and_then(|odb| odb.read_header(entry.id()))
                            .map_err(git_err)?;
                        pieces.insert((file.path.clone(), entry.id()), size);
                    }
                }
            }
            pieces.extend(staged);
        }
        let selected = std::mem::take(&mut pending);
        let group = if index == start {
            Group {
                subject: draft.subject.clone(),
                description: draft.description.clone(),
                units: selected,
            }
        } else {
            rules::combined_with_reason(
                &snapshot.changes,
                &units,
                selected,
                "Keep these changes together so the intermediate snapshot passes the configured checks.",
            )
        };
        groups.push(group);
        start = index + 1;
    }
    if last != Some(snapshot.tree) {
        return Err("The proposed hunks do not reproduce the final reviewed Git tree.".into());
    }
    if Snapshot::build(&Git::open(path)?, false)?.fingerprint(include_secrets) != expected {
        return Err(STALE.into());
    }
    let mut review = CompositionSecrets::new(expected, include_secrets);
    snapshot.review(&mut review)?;
    review_intermediate(&snapshot, &pieces, &mut review);
    Ok(BuildPlan {
        groups,
        review: review.finish(),
    })
}

struct Series<'a> {
    snapshot: &'a Snapshot,
    index: Index,
    active: BTreeMap<&'a str, Vec<&'a Change>>,
}

impl<'a> Series<'a> {
    fn new(snapshot: &'a Snapshot) -> Result<Self, String> {
        let mut index = Index::new().map_err(git_err)?;
        if let Some(id) = snapshot.head {
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
        Ok(Self {
            snapshot,
            index,
            active: BTreeMap::new(),
        })
    }

    fn advance(
        &mut self,
        units: &[Unit],
        selected: &[usize],
    ) -> Result<(Oid, Intermediate), String> {
        let files = selected.iter().flat_map(|id| {
            units[*id]
                .changes
                .iter()
                .map(|id| &self.snapshot.changes[*id])
        });
        let mut dirty = BTreeMap::new();
        for file in files.clone() {
            if file.chunk.is_some() {
                self.active.entry(&file.path).or_default().push(file);
                dirty.insert(file.path.as_str(), file);
            } else if file.old.is_some() {
                let previous = file.old_path.as_deref().unwrap_or(&file.path);
                self.index
                    .remove_path(Path::new(previous))
                    .map_err(git_err)?;
            }
        }
        for file in files.filter(|file| file.chunk.is_none()) {
            if let Some((id, mode)) = file.new {
                self.add(&file.path, id, mode)?;
            }
        }
        let mut reviews = BTreeMap::new();
        for (path, file) in dirty {
            let (final_blob, mode) = file.new.ok_or("Hunk has no final file.")?;
            let id = chunks::blob(&self.snapshot.repo, &self.active[path])?;
            if id != final_blob {
                let (size, _) = self
                    .snapshot
                    .repo
                    .odb()
                    .and_then(|odb| odb.read_header(id))
                    .map_err(git_err)?;
                reviews.insert((path.to_owned(), id), size);
            }
            self.add(path, id, mode)?;
        }
        Ok((
            self.index
                .write_tree_to(&self.snapshot.repo)
                .map_err(git_err)?,
            reviews,
        ))
    }

    fn add(&mut self, path: &str, id: Oid, mode: u32) -> Result<(), String> {
        self.index
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
                path: path.as_bytes().to_vec(),
            })
            .map_err(git_err)
    }
}

fn review_intermediate(
    snapshot: &Snapshot,
    blobs: &BTreeMap<(String, Oid), usize>,
    review: &mut CompositionSecrets,
) {
    for ((path, id), size) in blobs {
        review.inspect(&snapshot.repo, path, *id, *size, true);
    }
}

fn create_series(
    snapshot: &Snapshot,
    signature: &git2::Signature<'_>,
    units: &[Unit],
    drafts: &[Draft],
    mut review: Option<&mut CompositionSecrets>,
) -> Result<Vec<Oid>, String> {
    let mut series = Series::new(snapshot)?;
    let mut dependencies = dependencies::Check::new(
        &snapshot.repo,
        snapshot.head,
        snapshot.tree,
        &snapshot.changes,
    )?;
    let mut parent = snapshot.head;
    let mut commits = Vec::new();
    let mut tree = None;
    let mut reviewed = HashSet::new();
    for (number, draft) in drafts.iter().enumerate() {
        let (id, mut intermediate) = series.advance(units, &draft.units)?;
        let current_tree = snapshot.repo.find_tree(id).map_err(git_err)?;
        if let Some(problem) = dependencies.verify(&current_tree) {
            return Err(format!(
                "Commit {}: {problem} Keep the changes together or move the dependency earlier.",
                number + 1
            ));
        }
        if dependencies.limited && drafts.len() > 1 {
            return Err("The dependency review reached its limit. Keep these changes in one commit or stage a smaller batch.".into());
        }
        if let Some(review) = review.as_deref_mut() {
            intermediate.retain(|key, _| reviewed.insert(key.clone()));
            review_intermediate(snapshot, &intermediate, review);
        }
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
                &current_tree,
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

#[cfg(test)]
fn apply(
    path: &str,
    expected: &str,
    drafts: &[Draft],
    include_secrets: bool,
    approved: bool,
) -> Result<Vec<String>, String> {
    apply_with_verification(path, expected, drafts, include_secrets, approved, None)
}

pub fn apply_with_verification(
    path: &str,
    expected: &str,
    drafts: &[Draft],
    include_secrets: bool,
    approved: bool,
    build_command: Option<&str>,
) -> Result<Vec<String>, String> {
    let _guard = COMPOSER
        .lock()
        .map_err(|_| "Commit composer is unavailable.")?;
    let repo = Git::open(path)?;
    let snapshot = Snapshot::build(&repo, false)?;
    if snapshot.fingerprint(include_secrets) != expected {
        return Err(STALE.into());
    }
    let units = rules::atomic_units(&snapshot.changes);
    validate_drafts(&units, drafts)?;
    let mut review = CompositionSecrets::new(expected, include_secrets);
    snapshot.review(&mut review)?;
    let signature = repo.signature().map_err(git_err)?;
    let commits = create_series(
        &snapshot,
        &signature,
        &units,
        drafts,
        include_secrets.then_some(&mut review),
    )?;
    let review = review.finish();
    if !review.large_files.is_empty() {
        return Err(
            "The plan contains files over 100 MiB. Stage them with Git LFS Manager first.".into(),
        );
    }
    if (!review.findings.is_empty() || !review.skipped.is_empty()) && !approved {
        return Err("Review the secret scan warnings before confirming.".into());
    }
    if let Some(command) = build_command {
        let verify = verification::Verification::new(command)?;
        for (i, commit) in commits.iter().enumerate() {
            let tree = snapshot
                .repo
                .find_commit(*commit)
                .and_then(|commit| commit.tree())
                .map_err(git_err)?;
            verify
                .check(&snapshot.repo, &tree, i + 1)
                .map_err(|error| format!("{} No commits were published.", error.message()))?;
        }
        // The working copy can change while an external build is running.
        if Snapshot::build(&Git::open(path)?, false)?.fingerprint(include_secrets) != expected {
            return Err(STALE.into());
        }
    }

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

    fn draft_with<'a>(plan: &'a Composition, path: &str) -> &'a Group {
        plan.groups
            .iter()
            .find(|group| {
                group
                    .units
                    .iter()
                    .flat_map(|unit| &plan.units[*unit].changes)
                    .any(|index| plan.changes[*index].path == path)
            })
            .unwrap_or_else(|| panic!("No draft contains {path}"))
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
    fn drafts_a_feature_across_layers_and_keeps_ambiguous_names_independent() {
        let f = Fixture::new(true);
        let controller = "src/orders/controllers/CheckoutController.cs";
        let service = "src/orders/services/CheckoutService.cs";
        let test = "tests/orders/CheckoutServiceTests.cs";
        let docs = "docs/orders/checkout-guide.md";
        let other = "src/admin/controllers/CheckoutController.cs";
        let ambiguous = "tests/CheckoutTests.cs";
        let separate_a = "src/orders/services/TaxRules.cs";
        let separate_b = "src/orders/services/DiscountRules.cs";
        for path in [
            controller, service, test, other, ambiguous, separate_a, separate_b,
        ] {
            f.write(path, "// Fixture\n");
        }
        f.write(docs, "# Checkout examples\n");
        let before = f.git(&["rev-parse", "HEAD"]);
        let preview = plan(f.path(), false).unwrap();
        let checkout = draft_with(&preview, controller);
        assert_eq!(checkout.units, draft_with(&preview, service).units);
        assert_eq!(checkout.units, draft_with(&preview, test).units);
        assert_eq!(checkout.units, draft_with(&preview, docs).units);
        assert_ne!(checkout.units, draft_with(&preview, other).units);
        assert_ne!(checkout.units, draft_with(&preview, ambiguous).units);
        assert_ne!(
            draft_with(&preview, separate_a).units,
            draft_with(&preview, separate_b).units
        );
        assert!(
            !together(&preview, controller, test),
            "review groups must remain editable"
        );
        assert!(checkout
            .description
            .contains("Keep the matching implementation"));
        assert!(checkout.subject.contains("checkout implementation"));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), before);
        f.apply(&preview);
        assert_eq!(f.git(&["status", "--porcelain"]), "");
    }

    #[test]
    fn drafts_ui_with_named_stories_and_locales_without_crossing_workspaces() {
        let f = Fixture::new(true);
        let screen = "apps/store/src/features/cart/CartScreen.tsx";
        let story = "apps/store/src/features/cart/CartScreen.stories.tsx";
        let hook = "apps/store/src/features/cart/hooks/useCart.ts";
        let style = "apps/store/src/features/cart/CartScreen.module.css";
        let locale = "apps/store/locales/hr/cart.json";
        let common = "apps/store/locales/hr/common.json";
        let admin = "apps/admin/locales/hr/cart.json";
        f.write(screen, "export function CartScreen() {}\n");
        f.write(story, "export const EmptyCart = {};\n");
        f.write(hook, "export function useCart() {}\n");
        f.write(style, ".empty { display: none; }\n");
        for path in [locale, common, admin] {
            f.write(path, "{\"title\":\"Cart\"}\n");
        }
        let preview = plan(f.path(), false).unwrap();
        let ui = draft_with(&preview, screen);
        assert_eq!(ui.units, draft_with(&preview, story).units);
        assert_eq!(ui.units, draft_with(&preview, hook).units);
        assert_eq!(ui.units, draft_with(&preview, style).units);
        assert_eq!(ui.units, draft_with(&preview, locale).units);
        assert_ne!(ui.units, draft_with(&preview, common).units);
        assert_ne!(ui.units, draft_with(&preview, admin).units);
    }

    #[test]
    fn drafts_a_migration_with_the_source_that_uses_its_exact_table() {
        let f = Fixture::new(true);
        let write = "src/data/InvoiceRepository.cs";
        let unrelated = "src/data/PaymentRepository.cs";
        let migration = "data/migrations/001.sql";
        f.write(
            write,
            "var sql = \"INSERT INTO billing.invoices (id) VALUES (@id)\";\n",
        );
        f.write(
            unrelated,
            "var sql = \"SELECT id FROM reporting.invoices WHERE id = @id\";\n",
        );
        f.write(
            migration,
            "ALTER TABLE billing.invoices ADD COLUMN number TEXT;\n",
        );
        let preview = plan(f.path(), false).unwrap();
        let invoice = draft_with(&preview, write);
        assert_eq!(invoice.units, draft_with(&preview, migration).units);
        assert_ne!(invoice.units, draft_with(&preview, unrelated).units);
        assert!(invoice
            .description
            .contains("Keep the migration with the implementation"));
        assert!(!together(&preview, write, migration));

        f.write(
            "src/reporting/ExportRepository.cs",
            "var sql = \"SELECT id FROM billing.invoices WHERE id = @id\";\n",
        );
        let preview = plan(f.path(), false).unwrap();
        assert_ne!(
            draft_with(&preview, write).units,
            draft_with(&preview, migration).units
        );
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
        f.git(&["add", "--", "app/settings.env", "src/app.py"]);
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
    fn drafts_contract_and_function_body_messages_from_staged_evidence() {
        let f = Fixture::new(true);
        let source = "core/rank.rs";
        f.write(
            source,
            "fn rank_candidate(left: i32, right: i32) -> i32 {\n    left + right\n}\n",
        );
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Add baseline"]);
        f.write(
            source,
            "fn rank_candidate(left: i32, right: i32) -> i32 {\n    left - right\n}\n",
        );
        f.write(
            "rpc/catalog.proto",
            "rpc ListCatalog(Request) returns (Response);\n",
        );
        f.write(
            "data/migrations/001.sql",
            "ALTER TABLE catalog ADD COLUMN alias VARCHAR(32) DEFAULT 'private-test-value';\n",
        );
        f.git(&["add", "."]);
        let proposed = plan(f.path(), false).unwrap();
        let group = |path| {
            proposed
                .groups
                .iter()
                .find(|group| {
                    group.units.iter().any(|id| {
                        proposed.units[*id]
                            .changes
                            .iter()
                            .any(|id| proposed.changes[*id].path == path)
                    })
                })
                .unwrap()
        };
        assert!(group(source).subject.contains("Revise rank candidate"));
        assert!(group(source)
            .description
            .contains("Changes the implementation in `rank_candidate`."));
        assert!(group("rpc/catalog.proto")
            .subject
            .contains("Expose endpoint ListCatalog"));
        let sql = &group("data/migrations/001.sql").description;
        assert!(sql.contains("Adds the column `alias`."), "{sql}");
        assert!(!sql.contains("private-test-value"));
        assert!(proposed
            .groups
            .iter()
            .all(|group| group.subject.encode_utf16().count() <= 72));
        f.apply(&proposed);
        assert_eq!(f.git(&["status", "--porcelain"]), "");
    }

    #[test]
    fn composes_behavior_from_hunks_and_splits_unrelated_features_in_one_folder() {
        let f = Fixture::new(true);
        f.write("src/ui/account.ts", "export function selectAccount(account) {\n  const initial = 1;\n  const next = initial;\n  const saved = next;\n  const value = saved;\n  return account.id + value;\n}\n");
        f.write(
            "src/ui/dashboard.js",
            "function bindDashboard(el) {\n  return el;\n}\n",
        );
        f.write(
            "client/request.py",
            "async def request_status():\n    return await wait_for(fetch(), 10)\n",
        );
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Add baseline"]);
        f.write("src/ui/account.ts", "export function selectAccount(account) {\n  const initial = 1;\n  const next = initial;\n  const saved = next;\n  const value = saved;\n  if (account == null) return;\n  return account.id + value;\n}\n");
        f.write("src/ui/dashboard.js", "function bindDashboard(el) {\n  el.addEventListener('click', refresh);\n  return el;\n}\n");
        f.write(
            "client/request.py",
            "async def request_status():\n    return await wait_for(fetch(), 30)\n",
        );
        f.git(&["add", "."]);
        let plan = plan(f.path(), false).unwrap();
        let group_for = |path: &str| {
            plan.groups
                .iter()
                .find(|group| {
                    group.units.iter().any(|id| {
                        plan.units[*id]
                            .changes
                            .iter()
                            .any(|file| plan.changes[*file].path == path)
                    })
                })
                .unwrap()
        };
        let account = group_for("src/ui/account.ts");
        let dashboard = group_for("src/ui/dashboard.js");
        let request = group_for("client/request.py");
        assert!(
            account
                .subject
                .contains("Add null checks in select account"),
            "{}",
            account.subject
        );
        assert!(
            account
                .description
                .starts_with("Adds null checks in `selectAccount`."),
            "{}",
            account.description
        );
        assert!(
            dashboard
                .subject
                .contains("Add event handlers in bind dashboard"),
            "{}",
            dashboard.subject
        );
        assert_ne!(account.units, dashboard.units);
        assert!(
            request
                .description
                .starts_with("Updates timeouts in `request_status`."),
            "{}",
            request.description
        );
        f.apply(&plan);
        assert_eq!(f.git(&["status", "--porcelain"]), "");
    }

    #[test]
    fn stages_independent_hunks_of_one_file_without_consuming_unstaged_bytes() {
        let f = Fixture::new(true);
        let baseline = "export function normalizeInput(value) {\n    return value.trim();\n}\n\n// context one\n// context two\n// context three\n// context four\n// context five\n// context six\n// context seven\n\nexport function configureTimer() {\n    return 1;\n}\n";
        f.write("src/features.ts", baseline);
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Baseline"]);
        let after = baseline
            .replace(
                "    return value.trim();",
                "    if (value == null) return '';\n    return value.trim();",
            )
            .replace(
                "    return 1;",
                "    setTimeout(runTask, 20);\n    return 1;",
            );
        f.write("src/features.ts", &after);
        f.git(&["add", "--", "src/features.ts"]);
        f.write(
            "src/features.ts",
            &(after.clone() + "// Keep this unstaged\n"),
        );
        let head = f.git(&["rev-parse", "HEAD"]);
        let plan = plan(f.path(), false).unwrap();
        assert_eq!(plan.changes.len(), 2);
        assert!(plan
            .changes
            .iter()
            .all(|file| file.chunk.is_some() && file.partial));
        assert_eq!(plan.groups.len(), 2);
        let commits = f.apply(&plan);
        let intermediate = f.git(&["show", &format!("{}:src/features.ts", commits[0])]);
        assert_ne!(
            intermediate.contains("setTimeout"),
            intermediate.contains("value == null")
        );
        assert_eq!(f.git(&["show", "HEAD:src/features.ts"]), after.trim_end());
        assert_eq!(f.git(&["diff", "--cached", "--name-only"]), "");
        assert!(f
            .git(&["diff", "--", "src/features.ts"])
            .contains("+// Keep this unstaged"));
        assert_eq!(f.git(&["rev-parse", "HEAD~2"]), head);
    }

    #[test]
    fn refuses_missing_local_dependency_and_unsafe_manual_export_order() {
        let f = Fixture::new(true);
        f.write("src/core.ts", "export function existing() {}\n\n// one\n// two\n// three\n// four\n// five\n// six\n// seven\n\nexport function keep() {\n  return 1;\n}\n");
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Baseline"]);
        f.write("src/core.ts", "export function existing() {}\nexport function newFeature() {}\n\n// one\n// two\n// three\n// four\n// five\n// six\n// seven\n\nexport function keep() {\n  return 2;\n}\n");
        f.write(
            "src/use.ts",
            "import { newFeature } from './core';\nnewFeature();\n",
        );
        f.write("docs/README.md", "# Document baseline\n");
        f.git(&["add", "."]);
        let proposed = plan(f.path(), false).unwrap();
        let consumer = proposed
            .units
            .iter()
            .find(|unit| {
                unit.changes
                    .iter()
                    .any(|id| proposed.changes[*id].path == "src/use.ts")
            })
            .unwrap();
        assert!(consumer
            .changes
            .iter()
            .any(|id| proposed.changes[*id].path == "src/core.ts"
                || proposed.changes[*id].path == "src/use.ts"));
        f.apply(&proposed);
        let original = f.git(&["rev-parse", "HEAD"]);
        f.write("src/missing.ts", "import './missing-dependency';\n");
        assert!(plan(f.path(), false)
            .err()
            .is_some_and(|error| error.contains("missing-dependency")));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
    }

    #[test]
    fn blocks_reordering_a_caller_before_a_new_export() {
        let f = Fixture::new(true);
        f.write("src/feature.ts", "export const keep = 1;\n");
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Baseline"]);
        f.write(
            "src/feature.ts",
            "export const keep = 1;\nexport const enabled = true;\n",
        );
        // Deliberately bypass the grouping heuristic so the commit-tree guard is tested directly.
        f.write("src/caller.ts", "import { enabled } from './feature';\n");
        f.git(&["add", "."]);
        let original = f.git(&["rev-parse", "HEAD"]);
        let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
        let units: Vec<_> = (0..snapshot.changes.len())
            .map(|id| Unit {
                id,
                changes: vec![id],
            })
            .collect();
        let caller = snapshot
            .changes
            .iter()
            .position(|file| file.path == "src/caller.ts")
            .unwrap();
        let provider = snapshot
            .changes
            .iter()
            .position(|file| file.path == "src/feature.ts")
            .unwrap();
        let make = |id| Draft {
            subject: "Refactor(core): Review dependency".into(),
            description: String::new(),
            units: vec![id],
        };
        let signature = Git::open(f.path()).unwrap().signature().unwrap();
        let error = create_series(
            &snapshot,
            &signature,
            &units,
            &[make(caller), make(provider)],
            None,
        )
        .unwrap_err();
        assert!(error.contains("imports enabled before"), "{error}");
        assert_eq!(
            create_series(
                &snapshot,
                &signature,
                &units,
                &[make(provider), make(caller)],
                None
            )
            .unwrap()
            .len(),
            2
        );
        assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
    }

    #[test]
    fn verifies_every_commit_before_moving_the_branch() {
        let f = Fixture::new(true);
        f.write("src/first.rs", "const VALUE: i32 = 7;\n");
        f.write("src/second.rs", "const FALLBACK: i32 = 9;\n");
        f.write("verify.rs", "include!(\"src/first.rs\");\nfn main() {}\n");
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Baseline"]);
        f.write("src/first.rs", "const VALUE: i32 = invalid_name;\n");
        f.write("src/second.rs", "const invalid_name: i32 = 7;\n");
        f.write("docs/guide.md", "# Guide\n");
        f.git(&["add", "."]);
        let original = f.git(&["rev-parse", "HEAD"]);
        let index = f.git(&["write-tree"]);
        let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
        let units = rules::atomic_units(&snapshot.changes);
        let unit = |path: &str| {
            units
                .iter()
                .position(|unit| {
                    unit.changes
                        .iter()
                        .any(|id| snapshot.changes[*id].path == path)
                })
                .unwrap()
        };
        let consumer = unit("src/first.rs");
        let provider = unit("src/second.rs");
        let docs = unit("docs/guide.md");
        assert_ne!(consumer, provider);
        let make = |units| Draft {
            subject: "Test(core): Check each staged snapshot".into(),
            description: String::new(),
            units,
        };
        #[cfg(windows)]
        let command = "copy /b src\\second.rs+verify.rs combined.rs >nul && rustc --crate-name verify_sample combined.rs";
        #[cfg(not(windows))]
        let command = "cat src/second.rs verify.rs > combined.rs && rustc --crate-name verify_sample combined.rs";
        let risky = [
            make(vec![consumer]),
            make(vec![provider]),
            Draft {
                subject: "Docs(guide): Add setup instructions".into(),
                description: "Describe how to run the project.\n\nCloses #123".into(),
                units: vec![docs],
            },
        ];
        let result = apply_with_verification(
            f.path(),
            &snapshot.fingerprint(false),
            &risky,
            false,
            false,
            Some(command),
        );
        let error = result.expect_err("an intermediate commit should not compile");
        assert!(
            error.contains("Commit 1: Build verification failed") && error.contains("invalid_name"),
            "{error}"
        );
        assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
        assert_eq!(f.git(&["write-tree"]), index);
        assert!(!f.0.path().join("combined.rs").exists());
        let failed = find_buildable_groups(
            f.path(),
            &snapshot.fingerprint(false),
            &risky,
            false,
            "exit 1",
        );
        assert!(
            matches!(failed, Err(message) if message.contains("Regrouping cannot fix the final tree"))
        );
        let repaired = find_buildable_groups(
            f.path(),
            &snapshot.fingerprint(false),
            &risky,
            false,
            command,
        )
        .unwrap();
        assert_eq!(repaired.groups.len(), 2);
        assert_eq!(repaired.groups[0].units, [consumer, provider]);
        assert!(repaired.groups[0]
            .description
            .contains("Keep these changes together"));
        assert!(repaired.groups[0]
            .description
            .lines()
            .all(|line| line.len() <= 72));
        assert_eq!(repaired.groups[1].subject, risky[2].subject);
        assert_eq!(repaired.groups[1].description, risky[2].description);
        assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
        assert_eq!(f.git(&["write-tree"]), index);
        let safe = repaired
            .groups
            .into_iter()
            .map(|group| Draft {
                subject: group.subject,
                description: group.description,
                units: group.units,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            apply_with_verification(
                f.path(),
                &snapshot.fingerprint(false),
                &safe,
                false,
                false,
                Some(command)
            )
            .unwrap()
            .len(),
            2
        );
        assert_eq!(f.git(&["rev-parse", "HEAD~2"]), original);
        assert_eq!(f.git(&["status", "--porcelain"]), "");
        assert!(!f.0.path().join("combined.rs").exists());
    }

    #[test]
    fn detects_local_asset_includes_and_unchanged_callers() {
        for (caller, program, dependency) in [
            (
                "src/main.rs",
                "const DEFAULT: &str = include_str!(\"default.toml\");\n",
                "src/default.toml",
            ),
            (
                "lib/Bootstrap.php",
                "<?php\nrequire_once(__DIR__ . '/Config.php');\n",
                "lib/Config.php",
            ),
            ("objc/View.m", "#import \"View.h\"\n", "objc/View.h"),
        ] {
            let f = Fixture::new(true);
            f.write(caller, "");
            f.git(&["add", "."]);
            f.git(&["commit", "-m", "Add baseline"]);
            f.write(caller, program);
            f.write(dependency, "text\n");
            f.git(&["add", "."]);
            let proposed = plan(f.path(), false).unwrap();
            let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
            let caller_unit = proposed
                .units
                .iter()
                .find(|unit| {
                    unit.changes
                        .iter()
                        .any(|id| proposed.changes[*id].path == caller)
                })
                .unwrap();
            if !caller_unit
                .changes
                .iter()
                .any(|id| proposed.changes[*id].path == dependency)
            {
                let mut series = Series::new(&snapshot).unwrap();
                let (id, _) = series.advance(&proposed.units, &[caller_unit.id]).unwrap();
                let mut check = dependencies::Check::new(
                    &snapshot.repo,
                    snapshot.head,
                    snapshot.tree,
                    &snapshot.changes,
                )
                .unwrap();
                assert!(
                    check
                        .verify(&snapshot.repo.find_tree(id).unwrap())
                        .is_some(),
                    "{caller}"
                );
            }
            f.apply(&proposed);
            f.git(&["rm", "--", dependency]);
            assert!(matches!(
                plan(f.path(), false),
                Err(message) if message.contains("missing a local dependency") && message.contains(caller)
            ));
        }
    }

    #[test]
    fn detects_missing_python_and_c_dependencies_between_git_trees() {
        let f = Fixture::new(true);
        f.write("tools/build.py", "from .settings import limit\n");
        f.write("tools/settings.py", "limit = 1\n");
        f.write("native/main.c", "#include \"features.h\"\n");
        f.write("native/features.h", "int select_feature(void);\n");
        f.git(&["add", "."]);
        let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
        let units: Vec<_> = (0..snapshot.changes.len())
            .map(|id| Unit {
                id,
                changes: vec![id],
            })
            .collect();
        let select = |path: &str| {
            vec![snapshot
                .changes
                .iter()
                .position(|file| file.path == path)
                .unwrap()]
        };
        let mut check = dependencies::Check::new(
            &snapshot.repo,
            snapshot.head,
            snapshot.tree,
            &snapshot.changes,
        )
        .unwrap();
        for (consumer, provider) in [
            ("tools/build.py", "tools/settings.py"),
            ("native/main.c", "native/features.h"),
        ] {
            let mut series = Series::new(&snapshot).unwrap();
            let first = series.advance(&units, &select(consumer)).unwrap().0;
            let problem = check
                .verify(&snapshot.repo.find_tree(first).unwrap())
                .unwrap()
                .to_string();
            assert!(problem.contains(consumer), "{problem}");
            let second = series.advance(&units, &select(provider)).unwrap().0;
            assert!(check
                .verify(&snapshot.repo.find_tree(second).unwrap())
                .is_none());
        }
    }

    #[test]
    fn checks_unchanged_callers_when_a_local_module_is_deleted() {
        let f = Fixture::new(true);
        f.write("src/caller.ts", "import { value } from './feature';\n");
        f.write("src/feature.ts", "export const value = 1;\n");
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Baseline"]);
        f.git(&["rm", "--", "src/feature.ts"]);
        let error = plan(f.path(), false)
            .err()
            .expect("an unchanged caller still imports this module");
        assert!(
            error.contains("src/caller.ts") && error.contains("src/feature"),
            "{error}"
        );
    }

    #[test]
    fn combines_the_provider_with_its_caller_without_absorbing_unrelated_steps() {
        let f = Fixture::new(true);
        f.write("src/feature.ts", "export const keep = 1;\n");
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Baseline"]);
        f.write(
            "src/feature.ts",
            "export const keep = 1;\nexport const enabled = true;\n",
        );
        f.write("src/caller.ts", "import { enabled } from './feature';\n");
        f.write("docs/guide.md", "# Describe a separate workflow\n");
        f.git(&["add", "."]);
        let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
        let units: Vec<_> = (0..snapshot.changes.len())
            .map(|id| Unit {
                id,
                changes: vec![id],
            })
            .collect();
        let index = |path: &str| {
            snapshot
                .changes
                .iter()
                .position(|file| file.path == path)
                .unwrap()
        };
        let (caller, docs, provider) = (
            index("src/caller.ts"),
            index("docs/guide.md"),
            index("src/feature.ts"),
        );
        let mut groups: Vec<_> = [caller, docs, provider]
            .into_iter()
            .map(|id| rules::combined(&snapshot.changes, &units, vec![id]))
            .collect();
        let mut check = dependencies::Check::new(
            &snapshot.repo,
            snapshot.head,
            snapshot.tree,
            &snapshot.changes,
        )
        .unwrap();
        let (notice, _) =
            arrange_groups(&snapshot, &units, &mut groups, &mut check, false).unwrap();
        assert!(notice.is_some());
        assert_eq!(
            groups
                .iter()
                .map(|group| group.units.clone())
                .collect::<Vec<_>>(),
            [vec![caller, provider], vec![docs]]
        );
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

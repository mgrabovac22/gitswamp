use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use git2::{
    Commit, Delta, DiffFindOptions, DiffOptions, Index, IndexEntry, IndexTime, Oid, Repository,
    Signature, Tree,
};
use serde::{Deserialize, Serialize};

use super::secret_service::{self, CompositionSecrets, OwnedLock};
use crate::repositories::git_repository::GitRepository as Git;

const MAX_INDEX: u64 = 16 * 1024 * 1024;
const STALE: &str = "HEAD or the staged files changed. Review the amend selection again.";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    path: String,
    old_path: Option<String>,
    status: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Removal {
    pub head: String,
    pub paths: Vec<String>,
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn base<'repo>(head: &Commit<'repo>) -> Result<Option<Tree<'repo>>, String> {
    match head.parent_count() {
        0 => Ok(None),
        1 => head.parent(0).and_then(|parent| parent.tree()).map(Some).map_err(err),
        _ => Err("Moving files out of a merge commit is not supported. Its parents have different versions.".into()),
    }
}

fn changed(
    repo: &Repository,
    parent: Option<&Tree<'_>>,
    head: &Tree<'_>,
) -> Result<Vec<Choice>, String> {
    let mut options = DiffOptions::new();
    options.skip_binary_check(true).include_typechange(true);
    let mut diff = repo
        .diff_tree_to_tree(parent, Some(head), Some(&mut options))
        .map_err(err)?;
    diff.find_similar(Some(
        DiffFindOptions::new().renames(true).exact_match_only(true),
    ))
    .map_err(err)?;
    diff.deltas()
        .map(|delta| {
            let path = std::str::from_utf8(
                delta
                    .new_file()
                    .path_bytes()
                    .or_else(|| delta.old_file().path_bytes())
                    .ok_or("Missing file path.")?,
            )
            .map_err(|_| "File paths must be UTF-8 for a selective amend.")?
            .to_owned();
            let old_path = if delta.status() == Delta::Renamed {
                Some(
                    std::str::from_utf8(
                        delta
                            .old_file()
                            .path_bytes()
                            .ok_or("Missing source path.")?,
                    )
                    .map_err(|_| "Rename path must be UTF-8.")?
                    .to_owned(),
                )
            } else {
                None
            };
            let status = match delta.status() {
                Delta::Added => "added",
                Delta::Deleted => "deleted",
                Delta::Renamed => "renamed",
                _ => "modified",
            };
            Ok(Choice {
                path,
                old_path,
                status,
            })
        })
        .collect()
}

pub fn list(path: &str, expected: &str) -> Result<Vec<Choice>, String> {
    let repo = Git::open(path)?;
    let head = repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(err)?;
    if head.id().to_string() != expected {
        return Err(STALE.into());
    }
    let parent = base(&head)?;
    let tree = head.tree().map_err(err)?;
    changed(&repo, parent.as_ref(), &tree)
}

fn overlaps(a: &str, b: &str) -> bool {
    a == b
        || a.strip_prefix(b).is_some_and(|tail| tail.starts_with('/'))
        || b.strip_prefix(a).is_some_and(|tail| tail.starts_with('/'))
}

pub(super) struct Prepared {
    index_path: PathBuf,
    before: Vec<u8>,
    prepared_index: tempfile::TempDir,
    reference: String,
    head: Oid,
    pub(super) tree: Oid,
}

fn referent(repo: &Repository) -> Result<(String, Oid), String> {
    let head = repo.head().map_err(err)?;
    let reference = if head.is_branch() {
        head.name().ok_or("Invalid branch reference.")?.to_owned()
    } else {
        "HEAD".into()
    };
    Ok((reference, head.target().ok_or("Invalid HEAD.")?))
}

impl Prepared {
    pub(super) fn new(
        path: &str,
        repo: &Repository,
        head: &Commit<'_>,
        removal: Removal,
        secret_check: bool,
        secret_approval: Option<&str>,
    ) -> Result<Self, String> {
        let (reference, id) = referent(repo)?;
        if id != head.id() || id.to_string() != removal.head {
            return Err(STALE.into());
        }
        let live = repo.index().map_err(err)?;
        if live.has_conflicts() {
            return Err("Resolve conflicts before amending.".into());
        }
        let index_path = live
            .path()
            .ok_or("The working copy has no file-backed index.")?
            .to_path_buf();
        let before =
            Git::index_bytes(&index_path, MAX_INDEX)?.ok_or("The Git index is missing.")?;
        let head_tree = head.tree().map_err(err)?;
        let base = base(head)?;
        let changes = changed(repo, base.as_ref(), &head_tree)?;
        let chosen: BTreeSet<_> = removal.paths.iter().map(String::as_str).collect();
        let mut paths = BTreeSet::new();
        for choice in changes
            .iter()
            .filter(|choice| chosen.contains(choice.path.as_str()))
        {
            paths.insert(choice.path.as_str());
            paths.extend(choice.old_path.as_deref());
        }
        if chosen.is_empty() || !chosen.is_subset(&paths) {
            return Err("Select files listed in the last commit.".into());
        }
        // Refuse a partial directory/file replacement instead of silently removing neighbouring changes.
        for path in changes
            .iter()
            .flat_map(|choice| [Some(choice.path.as_str()), choice.old_path.as_deref()])
            .flatten()
        {
            if !paths.contains(path) && paths.iter().any(|p| overlaps(p, path)) {
                return Err(format!(
                    "Select the related path {path} too, or leave both in the commit."
                ));
            }
        }
        let mut options = DiffOptions::new();
        options.skip_binary_check(true).include_typechange(true);
        let staged = repo
            .diff_tree_to_index(Some(&head_tree), Some(&live), Some(&mut options))
            .map_err(err)?;
        for delta in staged.deltas() {
            for file in [delta.old_file(), delta.new_file()] {
                if let Some(staged_path) = file
                    .path_bytes()
                    .and_then(|path| std::str::from_utf8(path).ok())
                {
                    if paths.iter().any(|path| overlaps(path, staged_path)) {
                        return Err(format!("Unstage {staged_path} before moving it out of the last commit. Its working file will be kept."));
                    }
                }
            }
        }
        secret_service::validate_commit(path, secret_check, secret_approval)?;
        let prepared_index = tempfile::tempdir_in(repo.path()).map_err(err)?;
        let after = prepared_index.path().join("index");
        fs::write(&after, &before).map_err(err)?;
        let mut index = Index::open(&after).map_err(err)?;
        for path in &paths {
            if index.get_path(Path::new(path), 0).is_some() {
                index.remove_path(Path::new(path)).map_err(err)?;
            }
        }
        let odb = repo.odb().map_err(err)?;
        let mut review = CompositionSecrets::new(&removal.head, secret_check);
        for path in &paths {
            let Some(entry) = base
                .as_ref()
                .and_then(|tree| tree.get_path(Path::new(path)).ok())
                .filter(|entry| entry.kind() != Some(git2::ObjectType::Tree))
            else {
                continue;
            };
            if entry.filemode() != 0o160000 {
                let (size, _) = odb.read_header(entry.id()).map_err(err)?;
                review.inspect(repo, path, entry.id(), size, true);
            }
            index
                .add(&IndexEntry {
                    ctime: IndexTime::new(0, 0),
                    mtime: IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: entry.filemode() as u32,
                    uid: 0,
                    gid: 0,
                    file_size: 0,
                    id: entry.id(),
                    flags: 0,
                    flags_extended: 0,
                    path: path.as_bytes().to_vec(),
                })
                .map_err(err)?;
        }
        let review = review.finish();
        if !review.large_files.is_empty()
            || !review.findings.is_empty()
            || !review.skipped.is_empty()
        {
            return Err("The parent versions would restore oversized files or content that needs secret review. No staged files were changed.".into());
        }
        let tree = index.write_tree_to(repo).map_err(err)?;
        index.write().map_err(err)?;
        Ok(Self {
            index_path,
            before,
            prepared_index,
            reference,
            head: id,
            tree,
        })
    }

    pub(super) fn publish(
        self,
        repo: &Repository,
        commit: Oid,
        signature: &Signature<'_>,
    ) -> Result<(), String> {
        let lock_path = self.index_path.with_file_name("index.lock");
        let mut index_lock =
            OwnedLock::create(lock_path.clone(), &[]).map_err(|_| "The Git index is locked.")?;
        let mut refs = repo.transaction().map_err(err)?;
        refs.lock_ref("HEAD").map_err(err)?;
        if self.reference != "HEAD" {
            refs.lock_ref(&self.reference).map_err(err)?;
        }
        if referent(repo)? != (self.reference.clone(), self.head)
            || Git::index_bytes(&self.index_path, MAX_INDEX)?.as_deref()
                != Some(self.before.as_slice())
        {
            return Err(STALE.into());
        }
        refs.set_target(
            &self.reference,
            commit,
            Some(signature),
            "commit: amend (selected files)",
        )
        .map_err(err)?;
        let after = self.prepared_index.path().join("index");
        fs::copy(&after, &lock_path).map_err(err)?;
        fs::OpenOptions::new()
            .write(true)
            .open(&lock_path)
            .and_then(|file| file.sync_all())
            .map_err(err)?;
        index_lock.install(&self.index_path)?;
        if let Err(failure) = refs.commit() {
            let rollback = (|| {
                let mut lock = OwnedLock::create(lock_path, &self.before)?;
                if Git::index_bytes(&self.index_path, MAX_INDEX)?
                    != Git::index_bytes(&after, MAX_INDEX)?
                {
                    return Err(STALE.into());
                }
                lock.install(&self.index_path)
            })();
            if rollback.is_err() {
                return Err(format!("Could not update the branch ({failure}) or restore the index. Review Working Changes; no working files were overwritten."));
            }
            return Err(err(failure));
        }
        Ok(())
    }
}

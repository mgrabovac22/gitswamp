use std::fs;

use git2::{Oid, Repository, Signature};

use super::super::secret_service::OwnedLock;
use super::{git_err, head, Snapshot, MAX_INDEX, STALE};
use crate::repositories::git_repository::GitRepository as Git;

pub(super) fn publish(
    repo: &Repository,
    snapshot: &Snapshot,
    signature: &Signature<'_>,
    commits: &[Oid],
) -> Result<(), String> {
    let lock_path = snapshot.index_path.with_file_name("index.lock");
    let mut index_lock = OwnedLock::create(lock_path.clone(), &[])
        .map_err(|_| "The Git index is locked. Wait for the other operation and try again.")?;
    let mut refs = repo.transaction().map_err(git_err)?;
    refs.lock_ref("HEAD")
        .and_then(|_| refs.lock_ref(&snapshot.branch))
        .map_err(git_err)?;
    if head(repo)? != (snapshot.branch.clone(), snapshot.head)
        || Git::index_bytes(&snapshot.index_path, MAX_INDEX)? != snapshot.original_index
    {
        return Err(STALE.into());
    }
    Git::publish_objects(&snapshot.repo.path().join("objects"), &snapshot.objects)?;
    refs.set_target(
        &snapshot.branch,
        *commits.last().ok_or("Empty commit plan.")?,
        Some(signature),
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
                if Git::index_bytes(&snapshot.index_path, MAX_INDEX)?
                    != Git::index_bytes(&snapshot.temp.path().join("index"), MAX_INDEX)?
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
    Ok(())
}

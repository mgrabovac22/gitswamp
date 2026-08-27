use crate::models::{CommitFileInfo, StashInfo};
use crate::repositories::git_repository::GitRepository;

pub struct StashService;

pub const PULL_SAFETY_STASH_PREFIX: &str = "GitSwamp pull safety ";

impl StashService {
    pub fn stash_list(path: &str) -> Result<Vec<StashInfo>, String> {
        let mut repo = GitRepository::open(path)?;
        let mut raw: Vec<(usize, String, String)> = Vec::new();

        if repo
            .stash_foreach(|index, name, oid| {
                raw.push((index, name.to_string(), oid.to_string()));
                true
            })
            .is_err()
        {
            return Ok(Vec::new());
        }

        let mut stashes = Vec::with_capacity(raw.len());
        for (index, stash_name, oid_str) in raw {
            let parent_sha = git2::Oid::from_str(&oid_str)
                .ok()
                .and_then(|oid| repo.find_commit(oid).ok())
                .and_then(|c| c.parent_id(0).ok())
                .map(|oid| oid.to_string())
                .unwrap_or_default();

            let (branch, message) = if let Some(after) = stash_name
                .strip_prefix("WIP on ")
                .or_else(|| stash_name.strip_prefix("On "))
            {
                if let Some(colon_pos) = after.find(": ") {
                    (
                        after[..colon_pos].to_string(),
                        after[colon_pos + 2..].to_string(),
                    )
                } else {
                    (String::new(), stash_name.clone())
                }
            } else {
                (String::new(), stash_name.clone())
            };

            stashes.push(StashInfo {
                index,
                message,
                branch,
                timestamp: String::new(),
                parent_sha,
            });
        }

        Ok(stashes)
    }

    fn stash_signature(repo: &git2::Repository) -> Result<git2::Signature<'static>, String> {
        repo.signature()
            .or_else(|_| git2::Signature::now("GitSwamp", "gitswamp@local"))
            .map_err(|e| e.message().to_string())
    }

    pub fn stash_push_oid(
        path: &str,
        message: Option<&str>,
        include_untracked: bool,
    ) -> Result<git2::Oid, String> {
        let mut repo = GitRepository::open(path)?;
        let sig = Self::stash_signature(&repo)?;
        let msg = message.unwrap_or("WIP");
        let flags = include_untracked.then_some(git2::StashFlags::INCLUDE_UNTRACKED);

        repo.stash_save(&sig, msg, flags)
            .map_err(|error| format!("Failed to create stash: {}", error.message()))
    }

    pub fn stash_push(
        path: &str,
        message: Option<&str>,
        include_untracked: bool,
    ) -> Result<String, String> {
        let oid = Self::stash_push_oid(path, message, include_untracked)?;
        let scope = if include_untracked {
            "tracked and untracked changes"
        } else {
            "tracked changes"
        };

        Ok(format!("Saved {} as stash {}", scope, oid))
    }

    fn stash_index_for_oid(repo: &mut git2::Repository, oid: git2::Oid) -> Result<usize, String> {
        let mut found = None;
        repo.stash_foreach(|index, _name, candidate| {
            if *candidate == oid {
                found = Some(index);
                false
            } else {
                true
            }
        })
        .map_err(|e| e.message().to_string())?;

        found.ok_or_else(|| format!("Safety stash {} was not found", oid))
    }

    pub fn latest_stash_oid_with_message_prefix(
        path: &str,
        message_prefix: &str,
    ) -> Result<Option<git2::Oid>, String> {
        let mut repo = GitRepository::open(path)?;
        let mut found = None;
        repo.stash_foreach(|_, name, oid| {
            if name.contains(message_prefix) {
                found = Some(*oid);
                false
            } else {
                true
            }
        })
        .map_err(|e| e.message().to_string())?;
        Ok(found)
    }

    pub fn restore_stash_with_index(path: &str, oid: git2::Oid) -> Result<String, String> {
        let mut repo = GitRepository::open(path)?;
        let index = Self::stash_index_for_oid(&mut repo, oid)?;
        let mut opts = git2::StashApplyOptions::new();
        opts.reinstantiate_index();

        repo.stash_pop(index, Some(&mut opts)).map_err(|error| {
            format!(
                "Failed to restore safety stash@{{{}}}: {}. The stash was kept.",
                index,
                error.message()
            )
        })?;

        Ok(format!(
            "Restored safety stash {} with its staged state",
            oid
        ))
    }

    pub fn stash_pop(path: &str, index: usize) -> Result<String, String> {
        let mut repo = GitRepository::open(path)?;
        let mut opts = git2::StashApplyOptions::new();

        repo.stash_pop(index, Some(&mut opts)).map_err(|e| {
            let msg = e.message();
            if msg.contains("conflict") {
                format!(
                    "Stash pop failed due to conflicts: {}. Use stash_apply instead to inspect changes.",
                    msg
                )
            } else if msg.contains("not found") {
                format!("Stash at index {} not found", index)
            } else {
                format!("Failed to pop stash: {}", msg)
            }
        })?;
        Ok(format!("Dropped and applied stash@{{{}}}", index))
    }

    pub fn stash_apply(path: &str, index: usize) -> Result<String, String> {
        let mut repo = GitRepository::open(path)?;
        let mut opts = git2::StashApplyOptions::new();

        repo.stash_apply(index, Some(&mut opts)).map_err(|e| {
            let msg = e.message();
            if msg.contains("conflict") {
                format!(
                    "Stash apply created conflicts. Resolve them and use ConflictResolver. Error: {}",
                    msg
                )
            } else if msg.contains("not found") {
                format!("Stash at index {} not found", index)
            } else {
                format!("Failed to apply stash: {}", msg)
            }
        })?;
        Ok(format!("Applied stash@{{{}}}", index))
    }

    pub fn stash_drop(path: &str, index: usize) -> Result<String, String> {
        let mut repo = GitRepository::open(path)?;
        repo.stash_drop(index)
            .map_err(|e| e.message().to_string())?;
        Ok(format!("Dropped stash@{{{}}}", index))
    }

    pub fn stash_files(path: &str, index: usize) -> Result<Vec<CommitFileInfo>, String> {
        let mut repo = GitRepository::open(path)?;

        let mut stash_oid: Option<git2::Oid> = None;
        let target_index = index;

        repo.stash_foreach(|idx, _name, oid| {
            if idx == target_index {
                stash_oid = Some(*oid);
                false
            } else {
                true
            }
        })
        .map_err(|e| e.message().to_string())?;

        let oid = stash_oid.ok_or_else(|| format!("Stash at index {} not found", index))?;
        let stash_commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;

        let stash_tree = stash_commit.tree().map_err(|e| e.message().to_string())?;

        let parent_tree = if stash_commit.parent_count() > 0 {
            Some(
                stash_commit
                    .parent(0)
                    .map_err(|e| e.message().to_string())?
                    .tree()
                    .map_err(|e| e.message().to_string())?,
            )
        } else {
            None
        };

        let diff = repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&stash_tree), None)
            .map_err(|e| e.message().to_string())?;

        let n = diff.deltas().len();
        let mut files = Vec::with_capacity(n);
        for idx in 0..n {
            let delta = diff.get_delta(idx).unwrap();
            let file_path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let status = match delta.status() {
                git2::Delta::Added => "added",
                git2::Delta::Deleted => "deleted",
                git2::Delta::Modified => "modified",
                git2::Delta::Renamed => "renamed",
                git2::Delta::Copied => "copied",
                _ => "changed",
            };
            let (additions, deletions) = match git2::Patch::from_diff(&diff, idx) {
                Ok(Some(patch)) => patch.line_stats().map(|(_, a, d)| (a, d)).unwrap_or((0, 0)),
                _ => (0, 0),
            };
            files.push(CommitFileInfo {
                path: file_path,
                status: status.to_string(),
                additions,
                deletions,
            });
        }
        Ok(files)
    }
}

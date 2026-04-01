use crate::models::{CommitFileInfo, StashInfo};
use crate::repositories::git_repository::GitRepository;

pub struct StashService;

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

    pub fn stash_push(path: &str, message: Option<&str>) -> Result<String, String> {
        let mut repo = GitRepository::open(path)?;

        let sig = repo
            .signature()
            .or_else(|_| git2::Signature::now("GitSwamp", "gitswamp@local"))
            .map_err(|e| e.message().to_string())?;

        let msg = message.unwrap_or("WIP");

        match repo.stash_save(&sig, msg, Some(git2::StashFlags::INCLUDE_UNTRACKED)) {
            Ok(oid) => Ok(format!("Saved stash {}", oid)),
            Err(include_err) => {
                let include_msg = include_err.message().to_string();

                let fallback = repo
                    .stash_save(&sig, msg, None)
                    .map_err(|fallback_err| {
                        format!(
                            "Failed to stash tracked + untracked changes: {}. Fallback (tracked only) also failed: {}",
                            include_msg,
                            fallback_err.message()
                        )
                    })?;

                Ok(format!(
                    "Saved stash {} (tracked files only; untracked files were skipped due to repository size)",
                    fallback
                ))
            }
        }
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
        repo.stash_drop(index).map_err(|e| e.message().to_string())?;
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

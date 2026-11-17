use std::collections::HashSet;

use crate::models::{CommitFileInfo, StashInfo};
use crate::repositories::git_repository::GitRepository;

mod selection;

pub struct StashService;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionResult {
    message: String,
    remainder: Option<String>,
}

pub const PULL_SAFETY_STASH_PREFIX: &str = "GitSwamp pull safety ";

impl StashService {
    pub fn stash_list(path: &str) -> Result<Vec<StashInfo>, String> {
        let mut repo = GitRepository::open(path)?;
        let mut raw: Vec<(usize, String, git2::Oid)> = Vec::new();

        if repo
            .stash_foreach(|index, name, oid| {
                raw.push((index, name.to_string(), *oid));
                true
            })
            .is_err()
        {
            return Ok(Vec::new());
        }

        let mut stashes = Vec::with_capacity(raw.len());
        for (index, stash_name, oid) in raw {
            let parent_sha = repo
                .find_commit(oid)
                .ok()
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
                stash_sha: oid.to_string(),
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

    fn collect_diff_files(
        diff: &git2::Diff,
        odb: &git2::Odb,
        budget: &mut usize,
        files: &mut Vec<CommitFileInfo>,
        seen: &mut HashSet<String>,
        force_added: bool,
    ) {
        for (idx, delta) in diff.deltas().enumerate() {
            let file_path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            if file_path.is_empty() || !seen.insert(file_path.clone()) {
                continue;
            }

            let status = if force_added {
                "added"
            } else {
                match delta.status() {
                    git2::Delta::Added => "added",
                    git2::Delta::Deleted => "deleted",
                    git2::Delta::Modified => "modified",
                    git2::Delta::Renamed => "renamed",
                    git2::Delta::Copied => "copied",
                    _ => "changed",
                }
            };

            let size = |file: git2::DiffFile| {
                if file.id().is_zero() {
                    Some(0)
                } else {
                    odb.read_header(file.id()).ok().map(|(size, _)| size)
                }
            };
            let mut stats = (0, 0);
            if *budget >= 1024 {
                if let Some(total) = size(delta.old_file())
                    .zip(size(delta.new_file()))
                    .map(|(a, b)| a.saturating_add(b).max(1024))
                {
                    if total <= 128 * 1024 && total <= *budget {
                        *budget -= total;
                        if let Ok(Some(patch)) = git2::Patch::from_diff(diff, idx) {
                            stats = patch
                                .line_stats()
                                .map(|(_, a, d)| (a, d))
                                .unwrap_or_default();
                        }
                    }
                }
            }

            files.push(CommitFileInfo {
                path: file_path,
                status: status.to_string(),
                additions: stats.0,
                deletions: stats.1,
            });
        }
    }

    pub fn restore_selected(
        path: &str,
        index: usize,
        sha: &str,
        paths: Vec<String>,
        pop: bool,
    ) -> Result<SelectionResult, String> {
        selection::restore(path, index, sha, paths, pop)
    }

    pub fn stash_files(
        path: &str,
        index: usize,
        sha: Option<&str>,
    ) -> Result<Vec<CommitFileInfo>, String> {
        let repo = GitRepository::open(path)?;
        let oid = selection::open_stash(&repo, index, sha)?;
        let stash_commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        let stash_tree = stash_commit.tree().map_err(|e| e.message().to_string())?;
        let odb = repo.odb().map_err(|e| e.message().to_string())?;
        let mut budget = 1024 * 1024;
        let mut options = git2::DiffOptions::new();
        options.skip_binary_check(true).include_typechange(true);

        let mut files: Vec<CommitFileInfo> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        if stash_commit.parent_count() > 0 {
            let parent_tree = stash_commit
                .parent(0)
                .map_err(|e| e.message().to_string())?
                .tree()
                .map_err(|e| e.message().to_string())?;
            let diff = repo
                .diff_tree_to_tree(Some(&parent_tree), Some(&stash_tree), Some(&mut options))
                .map_err(|e| e.message().to_string())?;
            Self::collect_diff_files(&diff, &odb, &mut budget, &mut files, &mut seen, false);
            if let Ok(index_tree) = stash_commit.parent(1).and_then(|commit| commit.tree()) {
                if index_tree.id() != stash_tree.id() {
                    let diff = repo
                        .diff_tree_to_tree(
                            Some(&parent_tree),
                            Some(&index_tree),
                            Some(&mut options),
                        )
                        .map_err(|e| e.message().to_string())?;
                    Self::collect_diff_files(
                        &diff,
                        &odb,
                        &mut budget,
                        &mut files,
                        &mut seen,
                        false,
                    );
                }
            }
        }

        if stash_commit.parent_count() >= 3 {
            if let Ok(untracked_commit) = stash_commit.parent(2) {
                if let Ok(untracked_tree) = untracked_commit.tree() {
                    let diff = repo
                        .diff_tree_to_tree(None, Some(&untracked_tree), Some(&mut options))
                        .map_err(|e| e.message().to_string())?;
                    Self::collect_diff_files(&diff, &odb, &mut budget, &mut files, &mut seen, true);
                }
            }
        }

        Ok(files)
    }
}

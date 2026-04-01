use std::path::Path;

use crate::constants::{CONFLICT_END, CONFLICT_MID, CONFLICT_START};
use crate::models::FileDiff;
use crate::repositories::git_repository::GitRepository;
use crate::services::helpers::extract_file_diff;

pub struct DiffService;

impl DiffService {
    pub fn get_working_diff(path: &str, file_path: &str, staged: bool) -> Result<FileDiff, String> {
        let repo = GitRepository::open(path)?;
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.context_lines(3).interhunk_lines(1);

        let diff = if staged {
            let head_tree = repo.head().and_then(|h| h.peel_to_tree()).ok();
            repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_opts))
                .map_err(|e| e.message().to_string())?
        } else {
            repo.diff_index_to_workdir(None, Some(&mut diff_opts))
                .map_err(|e| e.message().to_string())?
        };

        extract_file_diff(&diff, file_path)
    }

    pub fn get_commit_diff(path: &str, sha: &str, file_path: &str) -> Result<FileDiff, String> {
        let repo = GitRepository::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        let tree = commit.tree().map_err(|e| e.message().to_string())?;
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.context_lines(3).interhunk_lines(1);

        let parent_tree = commit.parent(0).and_then(|p| p.tree()).ok();

        let diff = repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))
            .map_err(|e| e.message().to_string())?;

        extract_file_diff(&diff, file_path)
    }

    pub fn get_file_content(path: &str, file_path: &str, sha: Option<&str>) -> Result<String, String> {
        let repo = GitRepository::open(path)?;

        if let Some(commit_sha) = sha {
            let oid = git2::Oid::from_str(commit_sha).map_err(|e| e.message().to_string())?;
            let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
            let tree = commit.tree().map_err(|e| e.message().to_string())?;
            let entry = tree
                .get_path(Path::new(file_path))
                .map_err(|_| format!("File '{}' not found in commit", file_path))?;
            let blob = repo.find_blob(entry.id()).map_err(|e| e.message().to_string())?;
            if blob.is_binary() {
                return Err("Binary file".to_string());
            }
            String::from_utf8(blob.content().to_vec())
                .map_err(|_| "File is not valid UTF-8".to_string())
        } else {
            let full_path = Path::new(path).join(file_path);
            std::fs::read_to_string(&full_path).map_err(|e| e.to_string())
        }
    }

    pub fn get_staged_file_content(path: &str, file_path: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let index = repo.index().map_err(|e| e.message().to_string())?;
        let entry = index
            .get_path(Path::new(file_path), 0)
            .ok_or_else(|| format!("File '{}' not found in index", file_path))?;
        let blob = repo.find_blob(entry.id).map_err(|e| e.message().to_string())?;

        if blob.is_binary() {
            return Err("Binary file".to_string());
        }

        String::from_utf8(blob.content().to_vec()).map_err(|_| "File is not valid UTF-8".to_string())
    }

    pub fn has_conflict_markers(path: &str, file_path: &str) -> Result<bool, String> {
        let full_path = Path::new(path).join(file_path);
        let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
        Ok(content.contains(CONFLICT_START) && content.contains(CONFLICT_MID) && content.contains(CONFLICT_END))
    }

    pub fn save_file_content(path: &str, file_path: &str, content: &str) -> Result<(), String> {
        let full_path = Path::new(path).join(file_path);
        std::fs::write(&full_path, content).map_err(|e| e.to_string())
    }

    pub fn revert_hunk(path: &str, file_path: &str, hunk_index: usize, staged: bool) -> Result<(), String> {
        let diff_info = Self::get_working_diff(path, file_path, staged)?;

        if hunk_index >= diff_info.hunks.len() {
            return Err(format!("Hunk index {} out of range", hunk_index));
        }

        let hunk = &diff_info.hunks[hunk_index];

        let full_path = Path::new(path).join(file_path);
        let current_content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let current_lines: Vec<&str> = current_content.lines().collect();

        let mut result_lines: Vec<String> = Vec::new();
        let mut current_idx: usize = 0;

        let hunk_start = (hunk.new_start as usize).saturating_sub(1);

        while current_idx < hunk_start && current_idx < current_lines.len() {
            result_lines.push(current_lines[current_idx].to_string());
            current_idx += 1;
        }

        for line in &hunk.lines {
            match line.line_type.as_str() {
                "deletion" => {
                    result_lines.push(line.content.trim_end_matches('\n').to_string());
                }
                "addition" => {
                    current_idx += 1;
                }
                "context" => {
                    if current_idx < current_lines.len() {
                        result_lines.push(current_lines[current_idx].to_string());
                    }
                    current_idx += 1;
                }
                _ => {}
            }
        }

        while current_idx < current_lines.len() {
            result_lines.push(current_lines[current_idx].to_string());
            current_idx += 1;
        }

        let new_content = result_lines.join("\n") + if current_content.ends_with('\n') { "\n" } else { "" };
        std::fs::write(&full_path, new_content).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }
}

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::{CONFLICT_END, CONFLICT_MID, CONFLICT_START};
use crate::models::{DiffHunk, DiffLine, FileBlameLine, FileDiff};
use crate::repositories::git_repository::GitRepository;
use crate::services::helpers::extract_file_diff;

const UNCOMMITTED_SHA: &str = "0000000000000000000000000000000000000000";

#[derive(Clone)]
struct BlameCommitMeta {
    commit_sha: String,
    short_sha: String,
    author: String,
    author_email: String,
    summary: String,
    author_time: i64,
    is_uncommitted: bool,
}

pub struct DiffService;

impl DiffService {
    fn build_untracked_file_diff(repo_root: &Path, file_path: &str) -> Result<FileDiff, String> {
        let full_path = repo_root.join(file_path);
        let content = std::fs::read(&full_path).map_err(|e| e.to_string())?;

        if content.contains(&0) {
            return Ok(FileDiff {
                path: file_path.to_string(),
                old_path: None,
                status: "added".to_string(),
                hunks: Vec::new(),
                is_binary: true,
            });
        }

        let content = match String::from_utf8(content) {
            Ok(value) => value,
            Err(_) => {
                return Ok(FileDiff {
                    path: file_path.to_string(),
                    old_path: None,
                    status: "added".to_string(),
                    hunks: Vec::new(),
                    is_binary: true,
                });
            }
        };

        let lines: Vec<DiffLine> = content
            .split_inclusive('\n')
            .enumerate()
            .map(|(idx, line)| DiffLine {
                line_type: "addition".to_string(),
                old_line_no: None,
                new_line_no: Some((idx + 1) as u32),
                content: line.to_string(),
            })
            .collect();

        let new_lines = lines.len() as u32;
        let hunks = if new_lines == 0 {
            Vec::new()
        } else {
            vec![DiffHunk {
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines,
                header: format!("@@ -0,0 +1,{} @@", new_lines),
                lines,
            }]
        };

        Ok(FileDiff {
            path: file_path.to_string(),
            old_path: None,
            status: "added".to_string(),
            hunks,
            is_binary: false,
        })
    }

    fn patch_side_path(prefix: &str, path: &str) -> String {
        if path == "/dev/null" {
            "/dev/null".to_string()
        } else {
            format!("{}/{}", prefix, path)
        }
    }

    fn hunk_line_prefix(line_type: &str) -> Option<char> {
        match line_type {
            "addition" => Some('+'),
            "deletion" => Some('-'),
            "context" => Some(' '),
            _ => None,
        }
    }

    fn build_single_hunk_patch(
        diff_info: &FileDiff,
        file_path: &str,
        hunk_index: usize,
    ) -> Result<String, String> {
        let hunk = diff_info
            .hunks
            .get(hunk_index)
            .ok_or_else(|| format!("Hunk index {} out of range", hunk_index))?;

        let old_diff_path = diff_info.old_path.as_deref().unwrap_or(file_path);
        let new_diff_path = file_path;

        let old_side_path = match diff_info.status.as_str() {
            "added" => "/dev/null",
            _ => old_diff_path,
        };

        let new_side_path = match diff_info.status.as_str() {
            "deleted" => "/dev/null",
            _ => new_diff_path,
        };

        let mut patch = String::new();
        patch.push_str(&format!(
            "diff --git a/{} b/{}\n",
            old_diff_path, new_diff_path
        ));
        patch.push_str(&format!(
            "--- {}\n",
            Self::patch_side_path("a", old_side_path)
        ));
        patch.push_str(&format!(
            "+++ {}\n",
            Self::patch_side_path("b", new_side_path)
        ));

        let header_line = if hunk.header.trim_start().starts_with("@@") {
            hunk.header.trim_end_matches('\n').to_string()
        } else {
            format!(
                "@@ -{},{} +{},{} @@",
                hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
            )
        };

        patch.push_str(&header_line);
        patch.push('\n');

        for line in &hunk.lines {
            let Some(prefix) = Self::hunk_line_prefix(&line.line_type) else {
                continue;
            };

            patch.push(prefix);
            patch.push_str(line.content.trim_end_matches('\n'));
            patch.push('\n');
        }

        Ok(patch)
    }

    fn apply_reverse_hunk_patch(path: &str, patch_text: &str, staged: bool) -> Result<(), String> {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();

        let patch_path = std::env::temp_dir().join(format!(
            "gitswamp-revert-hunk-{}-{}.patch",
            std::process::id(),
            unique_suffix
        ));

        std::fs::write(&patch_path, patch_text)
            .map_err(|e| format!("Failed to write temporary patch file: {}", e))?;

        let patch_path_string = patch_path.to_string_lossy().to_string();
        let args_owned = if staged {
            vec![
                "apply".to_string(),
                "--cached".to_string(),
                "-R".to_string(),
                patch_path_string,
            ]
        } else {
            vec!["apply".to_string(), "-R".to_string(), patch_path_string]
        };
        let args: Vec<&str> = args_owned.iter().map(|value| value.as_str()).collect();

        let apply_result = GitRepository::git_cli(path, &args);
        let _ = std::fs::remove_file(&patch_path);

        apply_result
            .map(|_| ())
            .map_err(|e| format!("Failed to apply reverse patch for selected hunk: {}", e))
    }

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

        match extract_file_diff(&diff, file_path) {
            Ok(file_diff) => Ok(file_diff),
            Err(error) if !staged => {
                let status = repo
                    .status_file(Path::new(file_path))
                    .map_err(|e| e.message().to_string())?;
                if status.contains(git2::Status::WT_NEW) {
                    Self::build_untracked_file_diff(Path::new(path), file_path)
                } else {
                    Err(error)
                }
            }
            Err(error) => Err(error),
        }
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

    pub fn get_file_content(
        path: &str,
        file_path: &str,
        sha: Option<&str>,
    ) -> Result<String, String> {
        let repo = GitRepository::open(path)?;

        if let Some(commit_sha) = sha {
            let oid = git2::Oid::from_str(commit_sha).map_err(|e| e.message().to_string())?;
            let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
            let tree = commit.tree().map_err(|e| e.message().to_string())?;
            let entry = tree
                .get_path(Path::new(file_path))
                .map_err(|_| format!("File '{}' not found in commit", file_path))?;
            let blob = repo
                .find_blob(entry.id())
                .map_err(|e| e.message().to_string())?;
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
        let blob = repo
            .find_blob(entry.id)
            .map_err(|e| e.message().to_string())?;

        if blob.is_binary() {
            return Err("Binary file".to_string());
        }

        String::from_utf8(blob.content().to_vec())
            .map_err(|_| "File is not valid UTF-8".to_string())
    }

    pub fn get_file_blame(
        path: &str,
        file_path: &str,
        sha: Option<&str>,
    ) -> Result<Vec<FileBlameLine>, String> {
        let repo = GitRepository::open(path)?;
        let mut blame_options = git2::BlameOptions::new();

        if let Some(commit_sha) = sha {
            let oid = git2::Oid::from_str(commit_sha).map_err(|e| e.message().to_string())?;
            blame_options.newest_commit(oid);
        }

        let blame = repo
            .blame_file(Path::new(file_path), Some(&mut blame_options))
            .map_err(|e| e.message().to_string())?;

        let file_content = Self::get_file_content(path, file_path, sha)?;
        let file_lines: Vec<String> = file_content
            .split('\n')
            .map(|line| line.to_string())
            .collect();

        let mut commit_meta_cache: HashMap<String, BlameCommitMeta> = HashMap::new();
        let mut result: Vec<FileBlameLine> = Vec::new();

        for hunk in blame.iter() {
            let final_start = hunk.final_start_line() as u32;
            let lines_in_hunk = hunk.lines_in_hunk() as u32;
            let commit_id = hunk.final_commit_id();
            let commit_sha = commit_id.to_string();

            let meta = if let Some(cached) = commit_meta_cache.get(&commit_sha) {
                cached.clone()
            } else {
                let computed = Self::build_blame_commit_meta(
                    &repo,
                    &commit_sha,
                    commit_id,
                    hunk.final_signature(),
                )?;
                commit_meta_cache.insert(commit_sha.clone(), computed.clone());
                computed
            };

            for offset in 0..lines_in_hunk {
                let line_no = final_start + offset;
                let line_index = line_no.saturating_sub(1) as usize;
                let code = file_lines.get(line_index).cloned().unwrap_or_default();

                result.push(FileBlameLine {
                    line_no,
                    commit_sha: meta.commit_sha.clone(),
                    short_sha: meta.short_sha.clone(),
                    author: meta.author.clone(),
                    author_email: meta.author_email.clone(),
                    summary: meta.summary.clone(),
                    author_time: meta.author_time,
                    is_uncommitted: meta.is_uncommitted,
                    code,
                });
            }
        }

        result.sort_by_key(|entry| entry.line_no);
        Ok(result)
    }

    fn build_blame_commit_meta(
        repo: &git2::Repository,
        commit_sha: &str,
        commit_id: git2::Oid,
        signature: git2::Signature<'_>,
    ) -> Result<BlameCommitMeta, String> {
        let signature_name = signature.name().unwrap_or("Unknown").to_string();
        let signature_email = signature.email().unwrap_or("").to_string();
        let signature_time = signature.when().seconds();

        if commit_sha == UNCOMMITTED_SHA {
            return Ok(BlameCommitMeta {
                commit_sha: commit_sha.to_string(),
                short_sha: "LOCAL".to_string(),
                author: signature_name,
                author_email: signature_email,
                summary: "Uncommitted changes".to_string(),
                author_time: signature_time,
                is_uncommitted: true,
            });
        }

        if let Ok(commit) = repo.find_commit(commit_id) {
            let author = commit.author();
            return Ok(BlameCommitMeta {
                commit_sha: commit_sha.to_string(),
                short_sha: commit_sha.chars().take(8).collect(),
                author: author.name().unwrap_or("Unknown").to_string(),
                author_email: author.email().unwrap_or("").to_string(),
                summary: commit
                    .summary()
                    .unwrap_or("(no commit message)")
                    .to_string(),
                author_time: author.when().seconds(),
                is_uncommitted: false,
            });
        }

        Ok(BlameCommitMeta {
            commit_sha: commit_sha.to_string(),
            short_sha: commit_sha.chars().take(8).collect(),
            author: signature_name,
            author_email: signature_email,
            summary: "(commit metadata unavailable)".to_string(),
            author_time: signature_time,
            is_uncommitted: false,
        })
    }

    pub fn has_conflict_markers(path: &str, file_path: &str) -> Result<bool, String> {
        let full_path = Path::new(path).join(file_path);
        let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
        Ok(content.contains(CONFLICT_START)
            && content.contains(CONFLICT_MID)
            && content.contains(CONFLICT_END))
    }

    pub fn save_file_content(path: &str, file_path: &str, content: &str) -> Result<(), String> {
        let full_path = Path::new(path).join(file_path);
        std::fs::write(&full_path, content).map_err(|e| e.to_string())
    }

    pub fn revert_hunk(
        path: &str,
        file_path: &str,
        hunk_index: usize,
        staged: bool,
    ) -> Result<(), String> {
        let diff_info = Self::get_working_diff(path, file_path, staged)?;
        if diff_info.is_binary {
            return Err("Cannot revert hunks in binary files".to_string());
        }

        let patch_text = Self::build_single_hunk_patch(&diff_info, file_path, hunk_index)?;
        Self::apply_reverse_hunk_patch(path, &patch_text, staged)
    }
}

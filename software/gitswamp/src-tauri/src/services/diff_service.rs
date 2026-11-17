use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::{CONFLICT_END, CONFLICT_MID, CONFLICT_START};
use crate::models::{
    BinaryDiffInfo, BinarySideInfo, DiffHunk, DiffLine, FileBlameLine, FileDiff,
};
use crate::repositories::git_repository::GitRepository;
use crate::services::helpers::extract_file_diff;

const UNCOMMITTED_SHA: &str = "0000000000000000000000000000000000000000";
const MAX_BINARY_PREVIEW_BYTES: u64 = 10 * 1024 * 1024;

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
        let metadata = std::fs::metadata(&full_path).map_err(|e| e.to_string())?;
        if metadata.len() > MAX_BINARY_PREVIEW_BYTES {
            return Ok(FileDiff {
                path: file_path.to_string(),
                old_path: None,
                status: "added".to_string(),
                hunks: Vec::new(),
                is_binary: true,
            });
        }

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

    fn apply_hunk_patch(
        path: &str,
        patch_text: &str,
        cached: bool,
        reverse: bool,
        action_label: &str,
    ) -> Result<(), String> {
        let mut args = vec!["apply"];
        if cached { args.push("--cached"); }
        if reverse { args.push("-R"); }
        Self::apply_patch(path, patch_text.as_bytes(), &args)
            .map_err(|e| format!("Failed to {} selected hunk: {}", action_label, e))
    }

    pub(super) fn apply_patch(path: &str, patch: &[u8], args: &[&str]) -> Result<(), String> {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();

        let patch_path = std::env::temp_dir().join(format!(
            "gitswamp-diff-{}-{}.patch",
            std::process::id(),
            unique_suffix
        ));

        std::fs::write(&patch_path, patch)
            .map_err(|e| format!("Failed to write temporary patch file: {}", e))?;

        let patch_path_string = patch_path.to_string_lossy().to_string();
        let mut args = args.to_vec();
        args.push(&patch_path_string);
        let apply_result = GitRepository::git_cli(path, &args);
        let _ = std::fs::remove_file(&patch_path);
        apply_result.map(|_| ())
    }

    fn apply_reverse_hunk_patch(path: &str, patch_text: &str, staged: bool) -> Result<(), String> {
        Self::apply_hunk_patch(path, patch_text, staged, true, "revert")
    }

    pub fn get_working_diff(path: &str, file_path: &str, staged: bool) -> Result<FileDiff, String> {
        let repo = GitRepository::open(path)?;
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.context_lines(3).interhunk_lines(1).disable_pathspec_match(true).pathspec(file_path);

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
                    .unwrap_or_else(|_| git2::Status::empty());
                let tracked_in_index = repo
                    .index()
                    .ok()
                    .and_then(|index| index.get_path(Path::new(file_path), 0).map(|_| ()))
                    .is_some();
                let full_path = Path::new(path).join(file_path);
                if status.contains(git2::Status::WT_NEW)
                    || (!tracked_in_index && full_path.is_file())
                {
                    Self::build_untracked_file_diff(Path::new(path), file_path)
                } else if !status.intersects(git2::Status::WT_MODIFIED | git2::Status::WT_DELETED
                    | git2::Status::WT_RENAMED | git2::Status::WT_TYPECHANGE | git2::Status::CONFLICTED) {
                    Ok(FileDiff {
                        path: file_path.to_string(),
                        old_path: None,
                        status: "modified".to_string(),
                        hunks: Vec::new(),
                        is_binary: false,
                    })
                } else {
                    Err(error)
                }
            }
            Err(_) if diff.deltas().len() == 0 => Ok(FileDiff {
                path: file_path.to_string(), old_path: None, status: "modified".to_string(),
                hunks: Vec::new(), is_binary: false,
            }),
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

        match extract_file_diff(&diff, file_path) {
            Ok(file_diff) => Ok(file_diff),
            Err(not_found) => {
                if let Some(untracked_tree) = commit.parent(2).and_then(|p| p.tree()).ok() {
                    let mut untracked_opts = git2::DiffOptions::new();
                    untracked_opts.context_lines(3).interhunk_lines(1);
                    let untracked_diff = repo
                        .diff_tree_to_tree(None, Some(&untracked_tree), Some(&mut untracked_opts))
                        .map_err(|e| e.message().to_string())?;
                    extract_file_diff(&untracked_diff, file_path)
                } else {
                    Err(not_found)
                }
            }
        }
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
            let entry = match tree.get_path(Path::new(file_path)) {
                Ok(entry) => entry,
                Err(_) => {
                    let untracked_tree = commit
                        .parent(2)
                        .and_then(|p| p.tree())
                        .ok()
                        .ok_or_else(|| format!("File '{}' not found in commit", file_path))?;
                    untracked_tree
                        .get_path(Path::new(file_path))
                        .map_err(|_| format!("File '{}' not found in commit", file_path))?
                }
            };
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

    pub fn get_binary_diff_info(
        path: &str,
        file_path: &str,
        old_path: Option<&str>,
        sha: Option<&str>,
        staged: bool,
    ) -> Result<BinaryDiffInfo, String> {
        let repo = GitRepository::open(path)?;
        let repo_root = Path::new(path);
        let old_target = old_path.unwrap_or(file_path);

        let old_info = Self::repo_side_info(&repo, repo_root, old_target, sha, staged, "old")?;
        let new_info = Self::repo_side_info(&repo, repo_root, file_path, sha, staged, "new")?;

        let (old_exists, (old_size, old_id)) = match old_info {
            Some((size, id)) => (true, (size, id)),
            None => (false, (0, String::new())),
        };
        let (new_exists, (new_size, new_id)) = match new_info {
            Some((size, id)) => (true, (size, id)),
            None => (false, (0, String::new())),
        };

        Ok(BinaryDiffInfo {
            old: BinarySideInfo {
                exists: old_exists,
                size: old_size,
                id: old_id,
            },
            new: BinarySideInfo {
                exists: new_exists,
                size: new_size,
                id: new_id,
            },
        })
    }

    pub fn get_file_bytes(
        path: &str,
        file_path: &str,
        old_path: Option<&str>,
        sha: Option<&str>,
        staged: bool,
        side: &str,
    ) -> Result<Vec<u8>, String> {
        if side != "old" && side != "new" {
            return Err(format!("Invalid side '{}'", side));
        }

        let repo = GitRepository::open(path)?;
        let repo_root = Path::new(path);
        let target = if side == "old" {
            old_path.unwrap_or(file_path)
        } else {
            file_path
        };

        Self::repo_side_bytes(&repo, repo_root, target, sha, staged, side)?
            .ok_or_else(|| format!("File '{}' does not exist on the {} side", target, side))
    }

    fn repo_side_info(
        repo: &git2::Repository,
        repo_root: &Path,
        file_path: &str,
        sha: Option<&str>,
        staged: bool,
        side: &str,
    ) -> Result<Option<(u64, String)>, String> {
        if let Some(commit_sha) = sha {
            let oid = git2::Oid::from_str(commit_sha).map_err(|e| e.message().to_string())?;
            let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;

            if side == "old" {
                let tree = commit.parent(0).ok().and_then(|parent| parent.tree().ok());
                return match tree.and_then(|tree| Self::tree_blob_oid(&tree, file_path)) {
                    Some(blob_oid) => Self::blob_info(repo, blob_oid).map(Some),
                    None => Ok(None),
                };
            }

            let tree_oid = commit
                .tree()
                .ok()
                .and_then(|tree| Self::tree_blob_oid(&tree, file_path));
            if let Some(blob_oid) = tree_oid {
                return Self::blob_info(repo, blob_oid).map(Some);
            }

            let untracked_oid = commit
                .parent(2)
                .ok()
                .and_then(|parent| parent.tree().ok())
                .and_then(|tree| Self::tree_blob_oid(&tree, file_path));
            return match untracked_oid {
                Some(blob_oid) => Self::blob_info(repo, blob_oid).map(Some),
                None => Ok(None),
            };
        }

        if side == "new" {
            if staged {
                return Self::index_blob_info(repo, file_path);
            }
            let full_path = repo_root.join(file_path);
            return match std::fs::metadata(&full_path) {
                Ok(meta) if meta.is_file() => Ok(Some(Self::working_file_info(&meta))),
                _ => Ok(None),
            };
        }

        if staged {
            let tree_oid = repo
                .head()
                .and_then(|head| head.peel_to_tree())
                .ok()
                .and_then(|tree| Self::tree_blob_oid(&tree, file_path));
            return match tree_oid {
                Some(blob_oid) => Self::blob_info(repo, blob_oid).map(Some),
                None => Ok(None),
            };
        }

        Self::index_blob_info(repo, file_path)
    }

    fn repo_side_bytes(
        repo: &git2::Repository,
        repo_root: &Path,
        file_path: &str,
        sha: Option<&str>,
        staged: bool,
        side: &str,
    ) -> Result<Option<Vec<u8>>, String> {
        if let Some(commit_sha) = sha {
            let oid = git2::Oid::from_str(commit_sha).map_err(|e| e.message().to_string())?;
            let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;

            if side == "old" {
                let tree = commit.parent(0).ok().and_then(|parent| parent.tree().ok());
                return match tree.and_then(|tree| Self::tree_blob_oid(&tree, file_path)) {
                    Some(blob_oid) => Self::read_blob_limited(repo, blob_oid).map(Some),
                    None => Ok(None),
                };
            }

            let tree_oid = commit
                .tree()
                .ok()
                .and_then(|tree| Self::tree_blob_oid(&tree, file_path));
            if let Some(blob_oid) = tree_oid {
                return Self::read_blob_limited(repo, blob_oid).map(Some);
            }

            let untracked_oid = commit
                .parent(2)
                .ok()
                .and_then(|parent| parent.tree().ok())
                .and_then(|tree| Self::tree_blob_oid(&tree, file_path));
            return match untracked_oid {
                Some(blob_oid) => Self::read_blob_limited(repo, blob_oid).map(Some),
                None => Ok(None),
            };
        }

        if side == "new" {
            if staged {
                return Self::index_blob_bytes(repo, file_path);
            }
            let full_path = repo_root.join(file_path);
            if !full_path.is_file() {
                return Ok(None);
            }
            return Self::read_working_file_limited(&full_path).map(Some);
        }

        if staged {
            let tree_oid = repo
                .head()
                .and_then(|head| head.peel_to_tree())
                .ok()
                .and_then(|tree| Self::tree_blob_oid(&tree, file_path));
            return match tree_oid {
                Some(blob_oid) => Self::read_blob_limited(repo, blob_oid).map(Some),
                None => Ok(None),
            };
        }

        Self::index_blob_bytes(repo, file_path)
    }

    fn tree_blob_oid(tree: &git2::Tree<'_>, file_path: &str) -> Option<git2::Oid> {
        tree.get_path(Path::new(file_path)).ok().map(|entry| entry.id())
    }

    fn blob_info(repo: &git2::Repository, oid: git2::Oid) -> Result<(u64, String), String> {
        let (size, kind) = repo.odb().and_then(|odb| odb.read_header(oid))
            .map_err(|e| e.message().to_string())?;
        if kind != git2::ObjectType::Blob {
            return Err("Preview is only available for files".to_string());
        }
        Ok((size as u64, oid.to_string()))
    }

    fn index_blob_info(
        repo: &git2::Repository,
        file_path: &str,
    ) -> Result<Option<(u64, String)>, String> {
        let index = repo.index().map_err(|e| e.message().to_string())?;
        match index.get_path(Path::new(file_path), 0) {
            Some(entry) => Ok(Some(Self::blob_info(repo, entry.id)?)),
            None => Ok(None),
        }
    }

    fn working_file_info(metadata: &std::fs::Metadata) -> (u64, String) {
        let modified = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| format!("{}.{:09}", value.as_secs(), value.subsec_nanos()))
            .unwrap_or_else(|| "unknown".to_string());
        (metadata.len(), format!("worktree:{}:{}", metadata.len(), modified))
    }

    fn index_blob_bytes(
        repo: &git2::Repository,
        file_path: &str,
    ) -> Result<Option<Vec<u8>>, String> {
        let index = repo.index().map_err(|e| e.message().to_string())?;
        match index.get_path(Path::new(file_path), 0) {
            Some(entry) => Self::read_blob_limited(repo, entry.id).map(Some),
            None => Ok(None),
        }
    }

    fn read_blob_limited(
        repo: &git2::Repository,
        oid: git2::Oid,
    ) -> Result<Vec<u8>, String> {
        let (size, _) = Self::blob_info(repo, oid)?;
        if size > MAX_BINARY_PREVIEW_BYTES {
            return Err(Self::preview_too_large_message(size));
        }
        let blob = repo.find_blob(oid).map_err(|e| e.message().to_string())?;
        Ok(blob.content().to_vec())
    }

    fn read_working_file_limited(full_path: &Path) -> Result<Vec<u8>, String> {
        let metadata = std::fs::metadata(full_path).map_err(|e| e.to_string())?;
        if metadata.len() > MAX_BINARY_PREVIEW_BYTES {
            return Err(Self::preview_too_large_message(metadata.len()));
        }
        // Recheck the actual read too: the file may grow after metadata was read.
        let file = std::fs::File::open(full_path).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(MAX_BINARY_PREVIEW_BYTES + 1).read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BINARY_PREVIEW_BYTES {
            return Err(Self::preview_too_large_message(bytes.len() as u64));
        }
        Ok(bytes)
    }

    fn preview_too_large_message(size: u64) -> String {
        format!(
            "File is too large to preview ({} bytes, limit {} bytes)",
            size, MAX_BINARY_PREVIEW_BYTES
        )
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

    pub fn stage_hunk(path: &str, file_path: &str, hunk_index: usize) -> Result<(), String> {
        let diff_info = Self::get_working_diff(path, file_path, false)?;
        if diff_info.is_binary {
            return Err("Cannot stage hunks in binary files".to_string());
        }

        let patch_text = Self::build_single_hunk_patch(&diff_info, file_path, hunk_index)?;
        Self::apply_hunk_patch(path, &patch_text, true, false, "stage")
    }

    pub fn unstage_hunk(path: &str, file_path: &str, hunk_index: usize) -> Result<(), String> {
        let diff_info = Self::get_working_diff(path, file_path, true)?;
        if diff_info.is_binary {
            return Err("Cannot unstage hunks in binary files".to_string());
        }

        let patch_text = Self::build_single_hunk_patch(&diff_info, file_path, hunk_index)?;
        Self::apply_hunk_patch(path, &patch_text, true, true, "unstage")
    }
}

#[cfg(test)]
mod tests {
    use super::{DiffService, MAX_BINARY_PREVIEW_BYTES};
    use std::path::{Path, PathBuf};

    struct Fixture { root: PathBuf, repo: git2::Repository }
    impl Fixture {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/gitswamp-tests")
                .join(format!("binary-{}-{}", std::process::id(), nonce));
            std::fs::create_dir_all(&root).unwrap();
            let repo = git2::Repository::init(&root).unwrap();
            Self { root, repo }
        }
        fn path(&self) -> &str { self.root.to_str().unwrap() }
        fn write(&self, name: &str, bytes: &[u8]) { std::fs::write(self.root.join(name), bytes).unwrap(); }
        fn stage(&self, name: &str) {
            let mut index = self.repo.index().unwrap();
            index.add_path(Path::new(name)).unwrap(); index.write().unwrap();
        }
        fn commit(&self) -> git2::Oid {
            let tree_id = self.repo.index().unwrap().write_tree().unwrap();
            let tree = self.repo.find_tree(tree_id).unwrap();
            let sig = git2::Signature::now("Test", "test@example.invalid").unwrap();
            let parent = self.repo.head().ok().and_then(|head| head.peel_to_commit().ok());
            self.repo.commit(Some("HEAD"), &sig, &sig, "Test image", &tree, &parent.iter().collect::<Vec<_>>()).unwrap()
        }
        fn bytes(&self, sha: Option<&str>, staged: bool, side: &str) -> Vec<u8> {
            DiffService::get_file_bytes(self.path(), "picture.png", None, sha, staged, side).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); }
    }

    fn select_line(fixture: &Fixture, name: &str, staged: bool, kind: &str, text: &str) -> Result<(), String> {
        let diff = DiffService::get_working_diff(fixture.path(), name, staged)?;
        let (hunk_index, line_index, line) = diff.hunks.iter().enumerate().find_map(|(hunk_index, hunk)| {
            hunk.lines.iter().enumerate().find(|(_, line)| line.line_type == kind && line.content == text)
                .map(|(line_index, line)| (hunk_index, line_index, line))
        }).unwrap_or_else(|| panic!("Missing {kind} {text:?} in {diff:?}"));
        super::super::line_staging::apply(fixture.path(), name, staged, hunk_index, line_index, line)
    }

    fn index_bytes(fixture: &Fixture, name: &str) -> Option<Vec<u8>> {
        let mut index = fixture.repo.index().unwrap();
        index.read(true).unwrap();
        let entry = index.get_path(Path::new(name), 0)?;
        Some(fixture.repo.find_blob(entry.id).unwrap().content().to_vec())
    }

    #[test]
    fn stages_and_unstages_one_change_without_touching_neighbors_or_worktree() {
        let fixture = Fixture::new();
        let name = "with spaces [literal].txt";
        let base = b"first\nold\nlast\n";
        let edited = b"first\nnew one\nnew two\nlast\n";
        fixture.write(name, base);
        fixture.stage(name);
        fixture.commit();
        fixture.write(name, edited);
        select_line(&fixture, name, false, "addition", "new two\n").unwrap();
        assert_eq!(index_bytes(&fixture, name).unwrap(), b"first\nold\nnew two\nlast\n");
        select_line(&fixture, name, false, "deletion", "old\n").unwrap();
        assert_eq!(index_bytes(&fixture, name).unwrap(), b"first\nnew two\nlast\n");
        select_line(&fixture, name, true, "deletion", "old\n").unwrap();
        assert_eq!(index_bytes(&fixture, name).unwrap(), b"first\nold\nnew two\nlast\n");
        select_line(&fixture, name, true, "addition", "new two\n").unwrap();
        assert_eq!(index_bytes(&fixture, name).unwrap(), base);
        assert!(DiffService::get_working_diff(fixture.path(), name, true).unwrap().hunks.is_empty());
        assert_eq!(std::fs::read(fixture.root.join(name)).unwrap(), edited);
    }

    #[test]
    fn partial_new_file_and_final_unstage_preserve_bytes_and_untracked_file() {
        let fixture = Fixture::new();
        let name = "new file.txt";
        let edited = b"first\nlast with no newline";
        fixture.write(name, edited);
        select_line(&fixture, name, false, "addition", "last with no newline").unwrap();
        assert_eq!(index_bytes(&fixture, name).unwrap(), b"last with no newline");
        select_line(&fixture, name, true, "addition", "last with no newline").unwrap();
        assert_eq!(index_bytes(&fixture, name), None);
        assert_eq!(std::fs::read(fixture.root.join(name)).unwrap(), edited);
    }

    #[test]
    fn handles_partial_deleted_file_and_restores_one_line_to_index() {
        let fixture = Fixture::new();
        let name = "deleted.txt";
        fixture.write(name, b"one\ntwo\n");
        fixture.stage(name);
        fixture.commit();
        std::fs::remove_file(fixture.root.join(name)).unwrap();
        select_line(&fixture, name, false, "deletion", "two\n").unwrap();
        assert_eq!(index_bytes(&fixture, name).unwrap(), b"one\n");
        select_line(&fixture, name, false, "deletion", "one\n").unwrap();
        assert_eq!(index_bytes(&fixture, name), None);
        select_line(&fixture, name, true, "deletion", "two\n").unwrap();
        assert_eq!(index_bytes(&fixture, name).unwrap(), b"two\n");
        assert!(!fixture.root.join(name).exists());
    }

    #[test]
    fn rejects_a_changed_line_and_preserves_crlf_and_whitespace() {
        let fixture = Fixture::new();
        fixture.repo.config().unwrap().set_str("core.autocrlf", "false").unwrap();
        let name = "crlf.txt";
        fixture.write(name, b"one\r\n");
        fixture.stage(name);
        fixture.commit();
        fixture.write(name, b"one\r\nprior\r\n");
        let diff = DiffService::get_working_diff(fixture.path(), name, false).unwrap();
        fixture.write(name, b"one\r\nnext  \r\n");
        let result = super::super::line_staging::apply(fixture.path(), name, false, 0, 1, &diff.hunks[0].lines[1]);
        assert!(result.unwrap_err().contains("diff changed"));
        assert_eq!(index_bytes(&fixture, name).unwrap(), b"one\r\n");
        select_line(&fixture, name, false, "addition", "next  \r\n").unwrap();
        assert_eq!(index_bytes(&fixture, name).unwrap(), b"one\r\nnext  \r\n");
    }

    #[test]
    fn binary_preview_uses_index_worktree_head_and_parent_correctly() {
        let f = Fixture::new();
        f.write("picture.png", b"\0head"); f.stage("picture.png");
        let initial = f.commit().to_string();
        f.write("picture.png", b"\0index"); f.stage("picture.png");
        f.write("picture.png", b"\0worktree");
        assert_eq!(f.bytes(None, false, "old"), b"\0index");
        assert_eq!(f.bytes(None, false, "new"), b"\0worktree");
        assert_eq!(f.bytes(None, true, "old"), b"\0head");
        assert_eq!(f.bytes(None, true, "new"), b"\0index");
        assert!(DiffService::get_working_diff(f.path(), "picture.png", false).unwrap().is_binary);
        assert!(DiffService::get_working_diff(f.path(), "picture.png", true).unwrap().is_binary);
        let info = DiffService::get_binary_diff_info(f.path(), "picture.png", None, None, false).unwrap();
        assert_eq!(info.old.size, 6); assert_eq!(info.new.size, 9);
        assert!(info.new.id.starts_with("worktree:"));
        assert_ne!(info.old.id, info.new.id);
        let next = f.commit().to_string();
        assert_eq!(f.bytes(Some(&next), false, "old"), b"\0head");
        assert_eq!(f.bytes(Some(&next), false, "new"), b"\0index");
        assert!(DiffService::get_commit_diff(f.path(), &next, "picture.png").unwrap().is_binary);
        let root = DiffService::get_binary_diff_info(f.path(), "picture.png", None, Some(&initial), false).unwrap();
        assert!(!root.old.exists); assert!(root.new.exists);
    }

    #[test]
    fn binary_preview_handles_renames_deletions_and_untracked_files() {
        let f = Fixture::new();
        f.write("old.png", b"\0before"); f.stage("old.png"); f.commit();
        let mut index = f.repo.index().unwrap();
        index.remove_path(Path::new("old.png")).unwrap(); index.write().unwrap();
        std::fs::rename(f.root.join("old.png"), f.root.join("picture.png")).unwrap();
        f.write("picture.png", b"\0after"); f.stage("picture.png");
        let sha = f.commit().to_string();
        assert_eq!(DiffService::get_file_bytes(f.path(), "picture.png", Some("old.png"), Some(&sha), false, "old").unwrap(), b"\0before");
        assert_eq!(f.bytes(Some(&sha), false, "new"), b"\0after");
        std::fs::remove_file(f.root.join("picture.png")).unwrap();
        let deleted = DiffService::get_binary_diff_info(f.path(), "picture.png", None, None, false).unwrap();
        assert!(deleted.old.exists); assert!(!deleted.new.exists);
        let added = DiffService::get_binary_diff_info(f.path(), "new.png", None, None, false).unwrap();
        assert!(!added.old.exists); assert!(!added.new.exists);
        f.write("new.png", b"\0untracked");
        let added = DiffService::get_binary_diff_info(f.path(), "new.png", None, None, false).unwrap();
        assert!(!added.old.exists); assert!(added.new.exists);
        assert!(DiffService::get_working_diff(f.path(), "new.png", false).unwrap().is_binary);
    }

    #[test]
    fn binary_preview_reports_large_sizes_without_returning_large_payloads() {
        let f = Fixture::new();
        let file = std::fs::File::create(f.root.join("picture.png")).unwrap();
        file.set_len(MAX_BINARY_PREVIEW_BYTES + 1).unwrap(); drop(file);
        let info = DiffService::get_binary_diff_info(f.path(), "picture.png", None, None, false).unwrap();
        assert_eq!(info.new.size, MAX_BINARY_PREVIEW_BYTES + 1);
        assert!(DiffService::get_file_bytes(f.path(), "picture.png", None, None, false, "new").unwrap_err().contains("too large"));
        f.stage("picture.png");
        assert!(DiffService::get_file_bytes(f.path(), "picture.png", None, None, true, "new").unwrap_err().contains("too large"));
        assert!(DiffService::get_file_bytes(f.path(), "picture.png", None, None, true, "invalid").unwrap_err().contains("Invalid side"));
    }
}

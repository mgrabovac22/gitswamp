use std::collections::HashMap;

use git2::{Branch, Delta, Diff, Repository, Status};

use crate::models::{DiffHunk, DiffLine, FileDiff, RemoteInfo};

pub fn detect_provider(url: &str) -> String {
    let url_lower = url.to_lowercase();
    if url_lower.contains("github.com") || url_lower.contains("github.") {
        "github".to_string()
    } else if url_lower.contains("gitlab.com") || url_lower.contains("gitlab.") {
        "gitlab".to_string()
    } else if url_lower.contains("bitbucket.org") || url_lower.contains("bitbucket.") {
        "bitbucket".to_string()
    } else if url_lower.contains("dev.azure.com") || url_lower.contains("visualstudio.com") {
        "azure".to_string()
    } else {
        "unknown".to_string()
    }
}

pub fn build_remotes(repo: &Repository) -> Vec<RemoteInfo> {
    let mut remotes = Vec::new();
    if let Ok(remote_names) = repo.remotes() {
        for remote_name in remote_names.iter().flatten() {
            if let Ok(remote) = repo.find_remote(remote_name) {
                if let Some(url) = remote.url() {
                    remotes.push(RemoteInfo {
                        name: remote_name.to_string(),
                        url: url.to_string(),
                        provider: detect_provider(url),
                    });
                }
            }
        }
    }
    remotes
}

pub fn ahead_behind(repo: &Repository, branch: &Branch, upstream: &Option<String>) -> (usize, usize) {
    let Some(local_oid) = branch.get().target() else {
        return (0, 0);
    };
    let Some(ref up_name) = upstream else {
        return (0, 0);
    };
    let Ok(remote_ref) = repo.find_reference(&format!("refs/remotes/{}", up_name)) else {
        return (0, 0);
    };
    let Some(remote_oid) = remote_ref.target() else {
        return (0, 0);
    };
    repo.graph_ahead_behind(local_oid, remote_oid)
        .unwrap_or((0, 0))
}

pub fn time_ago(now: i64, timestamp: i64) -> String {
    let diff = now - timestamp;
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 604800 {
        format!("{}d ago", diff / 86400)
    } else if diff < 2592000 {
        format!("{}w ago", diff / 604800)
    } else {
        format!("{}mo ago", diff / 2592000)
    }
}

pub fn index_status_label(s: Status) -> String {
    if s.is_index_new() {
        "new"
    } else if s.is_index_modified() {
        "modified"
    } else if s.is_index_deleted() {
        "deleted"
    } else if s.is_index_renamed() {
        "renamed"
    } else {
        "typechange"
    }
    .to_string()
}

pub fn wt_status_label(s: Status) -> String {
    if s.is_wt_new() {
        "new"
    } else if s.is_wt_modified() {
        "modified"
    } else if s.is_wt_deleted() {
        "deleted"
    } else if s.is_wt_renamed() {
        "renamed"
    } else {
        "typechange"
    }
    .to_string()
}

pub fn urlencoded(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", b));
            }
        }
    }
    result
}

pub fn extract_file_diff(diff: &Diff, target_path: &str) -> Result<FileDiff, String> {
    let mut result: Option<FileDiff> = None;
    let num_deltas = diff.deltas().len();

    for idx in 0..num_deltas {
        let delta = diff.get_delta(idx).unwrap();
        let new_file = delta.new_file();
        let old_file = delta.old_file();

        let file_path = new_file
            .path()
            .or(old_file.path())
            .and_then(|p| p.to_str())
            .unwrap_or("");

        if file_path != target_path && old_file.path().and_then(|p| p.to_str()) != Some(target_path) {
            continue;
        }

        let status = match delta.status() {
            Delta::Added => "added",
            Delta::Deleted => "deleted",
            Delta::Modified => "modified",
            Delta::Renamed => "renamed",
            Delta::Copied => "copied",
            _ => "modified",
        };

        let old_path = if delta.status() == Delta::Renamed {
            old_file.path().and_then(|p| p.to_str()).map(|s| s.to_string())
        } else {
            None
        };

        let is_binary = new_file.is_binary() || old_file.is_binary();

        let mut hunks: Vec<DiffHunk> = Vec::new();

        if let Ok(Some(patch)) = git2::Patch::from_diff(diff, idx) {
            let num_hunks = patch.num_hunks();
            for hunk_idx in 0..num_hunks {
                if let Ok((hunk, num_lines)) = patch.hunk(hunk_idx) {
                    let header = String::from_utf8_lossy(hunk.header()).to_string();
                    let mut lines: Vec<DiffLine> = Vec::new();

                    for line_idx in 0..num_lines {
                        if let Ok(line) = patch.line_in_hunk(hunk_idx, line_idx) {
                            let line_type = match line.origin() {
                                '+' => "addition",
                                '-' => "deletion",
                                ' ' => "context",
                                _ => "context",
                            };
                            lines.push(DiffLine {
                                line_type: line_type.to_string(),
                                old_line_no: line.old_lineno(),
                                new_line_no: line.new_lineno(),
                                content: String::from_utf8_lossy(line.content()).to_string(),
                            });
                        }
                    }

                    hunks.push(DiffHunk {
                        old_start: hunk.old_start(),
                        old_lines: hunk.old_lines(),
                        new_start: hunk.new_start(),
                        new_lines: hunk.new_lines(),
                        header,
                        lines,
                    });
                }
            }
        }

        result = Some(FileDiff {
            path: file_path.to_string(),
            old_path,
            status: status.to_string(),
            hunks,
            is_binary,
        });
        break;
    }

    result.ok_or_else(|| format!("File '{}' not found in diff", target_path))
}

pub fn build_ref_map(repo: &Repository) -> HashMap<String, Vec<String>> {
    let mut ref_map: HashMap<String, Vec<String>> = HashMap::new();
    if let Ok(refs) = repo.references() {
        for reference in refs.flatten() {
            if let (Some(target), Some(name)) = (reference.target(), reference.shorthand()) {
                ref_map
                    .entry(target.to_string())
                    .or_default()
                    .push(name.to_string());
            }
        }
    }
    ref_map
}

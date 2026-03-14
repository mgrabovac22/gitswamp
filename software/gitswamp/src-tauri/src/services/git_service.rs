use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use git2::{BranchType, Repository, Sort, StatusOptions};

use crate::models::{BranchInfo, CommitFileInfo, CommitInfo, FileStatusInfo, RepoInfo, StashInfo, TagInfo};

static GIT_PATH: OnceLock<String> = OnceLock::new();

fn find_git() -> String {
    // Try "git" directly first (works if PATH is correct)
    if let Ok(output) = std::process::Command::new("git")
        .arg("--version")
        .output()
    {
        if output.status.success() {
            return "git".to_string();
        }
    }

    // Common Windows git locations
    let candidates = [
        r"C:\Program Files\Git\cmd\git.exe",
        r"C:\Program Files\Git\bin\git.exe",
        r"C:\Program Files (x86)\Git\cmd\git.exe",
        r"C:\Program Files (x86)\Git\bin\git.exe",
    ];
    for p in &candidates {
        if Path::new(p).exists() {
            return p.to_string();
        }
    }

    // Try where.exe to find git
    if let Ok(output) = std::process::Command::new("where.exe")
        .arg("git")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = stdout.lines().next() {
                let path = first_line.trim();
                if !path.is_empty() && Path::new(path).exists() {
                    return path.to_string();
                }
            }
        }
    }

    // Last resort
    "git".to_string()
}

fn git_executable() -> &'static str {
    GIT_PATH.get_or_init(find_git)
}

pub struct GitService;

impl GitService {
    fn open(path: &str) -> Result<Repository, String> {
        Repository::open(path).map_err(|e| e.message().to_string())
    }

    pub fn repo_info(path: &str) -> Result<RepoInfo, String> {
        let repo = Self::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = head.shorthand().unwrap_or("HEAD").to_string();
        let head_sha = head.target().map(|oid| oid.to_string());
        let statuses = repo.statuses(None).map_err(|e| e.message().to_string())?;

        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());

        Ok(RepoInfo {
            path: path.to_string(),
            name,
            current_branch: branch_name,
            is_clean: statuses.is_empty(),
            head_sha,
        })
    }

    pub fn commits(path: &str, max_count: usize) -> Result<Vec<CommitInfo>, String> {
        let repo = Self::open(path)?;
        let mut revwalk = repo.revwalk().map_err(|e| e.message().to_string())?;

        if let Ok(branches) = repo.branches(Some(BranchType::Local)) {
            for item in branches.flatten() {
                if let Some(oid) = item.0.get().target() {
                    let _ = revwalk.push(oid);
                }
            }
        }
        let _ = revwalk.push_head();
        revwalk
            .set_sorting(Sort::TIME | Sort::TOPOLOGICAL)
            .map_err(|e| e.message().to_string())?;

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

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for oid in revwalk.flatten().take(max_count) {
            if !seen.insert(oid) {
                continue;
            }
            let commit = match repo.find_commit(oid) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let sha = oid.to_string();
            let short_sha = sha[..7.min(sha.len())].to_string();
            let timestamp = commit.time().seconds();
            let parent_shas: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();
            let refs = ref_map.get(&sha).cloned().unwrap_or_default();

            result.push(CommitInfo {
                sha,
                short_sha,
                message: commit.message().unwrap_or("").trim().to_string(),
                author_name: commit.author().name().unwrap_or("Unknown").to_string(),
                author_email: commit.author().email().unwrap_or("").to_string(),
                timestamp,
                time_ago: Self::time_ago(now, timestamp),
                parent_shas,
                refs,
            });
        }

        Ok(result)
    }

    pub fn branches(path: &str) -> Result<Vec<BranchInfo>, String> {
        let repo = Self::open(path)?;
        let head = repo.head().ok();
        let head_name = head
            .as_ref()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let mut result = Vec::new();

        for branch_type in [BranchType::Local, BranchType::Remote] {
            let Ok(iter) = repo.branches(Some(branch_type)) else {
                continue;
            };
            for item in iter.flatten() {
                let (branch, bt) = item;
                let Ok(Some(name)) = branch.name() else {
                    continue;
                };

                let is_head = head_name.as_deref() == Some(name);
                let is_remote = bt == BranchType::Remote;

                let upstream = branch
                    .upstream()
                    .ok()
                    .and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));

                let (ahead, behind) = Self::ahead_behind(&repo, &branch, &upstream);

                result.push(BranchInfo {
                    name: name.to_string(),
                    is_head,
                    is_remote,
                    upstream,
                    ahead,
                    behind,
                });
            }
        }

        Ok(result)
    }

    pub fn status(path: &str) -> Result<Vec<FileStatusInfo>, String> {
        let repo = Self::open(path)?;
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = repo
            .statuses(Some(&mut opts))
            .map_err(|e| e.message().to_string())?;

        let mut files = Vec::new();

        for entry in statuses.iter() {
            let s = entry.status();
            let file_path = entry.path().unwrap_or("").to_string();

            if s.is_index_new()
                || s.is_index_modified()
                || s.is_index_deleted()
                || s.is_index_renamed()
                || s.is_index_typechange()
            {
                files.push(FileStatusInfo {
                    path: file_path.clone(),
                    status: Self::index_status_label(s),
                    staged: true,
                });
            }

            if s.is_wt_new()
                || s.is_wt_modified()
                || s.is_wt_deleted()
                || s.is_wt_renamed()
                || s.is_wt_typechange()
            {
                files.push(FileStatusInfo {
                    path: file_path,
                    status: Self::wt_status_label(s),
                    staged: false,
                });
            }
        }

        Ok(files)
    }

    pub fn stage_file(path: &str, file_path: &str) -> Result<(), String> {
        let repo = Self::open(path)?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        index
            .add_path(Path::new(file_path))
            .map_err(|e| e.message().to_string())?;
        index.write().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn unstage_file(path: &str, file_path: &str) -> Result<(), String> {
        let repo = Self::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let obj = head
            .peel_to_commit()
            .map_err(|e| e.message().to_string())?
            .tree()
            .map_err(|e| e.message().to_string())?;
        repo.reset_default(Some(obj.as_object()), &[file_path])
            .map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn create_commit(path: &str, message: &str) -> Result<String, String> {
        let repo = Self::open(path)?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let tree_oid = index.write_tree().map_err(|e| e.message().to_string())?;
        let tree = repo.find_tree(tree_oid).map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let parent = head
            .peel_to_commit()
            .map_err(|e| e.message().to_string())?;

        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
            .map_err(|e| e.message().to_string())?;

        Ok(oid.to_string())
    }

    pub fn checkout_branch(path: &str, branch_name: &str) -> Result<(), String> {
        let repo = Self::open(path)?;
        let (object, reference) = repo
            .revparse_ext(branch_name)
            .map_err(|e| e.message().to_string())?;

        repo.checkout_tree(&object, None)
            .map_err(|e| e.message().to_string())?;

        if let Some(reference) = reference {
            repo.set_head(reference.name().unwrap_or(""))
                .map_err(|e| e.message().to_string())?;
        } else {
            repo.set_head_detached(object.id())
                .map_err(|e| e.message().to_string())?;
        }

        Ok(())
    }

    pub fn pull(path: &str, token: Option<&str>) -> Result<String, String> {
        if let Some(t) = token {
            Self::git_cli_with_token(path, &["pull"], t)
        } else {
            Self::git_cli(path, &["pull"])
        }
    }

    pub fn push(path: &str, token: Option<&str>) -> Result<String, String> {
        let result = if let Some(t) = token {
            Self::git_cli_with_token(path, &["push"], t)
        } else {
            Self::git_cli(path, &["push"])
        };
        if result.is_ok() {
            return result;
        }
        // Fallback: set upstream if not configured
        let branch = Self::git_cli(path, &["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap_or_else(|_| "main".to_string());
        let branch = branch.trim();
        // First try without token to set upstream
        let _ = Self::git_cli(path, &["push", "--set-upstream", "origin", branch]);
        // Then push with token if available
        if let Some(t) = token {
            Self::git_cli_with_token(path, &["push"], t)
        } else {
            Self::git_cli(path, &["push", "-u", "origin", branch])
        }
    }

    pub fn fetch_all(path: &str, token: Option<&str>) -> Result<String, String> {
        if let Some(t) = token {
            // With token, fetch from origin only (can't use --all with URL)
            Self::git_cli_with_token(path, &["fetch"], t)
        } else {
            Self::git_cli(path, &["fetch", "--all"])
        }
    }

    fn git_cli_with_token(path: &str, args: &[&str], token: &str) -> Result<String, String> {
        // Get remote URL and inject token for HTTPS authentication
        let remote_url = Self::git_cli(path, &["remote", "get-url", "origin"]).ok();
        if let Some(ref raw_url) = remote_url {
            let url = raw_url.trim();
            if url.starts_with("https://") {
                let host_and_path = if url.contains('@') {
                    let after_proto = &url[8..];
                    if let Some(at_pos) = after_proto.find('@') {
                        &after_proto[at_pos + 1..]
                    } else {
                        after_proto
                    }
                } else {
                    &url[8..]
                };
                let authed_url = format!("https://x-access-token:{}@{}", token, host_and_path);

                // Build args: subcommand <url> [rest]
                if !args.is_empty() {
                    let mut new_args: Vec<String> = vec![args[0].to_string(), authed_url];
                    for a in &args[1..] {
                        new_args.push(a.to_string());
                    }
                    let str_refs: Vec<&str> = new_args.iter().map(|s| s.as_str()).collect();

                    let output = std::process::Command::new(git_executable())
                        .args(&str_refs)
                        .current_dir(path)
                        .env("GIT_TERMINAL_PROMPT", "0")
                        .output()
                        .map_err(|e| format!("Failed to run git: {}", e))?;

                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Ok(format!("{}{}", stdout, stderr).trim().to_string());
                    } else {
                        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
                    }
                }
            }
        }
        // Fallback to normal CLI if no remote URL or SSH
        Self::git_cli(path, args)
    }

    fn git_cli(path: &str, args: &[&str]) -> Result<String, String> {
        let output = std::process::Command::new(git_executable())
            .args(args)
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run git: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(format!("{}{}", stdout, stderr).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    fn ahead_behind(
        repo: &Repository,
        branch: &git2::Branch,
        upstream: &Option<String>,
    ) -> (usize, usize) {
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

    fn time_ago(now: i64, timestamp: i64) -> String {
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

    fn index_status_label(s: git2::Status) -> String {
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

    fn wt_status_label(s: git2::Status) -> String {
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

    pub fn clone_repo(url: &str, path: &str, shallow: bool) -> Result<String, String> {
        if shallow {
            Self::git_cli_global(&["clone", "--depth", "1", url, path])
        } else {
            Self::git_cli_global(&["clone", url, path])
        }
    }

    pub fn init_repo(path: &str, branch_name: Option<&str>) -> Result<String, String> {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let branch = branch_name.unwrap_or("main");
        Self::git_cli(path, &["init", "-b", branch])
    }

    pub fn search_commits(
        path: &str,
        query: &str,
        max_count: usize,
    ) -> Result<Vec<CommitInfo>, String> {
        let all = Self::commits(path, max_count)?;
        let q = query.to_lowercase();
        let filtered = all
            .into_iter()
            .filter(|c| {
                c.message.to_lowercase().contains(&q)
                    || c.author_name.to_lowercase().contains(&q)
                    || c.author_email.to_lowercase().contains(&q)
                    || c.sha.starts_with(&q)
                    || c.short_sha.starts_with(&q)
            })
            .collect();
        Ok(filtered)
    }

    fn git_cli_global(args: &[&str]) -> Result<String, String> {
        let output = std::process::Command::new(git_executable())
            .args(args)
            .output()
            .map_err(|e| format!("Failed to run git: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(format!("{}{}", stdout, stderr).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    pub fn commit_files(path: &str, sha: &str) -> Result<Vec<CommitFileInfo>, String> {
        // Use git CLI for reliable per-file stats
        let output = Self::git_cli(
            path,
            &["diff-tree", "--no-commit-id", "--numstat", "-r", sha],
        )?;

        let mut files = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let additions = parts[0].parse::<usize>().unwrap_or(0);
                let deletions = parts[1].parse::<usize>().unwrap_or(0);
                let file_path = parts[2].to_string();
                files.push(CommitFileInfo {
                    path: file_path,
                    status: if additions > 0 && deletions > 0 {
                        "modified".to_string()
                    } else if additions > 0 {
                        "added".to_string()
                    } else if deletions > 0 {
                        "deleted".to_string()
                    } else {
                        "changed".to_string()
                    },
                    additions,
                    deletions,
                });
            }
        }

        // If numstat gave no results, try name-status for at least file list
        if files.is_empty() {
            let output2 = Self::git_cli(
                path,
                &["diff-tree", "--no-commit-id", "--name-status", "-r", sha],
            )?;
            for line in output2.lines() {
                let parts: Vec<&str> = line.splitn(2, '\t').collect();
                if parts.len() >= 2 {
                    let status = match parts[0] {
                        "A" => "added",
                        "D" => "deleted",
                        "M" => "modified",
                        "R" | "R100" => "renamed",
                        "C" => "copied",
                        _ => "changed",
                    };
                    files.push(CommitFileInfo {
                        path: parts[1].to_string(),
                        status: status.to_string(),
                        additions: 0,
                        deletions: 0,
                    });
                }
            }
        }

        Ok(files)
    }

    pub fn create_branch(path: &str, name: &str, start_point: Option<&str>) -> Result<(), String> {
        let repo = Self::open(path)?;
        let commit = if let Some(sp) = start_point {
            let obj = repo
                .revparse_single(sp)
                .map_err(|e| e.message().to_string())?;
            obj.peel_to_commit().map_err(|e| e.message().to_string())?
        } else {
            let head = repo.head().map_err(|e| e.message().to_string())?;
            head.peel_to_commit()
                .map_err(|e| e.message().to_string())?
        };
        repo.branch(name, &commit, false)
            .map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn delete_branch(path: &str, name: &str) -> Result<(), String> {
        let repo = Self::open(path)?;
        let mut branch = repo
            .find_branch(name, BranchType::Local)
            .map_err(|e| e.message().to_string())?;
        branch.delete().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn stash_list(path: &str) -> Result<Vec<StashInfo>, String> {
        let output = Self::git_cli(path, &["stash", "list"])?;
        if output.is_empty() {
            return Ok(Vec::new());
        }
        let mut stashes = Vec::new();
        for line in output.lines() {
            // Format: stash@{N}: WIP on branch: sha message
            // or: stash@{N}: On branch: message
            let idx_end = match line.find("}: ") {
                Some(pos) => pos,
                None => continue,
            };
            let index = line
                .get(7..idx_end)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(stashes.len());

            let rest = line.get(idx_end + 3..).unwrap_or("");
            let (branch, message) = if let Some(after) = rest
                .strip_prefix("WIP on ")
                .or_else(|| rest.strip_prefix("On "))
            {
                if let Some(colon_pos) = after.find(": ") {
                    (
                        after[..colon_pos].to_string(),
                        after[colon_pos + 2..].to_string(),
                    )
                } else {
                    (String::new(), rest.to_string())
                }
            } else {
                (String::new(), rest.to_string())
            };

            stashes.push(StashInfo {
                index,
                message,
                branch,
                timestamp: String::new(),
            });
        }
        Ok(stashes)
    }

    pub fn stash_push(path: &str, message: Option<&str>) -> Result<String, String> {
        if let Some(msg) = message {
            Self::git_cli(path, &["stash", "push", "-m", msg])
        } else {
            Self::git_cli(path, &["stash", "push"])
        }
    }

    pub fn stash_pop(path: &str, index: usize) -> Result<String, String> {
        Self::git_cli(path, &["stash", "pop", &format!("stash@{{{}}}", index)])
    }

    pub fn stash_apply(path: &str, index: usize) -> Result<String, String> {
        Self::git_cli(path, &["stash", "apply", &format!("stash@{{{}}}", index)])
    }

    pub fn stash_drop(path: &str, index: usize) -> Result<String, String> {
        Self::git_cli(path, &["stash", "drop", &format!("stash@{{{}}}", index)])
    }

    pub fn tags(path: &str) -> Result<Vec<TagInfo>, String> {
        let repo = Self::open(path)?;
        let mut result = Vec::new();
        let tag_names = repo.tag_names(None).map_err(|e| e.message().to_string())?;
        for name in tag_names.iter().flatten() {
            let refname = format!("refs/tags/{}", name);
            if let Ok(reference) = repo.find_reference(&refname) {
                let target = reference.target().map(|o| o.to_string()).unwrap_or_default();
                let (message, is_annotated) = if let Ok(tag) = repo.find_tag(
                    reference.target().unwrap_or(git2::Oid::zero()),
                ) {
                    (tag.message().map(|m| m.to_string()), true)
                } else {
                    (None, false)
                };
                result.push(TagInfo {
                    name: name.to_string(),
                    sha: target,
                    message,
                    is_annotated,
                });
            }
        }
        Ok(result)
    }

    pub fn run_git_command(path: &str, args: &[&str]) -> Result<String, String> {
        Self::git_cli(path, args)
    }

    pub fn discard_file(path: &str, file_path: &str) -> Result<(), String> {
        // Try checkout for tracked modified files
        let result = Self::git_cli(path, &["checkout", "--", file_path]);
        if result.is_ok() {
            return Ok(());
        }
        // For untracked files, remove them
        Self::git_cli(path, &["clean", "-f", "--", file_path])?;
        Ok(())
    }
}

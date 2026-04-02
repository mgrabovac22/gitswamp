use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use git2::{BranchType, Repository, Sort, StatusOptions};

use crate::constants::{
    AUTH_USER_BITBUCKET, AUTH_USER_GITHUB, AUTH_USER_GITLAB, AZURE_HOST, AZURE_LEGACY_HOST,
    BITBUCKET_HOST, CONFLICT_END, CONFLICT_MID, CONFLICT_START, DEFAULT_BRANCH,
    DEFAULT_COMMIT_AUTHOR, DEFAULT_COMMIT_EMAIL, GITHUB_HOST,
};
use crate::models::{
    AzureRepo, BitbucketRepo, BranchInfo, CommitFileInfo, CommitInfo, ConflictHotspot,
    FileStatusInfo, GhostBranchState, GithubRepo, GitlabRepo, RepoInfo, StagedDiffSummary,
    StashInfo, TagInfo,
};
use crate::repositories::git_repository::GitRepository;
use crate::services::diff_service::DiffService;
use crate::services::helpers::{ahead_behind, build_ref_map, build_remotes, index_status_label, time_ago, wt_status_label};
use crate::services::integration_service::IntegrationService;
use crate::services::remote_service::RemoteService;
use crate::services::stash_service::StashService;

pub struct GitService;

const GHOST_ACTIVE_KEY: &str = "gitswamp.ghost.active";
const GHOST_BASE_KEY: &str = "gitswamp.ghost.base";
const GHOST_BRANCH_KEY: &str = "gitswamp.ghost.branch";

impl GitService {
    fn resolve_worktree_file_size(repo_root: &Path, repo_relative_path: &str) -> Option<u64> {
        if repo_relative_path.trim().is_empty() {
            return None;
        }

        let full_path = repo_root.join(repo_relative_path);
        let metadata = std::fs::metadata(full_path).ok()?;
        if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        }
    }

    fn sanitize_relative_path(input: &str) -> Result<PathBuf, String> {
        let mut out = PathBuf::new();

        for component in Path::new(input).components() {
            match component {
                std::path::Component::Normal(value) => out.push(value),
                std::path::Component::CurDir => {}
                _ => return Err("Invalid relative path.".to_string()),
            }
        }

        if out.as_os_str().is_empty() {
            return Err("Path cannot be empty.".to_string());
        }

        Ok(out)
    }

    fn normalize_relative_path(path: &Path) -> String {
        path
            .to_string_lossy()
            .replace('\\', "/")
            .trim_matches('/')
            .to_string()
    }

    fn infer_scope_from_path(path: &str) -> &'static str {
        if path.contains("/auth/") {
            return "auth";
        }
        if path.contains("/api/") || path.contains("/routes/") || path.contains("/controllers/") {
            return "api";
        }
        if path.contains("/ui/") || path.contains("/view/") || path.contains("/components/") {
            return "ui";
        }
        if path.contains("/styles/") || path.ends_with(".css") || path.ends_with(".scss") {
            return "style";
        }
        if path.contains("/docs/") || path.ends_with(".md") {
            return "docs";
        }
        if path.contains("/db/") || path.contains("migration") {
            return "data";
        }
        if path.contains("/test/") || path.contains("/tests/") || path.contains(".test.") || path.contains(".spec.") {
            return "tests";
        }
        "general"
    }

    fn should_skip_empty_scan_dir(name: &str) -> bool {
        matches!(
            name,
            ".git" | "node_modules" | "target" | "dist" | "build" | ".idea" | ".vscode"
        )
    }

    fn read_repo_config(repo: &Repository, key: &str) -> Option<String> {
        repo.config()
            .ok()
            .and_then(|cfg| cfg.get_string(key).ok())
            .and_then(|value| {
                if value.trim().is_empty() {
                    None
                } else {
                    Some(value)
                }
            })
    }

    fn write_repo_config(repo: &Repository, key: &str, value: &str) -> Result<(), String> {
        let mut cfg = repo.config().map_err(|e| e.message().to_string())?;
        cfg.set_str(key, value).map_err(|e| e.message().to_string())
    }

    fn clear_repo_config(repo: &Repository, key: &str) {
        if let Ok(mut cfg) = repo.config() {
            let _ = cfg.remove(key);
        }
    }

    fn sanitize_ghost_segment(value: &str) -> String {
        let mut out = String::new();
        for ch in value.chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                out.push(ch);
            } else {
                out.push('-');
            }
        }
        out.trim_matches('-').to_string()
    }

    fn ghost_branch_name(base_branch: &str) -> String {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let safe_base = Self::sanitize_ghost_segment(base_branch);
        if safe_base.is_empty() {
            format!("ghost/session-{}", stamp)
        } else {
            format!("ghost/{}-{}", safe_base, stamp)
        }
    }

    fn current_branch_name(repo: &Repository) -> Result<String, String> {
        let head = repo.head().map_err(|e| e.message().to_string())?;
        if !head.is_branch() {
            return Err("Cannot run this operation in detached HEAD state.".to_string());
        }
        let current = head.shorthand().unwrap_or_default().trim().to_string();
        if current.is_empty() || current == "HEAD" {
            return Err("Cannot run this operation in detached HEAD state.".to_string());
        }
        Ok(current)
    }

    fn is_worktree_clean(repo: &Repository) -> Result<bool, String> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = repo
            .statuses(Some(&mut opts))
            .map_err(|e| e.message().to_string())?;
        Ok(statuses.is_empty())
    }

    fn unique_ghost_branch_name(repo: &Repository, base_branch: &str) -> String {
        let base = Self::ghost_branch_name(base_branch);
        if repo.find_branch(&base, BranchType::Local).is_err() {
            return base;
        }

        let mut counter = 2usize;
        loop {
            let candidate = format!("{}-{}", base, counter);
            if repo.find_branch(&candidate, BranchType::Local).is_err() {
                return candidate;
            }
            counter += 1;
        }
    }

    pub fn get_git_path() -> String {
        RemoteService::get_git_path()
    }

    pub fn ghost_branch_state(path: &str) -> Result<GhostBranchState, String> {
        let repo = GitRepository::open(path)?;

        let active = Self::read_repo_config(&repo, GHOST_ACTIVE_KEY)
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let base_branch = Self::read_repo_config(&repo, GHOST_BASE_KEY).unwrap_or_default();
        let ghost_branch = Self::read_repo_config(&repo, GHOST_BRANCH_KEY).unwrap_or_default();
        let branch_exists = !ghost_branch.is_empty() && repo.find_branch(&ghost_branch, BranchType::Local).is_ok();

        Ok(GhostBranchState {
            active: active && !ghost_branch.is_empty() && branch_exists,
            base_branch,
            ghost_branch,
        })
    }

    pub fn start_ghost_branch(path: &str) -> Result<GhostBranchState, String> {
        let repo = GitRepository::open(path)?;
        let existing = Self::ghost_branch_state(path)?;
        if existing.active {
            return Err(format!(
                "Ghost branch is already active: {}",
                existing.ghost_branch
            ));
        }

        if !Self::is_worktree_clean(&repo)? {
            return Err(
                "Ghost mode requires a clean working tree. Commit, stash, or discard pending changes first."
                    .to_string(),
            );
        }

        let base_branch = Self::current_branch_name(&repo)?;
        let ghost_branch = Self::unique_ghost_branch_name(&repo, &base_branch);

        let head_commit = repo
            .head()
            .map_err(|e| e.message().to_string())?
            .peel_to_commit()
            .map_err(|e| e.message().to_string())?;

        repo.branch(&ghost_branch, &head_commit, false)
            .map_err(|e| e.message().to_string())?;

        Self::checkout_branch(path, &ghost_branch)?;

        let repo_after = GitRepository::open(path)?;
        Self::write_repo_config(&repo_after, GHOST_ACTIVE_KEY, "true")?;
        Self::write_repo_config(&repo_after, GHOST_BASE_KEY, &base_branch)?;
        Self::write_repo_config(&repo_after, GHOST_BRANCH_KEY, &ghost_branch)?;

        Ok(GhostBranchState {
            active: true,
            base_branch,
            ghost_branch,
        })
    }

    pub fn materialize_ghost_branch(path: &str, name: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let branch_name = name.trim();
        if branch_name.is_empty() {
            return Err("Materialize target branch name is required.".to_string());
        }

        let state = Self::ghost_branch_state(path)?;
        if !state.active {
            return Err("Ghost mode is not active.".to_string());
        }

        if repo.find_branch(branch_name, BranchType::Local).is_ok() {
            return Err(format!("Branch '{}' already exists.", branch_name));
        }

        let current = Self::current_branch_name(&repo)?;
        if current != state.ghost_branch {
            Self::checkout_branch(path, &state.ghost_branch)?;
        }

        let repo_after = GitRepository::open(path)?;
        let mut ghost = repo_after
            .find_branch(&state.ghost_branch, BranchType::Local)
            .map_err(|e| e.message().to_string())?;
        ghost
            .rename(branch_name, false)
            .map_err(|e| e.message().to_string())?;

        Self::clear_repo_config(&repo_after, GHOST_ACTIVE_KEY);
        Self::clear_repo_config(&repo_after, GHOST_BASE_KEY);
        Self::clear_repo_config(&repo_after, GHOST_BRANCH_KEY);

        Ok(format!(
            "Ghost branch materialized as '{}'.",
            branch_name
        ))
    }

    pub fn discard_ghost_branch(path: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let state = Self::ghost_branch_state(path)?;
        if !state.active {
            return Err("Ghost mode is not active.".to_string());
        }
        if state.base_branch.trim().is_empty() {
            return Err("Ghost mode state is missing base branch metadata.".to_string());
        }

        let current = Self::current_branch_name(&repo)?;
        if current != state.ghost_branch {
            Self::checkout_branch(path, &state.ghost_branch)?;
        }

        let repo_ghost = GitRepository::open(path)?;
        let head_commit = repo_ghost
            .head()
            .map_err(|e| e.message().to_string())?
            .peel_to_commit()
            .map_err(|e| e.message().to_string())?;
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force().remove_untracked(true);
        repo_ghost
            .reset(head_commit.as_object(), git2::ResetType::Hard, Some(&mut checkout))
            .map_err(|e| e.message().to_string())?;

        Self::checkout_branch(path, &state.base_branch)?;

        let repo_after = GitRepository::open(path)?;
        if let Ok(mut ghost_branch) = repo_after.find_branch(&state.ghost_branch, BranchType::Local) {
            ghost_branch.delete().map_err(|e| e.message().to_string())?;
        }

        Self::clear_repo_config(&repo_after, GHOST_ACTIVE_KEY);
        Self::clear_repo_config(&repo_after, GHOST_BASE_KEY);
        Self::clear_repo_config(&repo_after, GHOST_BRANCH_KEY);

        Ok(format!(
            "Discarded ghost branch '{}' and restored '{}'.",
            state.ghost_branch, state.base_branch
        ))
    }

    pub fn repo_info(path: &str) -> Result<RepoInfo, String> {
        let repo = GitRepository::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = if head.is_branch() {
            head.shorthand().unwrap_or("HEAD").to_string()
        } else {
            "HEAD".to_string()
        };
        let head_sha = head.target().map(|oid| oid.to_string());
        let statuses = repo.statuses(None).map_err(|e| e.message().to_string())?;

        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());

        let remotes = build_remotes(&repo);

        Ok(RepoInfo {
            path: path.to_string(),
            name,
            current_branch: branch_name,
            is_clean: statuses.is_empty(),
            head_sha,
            remotes,
        })
    }

    pub fn commits(path: &str, max_count: usize) -> Result<Vec<CommitInfo>, String> {
        let repo = GitRepository::open(path)?;
        let mut revwalk = repo.revwalk().map_err(|e| e.message().to_string())?;

        if let Ok(branches) = repo.branches(Some(BranchType::Local)) {
            for item in branches.flatten() {
                if let Some(oid) = item.0.get().target() {
                    let _ = revwalk.push(oid);
                }
            }
        }
        if let Ok(branches) = repo.branches(Some(BranchType::Remote)) {
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

        let mut ref_map = build_ref_map(&repo);
        if let Ok(head) = repo.head() {
            if !head.is_branch() {
                if let Some(head_target) = head.target() {
                    let refs = ref_map.entry(head_target.to_string()).or_default();
                    if !refs.iter().any(|value| value == "HEAD") {
                        refs.push("HEAD".to_string());
                    }
                }
            }
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut result = Vec::new();
        let mut seen = HashSet::new();

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
                time_ago: time_ago(now, timestamp),
                parent_shas,
                refs,
            });
        }

        Ok(result)
    }

    pub fn author_deletion_stats(
        path: &str,
        max_count: usize,
    ) -> Result<Vec<(String, usize, usize)>, String> {
        let repo = GitRepository::open(path)?;
        let commits = Self::commits(path, max_count)?;
        let mut by_author: HashMap<String, (usize, usize)> = HashMap::new();

        for commit in commits {
            let oid = match git2::Oid::from_str(&commit.sha) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let commit_object = match repo.find_commit(oid) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let tree = match commit_object.tree() {
                Ok(value) => value,
                Err(_) => continue,
            };

            let parent_tree = if commit_object.parent_count() > 0 {
                commit_object
                    .parent(0)
                    .ok()
                    .and_then(|parent| parent.tree().ok())
            } else {
                None
            };

            let diff = match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let mut deletion_sum = 0usize;
            let delta_count = diff.deltas().len();
            for idx in 0..delta_count {
                if let Ok(Some(patch)) = git2::Patch::from_diff(&diff, idx) {
                    if let Ok((_, _, deletions)) = patch.line_stats() {
                        deletion_sum += deletions;
                    }
                }
            }

            if deletion_sum == 0 {
                continue;
            }

            let author = if commit.author_name.trim().is_empty() {
                "Unknown".to_string()
            } else {
                commit.author_name
            };

            let entry = by_author.entry(author).or_insert((0, 0));
            entry.0 += deletion_sum;
            entry.1 += 1;
        }

        let mut rows: Vec<(String, usize, usize)> = by_author
            .into_iter()
            .map(|(author, (deletions, commits_count))| (author, deletions, commits_count))
            .collect();

        rows.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.0.cmp(&b.0))
        });

        Ok(rows)
    }

    pub fn conflict_hotspots(path: &str, max_count: usize) -> Result<Vec<ConflictHotspot>, String> {
        let scan_limit = max_count.saturating_mul(5).max(max_count);
        let commits = Self::commits(path, scan_limit)?;

        let mut by_path: HashMap<String, (usize, usize, usize)> = HashMap::new();
        let mut inspected_merges = 0usize;

        for commit in commits {
            if commit.parent_shas.len() < 2 {
                continue;
            }
            if inspected_merges >= max_count {
                break;
            }

            inspected_merges += 1;
            let mentions_conflict = commit.message.to_lowercase().contains("conflict");

            let files = match Self::commit_files(path, &commit.sha) {
                Ok(value) => value,
                Err(_) => continue,
            };

            for file in files {
                let file_path = file.path.trim();
                if file_path.is_empty() {
                    continue;
                }

                let entry = by_path.entry(file_path.to_string()).or_insert((0, 0, 0));
                entry.0 += 1;
                entry.1 += 1;
                if mentions_conflict {
                    entry.0 += 2;
                    entry.2 += 1;
                }
            }
        }

        let mut hotspots: Vec<ConflictHotspot> = by_path
            .into_iter()
            .map(|(path, (score, merge_touches, conflict_mentions))| ConflictHotspot {
                path,
                score,
                merge_touches,
                conflict_mentions,
            })
            .collect();

        hotspots.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| b.merge_touches.cmp(&a.merge_touches))
                .then_with(|| b.conflict_mentions.cmp(&a.conflict_mentions))
                .then_with(|| a.path.cmp(&b.path))
        });

        Ok(hotspots)
    }

    pub fn commit_tree_paths(path: &str, sha: &str) -> Result<Vec<String>, String> {
        let repo = GitRepository::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        let tree = commit.tree().map_err(|e| e.message().to_string())?;

        let mut files = Vec::new();
        Self::collect_commit_tree_paths(&repo, &tree, "", &mut files)?;
        files.sort();
        Ok(files)
    }

    fn collect_commit_tree_paths(
        repo: &Repository,
        tree: &git2::Tree,
        prefix: &str,
        files: &mut Vec<String>,
    ) -> Result<(), String> {
        for entry in tree.iter() {
            let name = match entry.name() {
                Some(value) if !value.is_empty() => value,
                _ => continue,
            };

            let next_path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", prefix, name)
            };

            match entry.kind() {
                Some(git2::ObjectType::Blob) => {
                    files.push(next_path);
                }
                Some(git2::ObjectType::Tree) => {
                    let subtree = repo.find_tree(entry.id()).map_err(|e| e.message().to_string())?;
                    Self::collect_commit_tree_paths(repo, &subtree, &next_path, files)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn branches(path: &str) -> Result<Vec<BranchInfo>, String> {
        let repo = GitRepository::open(path)?;
        let head = repo.head().ok();
        let head_name = head
            .as_ref()
            .and_then(|h| if h.is_branch() { h.shorthand().map(|s| s.to_string()) } else { None });

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

                let (ahead, behind) = ahead_behind(&repo, &branch, &upstream);

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
        let repo = GitRepository::open(path)?;
        let repo_root = Path::new(path);
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = repo
            .statuses(Some(&mut opts))
            .map_err(|e| e.message().to_string())?;

        let mut files = Vec::new();
        let mut size_cache: HashMap<String, Option<u64>> = HashMap::new();

        for entry in statuses.iter() {
            let s = entry.status();
            let file_path = entry.path().unwrap_or("").to_string();
            let file_size_bytes = size_cache
                .entry(file_path.clone())
                .or_insert_with(|| Self::resolve_worktree_file_size(repo_root, &file_path))
                .to_owned();

            if s.is_index_new()
                || s.is_index_modified()
                || s.is_index_deleted()
                || s.is_index_renamed()
                || s.is_index_typechange()
            {
                files.push(FileStatusInfo {
                    path: file_path.clone(),
                    status: index_status_label(s),
                    staged: true,
                    conflicted: s.is_conflicted(),
                    file_size_bytes,
                });
            }

            if s.is_wt_new()
                || s.is_wt_modified()
                || s.is_wt_deleted()
                || s.is_wt_renamed()
                || s.is_wt_typechange()
            {
                files.push(FileStatusInfo {
                    path: file_path.clone(),
                    status: wt_status_label(s),
                    staged: false,
                    conflicted: s.is_conflicted(),
                    file_size_bytes,
                });
            }

            if s.is_conflicted() && !files.iter().any(|f| f.path == file_path) {
                files.push(FileStatusInfo {
                    path: file_path,
                    status: "conflicted".to_string(),
                    staged: false,
                    conflicted: true,
                    file_size_bytes,
                });
            }
        }

        Ok(files)
    }

    pub fn staged_diff_summary(path: &str) -> Result<StagedDiffSummary, String> {
        let repo = GitRepository::open(path)?;
        let head_tree = repo.head().ok().and_then(|head| head.peel_to_tree().ok());

        let diff = repo
            .diff_tree_to_index(head_tree.as_ref(), None, None)
            .map_err(|e| e.message().to_string())?;

        let mut total_lines_added = 0usize;
        let mut total_lines_removed = 0usize;
        let mut file_types = HashSet::new();
        let mut has_test_changes = false;
        let mut has_migration_changes = false;
        let mut scope_hits: HashMap<String, usize> = HashMap::new();

        let files_changed = diff.deltas().len();
        for idx in 0..files_changed {
            let Some(delta) = diff.get_delta(idx) else {
                continue;
            };

            let file_path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            if !file_path.is_empty() {
                let normalized = file_path.replace('\\', "/").to_lowercase();

                if normalized.contains("/test/")
                    || normalized.contains("/tests/")
                    || normalized.contains(".test.")
                    || normalized.contains(".spec.")
                {
                    has_test_changes = true;
                }

                if normalized.contains("/migration/")
                    || normalized.contains("/migrations/")
                    || normalized.contains("migrate")
                {
                    has_migration_changes = true;
                }

                let scope = Self::infer_scope_from_path(&normalized).to_string();
                let current = scope_hits.entry(scope).or_insert(0);
                *current += 1;

                if let Some(ext) = Path::new(&file_path).extension().and_then(|value| value.to_str()) {
                    let cleaned = ext.trim().to_lowercase();
                    if !cleaned.is_empty() {
                        file_types.insert(format!(".{}", cleaned));
                    }
                }
            }

            if let Ok(Some(patch)) = git2::Patch::from_diff(&diff, idx) {
                if let Ok((_, additions, deletions)) = patch.line_stats() {
                    total_lines_added += additions;
                    total_lines_removed += deletions;
                }
            }
        }

        let inferred_scope = scope_hits
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(scope, _)| scope)
            .unwrap_or_else(|| "general".to_string());

        let mut file_types_vec: Vec<String> = file_types.into_iter().collect();
        file_types_vec.sort();

        Ok(StagedDiffSummary {
            total_lines_added,
            total_lines_removed,
            files_changed,
            file_types: file_types_vec,
            has_test_changes,
            has_migration_changes,
            inferred_scope,
        })
    }

    pub fn get_empty_directories(path: &str, max_count: usize) -> Result<Vec<String>, String> {
        if max_count == 0 {
            return Ok(Vec::new());
        }

        let repo_root = Path::new(path)
            .canonicalize()
            .map_err(|e| format!("Failed to access repository root: {}", e))?;

        let mut stack = vec![repo_root.clone()];
        let mut result = Vec::new();

        while let Some(current) = stack.pop() {
            if result.len() >= max_count {
                break;
            }

            let mut has_any_entry = false;
            let entries = match std::fs::read_dir(&current) {
                Ok(v) => v,
                Err(_) => continue,
            };

            for entry in entries {
                let entry = match entry {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                has_any_entry = true;
                let file_type = match entry.file_type() {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if file_type.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if Self::should_skip_empty_scan_dir(&name) {
                        continue;
                    }
                    stack.push(entry.path());
                }
            }

            if current == repo_root || has_any_entry {
                continue;
            }

            if let Ok(relative) = current.strip_prefix(&repo_root) {
                let normalized = Self::normalize_relative_path(relative);
                if !normalized.is_empty() {
                    result.push(normalized);
                }
            }
        }

        result.sort();
        Ok(result)
    }

    pub fn add_gitkeep(path: &str, directory_path: &str, stage: bool) -> Result<String, String> {
        let repo_root = Path::new(path)
            .canonicalize()
            .map_err(|e| format!("Failed to access repository root: {}", e))?;

        let relative = Self::sanitize_relative_path(directory_path)?;
        let target_directory = repo_root.join(&relative);

        if !target_directory.exists() {
            std::fs::create_dir_all(&target_directory)
                .map_err(|e| format!("Failed to create target directory: {}", e))?;
        }

        if !target_directory.is_dir() {
            return Err("Target path is not a directory.".to_string());
        }

        let gitkeep_path = target_directory.join(".gitkeep");
        if !gitkeep_path.exists() {
            std::fs::write(&gitkeep_path, "")
                .map_err(|e| format!("Failed to create .gitkeep: {}", e))?;
        }

        let relative_gitkeep = format!("{}/.gitkeep", Self::normalize_relative_path(&relative));

        if stage {
            Self::stage_file(path, &relative_gitkeep)?;
        }

        Ok(relative_gitkeep)
    }

    pub fn stage_file(path: &str, file_path: &str) -> Result<(), String> {
        let repo = GitRepository::open(path)?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let pathspec = [Path::new(file_path)];
        index
            .add_all(pathspec.iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| e.message().to_string())?;
        index.write().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn unstage_file(path: &str, file_path: &str) -> Result<(), String> {
        let repo = GitRepository::open(path)?;
        let file = Path::new(file_path);

        if let Ok(head_obj) = repo.revparse_single("HEAD") {
            repo.reset_default(Some(&head_obj), [file])
                .map_err(|e| e.message().to_string())?;
            return Ok(());
        }

        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let _ = index.remove_path(file);
        index.write().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn create_commit(path: &str, message: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let tree_oid = index.write_tree().map_err(|e| e.message().to_string())?;
        let tree = repo.find_tree(tree_oid).map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let parent = head.peel_to_commit().map_err(|e| e.message().to_string())?;

        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
            .map_err(|e| e.message().to_string())?;

        Ok(oid.to_string())
    }

    pub fn checkout_branch(path: &str, branch_name: &str) -> Result<(), String> {
        let repo = GitRepository::open(path)?;
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
        RemoteService::pull(path, token)
    }

    pub fn push(path: &str, token: Option<&str>) -> Result<String, String> {
        RemoteService::push(path, token)
    }

    pub fn push_force(path: &str, token: Option<&str>) -> Result<String, String> {
        RemoteService::push_force(path, token)
    }

    pub fn fetch_all(path: &str, token: Option<&str>) -> Result<String, String> {
        RemoteService::fetch_all(path, token)
    }

    pub fn run_git_command(path: &str, args: &[&str]) -> Result<String, String> {
        RemoteService::run_git_command(path, args)
    }

    pub fn run_shell_command(path: &str, command: &str) -> Result<String, String> {
        RemoteService::run_shell_command(path, command)
    }

    pub fn remove_cached_all(path: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let pathspec = [Path::new(".")];

        index
            .remove_all(pathspec.iter(), None)
            .map_err(|e| e.message().to_string())?;
        index.write().map_err(|e| e.message().to_string())?;

        Ok("Removed tracked files from index (equivalent to git rm -r --cached .).".to_string())
    }

    pub fn cherry_pick(path: &str, sha: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        repo.cherrypick(&commit, None)
            .map_err(|e| e.message().to_string())?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        if index.has_conflicts() {
            return Err("Cherry-pick has conflicts. Resolve them manually.".to_string());
        }
        let tree_oid = index.write_tree().map_err(|e| e.message().to_string())?;
        let tree = repo.find_tree(tree_oid).map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let parent = head.peel_to_commit().map_err(|e| e.message().to_string())?;
        let msg = commit.message().unwrap_or("cherry-pick");
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&parent])
            .map_err(|e| e.message().to_string())?;
        repo.cleanup_state().map_err(|e| e.message().to_string())?;
        Ok("Cherry-pick complete.".to_string())
    }

    pub fn revert_commit(path: &str, sha: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        repo.revert(&commit, None)
            .map_err(|e| e.message().to_string())?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        if index.has_conflicts() {
            return Err("Revert has conflicts. Resolve them manually.".to_string());
        }
        let tree_oid = index.write_tree().map_err(|e| e.message().to_string())?;
        let tree = repo.find_tree(tree_oid).map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let parent = head.peel_to_commit().map_err(|e| e.message().to_string())?;
        let msg = format!(
            "Revert \"{}\"",
            commit
                .message()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
        );
        repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&parent])
            .map_err(|e| e.message().to_string())?;
        repo.cleanup_state().map_err(|e| e.message().to_string())?;
        Ok("Revert complete.".to_string())
    }

    pub fn reset_to_commit(path: &str, sha: &str, mode: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo
            .find_commit(oid)
            .map_err(|e| e.message().to_string())?
            .into_object();
        let reset_type = match mode {
            "soft" => git2::ResetType::Soft,
            "hard" => git2::ResetType::Hard,
            _ => git2::ResetType::Mixed,
        };
        repo.reset(&commit, reset_type, None)
            .map_err(|e| e.message().to_string())?;
        Ok(format!("Reset ({}) to {}.", mode, &sha[..7.min(sha.len())]))
    }

    pub fn checkout_commit(path: &str, sha: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        let obj = commit.as_object();
        repo.checkout_tree(obj, Some(git2::build::CheckoutBuilder::default().safe()))
            .map_err(|e| e.message().to_string())?;
        repo.set_head_detached(oid)
            .map_err(|e| e.message().to_string())?;
        Ok(format!("Checked out {}.", &sha[..7.min(sha.len())]))
    }

    pub fn create_tag_at(path: &str, name: &str, sha: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let obj = repo.find_object(oid, None).map_err(|e| e.message().to_string())?;
        repo.tag_lightweight(name, &obj, false)
            .map_err(|e| e.message().to_string())?;
        Ok(format!("Tag '{}' created.", name))
    }

    pub fn clone_repo(url: &str, path: &str, shallow: bool, token: Option<&str>) -> Result<String, String> {
        let repo_name = url.split('/').last().unwrap_or("repo").trim_end_matches(".git");
        let dest = Path::new(path).join(repo_name);

        let clone_url = url.to_string();

        let mut callbacks = git2::RemoteCallbacks::new();
        if let Some(t) = token {
            let tok = t.to_string();
            callbacks.credentials(move |remote_url, username_from_url, allowed| {
                let remote_url_lower = remote_url.to_lowercase();
                let auth_user = if remote_url_lower.contains(GITHUB_HOST) {
                    AUTH_USER_GITHUB
                } else if remote_url_lower.contains(BITBUCKET_HOST) {
                    AUTH_USER_BITBUCKET
                } else if remote_url_lower.contains(AZURE_HOST)
                    || remote_url_lower.contains(AZURE_LEGACY_HOST)
                {
                    "pat"
                } else {
                    AUTH_USER_GITLAB
                };

                if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                    let user = username_from_url.unwrap_or(auth_user);
                    return git2::Cred::userpass_plaintext(user, &tok);
                }
                if allowed.contains(git2::CredentialType::USERNAME) {
                    let user = username_from_url.unwrap_or(auth_user);
                    return git2::Cred::username(user);
                }
                git2::Cred::default()
            });
        }
        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);
        if shallow {
            fetch_opts.depth(1);
        }

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_opts);

        builder
            .clone(&clone_url, &dest)
            .map_err(|e| e.message().to_string())?;
        Ok(dest.to_string_lossy().to_string())
    }

    pub fn init_repo(path: &str, branch_name: Option<&str>) -> Result<String, String> {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let repo = Repository::init(path).map_err(|e| e.message().to_string())?;
        let branch = branch_name.unwrap_or(DEFAULT_BRANCH);

        let sig = repo
            .signature()
            .or_else(|_| git2::Signature::now(DEFAULT_COMMIT_AUTHOR, DEFAULT_COMMIT_EMAIL))
            .map_err(|e| e.message().to_string())?;

        let tree_id = repo
            .index()
            .map_err(|e| e.message().to_string())?
            .write_tree()
            .map_err(|e| e.message().to_string())?;
        let tree = repo.find_tree(tree_id).map_err(|e| e.message().to_string())?;

        repo.commit(
            Some(&format!("refs/heads/{}", branch)),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )
        .map_err(|e| e.message().to_string())?;

        drop(tree);

        repo.set_head(&format!("refs/heads/{}", branch))
            .map_err(|e| e.message().to_string())?;

        Ok(format!("Initialized repository with branch '{}'.", branch))
    }

    pub fn search_commits(path: &str, query: &str, max_count: usize) -> Result<Vec<CommitInfo>, String> {
        let repo = GitRepository::open(path)?;
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let mut revwalk = repo.revwalk().map_err(|e| e.message().to_string())?;
        if let Ok(branches) = repo.branches(Some(BranchType::Local)) {
            for item in branches.flatten() {
                if let Some(oid) = item.0.get().target() {
                    let _ = revwalk.push(oid);
                }
            }
        }
        if let Ok(branches) = repo.branches(Some(BranchType::Remote)) {
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

        let ref_map = build_ref_map(&repo);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut result = Vec::new();
        let mut seen = HashSet::new();

        for oid in revwalk.flatten() {
            if !seen.insert(oid) {
                continue;
            }

            let commit = match repo.find_commit(oid) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let sha = oid.to_string();
            let short_sha = sha[..7.min(sha.len())].to_string();
            let message = commit.message().unwrap_or("").trim().to_string();
            let author_name = commit.author().name().unwrap_or("Unknown").to_string();
            let author_email = commit.author().email().unwrap_or("").to_string();

            if !message.to_lowercase().contains(&q)
                && !author_name.to_lowercase().contains(&q)
                && !author_email.to_lowercase().contains(&q)
                && !sha.starts_with(&q)
                && !short_sha.starts_with(&q)
            {
                continue;
            }

            let timestamp = commit.time().seconds();
            let parent_shas: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();
            let refs = ref_map.get(&sha).cloned().unwrap_or_default();

            result.push(CommitInfo {
                sha,
                short_sha,
                message,
                author_name,
                author_email,
                timestamp,
                time_ago: time_ago(now, timestamp),
                parent_shas,
                refs,
            });

            if result.len() >= max_count {
                break;
            }
        }

        Ok(result)
    }

    pub fn commit_files(path: &str, sha: &str) -> Result<Vec<CommitFileInfo>, String> {
        let repo = GitRepository::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        let tree = commit.tree().map_err(|e| e.message().to_string())?;

        let parent_tree = if commit.parent_count() > 0 {
            Some(
                commit
                    .parent(0)
                    .map_err(|e| e.message().to_string())?
                    .tree()
                    .map_err(|e| e.message().to_string())?,
            )
        } else {
            None
        };

        let diff = repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
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

    pub fn create_branch(path: &str, name: &str, start_point: Option<&str>) -> Result<(), String> {
        let repo = GitRepository::open(path)?;
        let commit = if let Some(sp) = start_point {
            let obj = repo.revparse_single(sp).map_err(|e| e.message().to_string())?;
            obj.peel_to_commit().map_err(|e| e.message().to_string())?
        } else {
            let head = repo.head().map_err(|e| e.message().to_string())?;
            head.peel_to_commit().map_err(|e| e.message().to_string())?
        };
        repo.branch(name, &commit, false)
            .map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn delete_branch(path: &str, name: &str) -> Result<(), String> {
        let repo = GitRepository::open(path)?;
        let mut branch = repo
            .find_branch(name, BranchType::Local)
            .map_err(|e| e.message().to_string())?;
        branch.delete().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn stash_list(path: &str) -> Result<Vec<StashInfo>, String> {
        StashService::stash_list(path)
    }

    pub fn stash_push(path: &str, message: Option<&str>) -> Result<String, String> {
        StashService::stash_push(path, message)
    }

    pub fn stash_pop(path: &str, index: usize) -> Result<String, String> {
        StashService::stash_pop(path, index)
    }

    pub fn stash_apply(path: &str, index: usize) -> Result<String, String> {
        StashService::stash_apply(path, index)
    }

    pub fn stash_drop(path: &str, index: usize) -> Result<String, String> {
        StashService::stash_drop(path, index)
    }

    pub fn stash_files(path: &str, index: usize) -> Result<Vec<CommitFileInfo>, String> {
        StashService::stash_files(path, index)
    }

    pub fn tags(path: &str) -> Result<Vec<TagInfo>, String> {
        let repo = GitRepository::open(path)?;
        let mut result = Vec::new();
        let tag_names = repo.tag_names(None).map_err(|e| e.message().to_string())?;
        for name in tag_names.iter().flatten() {
            let refname = format!("refs/tags/{}", name);
            if let Ok(reference) = repo.find_reference(&refname) {
                let object = match repo.revparse_single(&refname) {
                    Ok(o) => o,
                    Err(_) => continue,
                };

                let target = object
                    .peel(git2::ObjectType::Commit)
                    .map(|o| o.id().to_string())
                    .unwrap_or_else(|_| object.id().to_string());

                let (message, is_annotated) = if let Some(tag_oid) = reference.target() {
                    if let Ok(tag) = repo.find_tag(tag_oid) {
                        (tag.message().map(|m| m.to_string()), true)
                    } else {
                        (None, false)
                    }
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

    pub fn delete_tag(path: &str, name: &str) -> Result<String, String> {
        GitRepository::git_cli(path, &["tag", "-d", name])
    }

    pub fn discard_file(path: &str, file_path: &str) -> Result<(), String> {
        let repo = GitRepository::open(path)?;
        let statuses = repo.statuses(None).map_err(|e| e.message().to_string())?;
        let is_conflicted = statuses
            .iter()
            .any(|s| s.path() == Some(file_path) && s.status().is_conflicted());
        if is_conflicted {
            return Self::resolve_conflict_file(path, file_path, "ours");
        }
        let is_untracked = statuses
            .iter()
            .any(|s| s.path() == Some(file_path) && s.status().contains(git2::Status::WT_NEW));
        if is_untracked {
            let full = Path::new(path).join(file_path);
            std::fs::remove_file(&full).map_err(|e| e.to_string())?;
        } else {
            repo.checkout_head(Some(
                git2::build::CheckoutBuilder::default()
                    .path(file_path)
                    .force(),
            ))
            .map_err(|e| e.message().to_string())?;
        }
        Ok(())
    }

    pub fn resolve_conflict_file(path: &str, file_path: &str, strategy: &str) -> Result<(), String> {
        let repo = GitRepository::open(path)?;
        let full_path = Path::new(path).join(file_path);
        let mut index = repo.index().map_err(|e| e.message().to_string())?;

        match strategy {
            "ours" | "theirs" => {
                let content = std::fs::read_to_string(&full_path)
                    .map_err(|e| format!("Failed to read file: {}", e))?;

                let lines: Vec<&str> = content.lines().collect();
                let mut resolved_content = Vec::new();
                let mut i = 0;

                while i < lines.len() {
                    if lines[i].starts_with(CONFLICT_START) {
                        let mut our_lines = Vec::new();
                        let mut their_lines = Vec::new();

                        i += 1;
                        while i < lines.len() {
                            if lines[i].starts_with(CONFLICT_MID) {
                                i += 1;
                                break;
                            }
                            our_lines.push(lines[i]);
                            i += 1;
                        }

                        while i < lines.len() && !lines[i].starts_with(CONFLICT_END) {
                            their_lines.push(lines[i]);
                            i += 1;
                        }

                        if i < lines.len() && lines[i].starts_with(CONFLICT_END) {
                            i += 1;
                        }

                        if strategy == "ours" {
                            resolved_content.extend(our_lines);
                        } else {
                            resolved_content.extend(their_lines);
                        }
                    } else {
                        resolved_content.push(lines[i]);
                        i += 1;
                    }
                }

                let resolved = resolved_content.join("\n");
                std::fs::write(&full_path, resolved).map_err(|e| e.to_string())?;
                index
                    .add_path(Path::new(file_path))
                    .map_err(|e| e.message().to_string())?;
            }
            "delete" => {
                if full_path.exists() {
                    std::fs::remove_file(&full_path).map_err(|e| e.to_string())?;
                }
                index
                    .remove_path(Path::new(file_path))
                    .map_err(|e| e.message().to_string())?;
            }
            _ => return Err("Invalid resolve strategy".to_string()),
        }

        index.write().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn resolve_all_conflicts(path: &str, strategy: &str) -> Result<(), String> {
        let conflicts = Self::status(path)?
            .into_iter()
            .filter(|f| f.conflicted)
            .map(|f| f.path)
            .collect::<Vec<_>>();
        for file_path in conflicts {
            Self::resolve_conflict_file(path, &file_path, strategy)?;
        }
        Ok(())
    }

    pub fn rename_branch(path: &str, old_name: &str, new_name: &str) -> Result<String, String> {
        RemoteService::rename_branch(path, old_name, new_name)
    }

    pub fn delete_remote_branch(
        path: &str,
        remote: &str,
        branch: &str,
        token: Option<&str>,
    ) -> Result<String, String> {
        RemoteService::delete_remote_branch(path, remote, branch, token)
    }

    pub fn set_upstream(path: &str, branch: &str, remote_branch: &str) -> Result<String, String> {
        RemoteService::set_upstream(path, branch, remote_branch)
    }

    pub fn edit_commit_message(path: &str, sha: &str, new_message: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let head_commit = head.peel_to_commit().map_err(|e| e.message().to_string())?;
        if head_commit.id().to_string() == sha {
            GitRepository::git_cli(path, &["commit", "--amend", "-m", new_message])
        } else {
            Err("Can only edit the message of the HEAD commit.".to_string())
        }
    }

    pub fn create_annotated_tag(path: &str, name: &str, sha: &str, message: &str) -> Result<String, String> {
        GitRepository::git_cli(path, &["tag", "-a", name, sha, "-m", message])
    }

    pub fn reset_branch_to_remote(path: &str, branch: &str) -> Result<String, String> {
        RemoteService::reset_branch_to_remote(path, branch)
    }

    pub fn search_github_repos(token: &str, query: &str) -> Result<Vec<GithubRepo>, String> {
        IntegrationService::search_github_repos(token, query)
    }

    pub fn search_gitlab_repos(domain: &str, token: &str, query: &str) -> Result<Vec<GitlabRepo>, String> {
        IntegrationService::search_gitlab_repos(domain, token, query)
    }

    pub fn search_bitbucket_repos(token: &str, query: &str) -> Result<Vec<BitbucketRepo>, String> {
        IntegrationService::search_bitbucket_repos(token, query)
    }

    pub fn search_azure_repos(domain: &str, token: &str, query: &str) -> Result<Vec<AzureRepo>, String> {
        IntegrationService::search_azure_repos(domain, token, query)
    }

    pub fn generate_ssh_key(email: &str, key_name: &str) -> Result<(String, String), String> {
        IntegrationService::generate_ssh_key(email, key_name)
    }

    pub fn add_gitlab_ssh_key(domain: &str, token: &str, title: &str, key: &str) -> Result<(), String> {
        IntegrationService::add_gitlab_ssh_key(domain, token, title, key)
    }

    pub fn verify_gitlab_token(domain: &str, token: &str) -> Result<String, String> {
        IntegrationService::verify_gitlab_token(domain, token)
    }

    pub fn get_available_external_editors() -> Vec<String> {
        IntegrationService::available_external_editors()
    }

    pub fn get_available_external_tools() -> Vec<String> {
        IntegrationService::available_external_tools()
    }

    pub fn open_file_with_editor(path: &str, file_path: &str, editor: &str) -> Result<(), String> {
        IntegrationService::open_file_with_editor(path, file_path, editor)
    }

    pub fn open_path_with_tool(path: &str, tool: &str) -> Result<(), String> {
        IntegrationService::open_path_with_tool(path, tool)
    }

    pub fn get_working_diff(path: &str, file_path: &str, staged: bool) -> Result<crate::models::FileDiff, String> {
        DiffService::get_working_diff(path, file_path, staged)
    }

    pub fn get_commit_diff(path: &str, sha: &str, file_path: &str) -> Result<crate::models::FileDiff, String> {
        DiffService::get_commit_diff(path, sha, file_path)
    }

    pub fn get_file_content(path: &str, file_path: &str, sha: Option<&str>) -> Result<String, String> {
        DiffService::get_file_content(path, file_path, sha)
    }

    pub fn get_staged_file_content(path: &str, file_path: &str) -> Result<String, String> {
        DiffService::get_staged_file_content(path, file_path)
    }

    pub fn has_conflict_markers(path: &str, file_path: &str) -> Result<bool, String> {
        DiffService::has_conflict_markers(path, file_path)
    }

    pub fn save_file_content(path: &str, file_path: &str, content: &str) -> Result<(), String> {
        DiffService::save_file_content(path, file_path, content)
    }

    pub fn revert_hunk(path: &str, file_path: &str, hunk_index: usize, staged: bool) -> Result<(), String> {
        DiffService::revert_hunk(path, file_path, hunk_index, staged)
    }

    pub fn push_to_platform(path: &str, platform: &str, token: &str, repo_name: &str) -> Result<String, String> {
        RemoteService::push_to_platform(path, platform, token, repo_name)
    }

    pub fn check_origin(path: &str) -> Result<bool, String> {
        RemoteService::check_origin(path)
    }
}

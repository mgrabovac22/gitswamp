use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use git2::{BranchType, Repository, Sort, StatusOptions};

use crate::constants::{
    AUTH_USER_BITBUCKET, AUTH_USER_GITHUB, AUTH_USER_GITLAB, AZURE_HOST, AZURE_LEGACY_HOST,
    BITBUCKET_HOST, CONFLICT_END, CONFLICT_MID, CONFLICT_START, DEFAULT_BRANCH,
    DEFAULT_COMMIT_AUTHOR, DEFAULT_COMMIT_EMAIL, GITHUB_HOST,
};
use crate::models::{
    AzureRepo, BitbucketRepo, BranchInfo, CommitFileInfo, CommitInfo, ConflictHotspot,
    ConflictPair, FileStatusInfo, GhostBranchState, GithubRepo, GitlabRepo, MergeRiskPreflight,
    RepoInfo, StagedDiffSummary, StashInfo, TagInfo,
};
use crate::repositories::git_repository::GitRepository;
use crate::services::diff_service::DiffService;
use crate::services::helpers::{
    ahead_behind, build_ref_map, build_remotes, index_status_label, time_ago, wt_status_label,
};
use crate::services::integration_service::IntegrationService;
use crate::services::remote_service::RemoteService;
use crate::services::stash_service::StashService;

pub struct GitService;

#[derive(Clone, Debug, serde::Serialize)]
pub struct CloneProgressUpdate {
    pub url: String,
    pub destination: String,
    pub phase: String,
    pub percent: u8,
    pub message: String,
    pub received_objects: usize,
    pub total_objects: usize,
    pub indexed_objects: usize,
    pub received_bytes: usize,
    pub indexed_deltas: usize,
    pub total_deltas: usize,
}

const GHOST_ACTIVE_KEY: &str = "gitswamp.ghost.active";
const GHOST_BASE_KEY: &str = "gitswamp.ghost.base";
const GHOST_BRANCH_KEY: &str = "gitswamp.ghost.branch";
const REBASE_CONFLICT_PREFIX: &str = "REBASE_CONFLICT:";

enum RebaseRunState {
    Completed,
    Conflicts,
}

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
        path.to_string_lossy()
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
        if path.contains("/test/")
            || path.contains("/tests/")
            || path.contains(".test.")
            || path.contains(".spec.")
        {
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

    fn lookback_cutoff_seconds(lookback_months: Option<u32>) -> Option<i64> {
        let months = lookback_months?;
        if months == 0 {
            return None;
        }

        let seconds = (months as i64)
            .saturating_mul(30)
            .saturating_mul(24)
            .saturating_mul(3600);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Some(now.saturating_sub(seconds))
    }

    fn is_in_lookback(timestamp: i64, cutoff: Option<i64>) -> bool {
        match cutoff {
            Some(min_ts) => timestamp >= min_ts,
            None => true,
        }
    }

    fn collect_diff_paths(diff: &git2::Diff) -> HashSet<String> {
        let mut paths = HashSet::new();

        for delta in diff.deltas() {
            let candidate = delta.new_file().path().or_else(|| delta.old_file().path());

            let Some(path) = candidate else {
                continue;
            };

            let normalized = Self::normalize_relative_path(path);
            if !normalized.is_empty() {
                paths.insert(normalized);
            }
        }

        paths
    }

    fn merge_risk_level(score: usize) -> String {
        if score >= 170 {
            return "critical".to_string();
        }
        if score >= 95 {
            return "high".to_string();
        }
        if score >= 45 {
            return "moderate".to_string();
        }
        "low".to_string()
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

    fn rebase_conflict_error(message: &str) -> String {
        format!("{} {}", REBASE_CONFLICT_PREFIX, message)
    }

    fn rebase_signature(repo: &Repository) -> Result<git2::Signature<'_>, String> {
        repo.signature()
            .or_else(|_| git2::Signature::now(DEFAULT_COMMIT_AUTHOR, DEFAULT_COMMIT_EMAIL))
            .map_err(|e| e.message().to_string())
    }

    fn branch_annotated_commit<'repo>(
        repo: &'repo Repository,
        ref_name: &str,
    ) -> Result<git2::AnnotatedCommit<'repo>, String> {
        let reference = repo
            .find_reference(ref_name)
            .map_err(|e| e.message().to_string())?;
        repo.reference_to_annotated_commit(&reference)
            .map_err(|e| e.message().to_string())
    }

    fn resolve_target_annotated_commit<'repo>(
        repo: &'repo Repository,
        target_branch: &str,
    ) -> Result<git2::AnnotatedCommit<'repo>, String> {
        let trimmed = target_branch.trim();
        if trimmed.is_empty() {
            return Err("Target branch cannot be empty.".to_string());
        }

        if trimmed.starts_with("refs/") {
            return Self::branch_annotated_commit(repo, trimmed)
                .map_err(|_| format!("Target ref '{}' was not found.", trimmed));
        }

        let local_ref = format!("refs/heads/{}", trimmed);
        if let Ok(commit) = Self::branch_annotated_commit(repo, &local_ref) {
            return Ok(commit);
        }

        let remote_ref = format!("refs/remotes/origin/{}", trimmed);
        if let Ok(commit) = Self::branch_annotated_commit(repo, &remote_ref) {
            return Ok(commit);
        }

        if let Ok(object) = repo.revparse_single(trimmed) {
            return repo
                .find_annotated_commit(object.id())
                .map_err(|e| e.message().to_string());
        }

        Err(format!(
            "Target branch '{}' was not found locally or on origin.",
            trimmed
        ))
    }

    fn ensure_local_branch_from_origin(repo: &Repository, branch_name: &str) -> Result<(), String> {
        if repo.find_branch(branch_name, BranchType::Local).is_ok() {
            return Ok(());
        }

        let remote_ref_name = format!("refs/remotes/origin/{}", branch_name);
        let remote_ref = repo
            .find_reference(&remote_ref_name)
            .map_err(|_| format!("Remote branch 'origin/{}' was not found.", branch_name))?;

        let target_oid = remote_ref.target().ok_or_else(|| {
            format!(
                "Remote branch 'origin/{}' has no target commit.",
                branch_name
            )
        })?;
        let target_commit = repo
            .find_commit(target_oid)
            .map_err(|e| e.message().to_string())?;

        repo.branch(branch_name, &target_commit, false)
            .map_err(|e| e.message().to_string())?;
        Ok(())
    }

    fn finish_rebase(repo: &Repository, rebase: &mut git2::Rebase<'_>) -> Result<(), String> {
        let sig = Self::rebase_signature(repo)?;
        rebase
            .finish(Some(&sig))
            .map_err(|e| e.message().to_string())?;
        repo.cleanup_state().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    fn rebase_step_is_noop(repo: &Repository) -> Result<bool, String> {
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        if index.has_conflicts() {
            return Ok(false);
        }

        let index_tree = index
            .write_tree_to(repo)
            .map_err(|e| e.message().to_string())?;
        let head_commit = repo
            .head()
            .map_err(|e| e.message().to_string())?
            .peel_to_commit()
            .map_err(|e| e.message().to_string())?;
        let head_tree = head_commit.tree().map_err(|e| e.message().to_string())?;

        Ok(index_tree == head_tree.id())
    }

    fn commit_rebase_step(repo: &Repository, rebase: &mut git2::Rebase<'_>) -> Result<(), String> {
        if Self::rebase_step_is_noop(repo)? {
            return Ok(());
        }

        let sig = Self::rebase_signature(repo)?;
        match rebase.commit(None, &sig, None) {
            Ok(_) => Ok(()),
            Err(e) if e.code() == git2::ErrorCode::Applied => Ok(()),
            Err(e) => Err(e.message().to_string()),
        }
    }

    fn advance_rebase_operations(
        repo: &Repository,
        rebase: &mut git2::Rebase<'_>,
    ) -> Result<RebaseRunState, String> {
        while let Some(step) = rebase.next() {
            step.map_err(|e| e.message().to_string())?;

            let index = repo.index().map_err(|e| e.message().to_string())?;
            if index.has_conflicts() {
                return Ok(RebaseRunState::Conflicts);
            }

            Self::commit_rebase_step(repo, rebase)?;
        }

        Self::finish_rebase(repo, rebase)?;
        Ok(RebaseRunState::Completed)
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
        let branch_exists =
            !ghost_branch.is_empty() && repo.find_branch(&ghost_branch, BranchType::Local).is_ok();

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

        Ok(format!("Ghost branch materialized as '{}'.", branch_name))
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
            .reset(
                head_commit.as_object(),
                git2::ResetType::Hard,
                Some(&mut checkout),
            )
            .map_err(|e| e.message().to_string())?;

        Self::checkout_branch(path, &state.base_branch)?;

        let repo_after = GitRepository::open(path)?;
        if let Ok(mut ghost_branch) = repo_after.find_branch(&state.ghost_branch, BranchType::Local)
        {
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
                committer_name: commit.committer().name().unwrap_or("Unknown").to_string(),
                committer_email: commit.committer().email().unwrap_or("").to_string(),
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
        if max_count == 0 {
            return Ok(Vec::new());
        }

        let mut revwalk = repo.revwalk().map_err(|e| e.message().to_string())?;
        revwalk.push_head().map_err(|e| e.message().to_string())?;
        revwalk
            .set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
            .map_err(|e| e.message().to_string())?;

        let mut by_author: HashMap<String, (usize, usize)> = HashMap::new();

        for oid in revwalk.flatten().take(max_count) {
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

            let deletion_sum = match diff.stats() {
                Ok(stats) => stats.deletions(),
                Err(_) => 0,
            };

            if deletion_sum == 0 {
                continue;
            }

            let author = commit_object
                .author()
                .name()
                .unwrap_or("Unknown")
                .trim()
                .to_string();
            let author = if author.is_empty() {
                "Unknown".to_string()
            } else {
                author
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

    pub fn conflict_hotspots(
        path: &str,
        max_count: usize,
        lookback_months: Option<u32>,
    ) -> Result<Vec<ConflictHotspot>, String> {
        if max_count == 0 {
            return Ok(Vec::new());
        }

        let scan_limit = if max_count > 5000 {
            max_count
        } else {
            max_count.saturating_mul(8).max(800)
        };
        let commits = Self::commits(path, scan_limit)?;
        let cutoff = Self::lookback_cutoff_seconds(lookback_months);

        let mut by_path: HashMap<String, (usize, usize, usize)> = HashMap::new();
        let mut inspected_merges = 0usize;

        for commit in commits {
            if commit.parent_shas.len() < 2 {
                continue;
            }
            if !Self::is_in_lookback(commit.timestamp, cutoff) {
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

            let mut seen_paths = HashSet::new();

            for file in files {
                let file_path = file.path.trim();
                if file_path.is_empty() {
                    continue;
                }
                if !seen_paths.insert(file_path.to_string()) {
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
            .map(
                |(path, (score, merge_touches, conflict_mentions))| ConflictHotspot {
                    path,
                    score,
                    merge_touches,
                    conflict_mentions,
                    collision_index: conflict_mentions,
                },
            )
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

    pub fn conflict_pairs(
        path: &str,
        max_count: usize,
        lookback_months: Option<u32>,
    ) -> Result<Vec<ConflictPair>, String> {
        if max_count == 0 {
            return Ok(Vec::new());
        }

        let scan_limit = if max_count > 5000 {
            max_count
        } else {
            max_count.saturating_mul(8).max(800)
        };

        let commits = Self::commits(path, scan_limit)?;
        let cutoff = Self::lookback_cutoff_seconds(lookback_months);
        let mut by_pair: HashMap<(String, String), (usize, usize, usize)> = HashMap::new();
        let mut inspected_merges = 0usize;

        for commit in commits {
            if commit.parent_shas.len() < 2 {
                continue;
            }
            if !Self::is_in_lookback(commit.timestamp, cutoff) {
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

            let mut unique_paths: Vec<String> = files
                .into_iter()
                .map(|file| file.path.trim().to_string())
                .filter(|path_value| !path_value.is_empty())
                .collect();

            unique_paths.sort();
            unique_paths.dedup();

            if unique_paths.len() > 40 {
                unique_paths.truncate(40);
            }

            if unique_paths.len() < 2 {
                continue;
            }

            for left_idx in 0..unique_paths.len() {
                for right_idx in (left_idx + 1)..unique_paths.len() {
                    let left = unique_paths[left_idx].clone();
                    let right = unique_paths[right_idx].clone();
                    let entry = by_pair.entry((left, right)).or_insert((0, 0, 0));

                    entry.0 += 1;
                    entry.1 += 1;
                    if mentions_conflict {
                        entry.0 += 2;
                        entry.2 += 1;
                    }
                }
            }
        }

        let mut pairs: Vec<ConflictPair> = by_pair
            .into_iter()
            .map(
                |((left_path, right_path), (score, co_touches, conflict_touches))| ConflictPair {
                    left_path,
                    right_path,
                    co_touches,
                    conflict_touches,
                    score,
                },
            )
            .collect();

        pairs.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| b.conflict_touches.cmp(&a.conflict_touches))
                .then_with(|| b.co_touches.cmp(&a.co_touches))
                .then_with(|| a.left_path.cmp(&b.left_path))
                .then_with(|| a.right_path.cmp(&b.right_path))
        });

        if pairs.len() > 250 {
            pairs.truncate(250);
        }

        Ok(pairs)
    }

    pub fn repository_tree_paths(path: &str, max_count: usize) -> Result<Vec<String>, String> {
        if max_count == 0 {
            return Ok(Vec::new());
        }

        let repo_root = Path::new(path)
            .canonicalize()
            .map_err(|e| format!("Failed to access repository root: {}", e))?;

        let mut stack = vec![repo_root.clone()];
        let mut files = Vec::new();

        while let Some(current) = stack.pop() {
            if files.len() >= max_count {
                break;
            }

            let entries = match std::fs::read_dir(&current) {
                Ok(value) => value,
                Err(_) => continue,
            };

            for entry in entries {
                let entry = match entry {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                let file_type = match entry.file_type() {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                if file_type.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if Self::should_skip_empty_scan_dir(&name) {
                        continue;
                    }
                    stack.push(entry.path());
                    continue;
                }

                if !file_type.is_file() {
                    continue;
                }

                if let Ok(relative) = entry.path().strip_prefix(&repo_root) {
                    let normalized = Self::normalize_relative_path(relative);
                    if !normalized.is_empty() {
                        files.push(normalized);
                    }
                }

                if files.len() >= max_count {
                    break;
                }
            }
        }

        files.sort();
        Ok(files)
    }

    fn resolve_branch_oid(
        repo: &Repository,
        branch_name: &str,
        remote: bool,
    ) -> Result<git2::Oid, String> {
        let trimmed = branch_name.trim();
        if trimmed.is_empty() {
            return Err("Branch name cannot be empty.".to_string());
        }

        let mut candidates: Vec<String> = Vec::new();
        if trimmed.starts_with("refs/") {
            candidates.push(trimmed.to_string());
        }

        if remote {
            let without_origin = trimmed.strip_prefix("origin/").unwrap_or(trimmed);
            candidates.push(format!("refs/remotes/origin/{}", without_origin));
            candidates.push(format!("origin/{}", without_origin));
        }

        let local_name = trimmed.strip_prefix("refs/heads/").unwrap_or(trimmed);
        candidates.push(format!("refs/heads/{}", local_name));
        candidates.push(local_name.to_string());

        let mut seen = HashSet::new();
        for candidate in candidates {
            if !seen.insert(candidate.clone()) {
                continue;
            }

            let object = match repo.revparse_single(&candidate) {
                Ok(value) => value,
                Err(_) => continue,
            };

            if let Ok(commit) = object.peel_to_commit() {
                return Ok(commit.id());
            }
        }

        Err(format!("Could not resolve branch reference: {}", trimmed))
    }

    pub fn merge_preflight_risk(
        path: &str,
        source_branch: &str,
        source_remote: bool,
        target_branch: &str,
        max_count: usize,
        lookback_months: Option<u32>,
    ) -> Result<MergeRiskPreflight, String> {
        let repo = GitRepository::open(path)?;

        let source_oid = Self::resolve_branch_oid(&repo, source_branch, source_remote)?;
        let target_oid = Self::resolve_branch_oid(&repo, target_branch, false)?;

        let source_commit = repo
            .find_commit(source_oid)
            .map_err(|e| e.message().to_string())?;
        let target_commit = repo
            .find_commit(target_oid)
            .map_err(|e| e.message().to_string())?;
        let base_oid = repo
            .merge_base(source_oid, target_oid)
            .map_err(|e| format!("Could not compute merge-base: {}", e.message()))?;
        let base_commit = repo
            .find_commit(base_oid)
            .map_err(|e| e.message().to_string())?;

        let source_tree = source_commit.tree().map_err(|e| e.message().to_string())?;
        let target_tree = target_commit.tree().map_err(|e| e.message().to_string())?;
        let base_tree = base_commit.tree().map_err(|e| e.message().to_string())?;

        let source_diff = repo
            .diff_tree_to_tree(Some(&base_tree), Some(&source_tree), None)
            .map_err(|e| e.message().to_string())?;
        let target_diff = repo
            .diff_tree_to_tree(Some(&base_tree), Some(&target_tree), None)
            .map_err(|e| e.message().to_string())?;

        let source_paths = Self::collect_diff_paths(&source_diff);
        let target_paths = Self::collect_diff_paths(&target_diff);

        let shared_paths: HashSet<String> =
            source_paths.intersection(&target_paths).cloned().collect();

        let hotspots = Self::conflict_hotspots(path, max_count, lookback_months)?;
        let hotspots_by_path: HashMap<String, ConflictHotspot> = hotspots
            .into_iter()
            .map(|item| (item.path.clone(), item))
            .collect();

        let mut suspect_files: Vec<ConflictHotspot> = shared_paths
            .iter()
            .filter_map(|path_value| hotspots_by_path.get(path_value).cloned())
            .collect();

        suspect_files.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| b.merge_touches.cmp(&a.merge_touches))
                .then_with(|| b.collision_index.cmp(&a.collision_index))
                .then_with(|| a.path.cmp(&b.path))
        });

        if suspect_files.len() > 20 {
            suspect_files.truncate(20);
        }

        let shared_count = shared_paths.len();
        let suspect_score_sum: usize = suspect_files.iter().map(|item| item.score).sum();
        let overlap_density_bonus = if source_paths.is_empty() {
            0
        } else {
            (shared_count.saturating_mul(100) / source_paths.len()).min(35)
        };
        let risk_score = suspect_score_sum
            .saturating_add(shared_count.saturating_mul(3))
            .saturating_add(overlap_density_bonus);

        Ok(MergeRiskPreflight {
            source_ref: if source_remote {
                format!(
                    "origin/{}",
                    source_branch.trim().trim_start_matches("origin/")
                )
            } else {
                source_branch.trim().to_string()
            },
            target_ref: target_branch.trim().to_string(),
            lookback_months,
            inspected_merges: max_count,
            risk_level: Self::merge_risk_level(risk_score),
            risk_score,
            shared_change_count: shared_count,
            suspect_count: suspect_files.len(),
            suspect_files,
        })
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
                    let subtree = repo
                        .find_tree(entry.id())
                        .map_err(|e| e.message().to_string())?;
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
        let head_name = head.as_ref().and_then(|h| {
            if h.is_branch() {
                h.shorthand().map(|s| s.to_string())
            } else {
                None
            }
        });

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

                if let Some(ext) = Path::new(&file_path)
                    .extension()
                    .and_then(|value| value.to_str())
                {
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
        let tree = repo
            .find_tree(tree_oid)
            .map_err(|e| e.message().to_string())?;
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
        let tree = repo
            .find_tree(tree_oid)
            .map_err(|e| e.message().to_string())?;
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
        let tree = repo
            .find_tree(tree_oid)
            .map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let parent = head.peel_to_commit().map_err(|e| e.message().to_string())?;
        let msg = format!(
            "Revert \"{}\"",
            commit.message().unwrap_or("").lines().next().unwrap_or("")
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

    pub fn rebase_branch_onto(
        path: &str,
        source_branch: &str,
        source_remote: bool,
        target_branch: &str,
    ) -> Result<String, String> {
        let source = source_branch.trim();
        let target = target_branch.trim();

        if source.is_empty() {
            return Err("Source branch cannot be empty.".to_string());
        }
        if target.is_empty() {
            return Err("Target branch cannot be empty.".to_string());
        }
        if source == target {
            return Err("Source and target branches are the same.".to_string());
        }

        let repo = GitRepository::open(path)?;
        if source_remote {
            Self::ensure_local_branch_from_origin(&repo, source)?;
        }
        drop(repo);

        Self::checkout_branch(path, source)?;

        let repo = GitRepository::open(path)?;
        let target_annotated = Self::resolve_target_annotated_commit(&repo, target)?;
        let mut rebase = repo
            .rebase(None, Some(&target_annotated), None, None)
            .map_err(|e| e.message().to_string())?;

        match Self::advance_rebase_operations(&repo, &mut rebase)? {
            RebaseRunState::Completed => Ok(format!("Rebased '{}' onto '{}'.", source, target)),
            RebaseRunState::Conflicts => Err(Self::rebase_conflict_error(
                "Rebase has conflicts. Resolve files, then use Continue, Skip, or Abort.",
            )),
        }
    }

    pub fn rebase_continue(path: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let mut rebase = repo
            .open_rebase(None)
            .map_err(|_| "No active rebase operation.".to_string())?;

        let index = repo.index().map_err(|e| e.message().to_string())?;
        if index.has_conflicts() {
            return Err(Self::rebase_conflict_error(
                "Resolve all rebase conflicts before continuing.",
            ));
        }

        if rebase.operation_current().is_some() {
            Self::commit_rebase_step(&repo, &mut rebase)?;
        }

        match Self::advance_rebase_operations(&repo, &mut rebase)? {
            RebaseRunState::Completed => Ok("Rebase continue completed.".to_string()),
            RebaseRunState::Conflicts => Err(Self::rebase_conflict_error(
                "Rebase stopped on another conflicting patch.",
            )),
        }
    }

    pub fn rebase_abort(path: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let mut rebase = repo
            .open_rebase(None)
            .map_err(|_| "No active rebase operation.".to_string())?;

        rebase.abort().map_err(|e| e.message().to_string())?;
        let _ = repo.cleanup_state();
        Ok("Rebase aborted.".to_string())
    }

    pub fn rebase_skip(path: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let mut rebase = repo
            .open_rebase(None)
            .map_err(|_| "No active rebase operation.".to_string())?;

        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|e| e.message().to_string())?;

        let current_before = rebase.operation_current();
        match rebase.next() {
            Some(Ok(_)) => {
                if current_before == rebase.operation_current() {
                    return Err("Unable to advance rebase operation while skipping.".to_string());
                }

                let index = repo.index().map_err(|e| e.message().to_string())?;
                if index.has_conflicts() {
                    return Err(Self::rebase_conflict_error(
                        "Skip moved to the next patch, but it also has conflicts.",
                    ));
                }

                Self::commit_rebase_step(&repo, &mut rebase)?;
            }
            Some(Err(e)) => return Err(e.message().to_string()),
            None => {
                Self::finish_rebase(&repo, &mut rebase)?;
                return Ok("Rebase finished after skipping the current patch.".to_string());
            }
        }

        match Self::advance_rebase_operations(&repo, &mut rebase)? {
            RebaseRunState::Completed => {
                Ok("Skipped current patch and continued rebase.".to_string())
            }
            RebaseRunState::Conflicts => Err(Self::rebase_conflict_error(
                "Rebase stopped on another conflicting patch.",
            )),
        }
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
        let obj = repo
            .find_object(oid, None)
            .map_err(|e| e.message().to_string())?;
        repo.tag_lightweight(name, &obj, false)
            .map_err(|e| e.message().to_string())?;
        Ok(format!("Tag '{}' created.", name))
    }

    pub fn clone_repo_with_progress<F>(
        url: &str,
        path: &str,
        shallow: bool,
        token: Option<&str>,
        mut progress: F,
    ) -> Result<String, String>
    where
        F: FnMut(CloneProgressUpdate),
    {
        let repo_name = url
            .split('/')
            .last()
            .unwrap_or("repo")
            .trim_end_matches(".git");
        let dest = Path::new(path).join(repo_name);

        let clone_url = url.to_string();
        let destination = dest.to_string_lossy().to_string();

        progress(CloneProgressUpdate {
            url: clone_url.clone(),
            destination: destination.clone(),
            phase: "preparing".to_string(),
            percent: 0,
            message: "Preparing clone...".to_string(),
            received_objects: 0,
            total_objects: 0,
            indexed_objects: 0,
            received_bytes: 0,
            indexed_deltas: 0,
            total_deltas: 0,
        });

        let progress_url = clone_url.clone();
        let progress_destination = destination.clone();
        let mut last_emit = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(Instant::now);

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.transfer_progress(move |stats| {
            let total_objects = stats.total_objects();
            let received_objects = stats.received_objects();
            let indexed_objects = stats.indexed_objects();
            let received_bytes = stats.received_bytes();
            let indexed_deltas = stats.indexed_deltas();
            let total_deltas = stats.total_deltas();

            let now = Instant::now();
            let completed = total_objects > 0 && indexed_objects >= total_objects;
            if !completed && now.duration_since(last_emit) < Duration::from_millis(120) {
                return true;
            }
            last_emit = now;

            let phase = if total_objects == 0 || received_objects < total_objects {
                "receiving"
            } else if indexed_objects < total_objects {
                "indexing"
            } else {
                "finalizing"
            };

            let mut percent = if total_objects > 0 {
                (((received_objects.max(indexed_objects)) as f64 / total_objects as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u8
            } else {
                0
            };

            if phase != "finalizing" {
                percent = percent.min(99);
            }

            let message = if total_objects > 0 {
                format!(
                    "{} objects: {}/{}",
                    phase,
                    received_objects.max(indexed_objects),
                    total_objects
                )
            } else {
                "Receiving objects...".to_string()
            };

            progress(CloneProgressUpdate {
                url: progress_url.clone(),
                destination: progress_destination.clone(),
                phase: phase.to_string(),
                percent,
                message,
                received_objects,
                total_objects,
                indexed_objects,
                received_bytes,
                indexed_deltas,
                total_deltas,
            });

            true
        });

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
        Ok(destination)
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
        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| e.message().to_string())?;

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

    pub fn search_commits(
        path: &str,
        query: &str,
        max_count: usize,
    ) -> Result<Vec<CommitInfo>, String> {
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
                committer_name: commit.committer().name().unwrap_or("Unknown").to_string(),
                committer_email: commit.committer().email().unwrap_or("").to_string(),
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
            let obj = repo
                .revparse_single(sp)
                .map_err(|e| e.message().to_string())?;
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

    fn remove_worktree_path(repo_root: &Path, file_path: &str) -> Result<(), String> {
        let relative = Self::sanitize_relative_path(file_path)?;
        let full = repo_root.join(relative);
        if full.is_dir() {
            std::fs::remove_dir_all(&full).map_err(|e| e.to_string())?;
        } else if full.exists() {
            std::fs::remove_file(&full).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn discard_file_with_statuses(
        repo: &Repository,
        repo_root: &Path,
        statuses: &git2::Statuses<'_>,
        file_path: &str,
    ) -> Result<(), String> {
        let is_conflicted = statuses
            .iter()
            .any(|s| s.path() == Some(file_path) && s.status().is_conflicted());
        if is_conflicted {
            return Self::resolve_conflict_file(&repo_root.to_string_lossy(), file_path, "ours");
        }

        let is_untracked = statuses
            .iter()
            .any(|s| s.path() == Some(file_path) && s.status().contains(git2::Status::WT_NEW));
        let has_index_entry = repo
            .index()
            .map_err(|e| e.message().to_string())?
            .get_path(Path::new(file_path), 0)
            .is_some();

        if is_untracked || !has_index_entry {
            Self::remove_worktree_path(repo_root, file_path)?;
        } else {
            repo.checkout_index(
                None,
                Some(
                    git2::build::CheckoutBuilder::default()
                        .path(file_path)
                        .force(),
                ),
            )
            .map_err(|e| e.message().to_string())?;
        }
        Ok(())
    }

    pub fn discard_file(path: &str, file_path: &str) -> Result<(), String> {
        let repo = GitRepository::open(path)?;
        let repo_root = Path::new(path);
        let statuses = repo.statuses(None).map_err(|e| e.message().to_string())?;
        Self::discard_file_with_statuses(&repo, repo_root, &statuses, file_path)
    }

    pub fn discard_files(path: &str, file_paths: &[String]) -> Result<(), String> {
        let repo = GitRepository::open(path)?;
        let repo_root = Path::new(path);
        let statuses = repo.statuses(None).map_err(|e| e.message().to_string())?;
        for file_path in file_paths {
            Self::discard_file_with_statuses(&repo, repo_root, &statuses, file_path)?;
        }
        Ok(())
    }

    pub fn resolve_conflict_file(
        path: &str,
        file_path: &str,
        strategy: &str,
    ) -> Result<(), String> {
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

    pub fn create_annotated_tag(
        path: &str,
        name: &str,
        sha: &str,
        message: &str,
    ) -> Result<String, String> {
        GitRepository::git_cli(path, &["tag", "-a", name, sha, "-m", message])
    }

    pub fn reset_branch_to_remote(path: &str, branch: &str) -> Result<String, String> {
        RemoteService::reset_branch_to_remote(path, branch)
    }

    pub fn search_github_repos(
        token: &str,
        query: &str,
        include_public: bool,
    ) -> Result<Vec<GithubRepo>, String> {
        IntegrationService::search_github_repos(token, query, include_public)
    }

    pub fn search_gitlab_repos(
        domain: &str,
        token: &str,
        query: &str,
    ) -> Result<Vec<GitlabRepo>, String> {
        IntegrationService::search_gitlab_repos(domain, token, query)
    }

    pub fn search_bitbucket_repos(token: &str, query: &str) -> Result<Vec<BitbucketRepo>, String> {
        IntegrationService::search_bitbucket_repos(token, query)
    }

    pub fn search_azure_repos(
        domain: &str,
        token: &str,
        query: &str,
    ) -> Result<Vec<AzureRepo>, String> {
        IntegrationService::search_azure_repos(domain, token, query)
    }

    pub fn generate_ssh_key(email: &str, key_name: &str) -> Result<(String, String), String> {
        IntegrationService::generate_ssh_key(email, key_name)
    }

    pub fn add_gitlab_ssh_key(
        domain: &str,
        token: &str,
        title: &str,
        key: &str,
    ) -> Result<(), String> {
        IntegrationService::add_gitlab_ssh_key(domain, token, title, key)
    }

    pub fn add_github_ssh_key(token: &str, title: &str, key: &str) -> Result<(), String> {
        IntegrationService::add_github_ssh_key(token, title, key)
    }

    pub fn list_github_ssh_keys(token: &str) -> Result<Vec<crate::models::GithubSshKey>, String> {
        IntegrationService::list_github_ssh_keys(token)
    }

    pub fn delete_github_ssh_key(token: &str, key_id: u64) -> Result<(), String> {
        IntegrationService::delete_github_ssh_key(token, key_id)
    }

    pub fn verify_github_token(token: &str) -> Result<String, String> {
        IntegrationService::verify_github_token(token)
    }

    pub fn load_ssh_public_key_from_file(file_path: &str) -> Result<String, String> {
        IntegrationService::load_ssh_public_key_from_file(file_path)
    }

    pub fn connect_github_oauth_via_gh_cli() -> Result<String, String> {
        IntegrationService::connect_github_oauth_via_gh_cli()
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

    pub fn get_working_diff(
        path: &str,
        file_path: &str,
        staged: bool,
    ) -> Result<crate::models::FileDiff, String> {
        DiffService::get_working_diff(path, file_path, staged)
    }

    pub fn get_commit_diff(
        path: &str,
        sha: &str,
        file_path: &str,
    ) -> Result<crate::models::FileDiff, String> {
        DiffService::get_commit_diff(path, sha, file_path)
    }

    pub fn get_file_content(
        path: &str,
        file_path: &str,
        sha: Option<&str>,
    ) -> Result<String, String> {
        DiffService::get_file_content(path, file_path, sha)
    }

    pub fn get_staged_file_content(path: &str, file_path: &str) -> Result<String, String> {
        DiffService::get_staged_file_content(path, file_path)
    }

    pub fn get_file_blame(
        path: &str,
        file_path: &str,
        sha: Option<&str>,
    ) -> Result<Vec<crate::models::FileBlameLine>, String> {
        DiffService::get_file_blame(path, file_path, sha)
    }

    pub fn has_conflict_markers(path: &str, file_path: &str) -> Result<bool, String> {
        DiffService::has_conflict_markers(path, file_path)
    }

    pub fn save_file_content(path: &str, file_path: &str, content: &str) -> Result<(), String> {
        DiffService::save_file_content(path, file_path, content)
    }

    pub fn revert_hunk(
        path: &str,
        file_path: &str,
        hunk_index: usize,
        staged: bool,
    ) -> Result<(), String> {
        DiffService::revert_hunk(path, file_path, hunk_index, staged)
    }

    pub fn push_to_platform(
        path: &str,
        platform: &str,
        token: &str,
        repo_name: &str,
    ) -> Result<String, String> {
        RemoteService::push_to_platform(path, platform, token, repo_name)
    }

    pub fn check_origin(path: &str) -> Result<bool, String> {
        RemoteService::check_origin(path)
    }
}

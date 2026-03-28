use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use git2::{BranchType, Repository, Sort, StatusOptions};

use crate::constants::{
    AUTH_USER_GITHUB, AUTH_USER_GITLAB, CONFLICT_END, CONFLICT_MID, CONFLICT_START,
    DEFAULT_BRANCH, DEFAULT_COMMIT_AUTHOR, DEFAULT_COMMIT_EMAIL, GITHUB_HOST,
};
use crate::models::{
    BranchInfo, CommitFileInfo, CommitInfo, FileStatusInfo, GithubRepo, GitlabRepo, RepoInfo,
    StashInfo, TagInfo,
};
use crate::repositories::git_repository::GitRepository;
use crate::services::diff_service::DiffService;
use crate::services::helpers::{ahead_behind, build_ref_map, build_remotes, index_status_label, time_ago, wt_status_label};
use crate::services::integration_service::IntegrationService;
use crate::services::remote_service::RemoteService;
use crate::services::stash_service::StashService;

pub struct GitService;

impl GitService {
    pub fn get_git_path() -> String {
        RemoteService::get_git_path()
    }

    pub fn repo_info(path: &str) -> Result<RepoInfo, String> {
        let repo = GitRepository::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = head.shorthand().unwrap_or("HEAD").to_string();
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

        let ref_map = build_ref_map(&repo);

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

    pub fn branches(path: &str) -> Result<Vec<BranchInfo>, String> {
        let repo = GitRepository::open(path)?;
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
                    status: index_status_label(s),
                    staged: true,
                    conflicted: s.is_conflicted(),
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
                });
            }

            if s.is_conflicted() && !files.iter().any(|f| f.path == file_path) {
                files.push(FileStatusInfo {
                    path: file_path,
                    status: "conflicted".to_string(),
                    staged: false,
                    conflicted: true,
                });
            }
        }

        Ok(files)
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
                if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                    let user = if remote_url.contains(GITHUB_HOST) {
                        AUTH_USER_GITHUB
                    } else {
                        username_from_url.unwrap_or(AUTH_USER_GITLAB)
                    };
                    return git2::Cred::userpass_plaintext(user, &tok);
                }
                if allowed.contains(git2::CredentialType::USERNAME) {
                    let user = if remote_url.contains(GITHUB_HOST) {
                        AUTH_USER_GITHUB
                    } else {
                        username_from_url.unwrap_or(AUTH_USER_GITLAB)
                    };
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

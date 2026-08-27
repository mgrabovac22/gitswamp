use std::path::Path;

use crate::constants::{
    AUTH_USER_BITBUCKET, AUTH_USER_GITHUB, AUTH_USER_GITLAB, AZURE_HOST, AZURE_LEGACY_HOST,
    BITBUCKET_HOST, GITHUB_HOST, GITLAB_HOST, HTTPS_SCHEME, PLATFORM_AZURE, PLATFORM_BITBUCKET,
    PLATFORM_GITHUB, PLATFORM_GITHUB_ENTERPRISE, PLATFORM_GITLAB, PLATFORM_GITLAB_SELF_HOSTED,
    TEMP_PUSH_REMOTE_AUTH, TEMP_PUSH_REMOTE_PLATFORM,
};
use crate::repositories::git_repository::GitRepository;
use crate::services::helpers::urlencoded;
use crate::services::stash_service::StashService;

pub struct RemoteService;

enum PullPlan {
    UpToDate,
    FastForward {
        branch: String,
        remote_oid: git2::Oid,
    },
    Merge {
        branch: String,
        remote_oid: git2::Oid,
    },
}

impl RemoteService {
    pub fn get_git_path() -> String {
        GitRepository::get_git_path()
    }

    fn local_tag_refspecs(repo: &git2::Repository) -> Result<Vec<String>, String> {
        let mut refspecs = Vec::new();
        let tag_names = repo.tag_names(None).map_err(|e| e.message().to_string())?;

        for name in tag_names.iter().flatten() {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                continue;
            }
            refspecs.push(format!("refs/tags/{}:refs/tags/{}", trimmed, trimmed));
        }

        Ok(refspecs)
    }

    fn push_unique_candidate(candidates: &mut Vec<String>, candidate: &str) {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            return;
        }
        if !candidates.iter().any(|value| value == trimmed) {
            candidates.push(trimmed.to_string());
        }
    }

    fn auth_username_candidates(remote_url: &str, username_from_url: Option<&str>) -> Vec<String> {
        let host_and_path = remote_url.strip_prefix(HTTPS_SCHEME).unwrap_or(remote_url);
        let host = host_and_path
            .split('/')
            .next()
            .unwrap_or_default()
            .to_lowercase();
        let host_and_path_lower = host_and_path.to_lowercase();

        let mut candidates: Vec<String> = Vec::new();

        if let Some(user) = username_from_url {
            Self::push_unique_candidate(&mut candidates, user);
        }

        if host.contains(BITBUCKET_HOST) {
            Self::push_unique_candidate(&mut candidates, AUTH_USER_BITBUCKET);
            Self::push_unique_candidate(&mut candidates, AUTH_USER_GITHUB);
            Self::push_unique_candidate(&mut candidates, AUTH_USER_GITLAB);
            return candidates;
        }

        if host.contains(AZURE_HOST) || host.contains(AZURE_LEGACY_HOST) {
            Self::push_unique_candidate(&mut candidates, "pat");
            Self::push_unique_candidate(&mut candidates, AUTH_USER_GITHUB);
            Self::push_unique_candidate(&mut candidates, AUTH_USER_GITLAB);
            return candidates;
        }

        if host == GITLAB_HOST
            || host.contains("gitlab.")
            || host_and_path_lower.contains("/gitlab")
        {
            Self::push_unique_candidate(&mut candidates, AUTH_USER_GITLAB);
            Self::push_unique_candidate(&mut candidates, AUTH_USER_GITHUB);
            Self::push_unique_candidate(&mut candidates, AUTH_USER_BITBUCKET);
            return candidates;
        }

        if host == GITHUB_HOST || host.contains("github.") {
            Self::push_unique_candidate(&mut candidates, AUTH_USER_GITHUB);
            Self::push_unique_candidate(&mut candidates, AUTH_USER_GITLAB);
            Self::push_unique_candidate(&mut candidates, AUTH_USER_BITBUCKET);
            return candidates;
        }

        // Unknown hosts include self-hosted platforms, so prefer GitLab's oauth2 alias first
        // and then fall back to other common token usernames.
        Self::push_unique_candidate(&mut candidates, AUTH_USER_GITLAB);
        Self::push_unique_candidate(&mut candidates, AUTH_USER_GITHUB);
        Self::push_unique_candidate(&mut candidates, AUTH_USER_BITBUCKET);

        candidates
    }

    fn authenticated_url_with_username(
        remote_url: &str,
        username: &str,
        token: &str,
    ) -> Option<String> {
        if !remote_url.starts_with(HTTPS_SCHEME) || remote_url.contains('@') {
            return None;
        }

        let host_and_path = &remote_url[HTTPS_SCHEME.len()..];
        let enc_token = urlencoded(token);
        Some(format!(
            "{}{}:{}@{}",
            HTTPS_SCHEME, username, enc_token, host_and_path
        ))
    }

    fn home_dir() -> Option<std::path::PathBuf> {
        std::env::var("HOME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("USERPROFILE")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
                    .map(std::path::PathBuf::from)
            })
    }

    fn default_ssh_private_keys() -> Vec<std::path::PathBuf> {
        let mut keys = Vec::new();
        if let Some(home) = Self::home_dir() {
            let ssh_dir = home.join(".ssh");
            for name in ["id_ed25519", "id_rsa", "id_ecdsa"] {
                let key = ssh_dir.join(name);
                if key.exists() {
                    keys.push(key);
                }
            }
        }
        keys
    }

    fn build_remote_callbacks(
        token: Option<&str>,
        configured_url: &str,
    ) -> git2::RemoteCallbacks<'static> {
        let tok = token.map(|value| value.to_string());
        let configured_url = configured_url.to_string();
        let ssh_keys = Self::default_ssh_private_keys();
        let mut https_attempt = 0usize;
        let mut ssh_attempt = 0usize;

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(move |url, username_from_url, allowed| {
            let current_url = if url.is_empty() {
                configured_url.as_str()
            } else {
                url
            };
            let username_candidates =
                Self::auth_username_candidates(current_url, username_from_url);
            let default_user = username_candidates
                .first()
                .map(|value| value.as_str())
                .unwrap_or(AUTH_USER_GITLAB);

            if allowed.contains(git2::CredentialType::SSH_KEY) {
                let ssh_user = username_from_url.unwrap_or("git");

                if let Ok(cred) = git2::Cred::ssh_key_from_agent(ssh_user) {
                    return Ok(cred);
                }

                if !ssh_keys.is_empty() {
                    let key = ssh_keys
                        .get(ssh_attempt)
                        .or_else(|| ssh_keys.last())
                        .expect("ssh key candidates are not empty");
                    ssh_attempt = ssh_attempt.saturating_add(1);
                    if let Ok(cred) = git2::Cred::ssh_key(ssh_user, None, key, None) {
                        return Ok(cred);
                    }
                }
            }

            if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                if let Some(tok) = &tok {
                    let user = username_candidates
                        .get(https_attempt)
                        .or_else(|| username_candidates.last())
                        .map(|value| value.as_str())
                        .unwrap_or(AUTH_USER_GITLAB);
                    https_attempt = https_attempt.saturating_add(1);
                    return git2::Cred::userpass_plaintext(user, tok);
                }
            }

            if allowed.contains(git2::CredentialType::USERNAME) {
                return git2::Cred::username(default_user);
            }

            if allowed.contains(git2::CredentialType::DEFAULT) {
                if let Ok(cred) = git2::Cred::default() {
                    return Ok(cred);
                }
            }

            Err(git2::Error::from_str(
                "Authentication required but no usable credentials were available for this remote",
            ))
        });

        callbacks
    }

    pub fn fetch_all(path: &str, token: Option<&str>) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let remotes = repo.remotes().map_err(|e| e.message().to_string())?;
        let remote_names: Vec<String> = remotes
            .iter()
            .filter_map(|n| n.map(|s| s.to_string()))
            .collect();

        for remote_name in &remote_names {
            let mut remote = repo
                .find_remote(remote_name)
                .map_err(|e| e.message().to_string())?;

            let configured_url = remote.url().unwrap_or_default().to_string();
            let callbacks = Self::build_remote_callbacks(token, &configured_url);
            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.remote_callbacks(callbacks);
            fetch_opts.prune(git2::FetchPrune::On);

            remote
                .fetch::<&str>(&[], Some(&mut fetch_opts), None)
                .map_err(|e| format!("Fetch '{}' failed: {}", remote_name, e.message()))?;
        }
        Ok(format!(
            "Fetched {} remote(s) and pruned deleted remote branches.",
            remote_names.len()
        ))
    }

    fn pull_plan(path: &str) -> Result<PullPlan, String> {
        let repo = GitRepository::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = head.shorthand().unwrap_or("main").to_string();
        let remote_ref = format!("refs/remotes/origin/{}", branch_name);
        let remote_oid = repo
            .refname_to_id(&remote_ref)
            .map_err(|_| format!("No remote tracking branch for '{}'", branch_name))?;
        let remote_commit = repo
            .find_annotated_commit(remote_oid)
            .map_err(|e| e.message().to_string())?;
        let (analysis, _) = repo
            .merge_analysis(&[&remote_commit])
            .map_err(|e| e.message().to_string())?;
        if analysis.is_up_to_date() {
            return Ok(PullPlan::UpToDate);
        }
        if analysis.is_fast_forward() {
            return Ok(PullPlan::FastForward {
                branch: branch_name,
                remote_oid,
            });
        }
        if analysis.is_normal() {
            return Ok(PullPlan::Merge {
                branch: branch_name,
                remote_oid,
            });
        }
        Err("Pull failed: unexpected merge state.".to_string())
    }

    fn worktree_change_summary(path: &str) -> Result<(bool, bool), String> {
        let repo = GitRepository::open(path)?;
        let mut options = git2::StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(false)
            .include_ignored(false);
        let statuses = repo
            .statuses(Some(&mut options))
            .map_err(|e| e.message().to_string())?;
        let has_changes = !statuses.is_empty();
        let has_tracked = statuses
            .iter()
            .any(|entry| entry.status() != git2::Status::WT_NEW);
        Ok((has_changes, has_tracked))
    }

    fn execute_pull_plan(path: &str, plan: PullPlan) -> Result<String, String> {
        match plan {
            PullPlan::UpToDate => Ok("Already up to date.".to_string()),
            PullPlan::FastForward { branch, remote_oid } => {
                let repo = GitRepository::open(path)?;
                let target = repo
                    .find_object(remote_oid, None)
                    .map_err(|e| e.message().to_string())?;
                let mut checkout = git2::build::CheckoutBuilder::new();
                checkout.safe();
                repo.checkout_tree(&target, Some(&mut checkout))
                    .map_err(|e| e.message().to_string())?;
                let mut reference = repo
                    .find_reference(&format!("refs/heads/{}", branch))
                    .map_err(|e| e.message().to_string())?;
                reference
                    .set_target(remote_oid, "pull: fast-forward")
                    .map_err(|e| e.message().to_string())?;
                repo.set_head(&format!("refs/heads/{}", branch))
                    .map_err(|e| e.message().to_string())?;
                Ok("Fast-forward merge complete.".to_string())
            }
            PullPlan::Merge { branch, remote_oid } => {
                let repo = GitRepository::open(path)?;
                let remote_commit = repo
                    .find_annotated_commit(remote_oid)
                    .map_err(|e| e.message().to_string())?;
                repo.merge(&[&remote_commit], None, None)
                    .map_err(|e| e.message().to_string())?;
                let index = repo.index().map_err(|e| e.message().to_string())?;
                if index.has_conflicts() {
                    return Err("Merge conflicts detected. Resolve them manually.".to_string());
                }
                let mut index = repo.index().map_err(|e| e.message().to_string())?;
                let tree_oid = index.write_tree().map_err(|e| e.message().to_string())?;
                let tree = repo
                    .find_tree(tree_oid)
                    .map_err(|e| e.message().to_string())?;
                let head_commit = repo
                    .head()
                    .map_err(|e| e.message().to_string())?
                    .peel_to_commit()
                    .map_err(|e| e.message().to_string())?;
                let remote_commit_obj = repo
                    .find_commit(remote_oid)
                    .map_err(|e| e.message().to_string())?;
                let sig = repo.signature().map_err(|e| e.message().to_string())?;
                let msg = format!("Merge branch '{}' of origin into {}", branch, branch);
                repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &msg,
                    &tree,
                    &[&head_commit, &remote_commit_obj],
                )
                .map_err(|e| e.message().to_string())?;
                repo.cleanup_state().map_err(|e| e.message().to_string())?;
                Ok("Merge complete.".to_string())
            }
        }
    }

    fn safety_stash_message() -> String {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        format!("GitSwamp pull safety {}", timestamp)
    }

    pub fn pull(path: &str, token: Option<&str>, auto_stash: bool) -> Result<String, String> {
        Self::fetch_all(path, token)?;
        let plan = Self::pull_plan(path)?;
        if matches!(&plan, PullPlan::UpToDate) {
            return Ok("Already up to date. Local changes were left untouched.".to_string());
        }

        let (has_local_changes, has_tracked_changes) = Self::worktree_change_summary(path)?;
        if has_local_changes && !auto_stash {
            return Err(
                "WORKTREE_DIRTY: Pull stopped because local working changes are present."
                    .to_string(),
            );
        }

        // Leave untracked files in place. Safe checkout refuses to overwrite them,
        // avoiding an expensive stash of generated directories such as node_modules.
        let safety_stash = if has_tracked_changes {
            Some(StashService::stash_push_oid(
                path,
                Some(&Self::safety_stash_message()),
                false,
            )?)
        } else {
            None
        };

        let pull_result = Self::execute_pull_plan(path, plan);
        match (pull_result, safety_stash) {
            (Ok(result), None) if has_local_changes => {
                Ok(format!("{} Untracked files were left untouched.", result))
            }
            (Ok(result), None) => Ok(result),
            (Ok(result), Some(stash_oid)) => {
                StashService::restore_stash_with_index(path, stash_oid).map_err(|restore_error| {
                    format!(
                        "PULL_SUCCEEDED_STASH_RESTORE_FAILED: {} Local changes remain safe in stash {}. {}",
                        result, stash_oid, restore_error
                    )
                })?;
                Ok(format!("{} Local changes were restored.", result))
            }
            (Err(pull_error), None) => Err(pull_error),
            (Err(pull_error), Some(stash_oid)) => {
                let repo_state_is_clean = GitRepository::open(path)
                    .map(|repo| repo.state() == git2::RepositoryState::Clean)
                    .unwrap_or(false);

                if repo_state_is_clean {
                    match StashService::restore_stash_with_index(path, stash_oid) {
                        Ok(_) => Err(format!(
                            "Pull failed, and local changes were restored: {}",
                            pull_error
                        )),
                        Err(restore_error) => Err(format!(
                            "PULL_FAILED_STASH_RETAINED: Pull failed: {} Local changes remain safe in stash {}. {}",
                            pull_error, stash_oid, restore_error
                        )),
                    }
                } else {
                    Err(format!(
                        "PULL_FAILED_STASH_RETAINED: Pull stopped in a Git operation state: {} Local changes remain safe in stash {}.",
                        pull_error, stash_oid
                    ))
                }
            }
        }
    }

    pub fn push(path: &str, token: Option<&str>) -> Result<String, String> {
        Self::push_internal(path, token, false)
    }

    pub fn push_force(path: &str, token: Option<&str>) -> Result<String, String> {
        Self::push_internal(path, token, true)
    }

    fn push_internal(path: &str, token: Option<&str>, force: bool) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = head.shorthand().unwrap_or("main").to_string();
        let refspec = if force {
            format!("+refs/heads/{}:refs/heads/{}", branch_name, branch_name)
        } else {
            format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name)
        };
        let tag_refspecs = Self::local_tag_refspecs(&repo)?;

        let mut remote = repo
            .find_remote("origin")
            .map_err(|e| format!("No remote 'origin': {}", e.message()))?;

        let remote_url = remote.url().unwrap_or_default().to_string();
        if let Some(t) = token {
            if remote_url.starts_with(HTTPS_SCHEME) && !remote_url.contains('@') {
                let users = Self::auth_username_candidates(&remote_url, None);
                let mut last_error: Option<String> = None;

                for user in users {
                    let Some(authed_url) =
                        Self::authenticated_url_with_username(&remote_url, &user, t)
                    else {
                        continue;
                    };

                    if repo.find_remote(TEMP_PUSH_REMOTE_AUTH).is_ok() {
                        let _ = repo.remote_delete(TEMP_PUSH_REMOTE_AUTH);
                    }

                    let mut temp_remote =
                        repo.remote(TEMP_PUSH_REMOTE_AUTH, &authed_url)
                            .map_err(|e| {
                                format!(
                                    "Failed to create temporary authenticated remote: {}",
                                    e.message()
                                )
                            })?;

                    let callbacks = Self::build_remote_callbacks(Some(t), &authed_url);
                    let mut push_opts = git2::PushOptions::new();
                    push_opts.remote_callbacks(callbacks);

                    let mut push_refspecs: Vec<&str> = Vec::with_capacity(1 + tag_refspecs.len());
                    push_refspecs.push(refspec.as_str());
                    for tag_refspec in &tag_refspecs {
                        push_refspecs.push(tag_refspec.as_str());
                    }

                    match temp_remote.push(&push_refspecs, Some(&mut push_opts)) {
                        Ok(_) => {
                            let _ = repo.remote_delete(TEMP_PUSH_REMOTE_AUTH);
                            return Ok(if force {
                                "Force push complete. Tags pushed too.".to_string()
                            } else {
                                "Push complete. Tags pushed too.".to_string()
                            });
                        }
                        Err(err) => {
                            last_error = Some(err.message().to_string());
                            let _ = repo.remote_delete(TEMP_PUSH_REMOTE_AUTH);
                        }
                    }
                }

                if let Some(error) = last_error {
                    return Err(error);
                }
            }
        }

        let callbacks = Self::build_remote_callbacks(token, &remote_url);
        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        let mut push_refspecs: Vec<&str> = Vec::with_capacity(1 + tag_refspecs.len());
        push_refspecs.push(refspec.as_str());
        for tag_refspec in &tag_refspecs {
            push_refspecs.push(tag_refspec.as_str());
        }

        remote
            .push(&push_refspecs, Some(&mut push_opts))
            .map_err(|e| e.message().to_string())?;

        Ok(if force {
            "Force push complete. Tags pushed too.".to_string()
        } else {
            "Push complete. Tags pushed too.".to_string()
        })
    }

    pub fn run_git_command(path: &str, args: &[&str]) -> Result<String, String> {
        GitRepository::git_cli(path, args)
    }

    pub fn run_shell_command(path: &str, command: &str) -> Result<String, String> {
        GitRepository::run_shell_command(path, command)
    }

    pub fn rename_branch(path: &str, old_name: &str, new_name: &str) -> Result<String, String> {
        GitRepository::git_cli(path, &["branch", "-m", old_name, new_name])
    }

    pub fn delete_remote_branch(
        path: &str,
        remote: &str,
        branch: &str,
        token: Option<&str>,
    ) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let mut remote_ref = repo
            .find_remote(remote)
            .map_err(|e| format!("No remote '{}': {}", remote, e.message()))?;

        let refspec = format!(":refs/heads/{}", branch);

        let remote_url = remote_ref.url().unwrap_or_default().to_string();
        if let Some(t) = token {
            if remote_url.starts_with(HTTPS_SCHEME) && !remote_url.contains('@') {
                let users = Self::auth_username_candidates(&remote_url, None);
                let mut last_error: Option<String> = None;

                for user in users {
                    let Some(authed_url) =
                        Self::authenticated_url_with_username(&remote_url, &user, t)
                    else {
                        continue;
                    };

                    if repo.find_remote(TEMP_PUSH_REMOTE_AUTH).is_ok() {
                        let _ = repo.remote_delete(TEMP_PUSH_REMOTE_AUTH);
                    }

                    let mut temp_remote =
                        repo.remote(TEMP_PUSH_REMOTE_AUTH, &authed_url)
                            .map_err(|e| {
                                format!(
                                    "Failed to create temporary authenticated remote: {}",
                                    e.message()
                                )
                            })?;

                    let callbacks = Self::build_remote_callbacks(Some(t), &authed_url);
                    let mut push_opts = git2::PushOptions::new();
                    push_opts.remote_callbacks(callbacks);

                    match temp_remote.push(&[&refspec], Some(&mut push_opts)) {
                        Ok(_) => {
                            let _ = repo.remote_delete(TEMP_PUSH_REMOTE_AUTH);
                            return Ok(format!("Deleted remote branch {}/{}", remote, branch));
                        }
                        Err(err) => {
                            last_error = Some(err.message().to_string());
                            let _ = repo.remote_delete(TEMP_PUSH_REMOTE_AUTH);
                        }
                    }
                }

                if let Some(error) = last_error {
                    return Err(error);
                }
            }
        }

        let callbacks = Self::build_remote_callbacks(token, &remote_url);
        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        remote_ref
            .push(&[&refspec], Some(&mut push_opts))
            .map_err(|e| e.message().to_string())?;

        let tracking_ref = format!("refs/remotes/{}/{}", remote, branch);
        if let Ok(mut local_tracking) = repo.find_reference(&tracking_ref) {
            let _ = local_tracking.delete();
        }

        Ok(format!("Deleted remote branch {}/{}", remote, branch))
    }

    pub fn set_upstream(path: &str, branch: &str, remote_branch: &str) -> Result<String, String> {
        GitRepository::git_cli(
            path,
            &["branch", "--set-upstream-to", remote_branch, branch],
        )
    }

    pub fn reset_branch_to_remote(path: &str, branch: &str) -> Result<String, String> {
        let repo = GitRepository::open(path)?;

        let (object, reference) = repo
            .revparse_ext(branch)
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

        let remote_ref_name = format!("refs/remotes/origin/{}", branch);
        let remote_ref = repo.find_reference(&remote_ref_name).map_err(|e| {
            format!(
                "Cannot find remote branch origin/{}: {}",
                branch,
                e.message()
            )
        })?;
        let remote_oid = remote_ref
            .target()
            .ok_or_else(|| format!("Remote branch origin/{} has no target", branch))?;
        let remote_commit = repo
            .find_commit(remote_oid)
            .map_err(|e| e.message().to_string())?;

        repo.reset(remote_commit.as_object(), git2::ResetType::Hard, None)
            .map_err(|e| e.message().to_string())?;

        Ok(format!("Reset {} to origin/{}", branch, branch))
    }

    pub fn push_to_platform(
        path: &str,
        platform: &str,
        token: &str,
        repo_name: &str,
    ) -> Result<String, String> {
        let repo = GitRepository::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = head.shorthand().unwrap_or("main").to_string();
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        let tag_refspecs = Self::local_tag_refspecs(&repo)?;

        let (name_part, domain) = if let Some((left, right)) = repo_name.rsplit_once('@') {
            let domain_part = right.trim().to_string();
            if domain_part.is_empty() {
                return Err("Invalid repo_name format with @domain".to_string());
            }
            (left.trim().to_string(), Some(domain_part))
        } else {
            (repo_name.trim().to_string(), None)
        };

        let parsed_name_parts: Vec<&str> = name_part
            .split('/')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect();
        let repo_name_part_count = parsed_name_parts.len();

        let fallback_repo_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repository")
            .to_string();

        let owner = parsed_name_parts
            .first()
            .map(|value| (*value).to_string())
            .unwrap_or_else(|| "user".to_string());
        let actual_repo_name = if repo_name_part_count >= 2 {
            parsed_name_parts
                .last()
                .map(|value| (*value).to_string())
                .unwrap_or_else(|| fallback_repo_name.clone())
        } else {
            fallback_repo_name.clone()
        };
        let azure_project = if repo_name_part_count >= 3 {
            Some(parsed_name_parts[repo_name_part_count - 2].to_string())
        } else {
            None
        };

        let enc_token = urlencoded(token);

        let remote_url = match platform {
            PLATFORM_GITHUB => {
                if repo_name_part_count > 2 {
                    return Err(
                        "Invalid repo_name format. Use username/repo for GitHub.".to_string()
                    );
                }
                format!(
                    "{}{}:{}@{}/{}/{}.git",
                    HTTPS_SCHEME, owner, enc_token, GITHUB_HOST, owner, actual_repo_name
                )
            }
            PLATFORM_GITHUB_ENTERPRISE => {
                if repo_name_part_count > 2 {
                    return Err(
                        "Invalid repo_name format. Use username/repo@domain for GitHub Enterprise."
                            .to_string(),
                    );
                }
                let domain = domain.ok_or(
                    "GitHub Enterprise requires domain in format: username/repo@domain.com",
                )?;
                format!(
                    "{}{}:{}@{}/{}/{}.git",
                    HTTPS_SCHEME, owner, enc_token, domain, owner, actual_repo_name
                )
            }
            PLATFORM_GITLAB => {
                if repo_name_part_count > 2 {
                    return Err(
                        "Invalid repo_name format. Use username/repo for GitLab.".to_string()
                    );
                }
                format!(
                    "{}{}:{}@{}/{}/{}.git",
                    HTTPS_SCHEME, AUTH_USER_GITLAB, enc_token, GITLAB_HOST, owner, actual_repo_name
                )
            }
            PLATFORM_GITLAB_SELF_HOSTED => {
                if repo_name_part_count > 2 {
                    return Err("Invalid repo_name format. Use username/repo@domain for GitLab self-hosted.".to_string());
                }
                let domain = domain.ok_or(
                    "GitLab self-hosted requires domain in format: username/repo@domain.com",
                )?;
                format!(
                    "{}{}:{}@{}/{}/{}.git",
                    HTTPS_SCHEME, AUTH_USER_GITLAB, enc_token, domain, owner, actual_repo_name
                )
            }
            PLATFORM_BITBUCKET => {
                if repo_name_part_count > 2 {
                    return Err(
                        "Invalid repo_name format. Use workspace/repo for Bitbucket.".to_string(),
                    );
                }
                format!(
                    "{}{}:{}@{}/{}/{}.git",
                    HTTPS_SCHEME,
                    AUTH_USER_BITBUCKET,
                    enc_token,
                    BITBUCKET_HOST,
                    owner,
                    actual_repo_name
                )
            }
            PLATFORM_AZURE => {
                let project = azure_project.unwrap_or_else(|| "project".to_string());
                let domain_value = domain
                    .unwrap_or_else(|| AZURE_HOST.to_string())
                    .trim()
                    .trim_end_matches('/')
                    .to_string();
                let domain_host = domain_value
                    .strip_prefix("https://")
                    .or_else(|| domain_value.strip_prefix("http://"))
                    .unwrap_or(&domain_value)
                    .split('/')
                    .next()
                    .unwrap_or(AZURE_HOST)
                    .trim();

                if domain_host.is_empty() {
                    return Err("Azure host domain cannot be empty.".to_string());
                }

                format!(
                    "{}:{}@{}/{}/{}/_git/{}",
                    HTTPS_SCHEME, enc_token, domain_host, owner, project, actual_repo_name
                )
            }
            _ => return Err(format!("Unknown platform: {platform}")),
        };

        if repo.find_remote(TEMP_PUSH_REMOTE_PLATFORM).is_ok() {
            let _ = repo.remote_delete(TEMP_PUSH_REMOTE_PLATFORM);
        }

        let mut remote = repo
            .remote(TEMP_PUSH_REMOTE_PLATFORM, &remote_url)
            .map_err(|e| format!("Failed to create temporary remote: {}", e.message()))?;

        let callbacks = Self::build_remote_callbacks(Some(token), &remote_url);

        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        let mut push_refspecs: Vec<&str> = Vec::with_capacity(1 + tag_refspecs.len());
        push_refspecs.push(refspec.as_str());
        for tag_refspec in &tag_refspecs {
            push_refspecs.push(tag_refspec.as_str());
        }

        remote.push(&push_refspecs, Some(&mut push_opts)).map_err(|e| {
            let msg = e.message().to_string();
            if msg.contains("status code: 404") {
                return "Push failed with 404: repository not found or no access. For GitHub, create the repository first on GitHub (same username/repo) and verify token permissions (repo scope).".to_string();
            }
            msg
        })?;
        Ok(format!("Push to {platform} completed. Tags pushed too."))
    }

    pub fn check_origin(path: &str) -> Result<bool, String> {
        let repo = GitRepository::open(path)?;
        let has_origin = repo.find_remote("origin").is_ok();
        Ok(has_origin)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use git2::{IndexAddOption, Repository, Signature, StatusOptions};

    use super::RemoteService;
    use crate::services::git_service::GitService;

    struct TestWorkspace {
        path: PathBuf,
    }

    impl TestWorkspace {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after Unix epoch")
                .as_nanos();
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("gitswamp-tests")
                .join(format!("{}-{}-{}", label, std::process::id(), nonce));
            fs::create_dir_all(&path).expect("test workspace should be created");
            Self { path }
        }

        fn child(&self, name: &str) -> PathBuf {
            self.path.join(name)
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn configure_identity(repo: &Repository, name: &str, email: &str) {
        let mut config = repo.config().expect("repository config should open");
        config
            .set_str("user.name", name)
            .expect("name should be set");
        config
            .set_str("user.email", email)
            .expect("email should be set");
    }

    fn write_file(root: &Path, relative: &str, content: &str) {
        let target = root.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("parent directory should be created");
        }
        fs::write(target, content).expect("test file should be written");
    }

    fn commit_all(repo: &Repository, message: &str, signature: &Signature<'_>) -> git2::Oid {
        let mut index = repo.index().expect("index should open");
        index
            .add_all(["*"], IndexAddOption::DEFAULT, None)
            .expect("files should stage");
        index.write().expect("index should write");
        let tree_oid = index.write_tree().expect("tree should write");
        let tree = repo.find_tree(tree_oid).expect("tree should exist");

        if let Ok(parent) = repo.head().and_then(|head| head.peel_to_commit()) {
            repo.commit(
                Some("HEAD"),
                signature,
                signature,
                message,
                &tree,
                &[&parent],
            )
            .expect("commit should be created")
        } else {
            repo.commit(Some("HEAD"), signature, signature, message, &tree, &[])
                .expect("root commit should be created")
        }
    }

    fn push_head(repo: &Repository) {
        let head_ref = repo
            .head()
            .expect("HEAD should exist")
            .name()
            .expect("HEAD should have a ref name")
            .to_string();
        let refspec = format!("{}:{}", head_ref, head_ref);
        repo.find_remote("origin")
            .expect("origin should exist")
            .push(&[&refspec], None)
            .expect("HEAD should push");
    }

    #[test]
    fn amend_uses_staged_snapshot_and_preserves_unstaged_work() {
        let workspace = TestWorkspace::new("amend");
        let repo_path = workspace.child("repo");
        let repo = Repository::init(&repo_path).expect("repository should initialize");
        configure_identity(&repo, "Original Author", "original@example.com");
        let original = Signature::now("Original Author", "original@example.com")
            .expect("signature should be valid");

        write_file(&repo_path, "base.txt", "base\n");
        let original_oid = commit_all(&repo, "Initial commit", &original);
        configure_identity(&repo, "Current User", "current@example.com");

        write_file(&repo_path, "staged.txt", "included\n");
        write_file(&repo_path, "unstaged.txt", "keep me local\n");
        let mut index = repo.index().expect("index should open");
        index
            .add_path(Path::new("staged.txt"))
            .expect("staged file should be added");
        index.write().expect("index should write");
        drop(index);
        drop(repo);

        let amended_oid = GitService::amend_commit(
            repo_path.to_str().expect("path should be UTF-8"),
            "Updated subject\n\nDetailed body",
            false,
            true,
        )
        .expect("amend should succeed");
        assert_ne!(amended_oid, original_oid.to_string());

        let repo = Repository::open(&repo_path).expect("repository should reopen");
        let head = repo
            .head()
            .and_then(|value| value.peel_to_commit())
            .expect("amended HEAD should exist");
        assert_eq!(head.parent_count(), 0);
        assert_eq!(head.author().name(), Some("Original Author"));
        assert_eq!(head.committer().name(), Some("Current User"));
        assert!(head
            .message()
            .unwrap_or_default()
            .contains("Signed-off-by: Current User <current@example.com>"));
        assert!(head
            .tree()
            .expect("HEAD tree should exist")
            .get_path(Path::new("staged.txt"))
            .is_ok());
        assert!(repo
            .head()
            .and_then(|value| value.peel_to_tree())
            .expect("HEAD tree should exist")
            .get_path(Path::new("unstaged.txt"))
            .is_err());
        assert_eq!(
            fs::read_to_string(repo_path.join("unstaged.txt"))
                .expect("unstaged file should remain"),
            "keep me local\n"
        );

        let amended_sha = head.id().to_string();
        drop(head);
        drop(repo);

        write_file(&repo_path, "message-only.txt", "keep staged\n");
        let repo = Repository::open(&repo_path).expect("repository should reopen");
        let mut index = repo.index().expect("index should open");
        index
            .add_path(Path::new("message-only.txt"))
            .expect("message-only file should stage");
        index.write().expect("index should write");
        drop(index);
        drop(repo);

        GitService::edit_commit_message(
            repo_path.to_str().expect("path should be UTF-8"),
            &amended_sha,
            "Message-only edit",
        )
        .expect("message-only edit should succeed");

        let repo = Repository::open(&repo_path).expect("repository should reopen");
        let head = repo
            .head()
            .and_then(|value| value.peel_to_commit())
            .expect("message-only HEAD should exist");
        assert_eq!(head.message(), Some("Message-only edit"));
        assert!(head
            .tree()
            .expect("HEAD tree should exist")
            .get_path(Path::new("message-only.txt"))
            .is_err());
        let mut options = StatusOptions::new();
        options.include_untracked(true);
        let staged_file = repo
            .statuses(Some(&mut options))
            .expect("statuses should load")
            .iter()
            .find(|entry| entry.path() == Some("message-only.txt"))
            .map(|entry| entry.status())
            .expect("message-only file should remain staged");
        assert!(staged_file.contains(git2::Status::INDEX_NEW));
    }

    #[test]
    fn protected_pull_restores_staged_unstaged_and_untracked_changes() {
        let workspace = TestWorkspace::new("pull");
        let remote_path = workspace.child("remote.git");
        let seed_path = workspace.child("seed");
        let local_path = workspace.child("local");
        let peer_path = workspace.child("peer");
        let remote = Repository::init_bare(&remote_path).expect("bare remote should initialize");
        let seed = Repository::init(&seed_path).expect("seed should initialize");
        configure_identity(&seed, "Test User", "test@example.com");
        let signature =
            Signature::now("Test User", "test@example.com").expect("signature should be valid");

        write_file(&seed_path, "base.txt", "base\n");
        write_file(&seed_path, "partial.txt", "original\n");
        commit_all(&seed, "Initial commit", &signature);
        seed.remote(
            "origin",
            remote_path.to_str().expect("remote path should be UTF-8"),
        )
        .expect("origin should be created");
        let branch_ref = seed
            .head()
            .expect("seed HEAD should exist")
            .name()
            .expect("seed HEAD should be named")
            .to_string();
        remote
            .set_head(&branch_ref)
            .expect("bare HEAD should target seed branch");
        push_head(&seed);
        drop(remote);
        drop(seed);

        let local = Repository::clone(
            remote_path.to_str().expect("remote path should be UTF-8"),
            &local_path,
        )
        .expect("local clone should succeed");
        configure_identity(&local, "Local User", "local@example.com");
        let initial_local_head = local
            .head()
            .expect("local HEAD should exist")
            .target()
            .expect("local HEAD should have an oid");

        let peer = Repository::clone(
            remote_path.to_str().expect("remote path should be UTF-8"),
            &peer_path,
        )
        .expect("peer clone should succeed");
        configure_identity(&peer, "Peer User", "peer@example.com");
        let peer_signature =
            Signature::now("Peer User", "peer@example.com").expect("signature should be valid");
        write_file(&peer_path, "remote.txt", "from remote\n");
        let remote_head = commit_all(&peer, "Remote update", &peer_signature);
        push_head(&peer);
        drop(peer);

        write_file(&local_path, "base.txt", "local unstaged edit\n");
        write_file(&local_path, "staged.txt", "local staged edit\n");
        write_file(&local_path, "untracked.txt", "local untracked edit\n");
        write_file(&local_path, "partial.txt", "staged layer\n");
        let mut index = local.index().expect("local index should open");
        index
            .add_path(Path::new("staged.txt"))
            .expect("local file should stage");
        index
            .add_path(Path::new("partial.txt"))
            .expect("partial file should stage");
        index.write().expect("local index should write");
        drop(index);
        write_file(&local_path, "partial.txt", "unstaged layer\n");
        drop(local);

        let local_path_string = local_path.to_str().expect("local path should be UTF-8");
        let refused = RemoteService::pull(local_path_string, None, false)
            .expect_err("unprotected pull should be rejected");
        assert!(refused.starts_with("WORKTREE_DIRTY:"));
        let repo = Repository::open(&local_path).expect("local repository should reopen");
        assert_eq!(
            repo.head().expect("HEAD should exist").target(),
            Some(initial_local_head)
        );
        drop(repo);

        let result = RemoteService::pull(local_path_string, None, true)
            .expect("protected pull should succeed");
        assert!(result.contains("Local changes were restored"));

        let mut repo = Repository::open(&local_path).expect("local repository should reopen");
        assert_eq!(
            repo.head().expect("HEAD should exist").target(),
            Some(remote_head)
        );
        assert_eq!(
            fs::read_to_string(local_path.join("base.txt"))
                .expect("tracked edit should remain")
                .replace("\r\n", "\n"),
            "local unstaged edit\n"
        );
        assert_eq!(
            fs::read_to_string(local_path.join("untracked.txt"))
                .expect("untracked edit should remain")
                .replace("\r\n", "\n"),
            "local untracked edit\n"
        );
        assert_eq!(
            fs::read_to_string(local_path.join("partial.txt"))
                .expect("partial worktree edit should remain")
                .replace("\r\n", "\n"),
            "unstaged layer\n"
        );
        assert_eq!(
            fs::read_to_string(local_path.join("remote.txt"))
                .expect("remote file should arrive")
                .replace("\r\n", "\n"),
            "from remote\n"
        );

        let mut status_options = StatusOptions::new();
        status_options
            .include_untracked(true)
            .recurse_untracked_dirs(true);
        let statuses = repo
            .statuses(Some(&mut status_options))
            .expect("statuses should load");
        let status_for = |path: &str| {
            statuses
                .iter()
                .find(|entry| entry.path() == Some(path))
                .map(|entry| entry.status())
                .expect("expected status should exist")
        };
        assert!(status_for("staged.txt").is_index_new());
        assert!(status_for("base.txt").is_wt_modified());
        assert!(status_for("untracked.txt").is_wt_new());
        assert!(status_for("partial.txt").is_index_modified());
        assert!(status_for("partial.txt").is_wt_modified());
        drop(statuses);

        let index = repo.index().expect("restored index should open");
        let partial_entry = index
            .get_path(Path::new("partial.txt"), 0)
            .expect("partial file should remain staged");
        let partial_blob = repo
            .find_blob(partial_entry.id)
            .expect("staged partial blob should exist");
        assert_eq!(partial_blob.content(), b"staged layer\n");
        drop(partial_blob);
        drop(index);

        let mut stash_count = 0usize;
        repo.stash_foreach(|_, _, _| {
            stash_count += 1;
            true
        })
        .expect("stash list should load");
        assert_eq!(stash_count, 0, "successful safety stash should be removed");

        let head_object = repo
            .head()
            .and_then(|head| head.peel(git2::ObjectType::Commit))
            .expect("HEAD object should exist");
        repo.reset(&head_object, git2::ResetType::Hard, None)
            .expect("local test changes should reset");
        drop(head_object);
        drop(repo);
        for file in ["staged.txt", "untracked.txt"] {
            let _ = fs::remove_file(local_path.join(file));
        }

        let collision_peer_path = workspace.child("collision-peer");
        let collision_peer = Repository::clone(
            remote_path.to_str().expect("remote path should be UTF-8"),
            &collision_peer_path,
        )
        .expect("collision peer clone should succeed");
        configure_identity(&collision_peer, "Peer User", "peer@example.com");
        write_file(&collision_peer_path, "collision.txt", "remote content\n");
        commit_all(&collision_peer, "Add collision file", &peer_signature);
        push_head(&collision_peer);
        drop(collision_peer);

        write_file(&local_path, "collision.txt", "local untracked content\n");
        RemoteService::pull(local_path_string, None, true)
            .expect_err("safe pull should refuse to overwrite an untracked file");

        let mut repo = Repository::open(&local_path).expect("local repository should reopen");
        assert_eq!(
            repo.head().expect("HEAD should exist").target(),
            Some(remote_head),
            "failed checkout must not move the local branch"
        );
        assert_eq!(
            fs::read_to_string(local_path.join("collision.txt"))
                .expect("local collision file should remain")
                .replace("\r\n", "\n"),
            "local untracked content\n"
        );
        let mut stash_count = 0usize;
        repo.stash_foreach(|_, _, _| {
            stash_count += 1;
            true
        })
        .expect("stash list should load");
        assert_eq!(
            stash_count, 0,
            "untracked-only pull should not create a stash"
        );
    }
}

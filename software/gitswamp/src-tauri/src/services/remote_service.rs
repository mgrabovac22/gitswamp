use std::path::Path;

use crate::constants::{
    AUTH_USER_BITBUCKET, AUTH_USER_GITHUB, AUTH_USER_GITLAB, AZURE_HOST, AZURE_LEGACY_HOST,
    BITBUCKET_HOST, GITHUB_HOST, GITLAB_HOST, HTTPS_SCHEME, PLATFORM_AZURE, PLATFORM_BITBUCKET,
    PLATFORM_GITHUB, PLATFORM_GITHUB_ENTERPRISE, PLATFORM_GITLAB, PLATFORM_GITLAB_SELF_HOSTED,
    TEMP_PUSH_REMOTE_AUTH, TEMP_PUSH_REMOTE_PLATFORM,
};
use crate::repositories::git_repository::GitRepository;
use crate::services::helpers::urlencoded;

pub struct RemoteService;

impl RemoteService {
    pub fn get_git_path() -> String {
        GitRepository::get_git_path()
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
        let host_and_path = remote_url
            .strip_prefix(HTTPS_SCHEME)
            .unwrap_or(remote_url);
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

    fn authenticated_url_with_username(remote_url: &str, username: &str, token: &str) -> Option<String> {
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
            let username_candidates = Self::auth_username_candidates(current_url, username_from_url);
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

            remote
                .fetch::<&str>(&[], Some(&mut fetch_opts), None)
                .map_err(|e| format!("Fetch '{}' failed: {}", remote_name, e.message()))?;
        }
        Ok(format!("Fetched {} remote(s).", remote_names.len()))
    }

    pub fn pull(path: &str, token: Option<&str>) -> Result<String, String> {
        Self::fetch_all(path, token)?;
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
            return Ok("Already up to date.".to_string());
        }
        if analysis.is_fast_forward() {
            let mut reference = repo
                .find_reference(&format!("refs/heads/{}", branch_name))
                .map_err(|e| e.message().to_string())?;
            reference
                .set_target(remote_oid, "pull: fast-forward")
                .map_err(|e| e.message().to_string())?;
            repo.set_head(&format!("refs/heads/{}", branch_name))
                .map_err(|e| e.message().to_string())?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| e.message().to_string())?;
            return Ok("Fast-forward merge complete.".to_string());
        }
        if analysis.is_normal() {
            repo.merge(&[&remote_commit], None, None)
                .map_err(|e| e.message().to_string())?;
            let index = repo.index().map_err(|e| e.message().to_string())?;
            if index.has_conflicts() {
                return Err("Merge conflicts detected. Resolve them manually.".to_string());
            }
            let mut index = repo.index().map_err(|e| e.message().to_string())?;
            let tree_oid = index.write_tree().map_err(|e| e.message().to_string())?;
            let tree = repo.find_tree(tree_oid).map_err(|e| e.message().to_string())?;
            let head_commit = repo
                .head()
                .map_err(|e| e.message().to_string())?
                .peel_to_commit()
                .map_err(|e| e.message().to_string())?;
            let remote_commit_obj = repo
                .find_commit(remote_oid)
                .map_err(|e| e.message().to_string())?;
            let sig = repo.signature().map_err(|e| e.message().to_string())?;
            let msg = format!("Merge branch '{}' of origin into {}", branch_name, branch_name);
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
            return Ok("Merge complete.".to_string());
        }
        Err("Pull failed: unexpected merge state.".to_string())
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

        let mut remote = repo
            .find_remote("origin")
            .map_err(|e| format!("No remote 'origin': {}", e.message()))?;

        let remote_url = remote.url().unwrap_or_default().to_string();
        if let Some(t) = token {
            if remote_url.starts_with(HTTPS_SCHEME) && !remote_url.contains('@') {
                let users = Self::auth_username_candidates(&remote_url, None);
                let mut last_error: Option<String> = None;

                for user in users {
                    let Some(authed_url) = Self::authenticated_url_with_username(&remote_url, &user, t)
                    else {
                        continue;
                    };

                    if repo.find_remote(TEMP_PUSH_REMOTE_AUTH).is_ok() {
                        let _ = repo.remote_delete(TEMP_PUSH_REMOTE_AUTH);
                    }

                    let mut temp_remote = repo
                        .remote(TEMP_PUSH_REMOTE_AUTH, &authed_url)
                        .map_err(|e| {
                            format!("Failed to create temporary authenticated remote: {}", e.message())
                        })?;

                    let callbacks = Self::build_remote_callbacks(Some(t), &authed_url);
                    let mut push_opts = git2::PushOptions::new();
                    push_opts.remote_callbacks(callbacks);

                    match temp_remote.push(&[&refspec], Some(&mut push_opts)) {
                        Ok(_) => {
                            let _ = repo.remote_delete(TEMP_PUSH_REMOTE_AUTH);
                            return Ok(if force {
                                "Force push complete.".to_string()
                            } else {
                                "Push complete.".to_string()
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

        remote
            .push(&[&refspec], Some(&mut push_opts))
            .map_err(|e| e.message().to_string())?;

        Ok(if force {
            "Force push complete.".to_string()
        } else {
            "Push complete.".to_string()
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
                    let Some(authed_url) = Self::authenticated_url_with_username(&remote_url, &user, t)
                    else {
                        continue;
                    };

                    if repo.find_remote(TEMP_PUSH_REMOTE_AUTH).is_ok() {
                        let _ = repo.remote_delete(TEMP_PUSH_REMOTE_AUTH);
                    }

                    let mut temp_remote = repo
                        .remote(TEMP_PUSH_REMOTE_AUTH, &authed_url)
                        .map_err(|e| {
                            format!("Failed to create temporary authenticated remote: {}", e.message())
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
        GitRepository::git_cli(path, &["branch", "--set-upstream-to", remote_branch, branch])
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
        let remote_ref = repo
            .find_reference(&remote_ref_name)
            .map_err(|e| format!("Cannot find remote branch origin/{}: {}", branch, e.message()))?;
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
                    return Err("Invalid repo_name format. Use username/repo for GitHub.".to_string());
                }
                format!(
                    "{}{}:{}@{}/{}/{}.git",
                    HTTPS_SCHEME,
                    owner,
                    enc_token,
                    GITHUB_HOST,
                    owner,
                    actual_repo_name
                )
            }
            PLATFORM_GITHUB_ENTERPRISE => {
                if repo_name_part_count > 2 {
                    return Err("Invalid repo_name format. Use username/repo@domain for GitHub Enterprise.".to_string());
                }
                let domain =
                    domain.ok_or("GitHub Enterprise requires domain in format: username/repo@domain.com")?;
                format!(
                    "{}{}:{}@{}/{}/{}.git",
                    HTTPS_SCHEME,
                    owner, enc_token, domain, owner, actual_repo_name
                )
            }
            PLATFORM_GITLAB => {
                if repo_name_part_count > 2 {
                    return Err("Invalid repo_name format. Use username/repo for GitLab.".to_string());
                }
                format!(
                    "{}{}:{}@{}/{}/{}.git",
                    HTTPS_SCHEME,
                    AUTH_USER_GITLAB,
                    enc_token,
                    GITLAB_HOST,
                    owner,
                    actual_repo_name
                )
            }
            PLATFORM_GITLAB_SELF_HOSTED => {
                if repo_name_part_count > 2 {
                    return Err("Invalid repo_name format. Use username/repo@domain for GitLab self-hosted.".to_string());
                }
                let domain =
                    domain.ok_or("GitLab self-hosted requires domain in format: username/repo@domain.com")?;
                format!(
                    "{}{}:{}@{}/{}/{}.git",
                    HTTPS_SCHEME,
                    AUTH_USER_GITLAB,
                    enc_token, domain, owner, actual_repo_name
                )
            }
            PLATFORM_BITBUCKET => {
                if repo_name_part_count > 2 {
                    return Err("Invalid repo_name format. Use workspace/repo for Bitbucket.".to_string());
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
                    HTTPS_SCHEME,
                    enc_token,
                    domain_host,
                    owner,
                    project,
                    actual_repo_name
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

        remote.push(&[&refspec], Some(&mut push_opts)).map_err(|e| {
            let msg = e.message().to_string();
            if msg.contains("status code: 404") {
                return "Push failed with 404: repository not found or no access. For GitHub, create the repository first on GitHub (same username/repo) and verify token permissions (repo scope).".to_string();
            }
            msg
        })?;
        Ok(format!("Push to {platform} completed."))
    }

    pub fn check_origin(path: &str) -> Result<bool, String> {
        let repo = GitRepository::open(path)?;
        let has_origin = repo.find_remote("origin").is_ok();
        Ok(has_origin)
    }
}

use std::path::Path;

use crate::repositories::git_repository::GitRepository;
use crate::services::helpers::urlencoded;

pub struct RemoteService;

impl RemoteService {
    pub fn get_git_path() -> String {
        GitRepository::get_git_path()
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

            let mut callbacks = git2::RemoteCallbacks::new();
            if let Some(t) = token {
                let tok = t.to_string();
                callbacks.credentials(move |_url, _username, _allowed| {
                    git2::Cred::userpass_plaintext("x-access-token", &tok)
                });
            }
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
            if remote_url.starts_with("https://") && !remote_url.contains('@') {
                let host_and_path = &remote_url[8..];
                let enc_token = urlencoded(t);

                let authed_url = if host_and_path.contains("gitlab.") || host_and_path.contains("/gitlab") {
                    format!("https://oauth2:{}@{}", enc_token, host_and_path)
                } else if host_and_path.contains("bitbucket.org") {
                    format!("https://x-token-auth:{}@{}", enc_token, host_and_path)
                } else if host_and_path.contains("dev.azure.com")
                    || host_and_path.contains("visualstudio.com")
                {
                    format!("https://:{}@{}", enc_token, host_and_path)
                } else {
                    format!("https://x-access-token:{}@{}", enc_token, host_and_path)
                };

                if repo.find_remote("temp_push_origin_auth").is_ok() {
                    let _ = repo.remote_delete("temp_push_origin_auth");
                }

                let mut temp_remote = repo
                    .remote("temp_push_origin_auth", &authed_url)
                    .map_err(|e| {
                        format!("Failed to create temporary authenticated remote: {}", e.message())
                    })?;

                let callbacks = git2::RemoteCallbacks::new();
                let mut push_opts = git2::PushOptions::new();
                push_opts.remote_callbacks(callbacks);

                let result = temp_remote.push(&[&refspec], Some(&mut push_opts));
                let _ = repo.remote_delete("temp_push_origin_auth");

                result.map_err(|e| e.message().to_string())?;
                return Ok(if force {
                    "Force push complete.".to_string()
                } else {
                    "Push complete.".to_string()
                });
            }
        }

        let callbacks = git2::RemoteCallbacks::new();
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
            if remote_url.starts_with("https://") && !remote_url.contains('@') {
                let host_and_path = &remote_url[8..];
                let enc_token = urlencoded(t);

                let authed_url = if host_and_path.contains("gitlab.") || host_and_path.contains("/gitlab") {
                    format!("https://oauth2:{}@{}", enc_token, host_and_path)
                } else if host_and_path.contains("bitbucket.org") {
                    format!("https://x-token-auth:{}@{}", enc_token, host_and_path)
                } else if host_and_path.contains("dev.azure.com")
                    || host_and_path.contains("visualstudio.com")
                {
                    format!("https://:{}@{}", enc_token, host_and_path)
                } else {
                    format!("https://x-access-token:{}@{}", enc_token, host_and_path)
                };

                if repo.find_remote("temp_push_origin_auth").is_ok() {
                    let _ = repo.remote_delete("temp_push_origin_auth");
                }

                let mut temp_remote = repo
                    .remote("temp_push_origin_auth", &authed_url)
                    .map_err(|e| {
                        format!("Failed to create temporary authenticated remote: {}", e.message())
                    })?;

                let callbacks = git2::RemoteCallbacks::new();
                let mut push_opts = git2::PushOptions::new();
                push_opts.remote_callbacks(callbacks);

                let result = temp_remote.push(&[&refspec], Some(&mut push_opts));
                let _ = repo.remote_delete("temp_push_origin_auth");
                result.map_err(|e| e.message().to_string())?;

                return Ok(format!("Deleted remote branch {}/{}", remote, branch));
            }
        }

        let callbacks = git2::RemoteCallbacks::new();
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

        let (owner, actual_repo_name, domain) = if repo_name.contains('@') {
            let parts: Vec<&str> = repo_name.split('@').collect();
            if parts.len() == 2 {
                let name_parts: Vec<&str> = parts[0].split('/').collect();
                if name_parts.len() == 2 {
                    (
                        name_parts[0].to_string(),
                        name_parts[1].to_string(),
                        Some(parts[1].to_string()),
                    )
                } else {
                    return Err(
                        "Invalid repo_name format. Use username/repo or username/repo@domain.com"
                            .to_string(),
                    );
                }
            } else {
                return Err("Invalid repo_name format with @".to_string());
            }
        } else if repo_name.contains('/') {
            let parts: Vec<&str> = repo_name.split('/').collect();
            if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string(), None)
            } else {
                return Err("Invalid repo_name format".to_string());
            }
        } else {
            let repo_dir = Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repository")
                .to_string();
            ("user".to_string(), repo_dir, None)
        };

        let enc_token = urlencoded(token);

        let remote_url = match platform {
            "github" => format!(
                "https://{}:{}@github.com/{}/{}.git",
                owner, enc_token, owner, actual_repo_name
            ),
            "github-enterprise" => {
                let domain =
                    domain.ok_or("GitHub Enterprise requires domain in format: username/repo@domain.com")?;
                format!(
                    "https://{}:{}@{}/{}/{}.git",
                    owner, enc_token, domain, owner, actual_repo_name
                )
            }
            "gitlab" => format!(
                "https://oauth2:{}@gitlab.com/{}/{}.git",
                enc_token, owner, actual_repo_name
            ),
            "gitlab-self-hosted" => {
                let domain =
                    domain.ok_or("GitLab self-hosted requires domain in format: username/repo@domain.com")?;
                format!(
                    "https://oauth2:{}@{}/{}/{}.git",
                    enc_token, domain, owner, actual_repo_name
                )
            }
            "bitbucket" => format!(
                "https://x-token-auth:{}@bitbucket.org/{}/{}.git",
                enc_token, owner, actual_repo_name
            ),
            "azure" => format!(
                "https://:{}@dev.azure.com/{}/project/_git/{}",
                enc_token, owner, actual_repo_name
            ),
            _ => return Err(format!("Unknown platform: {platform}")),
        };

        if repo.find_remote("temp_push_remote").is_ok() {
            let _ = repo.remote_delete("temp_push_remote");
        }

        let mut remote = repo
            .remote("temp_push_remote", &remote_url)
            .map_err(|e| format!("Failed to create temporary remote: {}", e.message()))?;

        let callbacks = git2::RemoteCallbacks::new();

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

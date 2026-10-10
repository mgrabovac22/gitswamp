use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::constants::{
    API_AZURE_REPOS_PATH, API_BITBUCKET_USER_PATH, API_BITBUCKET_USER_SSH_KEYS_FMT,
    API_BITBUCKET_WORKSPACES, API_BITBUCKET_WORKSPACE_REPOS_FMT, API_GITHUB_LIST_REPOS,
    API_GITHUB_SEARCH_REPOS, API_GITHUB_USER_KEYS_PATH, API_GITHUB_USER_PATH,
    API_GITLAB_BASE_PATH, API_GITLAB_USER_KEYS_PATH, API_GITLAB_USER_PATH, APP_USER_AGENT,
    AZURE_HOST, AZURE_LEGACY_HOST, GITHUB_ACCEPT_HEADER, GITHUB_OAUTH_ACCESS_TOKEN_URL,
    GITHUB_OAUTH_CLIENT_ID, GITHUB_OAUTH_DEVICE_CODE_URL, GITHUB_OAUTH_SCOPES, HTTPS_SCHEME,
    JSON_ACCEPT_HEADER,
};
use crate::models::{
    AzureRepo, BitbucketRepo, BitbucketSshKey, GithubDeviceAuth, GithubRepo, GithubSshKey, GitlabRepo,
};
use crate::services::helpers::{split_bitbucket_credentials, urlencoded};

#[cfg(windows)]
use crate::constants::CREATE_NO_WINDOW;

pub struct IntegrationService;

impl IntegrationService {
    fn map_github_repo(
        item: &serde_json::Value,
        viewer_login: &str,
        is_public_search_result: bool,
    ) -> Option<GithubRepo> {
        Some(GithubRepo {
            full_name: item["full_name"].as_str()?.to_string(),
            clone_url: item["clone_url"].as_str()?.to_string(),
            description: item["description"].as_str().unwrap_or("").to_string(),
            is_private: item["private"].as_bool().unwrap_or(false),
            stars: item["stargazers_count"].as_u64().unwrap_or(0) as u32,
            owner_login: item["owner"]["login"].as_str().unwrap_or("").to_string(),
            owner_type: item["owner"]["type"].as_str().unwrap_or("User").to_string(),
            viewer_login: viewer_login.to_string(),
            is_public_search_result,
        })
    }

    fn github_repo_matches_query(item: &serde_json::Value, query: &str) -> bool {
        let normalized_query = query.trim().to_lowercase();
        if normalized_query.is_empty() {
            return true;
        }

        let full_name = item["full_name"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase();
        let description = item["description"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase();
        let owner_login = item["owner"]["login"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase();

        full_name.contains(&normalized_query)
            || description.contains(&normalized_query)
            || owner_login.contains(&normalized_query)
    }

    fn base64_encode(data: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();
        for chunk in data.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let triple = (b0 << 16) | (b1 << 8) | b2;
            result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
            if chunk.len() > 2 {
                result.push(CHARS[(triple & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
        }
        result
    }

    fn basic_auth_header(username: &str, password: &str) -> String {
        let payload = format!("{}:{}", username, password);
        let encoded = Self::base64_encode(payload.as_bytes());
        format!("Basic {}", encoded)
    }

    fn bitbucket_auth_header(token: &str) -> String {
        let secret = match split_bitbucket_credentials(API_BITBUCKET_USER_PATH, token) {
            Some((_user, secret)) => secret,
            None => token.trim().to_string(),
        };
        format!("Bearer {}", secret.trim())
    }

    fn bitbucket_ssh_keys_url(account_id: &str) -> String {
        API_BITBUCKET_USER_SSH_KEYS_FMT.replace("{}", account_id)
    }

    fn bitbucket_account_id(auth_header: &str) -> Result<String, String> {
        let resp = ureq::get(API_BITBUCKET_USER_PATH)
            .set("Authorization", auth_header)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| format!("Bitbucket account lookup failed: {}", e))?;

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        body["username"]
            .as_str()
            .or_else(|| body["uuid"].as_str())
            .map(|value| value.to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "Bitbucket did not return an account username. The credential may lack the account scope."
                    .to_string()
            })
    }

    fn azure_api_base_url(domain: &str) -> Result<String, String> {
        let trimmed = domain.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(
                "Azure host domain is required (for example: dev.azure.com/myorg)".to_string(),
            );
        }

        let without_scheme = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
            .unwrap_or(trimmed);

        let mut split = without_scheme.splitn(2, '/');
        let host = split.next().unwrap_or_default().trim().to_lowercase();
        let raw_path = split.next().unwrap_or_default().trim_matches('/');

        if host.is_empty() {
            return Err(
                "Azure host domain is required (for example: dev.azure.com/myorg)".to_string(),
            );
        }

        if host == AZURE_HOST {
            let organization = raw_path.split('/').next().unwrap_or_default().trim();
            if organization.is_empty() {
                return Err(
                    "Azure DevOps URL must include organization, for example: dev.azure.com/myorg"
                        .to_string(),
                );
            }

            return Ok(format!("{}{}/{}", HTTPS_SCHEME, host, organization));
        }

        if host.ends_with(AZURE_LEGACY_HOST) {
            return Ok(format!("{}{}", HTTPS_SCHEME, host));
        }

        if raw_path.is_empty() {
            Ok(format!("{}{}", HTTPS_SCHEME, host))
        } else {
            Ok(format!("{}{}/{}", HTTPS_SCHEME, host, raw_path))
        }
    }

    fn extract_bitbucket_clone_urls(item: &serde_json::Value) -> (Option<String>, Option<String>) {
        let mut https_url: Option<String> = None;
        let mut ssh_url: Option<String> = None;

        if let Some(clones) = item["links"]["clone"].as_array() {
            for clone in clones {
                let clone_name = clone["name"].as_str().unwrap_or_default().to_lowercase();
                let href = clone["href"].as_str().unwrap_or_default().to_string();
                if href.trim().is_empty() {
                    continue;
                }

                if clone_name == "https" && https_url.is_none() {
                    https_url = Some(href);
                } else if clone_name == "ssh" && ssh_url.is_none() {
                    ssh_url = Some(href);
                }
            }
        }

        (https_url, ssh_url)
    }

    pub fn search_github_repos(
        token: &str,
        query: &str,
        include_public: bool,
    ) -> Result<Vec<GithubRepo>, String> {
        let viewer_resp = ureq::get(API_GITHUB_USER_PATH)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", GITHUB_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| format!("GitHub API error: {}", e))?;
        let viewer_body: serde_json::Value = viewer_resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;
        let viewer_login = viewer_body["login"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let accessible_resp = ureq::get(API_GITHUB_LIST_REPOS)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", GITHUB_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| format!("GitHub API error: {}", e))?;
        let accessible_body: serde_json::Value = accessible_resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;
        let accessible_items = accessible_body.as_array().cloned().unwrap_or_default();

        let mut repos_by_name: HashMap<String, GithubRepo> = HashMap::new();

        for item in accessible_items
            .iter()
            .filter(|item| Self::github_repo_matches_query(item, query))
        {
            if let Some(repo) = Self::map_github_repo(item, &viewer_login, false) {
                repos_by_name.insert(repo.full_name.clone(), repo);
            }
        }

        if include_public && !query.trim().is_empty() {
            let public_url = API_GITHUB_SEARCH_REPOS.replace("{}", &urlencoded(query.trim()));
            let public_resp = ureq::get(&public_url)
                .set("Authorization", &format!("Bearer {}", token))
                .set("Accept", GITHUB_ACCEPT_HEADER)
                .set("User-Agent", APP_USER_AGENT)
                .call()
                .map_err(|e| format!("GitHub API error: {}", e))?;
            let public_body: serde_json::Value = public_resp
                .into_json()
                .map_err(|e| format!("JSON parse error: {}", e))?;
            let public_items = public_body["items"].as_array().cloned().unwrap_or_default();

            for item in public_items
                .iter()
                .filter(|item| Self::github_repo_matches_query(item, query))
            {
                if let Some(repo) = Self::map_github_repo(item, &viewer_login, true) {
                    repos_by_name.entry(repo.full_name.clone()).or_insert(repo);
                }
            }
        }

        let mut repos: Vec<GithubRepo> = repos_by_name.into_values().collect();
        repos.sort_by(|left, right| {
            left.is_public_search_result
                .cmp(&right.is_public_search_result)
                .then_with(|| left.full_name.cmp(&right.full_name))
        });
        Ok(repos)
    }

    pub fn search_gitlab_repos(
        domain: &str,
        token: &str,
        query: &str,
    ) -> Result<Vec<GitlabRepo>, String> {
        let base_url = format!("{}{}{}", HTTPS_SCHEME, domain, API_GITLAB_BASE_PATH);
        let url = if query.is_empty() {
            format!(
                "{}/projects?membership=true&per_page=50&order_by=last_activity_at",
                base_url
            )
        } else {
            format!(
                "{}/projects?search={}&per_page=30&order_by=last_activity_at",
                base_url,
                urlencoded(query)
            )
        };

        let resp = ureq::get(&url)
            .set("PRIVATE-TOKEN", token)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| format!("GitLab API error: {}", e))?;

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let items = body.as_array().cloned().unwrap_or_default();
        let repos = items
            .iter()
            .filter_map(|item| {
                Some(GitlabRepo {
                    full_name: item["name_with_namespace"].as_str()?.to_string(),
                    path_with_namespace: item["path_with_namespace"].as_str()?.to_string(),
                    clone_url_ssh: item["ssh_url_to_repo"].as_str()?.to_string(),
                    clone_url_https: item["http_url_to_repo"].as_str()?.to_string(),
                    description: item["description"].as_str().unwrap_or("").to_string(),
                    is_private: item["visibility"].as_str() != Some("public"),
                    stars: item["star_count"].as_u64().unwrap_or(0) as u32,
                })
            })
            .collect();
        Ok(repos)
    }

    fn parse_api_error_message(body: &str) -> String {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(text) = value["error"]["message"].as_str() {
                return text.to_string();
            }
            if let Some(text) = value["message"].as_str() {
                return text.to_string();
            }
        }
        let trimmed = body.trim();
        if trimmed.is_empty() {
            "no response body".to_string()
        } else {
            trimmed.chars().take(300).collect()
        }
    }

    fn bitbucket_token_hint(status: u16) -> &'static str {
        match status {
            401 => " The API token was rejected. Enter your Bitbucket username and an API token (Basic auth) for Git, or a bare API token (sent as a Bearer token) for API calls. The token needs the 'read:repository:bitbucket' and 'read:user:bitbucket' scopes.",
            403 => " The API token is valid but lacks the required scopes/permissions. Add the 'read:repository:bitbucket', 'read:workspace:bitbucket' and 'read:user:bitbucket' (plus 'account' for SSH keys) scopes to your token.",
            _ => "",
        }
    }

    fn parse_bitbucket_repos(body: &serde_json::Value) -> Vec<BitbucketRepo> {
        let items = body["values"].as_array().cloned().unwrap_or_default();
        items
            .iter()
            .filter_map(|item| {
                let (clone_url_https, clone_url_ssh) = Self::extract_bitbucket_clone_urls(item);
                Some(BitbucketRepo {
                    full_name: item["full_name"].as_str()?.to_string(),
                    clone_url_ssh: clone_url_ssh.unwrap_or_default(),
                    clone_url_https: clone_url_https?,
                    description: item["description"].as_str().unwrap_or("").to_string(),
                    is_private: item["is_private"].as_bool().unwrap_or(true),
                    stars: 0,
                })
            })
            .collect()
    }

    fn bitbucket_user_workspaces(auth: &str) -> Result<Vec<String>, String> {
        let resp = match ureq::get(API_BITBUCKET_WORKSPACES)
            .set("Authorization", auth)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
        {
            Ok(resp) => resp,
            Err(ureq::Error::Status(status, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                return Err(format!(
                    "Bitbucket workspace lookup failed (HTTP {}): {}.{}",
                    status,
                    Self::parse_api_error_message(&body),
                    Self::bitbucket_token_hint(status)
                ));
            }
            Err(e) => return Err(format!("Bitbucket workspace lookup failed: {}", e)),
        };

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;
        let items = body["values"].as_array().cloned().unwrap_or_default();
        Ok(items
            .iter()
            .filter_map(|ws| {
                ws["workspace"]["slug"]
                    .as_str()
                    .or_else(|| ws["slug"].as_str())
                    .or_else(|| ws["workspace"]["uuid"].as_str())
                    .or_else(|| ws["uuid"].as_str())
                    .map(|slug| slug.to_string())
            })
            .collect())
    }

    fn bitbucket_workspace_repos(
        auth: &str,
        workspace: &str,
        query: &str,
    ) -> Result<Vec<BitbucketRepo>, String> {
        let mut url = API_BITBUCKET_WORKSPACE_REPOS_FMT.replace("{}", &urlencoded(workspace));
        if !query.trim().is_empty() {
            url.push_str("&q=");
            url.push_str(&urlencoded(&format!("name~\"{}\"", query.trim())));
        }

        let resp = match ureq::get(&url)
            .set("Authorization", auth)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
        {
            Ok(resp) => resp,
            Err(ureq::Error::Status(status, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                return Err(format!(
                    "Bitbucket repository lookup failed (HTTP {}): {}.{}",
                    status,
                    Self::parse_api_error_message(&body),
                    Self::bitbucket_token_hint(status)
                ));
            }
            Err(e) => {
                return Err(format!("Bitbucket repository lookup failed: {}", e));
            }
        };

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;
        Ok(Self::parse_bitbucket_repos(&body))
    }

    fn bitbucket_personal_workspace(auth: &str) -> Option<String> {
        let resp = ureq::get(API_BITBUCKET_USER_PATH)
            .set("Authorization", auth)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .ok()?;
        let body: serde_json::Value = resp.into_json().ok()?;
        body["nickname"]
            .as_str()
            .or_else(|| body["username"].as_str())
            .map(|slug| slug.to_string())
    }

    pub fn search_bitbucket_repos(token: &str, query: &str) -> Result<Vec<BitbucketRepo>, String> {
        let auth = Self::bitbucket_auth_header(token);
        let workspaces_result = Self::bitbucket_user_workspaces(&auth);

        let mut workspaces = workspaces_result.clone().unwrap_or_default();
        if workspaces.is_empty() {
            if let Some(personal) = Self::bitbucket_personal_workspace(&auth) {
                workspaces.push(personal);
            }
        }

        let mut repos = Vec::new();
        let mut last_error: Option<String> = None;
        for workspace in &workspaces {
            match Self::bitbucket_workspace_repos(&auth, workspace, query) {
                Ok(mut found) => repos.append(&mut found),
                Err(e) => last_error = Some(e),
            }
        }

        if repos.is_empty() {
            if let Some(e) = last_error {
                return Err(e);
            }
            if let Err(e) = workspaces_result {
                return Err(e);
            }
        }

        Ok(repos)
    }

    pub fn verify_bitbucket_token(token: &str) -> Result<String, String> {
        let token_trimmed = token.trim();
        if token_trimmed.is_empty() {
            return Err("Bitbucket token is required.".to_string());
        }

        let resp = match ureq::get(API_BITBUCKET_USER_PATH)
            .set("Authorization", &Self::bitbucket_auth_header(token_trimmed))
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
        {
            Ok(resp) => resp,
            Err(ureq::Error::Status(status, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                return Err(format!(
                    "Bitbucket token check failed (HTTP {}): {}.{}",
                    status,
                    Self::parse_api_error_message(&body),
                    Self::bitbucket_token_hint(status)
                ));
            }
            Err(e) => return Err(format!("Bitbucket token check failed: {}", e)),
        };

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let account = body["display_name"]
            .as_str()
            .or_else(|| body["nickname"].as_str())
            .or_else(|| body["username"].as_str())
            .unwrap_or("unknown");
        Ok(account.to_string())
    }

    pub fn search_azure_repos(
        domain: &str,
        token: &str,
        query: &str,
    ) -> Result<Vec<AzureRepo>, String> {
        let base_url = Self::azure_api_base_url(domain)?;
        let url = format!("{}{}", base_url, API_AZURE_REPOS_PATH);
        let query_lower = query.trim().to_lowercase();

        let resp = ureq::get(&url)
            .set("Authorization", &Self::basic_auth_header("pat", token))
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| format!("Azure DevOps API error: {}", e))?;

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let items = body["value"].as_array().cloned().unwrap_or_default();
        let repos = items
            .iter()
            .filter_map(|item| {
                let repo_name = item["name"].as_str()?.to_string();
                let project_name = item["project"]["name"].as_str().unwrap_or("").to_string();
                let full_name = if project_name.is_empty() {
                    repo_name.clone()
                } else {
                    format!("{}/{}", project_name, repo_name)
                };

                let desc = item["description"].as_str().unwrap_or("").to_string();
                if !query_lower.is_empty() {
                    let haystack = format!("{} {} {}", full_name, repo_name, desc).to_lowercase();
                    if !haystack.contains(&query_lower) {
                        return None;
                    }
                }

                let visibility = item["project"]["visibility"]
                    .as_str()
                    .unwrap_or("private")
                    .to_lowercase();

                Some(AzureRepo {
                    full_name,
                    clone_url_ssh: item["sshUrl"].as_str().unwrap_or("").to_string(),
                    clone_url_https: item["remoteUrl"].as_str()?.to_string(),
                    description: desc,
                    is_private: visibility != "public",
                    stars: 0,
                })
            })
            .collect();

        Ok(repos)
    }

    pub fn generate_ssh_key(email: &str, key_name: &str) -> Result<(String, String), String> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "Cannot find home directory")?;

        let ssh_dir = Path::new(&home).join(".ssh");
        std::fs::create_dir_all(&ssh_dir)
            .map_err(|e| format!("Failed to create .ssh dir: {}", e))?;

        let key_path = ssh_dir.join(key_name);
        let pub_key_path = ssh_dir.join(format!("{}.pub", key_name));
        let ssh_keygen_candidates = Self::ssh_keygen_candidates();

        if key_path.exists() {
            let pub_key = if pub_key_path.exists() {
                std::fs::read_to_string(&pub_key_path)
                    .map_err(|e| format!("Failed to read existing public key: {}", e))?
            } else {
                let derived = Self::derive_public_key(&ssh_keygen_candidates, &key_path)?;
                std::fs::write(&pub_key_path, &derived)
                    .map_err(|e| format!("Failed to write derived public key: {}", e))?;
                derived
            };
            return Ok((key_path.to_string_lossy().to_string(), pub_key));
        }

        let mut failures: Vec<String> = Vec::new();
        for ssh_keygen in &ssh_keygen_candidates {
            let mut cmd = std::process::Command::new(ssh_keygen);
            cmd.args(["-t", "ed25519", "-C", email, "-f"])
                .arg(&key_path)
                .arg("-N")
                .arg("");

            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);

            match cmd.output() {
                Ok(output) if output.status.success() => {
                    let pub_key = std::fs::read_to_string(&pub_key_path)
                        .map_err(|e| format!("Failed to read public key: {}", e))?;

                    return Ok((key_path.to_string_lossy().to_string(), pub_key));
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let detail = if !stderr.is_empty() { stderr } else { stdout };
                    failures.push(format!(
                        "{} -> {}",
                        ssh_keygen.display(),
                        if detail.is_empty() {
                            "no additional error output".to_string()
                        } else {
                            detail
                        }
                    ));
                }
                Err(error) => {
                    failures.push(format!("{} -> {}", ssh_keygen.display(), error));
                }
            }
        }

        if failures.is_empty() {
            return Err("ssh-keygen failed: no executable candidates were found".to_string());
        }

        Err(format!(
            "ssh-keygen failed on all candidates: {}",
            failures.join(" | ")
        ))
    }

    pub fn add_gitlab_ssh_key(
        domain: &str,
        token: &str,
        title: &str,
        key: &str,
    ) -> Result<(), String> {
        let url = format!("{}{}{}", HTTPS_SCHEME, domain, API_GITLAB_USER_KEYS_PATH);

        let body = serde_json::json!({
            "title": title,
            "key": key
        });

        let result = ureq::post(&url)
            .set("PRIVATE-TOKEN", token)
            .set("Content-Type", "application/json")
            .set("User-Agent", APP_USER_AGENT)
            .send_json(&body);

        match result {
            Ok(resp) => {
                if resp.status() == 201 || resp.status() == 200 {
                    Ok(())
                } else {
                    Err(format!("GitLab returned status {}", resp.status()))
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                if let Ok(body) = resp.into_string() {
                    let body_lower = body.to_lowercase();
                    if body_lower.contains("has already been taken")
                        || body_lower.contains("already exists")
                        || body_lower.contains("fingerprint has already been taken")
                        || body_lower.contains("key has already been taken")
                    {
                        return Ok(());
                    }
                    Err(format!("GitLab error ({}): {}", code, body))
                } else {
                    Err(format!("GitLab returned status {}", code))
                }
            }
            Err(e) => Err(format!("Failed to add SSH key: {}", e)),
        }
    }

    pub fn add_github_ssh_key(token: &str, title: &str, key: &str) -> Result<(), String> {
        let token_trimmed = token.trim();
        if token_trimmed.is_empty() {
            return Err("GitHub token is required to add an SSH key.".to_string());
        }

        let normalized_key = Self::normalize_ssh_public_key(key)?;
        let key_title = if title.trim().is_empty() {
            "gitswamp"
        } else {
            title.trim()
        };

        let body = serde_json::json!({
            "title": key_title,
            "key": normalized_key,
        });

        let result = ureq::post(API_GITHUB_USER_KEYS_PATH)
            .set("Authorization", &format!("Bearer {}", token_trimmed))
            .set("Accept", GITHUB_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .set("Content-Type", "application/json")
            .send_json(&body);

        match result {
            Ok(resp) => {
                if resp.status() == 201 || resp.status() == 200 {
                    Ok(())
                } else {
                    Err(format!("GitHub returned status {}", resp.status()))
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                if let Ok(body) = resp.into_string() {
                    let body_lower = body.to_lowercase();
                    if code == 404 && body_lower.contains("not found") {
                        return Err("GitHub rejected SSH key creation. The token/oauth scope must include admin:public_key.".to_string());
                    }
                    if body_lower.contains("already in use")
                        || body_lower.contains("key is already")
                        || body_lower.contains("already exists")
                    {
                        return Ok(());
                    }
                    Err(format!("GitHub error ({}): {}", code, body))
                } else {
                    Err(format!("GitHub returned status {}", code))
                }
            }
            Err(e) => Err(format!("Failed to add GitHub SSH key: {}", e)),
        }
    }

    pub fn list_github_ssh_keys(token: &str) -> Result<Vec<GithubSshKey>, String> {
        let token_trimmed = token.trim();
        if token_trimmed.is_empty() {
            return Err("GitHub token is required to list SSH keys.".to_string());
        }

        let resp = ureq::get(API_GITHUB_USER_KEYS_PATH)
            .set("Authorization", &format!("Bearer {}", token_trimmed))
            .set("Accept", GITHUB_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| format!("GitHub API error: {}", e))?;

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let items = body.as_array().cloned().unwrap_or_default();
        let keys = items
            .iter()
            .filter_map(|item| {
                Some(GithubSshKey {
                    id: item["id"].as_u64()?,
                    title: item["title"].as_str().unwrap_or("Untitled").to_string(),
                    key: item["key"].as_str().unwrap_or("").to_string(),
                    fingerprint: item["fingerprint"].as_str().unwrap_or("").to_string(),
                    created_at: item["created_at"].as_str().unwrap_or("").to_string(),
                })
            })
            .collect();

        Ok(keys)
    }

    pub fn delete_github_ssh_key(token: &str, key_id: u64) -> Result<(), String> {
        let token_trimmed = token.trim();
        if token_trimmed.is_empty() {
            return Err("GitHub token is required to delete an SSH key.".to_string());
        }

        if key_id == 0 {
            return Err("A valid GitHub SSH key id is required.".to_string());
        }

        let url = format!("{}/{}", API_GITHUB_USER_KEYS_PATH, key_id);
        let result = ureq::delete(&url)
            .set("Authorization", &format!("Bearer {}", token_trimmed))
            .set("Accept", GITHUB_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call();

        match result {
            Ok(resp) => {
                if resp.status() == 204 || resp.status() == 200 || resp.status() == 202 {
                    Ok(())
                } else {
                    Err(format!("GitHub returned status {}", resp.status()))
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                if body.trim().is_empty() {
                    Err(format!("GitHub returned status {}", code))
                } else {
                    Err(format!("GitHub error ({}): {}", code, body))
                }
            }
            Err(e) => Err(format!("Failed to delete GitHub SSH key: {}", e)),
        }
    }

    pub fn add_bitbucket_ssh_key(token: &str, title: &str, key: &str) -> Result<(), String> {
        if token.trim().is_empty() {
            return Err("Bitbucket credential is required to add an SSH key.".to_string());
        }

        let auth = Self::bitbucket_auth_header(token);
        let account_id = Self::bitbucket_account_id(&auth)?;
        let url = Self::bitbucket_ssh_keys_url(&account_id);

        let normalized_key = Self::normalize_ssh_public_key(key)?;
        let label = if title.trim().is_empty() {
            "gitswamp"
        } else {
            title.trim()
        };

        let body = serde_json::json!({
            "key": normalized_key,
            "label": label,
        });

        let result = ureq::post(&url)
            .set("Authorization", &auth)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("Content-Type", "application/json")
            .set("User-Agent", APP_USER_AGENT)
            .send_json(&body);

        match result {
            Ok(resp) => {
                if resp.status() == 200 || resp.status() == 201 {
                    Ok(())
                } else {
                    Err(format!("Bitbucket returned status {}", resp.status()))
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                let body_lower = body.to_lowercase();
                if code == 403 {
                    return Err(
                        "Bitbucket rejected SSH key creation. The API token needs the 'account' scope (Account: Write) and must resolve to your account."
                            .to_string(),
                    );
                }
                if body_lower.contains("already")
                    || body_lower.contains("fingerprint")
                    || code == 409
                {
                    return Ok(());
                }
                Err(format!(
                    "Bitbucket error ({}): {}.{}",
                    code,
                    Self::parse_api_error_message(&body),
                    Self::bitbucket_token_hint(code)
                ))
            }
            Err(e) => Err(format!("Failed to add Bitbucket SSH key: {}", e)),
        }
    }

    pub fn list_bitbucket_ssh_keys(token: &str) -> Result<Vec<BitbucketSshKey>, String> {
        if token.trim().is_empty() {
            return Err("Bitbucket credential is required to list SSH keys.".to_string());
        }

        let auth = Self::bitbucket_auth_header(token);
        let account_id = Self::bitbucket_account_id(&auth)?;
        let url = Self::bitbucket_ssh_keys_url(&account_id);

        let resp = ureq::get(&url)
            .set("Authorization", &auth)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(status, _) => {
                    format!("Bitbucket SSH key list failed (HTTP {})", status)
                }
                other => format!("Bitbucket SSH key list failed: {}", other),
            })?;

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let items = body["values"].as_array().cloned().unwrap_or_default();
        let keys = items
            .iter()
            .filter_map(|item| {
                Some(BitbucketSshKey {
                    uuid: item["uuid"].as_str().unwrap_or("").to_string(),
                    label: item["label"].as_str().unwrap_or("Untitled").to_string(),
                    key: item["key"].as_str().unwrap_or("").to_string(),
                    fingerprint: item["fingerprint"].as_str().unwrap_or("").to_string(),
                    created_on: item["created_on"].as_str().unwrap_or("").to_string(),
                })
            })
            .collect();

        Ok(keys)
    }

    pub fn delete_bitbucket_ssh_key(token: &str, key_id: &str) -> Result<(), String> {
        if token.trim().is_empty() {
            return Err("Bitbucket credential is required to delete an SSH key.".to_string());
        }
        if key_id.trim().is_empty() {
            return Err("A valid Bitbucket SSH key id is required.".to_string());
        }

        let auth = Self::bitbucket_auth_header(token);
        let account_id = Self::bitbucket_account_id(&auth)?;
        let url = format!(
            "{}/{}",
            Self::bitbucket_ssh_keys_url(&account_id),
            key_id.trim()
        );

        let result = ureq::delete(&url)
            .set("Authorization", &auth)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call();

        match result {
            Ok(resp) => {
                if resp.status() == 204 || resp.status() == 200 || resp.status() == 202 {
                    Ok(())
                } else {
                    Err(format!("Bitbucket returned status {}", resp.status()))
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                if body.trim().is_empty() {
                    Err(format!("Bitbucket returned status {}", code))
                } else {
                    Err(format!("Bitbucket error ({}): {}", code, body))
                }
            }
            Err(e) => Err(format!("Failed to delete Bitbucket SSH key: {}", e)),
        }
    }

    pub fn verify_github_token(token: &str) -> Result<String, String> {
        let token_trimmed = token.trim();
        if token_trimmed.is_empty() {
            return Err("GitHub token is required.".to_string());
        }

        let resp = ureq::get(API_GITHUB_USER_PATH)
            .set("Authorization", &format!("Bearer {}", token_trimmed))
            .set("Accept", GITHUB_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| format!("GitHub API error: {}", e))?;

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(body["login"].as_str().unwrap_or("Unknown").to_string())
    }

    pub fn load_ssh_public_key_from_file(file_path: &str) -> Result<String, String> {
        let path = PathBuf::from(file_path.trim());
        if path.as_os_str().is_empty() {
            return Err("SSH key file path is required.".to_string());
        }

        if !path.exists() {
            return Err(format!("SSH key file not found: {}", path.display()));
        }

        if !path.is_file() {
            return Err(format!("Path is not a file: {}", path.display()));
        }

        let key_text = if path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("pub"))
            .unwrap_or(false)
        {
            fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read SSH public key file: {}", e))?
        } else {
            match fs::read_to_string(&path) {
                Ok(raw_text) => {
                    let raw_trimmed = raw_text.trim_start();
                    if raw_trimmed.starts_with("ssh-") || raw_trimmed.starts_with("ecdsa-") {
                        raw_text
                    } else {
                        let candidates = Self::ssh_keygen_candidates();
                        Self::derive_public_key(&candidates, &path)?
                    }
                }
                Err(_) => {
                    let candidates = Self::ssh_keygen_candidates();
                    Self::derive_public_key(&candidates, &path)?
                }
            }
        };

        Self::normalize_ssh_public_key(&key_text)
    }

    pub fn github_oauth_start() -> Result<GithubDeviceAuth, String> {
        let resp = match ureq::post(GITHUB_OAUTH_DEVICE_CODE_URL)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .send_form(&[
                ("client_id", GITHUB_OAUTH_CLIENT_ID),
                ("scope", GITHUB_OAUTH_SCOPES),
            ]) {
            Ok(resp) => resp,
            Err(ureq::Error::Status(status, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                return Err(format!(
                    "GitHub device flow could not be started (HTTP {}): {}",
                    status,
                    Self::parse_api_error_message(&body)
                ));
            }
            Err(e) => {
                return Err(format!("GitHub device flow could not be started: {}", e));
            }
        };

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let device_code = body["device_code"].as_str().unwrap_or_default().to_string();
        let user_code = body["user_code"].as_str().unwrap_or_default().to_string();
        let verification_uri = body["verification_uri"]
            .as_str()
            .unwrap_or("https://github.com/login/device")
            .to_string();

        if device_code.is_empty() || user_code.is_empty() {
            let reason = body["error"]
                .as_str()
                .map(|err| {
                    body["error_description"]
                        .as_str()
                        .map(|desc| format!("{}: {}", err, desc))
                        .unwrap_or_else(|| err.to_string())
                })
                .unwrap_or_else(|| "unexpected response".to_string());
            return Err(format!(
                "GitHub device flow could not be started: {}",
                reason
            ));
        }

        Ok(GithubDeviceAuth {
            device_code,
            user_code,
            verification_uri,
            expires_in: body["expires_in"].as_u64().unwrap_or(900),
            interval: body["interval"].as_u64().unwrap_or(5),
        })
    }

    pub fn github_oauth_poll(device_code: &str) -> Result<Option<String>, String> {
        if device_code.trim().is_empty() {
            return Err("Missing device code for GitHub OAuth polling.".to_string());
        }

        let resp = match ureq::post(GITHUB_OAUTH_ACCESS_TOKEN_URL)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .send_form(&[
                ("client_id", GITHUB_OAUTH_CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ]) {
            Ok(resp) => resp,
            Err(ureq::Error::Status(status, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                return Err(format!(
                    "GitHub OAuth polling failed (HTTP {}): {}",
                    status,
                    Self::parse_api_error_message(&body)
                ));
            }
            Err(e) => return Err(format!("GitHub OAuth polling failed: {}", e)),
        };

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        if let Some(token) = body["access_token"].as_str() {
            if !token.is_empty() {
                return Ok(Some(token.to_string()));
            }
        }

        match body["error"].as_str() {
            Some("authorization_pending") | Some("slow_down") => Ok(None),
            Some("expired_token") => Err(
                "The GitHub login code expired. Please start the connection again.".to_string(),
            ),
            Some("access_denied") => Err("Authorization was denied in the browser.".to_string()),
            Some(other) => Err(format!(
                "GitHub OAuth error: {}",
                body["error_description"].as_str().unwrap_or(other)
            )),
            None => Err("Unexpected response while polling GitHub for authorization.".to_string()),
        }
    }

    pub fn verify_gitlab_token(domain: &str, token: &str) -> Result<String, String> {
        let url = format!("{}{}{}", HTTPS_SCHEME, domain, API_GITLAB_USER_PATH);

        let resp = ureq::get(&url)
            .set("PRIVATE-TOKEN", token)
            .set("Accept", JSON_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| format!("GitLab API error: {}", e))?;

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(body["username"].as_str().unwrap_or("Unknown").to_string())
    }

    pub fn available_external_editors() -> Vec<String> {
        let mut editors = Vec::new();

        #[cfg(windows)]
        {
            editors.push("notepad".to_string());
        }

        if Self::detect_vscode_path().is_some() {
            editors.push("vscode".to_string());
        }

        if Self::detect_visual_studio_path().is_some() {
            editors.push("visualstudio".to_string());
        }

        if Self::detect_android_studio_path().is_some() {
            editors.push("androidstudio".to_string());
        }

        if Self::detect_intellij_path().is_some() {
            editors.push("intellij".to_string());
        }

        editors
    }

    pub fn available_external_tools() -> Vec<String> {
        let mut tools = Vec::new();

        if Self::detect_vscode_path().is_some() {
            tools.push("vscode".to_string());
        }

        if Self::detect_visual_studio_path().is_some() {
            tools.push("visualstudio".to_string());
        }

        if Self::detect_android_studio_path().is_some() {
            tools.push("androidstudio".to_string());
        }

        if Self::detect_intellij_path().is_some() {
            tools.push("intellij".to_string());
        }

        tools.push("explorer".to_string());

        tools
    }

    pub fn open_file_with_editor(
        repo_path: &str,
        file_path: &str,
        editor: &str,
    ) -> Result<(), String> {
        let resolved_file_path = Self::resolve_repo_file_path(repo_path, file_path)?;

        if !resolved_file_path.exists() {
            return Err(format!(
                "File does not exist on disk: {}",
                resolved_file_path.display()
            ));
        }

        match editor {
            "notepad" => Self::open_with_notepad(&resolved_file_path),
            "vscode" => Self::open_with_vscode(&resolved_file_path),
            "visualstudio" | "vs" => Self::open_with_visual_studio(&resolved_file_path),
            "androidstudio" => Self::open_with_android_studio(&resolved_file_path),
            "intellij" => Self::open_with_intellij(&resolved_file_path),
            _ => Err(format!("Unsupported editor '{}'.", editor)),
        }
    }

    pub fn open_path_with_tool(target_path: &str, tool: &str) -> Result<(), String> {
        let resolved_path = PathBuf::from(target_path);
        if !resolved_path.exists() {
            return Err(format!(
                "Path does not exist on disk: {}",
                resolved_path.display()
            ));
        }

        match tool {
            "vscode" => Self::open_with_vscode(&resolved_path),
            "visualstudio" | "vs" => Self::open_with_visual_studio(&resolved_path),
            "androidstudio" | "android-studio" => Self::open_with_android_studio(&resolved_path),
            "intellij" | "idea" => Self::open_with_intellij(&resolved_path),
            "explorer" | "file-explorer" | "folder" | "finder" => {
                Self::open_with_system_explorer(&resolved_path)
            }
            _ => Err(format!("Unsupported tool '{}'.", tool)),
        }
    }

    fn resolve_repo_file_path(repo_path: &str, file_path: &str) -> Result<PathBuf, String> {
        let candidate = PathBuf::from(file_path);
        if candidate.is_absolute() {
            return Ok(candidate);
        }

        let base = Path::new(repo_path);
        if !base.exists() {
            return Err(format!("Repository path not found: {}", repo_path));
        }

        Ok(base.join(candidate))
    }

    #[cfg(windows)]
    fn open_with_notepad(file_path: &Path) -> Result<(), String> {
        let binary = Self::detect_notepad_path().unwrap_or_else(|| PathBuf::from("notepad.exe"));
        let mut command = std::process::Command::new(binary);
        command.arg(file_path);
        Self::spawn_editor(command)
    }

    #[cfg(not(windows))]
    fn open_with_notepad(_file_path: &Path) -> Result<(), String> {
        Err("Notepad is only available on Windows.".to_string())
    }

    fn open_with_vscode(file_path: &Path) -> Result<(), String> {
        let binary = Self::detect_vscode_path()
            .ok_or_else(|| "VS Code executable not found on this machine.".to_string())?;

        let mut command = std::process::Command::new(binary);
        command.arg("--reuse-window").arg(file_path);
        Self::spawn_editor(command)
    }

    fn open_with_visual_studio(file_path: &Path) -> Result<(), String> {
        let binary = Self::detect_visual_studio_path().ok_or_else(|| {
            "Visual Studio executable (devenv.exe) not found on this machine.".to_string()
        })?;

        let mut command = std::process::Command::new(binary);
        command.arg(file_path);
        Self::spawn_editor(command)
    }

    fn open_with_android_studio(file_path: &Path) -> Result<(), String> {
        let binary = Self::detect_android_studio_path()
            .ok_or_else(|| "Android Studio executable not found on this machine.".to_string())?;

        let mut command = std::process::Command::new(binary);
        command.arg(file_path);
        Self::spawn_editor(command)
    }

    fn open_with_intellij(file_path: &Path) -> Result<(), String> {
        let binary = Self::detect_intellij_path().ok_or_else(|| {
            "IntelliJ IDEA executable not found. Install IntelliJ or add it to PATH.".to_string()
        })?;

        let mut command = std::process::Command::new(binary);
        command.arg(file_path);
        Self::spawn_editor(command)
    }

    fn open_with_system_explorer(target_path: &Path) -> Result<(), String> {
        #[cfg(windows)]
        {
            let mut command = std::process::Command::new("explorer.exe");
            command.arg(target_path);
            return Self::spawn_editor(command);
        }

        #[cfg(target_os = "macos")]
        {
            let mut command = std::process::Command::new("open");
            command.arg(target_path);
            return Self::spawn_editor(command);
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let mut command = std::process::Command::new("xdg-open");
            command.arg(target_path);
            return Self::spawn_editor(command);
        }

        #[allow(unreachable_code)]
        Err("System explorer is not supported on this platform.".to_string())
    }

    fn spawn_editor(mut command: std::process::Command) -> Result<(), String> {
        #[cfg(windows)]
        {
            command.creation_flags(CREATE_NO_WINDOW);
        }

        command
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to launch editor: {}", e))
    }

    #[cfg(windows)]
    fn detect_notepad_path() -> Option<PathBuf> {
        if let Ok(win_dir) = std::env::var("WINDIR") {
            let p = Path::new(&win_dir).join("System32").join("notepad.exe");
            if p.exists() {
                return Some(p);
            }
        }

        Self::find_command_in_path(&["notepad.exe", "notepad"])
    }

    fn detect_vscode_path() -> Option<PathBuf> {
        if let Some(path) = Self::find_command_in_path(&[
            "code.cmd",
            "code.exe",
            "code",
            "Code.exe",
            "code-insiders.cmd",
            "code-insiders.exe",
        ]) {
            return Some(path);
        }

        let mut candidates: Vec<PathBuf> = vec![
            PathBuf::from(r"C:\Program Files\Microsoft VS Code\Code.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Microsoft VS Code\Code.exe"),
            PathBuf::from(r"C:\Program Files\Microsoft VS Code Insiders\Code - Insiders.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Microsoft VS Code Insiders\Code - Insiders.exe"),
        ];

        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            candidates.push(
                Path::new(&local_app_data)
                    .join("Programs")
                    .join("Microsoft VS Code")
                    .join("Code.exe"),
            );
            candidates.push(
                Path::new(&local_app_data)
                    .join("Programs")
                    .join("Microsoft VS Code Insiders")
                    .join("Code - Insiders.exe"),
            );
        }

        candidates.into_iter().find(|p| p.exists())
    }

    fn detect_visual_studio_path() -> Option<PathBuf> {
        if let Some(path) = Self::find_command_in_path(&["devenv.exe", "devenv"]) {
            return Some(path);
        }

        let direct_candidates = vec![
            PathBuf::from(
                r"C:\Program Files\Microsoft Visual Studio\2022\Enterprise\Common7\IDE\devenv.exe",
            ),
            PathBuf::from(
                r"C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\IDE\devenv.exe",
            ),
            PathBuf::from(
                r"C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\devenv.exe",
            ),
            PathBuf::from(
                r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Enterprise\Common7\IDE\devenv.exe",
            ),
            PathBuf::from(
                r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Professional\Common7\IDE\devenv.exe",
            ),
            PathBuf::from(
                r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\Common7\IDE\devenv.exe",
            ),
        ];

        if let Some(path) = direct_candidates.into_iter().find(|p| p.exists()) {
            return Some(path);
        }

        if let Ok(program_files) = std::env::var("ProgramFiles") {
            if let Some(path) = Self::find_visual_studio_installation(
                &Path::new(&program_files).join("Microsoft Visual Studio"),
            ) {
                return Some(path);
            }
        }

        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            if let Some(path) = Self::find_visual_studio_installation(
                &Path::new(&program_files_x86).join("Microsoft Visual Studio"),
            ) {
                return Some(path);
            }
        }

        None
    }

    fn detect_android_studio_path() -> Option<PathBuf> {
        if let Some(path) =
            Self::find_command_in_path(&["studio64.exe", "studio.exe", "studio.bat", "studio"])
        {
            return Some(path);
        }

        let direct_candidates = vec![
            PathBuf::from(r"C:\Program Files\Android\Android Studio\bin\studio64.exe"),
            PathBuf::from(r"C:\Program Files\Android\Android Studio\bin\studio.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Android\Android Studio\bin\studio64.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Android\Android Studio\bin\studio.exe"),
        ];

        if let Some(path) = direct_candidates.into_iter().find(|p| p.exists()) {
            return Some(path);
        }

        if let Some(path) = Self::detect_android_studio_from_toolbox() {
            return Some(path);
        }

        None
    }

    fn detect_android_studio_from_toolbox() -> Option<PathBuf> {
        let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
        let toolbox_apps = Path::new(&local_app_data)
            .join("JetBrains")
            .join("Toolbox")
            .join("apps")
            .join("AndroidStudio");

        Self::find_latest_toolbox_studio_binary(&toolbox_apps)
    }

    fn find_latest_toolbox_studio_binary(studio_dir: &Path) -> Option<PathBuf> {
        if !studio_dir.exists() {
            return None;
        }

        let mut candidates = Vec::new();

        let channels = fs::read_dir(studio_dir).ok()?;
        for channel_entry in channels.flatten() {
            let channel_path = channel_entry.path();
            if !channel_path.is_dir() {
                continue;
            }

            let builds = match fs::read_dir(&channel_path) {
                Ok(value) => value,
                Err(_) => continue,
            };

            for build_entry in builds.flatten() {
                let build_path = build_entry.path();
                if !build_path.is_dir() {
                    continue;
                }

                let bin_dir = build_path.join("bin");
                for name in ["studio64.exe", "studio.exe", "studio.bat", "studio.sh"] {
                    let candidate = bin_dir.join(name);
                    if candidate.exists() {
                        candidates.push(candidate);
                        break;
                    }
                }
            }
        }

        candidates.sort();
        candidates.pop()
    }

    fn find_visual_studio_installation(base: &Path) -> Option<PathBuf> {
        if !base.exists() {
            return None;
        }

        let mut candidates = Vec::new();
        let years = fs::read_dir(base).ok()?;

        for year_entry in years.flatten() {
            let year_path = year_entry.path();
            if !year_path.is_dir() {
                continue;
            }

            let editions = match fs::read_dir(&year_path) {
                Ok(value) => value,
                Err(_) => continue,
            };

            for edition_entry in editions.flatten() {
                let install_root = edition_entry.path();
                if !install_root.is_dir() {
                    continue;
                }

                let devenv = install_root.join("Common7").join("IDE").join("devenv.exe");
                if devenv.exists() {
                    candidates.push(devenv);
                }
            }
        }

        candidates.sort();
        candidates.pop()
    }

    fn detect_intellij_path() -> Option<PathBuf> {
        if let Some(path) =
            Self::find_command_in_path(&["idea64.exe", "idea.exe", "idea.bat", "idea"])
        {
            return Some(path);
        }

        let direct_candidates = vec![
            PathBuf::from(r"C:\Program Files\JetBrains\IntelliJ IDEA\bin\idea64.exe"),
            PathBuf::from(
                r"C:\Program Files\JetBrains\IntelliJ IDEA Community Edition\bin\idea64.exe",
            ),
            PathBuf::from(r"C:\Program Files\JetBrains\IntelliJ IDEA Ultimate\bin\idea64.exe"),
            PathBuf::from(r"C:\Program Files (x86)\JetBrains\IntelliJ IDEA\bin\idea.exe"),
            PathBuf::from(
                r"C:\Program Files (x86)\JetBrains\IntelliJ IDEA Community Edition\bin\idea.exe",
            ),
        ];

        if let Some(path) = direct_candidates.into_iter().find(|p| p.exists()) {
            return Some(path);
        }

        if let Some(path) = Self::detect_intellij_from_toolbox() {
            return Some(path);
        }

        if let Ok(program_files) = std::env::var("ProgramFiles") {
            if let Some(path) =
                Self::find_intellij_installation(&Path::new(&program_files).join("JetBrains"))
            {
                return Some(path);
            }
        }

        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            if let Some(path) =
                Self::find_intellij_installation(&Path::new(&program_files_x86).join("JetBrains"))
            {
                return Some(path);
            }
        }

        None
    }

    fn detect_intellij_from_toolbox() -> Option<PathBuf> {
        let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
        let toolbox_apps = Path::new(&local_app_data)
            .join("JetBrains")
            .join("Toolbox")
            .join("apps");

        let families = ["IDEA-U", "IDEA-C", "IntelliJIdea"];
        for family in families {
            if let Some(path) = Self::find_latest_toolbox_idea_binary(&toolbox_apps.join(family)) {
                return Some(path);
            }
        }

        None
    }

    fn find_latest_toolbox_idea_binary(family_dir: &Path) -> Option<PathBuf> {
        if !family_dir.exists() {
            return None;
        }

        let mut candidates = Vec::new();

        let channels = fs::read_dir(family_dir).ok()?;
        for channel_entry in channels.flatten() {
            let channel_path = channel_entry.path();
            if !channel_path.is_dir() {
                continue;
            }

            let builds = match fs::read_dir(&channel_path) {
                Ok(v) => v,
                Err(_) => continue,
            };

            for build_entry in builds.flatten() {
                let build_path = build_entry.path();
                if !build_path.is_dir() {
                    continue;
                }
                if let Some(binary) = Self::idea_binary_from_install_root(&build_path) {
                    candidates.push(binary);
                }
            }
        }

        candidates.sort();
        candidates.pop()
    }

    fn find_intellij_installation(base: &Path) -> Option<PathBuf> {
        if !base.exists() {
            return None;
        }

        let mut candidates = Vec::new();

        let entries = fs::read_dir(base).ok()?;
        for entry in entries.flatten() {
            let install_root = entry.path();
            if !install_root.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !name.contains("intellij") && !name.contains("idea") {
                continue;
            }

            if let Some(binary) = Self::idea_binary_from_install_root(&install_root) {
                candidates.push(binary);
            }
        }

        candidates.sort();
        candidates.pop()
    }

    fn idea_binary_from_install_root(install_root: &Path) -> Option<PathBuf> {
        let bin_dir = install_root.join("bin");
        if !bin_dir.exists() {
            return None;
        }

        ["idea64.exe", "idea.exe", "idea.bat", "idea.sh"]
            .iter()
            .map(|name| bin_dir.join(name))
            .find(|path| path.exists())
    }

    fn push_unique_path(paths: &mut Vec<PathBuf>, value: PathBuf) {
        if value.as_os_str().is_empty() {
            return;
        }
        if paths.iter().any(|existing| existing == &value) {
            return;
        }
        paths.push(value);
    }

    fn normalize_ssh_public_key(value: &str) -> Result<String, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("SSH public key is empty.".to_string());
        }

        let mut parts = trimmed.split_whitespace();
        let key_type = parts.next().unwrap_or_default();
        let key_body = parts.next().unwrap_or_default();
        if key_type.is_empty() || key_body.is_empty() {
            return Err("Invalid SSH public key format.".to_string());
        }

        let remainder = parts.collect::<Vec<_>>().join(" ");
        if remainder.is_empty() {
            Ok(format!("{} {}", key_type, key_body))
        } else {
            Ok(format!("{} {} {}", key_type, key_body, remainder))
        }
    }

    fn ssh_keygen_candidates() -> Vec<PathBuf> {
        let mut candidates: Vec<PathBuf> = Vec::new();

        if let Ok(custom) = std::env::var("GITSWAMP_SSH_KEYGEN") {
            let custom_path = PathBuf::from(custom.trim());
            if !custom_path.as_os_str().is_empty() {
                Self::push_unique_path(&mut candidates, custom_path);
            }
        }

        if let Some(path) = Self::find_command_in_path(&["ssh-keygen.exe", "ssh-keygen"]) {
            Self::push_unique_path(&mut candidates, path);
        }

        let static_paths = [
            PathBuf::from("C:\\Program Files\\Git\\usr\\bin\\ssh-keygen.exe"),
            PathBuf::from("C:\\Program Files (x86)\\Git\\usr\\bin\\ssh-keygen.exe"),
            PathBuf::from("C:\\Program Files\\Git\\bin\\ssh-keygen.exe"),
            PathBuf::from("C:\\Program Files (x86)\\Git\\bin\\ssh-keygen.exe"),
            PathBuf::from("C:\\Program Files\\Git\\cmd\\ssh-keygen.exe"),
            PathBuf::from("C:\\Windows\\System32\\OpenSSH\\ssh-keygen.exe"),
            PathBuf::from("/bin/ssh-keygen"),
            PathBuf::from("/usr/bin/ssh-keygen"),
            PathBuf::from("/usr/local/bin/ssh-keygen"),
            PathBuf::from("/usr/sbin/ssh-keygen"),
            PathBuf::from("/usr/local/sbin/ssh-keygen"),
            PathBuf::from("/usr/lib/ssh/ssh-keygen"),
            PathBuf::from("/snap/bin/ssh-keygen"),
            PathBuf::from("/opt/homebrew/bin/ssh-keygen"),
        ];

        for candidate in static_paths {
            if candidate.exists() {
                Self::push_unique_path(&mut candidates, candidate);
            }
        }

        if let Some(path_env) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path_env) {
                for binary in ["ssh-keygen.exe", "ssh-keygen"] {
                    let candidate = dir.join(binary);
                    if candidate.exists() {
                        Self::push_unique_path(&mut candidates, candidate);
                    }
                }
            }
        }

        if candidates.is_empty() {
            if cfg!(windows) {
                candidates.push(PathBuf::from("ssh-keygen.exe"));
            } else {
                candidates.push(PathBuf::from("ssh-keygen"));
            }
        }

        candidates
    }

    fn derive_public_key(
        candidates: &[PathBuf],
        private_key_path: &Path,
    ) -> Result<String, String> {
        let mut failures: Vec<String> = Vec::new();

        for ssh_keygen in candidates {
            let mut cmd = std::process::Command::new(ssh_keygen);
            cmd.arg("-y").arg("-f").arg(private_key_path);

            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);

            match cmd.output() {
                Ok(output) if output.status.success() => {
                    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if value.is_empty() {
                        failures.push(format!(
                            "{} -> empty stdout while deriving public key",
                            ssh_keygen.display()
                        ));
                        continue;
                    }
                    return Ok(value + "\n");
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let detail = if !stderr.is_empty() { stderr } else { stdout };
                    failures.push(format!(
                        "{} -> {}",
                        ssh_keygen.display(),
                        if detail.is_empty() {
                            "no additional error output".to_string()
                        } else {
                            detail
                        }
                    ));
                }
                Err(error) => {
                    failures.push(format!("{} -> {}", ssh_keygen.display(), error));
                }
            }
        }

        Err(format!(
            "Failed to derive SSH public key from existing private key: {}",
            failures.join(" | ")
        ))
    }

    fn find_command_in_path(candidates: &[&str]) -> Option<PathBuf> {
        for candidate in candidates {
            #[cfg(windows)]
            let mut command = {
                let mut c = std::process::Command::new("where");
                c.creation_flags(CREATE_NO_WINDOW);
                c
            };

            #[cfg(not(windows))]
            let mut command = std::process::Command::new("which");

            let output = match command.arg(candidate).output() {
                Ok(value) => value,
                Err(_) => continue,
            };
            if !output.status.success() {
                continue;
            }

            let stdout = match String::from_utf8(output.stdout) {
                Ok(value) => value,
                Err(_) => continue,
            };
            for line in stdout.lines() {
                let path = PathBuf::from(line.trim());
                if path.exists() {
                    return Some(path);
                }
            }
        }

        None
    }
}

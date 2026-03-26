use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::constants::{
    API_GITHUB_LIST_REPOS, API_GITHUB_SEARCH_REPOS, API_GITLAB_BASE_PATH,
    API_GITLAB_USER_KEYS_PATH, API_GITLAB_USER_PATH, APP_USER_AGENT, CREATE_NO_WINDOW,
    GITHUB_ACCEPT_HEADER, HTTPS_SCHEME, JSON_ACCEPT_HEADER,
};
use crate::models::{GithubRepo, GitlabRepo};
use crate::services::helpers::urlencoded;

pub struct IntegrationService;

impl IntegrationService {
    pub fn search_github_repos(token: &str, query: &str) -> Result<Vec<GithubRepo>, String> {
        let url = if query.is_empty() {
            API_GITHUB_LIST_REPOS.to_string()
        } else {
            API_GITHUB_SEARCH_REPOS.replace("{}", &urlencoded(query))
        };
        let resp = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", GITHUB_ACCEPT_HEADER)
            .set("User-Agent", APP_USER_AGENT)
            .call()
            .map_err(|e| format!("GitHub API error: {}", e))?;
        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;
        let items = if query.is_empty() {
            body.as_array().cloned().unwrap_or_default()
        } else {
            body["items"].as_array().cloned().unwrap_or_default()
        };
        let repos = items
            .iter()
            .filter_map(|item| {
                Some(GithubRepo {
                    full_name: item["full_name"].as_str()?.to_string(),
                    clone_url: item["clone_url"].as_str()?.to_string(),
                    description: item["description"].as_str().unwrap_or("").to_string(),
                    is_private: item["private"].as_bool().unwrap_or(false),
                    stars: item["stargazers_count"].as_u64().unwrap_or(0) as u32,
                })
            })
            .collect();
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

    pub fn generate_ssh_key(email: &str, key_name: &str) -> Result<(String, String), String> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "Cannot find home directory")?;

        let ssh_dir = Path::new(&home).join(".ssh");
        std::fs::create_dir_all(&ssh_dir).map_err(|e| format!("Failed to create .ssh dir: {}", e))?;

        let key_path = ssh_dir.join(key_name);
        let pub_key_path = ssh_dir.join(format!("{}.pub", key_name));

        if key_path.exists() {
            let pub_key = std::fs::read_to_string(&pub_key_path)
                .map_err(|e| format!("Failed to read existing public key: {}", e))?;
            return Ok((key_path.to_string_lossy().to_string(), pub_key));
        }

        let ssh_keygen_paths = [
            Path::new("C:\\Program Files\\Git\\usr\\bin\\ssh-keygen.exe").to_path_buf(),
            Path::new("C:\\Program Files (x86)\\Git\\usr\\bin\\ssh-keygen.exe").to_path_buf(),
            Path::new("C:\\Program Files\\Git\\bin\\ssh-keygen.exe").to_path_buf(),
            Path::new("C:\\Program Files (x86)\\Git\\bin\\ssh-keygen.exe").to_path_buf(),
            Path::new("C:\\Program Files\\Git\\cmd\\ssh-keygen.exe").to_path_buf(),
            Path::new("C:\\Windows\\System32\\OpenSSH\\ssh-keygen.exe").to_path_buf(),
            Path::new("/usr/bin/ssh-keygen").to_path_buf(),
            Path::new("/usr/local/bin/ssh-keygen").to_path_buf(),
        ];

        let from_path: Option<PathBuf> = {
            #[cfg(windows)]
            {
                std::process::Command::new("where")
                    .arg("ssh-keygen")
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .and_then(|s| s.lines().next().map(|l| PathBuf::from(l.trim())))
                    .filter(|p: &PathBuf| p.exists())
            }
            #[cfg(not(windows))]
            {
                std::process::Command::new("which")
                    .arg("ssh-keygen")
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| PathBuf::from(s.trim()))
                    .filter(|p: &PathBuf| p.exists())
            }
        };

        let ssh_keygen = from_path
            .or_else(|| ssh_keygen_paths.iter().find(|p| p.exists()).cloned())
            .ok_or("ssh-keygen not found. Please ensure Git is installed with SSH support, or install Windows OpenSSH.")?;

        let mut cmd = std::process::Command::new(ssh_keygen);
        cmd.args(["-t", "ed25519", "-C", email, "-f"])
            .arg(&key_path)
            .arg("-N")
            .arg("");

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run ssh-keygen: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "ssh-keygen failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let pub_key = std::fs::read_to_string(&pub_key_path)
            .map_err(|e| format!("Failed to read public key: {}", e))?;

        Ok((key_path.to_string_lossy().to_string(), pub_key))
    }

    pub fn add_gitlab_ssh_key(domain: &str, token: &str, title: &str, key: &str) -> Result<(), String> {
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
}

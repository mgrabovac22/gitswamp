use std::fs;
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
        let binary = Self::detect_android_studio_path().ok_or_else(|| {
            "Android Studio executable not found on this machine.".to_string()
        })?;

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
            if let Some(path) =
                Self::find_visual_studio_installation(&Path::new(&program_files).join("Microsoft Visual Studio"))
            {
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
        if let Some(path) = Self::find_command_in_path(&[
            "studio64.exe",
            "studio.exe",
            "studio.bat",
            "studio",
        ]) {
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
        if let Some(path) = Self::find_command_in_path(&["idea64.exe", "idea.exe", "idea.bat", "idea"]) {
            return Some(path);
        }

        let direct_candidates = vec![
            PathBuf::from(r"C:\Program Files\JetBrains\IntelliJ IDEA\bin\idea64.exe"),
            PathBuf::from(r"C:\Program Files\JetBrains\IntelliJ IDEA Community Edition\bin\idea64.exe"),
            PathBuf::from(r"C:\Program Files\JetBrains\IntelliJ IDEA Ultimate\bin\idea64.exe"),
            PathBuf::from(r"C:\Program Files (x86)\JetBrains\IntelliJ IDEA\bin\idea.exe"),
            PathBuf::from(r"C:\Program Files (x86)\JetBrains\IntelliJ IDEA Community Edition\bin\idea.exe"),
        ];

        if let Some(path) = direct_candidates.into_iter().find(|p| p.exists()) {
            return Some(path);
        }

        if let Some(path) = Self::detect_intellij_from_toolbox() {
            return Some(path);
        }

        if let Ok(program_files) = std::env::var("ProgramFiles") {
            if let Some(path) = Self::find_intellij_installation(&Path::new(&program_files).join("JetBrains")) {
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

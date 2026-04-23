use std::path::Path;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use git2::Repository;

#[cfg(windows)]
use crate::constants::{COMMON_GIT_PATHS, CREATE_NO_WINDOW};

pub struct GitRepository;

impl GitRepository {
    pub fn open(path: &str) -> Result<Repository, String> {
        Repository::open(path).map_err(|e| e.message().to_string())
    }

    pub fn get_git_path() -> String {
        let exe = Self::git_executable();
        let full = Self::full_path_live();
        let path_count = full
            .split(Self::path_separator())
            .filter(|s| !s.trim().is_empty())
            .count();
        format!("{} (PATH has {} dirs)", exe, path_count)
    }

    pub fn install_git() -> Result<String, String> {
        if Self::is_git_available() {
            return Ok("Git is already installed.".to_string());
        }

        #[cfg(windows)]
        {
            if Self::command_exists("winget") {
                let mut cmd = std::process::Command::new("winget");
                cmd.args([
                    "install",
                    "--id",
                    "Git.Git",
                    "-e",
                    "--source",
                    "winget",
                    "--accept-source-agreements",
                    "--accept-package-agreements",
                ]);
                cmd.creation_flags(CREATE_NO_WINDOW);

                return cmd
                    .spawn()
                    .map(|_| {
                        "Started Git installation via winget. After completion, click Refresh detection.".to_string()
                    })
                    .map_err(|e| e.to_string());
            }

            Self::open_download_page("https://git-scm.com/download/win")?;
            return Ok("Winget was not found. Opened Git download page for Windows.".to_string());
        }

        #[cfg(target_os = "macos")]
        {
            if Self::command_exists("brew") {
                return std::process::Command::new("brew")
                    .args(["install", "git"])
                    .spawn()
                    .map(|_| {
                        "Started Git installation via Homebrew. After completion, click Refresh detection.".to_string()
                    })
                    .map_err(|e| e.to_string());
            }

            Self::open_download_page("https://git-scm.com/download/mac")?;
            return Ok("Homebrew was not found. Opened Git download page for macOS.".to_string());
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            Self::open_download_page("https://git-scm.com/download/linux")?;
            Ok("Opened Git installation instructions for Linux.".to_string())
        }
    }

    pub fn git_cli(path: &str, args: &[&str]) -> Result<String, String> {
        Self::run_git_cmd(Some(path), args)
    }

    pub fn run_shell_command(path: &str, command: &str) -> Result<String, String> {
        if command.trim().is_empty() {
            return Ok(String::new());
        }

        let live_path = Self::full_path_live();

        #[cfg(windows)]
        {
            let output = std::process::Command::new("cmd.exe")
                .args(["/d", "/s", "/c", command])
                .current_dir(path)
                .env("PATH", &live_path)
                .env("GIT_TERMINAL_PROMPT", "0")
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| e.to_string())?;

            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

            if output.status.success() {
                return Ok([stdout, stderr]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n"));
            }

            let message = if !stderr.is_empty() { stderr } else { stdout };

            if let Some(git_args) = Self::extract_git_args(command) {
                let git_arg_refs: Vec<&str> = git_args.iter().map(|value| value.as_str()).collect();
                if let Ok(fallback) = Self::run_git_cmd(Some(path), &git_arg_refs) {
                    return Ok(fallback);
                }
            }

            return Err(if message.is_empty() {
                format!("Command failed with exit code {:?}", output.status.code())
            } else {
                message
            });
        }

        #[cfg(not(windows))]
        {
            let output = std::process::Command::new("sh")
                .args(["-lc", command])
                .current_dir(path)
                .env("GIT_TERMINAL_PROMPT", "0")
                .output()
                .map_err(|e| e.to_string())?;

            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

            if output.status.success() {
                return Ok([stdout, stderr]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n"));
            }

            let message = if !stderr.is_empty() { stderr } else { stdout };

            if let Some(git_args) = Self::extract_git_args(command) {
                let git_arg_refs: Vec<&str> = git_args.iter().map(|value| value.as_str()).collect();
                if let Ok(fallback) = Self::run_git_cmd(Some(path), &git_arg_refs) {
                    return Ok(fallback);
                }
            }

            Err(if message.is_empty() {
                format!("Command failed with exit code {:?}", output.status.code())
            } else {
                message
            })
        }
    }

    fn path_separator() -> char {
        if cfg!(windows) {
            ';'
        } else {
            ':'
        }
    }

    fn is_git_available() -> bool {
        if let Ok(output) = std::process::Command::new("git").arg("--version").output() {
            if output.status.success() {
                return true;
            }
        }

        let detected = Self::find_git();
        detected != "git" && Path::new(&detected).exists()
    }

    fn command_exists(name: &str) -> bool {
        std::process::Command::new(name)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn open_download_page(url: &str) -> Result<(), String> {
        #[cfg(windows)]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", url])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| e.to_string())?;
            return Ok(());
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(url)
                .spawn()
                .map_err(|e| e.to_string())?;
            return Ok(());
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            std::process::Command::new("xdg-open")
                .arg(url)
                .spawn()
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    fn full_path_live() -> String {
        #[cfg(not(windows))]
        {
            return std::env::var("PATH").unwrap_or_default();
        }

        let mut paths: Vec<String> = Vec::new();

        if let Ok(current) = std::env::var("PATH") {
            for dir in current.split(';') {
                let d = dir.trim().to_string();
                if !d.is_empty() {
                    paths.push(d);
                }
            }
        }

        #[cfg(windows)]
        {
            if let Ok(output) = std::process::Command::new("reg")
                .args(&[
                    "query",
                    r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
                    "/v",
                    "Path",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                if output.status.success() {
                    Self::extract_reg_paths(&String::from_utf8_lossy(&output.stdout), &mut paths);
                }
            }

            if let Ok(output) = std::process::Command::new("reg")
                .args(&["query", r"HKCU\Environment", "/v", "Path"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                if output.status.success() {
                    Self::extract_reg_paths(&String::from_utf8_lossy(&output.stdout), &mut paths);
                }
            }
        }

        paths.join(";")
    }

    fn git_executable() -> String {
        Self::find_git()
    }

    fn run_git_cmd(cwd: Option<&str>, args: &[&str]) -> Result<String, String> {
        let primary = Self::git_executable();
        let full_path = Self::full_path_live();
        let separator = Self::path_separator();
        let git_name = if cfg!(windows) { "git.exe" } else { "git" };
        let split_count = full_path
            .split(separator)
            .filter(|s| !s.trim().is_empty())
            .count();

        if let Ok(output) = Self::try_git_exec(primary.as_str(), cwd, args, full_path.as_str()) {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Ok(format!("{}{}", stdout, stderr).trim().to_string());
            }
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let msg = if !stderr.is_empty() { stderr } else { stdout };
            if !msg.is_empty() {
                return Err(msg);
            }
        }

        for dir in full_path.split(separator) {
            let dir = dir.trim();
            if dir.is_empty() {
                continue;
            }
            let candidate = Path::new(dir).join(git_name);
            if candidate.exists() {
                let exe_str = candidate.to_string_lossy().to_string();
                if exe_str != primary {
                    if let Ok(output) = Self::try_git_exec(&exe_str, cwd, args, full_path.as_str()) {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            return Ok(format!("{}{}", stdout, stderr).trim().to_string());
                        }
                        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
                    }
                }
            }
        }

        #[cfg(windows)]
        for fallback in COMMON_GIT_PATHS {
            if Path::new(fallback).exists() {
                if let Ok(output) = Self::try_git_exec(fallback, cwd, args, full_path.as_str()) {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Ok(format!("{}{}", stdout, stderr).trim().to_string());
                    }
                    return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
                }
            }
        }

        #[cfg(windows)]
        {
            let mut cmd_args = vec!["/c", "git"];
            cmd_args.extend_from_slice(args);
            let mut cmd = std::process::Command::new("cmd.exe");
            cmd.args(&cmd_args);
            if let Some(dir) = cwd {
                cmd.current_dir(dir);
            }
            cmd.env("GIT_TERMINAL_PROMPT", "0");
            cmd.env("PATH", full_path.as_str());
            cmd.creation_flags(CREATE_NO_WINDOW);
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Ok(format!("{}{}", stdout, stderr).trim().to_string());
                }
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
            }
        }

        #[cfg(windows)]
        {
            return Err(format!(
                "Git not found. Detected path: '{}'. Searched {} PATH dirs and {} common locations.",
                primary,
                split_count,
                COMMON_GIT_PATHS.len()
            ));
        }

        #[cfg(not(windows))]
        {
            Err(format!(
                "Git not found. Detected path: '{}'. Searched {} PATH dirs.",
                primary, split_count
            ))
        }
    }

    fn find_git() -> String {
        let git_name = if cfg!(windows) { "git.exe" } else { "git" };
        let separator = Self::path_separator();

        if let Ok(path_var) = std::env::var("PATH") {
            for dir in path_var.split(separator) {
                let dir = dir.trim();
                if dir.is_empty() {
                    continue;
                }
                let git_path = Path::new(dir).join(git_name);
                if git_path.exists() {
                    return git_path.to_string_lossy().to_string();
                }
            }
        }

        let full = Self::full_path_live();
        for dir in full.split(separator) {
            let dir = dir.trim();
            if dir.is_empty() {
                continue;
            }
            let git_path = Path::new(dir).join(git_name);
            if git_path.exists() {
                return git_path.to_string_lossy().to_string();
            }
        }

        #[cfg(not(windows))]
        {
            for p in [
                "/usr/bin/git",
                "/usr/local/bin/git",
                "/opt/homebrew/bin/git",
                "/opt/local/bin/git",
            ] {
                if Path::new(p).exists() {
                    return p.to_string();
                }
            }
            if let Ok(output) = std::process::Command::new("which").arg("git").output() {
                if output.status.success() {
                    let found = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !found.is_empty() && Path::new(&found).exists() {
                        return found;
                    }
                }
            }
            return "git".to_string();
        }

        #[cfg(windows)]
        {
            for p in COMMON_GIT_PATHS {
                if Path::new(p).exists() {
                    return p.to_string();
                }
            }

            for env_key in &["LOCALAPPDATA", "APPDATA", "USERPROFILE"] {
                if let Ok(base) = std::env::var(env_key) {
                    let paths = [
                        format!(r"{}\Programs\Git\cmd\git.exe", base),
                        format!(r"{}\Programs\Git\bin\git.exe", base),
                        format!(r"{}\Git\cmd\git.exe", base),
                        format!(r"{}\scoop\shims\git.exe", base),
                        format!(r"{}\scoop\apps\git\current\cmd\git.exe", base),
                    ];
                    for p in &paths {
                        if Path::new(p).exists() {
                            return p.to_string();
                        }
                    }
                }
            }

            for env_key in &["PROGRAMFILES", "ProgramFiles(x86)", "ProgramW6432"] {
                if let Ok(pf) = std::env::var(env_key) {
                    let p = format!(r"{}\Git\cmd\git.exe", pf);
                    if Path::new(&p).exists() {
                        return p;
                    }
                    let p = format!(r"{}\Git\bin\git.exe", pf);
                    if Path::new(&p).exists() {
                        return p;
                    }
                }
            }

            for reg_path in &[
                r"HKLM\SOFTWARE\GitForWindows",
                r"HKCU\SOFTWARE\GitForWindows",
                r"HKLM\SOFTWARE\Wow6432Node\GitForWindows",
            ] {
                if let Ok(output) = std::process::Command::new("reg")
                    .args(&["query", reg_path, "/v", "InstallPath"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
                {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout);
                        for line in text.lines() {
                            if line.contains("InstallPath") {
                                if let Some(val) = line.split("REG_SZ").nth(1) {
                                    let install = val.trim();
                                    for sub in &["cmd", "bin", "mingw64\\bin"] {
                                        let p = format!(r"{}\{}\git.exe", install, sub);
                                        if Path::new(&p).exists() {
                                            return p;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Ok(output) = std::process::Command::new("git").arg("--version").output() {
                if output.status.success() {
                    return "git".to_string();
                }
            }

            if let Ok(output) = std::process::Command::new("where.exe")
                .arg("git")
                .creation_flags(CREATE_NO_WINDOW)
                .env("PATH", Self::full_path_live())
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

            if let Ok(output) = std::process::Command::new("cmd")
                .args(&["/c", "where", "git"])
                .creation_flags(CREATE_NO_WINDOW)
                .env("PATH", Self::full_path_live())
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
        }

        "git".to_string()
    }

    fn parse_shell_like_args(input: &str) -> Option<Vec<String>> {
        let mut args = Vec::<String>::new();
        let mut current = String::new();
        let mut quote: Option<char> = None;
        let mut escaping = false;

        for ch in input.chars() {
            if escaping {
                current.push(ch);
                escaping = false;
                continue;
            }

            if let Some(active_quote) = quote {
                if ch == active_quote {
                    quote = None;
                } else {
                    current.push(ch);
                }
                continue;
            }

            match ch {
                '\\' => escaping = true,
                '"' | '\'' => quote = Some(ch),
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        args.push(current.clone());
                        current.clear();
                    }
                }
                _ => current.push(ch),
            }
        }

        if escaping || quote.is_some() {
            return None;
        }

        if !current.is_empty() {
            args.push(current);
        }

        Some(args)
    }

    fn extract_git_args(command: &str) -> Option<Vec<String>> {
        let parsed = Self::parse_shell_like_args(command.trim())?;
        if parsed.is_empty() {
            return None;
        }

        let first = parsed[0].to_lowercase();
        if first == "git"
            || first.ends_with("/git")
            || first.ends_with("\\git")
            || first.ends_with("/git.exe")
            || first.ends_with("\\git.exe")
        {
            return Some(parsed.into_iter().skip(1).collect());
        }

        None
    }

    fn try_git_exec(
        exe: &str,
        cwd: Option<&str>,
        args: &[&str],
        path_env: &str,
    ) -> Result<std::process::Output, std::io::Error> {
        let mut cmd = std::process::Command::new(exe);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        #[cfg(windows)]
        {
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.env("PATH", path_env);
        }
        cmd.output()
    }

    fn expand_env_vars(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();
        let mut i = 0;
        while i < len {
            if chars[i] == '%' && i + 1 < len {
                if let Some(end_offset) = chars[i + 1..].iter().position(|&c| c == '%') {
                    let var_name: String = chars[i + 1..i + 1 + end_offset].iter().collect();
                    if !var_name.is_empty() {
                        if let Ok(val) = std::env::var(&var_name) {
                            result.push_str(&val);
                        }
                        i = i + 1 + end_offset + 1;
                        continue;
                    }
                }
            }
            result.push(chars[i]);
            i += 1;
        }
        result
    }

    fn extract_reg_paths(text: &str, paths: &mut Vec<String>) {
        for line in text.lines() {
            let val = line
                .split("REG_EXPAND_SZ")
                .nth(1)
                .or_else(|| line.split("REG_SZ").nth(1));
            if let Some(path_str) = val {
                for dir in path_str.trim().split(';') {
                    let expanded = Self::expand_env_vars(dir.trim());
                    if !expanded.is_empty()
                        && !paths.iter().any(|p| p.eq_ignore_ascii_case(&expanded))
                    {
                        paths.push(expanded);
                    }
                }
            }
        }
    }
}

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use git2::{BranchType, Repository, Sort, StashApplyOptions, StashFlags, StatusOptions};

use crate::models::{BranchInfo, CommitFileInfo, CommitInfo, DiffHunk, DiffLine, FileDiff, FileStatusInfo, GithubRepo, RepoInfo, StashInfo, TagInfo};

static GIT_PATH: OnceLock<String> = OnceLock::new();
static FULL_PATH: OnceLock<String> = OnceLock::new();

/// Expand all %VAR% references in a string using environment variables
#[allow(dead_code)]
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

/// Extract PATH dirs from a registry query line
#[allow(dead_code)]
fn extract_reg_paths(text: &str, paths: &mut Vec<String>) {
    for line in text.lines() {
        let val = line.split("REG_EXPAND_SZ").nth(1)
            .or_else(|| line.split("REG_SZ").nth(1));
        if let Some(path_str) = val {
            for dir in path_str.trim().split(';') {
                let expanded = expand_env_vars(dir.trim());
                if !expanded.is_empty() && !paths.iter().any(|p| p.eq_ignore_ascii_case(&expanded)) {
                    paths.push(expanded);
                }
            }
        }
    }
}

/// Build the full system + user PATH by reading from the Windows registry.
fn get_full_path() -> &'static str {
    FULL_PATH.get_or_init(|| {
        let mut paths: Vec<String> = Vec::new();

        // Start with current process PATH
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
            // System PATH from registry
            if let Ok(output) = std::process::Command::new("reg")
                .args(&["query", r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment", "/v", "Path"])
                .creation_flags(0x08000000)
                .output()
            {
                if output.status.success() {
                    extract_reg_paths(&String::from_utf8_lossy(&output.stdout), &mut paths);
                }
            }

            // User PATH from registry
            if let Ok(output) = std::process::Command::new("reg")
                .args(&["query", r"HKCU\Environment", "/v", "Path"])
                .creation_flags(0x08000000)
                .output()
            {
                if output.status.success() {
                    extract_reg_paths(&String::from_utf8_lossy(&output.stdout), &mut paths);
                }
            }
        }

        paths.join(";")
    })
}

fn find_git() -> String {
    // 1. Search current PATH environment variable
    if let Ok(path_var) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for dir in path_var.split(separator) {
            let dir = dir.trim();
            if dir.is_empty() { continue; }
            let git_path = Path::new(dir).join(if cfg!(windows) { "git.exe" } else { "git" });
            if git_path.exists() {
                return git_path.to_string_lossy().to_string();
            }
        }
    }

    // 1b. Search the full system+user PATH (from registry, may have more entries)
    let full = get_full_path();
    for dir in full.split(';') {
        let dir = dir.trim();
        if dir.is_empty() { continue; }
        let git_path = Path::new(dir).join(if cfg!(windows) { "git.exe" } else { "git" });
        if git_path.exists() {
            return git_path.to_string_lossy().to_string();
        }
    }

    // 2. Common Windows install locations
    let candidates = [
        r"C:\Program Files\Git\cmd\git.exe",
        r"C:\Program Files\Git\bin\git.exe",
        r"C:\Program Files\Git\mingw64\bin\git.exe",
        r"C:\Program Files (x86)\Git\cmd\git.exe",
        r"C:\Program Files (x86)\Git\bin\git.exe",
    ];
    for p in &candidates {
        if Path::new(p).exists() {
            return p.to_string();
        }
    }

    // 3. Check user-specific locations via env vars
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

    // 4. Check PROGRAMFILES variants from env
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

    // 5. Try Windows registry for Git install path (HKLM)
    #[cfg(windows)]
    {
        for reg_path in &[
            r"HKLM\SOFTWARE\GitForWindows",
            r"HKCU\SOFTWARE\GitForWindows",
            r"HKLM\SOFTWARE\Wow6432Node\GitForWindows",
        ] {
            if let Ok(output) = std::process::Command::new("reg")
                .args(&["query", reg_path, "/v", "InstallPath"])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
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

        // 6. Read system PATH from registry with full env var expansion
        if let Ok(output) = std::process::Command::new("reg")
            .args(&["query", r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment", "/v", "Path"])
            .creation_flags(0x08000000)
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let val = line.split("REG_EXPAND_SZ").nth(1)
                        .or_else(|| line.split("REG_SZ").nth(1));
                    if let Some(path_str) = val {
                        for dir in path_str.trim().split(';') {
                            let expanded = expand_env_vars(dir.trim());
                            if expanded.is_empty() { continue; }
                            let p = Path::new(&expanded).join("git.exe");
                            if p.exists() {
                                return p.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }
        }

        // 7. Read user PATH from registry with full env var expansion
        if let Ok(output) = std::process::Command::new("reg")
            .args(&["query", r"HKCU\Environment", "/v", "Path"])
            .creation_flags(0x08000000)
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let val = line.split("REG_EXPAND_SZ").nth(1)
                        .or_else(|| line.split("REG_SZ").nth(1));
                    if let Some(path_str) = val {
                        for dir in path_str.trim().split(';') {
                            let expanded = expand_env_vars(dir.trim());
                            if expanded.is_empty() { continue; }
                            let p = Path::new(&expanded).join("git.exe");
                            if p.exists() {
                                return p.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    // 8. Try running "git" directly as last resort
    if let Ok(output) = std::process::Command::new("git")
        .arg("--version")
        .output()
    {
        if output.status.success() {
            return "git".to_string();
        }
    }

    // 9. Try where.exe with full PATH
    #[cfg(windows)]
    {
        if let Ok(output) = std::process::Command::new("where.exe")
            .arg("git")
            .creation_flags(0x08000000)
            .env("PATH", get_full_path())
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

        // 10. Try cmd /c where git with full PATH
        if let Ok(output) = std::process::Command::new("cmd")
            .args(&["/c", "where", "git"])
            .creation_flags(0x08000000)
            .env("PATH", get_full_path())
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

fn git_executable() -> &'static str {
    GIT_PATH.get_or_init(find_git)
}

/// Try executing git with given executable path, returning output or error
#[allow(unused_variables)]
fn try_git_exec(exe: &str, cwd: Option<&str>, args: &[&str], path_env: &str) -> Result<std::process::Output, std::io::Error> {
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
        cmd.env("PATH", path_env);
    }
    cmd.output()
}

/// Hardcoded common git locations as last resort
const COMMON_GIT_PATHS: &[&str] = &[
    r"C:\Program Files\Git\cmd\git.exe",
    r"C:\Program Files\Git\bin\git.exe",
    r"C:\Program Files\Git\mingw64\bin\git.exe",
    r"C:\Program Files (x86)\Git\cmd\git.exe",
    r"C:\Program Files (x86)\Git\bin\git.exe",
];

/// Central function to run git commands with multiple fallback strategies.
fn run_git_cmd(cwd: Option<&str>, args: &[&str]) -> Result<String, String> {
    let primary = git_executable();
    let full_path = get_full_path();

    // Strategy 1: Use the detected git executable
    if let Ok(output) = try_git_exec(primary, cwd, args, full_path) {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(format!("{}{}", stdout, stderr).trim().to_string());
        } else {
            // Git ran but returned an error — that's a real git error, return it
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let msg = if !stderr.is_empty() { stderr } else { stdout };
            if !msg.is_empty() {
                return Err(msg);
            }
        }
    }

    // Strategy 2: Search full PATH for git.exe directly
    for dir in full_path.split(';') {
        let dir = dir.trim();
        if dir.is_empty() { continue; }
        let candidate = Path::new(dir).join("git.exe");
        if candidate.exists() {
            let exe_str = candidate.to_string_lossy().to_string();
            if exe_str != primary {
                if let Ok(output) = try_git_exec(&exe_str, cwd, args, full_path) {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Ok(format!("{}{}", stdout, stderr).trim().to_string());
                    } else {
                        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
                    }
                }
            }
        }
    }

    // Strategy 3: Hardcoded common git locations
    for fallback in COMMON_GIT_PATHS {
        if Path::new(fallback).exists() {
            if let Ok(output) = try_git_exec(fallback, cwd, args, full_path) {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Ok(format!("{}{}", stdout, stderr).trim().to_string());
                } else {
                    return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
                }
            }
        }
    }

    // Strategy 4: Use cmd.exe /c git (cmd.exe has its own PATH resolution)
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
        cmd.env("PATH", full_path);
        cmd.creation_flags(0x08000000);
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Ok(format!("{}{}", stdout, stderr).trim().to_string());
            } else {
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
            }
        }
    }

    Err(format!("Git not found. Detected path: '{}'. Searched {} PATH dirs and {} common locations.",
        primary, full_path.split(';').count(), COMMON_GIT_PATHS.len()))
}

pub struct GitService;

impl GitService {
    fn open(path: &str) -> Result<Repository, String> {
        Repository::open(path).map_err(|e| e.message().to_string())
    }

    pub fn get_git_path() -> String {
        let exe = git_executable();
        let full = get_full_path();
        let path_count = full.split(';').filter(|s| !s.trim().is_empty()).count();
        format!("{} (PATH has {} dirs)", exe, path_count)
    }

    pub fn repo_info(path: &str) -> Result<RepoInfo, String> {
        let repo = Self::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = head.shorthand().unwrap_or("HEAD").to_string();
        let head_sha = head.target().map(|oid| oid.to_string());
        let statuses = repo.statuses(None).map_err(|e| e.message().to_string())?;

        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());

        Ok(RepoInfo {
            path: path.to_string(),
            name,
            current_branch: branch_name,
            is_clean: statuses.is_empty(),
            head_sha,
        })
    }

    pub fn commits(path: &str, max_count: usize) -> Result<Vec<CommitInfo>, String> {
        let repo = Self::open(path)?;
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

        let mut ref_map: HashMap<String, Vec<String>> = HashMap::new();
        if let Ok(refs) = repo.references() {
            for reference in refs.flatten() {
                if let (Some(target), Some(name)) = (reference.target(), reference.shorthand()) {
                    ref_map
                        .entry(target.to_string())
                        .or_default()
                        .push(name.to_string());
                }
            }
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

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
                time_ago: Self::time_ago(now, timestamp),
                parent_shas,
                refs,
            });
        }

        Ok(result)
    }

    pub fn branches(path: &str) -> Result<Vec<BranchInfo>, String> {
        let repo = Self::open(path)?;
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

                let (ahead, behind) = Self::ahead_behind(&repo, &branch, &upstream);

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
        let repo = Self::open(path)?;
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
                    status: Self::index_status_label(s),
                    staged: true,
                });
            }

            if s.is_wt_new()
                || s.is_wt_modified()
                || s.is_wt_deleted()
                || s.is_wt_renamed()
                || s.is_wt_typechange()
            {
                files.push(FileStatusInfo {
                    path: file_path,
                    status: Self::wt_status_label(s),
                    staged: false,
                });
            }
        }

        Ok(files)
    }

    pub fn stage_file(path: &str, file_path: &str) -> Result<(), String> {
        let repo = Self::open(path)?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let pathspec = [Path::new(file_path)];
        index
            .add_all(pathspec.iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| e.message().to_string())?;
        index.write().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn unstage_file(path: &str, file_path: &str) -> Result<(), String> {
        let repo = Self::open(path)?;
        let file = Path::new(file_path);

        if let Ok(head_obj) = repo.revparse_single("HEAD") {
            repo
                .reset_default(Some(&head_obj), [file])
                .map_err(|e| e.message().to_string())?;
            return Ok(());
        }

        // Initial commit (no HEAD): remove path from index directly.
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let _ = index.remove_path(file);
        index.write().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn create_commit(path: &str, message: &str) -> Result<String, String> {
        let repo = Self::open(path)?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let tree_oid = index.write_tree().map_err(|e| e.message().to_string())?;
        let tree = repo.find_tree(tree_oid).map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let parent = head
            .peel_to_commit()
            .map_err(|e| e.message().to_string())?;

        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
            .map_err(|e| e.message().to_string())?;

        Ok(oid.to_string())
    }

    pub fn checkout_branch(path: &str, branch_name: &str) -> Result<(), String> {
        let repo = Self::open(path)?;
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
        Self::fetch_all(path, token)?;
        let repo = Self::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = head.shorthand().unwrap_or("main").to_string();
        let remote_ref = format!("refs/remotes/origin/{}", branch_name);
        let remote_oid = repo.refname_to_id(&remote_ref)
            .map_err(|_| format!("No remote tracking branch for '{}'", branch_name))?;
        let remote_commit = repo.find_annotated_commit(remote_oid)
            .map_err(|e| e.message().to_string())?;
        let (analysis, _) = repo.merge_analysis(&[&remote_commit])
            .map_err(|e| e.message().to_string())?;
        if analysis.is_up_to_date() {
            return Ok("Already up to date.".to_string());
        }
        if analysis.is_fast_forward() {
            let mut reference = repo.find_reference(&format!("refs/heads/{}", branch_name))
                .map_err(|e| e.message().to_string())?;
            reference.set_target(remote_oid, "pull: fast-forward")
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
            let head_commit = repo.head().map_err(|e| e.message().to_string())?
                .peel_to_commit().map_err(|e| e.message().to_string())?;
            let remote_commit_obj = repo.find_commit(remote_oid)
                .map_err(|e| e.message().to_string())?;
            let sig = repo.signature().map_err(|e| e.message().to_string())?;
            let msg = format!("Merge branch '{}' of origin into {}", branch_name, branch_name);
            repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&head_commit, &remote_commit_obj])
                .map_err(|e| e.message().to_string())?;
            repo.cleanup_state().map_err(|e| e.message().to_string())?;
            return Ok("Merge complete.".to_string());
        }
        Err("Pull failed: unexpected merge state.".to_string())
    }

    pub fn push(path: &str, token: Option<&str>) -> Result<String, String> {
        let repo = Self::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = head.shorthand().unwrap_or("main").to_string();
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

        let mut remote = repo.find_remote("origin")
            .map_err(|e| format!("No remote 'origin': {}", e.message()))?;

        let mut callbacks = git2::RemoteCallbacks::new();
        if let Some(t) = token {
            let tok = t.to_string();
            callbacks.credentials(move |_url, _username, _allowed| {
                git2::Cred::userpass_plaintext("x-access-token", &tok)
            });
        }
        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        remote.push(&[&refspec], Some(&mut push_opts))
            .map_err(|e| e.message().to_string())?;
        Ok("Push complete.".to_string())
    }

    pub fn fetch_all(path: &str, token: Option<&str>) -> Result<String, String> {
        let repo = Self::open(path)?;
        let remotes = repo.remotes().map_err(|e| e.message().to_string())?;
        let remote_names: Vec<String> = remotes.iter()
            .filter_map(|n| n.map(|s| s.to_string()))
            .collect();

        for remote_name in &remote_names {
            let mut remote = repo.find_remote(remote_name)
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

            remote.fetch::<&str>(&[], Some(&mut fetch_opts), None)
                .map_err(|e| format!("Fetch '{}' failed: {}", remote_name, e.message()))?;
        }
        Ok(format!("Fetched {} remote(s).", remote_names.len()))
    }

    #[allow(dead_code)]
    fn git_cli_with_token(path: &str, args: &[&str], token: &str) -> Result<String, String> {
        let remote_url = Self::git_cli(path, &["remote", "get-url", "origin"]).ok();
        if let Some(ref raw_url) = remote_url {
            let url = raw_url.trim();
            if url.starts_with("https://") {
                let host_and_path = if url.contains('@') {
                    let after_proto = &url[8..];
                    if let Some(at_pos) = after_proto.find('@') {
                        &after_proto[at_pos + 1..]
                    } else {
                        after_proto
                    }
                } else {
                    &url[8..]
                };
                let authed_url = format!("https://x-access-token:{}@{}", token, host_and_path);

                if !args.is_empty() {
                    let mut new_args: Vec<String> = vec![args[0].to_string(), authed_url];
                    for a in &args[1..] {
                        new_args.push(a.to_string());
                    }
                    let str_refs: Vec<&str> = new_args.iter().map(|s| s.as_str()).collect();
                    return run_git_cmd(Some(path), &str_refs);
                }
            }
        }
        Self::git_cli(path, args)
    }

    fn git_cli(path: &str, args: &[&str]) -> Result<String, String> {
        run_git_cmd(Some(path), args)
    }

    fn ahead_behind(
        repo: &Repository,
        branch: &git2::Branch,
        upstream: &Option<String>,
    ) -> (usize, usize) {
        let Some(local_oid) = branch.get().target() else {
            return (0, 0);
        };
        let Some(ref up_name) = upstream else {
            return (0, 0);
        };
        let Ok(remote_ref) = repo.find_reference(&format!("refs/remotes/{}", up_name)) else {
            return (0, 0);
        };
        let Some(remote_oid) = remote_ref.target() else {
            return (0, 0);
        };
        repo.graph_ahead_behind(local_oid, remote_oid)
            .unwrap_or((0, 0))
    }

    fn time_ago(now: i64, timestamp: i64) -> String {
        let diff = now - timestamp;
        if diff < 60 {
            "just now".to_string()
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else if diff < 86400 {
            format!("{}h ago", diff / 3600)
        } else if diff < 604800 {
            format!("{}d ago", diff / 86400)
        } else if diff < 2592000 {
            format!("{}w ago", diff / 604800)
        } else {
            format!("{}mo ago", diff / 2592000)
        }
    }

    fn index_status_label(s: git2::Status) -> String {
        if s.is_index_new() {
            "new"
        } else if s.is_index_modified() {
            "modified"
        } else if s.is_index_deleted() {
            "deleted"
        } else if s.is_index_renamed() {
            "renamed"
        } else {
            "typechange"
        }
        .to_string()
    }

    fn wt_status_label(s: git2::Status) -> String {
        if s.is_wt_new() {
            "new"
        } else if s.is_wt_modified() {
            "modified"
        } else if s.is_wt_deleted() {
            "deleted"
        } else if s.is_wt_renamed() {
            "renamed"
        } else {
            "typechange"
        }
        .to_string()
    }

    pub fn cherry_pick(path: &str, sha: &str) -> Result<String, String> {
        let repo = Self::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        repo.cherrypick(&commit, None).map_err(|e| e.message().to_string())?;
        // Auto-commit the cherry-pick
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
        let repo = Self::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        repo.revert(&commit, None).map_err(|e| e.message().to_string())?;
        // Auto-commit the revert
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        if index.has_conflicts() {
            return Err("Revert has conflicts. Resolve them manually.".to_string());
        }
        let tree_oid = index.write_tree().map_err(|e| e.message().to_string())?;
        let tree = repo.find_tree(tree_oid).map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let parent = head.peel_to_commit().map_err(|e| e.message().to_string())?;
        let msg = format!("Revert \"{}\"", commit.message().unwrap_or("").lines().next().unwrap_or(""));
        repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&parent])
            .map_err(|e| e.message().to_string())?;
        repo.cleanup_state().map_err(|e| e.message().to_string())?;
        Ok("Revert complete.".to_string())
    }

    pub fn reset_to_commit(path: &str, sha: &str, mode: &str) -> Result<String, String> {
        let repo = Self::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?.into_object();
        let reset_type = match mode {
            "soft" => git2::ResetType::Soft,
            "hard" => git2::ResetType::Hard,
            _ => git2::ResetType::Mixed,
        };
        repo.reset(&commit, reset_type, None).map_err(|e| e.message().to_string())?;
        Ok(format!("Reset ({}) to {}.", mode, &sha[..7.min(sha.len())]))
    }

    pub fn checkout_commit(path: &str, sha: &str) -> Result<String, String> {
        let repo = Self::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        let obj = commit.as_object();
        repo.checkout_tree(obj, Some(git2::build::CheckoutBuilder::default().safe()))
            .map_err(|e| e.message().to_string())?;
        repo.set_head_detached(oid).map_err(|e| e.message().to_string())?;
        Ok(format!("Checked out {}.", &sha[..7.min(sha.len())]))
    }

    pub fn create_tag_at(path: &str, name: &str, sha: &str) -> Result<String, String> {
        let repo = Self::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let obj = repo.find_object(oid, None).map_err(|e| e.message().to_string())?;
        repo.tag_lightweight(name, &obj, false).map_err(|e| e.message().to_string())?;
        Ok(format!("Tag '{}' created.", name))
    }

    pub fn clone_repo(url: &str, path: &str, shallow: bool, token: Option<&str>) -> Result<String, String> {
        // Determine final clone directory: path/repoName (matching git clone behavior)
        let repo_name = url.split('/').last().unwrap_or("repo")
            .trim_end_matches(".git");
        let dest = Path::new(path).join(repo_name);

        let clone_url = if let Some(t) = token {
            if url.starts_with("https://") {
                let host_part = if url.contains('@') {
                    let after_proto = &url[8..];
                    if let Some(at_pos) = after_proto.find('@') {
                        &after_proto[at_pos + 1..]
                    } else {
                        after_proto
                    }
                } else {
                    &url[8..]
                };
                format!("https://x-access-token:{}@{}", t, host_part)
            } else {
                url.to_string()
            }
        } else {
            url.to_string()
        };

        let mut callbacks = git2::RemoteCallbacks::new();
        if let Some(t) = token {
            let tok = t.to_string();
            callbacks.credentials(move |_url, _username, _allowed| {
                git2::Cred::userpass_plaintext("x-access-token", &tok)
            });
        }
        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);
        if shallow {
            fetch_opts.depth(1);
        }

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_opts);

        builder.clone(&clone_url, &dest)
            .map_err(|e| e.message().to_string())?;
        Ok(dest.to_string_lossy().to_string())
    }

    pub fn init_repo(path: &str, branch_name: Option<&str>) -> Result<String, String> {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let repo = Repository::init(path).map_err(|e| e.message().to_string())?;
        let branch = branch_name.unwrap_or("main");
        // Set initial branch name via HEAD reference
        repo.set_head(&format!("refs/heads/{}", branch))
            .map_err(|e| e.message().to_string())?;
        Ok(format!("Initialized repository with branch '{}'.", branch))
    }

    pub fn search_commits(
        path: &str,
        query: &str,
        max_count: usize,
    ) -> Result<Vec<CommitInfo>, String> {
        let all = Self::commits(path, max_count)?;
        let q = query.to_lowercase();
        let filtered = all
            .into_iter()
            .filter(|c| {
                c.message.to_lowercase().contains(&q)
                    || c.author_name.to_lowercase().contains(&q)
                    || c.author_email.to_lowercase().contains(&q)
                    || c.sha.starts_with(&q)
                    || c.short_sha.starts_with(&q)
            })
            .collect();
        Ok(filtered)
    }

    #[allow(dead_code)]
    fn git_cli_global(args: &[&str]) -> Result<String, String> {
        run_git_cmd(None, args)
    }

    pub fn commit_files(path: &str, sha: &str) -> Result<Vec<CommitFileInfo>, String> {
        let repo = Self::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        let tree = commit.tree().map_err(|e| e.message().to_string())?;

        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0).map_err(|e| e.message().to_string())?
                .tree().map_err(|e| e.message().to_string())?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .map_err(|e| e.message().to_string())?;

        let n = diff.deltas().len();
        let mut files = Vec::with_capacity(n);
        for idx in 0..n {
            let delta = diff.get_delta(idx).unwrap();
            let file_path = delta.new_file().path()
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
        let repo = Self::open(path)?;
        let commit = if let Some(sp) = start_point {
            let obj = repo
                .revparse_single(sp)
                .map_err(|e| e.message().to_string())?;
            obj.peel_to_commit().map_err(|e| e.message().to_string())?
        } else {
            let head = repo.head().map_err(|e| e.message().to_string())?;
            head.peel_to_commit()
                .map_err(|e| e.message().to_string())?
        };
        repo.branch(name, &commit, false)
            .map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn delete_branch(path: &str, name: &str) -> Result<(), String> {
        let repo = Self::open(path)?;
        let mut branch = repo
            .find_branch(name, BranchType::Local)
            .map_err(|e| e.message().to_string())?;
        branch.delete().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn stash_list(path: &str) -> Result<Vec<StashInfo>, String> {
        let mut repo = Self::open(path)?;
        let mut raw: Vec<(usize, String, String)> = Vec::new();

        if repo
            .stash_foreach(|index, name, oid| {
                raw.push((index, name.to_string(), oid.to_string()));
                true
            })
            .is_err()
        {
            return Ok(Vec::new());
        }

        let mut stashes = Vec::with_capacity(raw.len());
        for (index, stash_name, oid_str) in raw {
            let parent_sha = git2::Oid::from_str(&oid_str)
                .ok()
                .and_then(|oid| repo.find_commit(oid).ok())
                .and_then(|c| c.parent_id(0).ok())
                .map(|oid| oid.to_string())
                .unwrap_or_default();

            // git2 stash name format is usually: "WIP on main: message" or "On main: message"
            let (branch, message) = if let Some(after) = stash_name
                .strip_prefix("WIP on ")
                .or_else(|| stash_name.strip_prefix("On "))
            {
                if let Some(colon_pos) = after.find(": ") {
                    (after[..colon_pos].to_string(), after[colon_pos + 2..].to_string())
                } else {
                    (String::new(), stash_name.clone())
                }
            } else {
                (String::new(), stash_name.clone())
            };

            stashes.push(StashInfo {
                index,
                message,
                branch,
                timestamp: String::new(),
                parent_sha,
            });
        }

        Ok(stashes)
    }

    pub fn stash_push(path: &str, message: Option<&str>) -> Result<String, String> {
        let mut repo = Self::open(path)?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        let msg = message.unwrap_or("WIP");
        let oid = repo
            .stash_save(&sig, msg, Some(StashFlags::INCLUDE_UNTRACKED))
            .map_err(|e| e.message().to_string())?;
        Ok(format!("Saved stash {}", oid))
    }

    pub fn stash_pop(path: &str, index: usize) -> Result<String, String> {
        let mut repo = Self::open(path)?;
        let mut opts = StashApplyOptions::new();
        repo
            .stash_pop(index, Some(&mut opts))
            .map_err(|e| e.message().to_string())?;
        Ok(format!("Dropped and applied stash@{{{}}}", index))
    }

    pub fn stash_apply(path: &str, index: usize) -> Result<String, String> {
        let mut repo = Self::open(path)?;
        let mut opts = StashApplyOptions::new();
        repo
            .stash_apply(index, Some(&mut opts))
            .map_err(|e| e.message().to_string())?;
        Ok(format!("Applied stash@{{{}}}", index))
    }

    pub fn stash_drop(path: &str, index: usize) -> Result<String, String> {
        let mut repo = Self::open(path)?;
        repo.stash_drop(index).map_err(|e| e.message().to_string())?;
        Ok(format!("Dropped stash@{{{}}}", index))
    }

    pub fn stash_files(path: &str, index: usize) -> Result<Vec<CommitFileInfo>, String> {
        let mut repo = Self::open(path)?;
        
        // Find the stash commit by iterating through stashes
        let mut stash_oid: Option<git2::Oid> = None;
        let target_index = index;
        
        repo.stash_foreach(|idx, _name, oid| {
            if idx == target_index {
                stash_oid = Some(*oid);
                false // stop iterating
            } else {
                true // continue
            }
        }).map_err(|e| e.message().to_string())?;
        
        let oid = stash_oid.ok_or_else(|| format!("Stash at index {} not found", index))?;
        let stash_commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        
        // Stash commit structure:
        // - stash commit has the working tree changes
        // - parent[0] is the original commit (HEAD when stash was created)
        // We diff the stash against its first parent to see what was stashed
        let stash_tree = stash_commit.tree().map_err(|e| e.message().to_string())?;
        
        let parent_tree = if stash_commit.parent_count() > 0 {
            Some(stash_commit.parent(0).map_err(|e| e.message().to_string())?
                .tree().map_err(|e| e.message().to_string())?)
        } else {
            None
        };
        
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&stash_tree), None)
            .map_err(|e| e.message().to_string())?;
        
        let n = diff.deltas().len();
        let mut files = Vec::with_capacity(n);
        for idx in 0..n {
            let delta = diff.get_delta(idx).unwrap();
            let file_path = delta.new_file().path()
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

    pub fn tags(path: &str) -> Result<Vec<TagInfo>, String> {
        let repo = Self::open(path)?;
        let mut result = Vec::new();
        let tag_names = repo.tag_names(None).map_err(|e| e.message().to_string())?;
        for name in tag_names.iter().flatten() {
            let refname = format!("refs/tags/{}", name);
            if let Ok(reference) = repo.find_reference(&refname) {
                let object = match repo.revparse_single(&refname) {
                    Ok(o) => o,
                    Err(_) => continue,
                };

                // For annotated tags, peel to commit so graph matching works.
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
        Self::git_cli(path, &["tag", "-d", name])
    }

    pub fn run_git_command(path: &str, args: &[&str]) -> Result<String, String> {
        Self::git_cli(path, args)
    }

    pub fn discard_file(path: &str, file_path: &str) -> Result<(), String> {
        let repo = Self::open(path)?;
        // Check if the file is tracked
        let statuses = repo.statuses(None).map_err(|e| e.message().to_string())?;
        let is_untracked = statuses.iter().any(|s| {
            s.path() == Some(file_path) && s.status().contains(git2::Status::WT_NEW)
        });
        if is_untracked {
            // Untracked file: just delete it
            let full = Path::new(path).join(file_path);
            std::fs::remove_file(&full).map_err(|e| e.to_string())?;
        } else {
            // Tracked file: checkout from HEAD
            repo.checkout_head(Some(
                git2::build::CheckoutBuilder::default()
                    .path(file_path)
                    .force()
            )).map_err(|e| e.message().to_string())?;
        }
        Ok(())
    }

    pub fn rename_branch(path: &str, old_name: &str, new_name: &str) -> Result<String, String> {
        Self::git_cli(path, &["branch", "-m", old_name, new_name])
    }

    pub fn delete_remote_branch(path: &str, remote: &str, branch: &str) -> Result<String, String> {
        Self::git_cli(path, &["push", remote, "--delete", branch])
    }

    pub fn set_upstream(path: &str, branch: &str, remote_branch: &str) -> Result<String, String> {
        Self::git_cli(path, &["branch", "--set-upstream-to", remote_branch, branch])
    }

    pub fn edit_commit_message(path: &str, sha: &str, new_message: &str) -> Result<String, String> {
        // Only works for HEAD commit
        let repo = Self::open(path)?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let head_commit = head.peel_to_commit().map_err(|e| e.message().to_string())?;
        if head_commit.id().to_string() == sha {
            Self::git_cli(path, &["commit", "--amend", "-m", new_message])
        } else {
            Err("Can only edit the message of the HEAD commit.".to_string())
        }
    }

    pub fn create_annotated_tag(path: &str, name: &str, sha: &str, message: &str) -> Result<String, String> {
        Self::git_cli(path, &["tag", "-a", name, sha, "-m", message])
    }

    pub fn reset_branch_to_remote(path: &str, branch: &str) -> Result<String, String> {
        let repo = Self::open(path)?;
        
        // First checkout the branch using libgit2
        Self::checkout_branch(path, branch)?;
        
        // Find the remote ref for origin/<branch>
        let remote_ref_name = format!("refs/remotes/origin/{}", branch);
        let remote_ref = repo.find_reference(&remote_ref_name)
            .map_err(|e| format!("Cannot find remote branch origin/{}: {}", branch, e.message()))?;
        let remote_oid = remote_ref.target()
            .ok_or_else(|| format!("Remote branch origin/{} has no target", branch))?;
        let remote_commit = repo.find_commit(remote_oid)
            .map_err(|e| e.message().to_string())?;
        
        // Reset hard to the remote commit
        repo.reset(remote_commit.as_object(), git2::ResetType::Hard, None)
            .map_err(|e| e.message().to_string())?;
        
        Ok(format!("Reset {} to origin/{}", branch, branch))
    }

    pub fn search_github_repos(token: &str, query: &str) -> Result<Vec<GithubRepo>, String> {
        let url = if query.is_empty() {
            "https://api.github.com/user/repos?per_page=50&sort=updated&affiliation=owner,collaborator,organization_member".to_string()
        } else {
            format!("https://api.github.com/search/repositories?q={}&per_page=30", urlencoded(query))
        };
        let resp = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "GitSwamp")
            .call()
            .map_err(|e| format!("GitHub API error: {}", e))?;
        let body: serde_json::Value = resp.into_json()
            .map_err(|e| format!("JSON parse error: {}", e))?;
        let items = if query.is_empty() {
            body.as_array().cloned().unwrap_or_default()
        } else {
            body["items"].as_array().cloned().unwrap_or_default()
        };
        let repos = items.iter().filter_map(|item| {
            Some(GithubRepo {
                full_name: item["full_name"].as_str()?.to_string(),
                clone_url: item["clone_url"].as_str()?.to_string(),
                description: item["description"].as_str().unwrap_or("").to_string(),
                is_private: item["private"].as_bool().unwrap_or(false),
                stars: item["stargazers_count"].as_u64().unwrap_or(0) as u32,
            })
        }).collect();
        Ok(repos)
    }

    /// Get diff for a working directory file (unstaged or staged)
    pub fn get_working_diff(path: &str, file_path: &str, staged: bool) -> Result<FileDiff, String> {
        let repo = Self::open(path)?;
        
        let diff = if staged {
            // Staged: diff HEAD vs index
            let head_tree = repo.head()
                .and_then(|h| h.peel_to_tree())
                .ok();
            repo.diff_tree_to_index(head_tree.as_ref(), None, None)
                .map_err(|e| e.message().to_string())?
        } else {
            // Unstaged: diff index vs workdir
            repo.diff_index_to_workdir(None, None)
                .map_err(|e| e.message().to_string())?
        };

        Self::extract_file_diff(&diff, file_path)
    }

    /// Get diff for a file in a specific commit
    pub fn get_commit_diff(path: &str, sha: &str, file_path: &str) -> Result<FileDiff, String> {
        let repo = Self::open(path)?;
        let oid = git2::Oid::from_str(sha).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        let tree = commit.tree().map_err(|e| e.message().to_string())?;
        
        let parent_tree = commit.parent(0)
            .and_then(|p| p.tree())
            .ok();
        
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .map_err(|e| e.message().to_string())?;

        Self::extract_file_diff(&diff, file_path)
    }

    /// Get full file content from working directory or a commit
    pub fn get_file_content(path: &str, file_path: &str, sha: Option<&str>) -> Result<String, String> {
        let repo = Self::open(path)?;
        
        if let Some(commit_sha) = sha {
            // Get file from a specific commit
            let oid = git2::Oid::from_str(commit_sha).map_err(|e| e.message().to_string())?;
            let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
            let tree = commit.tree().map_err(|e| e.message().to_string())?;
            let entry = tree.get_path(Path::new(file_path))
                .map_err(|_| format!("File '{}' not found in commit", file_path))?;
            let blob = repo.find_blob(entry.id())
                .map_err(|e| e.message().to_string())?;
            if blob.is_binary() {
                return Err("Binary file".to_string());
            }
            String::from_utf8(blob.content().to_vec())
                .map_err(|_| "File is not valid UTF-8".to_string())
        } else {
            // Get file from working directory
            let full_path = Path::new(path).join(file_path);
            std::fs::read_to_string(&full_path)
                .map_err(|e| e.to_string())
        }
    }

    fn extract_file_diff(diff: &git2::Diff, target_path: &str) -> Result<FileDiff, String> {
        let mut result: Option<FileDiff> = None;
        let num_deltas = diff.deltas().len();

        for idx in 0..num_deltas {
            let delta = diff.get_delta(idx).unwrap();
            let new_file = delta.new_file();
            let old_file = delta.old_file();
            
            let file_path = new_file.path()
                .or(old_file.path())
                .and_then(|p| p.to_str())
                .unwrap_or("");
            
            // Skip if not our target file
            if file_path != target_path && old_file.path().and_then(|p| p.to_str()) != Some(target_path) {
                continue;
            }

            let status = match delta.status() {
                git2::Delta::Added => "added",
                git2::Delta::Deleted => "deleted",
                git2::Delta::Modified => "modified",
                git2::Delta::Renamed => "renamed",
                git2::Delta::Copied => "copied",
                _ => "modified",
            };

            let old_path = if delta.status() == git2::Delta::Renamed {
                old_file.path().and_then(|p| p.to_str()).map(|s| s.to_string())
            } else {
                None
            };

            let is_binary = new_file.is_binary() || old_file.is_binary();

            // Get patch with hunks
            let mut hunks: Vec<DiffHunk> = Vec::new();
            
            if let Ok(Some(patch)) = git2::Patch::from_diff(diff, idx) {
                let num_hunks = patch.num_hunks();
                for hunk_idx in 0..num_hunks {
                    if let Ok((hunk, num_lines)) = patch.hunk(hunk_idx) {
                        let header = String::from_utf8_lossy(hunk.header()).to_string();
                        let mut lines: Vec<DiffLine> = Vec::new();

                        for line_idx in 0..num_lines {
                            if let Ok(line) = patch.line_in_hunk(hunk_idx, line_idx) {
                                let line_type = match line.origin() {
                                    '+' => "addition",
                                    '-' => "deletion",
                                    ' ' => "context",
                                    _ => "context",
                                };
                                lines.push(DiffLine {
                                    line_type: line_type.to_string(),
                                    old_line_no: line.old_lineno(),
                                    new_line_no: line.new_lineno(),
                                    content: String::from_utf8_lossy(line.content()).to_string(),
                                });
                            }
                        }

                        hunks.push(DiffHunk {
                            old_start: hunk.old_start(),
                            old_lines: hunk.old_lines(),
                            new_start: hunk.new_start(),
                            new_lines: hunk.new_lines(),
                            header,
                            lines,
                        });
                    }
                }
            }

            result = Some(FileDiff {
                path: file_path.to_string(),
                old_path,
                status: status.to_string(),
                hunks,
                is_binary,
            });
            break;
        }

        result.ok_or_else(|| format!("File '{}' not found in diff", target_path))
    }

    /// Save content to a file in the working directory
    pub fn save_file_content(path: &str, file_path: &str, content: &str) -> Result<(), String> {
        let full_path = Path::new(path).join(file_path);
        std::fs::write(&full_path, content).map_err(|e| e.to_string())
    }

    /// Revert a specific hunk in a file (for working directory changes)
    pub fn revert_hunk(path: &str, file_path: &str, hunk_index: usize, staged: bool) -> Result<(), String> {
        // Get the diff to find the hunk
        let diff_info = if staged {
            Self::get_working_diff(path, file_path, true)?
        } else {
            Self::get_working_diff(path, file_path, false)?
        };

        if hunk_index >= diff_info.hunks.len() {
            return Err(format!("Hunk index {} out of range", hunk_index));
        }

        let hunk = &diff_info.hunks[hunk_index];
        
        // Read the current file content
        let full_path = Path::new(path).join(file_path);
        let current_content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let lines: Vec<&str> = current_content.lines().collect();
        let mut new_lines: Vec<String> = Vec::new();
        
        // Build a set of line numbers that are additions (to remove)
        // and collect the deletions (to restore)
        let mut additions_to_remove: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut deletions_to_restore: Vec<(u32, String)> = Vec::new();
        
        for line in &hunk.lines {
            if line.line_type == "addition" {
                if let Some(ln) = line.new_line_no {
                    additions_to_remove.insert(ln);
                }
            } else if line.line_type == "deletion" {
                if let Some(ln) = line.old_line_no {
                    deletions_to_restore.push((ln, line.content.clone()));
                }
            }
        }
        
        // Sort deletions by line number
        deletions_to_restore.sort_by_key(|(ln, _)| *ln);
        
        // Reconstruct the file
        let mut current_line_no: u32 = 1;
        let mut deletion_idx = 0;
        
        for line in &lines {
            // Check if we need to insert deletions before this line
            while deletion_idx < deletions_to_restore.len() {
                let (del_ln, _) = &deletions_to_restore[deletion_idx];
                if *del_ln < current_line_no {
                    let content = deletions_to_restore[deletion_idx].1.trim_end_matches('\n');
                    new_lines.push(content.to_string());
                    deletion_idx += 1;
                } else {
                    break;
                }
            }
            
            // Skip additions
            if !additions_to_remove.contains(&current_line_no) {
                new_lines.push(line.to_string());
            }
            current_line_no += 1;
        }
        
        // Add any remaining deletions at the end
        while deletion_idx < deletions_to_restore.len() {
            let content = deletions_to_restore[deletion_idx].1.trim_end_matches('\n');
            new_lines.push(content.to_string());
            deletion_idx += 1;
        }
        
        // Write the reverted content
        let new_content = new_lines.join("\n") + if current_content.ends_with('\n') { "\n" } else { "" };
        std::fs::write(&full_path, new_content).map_err(|e| format!("Failed to write file: {}", e))?;
        
        Ok(())
    }
}

fn urlencoded(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", b));
            }
        }
    }
    result
}

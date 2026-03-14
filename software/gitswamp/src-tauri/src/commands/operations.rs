use crate::services::git_service::GitService;

#[tauri::command]
pub fn pull(path: String, token: Option<String>) -> Result<String, String> {
    GitService::pull(&path, token.as_deref())
}

#[tauri::command]
pub fn push(path: String, token: Option<String>) -> Result<String, String> {
    GitService::push(&path, token.as_deref())
}

#[tauri::command]
pub fn fetch_all(path: String, token: Option<String>) -> Result<String, String> {
    GitService::fetch_all(&path, token.as_deref())
}

#[tauri::command]
pub fn run_git_command(path: String, args: Vec<String>) -> Result<String, String> {
    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    GitService::run_git_command(&path, &str_args)
}

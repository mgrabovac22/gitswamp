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

#[tauri::command]
pub fn get_git_path() -> String {
    GitService::get_git_path()
}

#[tauri::command]
pub fn cherry_pick(path: String, sha: String) -> Result<String, String> {
    GitService::cherry_pick(&path, &sha)
}

#[tauri::command]
pub fn revert_commit(path: String, sha: String) -> Result<String, String> {
    GitService::revert_commit(&path, &sha)
}

#[tauri::command]
pub fn reset_to_commit(path: String, sha: String, mode: String) -> Result<String, String> {
    GitService::reset_to_commit(&path, &sha, &mode)
}

#[tauri::command]
pub fn checkout_commit(path: String, sha: String) -> Result<String, String> {
    GitService::checkout_commit(&path, &sha)
}

#[tauri::command]
pub fn create_tag_at(path: String, name: String, sha: String) -> Result<String, String> {
    GitService::create_tag_at(&path, &name, &sha)
}

#[tauri::command]
pub fn search_github_repos(token: String, query: String) -> Result<Vec<crate::models::GithubRepo>, String> {
    crate::services::git_service::GitService::search_github_repos(&token, &query)
}

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

#[tauri::command]
pub fn rename_branch(path: String, old_name: String, new_name: String) -> Result<String, String> {
    GitService::rename_branch(&path, &old_name, &new_name)
}

#[tauri::command]
pub fn delete_remote_branch(path: String, remote: String, branch: String) -> Result<String, String> {
    GitService::delete_remote_branch(&path, &remote, &branch)
}

#[tauri::command]
pub fn set_upstream(path: String, branch: String, remote_branch: String) -> Result<String, String> {
    GitService::set_upstream(&path, &branch, &remote_branch)
}

#[tauri::command]
pub fn edit_commit_message(path: String, sha: String, new_message: String) -> Result<String, String> {
    GitService::edit_commit_message(&path, &sha, &new_message)
}

#[tauri::command]
pub fn create_annotated_tag(path: String, name: String, sha: String, message: String) -> Result<String, String> {
    GitService::create_annotated_tag(&path, &name, &sha, &message)
}

#[tauri::command]
pub fn reset_branch_to_remote(path: String, branch: String) -> Result<String, String> {
    GitService::reset_branch_to_remote(&path, &branch)
}

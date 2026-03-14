use crate::services::git_service::GitService;

#[tauri::command]
pub fn clone_repo(url: String, path: String, shallow: bool) -> Result<String, String> {
    GitService::clone_repo(&url, &path, shallow)
}

#[tauri::command]
pub fn init_repo(path: String, branch_name: Option<String>) -> Result<String, String> {
    GitService::init_repo(&path, branch_name.as_deref())
}

#[tauri::command]
pub fn search_commits(
    path: String,
    query: String,
    max_count: Option<usize>,
) -> Result<Vec<crate::models::CommitInfo>, String> {
    GitService::search_commits(&path, &query, max_count.unwrap_or(200))
}

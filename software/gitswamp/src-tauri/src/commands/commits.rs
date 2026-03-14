use crate::models::CommitInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub fn get_commits(path: String, max_count: Option<usize>) -> Result<Vec<CommitInfo>, String> {
    GitService::commits(&path, max_count.unwrap_or(200))
}

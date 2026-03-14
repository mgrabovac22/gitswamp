use crate::models::RepoInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub fn get_repo_info(path: String) -> Result<RepoInfo, String> {
    GitService::repo_info(&path)
}

use crate::models::RepoInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub async fn get_repo_info(path: String) -> Result<RepoInfo, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::repo_info(&path))
        .await
        .map_err(|e| e.to_string())?
}

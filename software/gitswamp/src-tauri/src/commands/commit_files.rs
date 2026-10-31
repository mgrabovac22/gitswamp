use crate::models::CommitFileInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub async fn get_commit_files(path: String, sha: String) -> Result<Vec<CommitFileInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::commit_files(&path, &sha))
        .await
        .map_err(|e| e.to_string())?
}

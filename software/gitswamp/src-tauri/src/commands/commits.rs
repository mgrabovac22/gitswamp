use crate::models::CommitInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub async fn get_commits(path: String, max_count: Option<usize>) -> Result<Vec<CommitInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::commits(&path, max_count.unwrap_or(200))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_author_deletion_stats(
    path: String,
    max_count: Option<usize>,
) -> Result<Vec<(String, usize, usize)>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::author_deletion_stats(&path, max_count.unwrap_or(60000))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_commit_tree_paths(path: String, sha: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::commit_tree_paths(&path, &sha))
        .await
        .map_err(|e| e.to_string())?
}

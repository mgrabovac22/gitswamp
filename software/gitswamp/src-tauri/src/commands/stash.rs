use crate::models::CommitFileInfo;
use crate::models::StashInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub async fn stash_list(path: String) -> Result<Vec<StashInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::stash_list(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn stash_push(path: String, message: Option<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::stash_push(&path, message.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn stash_pop(path: String, index: usize) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::stash_pop(&path, index))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn stash_apply(path: String, index: usize) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::stash_apply(&path, index))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn stash_drop(path: String, index: usize) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::stash_drop(&path, index))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn stash_files(path: String, index: usize) -> Result<Vec<CommitFileInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::stash_files(&path, index))
        .await
        .map_err(|e| e.to_string())?
}

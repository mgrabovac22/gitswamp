use crate::models::CommitFileInfo;
use crate::models::StashInfo;
use crate::services::git_service::GitService;
use crate::services::stash_service::{SelectionResult, StashService};

#[tauri::command]
pub async fn stash_list(path: String) -> Result<Vec<StashInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::stash_list(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn stash_push(
    path: String,
    message: Option<String>,
    include_untracked: Option<bool>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::stash_push(
            &path,
            message.as_deref(),
            include_untracked.unwrap_or(false),
        )
    })
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
pub async fn stash_files(
    path: String,
    index: usize,
    sha: Option<String>,
) -> Result<Vec<CommitFileInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::stash_files(&path, index, sha.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn restore_selected_stash(
    path: String,
    index: usize,
    sha: String,
    paths: Vec<String>,
    pop: bool,
) -> Result<SelectionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        StashService::restore_selected(&path, index, &sha, paths, pop)
    })
    .await
    .map_err(|e| e.to_string())?
}

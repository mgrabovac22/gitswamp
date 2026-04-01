use crate::models::{FileDiff, StagedDiffSummary};
use crate::services::git_service::GitService;

#[tauri::command]
pub async fn get_working_diff(path: String, file_path: String, staged: bool) -> Result<FileDiff, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::get_working_diff(&path, &file_path, staged))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_commit_diff(path: String, sha: String, file_path: String) -> Result<FileDiff, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::get_commit_diff(&path, &sha, &file_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_file_content(path: String, file_path: String, sha: Option<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::get_file_content(&path, &file_path, sha.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_staged_file_content(path: String, file_path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::get_staged_file_content(&path, &file_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_staged_diff_summary(path: String) -> Result<StagedDiffSummary, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::staged_diff_summary(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn has_conflict_markers(path: String, file_path: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::has_conflict_markers(&path, &file_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_file_content(path: String, file_path: String, content: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || GitService::save_file_content(&path, &file_path, &content))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn revert_hunk(path: String, file_path: String, hunk_index: usize, staged: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || GitService::revert_hunk(&path, &file_path, hunk_index, staged))
        .await
        .map_err(|e| e.to_string())?
}

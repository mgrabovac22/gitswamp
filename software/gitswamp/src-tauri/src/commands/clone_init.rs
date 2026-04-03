use crate::services::git_service::GitService;
use tauri::{AppHandle, Emitter};

const CLONE_PROGRESS_EVENT: &str = "clone-progress";

#[tauri::command]
pub async fn clone_repo(
    app: AppHandle,
    url: String,
    path: String,
    shallow: bool,
    token: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_handle = app;
        GitService::clone_repo_with_progress(&url, &path, shallow, token.as_deref(), |progress| {
            let _ = app_handle.emit(CLONE_PROGRESS_EVENT, &progress);
        })
    })
    .await
    .map_err(|e| format!("Clone task failed: {e}"))?
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

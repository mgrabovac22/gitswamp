use std::path::Path;
use tauri_plugin_dialog::DialogExt;

use crate::models::CommitInfo;
use crate::services::integration_service::IntegrationService;
use crate::services::worktree_service::{
    self, WorkspaceAction, WorkspaceLocation, WorkspaceStatus,
};

#[tauri::command]
pub async fn open_branch_workspace(
    path: String,
    branch_ref: String,
) -> Result<WorkspaceLocation, String> {
    tauri::async_runtime::spawn_blocking(move || worktree_service::open(&path, &branch_ref))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn open_branch_workspace_with_application(
    window: tauri::WebviewWindow,
    path: String,
    branch: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        worktree_service::validate_checkout(&path, Some(&branch))?;
        let picker = window.dialog().file().set_parent(&window).set_title("Choose an application");
        #[cfg(windows)]
        let picker = picker.add_filter("Applications", &["exe"]);
        #[cfg(target_os = "macos")]
        let picker = picker.set_directory("/Applications").add_filter("Applications", &["app"]);

        let Some(selection) = picker.blocking_pick_file() else { return Ok(()); };
        let application = selection.into_path().map_err(|error| error.to_string())?;
        worktree_service::validate_checkout(&path, Some(&branch))?;
        IntegrationService::open_folder_with_application(Path::new(&path), &application)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn remove_branch_workspace(
    path: String,
    worktree_path: String,
    branch: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        worktree_service::remove(&path, &worktree_path, &branch)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn get_branch_workspace_status(
    path: String,
    branch: String,
) -> Result<WorkspaceStatus, String> {
    tauri::async_runtime::spawn_blocking(move || worktree_service::status(&path, &branch))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn get_branch_workspace_history(
    path: String,
    tip: String,
    offset: usize,
) -> Result<Vec<CommitInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || worktree_service::history(&path, &tip, offset))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn mutate_branch_workspace(
    path: String,
    branch: String,
    action: WorkspaceAction,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || worktree_service::mutate(&path, &branch, action))
        .await
        .map_err(|error| error.to_string())?
}

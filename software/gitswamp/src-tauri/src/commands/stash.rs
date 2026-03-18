use crate::models::StashInfo;
use crate::models::CommitFileInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub fn stash_list(path: String) -> Result<Vec<StashInfo>, String> {
    GitService::stash_list(&path)
}

#[tauri::command]
pub fn stash_push(path: String, message: Option<String>) -> Result<String, String> {
    GitService::stash_push(&path, message.as_deref())
}

#[tauri::command]
pub fn stash_pop(path: String, index: usize) -> Result<String, String> {
    GitService::stash_pop(&path, index)
}

#[tauri::command]
pub fn stash_apply(path: String, index: usize) -> Result<String, String> {
    GitService::stash_apply(&path, index)
}

#[tauri::command]
pub fn stash_drop(path: String, index: usize) -> Result<String, String> {
    GitService::stash_drop(&path, index)
}

#[tauri::command]
pub fn stash_files(path: String, index: usize) -> Result<Vec<CommitFileInfo>, String> {
    GitService::stash_files(&path, index)
}

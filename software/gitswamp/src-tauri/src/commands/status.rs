use crate::models::FileStatusInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub fn get_status(path: String) -> Result<Vec<FileStatusInfo>, String> {
    GitService::status(&path)
}

#[tauri::command]
pub fn stage_file(path: String, file_path: String) -> Result<(), String> {
    GitService::stage_file(&path, &file_path)
}

#[tauri::command]
pub fn unstage_file(path: String, file_path: String) -> Result<(), String> {
    GitService::unstage_file(&path, &file_path)
}

#[tauri::command]
pub fn create_commit(path: String, message: String) -> Result<String, String> {
    GitService::create_commit(&path, &message)
}

#[tauri::command]
pub fn discard_file(path: String, file_path: String) -> Result<(), String> {
    GitService::discard_file(&path, &file_path)
}

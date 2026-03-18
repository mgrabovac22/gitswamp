use crate::models::FileDiff;
use crate::services::git_service::GitService;

#[tauri::command]
pub fn get_working_diff(path: String, file_path: String, staged: bool) -> Result<FileDiff, String> {
    GitService::get_working_diff(&path, &file_path, staged)
}

#[tauri::command]
pub fn get_commit_diff(path: String, sha: String, file_path: String) -> Result<FileDiff, String> {
    GitService::get_commit_diff(&path, &sha, &file_path)
}

#[tauri::command]
pub fn get_file_content(path: String, file_path: String, sha: Option<String>) -> Result<String, String> {
    GitService::get_file_content(&path, &file_path, sha.as_deref())
}

#[tauri::command]
pub fn save_file_content(path: String, file_path: String, content: String) -> Result<(), String> {
    GitService::save_file_content(&path, &file_path, &content)
}

#[tauri::command]
pub fn revert_hunk(path: String, file_path: String, hunk_index: usize, staged: bool) -> Result<(), String> {
    GitService::revert_hunk(&path, &file_path, hunk_index, staged)
}

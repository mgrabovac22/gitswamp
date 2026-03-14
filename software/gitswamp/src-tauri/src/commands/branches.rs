use crate::models::BranchInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub fn get_branches(path: String) -> Result<Vec<BranchInfo>, String> {
    GitService::branches(&path)
}

#[tauri::command]
pub fn checkout_branch(path: String, branch_name: String) -> Result<(), String> {
    GitService::checkout_branch(&path, &branch_name)
}

#[tauri::command]
pub fn create_branch(path: String, name: String, start_point: Option<String>) -> Result<(), String> {
    GitService::create_branch(&path, &name, start_point.as_deref())
}

#[tauri::command]
pub fn delete_branch(path: String, name: String) -> Result<(), String> {
    GitService::delete_branch(&path, &name)
}

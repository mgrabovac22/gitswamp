use crate::models::GhostBranchState;
use crate::services::git_service::GitService;

#[tauri::command]
pub fn get_ghost_branch_state(path: String) -> Result<GhostBranchState, String> {
    GitService::ghost_branch_state(&path)
}

#[tauri::command]
pub fn start_ghost_branch(path: String) -> Result<GhostBranchState, String> {
    GitService::start_ghost_branch(&path)
}

#[tauri::command]
pub fn materialize_ghost_branch(path: String, name: String) -> Result<String, String> {
    GitService::materialize_ghost_branch(&path, &name)
}

#[tauri::command]
pub fn discard_ghost_branch(path: String) -> Result<String, String> {
    GitService::discard_ghost_branch(&path)
}

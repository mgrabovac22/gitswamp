use crate::models::CommitFileInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub fn get_commit_files(path: String, sha: String) -> Result<Vec<CommitFileInfo>, String> {
    GitService::commit_files(&path, &sha)
}

use crate::models::TagInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub fn get_tags(path: String) -> Result<Vec<TagInfo>, String> {
    GitService::tags(&path)
}

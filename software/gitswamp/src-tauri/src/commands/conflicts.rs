use crate::models::ConflictHotspot;
use crate::services::git_service::GitService;

#[tauri::command]
pub async fn get_conflict_hotspots(
    path: String,
    max_count: Option<usize>,
) -> Result<Vec<ConflictHotspot>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::conflict_hotspots(&path, max_count.unwrap_or(300))
    })
    .await
    .map_err(|e| e.to_string())?
}

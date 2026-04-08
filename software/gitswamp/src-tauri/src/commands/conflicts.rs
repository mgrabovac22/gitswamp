use crate::models::{ConflictHotspot, ConflictPair, MergeRiskPreflight};
use crate::services::git_service::GitService;

#[tauri::command]
pub async fn get_conflict_hotspots(
    path: String,
    max_count: Option<usize>,
    lookback_months: Option<u32>,
) -> Result<Vec<ConflictHotspot>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::conflict_hotspots(&path, max_count.unwrap_or(300), lookback_months)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_conflict_pairs(
    path: String,
    max_count: Option<usize>,
    lookback_months: Option<u32>,
) -> Result<Vec<ConflictPair>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::conflict_pairs(&path, max_count.unwrap_or(300), lookback_months)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_repository_tree_paths(
    path: String,
    max_count: Option<usize>,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::repository_tree_paths(&path, max_count.unwrap_or(120000))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_merge_preflight_risk(
    path: String,
    source_branch: String,
    source_remote: Option<bool>,
    target_branch: String,
    max_count: Option<usize>,
    lookback_months: Option<u32>,
) -> Result<MergeRiskPreflight, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::merge_preflight_risk(
            &path,
            &source_branch,
            source_remote.unwrap_or(false),
            &target_branch,
            max_count.unwrap_or(500),
            lookback_months,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

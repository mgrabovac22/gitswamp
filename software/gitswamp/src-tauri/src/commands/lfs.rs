use crate::services::lfs;

#[tauri::command]
pub async fn get_lfs_overview(path: String) -> Result<lfs::Overview, String> {
    tauri::async_runtime::spawn_blocking(move || lfs::overview(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn setup_lfs(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || lfs::setup(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn change_lfs_rule(path: String, pattern: String, remove: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || lfs::rule(&path, &pattern, remove))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn convert_staged_lfs(
    path: String,
    files: Vec<String>,
    snapshot: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || lfs::convert(&path, &files, &snapshot))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn transfer_lfs(path: String, remote: String, upload: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || lfs::transfer(&path, &remote, upload))
        .await
        .map_err(|e| e.to_string())?
}

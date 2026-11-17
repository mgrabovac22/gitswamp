use std::path::Path;

use crate::services::patches::{self, ExportScope};

#[tauri::command]
pub async fn export_patch(
    path: String,
    destination: String,
    sha: Option<String>,
    scope: ExportScope,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        patches::export(&path, Path::new(&destination), sha.as_deref(), scope)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn import_patch(
    path: String,
    source: String,
    staged: bool,
    expected_head: Option<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        patches::import(&path, Path::new(&source), staged, expected_head.as_deref())
    })
    .await
    .map_err(|error| error.to_string())?
}

use crate::services::commit_composer::{self, Composition, Draft};

#[tauri::command]
pub async fn plan_commit_composition(
    path: String,
    secret_check: bool,
) -> Result<Composition, String> {
    tauri::async_runtime::spawn_blocking(move || commit_composer::plan(&path, secret_check))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn apply_commit_composition(
    path: String,
    snapshot: String,
    drafts: Vec<Draft>,
    secret_check: bool,
    secret_approval: bool,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commit_composer::apply(&path, &snapshot, &drafts, secret_check, secret_approval)
    })
    .await
    .map_err(|error| error.to_string())?
}

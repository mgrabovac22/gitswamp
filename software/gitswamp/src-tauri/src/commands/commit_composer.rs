use crate::services::commit_composer::{self, BuildPlan, Composition, Draft};

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
pub async fn find_buildable_commit_groups(
    path: String,
    snapshot: String,
    drafts: Vec<Draft>,
    secret_check: bool,
    build_command: String,
) -> Result<BuildPlan, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commit_composer::find_buildable_groups(
            &path,
            &snapshot,
            &drafts,
            secret_check,
            &build_command,
        )
    })
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
    build_command: Option<String>,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commit_composer::apply_with_verification(
            &path,
            &snapshot,
            &drafts,
            secret_check,
            secret_approval,
            build_command.as_deref(),
        )
    })
    .await
    .map_err(|error| error.to_string())?
}

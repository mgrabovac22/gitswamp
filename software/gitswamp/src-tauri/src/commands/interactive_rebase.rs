use crate::services::interactive_rebase::{self, FollowUp, Outcome, RebasePlan, RebaseStep};

#[tauri::command]
pub async fn get_interactive_rebase_plan(path: String, base: String) -> Result<RebasePlan, String> {
    tauri::async_runtime::spawn_blocking(move || interactive_rebase::plan(&path, &base))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn start_interactive_rebase(
    path: String,
    base: String,
    head: String,
    branch: String,
    steps: Vec<RebaseStep>,
) -> Result<Outcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        interactive_rebase::start(&path, &base, &head, &branch, &steps)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn interactive_rebase_action(
    path: String,
    action: FollowUp,
    secret_check: Option<bool>,
    secret_approval: Option<String>,
) -> Result<Outcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        interactive_rebase::follow_up(
            &path,
            action,
            secret_check.unwrap_or(true),
            secret_approval.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

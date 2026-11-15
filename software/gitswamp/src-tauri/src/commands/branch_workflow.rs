use crate::services::branch_workflow::{self, Config, Inspection, Outcome, Plan, Request};

#[tauri::command]
pub fn validate_branch_workflow(config: Config) -> Result<(), String> {
    branch_workflow::validate(&config)
}

#[tauri::command]
pub async fn inspect_branch_workflow(path: String) -> Result<Inspection, String> {
    tauri::async_runtime::spawn_blocking(move || branch_workflow::inspect(&path))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn preview_branch_workflow(
    path: String,
    config: Config,
    request: Request,
) -> Result<Plan, String> {
    tauri::async_runtime::spawn_blocking(move || branch_workflow::preview(&path, &config, &request))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn execute_branch_workflow(
    path: String,
    config: Config,
    request: Request,
    token: String,
) -> Result<Outcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        branch_workflow::execute(&path, &config, &request, &token)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn verify_branch_handoff(path: String, config: Config, request: Request, token: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || branch_workflow::verify_handoff(&path, &config, &request, &token))
        .await
        .map_err(|error| error.to_string())?
}

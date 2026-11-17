use crate::services::remote_tracking::{self, Credentials, Request, TrackingPage};

#[tauri::command]
pub async fn remote_tracking(
    path: String,
    remote: String,
    credentials: Credentials,
    request: Request,
) -> Result<TrackingPage, String> {
    tauri::async_runtime::spawn_blocking(move || {
        remote_tracking::track(&path, &remote, credentials, request)
    })
    .await
    .map_err(|error| error.to_string())?
}

use crate::services::operation_preview::{self, OperationKind, OperationSummary};

#[tauri::command]
pub async fn get_operation_preview(
    path: String,
    kind: OperationKind,
    result_sha: String,
    before_sha: Option<String>,
    source_sha: Option<String>,
) -> Result<Option<OperationSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        operation_preview::summarize(
            &path,
            kind,
            &result_sha,
            before_sha.as_deref(),
            source_sha.as_deref(),
        )
    })
    .await
    .map_err(|error| error.to_string())?
}

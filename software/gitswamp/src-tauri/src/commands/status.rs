use crate::models::FileStatusInfo;
use crate::services::git_service::GitService;

#[tauri::command]
pub async fn get_status(path: String) -> Result<Vec<FileStatusInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::status(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn stage_file(path: String, file_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || GitService::stage_file(&path, &file_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn stage_files(path: String, file_paths: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        for file_path in file_paths {
            GitService::stage_file(&path, &file_path)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn unstage_file(path: String, file_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || GitService::unstage_file(&path, &file_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn unstage_files(path: String, file_paths: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        for file_path in file_paths {
            GitService::unstage_file(&path, &file_path)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_commit(path: String, message: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::create_commit(&path, &message))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn discard_file(path: String, file_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || GitService::discard_file(&path, &file_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn discard_files(path: String, file_paths: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        for file_path in file_paths {
            GitService::discard_file(&path, &file_path)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn resolve_conflict_file(path: String, file_path: String, strategy: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::resolve_conflict_file(&path, &file_path, &strategy)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn resolve_all_conflicts(path: String, strategy: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || GitService::resolve_all_conflicts(&path, &strategy))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_empty_directories(path: String, max_count: Option<usize>) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::get_empty_directories(&path, max_count.unwrap_or(200))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_gitkeep(path: String, directory_path: String, stage: Option<bool>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::add_gitkeep(&path, &directory_path, stage.unwrap_or(true))
    })
    .await
    .map_err(|e| e.to_string())?
}

use std::fs;
use std::path::PathBuf;

const APP_ROOT_DIR: &str = ".gitswamp";
const APP_STATE_DIR: &str = "state";

fn appdata_root() -> PathBuf {
    std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

fn state_dir() -> PathBuf {
    appdata_root().join(APP_ROOT_DIR).join(APP_STATE_DIR)
}

fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if cleaned.is_empty() {
        "store".to_string()
    } else {
        cleaned
    }
}

fn state_file(name: &str) -> PathBuf {
    state_dir().join(format!("{}.json", sanitize_name(name)))
}

#[tauri::command]
pub fn read_app_state(name: String) -> Result<Option<String>, String> {
    let path = state_file(&name);
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| format!("Failed to read app state: {}", e))
}

#[tauri::command]
pub fn write_app_state(name: String, contents: String) -> Result<(), String> {
    let dir = state_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app state directory: {}", e))?;

    let path = state_file(&name);
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, contents).map_err(|e| format!("Failed to write app state: {}", e))?;
    fs::rename(&temp, &path).map_err(|e| format!("Failed to persist app state: {}", e))
}

#[tauri::command]
pub fn get_app_state_path(name: String) -> Result<String, String> {
    let dir = state_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app state directory: {}", e))?;
    Ok(state_file(&name).to_string_lossy().to_string())
}

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_ROOT_DIR: &str = ".gitswamp";
const LOG_SUB_DIR: &str = "logs";
const LOG_FILE: &str = "app.log";
const LOG_ROTATED_FILE: &str = "app.log.1";
const LOG_ROTATE_SIZE_BYTES: u64 = 5 * 1024 * 1024;

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

fn logs_dir() -> PathBuf {
    appdata_root().join(LOG_ROOT_DIR).join(LOG_SUB_DIR)
}

fn log_file_path() -> PathBuf {
    logs_dir().join(LOG_FILE)
}

fn rotate_log_if_needed(path: &PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let metadata = fs::metadata(path).map_err(|e| format!("Failed to stat log file: {}", e))?;
    if metadata.len() < LOG_ROTATE_SIZE_BYTES {
        return Ok(());
    }

    let rotated = logs_dir().join(LOG_ROTATED_FILE);
    if rotated.exists() {
        let _ = fs::remove_file(&rotated);
    }

    fs::rename(path, rotated).map_err(|e| format!("Failed to rotate log file: {}", e))
}

#[tauri::command]
pub fn append_app_log(channel: String, message: String) -> Result<(), String> {
    let dir = logs_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create logs directory: {}", e))?;

    let file_path = log_file_path();
    rotate_log_if_needed(&file_path)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sanitized_channel = channel.trim().to_lowercase();
    let sanitized_message = message.replace('\n', " ").replace('\r', " ");

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;

    writeln!(file, "[{}] [{}] {}", now, sanitized_channel, sanitized_message)
        .map_err(|e| format!("Failed to append log entry: {}", e))
}

#[tauri::command]
pub fn get_app_log_path() -> Result<String, String> {
    let dir = logs_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create logs directory: {}", e))?;

    Ok(log_file_path().to_string_lossy().to_string())
}

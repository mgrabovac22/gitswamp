use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FileStatusInfo {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub conflicted: bool,
    pub file_size_bytes: Option<u64>,
}

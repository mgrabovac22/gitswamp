use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
    pub branch: String,
    pub timestamp: String,
    pub parent_sha: String,
}

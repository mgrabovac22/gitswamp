use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CommitFileInfo {
    pub path: String,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
}

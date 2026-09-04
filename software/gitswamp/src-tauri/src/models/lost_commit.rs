use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LostCommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub time_ago: String,
    pub parent_shas: Vec<String>,
    pub source: String,
}

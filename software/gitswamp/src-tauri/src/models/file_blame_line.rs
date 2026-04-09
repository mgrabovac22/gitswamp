use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FileBlameLine {
    pub line_no: u32,
    pub commit_sha: String,
    pub short_sha: String,
    pub author: String,
    pub author_email: String,
    pub summary: String,
    pub author_time: i64,
    pub is_uncommitted: bool,
    pub code: String,
}

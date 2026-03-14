use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TagInfo {
    pub name: String,
    pub sha: String,
    pub message: Option<String>,
    pub is_annotated: bool,
}

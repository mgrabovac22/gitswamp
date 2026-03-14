use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RepoInfo {
    pub path: String,
    pub name: String,
    pub current_branch: String,
    pub is_clean: bool,
    pub head_sha: Option<String>,
}

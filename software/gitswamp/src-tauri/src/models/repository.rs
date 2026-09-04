use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepositoryOperationInfo {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoInfo {
    pub path: String,
    pub name: String,
    pub current_branch: String,
    pub is_clean: bool,
    pub head_sha: Option<String>,
    pub remotes: Vec<RemoteInfo>,
    pub operation: Option<RepositoryOperationInfo>,
}

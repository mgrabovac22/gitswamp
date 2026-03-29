use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GhostBranchState {
    pub active: bool,
    pub base_branch: String,
    pub ghost_branch: String,
}

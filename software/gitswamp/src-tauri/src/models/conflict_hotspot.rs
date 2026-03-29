use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ConflictHotspot {
    pub path: String,
    pub score: usize,
    pub merge_touches: usize,
    pub conflict_mentions: usize,
}

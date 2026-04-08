use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ConflictHotspot {
    pub path: String,
    pub score: usize,
    pub merge_touches: usize,
    pub conflict_mentions: usize,
    pub collision_index: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictPair {
    pub left_path: String,
    pub right_path: String,
    pub co_touches: usize,
    pub conflict_touches: usize,
    pub score: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MergeRiskPreflight {
    pub source_ref: String,
    pub target_ref: String,
    pub lookback_months: Option<u32>,
    pub inspected_merges: usize,
    pub risk_level: String,
    pub risk_score: usize,
    pub shared_change_count: usize,
    pub suspect_count: usize,
    pub suspect_files: Vec<ConflictHotspot>,
}

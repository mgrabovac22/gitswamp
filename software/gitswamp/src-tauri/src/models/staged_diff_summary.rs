use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StagedDiffSummary {
    pub total_lines_added: usize,
    pub total_lines_removed: usize,
    pub files_changed: usize,
    pub file_types: Vec<String>,
    pub has_test_changes: bool,
    pub has_migration_changes: bool,
    pub inferred_scope: String,
}

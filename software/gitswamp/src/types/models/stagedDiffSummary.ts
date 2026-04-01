export interface StagedDiffSummary {
  total_lines_added: number;
  total_lines_removed: number;
  files_changed: number;
  file_types: string[];
  has_test_changes: boolean;
  has_migration_changes: boolean;
  inferred_scope: string;
}

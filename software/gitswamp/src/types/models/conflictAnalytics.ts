import type { ConflictHotspot } from "./conflictHotspot"

export interface ConflictPair {
  left_path: string
  right_path: string
  co_touches: number
  conflict_touches: number
  score: number
}

export interface MergeRiskPreflight {
  source_ref: string
  target_ref: string
  lookback_months?: number
  inspected_merges: number
  risk_level: "low" | "moderate" | "high" | "critical"
  risk_score: number
  shared_change_count: number
  suspect_count: number
  suspect_files: ConflictHotspot[]
}

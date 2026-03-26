import type { DiffLine } from "./diffLine"

export interface DiffHunk {
  old_start: number
  old_lines: number
  new_start: number
  new_lines: number
  header: string
  lines: DiffLine[]
}

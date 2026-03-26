import type { DiffHunk } from "./diffHunk"

export interface FileDiff {
  path: string
  old_path: string | null
  status: string
  hunks: DiffHunk[]
  is_binary: boolean
}

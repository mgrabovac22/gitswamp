import type { DiffHunk } from "./diffHunk"

export interface FileDiff {
  path: string
  old_path: string | null
  status: string
  hunks: DiffHunk[]
  is_binary: boolean
}

export interface BinarySideInfo {
  exists: boolean
  size: number
  id: string
}

export interface BinaryDiffInfo {
  old: BinarySideInfo
  new: BinarySideInfo
}

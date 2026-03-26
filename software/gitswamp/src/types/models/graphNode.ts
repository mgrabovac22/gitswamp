import type { CommitInfo } from "./commitInfo"

export interface GraphNode {
  commit: CommitInfo
  lane: number
  color: string
}

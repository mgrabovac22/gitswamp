export interface CommitFileInfo {
  path: string
  status: string
  additions: number
  deletions: number
  commit_shas?: string[]
}

export interface CommitInfo {
  sha: string
  short_sha: string
  message: string
  author_name: string
  author_email: string
  committer_name?: string
  committer_email?: string
  timestamp: number
  time_ago: string
  parent_shas: string[]
  refs: string[]
}

export interface CommitInfo {
  sha: string
  short_sha: string
  message: string
  author_name: string
  author_email: string
  timestamp: number
  time_ago: string
  parent_shas: string[]
  refs: string[]
}

export interface BranchInfo {
  name: string
  is_head: boolean
  is_remote: boolean
  upstream: string | null
  ahead: number
  behind: number
}

export interface FileStatusInfo {
  path: string
  status: string
  staged: boolean
}

export interface RepoInfo {
  path: string
  name: string
  current_branch: string
  is_clean: boolean
  head_sha: string | null
}

export interface CommitFileInfo {
  path: string
  status: string
  additions: number
  deletions: number
}

export interface StashInfo {
  index: number
  message: string
  branch: string
  timestamp: string
  parent_sha: string
}

export interface TagInfo {
  name: string
  sha: string
  message: string | null
  is_annotated: boolean
}

export interface GraphNode {
  commit: CommitInfo
  lane: number
  color: string
}

export interface GraphEdge {
  fromIndex: number
  toIndex: number
  fromLane: number
  toLane: number
  color: string
}

export interface GithubRepo {
  full_name: string
  clone_url: string
  description: string
  is_private: boolean
  stars: number
}

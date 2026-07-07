import type { RemoteLabelInfo, RemoteMilestoneInfo, RemoteReferenceInfo } from "./issueInfo"

export interface PullRequestFileInfo {
  filename: string
  status: string
  additions: number
  deletions: number
  changes: number
  url: string
}

export interface PullRequestInfo {
  id: number
  number: number
  title: string
  state: string
  draft: boolean
  author: string
  sourceBranch: string
  targetBranch: string
  createdAt: string
  updatedAt: string
  url: string
  description: string
  assignees?: string[]
  requestedReviewers?: string[]
  labels?: RemoteLabelInfo[]
  milestone?: RemoteMilestoneInfo | null
  comments?: number
  reviewComments?: number
  commitsCount?: number
  changedFiles?: number
  additions?: number
  deletions?: number
  mergeable?: boolean | null
  mergeableState?: string
  merged?: boolean
  mergedAt?: string
  mergedBy?: string
  linkedIssues?: RemoteReferenceInfo[]
  linkedCommits?: RemoteReferenceInfo[]
  files?: PullRequestFileInfo[]
}

export interface RemoteLabelInfo {
  name: string
  color: string
  description: string
}

export interface RemoteMilestoneInfo {
  number?: number
  title: string
  state: string
  description: string
  dueOn: string
  openIssues: number
  closedIssues: number
}

export interface RemoteUserInfo {
  login: string
  avatarUrl: string
  url: string
}

export interface RemoteReferenceInfo {
  kind: "commit" | "issue" | "pull_request"
  title: string
  number?: number
  sha?: string
  state?: string
  url: string
  author: string
  occurredAt: string
}

export interface RemoteIssueCreatePayload {
  title: string
  description: string
  labels: string[]
  assignees: string[]
  milestone: number | null
}

export interface RemotePullRequestCreatePayload extends RemoteIssueCreatePayload {
  sourceBranch: string
  targetBranch: string
  reviewers: string[]
}

export interface IssueInfo {
  id: number
  number: number
  title: string
  state: string
  author: string
  createdAt: string
  updatedAt: string
  url: string
  description: string
  assignees?: string[]
  labels?: RemoteLabelInfo[]
  milestone?: RemoteMilestoneInfo | null
  comments?: number
  closedAt?: string
  stateReason?: string
  linkedPullRequests?: RemoteReferenceInfo[]
  linkedCommits?: RemoteReferenceInfo[]
  linkedIssues?: RemoteReferenceInfo[]
}

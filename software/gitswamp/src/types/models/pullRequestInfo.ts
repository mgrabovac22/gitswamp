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
}

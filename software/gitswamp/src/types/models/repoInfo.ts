import type { RemoteInfo } from "./remoteInfo"

export interface RepositoryOperationInfo {
  kind: "merge" | "rebase" | "cherry-pick" | "revert" | "bisect" | "apply-mailbox" | "unknown"
  message: string
}

export interface RepoInfo {
  path: string
  name: string
  current_branch: string
  is_clean: boolean
  head_sha: string | null
  remotes: RemoteInfo[]
  operation: RepositoryOperationInfo | null
}

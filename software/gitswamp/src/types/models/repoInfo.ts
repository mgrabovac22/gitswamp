import type { RemoteInfo } from "./remoteInfo"

export interface RepoInfo {
  path: string
  name: string
  current_branch: string
  is_clean: boolean
  head_sha: string | null
  remotes: RemoteInfo[]
}

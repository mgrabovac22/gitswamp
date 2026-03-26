export interface BranchInfo {
  name: string
  is_head: boolean
  is_remote: boolean
  upstream: string | null
  ahead: number
  behind: number
}

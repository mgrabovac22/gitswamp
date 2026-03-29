export interface FileStatusInfo {
  path: string
  status: string
  staged: boolean
  conflicted?: boolean
  file_size_bytes?: number
}

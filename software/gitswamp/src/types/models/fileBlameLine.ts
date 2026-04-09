export interface FileBlameLine {
  line_no: number
  commit_sha: string
  short_sha: string
  author: string
  author_email: string
  summary: string
  author_time: number
  is_uncommitted: boolean
  code: string
}

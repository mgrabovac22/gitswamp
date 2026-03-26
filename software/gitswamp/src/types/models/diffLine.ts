export interface DiffLine {
  line_type: "context" | "addition" | "deletion" | "hunk_header"
  old_line_no: number | null
  new_line_no: number | null
  content: string
}

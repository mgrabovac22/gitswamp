export interface Repository {
  id: string
  name: string
  path: string
}

export interface Commit {
  id: string
  branch: string
  time: string
  number: string
  message: string
  author: string
  sha: string
  avatar: string
  color: string
}

export interface SidebarSectionItem {
  label: string
  items?: string[]
  count: number | string
}

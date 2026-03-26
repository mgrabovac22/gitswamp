export interface GitlabRepo {
  full_name: string
  path_with_namespace: string
  clone_url_ssh: string
  clone_url_https: string
  description: string
  is_private: boolean
  stars: number
}

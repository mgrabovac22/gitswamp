export interface RemoteInfo {
  name: string
  url: string
  provider: "github" | "gitlab" | "bitbucket" | "azure" | "unknown"
}

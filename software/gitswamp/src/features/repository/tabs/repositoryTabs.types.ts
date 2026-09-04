import type { RepoInfo } from "@/types";

export interface RepositoryTab {
  id: string;
  repo: RepoInfo | null;
  label: string;
  path: string;
}

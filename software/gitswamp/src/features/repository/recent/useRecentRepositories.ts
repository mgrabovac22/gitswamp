import { ref, watch } from "vue";
import type { RepoInfo } from "@/types";
import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

export interface RecentRepository {
  name: string;
  path: string;
  branch: string;
  owner?: string;
}

const RECENT_REPOS_KEY = "gitswamp-recent";
const MAX_RECENT_REPOS = 20;

function readStoredRecentRepos(): RecentRepository[] {
  const saved = safeStorageGet(RECENT_REPOS_KEY);
  if (!saved) return [];

  try {
    const parsed = JSON.parse(saved);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function useRecentRepositories() {
  const recentRepos = ref<RecentRepository[]>(readStoredRecentRepos());

  function addToRecent(repo: RepoInfo) {
    recentRepos.value = [
      {
        name: repo.name,
        path: repo.path,
        branch: repo.current_branch,
      },
      ...recentRepos.value.filter((item) => item.path !== repo.path),
    ].slice(0, MAX_RECENT_REPOS);
  }

  function removeRecent(path: string) {
    recentRepos.value = recentRepos.value.filter((item) => item.path !== path);
  }

  function clearRecent() {
    recentRepos.value = [];
  }

  watch(recentRepos, () => {
    safeStorageSet(RECENT_REPOS_KEY, JSON.stringify(recentRepos.value));
  }, { deep: true });

  return {
    recentRepos,
    addToRecent,
    removeRecent,
    clearRecent,
  };
}

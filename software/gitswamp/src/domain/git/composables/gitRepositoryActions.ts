import type { GithubRepo, RepoInfo } from "@/types";

import { callTauri } from "./gitCall";
import { getTokenForUrl } from "./gitHelpers";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshCommits: () => Promise<void>;
  refreshBranches: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  refreshStashes: () => Promise<void>;
  refreshTags: () => Promise<void>;
};

type WatcherDeps = {
  startFileWatcher: () => void;
};

export function createRepoActions(state: GitState, refresh: RefreshDeps, watcher: WatcherDeps) {
  async function openRepository(path: string) {
    try {
      state.loading.value = true;
      state.error.value = null;
      state.repoPath.value = path;
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path });
      state.hasMoreCommits.value = true;
      await Promise.all([
        refresh.refreshCommits(),
        refresh.refreshBranches(),
        refresh.refreshStatus(),
        refresh.refreshStashes(),
        refresh.refreshTags(),
      ]);
      watcher.startFileWatcher();
    } catch (e) {
      state.error.value = String(e);
    } finally {
      state.loading.value = false;
    }
  }

  async function cloneRepo(url: string, path: string, shallow = false, token?: string | null): Promise<string | null> {
    try {
      state.loading.value = true;
      state.error.value = null;
      const t = token || getTokenForUrl(state, url);
      return await callTauri<string>("clone_repo", { url, path, shallow, token: t });
    } catch (e) {
      state.error.value = String(e);
      return null;
    } finally {
      state.loading.value = false;
    }
  }

  async function initRepo(path: string, branchName?: string) {
    try {
      state.loading.value = true;
      state.error.value = null;
      await callTauri<string>("init_repo", { path, branchName });
      return true;
    } catch (e) {
      state.error.value = String(e);
      return false;
    } finally {
      state.loading.value = false;
    }
  }

  async function searchGithubRepos(query: string): Promise<GithubRepo[]> {
    if (!state.githubToken.value) {
      state.error.value = "No GitHub token configured. Go to Settings to add one.";
      return [];
    }

    try {
      return await callTauri<GithubRepo[]>("search_github_repos", {
        token: state.githubToken.value,
        query,
      });
    } catch (e) {
      state.error.value = String(e);
      return [];
    }
  }

  return {
    openRepository,
    cloneRepo,
    initRepo,
    searchGithubRepos,
  };
}

import type { BranchInfo, CommitInfo, FileStatusInfo, GithubRepo, RepoInfo, StashInfo, TagInfo } from "@/types";

import { callTauri } from "./gitCall";
import { getTokenForUrl } from "./gitHelpers";
import { PAGE_SIZE, type GitState } from "./gitState";

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

type RepositorySnapshot = {
  repoInfo: RepoInfo | null;
  commits: CommitInfo[];
  branches: BranchInfo[];
  fileStatuses: FileStatusInfo[];
  stashes: StashInfo[];
  tags: TagInfo[];
  hasMoreCommits: boolean;
  lastStatusHash: string;
};

const REPOSITORY_SNAPSHOT_LIMIT = 4;

export function createRepoActions(state: GitState, refresh: RefreshDeps, watcher: WatcherDeps) {
  const repositorySnapshots = new Map<string, RepositorySnapshot>();
  let openRepositoryRunId = 0;

  function getRepositorySnapshotKey(path: string) {
    return path.trim().replace(/\\/g, "/").replace(/\/+$/g, "").toLowerCase();
  }

  function rememberRepositorySnapshot(path: string, snapshot: RepositorySnapshot) {
    const key = getRepositorySnapshotKey(path);
    if (!key) return;

    if (repositorySnapshots.has(key)) {
      repositorySnapshots.delete(key);
    }
    repositorySnapshots.set(key, snapshot);

    while (repositorySnapshots.size > REPOSITORY_SNAPSHOT_LIMIT) {
      const oldestPath = repositorySnapshots.keys().next().value;
      if (!oldestPath) break;
      repositorySnapshots.delete(oldestPath);
    }
  }

  function snapshotCurrentRepository() {
    const path = state.repoPath.value;
    if (!path) return;

    const commits = state.commits.value.length > PAGE_SIZE
      ? state.commits.value.slice(0, PAGE_SIZE)
      : state.commits.value;

    const snapshot = {
      repoInfo: state.repoInfo.value,
      commits,
      branches: state.branches.value,
      fileStatuses: state.fileStatuses.value,
      stashes: state.stashes.value,
      tags: state.tags.value,
      hasMoreCommits: state.hasMoreCommits.value || state.commits.value.length > commits.length,
      lastStatusHash: state.lastStatusHash.value,
    };

    rememberRepositorySnapshot(path, snapshot);
    if (state.repoInfo.value?.path && state.repoInfo.value.path !== path) {
      rememberRepositorySnapshot(state.repoInfo.value.path, snapshot);
    }
  }

  function applyRepositorySnapshot(path: string) {
    const snapshot = repositorySnapshots.get(getRepositorySnapshotKey(path));
    if (!snapshot) return false;

    rememberRepositorySnapshot(path, snapshot);
    state.repoInfo.value = snapshot.repoInfo;
    state.commits.value = snapshot.commits;
    state.branches.value = snapshot.branches;
    state.fileStatuses.value = snapshot.fileStatuses;
    state.stashes.value = snapshot.stashes;
    state.tags.value = snapshot.tags;
    state.hasMoreCommits.value = snapshot.hasMoreCommits;
    state.lastStatusHash.value = snapshot.lastStatusHash;
    return true;
  }

  function clearRepositorySnapshotState() {
    state.repoInfo.value = null;
    state.commits.value = [];
    state.branches.value = [];
    state.fileStatuses.value = [];
    state.stashes.value = [];
    state.tags.value = [];
    state.hasMoreCommits.value = true;
    state.lastStatusHash.value = "";
  }

  async function refreshRepositoryFromDisk(path: string, runId: number, showLoading: boolean) {
    try {
      if (showLoading) {
        state.loading.value = true;
      }

      const repoInfo = await callTauri<RepoInfo>("get_repo_info", { path });
      if (runId !== openRepositoryRunId || state.repoPath.value !== path) {
        return;
      }

      state.repoInfo.value = repoInfo;
      await Promise.all([
        refresh.refreshCommits(),
        refresh.refreshBranches(),
        refresh.refreshStatus(),
        refresh.refreshStashes(),
        refresh.refreshTags(),
      ]);
      if (runId !== openRepositoryRunId || state.repoPath.value !== path) {
        return;
      }

      snapshotCurrentRepository();
      watcher.startFileWatcher();
    } catch (e) {
      if (runId !== openRepositoryRunId || state.repoPath.value !== path) return;
      state.error.value = String(e);
    } finally {
      if (showLoading && runId === openRepositoryRunId && state.repoPath.value === path) {
        state.loading.value = false;
      }
    }
  }

  async function openRepository(path: string) {
    if (
      state.repoInfo.value
      && getRepositorySnapshotKey(state.repoPath.value) === getRepositorySnapshotKey(path)
    ) {
      return;
    }

    const runId = ++openRepositoryRunId;
    snapshotCurrentRepository();

    state.error.value = null;
    state.repoPath.value = path;
    state.loadingMore.value = false;

    const hasSnapshot = applyRepositorySnapshot(path);
    if (hasSnapshot) {
      state.loading.value = false;
      watcher.startFileWatcher();
      globalThis.setTimeout(() => {
        void refreshRepositoryFromDisk(path, runId, false);
      }, 0);
      return;
    }

    clearRepositorySnapshotState();
    await refreshRepositoryFromDisk(path, runId, true);
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
      state.error.value = "No GitHub token configured. Go to Options > Integrations to add one.";
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

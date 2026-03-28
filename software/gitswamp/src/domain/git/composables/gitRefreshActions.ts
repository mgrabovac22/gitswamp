import type { CommitFileInfo, CommitInfo, BranchInfo, FileStatusInfo, RepoInfo, StashInfo, TagInfo } from "@/types";

import { callTauri } from "./gitCall";
import { PAGE_SIZE, type GitState } from "./gitState";
import { statusHash } from "./gitHelpers";

export function createRefreshActions(state: GitState) {
  let loadMoreDebounce: ReturnType<typeof setTimeout> | null = null;

  async function loadCommitsToCount(targetCount: number): Promise<boolean> {
    if (!state.repoPath.value || state.loadingMore.value) return false;

    state.loadingMore.value = true;
    try {
      const currentCount = state.commits.value.length;
      const result = await callTauri<CommitInfo[]>("get_commits", {
        path: state.repoPath.value,
        maxCount: targetCount,
      });

      if (result.length <= currentCount) {
        state.hasMoreCommits.value = false;
        return false;
      }

      state.commits.value = result;
      state.hasMoreCommits.value = result.length >= targetCount;
      return true;
    } catch (e) {
      state.error.value = String(e);
      return false;
    } finally {
      state.loadingMore.value = false;
    }
  }

  async function refreshCommits() {
    if (!state.repoPath.value) return;
    try {
      const result = await callTauri<CommitInfo[]>("get_commits", {
        path: state.repoPath.value,
        maxCount: PAGE_SIZE,
      });
      state.commits.value = result;
      state.hasMoreCommits.value = result.length >= PAGE_SIZE;
    } catch (e) {
      state.error.value = String(e);
    }
  }

  function loadMoreCommits() {
    if (loadMoreDebounce) clearTimeout(loadMoreDebounce);
    loadMoreDebounce = setTimeout(() => {
      void doLoadMoreCommits();
    }, 50);
  }

  async function doLoadMoreCommits() {
    if (!state.repoPath.value || !state.hasMoreCommits.value || state.loadingMore.value) return;

    const currentCount = state.commits.value.length;
    const nextCount = currentCount + PAGE_SIZE;
    await loadCommitsToCount(nextCount);
  }

  async function ensureCommitLoaded(sha: string): Promise<boolean> {
    if (!sha || !state.repoPath.value) return false;

    const hasCommit = () => state.commits.value.some((commit) => commit.sha === sha);
    if (hasCommit()) {
      return true;
    }

    while (state.hasMoreCommits.value && !state.loadingMore.value) {
      const currentCount = state.commits.value.length;
      const changed = await loadCommitsToCount(currentCount + PAGE_SIZE * 3);
      if (!changed) {
        break;
      }
      if (hasCommit()) {
        return true;
      }
    }

    return hasCommit();
  }

  async function refreshBranches() {
    if (!state.repoPath.value) return;
    try {
      state.branches.value = await callTauri<BranchInfo[]>("get_branches", { path: state.repoPath.value });
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function refreshStatus() {
    if (!state.repoPath.value) return;
    try {
      state.fileStatuses.value = await callTauri<FileStatusInfo[]>("get_status", { path: state.repoPath.value });
      state.lastStatusHash.value = statusHash(state.fileStatuses.value);
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function refreshStashes() {
    if (!state.repoPath.value) return;
    try {
      state.stashes.value = await callTauri<StashInfo[]>("stash_list", { path: state.repoPath.value });
    } catch {
      // Ignore optional refresh failures.
    }
  }

  async function refreshTags() {
    if (!state.repoPath.value) return;
    try {
      state.tags.value = await callTauri<TagInfo[]>("get_tags", { path: state.repoPath.value });
    } catch {
      // Ignore optional refresh failures.
    }
  }

  async function refreshAll() {
    if (!state.repoPath.value) return;
    state.loading.value = true;
    try {
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
      await Promise.all([refreshCommits(), refreshBranches(), refreshStatus(), refreshStashes(), refreshTags()]);
    } catch (e) {
      state.error.value = String(e);
    } finally {
      state.loading.value = false;
    }
  }

  async function getCommitFiles(sha: string) {
    if (!state.repoPath.value) return;
    try {
      state.selectedCommitFiles.value = await callTauri<CommitFileInfo[]>("get_commit_files", {
        path: state.repoPath.value,
        sha,
      });
    } catch (e) {
      state.error.value = String(e);
      state.selectedCommitFiles.value = [];
    }
  }

  async function selectStash(stash: StashInfo | null) {
    state.selectedStash.value = stash;
    state.selectedCommit.value = null;
    state.selectedCommitFiles.value = [];

    if (!stash || !state.repoPath.value) {
      state.selectedStashFiles.value = [];
      return;
    }

    try {
      state.selectedStashFiles.value = await callTauri<CommitFileInfo[]>("stash_files", {
        path: state.repoPath.value,
        index: stash.index,
      });
    } catch (e) {
      state.error.value = String(e);
      state.selectedStashFiles.value = [];
    }
  }

  function clearStashSelection() {
    state.selectedStash.value = null;
    state.selectedStashFiles.value = [];
  }

  async function searchCommits(query: string) {
    if (!state.repoPath.value) return;
    if (!query.trim()) {
      clearSearch();
      return;
    }

    try {
      state.searchQuery.value = query;
      const results = await callTauri<CommitInfo[]>("search_commits", {
        path: state.repoPath.value,
        query,
        maxCount: 50000,
      });
      state.searchResults.value = results;
      state.hasMoreSearchResults.value = false;

      if (results.length > 0) {
        await ensureCommitLoaded(results[0].sha);
      }
    } catch (e) {
      state.error.value = String(e);
    }
  }

  function clearSearch() {
    state.searchQuery.value = "";
    state.searchResults.value = null;
    state.hasMoreSearchResults.value = false;
  }

  return {
    refreshCommits,
    loadMoreCommits,
    refreshBranches,
    refreshStatus,
    refreshStashes,
    refreshTags,
    refreshAll,
    getCommitFiles,
    selectStash,
    clearStashSelection,
    searchCommits,
    clearSearch,
    ensureCommitLoaded,
  };
}

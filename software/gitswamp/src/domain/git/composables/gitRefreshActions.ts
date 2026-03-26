import type { CommitFileInfo, CommitInfo, BranchInfo, FileStatusInfo, RepoInfo, StashInfo, TagInfo } from "@/types";

import { callTauri } from "./gitCall";
import { PAGE_SIZE, type GitState } from "./gitState";
import { statusHash } from "./gitHelpers";

export function createRefreshActions(state: GitState) {
  let loadMoreDebounce: ReturnType<typeof setTimeout> | null = null;

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
    if (state.searchQuery.value && state.searchResults.value !== null) return;
    if (!state.repoPath.value || !state.hasMoreCommits.value || state.loadingMore.value) return;

    state.loadingMore.value = true;
    try {
      const currentCount = state.commits.value.length;
      const nextCount = currentCount + PAGE_SIZE;
      const result = await callTauri<CommitInfo[]>("get_commits", {
        path: state.repoPath.value,
        maxCount: nextCount,
      });
      if (result.length <= currentCount) {
        state.hasMoreCommits.value = false;
      } else {
        state.commits.value = result;
        state.hasMoreCommits.value = result.length >= nextCount;
      }
    } catch (e) {
      state.error.value = String(e);
    } finally {
      state.loadingMore.value = false;
    }
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
      state.searchResults.value = await callTauri<CommitInfo[]>("search_commits", {
        path: state.repoPath.value,
        query,
        maxCount: 2000,
      });
      state.hasMoreSearchResults.value = false;
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
  };
}

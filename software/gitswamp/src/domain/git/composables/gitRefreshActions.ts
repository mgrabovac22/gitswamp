import type { CommitFileInfo, CommitInfo, BranchInfo, FileStatusInfo, RepoInfo, StashInfo, TagInfo } from "@/types";
import { RepositoryRefreshCoordinator, type RepositoryRefreshKind } from "@/app/refresh/RepositoryRefreshCoordinator";

import { callTauri } from "./gitCall";
import { PAGE_SIZE, type GitState } from "./gitState";
import { statusHash } from "./gitHelpers";

type RefreshCommitsOptions = {
  progressive?: boolean;
};

const INITIAL_COMMIT_REVEAL_COUNT = 36;
const COMMIT_REVEAL_FRAME_MS = 16;
const INITIAL_REVEAL_FRAME_COUNT = 5;
const COMPLETION_REVEAL_FRAME_COUNT = 8;

export function createRefreshActions(state: GitState) {
  let loadMoreDebounce: ReturnType<typeof setTimeout> | null = null;
  let loadMoreRequestId = 0;
  let commitRefreshRequestId = 0;
  let statusRequestId = 0;
  const coordinator = new RepositoryRefreshCoordinator();

  function scheduleCommitWaveFrame(callback: () => void) {
    if (typeof globalThis.requestAnimationFrame === "function") {
      globalThis.requestAnimationFrame(callback);
      return;
    }
    globalThis.setTimeout(callback, COMMIT_REVEAL_FRAME_MS);
  }

  function sameCommitRefs(left: CommitInfo, right: CommitInfo): boolean {
    if (left.refs.length !== right.refs.length) return false;
    return left.refs.every((ref, index) => ref === right.refs[index]);
  }

  function sameCommitPage(current: CommitInfo[], next: CommitInfo[]): boolean {
    if (current.length !== next.length) return false;
    return current.every((commit, index) => commit.sha === next[index]?.sha && sameCommitRefs(commit, next[index]));
  }

  function visibleCommitSlice(commits: CommitInfo[], visibleCount: number, preserveExistingPrefix: boolean): CommitInfo[] {
    const next = commits.slice(0, visibleCount);
    if (!preserveExistingPrefix) {
      return next;
    }

    const existing = state.commits.value;
    const prefixCount = Math.min(existing.length, next.length);
    for (let index = 0; index < prefixCount; index += 1) {
      if (existing[index]?.sha !== next[index]?.sha) {
        break;
      }
      next[index] = existing[index];
    }
    return next;
  }

  function revealCommitsInWave(repoPath: string, commits: CommitInfo[], requestId: number, startCount = 0): Promise<boolean> {
    if (commits.length === 0) {
      state.commits.value = [];
      return Promise.resolve(true);
    }

    let visibleCount = Math.min(startCount, commits.length);
    return new Promise((resolve) => {
      const step = () => {
        if (requestId !== commitRefreshRequestId || repoPath !== state.repoPath.value) {
          resolve(false);
          return;
        }

        const remainingCount = commits.length - visibleCount;
        const nextIncrement = startCount === 0
          ? Math.max(6, Math.ceil(commits.length / INITIAL_REVEAL_FRAME_COUNT))
          : Math.max(8, Math.ceil(remainingCount / COMPLETION_REVEAL_FRAME_COUNT));
        visibleCount = Math.min(commits.length, visibleCount + nextIncrement);
        state.commits.value = visibleCommitSlice(commits, visibleCount, startCount > 0);

        if (visibleCount >= commits.length) {
          resolve(true);
          return;
        }

        scheduleCommitWaveFrame(step);
      };

      scheduleCommitWaveFrame(step);
    });
  }

  async function completeInitialCommitPage(
    repoPath: string,
    requestId: number,
    resultPromise?: Promise<CommitInfo[]>,
  ) {
    try {
      const result = await (resultPromise ?? callTauri<CommitInfo[]>("get_commits", {
        path: repoPath,
        maxCount: PAGE_SIZE,
      }));

      if (requestId !== commitRefreshRequestId || repoPath !== state.repoPath.value) return;

      const currentCount = state.commits.value.length;
      if (result.length > currentCount) {
        await revealCommitsInWave(repoPath, result, requestId, currentCount);
      } else {
        state.commits.value = result;
      }

      if (requestId !== commitRefreshRequestId || repoPath !== state.repoPath.value) return;
      state.hasMoreCommits.value = result.length >= PAGE_SIZE;
    } catch (e) {
      if (requestId !== commitRefreshRequestId || repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
    } finally {
      if (requestId === commitRefreshRequestId && repoPath === state.repoPath.value) {
        state.commitWaveLoading.value = false;
      }
    }
  }

  async function loadCommitsToCount(targetCount: number): Promise<boolean> {
    const repoPath = state.repoPath.value;
    if (!repoPath || state.loadingMore.value) return false;

    commitRefreshRequestId += 1;
    state.commitWaveLoading.value = false;
    state.loadingMore.value = true;
    const requestId = ++loadMoreRequestId;
    try {
      const currentCount = state.commits.value.length;
      const result = await callTauri<CommitInfo[]>("get_commits", {
        path: repoPath,
        maxCount: targetCount,
      });

      if (repoPath !== state.repoPath.value) return false;

      if (result.length <= currentCount) {
        state.hasMoreCommits.value = false;
        return false;
      }

      state.commits.value = result;
      state.hasMoreCommits.value = result.length >= targetCount;
      return true;
    } catch (e) {
      if (repoPath !== state.repoPath.value) return false;
      state.error.value = String(e);
      return false;
    } finally {
      if (requestId === loadMoreRequestId) {
        state.loadingMore.value = false;
      }
    }
  }

  async function refreshCommits(options: RefreshCommitsOptions = {}) {
    const repoPath = state.repoPath.value;
    if (!repoPath) return;
    const requestId = ++commitRefreshRequestId;
    const useProgressiveWave = options.progressive === true && state.commits.value.length === 0;

    if (useProgressiveWave) {
      state.commitWaveLoading.value = true;
      try {
        const firstResult = await callTauri<CommitInfo[]>("get_commits", {
          path: repoPath,
          maxCount: INITIAL_COMMIT_REVEAL_COUNT,
          quick: true,
        });
        if (requestId !== commitRefreshRequestId || repoPath !== state.repoPath.value) return;

        state.hasMoreCommits.value = firstResult.length >= INITIAL_COMMIT_REVEAL_COUNT;
        const fullPagePromise = callTauri<CommitInfo[]>("get_commits", {
          path: repoPath,
          maxCount: PAGE_SIZE,
        });
        const revealed = await revealCommitsInWave(repoPath, firstResult, requestId);
        if (!revealed || requestId !== commitRefreshRequestId || repoPath !== state.repoPath.value) return;

        void completeInitialCommitPage(repoPath, requestId, fullPagePromise);
      } catch (e) {
        if (requestId !== commitRefreshRequestId || repoPath !== state.repoPath.value) return;
        state.error.value = String(e);
        state.commitWaveLoading.value = false;
      }
      return;
    }

    state.commitWaveLoading.value = false;
    try {
      const result = await callTauri<CommitInfo[]>("get_commits", {
        path: repoPath,
        maxCount: PAGE_SIZE,
      });
      if (requestId !== commitRefreshRequestId || repoPath !== state.repoPath.value) return;
      if (!sameCommitPage(state.commits.value, result)) {
        state.commits.value = result;
      }
      state.hasMoreCommits.value = result.length >= PAGE_SIZE;
    } catch (e) {
      if (requestId !== commitRefreshRequestId || repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
    }
  }

  function loadMoreCommits() {
    if (loadMoreDebounce) clearTimeout(loadMoreDebounce);
    loadMoreDebounce = setTimeout(() => {
      void doLoadMoreCommits();
    }, 50);
  }

  async function loadAllCommits(maxCount = 50000) {
    if (!state.repoPath.value || state.loadingMore.value) return;
    await loadCommitsToCount(maxCount);
    if (state.commits.value.length >= maxCount) {
      state.hasMoreCommits.value = false;
    }
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
    const repoPath = state.repoPath.value;
    if (!repoPath) return;
    try {
      const result = await callTauri<BranchInfo[]>("get_branches", { path: repoPath });
      if (repoPath !== state.repoPath.value) return;
      state.branches.value = result;
    } catch (e) {
      if (repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
    }
  }

  async function refreshStatus() {
    if (!state.repoPath.value) return;
    const requestId = ++statusRequestId;
    const repoPath = state.repoPath.value;
    try {
      const result = await callTauri<FileStatusInfo[]>("get_status", { path: repoPath });
      if (requestId !== statusRequestId || repoPath !== state.repoPath.value) return;
      state.fileStatuses.value = result;
      state.lastStatusHash.value = statusHash(result);
    } catch (e) {
      if (requestId !== statusRequestId || repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
    }
  }

  async function refreshStashes() {
    const repoPath = state.repoPath.value;
    if (!repoPath) return;
    try {
      const result = await callTauri<StashInfo[]>("stash_list", { path: repoPath });
      if (repoPath !== state.repoPath.value) return;
      state.stashes.value = result;
    } catch {
      // Ignore optional refresh failures.
    }
  }

  async function refreshTags() {
    const repoPath = state.repoPath.value;
    if (!repoPath) return;
    try {
      const result = await callTauri<TagInfo[]>("get_tags", { path: repoPath });
      if (repoPath !== state.repoPath.value) return;
      state.tags.value = result;
    } catch {
      // Ignore optional refresh failures.
    }
  }

  async function refreshAll() {
    const repoPath = state.repoPath.value;
    if (!repoPath) return;
    coordinator.cancel();
    state.loading.value = true;
    try {
      const repoInfo = await callTauri<RepoInfo>("get_repo_info", { path: repoPath });
      if (repoPath !== state.repoPath.value) return;
      state.repoInfo.value = repoInfo;
      await Promise.all([refreshCommits(), refreshBranches(), refreshStatus(), refreshStashes(), refreshTags()]);
    } catch (e) {
      if (repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
    } finally {
      if (repoPath === state.repoPath.value) {
        state.loading.value = false;
      }
    }
  }

  async function runCoordinatedRefresh(kinds: ReadonlySet<RepositoryRefreshKind>) {
    const tasks: Promise<void>[] = [];
    if (kinds.has("commits")) tasks.push(refreshCommits());
    if (kinds.has("branches")) tasks.push(refreshBranches());
    if (kinds.has("status")) tasks.push(refreshStatus());
    if (kinds.has("stashes")) tasks.push(refreshStashes());
    if (kinds.has("tags")) tasks.push(refreshTags());
    await Promise.all(tasks);
  }

  function requestRefresh(kinds: RepositoryRefreshKind | RepositoryRefreshKind[]) {
    coordinator.request(kinds, runCoordinatedRefresh);
  }

  function requestStatusValidation() {
    requestRefresh("status");
  }

  async function getCommitFiles(sha: string) {
    const repoPath = state.repoPath.value;
    if (!repoPath) return;
    try {
      const files = await callTauri<CommitFileInfo[]>("get_commit_files", { path: repoPath, sha });
      if (repoPath !== state.repoPath.value) return;
      state.selectedCommitFiles.value = files;
    } catch (e) {
      if (repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
      state.selectedCommitFiles.value = [];
    }
  }

  async function selectStash(stash: StashInfo | null) {
    state.selectedStash.value = stash;
    state.selectedCommit.value = null;
    state.selectedCommits.value = [];
    state.selectedCommitFiles.value = [];

    const repoPath = state.repoPath.value;
    if (!stash || !repoPath) {
      state.selectedStashFiles.value = [];
      return;
    }

    try {
      const files = await callTauri<CommitFileInfo[]>("stash_files", { path: repoPath, index: stash.index });
      if (repoPath !== state.repoPath.value || state.selectedStash.value?.index !== stash.index) return;
      state.selectedStashFiles.value = files;
    } catch (e) {
      if (repoPath !== state.repoPath.value || state.selectedStash.value?.index !== stash.index) return;
      state.error.value = String(e);
      state.selectedStashFiles.value = [];
    }
  }

  function clearStashSelection() {
    state.selectedStash.value = null;
    state.selectedStashFiles.value = [];
  }

  async function searchCommits(query: string) {
    const repoPath = state.repoPath.value;
    if (!repoPath) return;
    if (!query.trim()) {
      clearSearch();
      return;
    }

    try {
      state.searchQuery.value = query;
      const results = await callTauri<CommitInfo[]>("search_commits", {
        path: repoPath,
        query,
        maxCount: 50000,
      });
      if (repoPath !== state.repoPath.value) return;
      state.searchResults.value = results;
      state.hasMoreSearchResults.value = false;

      if (results.length > 0) {
        await ensureCommitLoaded(results[0].sha);
      }
    } catch (e) {
      if (repoPath !== state.repoPath.value) return;
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
    loadAllCommits,
    refreshBranches,
    refreshStatus,
    refreshStashes,
    refreshTags,
    refreshAll,
    requestRefresh,
    requestStatusValidation,
    getCommitFiles,
    selectStash,
    clearStashSelection,
    searchCommits,
    clearSearch,
    ensureCommitLoaded,
  };
}

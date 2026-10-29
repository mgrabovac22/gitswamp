import { useToast } from "@/shared/notifications/useToast";
import { computed, watch } from "vue";
import { createDestructiveScheduler } from "@/shared/notifications/destructiveScheduler";
import { getStoredDestructiveUndoEnabled } from "@/shared/config/undoPreferences";

import { createBranchActions } from "./composables/gitBranchActions";
import { createGhostActions } from "./composables/gitGhostActions";
import { createHistoryActions } from "./composables/gitHistoryActions";
import { createRefreshActions } from "./composables/gitRefreshActions";
import { createRemoteActions } from "./composables/gitRemoteActions";
import { createRepoActions, type OpenRepositoryOptions } from "./composables/gitRepositoryActions";
import { createGitState } from "./composables/gitState";
import { createStashActions } from "./composables/gitStashActions";
import { createStatusActions } from "./composables/gitStatusActions";
import { createTerminalActions } from "./composables/gitTerminalActions";
import { createTokenActions } from "./composables/gitTokenActions";
import { createWatcherActions } from "./composables/gitWatcherActions";

const toast = useToast();
const state = createGitState();
const destructiveContext = computed(() => JSON.stringify([
  state.repoPath.value, state.currentBranch.value, state.repoInfo.value?.head_sha,
  state.repositoryOperation.value?.kind, state.repositoryOperation.value?.message,
  state.ghostBranchState.value, state.stashes.value.map(stash => [stash.index, stash.stash_sha]),
  state.tags.value.map(tag => [tag.name, tag.sha]),
  state.fileStatuses.value.map(file => [file.path, file.status, file.staged, file.conflicted, file.file_size_bytes]),
]));
const abortScheduler = createDestructiveScheduler(() => destructiveContext.value, toast,
  getStoredDestructiveUndoEnabled, () => !state.loading.value);
watch(destructiveContext, abortScheduler.cancelPendingActions, { flush: "sync" });

function scheduleAbortOperation(kind: "merge" | "cherry-pick", afterAbort?: () => void) {
  if (state.repositoryOperation.value?.kind !== kind) return;
  abortScheduler.scheduleDestructiveAction({
    message: `Abort ${kind} in 5 seconds.`,
    detail: "Conflict resolutions will be cancelled. Click Undo to keep the operation active.",
    run: async () => {
      const aborted = kind === "merge" ? await branches.abortMerge(true) : await branches.abortCherryPick();
      if (aborted) afterAbort?.();
    },
  });
}

const refresh = createRefreshActions(state);
const watcher = createWatcherActions(state, {
  refreshCommits: refresh.refreshCommits,
  refreshBranches: refresh.refreshBranches,
});

const tokens = createTokenActions(state);
const repo = createRepoActions(
  state,
  {
    refreshCommits: refresh.refreshCommits,
    refreshBranches: refresh.refreshBranches,
    refreshStatus: refresh.refreshStatus,
    refreshStashes: refresh.refreshStashes,
    refreshTags: refresh.refreshTags,
  },
  { startFileWatcher: watcher.startFileWatcher },
);

const status = createStatusActions(
  state,
  {
    refreshStatus: refresh.refreshStatus,
    requestAbortOperation: scheduleAbortOperation,
    getDestructiveContext: () => destructiveContext.value,
    requestStatusValidation: refresh.requestStatusValidation,
    refreshCommits: refresh.refreshCommits,
    refreshBranches: refresh.refreshBranches,
    refreshStashes: refresh.refreshStashes,
    refreshRepoInfo: refresh.refreshRepoInfo,
  },
  toast,
);

const branches = createBranchActions(
  state,
  {
    refreshCommits: refresh.refreshCommits,
    refreshBranches: refresh.refreshBranches,
    refreshStatus: refresh.refreshStatus,
    refreshStashes: refresh.refreshStashes,
    refreshRepoInfo: refresh.refreshRepoInfo,
  },
  toast,
);

const ghost = createGhostActions(
  state,
  {
    refreshCommits: refresh.refreshCommits,
    refreshBranches: refresh.refreshBranches,
    refreshStatus: refresh.refreshStatus,
  },
  toast,
);

const remote = createRemoteActions(
  state,
  {
    refreshCommits: refresh.refreshCommits,
    refreshStatus: refresh.refreshStatus,
    refreshBranches: refresh.refreshBranches,
    refreshTags: refresh.refreshTags,
    refreshStashes: refresh.refreshStashes,
    refreshRepoInfo: refresh.refreshRepoInfo,
  },
  toast,
);

const stash = createStashActions(
  state,
  {
    refreshStatus: refresh.refreshStatus,
    refreshStashes: refresh.refreshStashes,
  },
  toast,
);

const history = createHistoryActions(
  state,
  {
    refreshCommits: refresh.refreshCommits,
    refreshStatus: refresh.refreshStatus,
    refreshBranches: refresh.refreshBranches,
    refreshTags: refresh.refreshTags,
  },
  toast,
);

const terminal = createTerminalActions(
  state,
  {
    refreshStatus: refresh.refreshStatus,
    refreshStashes: refresh.refreshStashes,
  },
);

async function openRepositoryWithGhost(path: string, options?: OpenRepositoryOptions) {
  const openPromise = repo.openRepository(path, options);
  if (options?.background) {
    void openPromise.finally(() => {
      if (state.repoPath.value === path) {
        void ghost.refreshGhostBranchState();
      }
    });
    return;
  }

  await openPromise;
  void ghost.refreshGhostBranchState();
}

async function refreshAllWithGhost() {
  await refresh.refreshAll();
  await ghost.refreshGhostBranchState();
}

function normalizeBranchName(name: string): string {
  return name
    .replace(/^origin\//i, "")
    .replace(/^remotes\/[a-z0-9_-]+\//i, "")
    .trim();
}

function shouldOfferGhostExitForTarget(targetBranchName?: string | null): boolean {
  const ghostState = state.ghostBranchState.value;
  if (!ghostState.active) {
    return false;
  }

  const ghostBranch = normalizeBranchName(ghostState.ghost_branch || "");
  if (!ghostBranch) {
    return false;
  }

  if (!targetBranchName) {
    return true;
  }

  return normalizeBranchName(targetBranchName) !== ghostBranch;
}

async function confirmAndExitGhostMode(actionLabel: string): Promise<boolean> {
  const ghostBranch = normalizeBranchName(state.ghostBranchState.value.ghost_branch || "");
  if (!state.ghostBranchState.value.active || !ghostBranch) {
    return true;
  }

  const message = [
    `Ghost mode is currently active on "${ghostBranch}".`,
    `If you continue, Ghost mode will be discarded before ${actionLabel}.`,
    "Do you want to continue?",
  ].join("\n");

  const confirmed = typeof globalThis.confirm === "function" ? globalThis.confirm(message) : true;
  if (!confirmed) {
    return false;
  }

  await ghost.discardGhostBranch();
  if (state.ghostBranchState.value.active) {
    toast.error("Could not disable Ghost mode. Resolve this first and retry.");
    return false;
  }

  return true;
}

async function focusHeadCommit(notifyIfMissing = true): Promise<boolean> {
  const repoPath = state.repoPath.value;
  const sha = state.repoInfo.value?.head_sha || "";
  if (!repoPath || !sha) {
    if (notifyIfMissing) toast.warning("This repository does not have a HEAD commit yet.");
    return false;
  }

  const loaded = await refresh.ensureCommitLoaded(sha);
  if (repoPath !== state.repoPath.value) return false;
  if (!loaded) {
    if (notifyIfMissing) toast.warning("HEAD could not be located in the first 50,000 commits.");
    return false;
  }

  globalThis.dispatchEvent(new CustomEvent("gitswamp-focus-head-commit", {
    detail: { repoPath, sha },
  }));
  return true;
}

async function checkoutBranchWithGhostGuard(branchName: string) {
  if (shouldOfferGhostExitForTarget(branchName)) {
    const proceed = await confirmAndExitGhostMode(`checking out "${branchName}"`);
    if (!proceed) {
      return;
    }
  }

  const checkedOut = await branches.checkoutBranch(branchName);
  if (checkedOut) {
    await focusHeadCommit();
  }
}

async function checkoutCommitWithGhostGuard(sha: string) {
  if (shouldOfferGhostExitForTarget(null)) {
    const proceed = await confirmAndExitGhostMode(`checking out commit ${sha.substring(0, 7)}`);
    if (!proceed) {
      return;
    }
  }

  const checkedOut = await history.checkoutCommit(sha);
  if (checkedOut) {
    await focusHeadCommit();
  }
}

async function deleteBranchWithGhostGuard(name: string) {
  const ghostBranch = normalizeBranchName(state.ghostBranchState.value.ghost_branch || "");
  const targetBranch = normalizeBranchName(name);
  if (targetBranch === normalizeBranchName(state.currentBranch.value)) {
    toast.warning(`Cannot delete the current branch "${name}". Check out another branch first.`);
    return false;
  }

  if (state.ghostBranchState.value.active && ghostBranch && targetBranch === ghostBranch) {
    const proceed = await confirmAndExitGhostMode(`deleting branch "${name}"`);
    if (!proceed) {
      return;
    }

    return;
  }

  return branches.deleteBranch(name);
}

void tokens.loadSavedToken();
void tokens.loadProviderTokens();
void tokens.loadGitPath();

export function useGit() {
  return {
    repoPath: state.repoPath,
    repoInfo: state.repoInfo,
    commits: state.commits,
    branches: state.branches,
    localBranches: state.localBranches,
    remoteBranches: state.remoteBranches,
    fileStatuses: state.fileStatuses,
    stagedFiles: state.stagedFiles,
    unstagedFiles: state.unstagedFiles,
    conflictFiles: state.conflictFiles,
    hasConflicts: state.hasConflicts,
    selectedCommit: state.selectedCommit,
    selectedCommits: state.selectedCommits,
    selectedCommitFiles: state.selectedCommitFiles,
    selectedStash: state.selectedStash,
    selectedStashFiles: state.selectedStashFiles,
    stashes: state.stashes,
    tags: state.tags,
    currentBranch: state.currentBranch,
    repositoryOperation: state.repositoryOperation,
    loading: state.loading,
    loadingMore: state.loadingMore,
    commitWaveLoading: state.commitWaveLoading,
    error: state.error,
    searchQuery: state.searchQuery,
    searchResults: state.searchResults,
    displayedCommits: state.displayedCommits,
    terminalOutput: state.terminalOutput,
    githubToken: state.githubToken,
    providerTokens: state.providerTokens,
    hasMoreCommits: state.hasMoreCommits,
    hasMoreSearchResults: state.hasMoreSearchResults,
    gitPath: state.gitPath,
    ghostBranchState: state.ghostBranchState,

    openRepository: openRepositoryWithGhost,
    refreshCommits: refresh.refreshCommits,
    refreshBranches: refresh.refreshBranches,
    refreshRepoInfo: refresh.refreshRepoInfo,
    refreshStatus: refresh.refreshStatus,
    refreshStashes: refresh.refreshStashes,
    refreshTags: refresh.refreshTags,
    refreshAll: refreshAllWithGhost,
    loadMoreCommits: refresh.loadMoreCommits,
    loadAllCommits: refresh.loadAllCommits,
    ensureCommitLoaded: refresh.ensureCommitLoaded,
    focusHeadCommit,
    getCommitFiles: refresh.getCommitFiles,

    stageFile: status.stageFile,
    unstageFile: status.unstageFile,
    stageAll: status.stageAll,
    unstageAll: status.unstageAll,
    commitChanges: status.commitChanges,
    restorePullSafetyStash: status.restorePullSafetyStash,
    amendLastCommit: status.amendLastCommit,
    discardFile: status.discardFile,
    discardAll: status.discardAll,
    resolveAllConflicts: status.resolveAllConflicts,
    promptResolveConflict: status.promptResolveConflict,

    checkoutBranch: checkoutBranchWithGhostGuard,
    createBranch: branches.createBranch,
    deleteBranch: deleteBranchWithGhostGuard,
    renameBranch: branches.renameBranch,
    mergeBranchIntoCurrent: branches.mergeBranchIntoCurrent,
    abortMerge: branches.abortMerge,
    scheduleAbortOperation,
    destructiveContext,
    abortCherryPick: branches.abortCherryPick,
    rebaseBranchOnto: branches.rebaseBranchOnto,
    rebaseContinue: branches.rebaseContinue,
    rebaseSkip: branches.rebaseSkip,
    rebaseAbort: branches.rebaseAbort,

    pull: remote.pull,
    push: remote.push,
    pushToMultiplePlatforms: remote.pushToMultiplePlatforms,
    checkOriginExists: remote.checkOriginExists,
    fetchAll: remote.fetchAll,
    backgroundFetchAll: remote.backgroundFetchAll,
    deleteRemoteBranch: remote.deleteRemoteBranch,
    setUpstream: remote.setUpstream,
    resetBranchToRemote: remote.resetBranchToRemote,

    stashPush: stash.stashPush,
    stashPop: stash.stashPop,
    stashApply: stash.stashApply,
    stashDrop: stash.stashDrop,
    selectStash: refresh.selectStash,
    clearStashSelection: refresh.clearStashSelection,

    cherryPick: history.cherryPick,
    revertCommit: history.revertCommit,
    resetToCommit: history.resetToCommit,
    checkoutCommit: checkoutCommitWithGhostGuard,
    createTagAt: history.createTagAt,
    deleteTag: history.deleteTag,
    editCommitMessage: history.editCommitMessage,
    createAnnotatedTag: history.createAnnotatedTag,

    cloneRepo: repo.cloneRepo,
    initRepo: repo.initRepo,
    initializeEmptyRepository: repo.initializeEmptyRepository,
    searchCommits: refresh.searchCommits,
    clearSearch: refresh.clearSearch,
    runTerminalCommand: terminal.runTerminalCommand,
    runFastScript: terminal.runFastScript,
    searchGithubRepos: repo.searchGithubRepos,
    refreshGhostBranchState: ghost.refreshGhostBranchState,
    startGhostBranch: ghost.startGhostBranch,
    materializeGhostBranch: ghost.materializeGhostBranch,
    discardGhostBranch: ghost.discardGhostBranch,

    saveToken: tokens.saveToken,
    deleteToken: tokens.deleteToken,
    saveProviderToken: tokens.saveProviderToken,
    deleteProviderToken: tokens.deleteProviderToken,
    reloadAuthTokens: tokens.reloadAuthTokens,

    startFileWatcher: watcher.startFileWatcher,
    stopFileWatcher: watcher.stopFileWatcher,
  };
}

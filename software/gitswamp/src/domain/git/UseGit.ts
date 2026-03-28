import { useToast } from "@/shared/notifications/useToast";

import { createBranchActions } from "./composables/gitBranchActions";
import { createHistoryActions } from "./composables/gitHistoryActions";
import { createRefreshActions } from "./composables/gitRefreshActions";
import { createRemoteActions } from "./composables/gitRemoteActions";
import { createRepoActions } from "./composables/gitRepositoryActions";
import { createGitState } from "./composables/gitState";
import { createStashActions } from "./composables/gitStashActions";
import { createStatusActions } from "./composables/gitStatusActions";
import { createTerminalActions } from "./composables/gitTerminalActions";
import { createTokenActions } from "./composables/gitTokenActions";
import { createWatcherActions } from "./composables/gitWatcherActions";

const toast = useToast();
const state = createGitState();

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
    refreshCommits: refresh.refreshCommits,
    refreshBranches: refresh.refreshBranches,
  },
  toast,
);

const branches = createBranchActions(
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
  },
  toast,
);

const stash = createStashActions(
  state,
  {
    refreshStatus: refresh.refreshStatus,
    refreshStashes: refresh.refreshStashes,
    refreshCommits: refresh.refreshCommits,
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

const terminal = createTerminalActions(state);

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
    selectedCommitFiles: state.selectedCommitFiles,
    selectedStash: state.selectedStash,
    selectedStashFiles: state.selectedStashFiles,
    stashes: state.stashes,
    tags: state.tags,
    currentBranch: state.currentBranch,
    loading: state.loading,
    loadingMore: state.loadingMore,
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

    openRepository: repo.openRepository,
    refreshCommits: refresh.refreshCommits,
    refreshBranches: refresh.refreshBranches,
    refreshStatus: refresh.refreshStatus,
    refreshStashes: refresh.refreshStashes,
    refreshTags: refresh.refreshTags,
    refreshAll: refresh.refreshAll,
    loadMoreCommits: refresh.loadMoreCommits,
    ensureCommitLoaded: refresh.ensureCommitLoaded,
    getCommitFiles: refresh.getCommitFiles,

    stageFile: status.stageFile,
    unstageFile: status.unstageFile,
    stageAll: status.stageAll,
    unstageAll: status.unstageAll,
    commitChanges: status.commitChanges,
    discardFile: status.discardFile,
    discardAll: status.discardAll,
    resolveAllConflicts: status.resolveAllConflicts,
    promptResolveConflict: status.promptResolveConflict,

    checkoutBranch: branches.checkoutBranch,
    createBranch: branches.createBranch,
    deleteBranch: branches.deleteBranch,
    renameBranch: branches.renameBranch,
    mergeBranchIntoCurrent: branches.mergeBranchIntoCurrent,

    pull: remote.pull,
    push: remote.push,
    pushToMultiplePlatforms: remote.pushToMultiplePlatforms,
    checkOriginExists: remote.checkOriginExists,
    fetchAll: remote.fetchAll,
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
    checkoutCommit: history.checkoutCommit,
    createTagAt: history.createTagAt,
    deleteTag: history.deleteTag,
    editCommitMessage: history.editCommitMessage,
    createAnnotatedTag: history.createAnnotatedTag,

    cloneRepo: repo.cloneRepo,
    initRepo: repo.initRepo,
    searchCommits: refresh.searchCommits,
    clearSearch: refresh.clearSearch,
    runTerminalCommand: terminal.runTerminalCommand,
    searchGithubRepos: repo.searchGithubRepos,

    saveToken: tokens.saveToken,
    deleteToken: tokens.deleteToken,
    saveProviderToken: tokens.saveProviderToken,
    deleteProviderToken: tokens.deleteProviderToken,

    startFileWatcher: watcher.startFileWatcher,
    stopFileWatcher: watcher.stopFileWatcher,
  };
}

<script setup lang="ts">
import { computed, defineAsyncComponent, ref, watch } from "vue";
import ConflictResolver from "@/shared/ui/ConflictResolver.vue";
import CommitGraph from "@/view/commit/CommitGraph.vue";
import CommitDetails from "@/view/commit/CommitDetails.vue";
import TerminalPanel from "@/view/shell/TerminalPanel.vue";
import LogsPanel from "@/view/shell/LogsPanel.vue";
import RepositorySidebar from "@/view/repository/RepositorySidebar.vue";
import { useResizableWorkspace } from "@/features/repository/workspace/useResizableWorkspace";
import { useUndoableDestructiveAction } from "@/shared/notifications/useUndoableDestructiveAction";
import ManualBisectOverlay from "@/features/repository/manual-bisect/ManualBisectOverlay.vue";
import { useManualBisect } from "@/features/repository/manual-bisect/useManualBisect";
import type {
  AmendCommitOptions,
  CommitInfo,
  StashInfo,
  IssueInfo,
  PullRequestInfo,
  GistInfo,
  LostCommitInfo,
  RemoteIssueCreatePayload,
  RemotePullRequestCreatePayload,
  RemoteLabelInfo,
  RemoteMilestoneInfo,
  RemoteUserInfo,
} from "@/types";

type HistoryViewMode = "graph" | "galaxy" | "city" | "productivity" | "time-machine" | "conflict-heatmap" | "burnout" | "remote-insights" | "conflict-resolve" | "lost-found";
type RemoteInsightsViewMode = "pull-request-detail" | "pull-request-create" | "issue-detail" | "issue-create";
type CommitSelectionPayload = { commit: CommitInfo | null; additive?: boolean };
interface RemoteCreateOptions {
  labels: RemoteLabelInfo[];
  milestones: RemoteMilestoneInfo[];
  assignees: RemoteUserInfo[];
  reviewers: RemoteUserInfo[];
}

const FileDiffViewer = defineAsyncComponent(() => import("@/shared/ui/FileDiffViewer.vue"));
const CommitGalaxyPanel = defineAsyncComponent(() => import("@/view/commit/CommitGalaxyPanel.vue"));
const RepositoryCityPanel = defineAsyncComponent(() => import("@/features/repository/city/RepositoryCityPanel.vue"));
const CommitProductivityPanel = defineAsyncComponent(() => import("@/view/commit/CommitProductivityPanel.vue"));
const CommitTimeMachinePanel = defineAsyncComponent(() => import("@/view/commit/CommitTimeMachinePanel.vue"));
const CommitConflictHeatmapPanel = defineAsyncComponent(() => import("@/view/commit/CommitConflictHeatmapPanel.vue"));
const CommitBurnoutAnalyticsPanel = defineAsyncComponent(() => import("@/view/commit/CommitBurnoutAnalyticsPanel.vue"));
const RemoteInsightsPanel = defineAsyncComponent(() => import("@/view/repository/RemoteInsightsPanel.vue"));
const LostFoundPanel = defineAsyncComponent(() => import("@/view/repository/LostFoundPanel.vue"));

const props = defineProps<{
  git: any;
  showTerminal: boolean;
  terminalAllowAll: boolean;
  openPullRequestBranches?: string[];
  issues?: IssueInfo[];
  pullRequests?: PullRequestInfo[];
  gists?: GistInfo[];
  lostCommits?: LostCommitInfo[];
  lostCommitsLoading?: boolean;
  rescuingLostCommitSha?: string | null;
  issuesHasMore?: boolean;
  pullRequestsHasMore?: boolean;
  issuesLoadingAll?: boolean;
  pullRequestsLoadingAll?: boolean;
  selectedIssue?: IssueInfo | null;
  selectedPullRequest?: PullRequestInfo | null;
  remoteInsightDetailLoading?: boolean;
  remoteCreateOptions?: RemoteCreateOptions;
  remoteCreateOptionsLoading?: boolean;
  remoteInsightsMode?: RemoteInsightsViewMode;
  showDiffViewer: boolean;
  diffFilePath: string;
  diffCommitSha: string | null;
  diffStaged: boolean;
  diffFallbackStatus?: string | null;
  diffFallbackOldPath?: string | null;
  conflictResolverPath?: string;
  detailsPanelCollapsed: boolean;
  historyViewMode: HistoryViewMode;
  timeMachineFocusSha: string | null;
  viewingWorkingChanges: boolean;
  viewingStash: boolean;
  showLogsPanel: boolean;
  appLogs: string[];
  userLogs: string[];
  errorLogs: string[];
  smartGitignoreWizardEnabled?: boolean;
  bugAutopsyEnabled?: boolean;
}>();

const emit = defineEmits<{
  "update:showTerminal": [value: boolean];
  "update:terminalAllowAll": [value: boolean];
  "update:detailsPanelCollapsed": [value: boolean];
  "update:showLogsPanel": [value: boolean];
  setHistoryView: [mode: HistoryViewMode];
  closeDiffViewer: [];
  closeConflictResolver: [];
  conflictResolved: [];
  openDiffViewer: [payload: { path: string; sha: string | null; staged: boolean; fallbackStatus?: string | null; fallbackOldPath?: string | null }];
  openConflictResolver: [filePath: string];
  selectCommit: [payload: CommitSelectionPayload];
  selectWorkingChanges: [];
  selectConflicts: [];
  selectStash: [stash: StashInfo];
  selectIssue: [issueNumber: number];
  selectPullRequest: [pullRequestNumber: number];
  loadAllIssues: [];
  loadAllPullRequests: [];
  openGist: [url: string];
  openCreateIssue: [];
  openCreatePullRequest: [];
  createIssue: [payload: RemoteIssueCreatePayload];
  createPullRequest: [payload: RemotePullRequestCreatePayload];
  refreshLostFound: [];
  rescueLostCommit: [payload: { sha: string; branchName: string }];
  requestMerge: [payload: { source: string; sourceRemote: boolean; target: string }];
  requestRebase: [payload: { source: string; sourceRemote: boolean; target: string }];
  checkoutRemoteBranch: [name: string];
  pull: [];
  push: [];
  createBranchAt: [sha: string];
  createTagAt: [sha: string];
  createAnnotatedTagAt: [sha: string];
  editCommitMessage: [sha: string];
  renameBranch: [name: string];
  deleteBranchAndRemote: [name: string];
  timeMachineBlame: [sha: string];
  createGist: [];
}>();

const hasWorkingChanges = computed(
  () => props.git.stagedFiles.value.length > 0 || props.git.unstagedFiles.value.length > 0,
);

const hasConflicts = computed(() => props.git.hasConflicts.value);
const repositoryOperation = computed(() => props.git.repositoryOperation.value);
const selectedCommitCount = computed(() => props.git.selectedCommits.value.length);
const amendModeRequested = ref(false);
const headCommit = computed<CommitInfo | null>(() => {
  const headSha = props.git.repoInfo.value?.head_sha || "";
  if (!headSha) return null;
  return props.git.commits.value.find((commit: CommitInfo) => commit.sha === headSha) ?? null;
});
const headCommitPublished = computed(() => {
  const head = headCommit.value;
  if (!head) return false;
  const remoteNames = new Set(props.git.remoteBranches.value.map((branch: { name: string }) => branch.name));
  return head.refs.some((ref: string) => remoteNames.has(ref));
});
const terminalUntrackedFileCount = computed(() =>
  props.git.unstagedFiles.value.filter((file: { status?: string; staged?: boolean; conflicted?: boolean }) => {
    const status = (file.status || "").toLowerCase();
    return !file.staged && !file.conflicted && (status === "new" || status === "added" || status === "untracked" || status === "??");
  }).length,
);
const workingFilePaths = computed(() => Array.from(new Set([
  ...props.git.stagedFiles.value.map((file: { path: string }) => file.path),
  ...props.git.unstagedFiles.value.map((file: { path: string }) => file.path),
])));

const showDetailsPanel = computed(
  () => props.historyViewMode === "graph"
    && (props.viewingWorkingChanges
      || props.viewingStash
      || selectedCommitCount.value > 0
      || props.git.selectedCommit.value !== null
      || repositoryOperation.value !== null),
);

const canAmendSelectedCommit = computed(() => {
  if (selectedCommitCount.value !== 1) return false;

  const selected = props.git.selectedCommit.value;
  const headSha = props.git.repoInfo.value?.head_sha || "";
  return !!selected && !!headSha && selected.sha === headSha;
});

watch(() => props.git.repoPath.value, () => {
  amendModeRequested.value = false;
});

watch(
  () => repositoryOperation.value?.kind || "",
  (kind) => {
    if (kind !== "merge") return;
    emit("selectWorkingChanges");
    emit("update:detailsPanelCollapsed", false);
  },
  { immediate: true },
);

const {
  sidebarWidth,
  detailsWidth,
  logsWidth,
  workspaceColumnRef,
  contentAreaStyle,
  terminalPanelStyle,
  beginResize,
} = useResizableWorkspace(() => props.showTerminal);

const { scheduleDestructiveAction } = useUndoableDestructiveAction();
const {
  session: manualBisectSession,
  busy: manualBisectBusy,
  detailsState: manualBisectDetailsState,
  remaining: manualBisectRemaining,
  currentCommit: manualBisectCurrentCommit,
  badBound: manualBisectBadBound,
  goodBound: manualBisectGoodBound,
  culprit: manualBisectCulprit,
  start: startManualBisect,
  selectGood: selectManualBisectGood,
  mark: markManualBisect,
  retryCheckout: retryManualBisectCheckout,
  cancel: cancelManualBisect,
  close: closeManualBisect,
  checkoutCulprit: checkoutManualBisectCulprit,
  returnToOriginalBranch: returnManualBisectToOriginalBranch,
} = useManualBisect({
  commits: props.git.commits,
  displayedCommits: props.git.displayedCommits,
  currentBranch: props.git.currentBranch,
  hasWorkingChanges,
  hasConflicts,
  gitError: props.git.error,
  checkoutCommit: props.git.checkoutCommit,
  checkoutBranch: props.git.checkoutBranch,
  ensureCommitLoaded: props.git.ensureCommitLoaded,
  selectCommit: (commit) => emit("selectCommit", { commit, additive: false }),
});

function toggleDetailsPanel() {
  emit("update:detailsPanelCollapsed", !props.detailsPanelCollapsed);
}

function fileNameFromPath(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() || path;
}

function scheduleDeleteBranch(name: string) {
  scheduleDestructiveAction({
    message: `Delete branch "${name}" in 5 seconds.`,
    detail: "Click Undo to keep the branch.",
    run: () => props.git.deleteBranch(name),
  });
}

function scheduleDeleteRemoteBranch(name: string) {
  scheduleDestructiveAction({
    message: `Delete remote branch "${name}" in 5 seconds.`,
    detail: "Click Undo to keep the remote branch.",
    run: () => props.git.deleteRemoteBranch(name),
  });
}

function scheduleDeleteBranchAndRemote(name: string) {
  scheduleDestructiveAction({
    message: `Delete local and remote branch "${name}" in 5 seconds.`,
    detail: "Click Undo to keep both branch refs.",
    run: () => emit("deleteBranchAndRemote", name),
  });
}

function scheduleDeleteTag(name: string) {
  scheduleDestructiveAction({
    message: `Delete tag "${name}" in 5 seconds.`,
    detail: "Click Undo to keep the tag.",
    run: () => props.git.deleteTag(name),
  });
}

function scheduleResetToCommit(sha: string, mode: "soft" | "mixed" | "hard") {
  scheduleDestructiveAction({
    message: `Reset ${mode} to ${sha.slice(0, 8)} in 5 seconds.`,
    detail: mode === "hard"
      ? "Hard reset can drop working-tree changes. Click Undo to cancel."
      : "Click Undo to keep the current branch position.",
    run: () => props.git.resetToCommit(sha, mode),
  });
}

function scheduleResetBranchToRemote(branch: string) {
  scheduleDestructiveAction({
    message: `Reset "${branch}" to remote in 5 seconds.`,
    detail: "Click Undo to keep the local branch state.",
    run: () => props.git.resetBranchToRemote(branch),
  });
}

function scheduleStashPop(index: number) {
  scheduleDestructiveAction({
    message: `Pop stash@{${index}} in 5 seconds.`,
    detail: "Click Undo to leave the stash untouched.",
    run: () => props.git.stashPop(index),
  });
}

function scheduleStashDrop(index: number) {
  scheduleDestructiveAction({
    message: `Drop stash@{${index}} in 5 seconds.`,
    detail: "Click Undo to keep the stash entry.",
    run: () => props.git.stashDrop(index),
  });
}

function scheduleDiscardFile(path: string) {
  scheduleDestructiveAction({
    message: `Discard "${fileNameFromPath(path)}" in 5 seconds.`,
    detail: "Only unstaged working-tree changes are discarded. Click Undo to cancel.",
    run: () => props.git.discardFile(path),
  });
}

function scheduleDiscardAll() {
  scheduleDestructiveAction({
    message: "Discard all unstaged changes in 5 seconds.",
    detail: "Staged changes stay staged. Click Undo to cancel.",
    run: () => props.git.discardAll(),
  });
}

async function abortCurrentMerge() {
  await props.git.abortMerge(true);
}

async function handleJumpToSearchResult(sha: string) {
  const loaded = await props.git.ensureCommitLoaded(sha);
  if (!loaded) return;

  const target = props.git.commits.value.find((commit: CommitInfo) => commit.sha === sha) ?? null;
  if (target) {
    emit("selectCommit", { commit: target, additive: false });
  }
}

async function handleAmendCommitMessage(newMessage: string) {
  const selected = props.git.selectedCommit.value;
  if (!selected) return;

  await props.git.editCommitMessage(selected.sha, newMessage);
  if (props.git.error.value) return;

  const branch = props.git.currentBranch.value;
  const remoteBranch = branch ? `origin/${branch}` : "";
  const headRef = branch ? `HEAD -> ${branch}` : "";
  const nextSelected = props.git.commits.value.find((commit: CommitInfo) =>
    branch && commit.refs.some((ref: string) => ref === branch || ref === remoteBranch || ref.includes(headRef)),
  ) ?? props.git.commits.value[0] ?? null;

  if (nextSelected) {
    emit("selectCommit", nextSelected);
  }
}

async function handleAmendCommit(options: AmendCommitOptions) {
  await props.git.amendLastCommit(options);
}

function setAmendModeFromGraph(enabled: boolean) {
  amendModeRequested.value = enabled;
  if (!enabled) return;
  emit("selectWorkingChanges");
  emit("update:detailsPanelCollapsed", false);
}

function handleRefreshState() {
  Promise.all([
    props.git.refreshStatus(),
    props.git.refreshCommits(),
    props.git.refreshBranches(),
    props.git.refreshStashes(),
    props.git.refreshRepoInfo(),
  ]).catch(() => {});
}
</script>

<template>
  <div class="relative flex-1 flex overflow-hidden">
    <div class="h-full flex-shrink-0" :style="{ width: `${sidebarWidth}px` }">
      <RepositorySidebar
        :branches="props.git.localBranches.value"
        :remote-branches="props.git.remoteBranches.value"
        :current-branch="props.git.currentBranch.value"
        :stashes="props.git.stashes.value"
        :tags="props.git.tags.value"
        :open-pull-request-branches="props.openPullRequestBranches || []"
        :issues="props.issues || []"
        :pull-requests="props.pullRequests || []"
        :gists="props.gists || []"
        :lost-commits="props.lostCommits || []"
        :issues-has-more="props.issuesHasMore"
        :pull-requests-has-more="props.pullRequestsHasMore"
        :issues-loading-all="props.issuesLoadingAll"
        :pull-requests-loading-all="props.pullRequestsLoadingAll"
        :selected-issue-number="props.selectedIssue?.number || null"
        :selected-pull-request-number="props.selectedPullRequest?.number || null"
        :remote-provider="props.git.repoInfo.value?.remotes?.[0]?.provider || 'unknown'"
        @checkout="props.git.checkoutBranch($event)"
        @checkout-remote="emit('checkoutRemoteBranch', $event)"
        @checkout-tag="props.git.checkoutCommit($event)"
        @create-branch="props.git.createBranch($event)"
        @delete-branch="scheduleDeleteBranch($event)"
        @stash-pop="scheduleStashPop($event)"
        @stash-apply="props.git.stashApply($event)"
        @stash-drop="scheduleStashDrop($event)"
        @select-issue="emit('selectIssue', $event)"
        @select-pull-request="emit('selectPullRequest', $event)"
        @load-all-issues="emit('loadAllIssues')"
        @load-all-pull-requests="emit('loadAllPullRequests')"
        @open-gist="emit('openGist', $event)"
        @open-create-issue="emit('openCreateIssue')"
        @open-create-pull-request="emit('openCreatePullRequest')"
        @create-gist="emit('createGist')"
        @open-lost-found="emit('setHistoryView', 'lost-found')"
      />
    </div>

    <div
      class="w-1.5 h-full flex-shrink-0 cursor-col-resize bg-[var(--border)]/40 hover:bg-[var(--primary)]/40 transition-colors"
      title="Resize sidebar"
      @mousedown.prevent="beginResize('sidebar', $event)"
    />

    <div ref="workspaceColumnRef" class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 flex overflow-hidden" :style="contentAreaStyle">
        <FileDiffViewer
          v-if="props.showDiffViewer"
          class="flex-1"
          :repo-path="props.git.repoPath.value"
          :file-path="props.diffFilePath"
          :commit-sha="props.diffCommitSha"
          :staged="props.diffStaged"
          :fallback-status="props.diffFallbackStatus"
          :fallback-old-path="props.diffFallbackOldPath"
          @close="emit('closeDiffViewer')"
          @refresh="props.git.refreshStatus()"
        />

        <ConflictResolver
          v-else-if="props.historyViewMode === 'conflict-resolve' && props.conflictResolverPath && props.git.repoPath.value"
          class="flex-1"
          :repo-path="props.git.repoPath.value"
          :file-path="props.conflictResolverPath"
          :embedded="true"
          @close="emit('closeConflictResolver')"
          @resolved="emit('conflictResolved')"
        />

        <CommitGraph
          v-else-if="props.historyViewMode === 'graph'"
          :class="showDetailsPanel ? '' : 'flex-1'"
          :repo-path="props.git.repoPath.value"
          :commits="props.git.displayedCommits.value"
          :selected="props.git.selectedCommit.value"
          :selected-shas="props.git.selectedCommits.value.map((commit: CommitInfo) => commit.sha)"
          :search-query="props.git.searchQuery.value"
          :search-results="props.git.searchResults.value"
          :has-working-changes="hasWorkingChanges"
          :has-conflicts="hasConflicts"
          :current-branch="props.git.currentBranch.value"
          :head-sha="props.git.repoInfo.value?.head_sha || null"
          :amend-mode-active="amendModeRequested"
          :has-more="props.git.hasMoreCommits.value"
          :commit-wave-loading="props.git.commitWaveLoading.value"
          :stashes="props.git.stashes.value"
          :tags="props.git.tags.value"
          :open-pull-request-branches="props.openPullRequestBranches || []"
          :remote-provider="props.git.repoInfo.value?.remotes?.[0]?.provider || 'unknown'"
          @select="emit('selectCommit', $event)"
          @search="props.git.searchCommits($event)"
          @clear-search="props.git.clearSearch()"
          @select-working-changes="emit('selectWorkingChanges')"
          @select-conflicts="emit('selectConflicts')"
          @load-more="props.git.loadMoreCommits()"
          @focus-head="props.git.focusHeadCommit()"
          @set-amend-mode="setAmendModeFromGraph($event)"
          @checkout="props.git.checkoutCommit($event)"
          @create-branch-at="emit('createBranchAt', $event)"
          @cherry-pick="props.git.cherryPick($event)"
          @revert="props.git.revertCommit($event)"
          @reset-soft="scheduleResetToCommit($event, 'soft')"
          @reset-mixed="scheduleResetToCommit($event, 'mixed')"
          @reset-hard="scheduleResetToCommit($event, 'hard')"
          @copy-sha="() => {}"
          @create-tag-at="emit('createTagAt', $event)"
          @create-annotated-tag-at="emit('createAnnotatedTagAt', $event)"
          @checkout-branch="props.git.checkoutBranch($event)"
          @checkout-remote-branch="emit('checkoutRemoteBranch', $event)"
          @pull="emit('pull')"
          @push="emit('push')"
          @set-upstream="(branch: string, remoteBranch: string) => props.git.setUpstream(branch, remoteBranch)"
          @edit-commit-message="emit('editCommitMessage', $event)"
          @rename-branch="emit('renameBranch', $event)"
          @delete-branch="scheduleDeleteBranch($event)"
          @delete-remote-branch="scheduleDeleteRemoteBranch($event)"
          @delete-branch-and-remote="scheduleDeleteBranchAndRemote($event)"
          @copy-branch-name="() => {}"
          @reset-branch-to-remote="scheduleResetBranchToRemote($event)"
          @delete-tag="scheduleDeleteTag($event)"
          @stash-pop="scheduleStashPop($event)"
          @stash-apply="props.git.stashApply($event)"
          @stash-drop="scheduleStashDrop($event)"
          @select-stash="emit('selectStash', $event)"
          @request-merge="emit('requestMerge', $event)"
          @request-rebase="emit('requestRebase', $event)"
          @time-machine-blame="emit('timeMachineBlame', $event)"
          @jump-to-search-result="handleJumpToSearchResult($event)"
        />

        <CommitGalaxyPanel
          v-else-if="props.historyViewMode === 'galaxy'"
          class="flex-1"
          :commits="props.git.displayedCommits.value"
          :branches="props.git.branches.value"
          :current-branch="props.git.currentBranch.value"
          :selected-sha="props.git.selectedCommit.value?.sha || null"
          :has-more="props.git.hasMoreCommits.value"
          @close="emit('setHistoryView', 'graph')"
          @load-more="props.git.loadMoreCommits()"
          @load-all="props.git.loadAllCommits()"
          @select="emit('selectCommit', $event)"
        />

        <RepositoryCityPanel
          v-else-if="props.historyViewMode === 'city'"
          class="flex-1"
          :repo-path="props.git.repoPath.value"
          :branches="props.git.branches.value"
          :current-branch="props.git.currentBranch.value"
          :working-file-paths="workingFilePaths"
          @close="emit('setHistoryView', 'graph')"
          @open-file="emit('openDiffViewer', { path: $event.path, sha: $event.sha, staged: false })"
        />

        <CommitProductivityPanel
          v-else-if="props.historyViewMode === 'productivity'"
          class="flex-1"
          :repo-path="props.git.repoPath.value"
          @close="emit('setHistoryView', 'graph')"
        />

        <CommitTimeMachinePanel
          v-else-if="props.historyViewMode === 'time-machine'"
          class="flex-1"
          :repo-path="props.git.repoPath.value"
          :focus-sha="props.timeMachineFocusSha"
          @close="emit('setHistoryView', 'graph')"
        />

        <CommitConflictHeatmapPanel
          v-else-if="props.historyViewMode === 'conflict-heatmap'"
          class="flex-1"
          :repo-path="props.git.repoPath.value"
          @close="emit('setHistoryView', 'graph')"
        />

        <CommitBurnoutAnalyticsPanel
          v-else-if="props.historyViewMode === 'burnout'"
          class="flex-1"
          :repo-path="props.git.repoPath.value"
          :commits="props.git.displayedCommits.value"
          @close="emit('setHistoryView', 'graph')"
        />

        <LostFoundPanel
          v-else-if="props.historyViewMode === 'lost-found'"
          class="flex-1"
          :repo-path="props.git.repoPath.value"
          :lost-commits="props.lostCommits || []"
          :loading="props.lostCommitsLoading"
          :rescuing-sha="props.rescuingLostCommitSha"
          @close="emit('setHistoryView', 'graph')"
          @refresh="emit('refreshLostFound')"
          @rescue="emit('rescueLostCommit', $event)"
          @open-diff="emit('openDiffViewer', { path: $event.path, sha: $event.sha, staged: false })"
        />

        <RemoteInsightsPanel
          v-else
          class="flex-1"
          :mode="props.remoteInsightsMode || 'pull-request-detail'"
          :pull-request="props.selectedPullRequest || null"
          :issue="props.selectedIssue || null"
          :detail-loading="props.remoteInsightDetailLoading"
          :create-options="props.remoteCreateOptions"
          :create-options-loading="props.remoteCreateOptionsLoading"
          :remote-branches="props.git.remoteBranches.value"
          :current-branch="props.git.currentBranch.value"
          @close="emit('setHistoryView', 'graph')"
          @create-pull-request="emit('createPullRequest', $event)"
          @create-issue="emit('createIssue', $event)"
        />

        <div
          v-if="showDetailsPanel && !props.detailsPanelCollapsed"
          class="w-1.5 h-full flex-shrink-0 cursor-col-resize bg-[var(--border)]/40 hover:bg-[var(--primary)]/40 transition-colors"
          title="Resize details panel"
          @mousedown.prevent="beginResize('details', $event)"
        />

        <div
          v-if="showDetailsPanel"
          class="relative flex h-full"
          :style="{ width: props.detailsPanelCollapsed ? '0px' : `${detailsWidth}px` }"
        >
          <button
            @click="toggleDetailsPanel"
            :class="[
              'absolute z-[400] w-5 h-7 rounded-l-lg flex items-center justify-center hover:bg-[var(--primary)]/15 transition-colors text-[var(--primary)] bg-[var(--card)] border border-r-0 border-[var(--primary)]/30 hover:border-[var(--primary)]/60',
              '-left-5',
            ]"
            :title="props.detailsPanelCollapsed ? 'Expand panel' : 'Collapse panel'"
          >
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              class="w-3.5 h-3.5 transition-transform duration-200"
              :class="props.detailsPanelCollapsed ? 'rotate-180' : ''"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
            </svg>
          </button>

          <CommitDetails
            v-model:amend-mode-requested="amendModeRequested"
            v-show="!props.detailsPanelCollapsed"
            class="h-full"
            :commit="props.git.selectedCommit.value"
            :selected-commits="props.git.selectedCommits.value"
            :can-amend-selected-commit="canAmendSelectedCommit"
            :head-commit="headCommit"
            :head-commit-published="headCommitPublished"
            :operation-busy="props.git.loading.value"
            :repository-operation="repositoryOperation"
            :staged-files="props.git.stagedFiles.value"
            :unstaged-files="props.git.unstagedFiles.value"
            :conflict-files="props.git.conflictFiles.value"
            :has-conflicts="props.git.hasConflicts.value"
            :commit-files="props.git.selectedCommitFiles.value"
            :is-working-changes="props.viewingWorkingChanges || repositoryOperation !== null"
            :is-stash="props.viewingStash"
            :selected-stash="props.git.selectedStash.value"
            :stash-files="props.git.selectedStashFiles.value"
            :repo-path="props.git.repoPath.value"
            :smart-gitignore-wizard-enabled="props.smartGitignoreWizardEnabled"
            :bug-autopsy-enabled="props.bugAutopsyEnabled"
            :manual-bisect="manualBisectDetailsState"
            @stage="props.git.stageFile($event)"
            @unstage="props.git.unstageFile($event)"
            @stage-all="props.git.stageAll()"
            @unstage-all="props.git.unstageAll()"
            @commit="props.git.commitChanges($event)"
            @abort-merge="abortCurrentMerge"
            @amend-commit="handleAmendCommit($event)"
            @discard="scheduleDiscardFile($event)"
            @discard-all="scheduleDiscardAll"
            @resolve-all-conflicts="props.git.resolveAllConflicts()"
            @resolve-conflict="props.git.promptResolveConflict($event)"
            @manual-resolve="emit('openConflictResolver', $event)"
            @stash-pop="scheduleStashPop($event)"
            @stash-apply="props.git.stashApply($event)"
            @stash-drop="scheduleStashDrop($event)"
            @amend-commit-message="handleAmendCommitMessage($event)"
            @view-diff="emit('openDiffViewer', { path: $event.path, sha: $event.sha, staged: $event.staged })"
            @refresh-state="handleRefreshState"
            @start-manual-bisect="startManualBisect($event)"
            @select-manual-bisect-good="selectManualBisectGood($event)"
          />
        </div>

        <div
          v-if="props.showLogsPanel"
          class="w-1.5 h-full flex-shrink-0 cursor-col-resize bg-[var(--border)]/40 hover:bg-[var(--primary)]/40 transition-colors"
          title="Resize logs panel"
          @mousedown.prevent="beginResize('logs', $event)"
        />

        <div
          v-if="props.showLogsPanel"
          class="h-full flex-shrink-0"
          :style="{ width: `${logsWidth}px` }"
        >
          <LogsPanel
            :app-logs="props.appLogs"
            :user-logs="props.userLogs"
            :error-logs="props.errorLogs"
            @close="emit('update:showLogsPanel', false)"
          />
        </div>
      </div>

      <div
        v-if="props.showTerminal"
        class="h-1.5 flex-shrink-0 cursor-row-resize bg-[var(--border)]/40 hover:bg-[var(--primary)]/40 transition-colors"
        title="Resize terminal"
        @mousedown.prevent="beginResize('terminal', $event)"
      />

      <TerminalPanel
        v-if="props.showTerminal"
        :output="props.git.terminalOutput.value"
        :repo-path="props.git.repoPath.value"
        :allow-all-commands="props.terminalAllowAll"
        :staged-file-count="props.git.stagedFiles.value.length"
        :unstaged-file-count="props.git.unstagedFiles.value.length"
        :untracked-file-count="terminalUntrackedFileCount"
        :conflict-file-count="props.git.conflictFiles.value.length"
        :style="terminalPanelStyle"
        @run="props.git.runTerminalCommand($event.command, $event.allowAll, { safetyStashFirst: $event.safetyStashFirst })"
        @update:allow-all-commands="emit('update:terminalAllowAll', $event)"
        @close="emit('update:showTerminal', false)"
      />
    </div>

    <ManualBisectOverlay
      v-if="manualBisectSession"
      :session="manualBisectSession"
      :busy="manualBisectBusy"
      :remaining="manualBisectRemaining"
      :current-commit="manualBisectCurrentCommit"
      :bad-bound="manualBisectBadBound"
      :good-bound="manualBisectGoodBound"
      :culprit="manualBisectCulprit"
      @cancel="cancelManualBisect"
      @retry-checkout="retryManualBisectCheckout"
      @mark="markManualBisect($event)"
      @checkout-culprit="checkoutManualBisectCulprit"
      @return-to-original-branch="returnManualBisectToOriginalBranch"
      @close="closeManualBisect"
    />
  </div>
</template>

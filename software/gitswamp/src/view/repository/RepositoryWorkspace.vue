<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import FileDiffViewer from "@/shared/ui/FileDiffViewer.vue";
import ConflictResolver from "@/shared/ui/ConflictResolver.vue";
import CommitGraph from "@/view/commit/CommitGraph.vue";
import CommitProductivityPanel from "@/view/commit/CommitProductivityPanel.vue";
import CommitTimeMachinePanel from "@/view/commit/CommitTimeMachinePanel.vue";
import CommitConflictHeatmapPanel from "@/view/commit/CommitConflictHeatmapPanel.vue";
import CommitDetails from "@/view/commit/CommitDetails.vue";
import TerminalPanel from "@/view/shell/TerminalPanel.vue";
import LogsPanel from "@/view/shell/LogsPanel.vue";
import RepositorySidebar from "@/view/repository/RepositorySidebar.vue";
import RemoteInsightsPanel from "@/view/repository/RemoteInsightsPanel.vue";
import type { CommitInfo, StashInfo, IssueInfo, PullRequestInfo } from "@/types";

type HistoryViewMode = "graph" | "productivity" | "time-machine" | "conflict-heatmap" | "remote-insights" | "conflict-resolve";
type RemoteInsightsViewMode = "pull-request-detail" | "pull-request-create" | "issue-detail" | "issue-create";
type CommitSelectionPayload = { commit: CommitInfo | null; additive?: boolean };

const props = defineProps<{
  git: any;
  showTerminal: boolean;
  terminalAllowAll: boolean;
  openPullRequestBranches?: string[];
  issues?: IssueInfo[];
  pullRequests?: PullRequestInfo[];
  selectedIssue?: IssueInfo | null;
  selectedPullRequest?: PullRequestInfo | null;
  remoteInsightsMode?: RemoteInsightsViewMode;
  showDiffViewer: boolean;
  diffFilePath: string;
  diffCommitSha: string | null;
  diffStaged: boolean;
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
  openDiffViewer: [payload: { path: string; sha: string | null; staged: boolean }];
  openConflictResolver: [filePath: string];
  selectCommit: [payload: CommitSelectionPayload];
  selectWorkingChanges: [];
  selectConflicts: [];
  selectStash: [stash: StashInfo];
  selectIssue: [issueNumber: number];
  selectPullRequest: [pullRequestNumber: number];
  openCreateIssue: [];
  openCreatePullRequest: [];
  createIssue: [payload: { title: string; description: string }];
  createPullRequest: [payload: { title: string; description: string; sourceBranch: string; targetBranch: string }];
  requestMerge: [payload: { source: string; sourceRemote: boolean; target: string }];
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
const selectedCommitCount = computed(() => props.git.selectedCommits.value.length);

const showDetailsPanel = computed(
  () => props.historyViewMode === "graph"
    && (props.viewingWorkingChanges || props.viewingStash || selectedCommitCount.value > 0 || props.git.selectedCommit.value !== null),
);

const canAmendSelectedCommit = computed(() => {
  if (selectedCommitCount.value !== 1) return false;

  const selected = props.git.selectedCommit.value;
  const branch = props.git.currentBranch.value;
  if (!selected || !branch) return false;

  const remoteBranch = `origin/${branch}`;
  const headRef = `HEAD -> ${branch}`;
  return selected.refs.some((ref: string) => ref === branch || ref === remoteBranch || ref.includes(headRef));
});

const SIDEBAR_WIDTH_KEY = "gitswamp-sidebar-width";
const DETAILS_WIDTH_KEY = "gitswamp-details-width";
const LOGS_WIDTH_KEY = "gitswamp-logs-width";
const TERMINAL_HEIGHT_KEY = "gitswamp-terminal-height";
const sidebarWidth = ref(Number(localStorage.getItem(SIDEBAR_WIDTH_KEY)) || 224);
const detailsWidth = ref(Number(localStorage.getItem(DETAILS_WIDTH_KEY)) || 320);
const logsWidth = ref(Number(localStorage.getItem(LOGS_WIDTH_KEY)) || 360);
const terminalHeight = ref(Number(localStorage.getItem(TERMINAL_HEIGHT_KEY)) || 240);
const resizeTarget = ref<"sidebar" | "details" | "logs" | "terminal" | null>(null);
const resizeStartX = ref(0);
const resizeStartWidth = ref(0);
const resizeStartY = ref(0);
const resizeStartHeight = ref(0);
const workspaceColumnRef = ref<HTMLElement | null>(null);

const contentAreaStyle = computed(() => {
  if (!props.showTerminal) return undefined;
  return { height: `calc(100% - ${terminalHeight.value}px)` };
});

const terminalPanelStyle = computed(() => ({
  height: `${terminalHeight.value}px`,
}));

function clampWidth(target: "sidebar" | "details" | "logs", width: number): number {
  if (target === "sidebar") {
    return Math.min(Math.max(width, 180), 420);
  }

  if (target === "logs") {
    return Math.min(Math.max(width, 280), 720);
  }

  return Math.min(Math.max(width, 260), 700);
}

function clampTerminalHeight(height: number): number {
  const containerHeight = workspaceColumnRef.value?.clientHeight ?? globalThis.innerHeight;
  const maxHeight = Math.max(180, Math.floor(containerHeight * 0.7));
  return Math.min(Math.max(height, 140), maxHeight);
}

function onWindowResize() {
  terminalHeight.value = clampTerminalHeight(terminalHeight.value);
}

function beginResize(target: "sidebar" | "details" | "logs" | "terminal", event: MouseEvent) {
  resizeTarget.value = target;

  if (target === "terminal") {
    resizeStartY.value = event.clientY;
    resizeStartHeight.value = terminalHeight.value;
    document.body.style.cursor = "row-resize";
  } else {
    resizeStartX.value = event.clientX;
    if (target === "sidebar") {
      resizeStartWidth.value = sidebarWidth.value;
    } else if (target === "logs") {
      resizeStartWidth.value = logsWidth.value;
    } else {
      resizeStartWidth.value = detailsWidth.value;
    }
    document.body.style.cursor = "col-resize";
  }

  document.body.style.userSelect = "none";
}

function onPointerMove(event: MouseEvent) {
  if (!resizeTarget.value) return;

  if (resizeTarget.value === "terminal") {
    const deltaY = event.clientY - resizeStartY.value;
    terminalHeight.value = clampTerminalHeight(resizeStartHeight.value - deltaY);
    return;
  }

  const deltaX = event.clientX - resizeStartX.value;
  if (resizeTarget.value === "sidebar") {
    sidebarWidth.value = clampWidth("sidebar", resizeStartWidth.value + deltaX);
  } else if (resizeTarget.value === "logs") {
    logsWidth.value = clampWidth("logs", resizeStartWidth.value - deltaX);
  } else {
    detailsWidth.value = clampWidth("details", resizeStartWidth.value - deltaX);
  }
}

function endResize() {
  if (!resizeTarget.value) return;

  if (resizeTarget.value === "terminal") {
    localStorage.setItem(TERMINAL_HEIGHT_KEY, String(terminalHeight.value));
  } else if (resizeTarget.value === "logs") {
    localStorage.setItem(LOGS_WIDTH_KEY, String(logsWidth.value));
  } else {
    localStorage.setItem(SIDEBAR_WIDTH_KEY, String(sidebarWidth.value));
    localStorage.setItem(DETAILS_WIDTH_KEY, String(detailsWidth.value));
  }

  resizeTarget.value = null;
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
}

onMounted(() => {
  terminalHeight.value = clampTerminalHeight(terminalHeight.value);
  globalThis.addEventListener("mousemove", onPointerMove);
  globalThis.addEventListener("mouseup", endResize);
  globalThis.addEventListener("resize", onWindowResize);
});

onUnmounted(() => {
  globalThis.removeEventListener("mousemove", onPointerMove);
  globalThis.removeEventListener("mouseup", endResize);
  globalThis.removeEventListener("resize", onWindowResize);
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
});

function toggleDetailsPanel() {
  emit("update:detailsPanelCollapsed", !props.detailsPanelCollapsed);
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

function handleRefreshState() {
  Promise.all([
    props.git.refreshStatus(),
    props.git.refreshCommits(),
    props.git.refreshBranches(),
  ]).catch(() => {});
}
</script>

<template>
  <div class="flex-1 flex overflow-hidden">
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
        :selected-issue-number="props.selectedIssue?.number || null"
        :selected-pull-request-number="props.selectedPullRequest?.number || null"
        :remote-provider="props.git.repoInfo.value?.remotes?.[0]?.provider || 'unknown'"
        @checkout="props.git.checkoutBranch($event)"
        @create-branch="props.git.createBranch($event)"
        @delete-branch="props.git.deleteBranch($event)"
        @stash-pop="props.git.stashPop($event)"
        @stash-apply="props.git.stashApply($event)"
        @stash-drop="props.git.stashDrop($event)"
        @select-issue="emit('selectIssue', $event)"
        @select-pull-request="emit('selectPullRequest', $event)"
        @open-create-issue="emit('openCreateIssue')"
        @open-create-pull-request="emit('openCreatePullRequest')"
        @create-gist="emit('createGist')"
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
          :has-more="props.git.hasMoreCommits.value"
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
          @checkout="props.git.checkoutCommit($event)"
          @create-branch-at="emit('createBranchAt', $event)"
          @cherry-pick="props.git.cherryPick($event)"
          @revert="props.git.revertCommit($event)"
          @reset-soft="props.git.resetToCommit($event, 'soft')"
          @reset-mixed="props.git.resetToCommit($event, 'mixed')"
          @reset-hard="props.git.resetToCommit($event, 'hard')"
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
          @delete-branch="props.git.deleteBranch($event)"
          @delete-remote-branch="props.git.deleteRemoteBranch($event)"
          @delete-branch-and-remote="emit('deleteBranchAndRemote', $event)"
          @copy-branch-name="() => {}"
          @reset-branch-to-remote="props.git.resetBranchToRemote($event)"
          @delete-tag="props.git.deleteTag($event)"
          @stash-pop="props.git.stashPop($event)"
          @stash-apply="props.git.stashApply($event)"
          @stash-drop="props.git.stashDrop($event)"
          @select-stash="emit('selectStash', $event)"
          @request-merge="emit('requestMerge', $event)"
          @time-machine-blame="emit('timeMachineBlame', $event)"
          @jump-to-search-result="handleJumpToSearchResult($event)"
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

        <RemoteInsightsPanel
          v-else
          class="flex-1"
          :mode="props.remoteInsightsMode || 'pull-request-detail'"
          :pull-request="props.selectedPullRequest || null"
          :issue="props.selectedIssue || null"
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
            v-show="!props.detailsPanelCollapsed"
            :commit="props.git.selectedCommit.value"
            :selected-commits="props.git.selectedCommits.value"
            :can-amend-selected-commit="canAmendSelectedCommit"
            :staged-files="props.git.stagedFiles.value"
            :unstaged-files="props.git.unstagedFiles.value"
            :conflict-files="props.git.conflictFiles.value"
            :has-conflicts="props.git.hasConflicts.value"
            :commit-files="props.git.selectedCommitFiles.value"
            :is-working-changes="props.viewingWorkingChanges"
            :is-stash="props.viewingStash"
            :selected-stash="props.git.selectedStash.value"
            :stash-files="props.git.selectedStashFiles.value"
            :repo-path="props.git.repoPath.value"
            @stage="props.git.stageFile($event)"
            @unstage="props.git.unstageFile($event)"
            @stage-all="props.git.stageAll()"
            @unstage-all="props.git.unstageAll()"
            @commit="props.git.commitChanges($event)"
            @discard="props.git.discardFile($event)"
            @discard-all="props.git.discardAll()"
            @resolve-all-conflicts="props.git.resolveAllConflicts()"
            @resolve-conflict="props.git.promptResolveConflict($event)"
            @manual-resolve="emit('openConflictResolver', $event)"
            @stash-pop="props.git.stashPop($event)"
            @stash-apply="props.git.stashApply($event)"
            @stash-drop="props.git.stashDrop($event)"
            @amend-commit-message="handleAmendCommitMessage($event)"
            @view-diff="emit('openDiffViewer', { path: $event.path, sha: $event.sha, staged: $event.staged })"
            @refresh-state="handleRefreshState"
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
        :style="terminalPanelStyle"
        @run="props.git.runTerminalCommand($event.command, $event.allowAll)"
        @update:allow-all-commands="emit('update:terminalAllowAll', $event)"
        @close="emit('update:showTerminal', false)"
      />
    </div>
  </div>
</template>

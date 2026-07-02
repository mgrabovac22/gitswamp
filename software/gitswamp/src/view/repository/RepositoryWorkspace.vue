<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import ConflictResolver from "@/shared/ui/ConflictResolver.vue";
import CommitGraph from "@/view/commit/CommitGraph.vue";
import CommitDetails from "@/view/commit/CommitDetails.vue";
import TerminalPanel from "@/view/shell/TerminalPanel.vue";
import LogsPanel from "@/view/shell/LogsPanel.vue";
import RepositorySidebar from "@/view/repository/RepositorySidebar.vue";
import { useResizableWorkspace } from "@/features/repository/workspace/useResizableWorkspace";
import { useUndoableDestructiveAction } from "@/shared/notifications/useUndoableDestructiveAction";
import type { CommitInfo, StashInfo, IssueInfo, PullRequestInfo } from "@/types";

type HistoryViewMode = "graph" | "galaxy" | "productivity" | "time-machine" | "conflict-heatmap" | "remote-insights" | "conflict-resolve";
type RemoteInsightsViewMode = "pull-request-detail" | "pull-request-create" | "issue-detail" | "issue-create";
type CommitSelectionPayload = { commit: CommitInfo | null; additive?: boolean };

const FileDiffViewer = defineAsyncComponent(() => import("@/shared/ui/FileDiffViewer.vue"));
const CommitGalaxyPanel = defineAsyncComponent(() => import("@/view/commit/CommitGalaxyPanel.vue"));
const CommitProductivityPanel = defineAsyncComponent(() => import("@/view/commit/CommitProductivityPanel.vue"));
const CommitTimeMachinePanel = defineAsyncComponent(() => import("@/view/commit/CommitTimeMachinePanel.vue"));
const CommitConflictHeatmapPanel = defineAsyncComponent(() => import("@/view/commit/CommitConflictHeatmapPanel.vue"));
const RemoteInsightsPanel = defineAsyncComponent(() => import("@/view/repository/RemoteInsightsPanel.vue"));

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
  smartGitignoreWizardEnabled?: boolean;
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
const selectedCommitCount = computed(() => props.git.selectedCommits.value.length);
const terminalUntrackedFileCount = computed(() =>
  props.git.unstagedFiles.value.filter((file: { status?: string; staged?: boolean; conflicted?: boolean }) => {
    const status = (file.status || "").toLowerCase();
    return !file.staged && !file.conflicted && (status === "new" || status === "added" || status === "untracked" || status === "??");
  }).length,
);

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
        @delete-branch="scheduleDeleteBranch($event)"
        @stash-pop="scheduleStashPop($event)"
        @stash-apply="props.git.stashApply($event)"
        @stash-drop="scheduleStashDrop($event)"
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
            class="h-full"
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
            :smart-gitignore-wizard-enabled="props.smartGitignoreWizardEnabled"
            @stage="props.git.stageFile($event)"
            @unstage="props.git.unstageFile($event)"
            @stage-all="props.git.stageAll()"
            @unstage-all="props.git.unstageAll()"
            @commit="props.git.commitChanges($event)"
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
  </div>
</template>

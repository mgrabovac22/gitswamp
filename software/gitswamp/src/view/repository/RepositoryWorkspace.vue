<script setup lang="ts">
import { computed } from "vue";
import FileDiffViewer from "@/shared/ui/FileDiffViewer.vue";
import CommitGraph from "@/view/commit/CommitGraph.vue";
import CommitDetails from "@/view/commit/CommitDetails.vue";
import TerminalPanel from "@/view/shell/TerminalPanel.vue";
import RepositorySidebar from "@/view/repository/RepositorySidebar.vue";
import type { CommitInfo, StashInfo } from "@/types";

const props = defineProps<{
  git: any;
  showTerminal: boolean;
  showDiffViewer: boolean;
  diffFilePath: string;
  diffCommitSha: string | null;
  diffStaged: boolean;
  detailsPanelCollapsed: boolean;
  viewingWorkingChanges: boolean;
  viewingStash: boolean;
}>();

const emit = defineEmits<{
  "update:showTerminal": [value: boolean];
  "update:detailsPanelCollapsed": [value: boolean];
  closeDiffViewer: [];
  openDiffViewer: [payload: { path: string; sha: string | null; staged: boolean }];
  openConflictResolver: [filePath: string];
  selectCommit: [commit: CommitInfo | null];
  selectWorkingChanges: [];
  selectConflicts: [];
  selectStash: [stash: StashInfo];
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
}>();

const hasWorkingChanges = computed(
  () => props.git.stagedFiles.value.length > 0 || props.git.unstagedFiles.value.length > 0,
);

const hasConflicts = computed(() => props.git.hasConflicts.value);

const showDetailsPanel = computed(
  () => props.viewingWorkingChanges || props.viewingStash || props.git.selectedCommit.value !== null,
);

function toggleDetailsPanel() {
  emit("update:detailsPanelCollapsed", !props.detailsPanelCollapsed);
}
</script>

<template>
  <div class="flex-1 flex overflow-hidden">
    <RepositorySidebar
      :branches="props.git.localBranches.value"
      :remote-branches="props.git.remoteBranches.value"
      :current-branch="props.git.currentBranch.value"
      :stashes="props.git.stashes.value"
      :tags="props.git.tags.value"
      :remote-provider="props.git.repoInfo.value?.remotes?.[0]?.provider || 'unknown'"
      @checkout="props.git.checkoutBranch($event)"
      @create-branch="props.git.createBranch($event)"
      @delete-branch="props.git.deleteBranch($event)"
      @stash-pop="props.git.stashPop($event)"
      @stash-apply="props.git.stashApply($event)"
      @stash-drop="props.git.stashDrop($event)"
    />

    <div class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 flex overflow-hidden" :style="props.showTerminal ? 'height: 75%' : ''">
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

        <CommitGraph
          v-else
          :class="showDetailsPanel ? '' : 'flex-1'"
          :commits="props.git.displayedCommits.value"
          :selected="props.git.selectedCommit.value"
          :search-query="props.git.searchQuery.value"
          :has-working-changes="hasWorkingChanges"
          :has-conflicts="hasConflicts"
          :current-branch="props.git.currentBranch.value"
          :has-more="props.git.searchQuery.value ? props.git.hasMoreSearchResults.value : props.git.hasMoreCommits.value"
          :stashes="props.git.stashes.value"
          :tags="props.git.tags.value"
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
        />

        <div v-if="showDetailsPanel" class="relative flex h-full">
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
            @view-diff="emit('openDiffViewer', { path: $event.path, sha: $event.sha, staged: $event.staged })"
          />
        </div>
      </div>

      <TerminalPanel
        v-if="props.showTerminal"
        :output="props.git.terminalOutput.value"
        :repo-path="props.git.repoPath.value"
        style="height: 25%"
        @run="props.git.runTerminalCommand($event)"
        @close="emit('update:showTerminal', false)"
      />
    </div>
  </div>
</template>

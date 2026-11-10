<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { AlertTriangle, ArrowUpRight, GitBranch, GitCommitHorizontal, Loader2, PanelBottom, RefreshCw } from "lucide-vue-next";
import CommitDetails from "@/view/commit/CommitDetails.vue";
import { useWindowedRows } from "@/shared/ui/useWindowedRows";
import { useBranchWorkspaceColumn } from "./useBranchWorkspaceColumn";
import { isOriginalWorktree, type WorkspaceLocation } from "./branchWorkspaceTypes";
import WorkspaceOpenWith from "./WorkspaceOpenWith.vue";

const props = defineProps<{ repoPath: string; branchRef: string; active: boolean }>();
const emit = defineEmits<{
  activate: []; openRepository: [location: WorkspaceLocation]; ready: [location: WorkspaceLocation];
  busyChange: [busy: boolean];
  inspect: [value: { branch: string; repoPath: string; path: string; sha?: string | null; staged?: boolean; conflict?: boolean }];
}>();
const state = useBranchWorkspaceColumn(props.repoPath, props.branchRef);
const { location, snapshot, commits, selectedCommit, commitFiles, commitFilesLoading, commitFilesError, opening,
  refreshing, historyLoading, hasMore, busy, error, historyError, staged, unstaged, conflicts, context, open, refresh,
  loadHistory, selectCommit, changeStage, commit, discard, abort, resolve } = state;
const list = ref<HTMLElement | null>(null);
const expandDetails = ref(false);
const title = computed(() => location.value?.branch ?? props.branchRef.replace(/^refs\/(heads|remotes)\//, ""));
const original = computed(() => !!location.value && isOriginalWorktree(props.repoPath, location.value));
const rowHeight = 46;
const visible = useWindowedRows(() => commits.value, list, () => rowHeight);
watch(busy, value => emit("busyChange", value), { flush: "sync" });

onMounted(async () => {
  await open();
  if (location.value) emit("ready", location.value);
});

async function retry() {
  await open();
  if (location.value) emit("ready", location.value);
}

function inspect(file: { path: string; sha?: string | null; staged?: boolean; conflict?: boolean }) {
  const workspace = location.value;
  if (!workspace || error.value || busy.value) return;
  emit("inspect", { ...file, repoPath: workspace.path, branch: workspace.branch });
}

defineExpose({ refresh, busy });
</script>

<template>
  <section class="branch-pane" :class="{ 'branch-pane--active': active, 'branch-pane--expanded': expandDetails }" :aria-label="title" @focusin="emit('activate')" @pointerdown="emit('activate')">
    <header class="pane-header">
      <GitBranch :size="16" class="branch-icon" />
      <div class="pane-title">
        <h3 :title="title">{{ title }}</h3>
        <span v-if="location" :title="location.path">{{ original ? 'Original worktree' : 'Linked worktree' }}</span>
      </div>
      <button v-if="location" type="button" class="pane-tool" :disabled="busy || refreshing" title="Refresh workspace" aria-label="Refresh workspace" @click="refresh()">
        <RefreshCw :size="14" :class="{ 'animate-spin': refreshing }" />
      </button>
      <button v-if="location" type="button" class="pane-tool" :disabled="busy" title="Open worktree in repository tab" aria-label="Open worktree in repository tab" @click="emit('openRepository', location)">
        <ArrowUpRight :size="16" />
      </button>
      <WorkspaceOpenWith v-if="location" :path="location.path" :branch="location.branch" :disabled="busy || !!error" />
    </header>

    <div v-if="error" class="pane-state" role="alert">
      <AlertTriangle :size="19" /><p>{{ error }}</p><button type="button" class="pane-retry" @click="retry">Retry</button>
    </div>
    <div v-else-if="opening || !location || !snapshot" class="pane-state" role="status"><Loader2 :size="18" class="animate-spin" />Opening worktree</div>
    <template v-else>
      <div class="pane-history">
        <h4>Commits</h4>
        <div ref="list" class="pane-commit-list">
          <div v-if="visible.before" :style="{ height: `${visible.before}px` }" aria-hidden="true" />
          <button v-for="item in visible.rows" :key="item.sha" type="button" class="pane-commit" :aria-pressed="selectedCommit?.sha === item.sha"
            :title="item.message" @click="selectCommit(item)">
            <GitCommitHorizontal :size="15" :class="{ 'branch-icon': item.sha === snapshot.headSha }" />
            <span class="pane-commit-text">
              <strong>{{ item.message.split('\n', 1)[0] || '(No commit message)' }}</strong>
              <span><code>{{ item.short_sha }}</code><span>{{ item.author_name }}</span><span>{{ item.time_ago }}</span></span>
            </span>
          </button>
          <div v-if="visible.after" :style="{ height: `${visible.after}px` }" aria-hidden="true" />
          <div v-if="historyLoading" class="history-state" role="status"><Loader2 :size="14" class="animate-spin" />Loading commits</div>
          <div v-else-if="historyError" class="history-state"><span>{{ historyError }}</span><button type="button" @click="loadHistory(!commits.length)">Retry</button></div>
          <button v-else-if="hasMore" type="button" class="pane-more" @click="loadHistory()">Load older commits</button>
        </div>
      </div>

      <div class="pane-changes-bar">
        <button type="button" class="pane-changes" :aria-pressed="!selectedCommit" @click="selectCommit(null)">
          <span>Working Changes</span>
          <span class="change-count" :aria-label="`${staged.length} staged, ${unstaged.length} unstaged`">
            <span :title="`${staged.length} staged`" class="staged-count">{{ staged.length }}</span>
            <span :title="`${unstaged.length} unstaged`">{{ unstaged.length }}</span>
            <span v-if="conflicts.length" class="conflict-count" :title="`${conflicts.length} conflicts`">{{ conflicts.length }}</span>
          </span>
        </button>
        <button type="button" class="pane-tool" :title="expandDetails ? 'Compact changes panel' : 'Expand changes panel'"
          :aria-label="expandDetails ? 'Compact changes panel' : 'Expand changes panel'" :aria-pressed="expandDetails" @click="expandDetails = !expandDetails"><PanelBottom :size="14" /></button>
      </div>
      <div class="pane-details">
        <CommitDetails
          :repo-path="location.path" :commit="selectedCommit" :commit-files="commitFiles"
          :commit-files-loading="commitFilesLoading" :commit-files-error="commitFilesError"
          :staged-files="staged" :unstaged-files="unstaged" :conflict-files="conflicts"
          :has-conflicts="!!conflicts.length" :operation-busy="busy" :destructive-context="context"
          :repository-operation="snapshot.operation" :is-working-changes="!selectedCommit" :workspace-branch="title" workspace-pane
          @stage="changeStage([$event], 'stage')" @unstage="changeStage([$event], 'unstage')"
          @stage-all="changeStage(unstaged.map(file => file.path), 'stage')"
          @unstage-all="changeStage(staged.map(file => file.path), 'unstage')"
          @discard="discard([$event])" @discard-all="discard(unstaged.map(file => file.path))"
          @commit="commit" @abort-merge="abort('merge')" @abort-cherry-pick="abort('cherry-pick')"
          @resolve-conflict="resolve($event)" @resolve-all-conflicts="resolve()"
          @manual-resolve="inspect({ path: $event, conflict: true })" @view-diff="inspect($event)" @refresh-state="refresh(true)"
        />
      </div>
    </template>
  </section>
</template>

<style scoped>
.branch-pane { min-width: 0; min-height: 0; display: flex; flex-direction: column; border: 1px solid var(--border); border-radius: 7px; background: var(--card); overflow: hidden; }
.branch-pane--active { border-color: color-mix(in srgb, var(--primary) 55%, var(--border)); }
.pane-header { display: flex; align-items: center; gap: 5px; padding: 6px 7px; min-height: 46px; border-bottom: 1px solid var(--border); }
.pane-header svg, .pane-commit > svg { flex-shrink: 0; }
.branch-icon { color: var(--primary); }
.pane-title { min-width: 0; flex: 1; }
.pane-title h3, .pane-title span { display: block; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.pane-title h3 { font-size: 13px; font-weight: 650; }
.pane-title span { font-size: 10px; color: var(--muted-foreground); }
.pane-tool { width: 27px; height: 27px; display: grid; place-items: center; flex: 0 0 27px; border-radius: 4px; color: var(--muted-foreground); }
.pane-tool:not(:disabled):hover, .pane-tool:focus-visible { background: var(--accent); color: var(--foreground); }
.pane-tool:disabled { opacity: .55; }
.pane-history { flex: 1 1 auto; min-height: 77px; display: flex; flex-direction: column; }
.pane-history h4 { padding: 5px 8px; color: var(--muted-foreground); font-size: 11px; font-weight: 650; border-bottom: 1px solid var(--border); }
.pane-commit-list { min-height: 0; flex: 1; overflow: auto; }
.pane-commit { width: 100%; height: 46px; padding: 4px 8px; display: flex; align-items: center; gap: 6px; text-align: left; border-bottom: 1px solid var(--border); color: var(--foreground); }
.pane-commit:hover, .pane-commit[aria-pressed="true"] { background: var(--secondary); }
.pane-commit-text { display: grid; min-width: 0; gap: 2px; }
.pane-commit-text strong, .pane-commit-text span { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.pane-commit-text strong { font-size: 12px; font-weight: 550; }
.pane-commit-text > span { display: flex; align-items: center; gap: 6px; font-size: 10px; color: var(--muted-foreground); }
.pane-commit-text code { flex: none; }
.pane-commit-text > span > span:last-child { flex-shrink: 0; }
.pane-changes-bar { display: flex; align-items: center; padding-inline: 3px; border-block: 1px solid var(--border); background: var(--secondary); }
.pane-changes { display: flex; flex: 1; align-items: center; justify-content: space-between; gap: 5px; min-width: 0; padding: 4px 5px; font-size: 11px; font-weight: 650; }
.pane-changes[aria-pressed="true"], .pane-changes:hover { color: var(--primary); }
.change-count { display: flex; gap: 4px; color: var(--muted-foreground); font: 10px monospace; }
.change-count > span { padding-inline: 4px; border-radius: 3px; background: var(--background); }
.change-count .staged-count { color: var(--chart-5); }
.change-count .conflict-count { color: var(--destructive); }
.pane-details { min-height: 0; flex: 0 0 clamp(218px, 41%, 336px); isolation: isolate; overflow: hidden; }
.branch-pane--expanded .pane-details { flex-basis: min(68%, calc(100% - 155px)); }
.pane-details :deep(> div) { border-left: 0; }
.pane-state { display: grid; justify-items: center; align-content: center; gap: 9px; padding: 14px; flex: 1; font-size: 12px; text-align: center; overflow-wrap: anywhere; }
.history-state { display: flex; justify-content: center; align-items: center; flex-wrap: wrap; gap: 5px; padding: 8px; font-size: 11px; overflow-wrap: anywhere; }
.pane-more, .pane-retry, .history-state button { font-size: 11px; color: var(--primary); padding: 5px 8px; }
</style>

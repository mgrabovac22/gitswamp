<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import type { CommitInfo, StashInfo, TagInfo } from "@/types";
import {
  AUTHOR_COL,
  BRANCH_COL,
  GRAPH_COLORS,
  NODE_RADIUS,
  SHA_COL,
} from "./graph/graph.constants";
import {
  avatarSvg as buildAvatarSvg,
  isMergeCommit,
  mergeDotSvg as buildMergeDotSvg,
  providerIconSvg,
} from "./graph/graph.utils";
import { useCommitGraphLayout, type DisplayRef, type MergedRef } from "./graph/useCommitGraphLayout";
import { useCompactRowHeight } from "./graph/useCompactRowHeight";
import { useThemeMode } from "./graph/useThemeMode";

const { rowHeight } = useCompactRowHeight();
const { svgBgOuter, svgBgInner } = useThemeMode();

const props = defineProps<{
  commits: CommitInfo[];
  selected: CommitInfo | null;
  searchQuery?: string;
  hasWorkingChanges: boolean;
  hasConflicts?: boolean;
  currentBranch: string;
  hasMore?: boolean;
  stashes?: StashInfo[];
  tags?: TagInfo[];
  remoteProvider?: 'github' | 'gitlab' | 'bitbucket' | 'azure' | 'unknown';
}>();

const emit = defineEmits<{
  select: [commit: CommitInfo | null];
  search: [query: string];
  clearSearch: [];
  selectWorkingChanges: [];
  selectConflicts: [];
  loadMore: [];
  checkout: [sha: string];
  createBranchAt: [sha: string];
  cherryPick: [sha: string];
  revert: [sha: string];
  resetSoft: [sha: string];
  resetMixed: [sha: string];
  resetHard: [sha: string];
  copySha: [sha: string];
  createTagAt: [sha: string];
  createAnnotatedTagAt: [sha: string];
  checkoutBranch: [name: string];
  checkoutRemoteBranch: [name: string];
  pull: [];
  push: [];
  setUpstream: [branch: string, remoteBranch: string];
  editCommitMessage: [sha: string];
  renameBranch: [oldName: string];
  deleteBranch: [name: string];
  deleteRemoteBranch: [name: string];
  deleteBranchAndRemote: [name: string];
  copyBranchName: [name: string];
  resetBranchToRemote: [branch: string];
  deleteTag: [name: string];
  stashPop: [index: number];
  stashApply: [index: number];
  stashDrop: [index: number];
  selectStash: [stash: StashInfo];
  requestMerge: [payload: { source: string; sourceRemote: boolean; target: string }];
}>();

const searchInput = ref("");
const scrollContainer = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const viewportHeight = ref(600);
const hoveredRefRow = ref<number | null>(null);
const hoveredStashRow = ref<number | null>(null);

const ctxVisible = ref(false);
const ctxX = ref(0);
const ctxY = ref(0);
const ctxCommit = ref<CommitInfo | null>(null);
const ctxResetSub = ref(false);
const refCtxVisible = ref(false);
const refCtxX = ref(0);
const refCtxY = ref(0);
const refCtxRef = ref<DisplayRef | null>(null);
const stashCtxVisible = ref(false);
const stashCtxX = ref(0);
const stashCtxY = ref(0);
const stashCtxItem = ref<(StashInfo & { parentIdx: number; lane: number; offsetIdx: number }) | null>(null);
const stashCtxItems = ref<(StashInfo & { parentIdx: number; lane: number; offsetIdx: number })[]>([]);
const dragBranch = ref<{ name: string; remote: boolean } | null>(null);

function avatarSvg(name: string, cx: number, cy: number, r: number, branchColor: string): string {
  return buildAvatarSvg(
    name,
    cx,
    cy,
    r,
    branchColor,
    svgBgOuter.value,
    svgBgInner.value,
    GRAPH_COLORS,
  );
}

function mergeDotSvg(cx: number, cy: number, color: string): string {
  return buildMergeDotSvg(cx, cy, color, svgBgOuter.value);
}

const {
  graph,
  graphWidth,
  wcOffset,
  totalH,
  visibleNodes,
  visibleEdges,
  stashesAtCommit,
  lx,
  ry,
  ep,
  wcEdge,
  conflictEdge,
  conflictSpineEdge,
  wcLaneX,
  mergedRefs,
  displayRefs,
  topDisplayRef,
  extraDisplayRefCount,
} = useCommitGraphLayout(props, rowHeight, scrollTop, viewportHeight);

let st: ReturnType<typeof setTimeout> | null = null;
function onSearch() {
  if (st) clearTimeout(st);
  st = setTimeout(() => {
    if (searchInput.value.trim()) emit("search", searchInput.value.trim());
    else emit("clearSearch");
  }, 300);
}
function clearSearch() { searchInput.value = ""; emit("clearSearch"); }

function onScroll(e: Event) {
  const el = e.target as HTMLElement;
  scrollTop.value = el.scrollTop;
  viewportHeight.value = el.clientHeight;
  if (props.hasMore && el.scrollTop + el.clientHeight >= el.scrollHeight - 200) {
    emit("loadMore");
  }
}

function onRefDblClick(ref: DisplayRef, commit: CommitInfo) {
  if (ref.kind === "tag") {
    emit("checkout", commit.sha);
    return;
  }
  if (ref.local) {
    emit("checkoutBranch", ref.name);
  } else {
    emit("checkoutRemoteBranch", ref.name);
  }
}

function onRefContextMenu(event: MouseEvent, ref: DisplayRef) {
  event.preventDefault();
  event.stopPropagation();
  refCtxRef.value = ref;
  const menuWidth = 200;
  const menuHeight = 160;
  let x = event.clientX;
  let y = event.clientY;
  if (x + menuWidth > window.innerWidth) {
    x = window.innerWidth - menuWidth - 8;
  }
  if (y + menuHeight > window.innerHeight) {
    y = window.innerHeight - menuHeight - 8;
  }
  refCtxX.value = x;
  refCtxY.value = y;
  refCtxVisible.value = true;
  ctxVisible.value = false;
}

function canMergeRefIntoCurrent(ref: DisplayRef): boolean {
  if (ref.kind !== "branch") return false;
  if (ref.local && ref.name === props.currentBranch) return false;
  return true;
}

function onStashContextMenu(event: MouseEvent, stash: StashInfo & { parentIdx: number; lane: number; offsetIdx: number }) {
  event.preventDefault();
  event.stopPropagation();
  stashCtxItem.value = stash;
  stashCtxItems.value = [];
  const menuWidth = 180;
  const menuHeight = 140;
  let x = event.clientX;
  let y = event.clientY;
  if (x + menuWidth > window.innerWidth) {
    x = window.innerWidth - menuWidth - 8;
  }
  if (y + menuHeight > window.innerHeight) {
    y = window.innerHeight - menuHeight - 8;
  }
  stashCtxX.value = x;
  stashCtxY.value = y;
  stashCtxVisible.value = true;
  ctxVisible.value = false;
  refCtxVisible.value = false;
}

function onStashBadgeContextMenu(event: MouseEvent, stashes: (StashInfo & { parentIdx: number; lane: number; offsetIdx: number })[]) {
  event.preventDefault();
  event.stopPropagation();
  if (stashes.length === 1) {
    onStashContextMenu(event, stashes[0]);
    return;
  }
  stashCtxItems.value = stashes;
  stashCtxItem.value = null;
  const menuWidth = 220;
  const menuHeight = 60 + stashes.length * 72;
  let x = event.clientX;
  let y = event.clientY;
  if (x + menuWidth > window.innerWidth) x = window.innerWidth - menuWidth - 8;
  if (y + menuHeight > window.innerHeight) y = Math.max(8, window.innerHeight - menuHeight - 8);
  stashCtxX.value = x;
  stashCtxY.value = y;
  stashCtxVisible.value = true;
  ctxVisible.value = false;
  refCtxVisible.value = false;
}

function closeStashCtx() {
  stashCtxVisible.value = false;
  stashCtxItem.value = null;
  stashCtxItems.value = [];
}

function stashAction(action: "pop" | "apply" | "drop" | "view", item?: StashInfo & { parentIdx: number; lane: number; offsetIdx: number }) {
  const target = item ?? stashCtxItem.value;
  if (!target) return;
  const idx = target.index;
  closeStashCtx();
  if (action === "pop") emit("stashPop", idx);
  if (action === "apply") emit("stashApply", idx);
  if (action === "drop") emit("stashDrop", idx);
  if (action === "view") emit("selectStash", target);
}

function closeRefCtx() {
  refCtxVisible.value = false;
  refCtxRef.value = null;
}

function refAction(action: string) {
  if (!refCtxRef.value) return;
  const r = refCtxRef.value;
  closeRefCtx();
  switch (action) {
    case "checkout-local":
      emit("checkoutBranch", r.name);
      break;
    case "checkout-remote":
      emit("checkoutRemoteBranch", r.name);
      break;
    case "delete-local":
      emit("deleteBranch", r.name);
      break;
    case "delete-remote":
      emit("deleteRemoteBranch", r.name);
      break;
    case "copy-name":
      navigator.clipboard.writeText(r.name).catch(() => {});
      break;
    case "merge-into-current":
      emit("requestMerge", { source: r.name, sourceRemote: !!r.remote && !r.local, target: props.currentBranch });
      break;
    case "delete-tag":
      emit("deleteTag", r.name);
      break;
  }
}

function onBranchDragStart(event: DragEvent, ref: DisplayRef) {
  if (ref.kind !== "branch") return;
  dragBranch.value = { name: ref.name, remote: !!ref.remote && !ref.local };
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", ref.name);
  }
}

function onBranchDragOver(event: DragEvent, ref: DisplayRef) {
  if (ref.kind !== "branch") return;
  event.preventDefault();
}

function onBranchDrop(event: DragEvent, ref: DisplayRef) {
  if (ref.kind !== "branch") return;
  event.preventDefault();
  const dragged = dragBranch.value;
  dragBranch.value = null;
  if (!dragged || dragged.name === ref.name) return;
  emit("requestMerge", { source: dragged.name, sourceRemote: dragged.remote, target: ref.name });
}

function ctxHasBranch(): boolean {
  if (!ctxCommit.value) return false;
  return mergedRefs(ctxCommit.value).length > 0;
}

function ctxBranchName(): string {
  if (!ctxCommit.value) return "";
  const refs = mergedRefs(ctxCommit.value);
  return refs.length > 0 ? refs[0].name : "";
}

function ctxBranchRef(): MergedRef | null {
  if (!ctxCommit.value) return null;
  const refs = mergedRefs(ctxCommit.value);
  return refs.length > 0 ? refs[0] : null;
}

function ctxIsHeadCommit(): boolean {
  if (!ctxCommit.value) return false;
  const refs = mergedRefs(ctxCommit.value);
  return refs.some(r => r.name === props.currentBranch);
}

function onCtx(e: MouseEvent, commit: CommitInfo) {
  e.preventDefault();
  ctxCommit.value = commit;
  const menuWidth = 260;
  const menuMaxHeight = window.innerHeight * 0.8;
  let x = e.clientX;
  let y = e.clientY;
  if (x + menuWidth > window.innerWidth) {
    x = window.innerWidth - menuWidth - 8;
  }
  if (y + menuMaxHeight > window.innerHeight) {
    y = Math.max(8, window.innerHeight - menuMaxHeight - 8);
  }
  ctxX.value = x;
  ctxY.value = y;
  ctxResetSub.value = false;
  ctxVisible.value = true;
}

function closeCtx() {
  ctxVisible.value = false;
  ctxResetSub.value = false;
  closeRefCtx();
  closeStashCtx();
}

function ctxAction(action: string) {
  if (!ctxCommit.value) return;
  const sha = ctxCommit.value.sha;
  const branch = ctxBranchName();
  closeCtx();
  switch (action) {
    case "checkout": emit("checkout", sha); break;
    case "branch": emit("createBranchAt", sha); break;
    case "cherry-pick": emit("cherryPick", sha); break;
    case "revert": emit("revert", sha); break;
    case "reset-soft": emit("resetSoft", sha); break;
    case "reset-mixed": emit("resetMixed", sha); break;
    case "reset-hard": emit("resetHard", sha); break;
    case "copy-sha":
      navigator.clipboard.writeText(sha).catch(() => {});
      emit("copySha", sha);
      break;
    case "tag": emit("createTagAt", sha); break;
    case "annotated-tag": emit("createAnnotatedTagAt", sha); break;
    case "pull": emit("pull"); break;
    case "push": emit("push"); break;
    case "set-upstream":
      if (branch) emit("setUpstream", branch, "origin/" + branch);
      break;
    case "checkout-branch":
      if (branch) emit("checkoutBranch", branch);
      break;
    case "edit-message": emit("editCommitMessage", sha); break;
    case "rename-branch":
      if (branch) emit("renameBranch", branch);
      break;
    case "delete-branch":
      if (branch) emit("deleteBranch", branch);
      break;
    case "delete-remote-branch":
      if (branch) emit("deleteRemoteBranch", branch);
      break;
    case "delete-both":
      if (branch) emit("deleteBranchAndRemote", branch);
      break;
    case "copy-branch-name":
      if (branch) {
        navigator.clipboard.writeText(branch).catch(() => {});
        emit("copyBranchName", branch);
      }
      break;
    case "reset-to-remote":
      if (branch) emit("resetBranchToRemote", branch);
      break;
  }
}

function topRefStyle(commit: CommitInfo, color: string): Record<string, string> {
  const ref = topDisplayRef(commit);
  if (!ref) return {};
  if (ref.kind === 'tag') {
    return { backgroundColor: '#f59e0b2a', color: '#f59e0b', border: '1.5px solid #f59e0b66' };
  }
  if (ref.local && ref.name === props.currentBranch) {
    return { backgroundColor: color + '35', color };
  }
  return { backgroundColor: color + '28', color, border: '1.5px solid ' + color + '55' };
}

const providerIconMarkup = computed(() => providerIconSvg(props.remoteProvider));

function onDocClick() { closeCtx(); }
onMounted(() => {
  document.addEventListener("click", onDocClick);
  if (scrollContainer.value) {
    viewportHeight.value = scrollContainer.value.clientHeight;
  }
});
onUnmounted(() => {
  document.removeEventListener("click", onDocClick);
});
</script>

<template>
  <div class="flex-1 bg-[var(--background)] flex flex-col overflow-hidden min-w-0">
    <div class="flex-shrink-0 border-b border-[var(--border)] bg-[var(--background)]/95 backdrop-blur-sm z-10">
      <div class="px-3 py-1.5 flex items-center gap-2">
        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-[var(--muted-foreground)] flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
        <input v-model="searchInput" @input="onSearch" placeholder="Search commits, messages, authors, SHA..." class="flex-1 bg-transparent text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none" />
        <button v-if="searchInput" @click="clearSearch" class="p-0.5 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
        <span v-if="searchQuery" class="text-[10px] text-[var(--muted-foreground)]">{{ commits.length }} results</span>
      </div>
      <div class="flex items-center py-0.5 text-[9px] text-[var(--muted-foreground)] uppercase tracking-wider font-medium border-t border-[var(--border)]">
        <div class="flex-shrink-0 px-2 text-right" :style="{ width: BRANCH_COL + 'px' }">Branch / Tag</div>
        <div class="flex-shrink-0 text-center" :style="{ width: graphWidth + 'px' }">Graph</div>
        <div class="flex-1 px-3">Commit Message</div>
        <div class="flex-shrink-0 px-1" :style="{ width: AUTHOR_COL + 'px' }">Author</div>
        <div class="flex-shrink-0 px-1" :style="{ width: SHA_COL + 'px' }">SHA</div>
      </div>
    </div>

    <div v-if="!commits.length && !hasWorkingChanges" class="flex-1 flex items-center justify-center text-sm text-[var(--muted-foreground)]">
      No commits to display
    </div>

    <div v-else ref="scrollContainer" class="commit-scroll flex-1 overflow-y-auto min-h-0" @scroll="onScroll">
      <div class="relative" :style="{ height: totalH + 'px' }">
        <svg
          class="absolute top-0"
          style="pointer-events: none;"
          :style="{ left: BRANCH_COL + 'px', width: graphWidth + 'px', height: totalH + 'px' }"
        >
          <path
            v-if="hasWorkingChanges && graph.nodes.length > 0"
            :d="wcEdge()"
            stroke="#8b5cf6" stroke-width="2" fill="none" opacity="0.4"
            stroke-dasharray="4 3" stroke-linecap="round"
          />
          <path
            v-if="hasConflicts && graph.nodes.length > 0"
            :d="conflictEdge()"
            stroke="#ef4444" stroke-width="2" fill="none" opacity="0.55"
            stroke-dasharray="5 3" stroke-linecap="round"
          />
          <path
            v-if="hasConflicts && hasWorkingChanges"
            :d="conflictSpineEdge()"
            stroke="#ef4444" stroke-width="2" fill="none" opacity="0.55"
            stroke-dasharray="5 3" stroke-linecap="round"
          />
          <path
            v-for="(edge, i) in visibleEdges"
            :key="'e' + edge.fromIndex + '-' + edge.toIndex + '-' + i"
            :d="ep(edge)"
            :stroke="edge.color"
            stroke-width="2" fill="none" opacity="0.7"
            stroke-linecap="round" stroke-linejoin="round"
          />
          <rect
            v-if="hasWorkingChanges"
            :x="wcLaneX() - 7" :y="rowHeight / 2 - 7"
            width="14" height="14" rx="2"
            fill="rgba(139, 92, 246, 0.14)"
            stroke="#8b5cf6"
            stroke-width="1.8"
            stroke-dasharray="4 3"
            class="working-node"
          />
          <rect
            v-if="hasConflicts"
            :x="wcLaneX() + 15"
            :y="((hasWorkingChanges ? 1 : 0) * rowHeight) + rowHeight / 2 - 7"
            width="14" height="14" rx="2"
            fill="rgba(239, 68, 68, 0.18)"
            stroke="#ef4444"
            stroke-width="1.8"
            stroke-dasharray="3 2"
          />
          <g
            v-for="item in visibleNodes"
            :key="'n' + item.node.commit.sha"
            v-html="isMergeCommit(item.node.commit) ? mergeDotSvg(lx(item.node.lane), ry(item.idx), item.node.color) : avatarSvg(item.node.commit.author_name, lx(item.node.lane), ry(item.idx), NODE_RADIUS, item.node.color)"
            class="node-pop"
          />

        </svg>

        <div
          v-if="hasWorkingChanges"
          class="absolute left-0 right-0 flex items-center cursor-pointer transition-colors"
          :class="selected === null ? 'bg-[var(--primary)]/10' : 'hover:bg-[var(--secondary)]'"
          :style="{ top: '0px', height: rowHeight + 'px' }"
          @click="emit('selectWorkingChanges')"
        >
          <div class="flex-shrink-0" :style="{ width: BRANCH_COL + 'px' }" />
          <div class="flex-shrink-0" :style="{ width: graphWidth + 'px' }" />
          <div class="flex-1 flex items-center px-3 min-w-0">
            <span class="px-1.5 py-0.5 rounded text-[9px] font-semibold bg-[var(--primary)]/20 text-[var(--primary)] border border-[var(--primary)]/30">
              ● Working Changes
            </span>
          </div>
          <div class="flex-shrink-0 text-[10px] text-[var(--muted-foreground)] px-1" :style="{ width: AUTHOR_COL + 'px' }">—</div>
          <div class="flex-shrink-0 text-[9px] font-mono text-[var(--muted-foreground)] px-1" :style="{ width: SHA_COL + 'px' }">—</div>
        </div>

        <div
          v-if="hasConflicts"
          class="absolute left-0 right-0 flex items-center cursor-pointer transition-colors"
          :class="selected === null ? 'bg-[#ef4444]/10' : 'hover:bg-[var(--secondary)]'"
          :style="{ top: ((hasWorkingChanges ? 1 : 0) * rowHeight) + 'px', height: rowHeight + 'px' }"
          @click="emit('selectConflicts')"
        >
          <div class="flex-shrink-0" :style="{ width: BRANCH_COL + 'px' }" />
          <div class="flex-shrink-0" :style="{ width: graphWidth + 'px' }" />
          <div class="flex-1 flex items-center px-3 min-w-0">
            <span class="px-1.5 py-0.5 rounded text-[9px] font-semibold bg-[#ef4444]/15 text-[#ef4444] border border-[#ef4444]/35">
              ● Conflicts
            </span>
          </div>
          <div class="flex-shrink-0 text-[10px] text-[var(--muted-foreground)] px-1" :style="{ width: AUTHOR_COL + 'px' }">—</div>
          <div class="flex-shrink-0 text-[9px] font-mono text-[var(--muted-foreground)] px-1" :style="{ width: SHA_COL + 'px' }">—</div>
        </div>

        <div
          v-for="item in visibleNodes"
          :key="item.node.commit.sha"
          class="absolute left-0 right-0 flex items-center cursor-pointer transition-colors graph-row"
          :class="selected?.sha === item.node.commit.sha ? 'bg-[var(--primary)]/10' : 'hover:bg-[var(--secondary)]'"
          :style="{ top: (item.idx * rowHeight + wcOffset) + 'px', height: rowHeight + 'px' }"
          @click="emit('select', item.node.commit)"
          @contextmenu="onCtx($event, item.node.commit)"
        >
          <div class="flex-shrink-0 flex items-center justify-start gap-0.5 px-1 relative" :style="{ width: BRANCH_COL + 'px' }"
            @mouseenter="hoveredRefRow = item.idx" @mouseleave="hoveredRefRow = null; hoveredStashRow = null"
          >
            <template v-if="topDisplayRef(item.node.commit)">
              <span
                class="flex-shrink-0 flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[9px] font-semibold truncate max-w-[110px] cursor-pointer shadow-sm"
                :class="{
                  'current-branch-badge': topDisplayRef(item.node.commit)?.kind === 'branch'
                    && topDisplayRef(item.node.commit)?.local
                    && topDisplayRef(item.node.commit)?.name === currentBranch
                }"
                :style="topRefStyle(item.node.commit, item.node.color)"
                :title="topDisplayRef(item.node.commit)?.name || ''"
                :draggable="topDisplayRef(item.node.commit)?.kind === 'branch'"
                @dblclick.stop="onRefDblClick(topDisplayRef(item.node.commit)!, item.node.commit)"
                @dragstart.stop="onBranchDragStart($event, topDisplayRef(item.node.commit)!)"
                @dragover.stop.prevent="onBranchDragOver($event, topDisplayRef(item.node.commit)!)"
                @drop.stop.prevent="onBranchDrop($event, topDisplayRef(item.node.commit)!)"
                @contextmenu.stop.prevent="onRefContextMenu($event, topDisplayRef(item.node.commit)!)"
              >
                <svg v-if="topDisplayRef(item.node.commit)?.kind === 'branch' && topDisplayRef(item.node.commit)?.local" class="w-2.5 h-2.5 flex-shrink-0 opacity-70" viewBox="0 0 16 16" fill="currentColor"><rect x="2" y="4" width="12" height="8" rx="1.5" /><rect x="4" y="12" width="8" height="1.5" rx="0.5" opacity="0.6"/><rect x="6" y="13.5" width="4" height="1" rx="0.5" opacity="0.4"/></svg>
                <span v-if="topDisplayRef(item.node.commit)?.kind === 'branch' && topDisplayRef(item.node.commit)?.remote" v-html="providerIconMarkup" />
                <svg v-if="topDisplayRef(item.node.commit)?.kind === 'tag'" class="w-2.5 h-2.5 flex-shrink-0 opacity-80" viewBox="0 0 16 16" fill="currentColor"><path d="M2 8.2V3.5C2 2.7 2.7 2 3.5 2h4.7c.4 0 .8.2 1.1.4l4.3 4.3c.6.6.6 1.6 0 2.1l-4.9 4.9c-.6.6-1.6.6-2.1 0L2.4 9.3C2.1 9 2 8.6 2 8.2zm4.2-3.5a1.2 1.2 0 100 2.4 1.2 1.2 0 000-2.4z"/></svg>
                <span class="truncate">{{ topDisplayRef(item.node.commit)?.name }}</span>
              </span>
              <span
                v-if="extraDisplayRefCount(item.node.commit) > 0"
                class="flex-shrink-0 px-1 py-0.5 rounded-full text-[8px] font-bold shadow-sm cursor-pointer hover:scale-110 transition-transform"
                :style="{ backgroundColor: item.node.color + '30', color: item.node.color, border: '1px solid ' + item.node.color + '55' }"
              >+{{ extraDisplayRefCount(item.node.commit) }}</span>
            </template>

            <template v-if="stashesAtCommit(item.idx).length > 0">
              <span
                class="stash-badge flex-shrink-0 flex items-center gap-1 px-1.5 py-0.5 rounded-md cursor-default select-none"
                :class="{ 'stash-compact': topDisplayRef(item.node.commit) !== null }"
                :title="stashesAtCommit(item.idx).map(s => 'stash@{' + s.index + '}: ' + (s.message || '')).join('\n')"
                @contextmenu.stop.prevent="onStashBadgeContextMenu($event, stashesAtCommit(item.idx))"
              >
                <svg class="w-3 h-3 flex-shrink-0" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <rect x="1" y="5" width="12" height="8" rx="1.5" fill="rgba(245,158,11,0.25)" stroke="#f59e0b" stroke-width="1.2"/>
                  <rect x="1" y="2" width="12" height="3.5" rx="1.2" fill="rgba(245,158,11,0.40)" stroke="#f59e0b" stroke-width="1.2"/>
                  <rect x="4.5" y="1" width="5" height="2" rx="0.8" fill="rgba(245,158,11,0.55)" stroke="#f59e0b" stroke-width="1"/>
                  <line x1="3.5" y1="9" x2="10.5" y2="9" stroke="#f59e0b" stroke-width="1" opacity="0.6"/>
                </svg>
                <span v-if="!topDisplayRef(item.node.commit)" class="text-[9px] font-semibold truncate max-w-[75px]" style="color:#f59e0b;">
                  {{ stashesAtCommit(item.idx).length > 1
                    ? stashesAtCommit(item.idx).length + '×'
                    : (stashesAtCommit(item.idx)[0].message || 'stash') }}
                </span>
                <span v-else-if="stashesAtCommit(item.idx).length > 1" class="text-[9px] font-semibold" style="color:#f59e0b;">
                  {{ stashesAtCommit(item.idx).length }}
                </span>
              </span>
            </template>

            <div
              v-if="hoveredRefRow === item.idx && displayRefs(item.node.commit).length > 1"
              class="absolute left-0 top-full z-50 min-w-[150px] bg-[var(--popover)] border border-[var(--border)] rounded-lg shadow-2xl py-1"
            >
              <div class="px-2 py-0.5 text-[8px] text-[var(--muted-foreground)] border-b border-[var(--border)] mb-0.5 uppercase tracking-wider">
                double-click to checkout
              </div>
              <button
                v-for="mr in displayRefs(item.node.commit)"
                :key="mr.key"
                class="w-full text-left px-2 py-1.5 text-[9px] hover:bg-[var(--primary)]/15 transition-colors truncate flex items-center gap-1.5 group"
                :class="{ 'font-bold': mr.kind === 'branch' && mr.local && mr.name === currentBranch }"
                :style="mr.kind === 'tag' ? { color: '#f59e0b' } : { color: item.node.color }"
                :draggable="mr.kind === 'branch'"
                @dblclick.stop="onRefDblClick(mr, item.node.commit)"
                @dragstart.stop="onBranchDragStart($event, mr)"
                @dragover.stop.prevent="onBranchDragOver($event, mr)"
                @drop.stop.prevent="onBranchDrop($event, mr)"
                @contextmenu.stop.prevent="onRefContextMenu($event, mr)"
              >
                <svg v-if="mr.kind === 'branch' && mr.local" class="w-2.5 h-2.5 flex-shrink-0 opacity-60" viewBox="0 0 16 16" fill="currentColor"><rect x="2" y="4" width="12" height="8" rx="1.5"/></svg>
                <span v-if="mr.kind === 'branch' && mr.remote" v-html="providerIconMarkup" />
                <svg v-if="mr.kind === 'tag'" class="w-2.5 h-2.5 flex-shrink-0 opacity-80" viewBox="0 0 16 16" fill="currentColor"><path d="M2 8.2V3.5C2 2.7 2.7 2 3.5 2h4.7c.4 0 .8.2 1.1.4l4.3 4.3c.6.6.6 1.6 0 2.1l-4.9 4.9c-.6.6-1.6.6-2.1 0L2.4 9.3C2.1 9 2 8.6 2 8.2zm4.2-3.5a1.2 1.2 0 100 2.4 1.2 1.2 0 000-2.4z"/></svg>
                <span class="truncate flex-1">{{ mr.name }}</span>
                <span
                  v-if="mr.kind === 'branch' && mr.local && mr.name === currentBranch"
                  class="text-[7px] px-1 py-0.5 rounded-full font-bold flex-shrink-0"
                  :style="{ backgroundColor: item.node.color + '30', border: '1px solid ' + item.node.color + '60' }"
                >HEAD</span>
              </button>
            </div>
          </div>

          <div class="flex-shrink-0 relative h-full overflow-visible" :style="{ width: graphWidth + 'px' }">
            <div
              class="absolute top-0 bottom-0 pointer-events-none"
              :style="{
                left: Math.max(0, lx(item.node.lane) - 4) + 'px',
                right: '-5px',
                background: 'linear-gradient(to right, ' + item.node.color + '08 0%, ' + item.node.color + '18 40%, ' + item.node.color + '28 100%)',
                clipPath: 'ellipse(110% 45% at 100% 50%)',
                borderRight: '1.5px solid ' + item.node.color + '90'
              }"
            />
          </div>

          <div class="flex-1 flex items-center px-3 min-w-0">
            <span class="text-[11px] text-[var(--foreground)] truncate opacity-85">{{ item.node.commit.message.split('\n')[0] }}</span>
          </div>

          <div class="flex-shrink-0 flex items-center gap-1 px-1" :style="{ width: AUTHOR_COL + 'px' }">
            <svg class="flex-shrink-0" :width="14" :height="14" viewBox="0 0 14 14" v-html="avatarSvg(item.node.commit.author_name, 7, 7, 6, item.node.color)" />
            <span class="text-[10px] text-[var(--muted-foreground)] truncate">{{ item.node.commit.author_name }}</span>
          </div>

          <div class="flex-shrink-0 px-1" :style="{ width: SHA_COL + 'px' }">
            <span class="text-[9px] font-mono" :style="{ color: item.node.color + 'cc' }">{{ item.node.commit.short_sha }}</span>
          </div>
        </div>

        <div
          v-if="hasMore"
          class="absolute left-0 right-0 flex items-center justify-center text-[9px] text-[var(--muted-foreground)]"
          :style="{ top: totalH + 'px', height: rowHeight + 'px' }"
        >
          Loading more commits...
        </div>
      </div>
    </div>

    <Teleport to="body">
      <div
        v-if="refCtxVisible && refCtxRef"
        class="fixed z-[120] w-[280px] bg-[var(--popover)] border border-[var(--border)] rounded-lg shadow-2xl p-1.5 text-[10px] text-[var(--foreground)]"
        :style="{ left: refCtxX + 'px', top: refCtxY + 'px' }"
        @click.stop
      >
        <template v-if="refCtxRef.kind === 'branch'">
          <div class="space-y-1">
            <button v-if="refCtxRef.local" class="ctx-item" @click="refAction('checkout-local')"><span class="ctx-icon">⎇</span><span class="ctx-main">Checkout branch</span><span class="ctx-sub">Switch to local branch {{ refCtxRef.name }}</span></button>
            <button v-if="!refCtxRef.local" class="ctx-item" @click="refAction('checkout-remote')"><span class="ctx-icon">⎇</span><span class="ctx-main">Checkout remote branch</span><span class="ctx-sub">Create/switch to origin/{{ refCtxRef.name }}</span></button>
            <button v-if="canMergeRefIntoCurrent(refCtxRef)" class="ctx-item" @click="refAction('merge-into-current')"><span class="ctx-icon">🍒</span><span class="ctx-main">Merge into {{ currentBranch }}</span><span class="ctx-sub">Merge selected branch into current branch</span></button>
            <button v-if="refCtxRef.local" class="ctx-item" @click="refAction('delete-local')"><span class="ctx-icon">🗑</span><span class="ctx-main">Delete local branch</span><span class="ctx-sub">Delete {{ refCtxRef.name }} from local repository</span></button>
            <button v-if="refCtxRef.remote" class="ctx-item" @click="refAction('delete-remote')"><span class="ctx-icon">🛰</span><span class="ctx-main">Delete remote branch</span><span class="ctx-sub">Delete origin/{{ refCtxRef.name }} on remote</span></button>
            <button class="ctx-item" @click="refAction('copy-name')"><span class="ctx-icon">📋</span><span class="ctx-main">Copy branch name</span><span class="ctx-sub">Copy name to clipboard</span></button>
          </div>
        </template>
        <template v-else>
          <div class="space-y-1">
            <button class="ctx-item" @click="refAction('delete-tag')"><span class="ctx-icon">🏷</span><span class="ctx-main">Delete tag</span><span class="ctx-sub">Delete tag {{ refCtxRef.name }}</span></button>
            <button class="ctx-item" @click="refAction('copy-name')"><span class="ctx-icon">📋</span><span class="ctx-main">Copy tag name</span><span class="ctx-sub">Copy name to clipboard</span></button>
          </div>
        </template>
      </div>

      <div
        v-if="stashCtxVisible && (stashCtxItem || stashCtxItems.length > 0)"
        class="fixed z-[125] min-w-[200px] bg-[var(--popover)] border border-[var(--border)] rounded-lg shadow-2xl py-1 text-[11px] text-[var(--foreground)]"
        :style="{ left: stashCtxX + 'px', top: stashCtxY + 'px' }"
        @click.stop
      >
        <template v-if="stashCtxItems.length > 0">
          <div class="px-3 py-1 text-[10px] text-[var(--muted-foreground)] border-b border-[var(--border)] mb-1 flex items-center gap-1">
            <svg class="w-3 h-3 opacity-60" viewBox="0 0 16 16" fill="currentColor"><path d="M2 5.5A1.5 1.5 0 013.5 4h9A1.5 1.5 0 0114 5.5v6A1.5 1.5 0 0112.5 13h-9A1.5 1.5 0 012 11.5v-6zm2 1v4h8v-4H4z"/></svg>
            {{ stashCtxItems.length }} stashes — choose action
          </div>
          <div v-for="s in stashCtxItems" :key="s.index" class="border-b border-[var(--border)]/40 last:border-0">
            <div class="px-3 py-1 text-[9px] text-[#f59e0b] font-mono font-semibold opacity-80">
              stash@{{ '{' + s.index + '}' }}
              <span v-if="s.message" class="text-[var(--muted-foreground)] font-normal normal-case ml-1 truncate inline-block max-w-[130px] align-bottom">{{ s.message }}</span>
            </div>
            <div class="flex pb-1">
              <button class="flex-1 text-center py-1 hover:bg-[var(--primary)]/15 text-[10px] transition-colors flex items-center justify-center gap-0.5" @click="stashAction('view', s)">
                <svg class="w-2.5 h-2.5 opacity-50" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="8" cy="8" r="2.5"/><path d="M1.5 8C3 4.5 5.3 2.5 8 2.5S13 4.5 14.5 8C13 11.5 10.7 13.5 8 13.5S3 11.5 1.5 8z"/></svg>
                View
              </button>
              <button class="flex-1 text-center py-1 hover:bg-[#f59e0b]/15 text-[#f59e0b] text-[10px] font-medium transition-colors" @click="stashAction('pop', s)">Pop</button>
              <button class="flex-1 text-center py-1 hover:bg-[var(--primary)]/15 text-[10px] transition-colors" @click="stashAction('apply', s)">Apply</button>
              <button class="flex-1 text-center py-1 hover:bg-[#ef4444]/15 text-[#ef4444] text-[10px] transition-colors" @click="stashAction('drop', s)">Drop</button>
            </div>
          </div>
        </template>

        <template v-else-if="stashCtxItem">
          <div class="px-3 py-1 text-[10px] text-[var(--muted-foreground)] border-b border-[var(--border)] mb-1">
            stash@{{ '{' + stashCtxItem.index + '}' }}
            <span v-if="stashCtxItem.message" class="ml-1 text-[var(--foreground)]/60 truncate">{{ stashCtxItem.message }}</span>
          </div>
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors flex items-center gap-2" @click="stashAction('view')">
            <svg class="w-3 h-3 opacity-60" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="8" cy="8" r="2.5"/><path d="M1.5 8C3 4.5 5.3 2.5 8 2.5S13 4.5 14.5 8C13 11.5 10.7 13.5 8 13.5S3 11.5 1.5 8z"/></svg>
            View changes
          </button>
          <div class="border-t border-[var(--border)] my-1" />
          <button class="w-full text-left px-3 py-1.5 hover:bg-[#f59e0b]/15 text-[#f59e0b] transition-colors" @click="stashAction('pop')">Pop stash</button>
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="stashAction('apply')">Apply stash</button>
          <button class="w-full text-left px-3 py-1.5 hover:bg-[#ef4444]/15 text-[#ef4444] transition-colors" @click="stashAction('drop')">Drop stash</button>
        </template>
      </div>

      <div
        v-if="ctxVisible"
        class="fixed z-[100] w-[360px] bg-[var(--popover)] border border-[var(--border)] rounded-lg shadow-2xl p-1.5 text-[10px] text-[var(--foreground)] max-h-[80vh] overflow-y-auto"
        :style="{ left: ctxX + 'px', top: ctxY + 'px' }"
        @click.stop
      >
        <template v-if="ctxHasBranch()">
          <div class="space-y-1">
            <button class="ctx-item" @click="ctxAction('pull')"><span class="ctx-icon">⬇</span><span class="ctx-main">Pull changes</span><span class="ctx-sub">Fetch and fast-forward current branch</span></button>
            <button class="ctx-item" @click="ctxAction('push')"><span class="ctx-icon">⬆</span><span class="ctx-main">Push changes</span><span class="ctx-sub">Push current branch commits to remote</span></button>
            <button class="ctx-item" @click="ctxAction('set-upstream')"><span class="ctx-icon">🔗</span><span class="ctx-main">Set upstream</span><span class="ctx-sub">Link local branch to origin/{{ ctxBranchName() }}</span></button>
            <button class="ctx-item" @click="ctxAction('checkout-branch')"><span class="ctx-icon">⎇</span><span class="ctx-main">Checkout branch</span><span class="ctx-sub">Switch working tree to {{ ctxBranchName() }}</span></button>
          </div>
          <div class="border-t border-[var(--border)] my-1" />
        </template>

        <div class="space-y-1">
          <button class="ctx-item" @click="ctxAction('checkout')"><span class="ctx-icon">⎇</span><span class="ctx-main">Checkout this commit</span><span class="ctx-sub">Detached HEAD at {{ ctxCommit?.short_sha }}</span></button>
          <button class="ctx-item" @click="ctxAction('branch')"><span class="ctx-icon">🌿</span><span class="ctx-main">Create branch here</span><span class="ctx-sub">Create new branch from this commit</span></button>
          <button class="ctx-item" @click="ctxAction('cherry-pick')"><span class="ctx-icon">🍒</span><span class="ctx-main">Cherry-pick commit</span><span class="ctx-sub">Apply this commit on top of current branch</span></button>
          <button class="ctx-item" @click="ctxAction('revert')"><span class="ctx-icon">↩</span><span class="ctx-main">Revert commit</span><span class="ctx-sub">Create new commit that undoes this one</span></button>
        </div>

        <div class="relative">
          <button
            class="ctx-item ctx-item-reset mt-1 w-full"
            @click.stop="ctxResetSub = !ctxResetSub"
          >
            <span class="ctx-icon">🔄</span>
            <span class="ctx-main !whitespace-nowrap">Reset {{ currentBranch }} to this commit</span>
            <svg :class="['ctx-arrow w-3 h-3 text-[var(--muted-foreground)] transition-transform', ctxResetSub ? 'rotate-90' : '']" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>
            <span class="ctx-sub">Choose Soft / Mixed / Hard reset mode</span>
          </button>
          <div
            v-if="ctxResetSub"
            class="bg-[var(--popover)]/80 py-1 pl-2 pr-1"
          >
            <button class="ctx-item-sub" @click="ctxAction('reset-soft')">Soft — move HEAD, keep all changes staged</button>
            <button class="ctx-item-sub" @click="ctxAction('reset-mixed')">Mixed — move HEAD, unstage files, keep working tree</button>
            <button class="ctx-item-sub text-[#ef4444] hover:bg-[#ef4444]/15" @click="ctxAction('reset-hard')">Hard — move HEAD and discard all working changes</button>
          </div>
        </div>

        <template v-if="ctxHasBranch() && ctxIsHeadCommit()">
          <button class="ctx-item mt-1" @click="ctxAction('edit-message')"><span class="ctx-icon">✏</span><span class="ctx-main">Edit commit message</span><span class="ctx-sub">Amend message of current HEAD commit</span></button>
        </template>

        <div class="border-t border-[var(--border)] my-1" />

        <template v-if="ctxHasBranch()">
          <div class="space-y-1">
            <button class="ctx-item" @click="ctxAction('rename-branch')"><span class="ctx-icon">✏</span><span class="ctx-main">Rename branch</span><span class="ctx-sub">Rename local branch {{ ctxBranchName() }}</span></button>
            <button class="ctx-item" @click="ctxAction('delete-branch')"><span class="ctx-icon">🗑</span><span class="ctx-main">Delete local branch</span><span class="ctx-sub">Remove local branch {{ ctxBranchName() }}</span></button>
            <button v-if="ctxBranchRef()?.remote" class="ctx-item" @click="ctxAction('delete-remote-branch')"><span class="ctx-icon">🛰</span><span class="ctx-main">Delete remote branch</span><span class="ctx-sub">Remove origin/{{ ctxBranchName() }} on remote</span></button>
            <button v-if="ctxBranchRef()?.local && ctxBranchRef()?.remote" class="ctx-item text-[#ef4444] hover:bg-[#ef4444]/15" @click="ctxAction('delete-both')"><span class="ctx-icon">💥</span><span class="ctx-main">Delete local and remote</span><span class="ctx-sub">Delete both {{ ctxBranchName() }} and origin/{{ ctxBranchName() }}</span></button>
            <button v-if="ctxBranchRef()?.remote && ctxBranchRef()?.local" class="ctx-item" @click="ctxAction('reset-to-remote')"><span class="ctx-icon">🔄</span><span class="ctx-main">Reset branch to remote</span><span class="ctx-sub">Force local {{ ctxBranchName() }} to origin/{{ ctxBranchName() }}</span></button>
            <button class="ctx-item" @click="ctxAction('copy-branch-name')"><span class="ctx-icon">📋</span><span class="ctx-main">Copy branch name</span><span class="ctx-sub">Copy branch name to clipboard</span></button>
          </div>
        </template>
        <div class="space-y-1 mt-1">
          <button class="ctx-item" @click="ctxAction('copy-sha')"><span class="ctx-icon">📋</span><span class="ctx-main">Copy commit SHA</span><span class="ctx-sub">Copy full commit hash to clipboard</span></button>
          <button class="ctx-item" @click="ctxAction('tag')"><span class="ctx-icon">🏷</span><span class="ctx-main">Create lightweight tag</span><span class="ctx-sub">Create tag on this commit</span></button>
          <button class="ctx-item" @click="ctxAction('annotated-tag')"><span class="ctx-icon">🏷✏</span><span class="ctx-main">Create annotated tag</span><span class="ctx-sub">Create tag with message and metadata</span></button>
        </div>

      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.commit-scroll::-webkit-scrollbar {
  width: 4px;
}
.commit-scroll::-webkit-scrollbar-track {
  background: transparent;
}
.commit-scroll::-webkit-scrollbar-thumb {
  background: rgba(139, 92, 246, 0.15);
  border-radius: 10px;
}
.commit-scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(139, 92, 246, 0.35);
}

.node-pop {
  transform-origin: center;
  animation: popIn 0.3s ease-out forwards;
}
@keyframes popIn {
  0% { opacity: 0; transform: scale(0.3); }
  70% { transform: scale(1.08); }
  100% { opacity: 1; transform: scale(1); }
}

.graph-row {
  animation: rowFade 0.2s ease-out;
}
@keyframes rowFade {
  from { opacity: 0; transform: translateY(3px); }
  to { opacity: 1; transform: translateY(0); }
}

.stash-badge {
  background: linear-gradient(135deg, rgba(245,158,11,0.18) 0%, rgba(245,158,11,0.08) 100%);
  border: 1px solid rgba(245,158,11,0.45);
  border-radius: 5px;
  box-shadow: 0 1px 4px rgba(245,158,11,0.12), inset 0 1px 0 rgba(255,255,255,0.06);
  transition: border-color 0.2s, box-shadow 0.2s;
}
.stash-badge:hover {
  border-color: rgba(245,158,11,0.7);
  box-shadow: 0 1px 6px rgba(245,158,11,0.25);
}

.working-node {
  animation: workingDash 1.2s linear infinite;
}
@keyframes workingDash {
  to { stroke-dashoffset: -28; }
}

.current-branch-badge {
  position: relative;
  z-index: 0;
  overflow: visible;
}
.current-branch-badge::before {
  content: '';
  position: absolute;
  inset: -2px;
  border-radius: inherit;
  border: 1.5px solid currentColor;
  opacity: 0.15;
  pointer-events: none;
}
.current-branch-badge::after {
  content: '';
  position: absolute;
  inset: -2px;
  border-radius: inherit;
  pointer-events: none;
  background: conic-gradient(
    from var(--a),
    transparent 0deg,
    currentColor 20deg,
    currentColor 40deg,
    transparent 60deg,
    transparent 180deg,
    currentColor 200deg,
    currentColor 220deg,
    transparent 240deg
  );
  mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  mask-composite: exclude;
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  padding: 1.5px;
  opacity: 0.9;
  animation: rotateBorder 1.2s linear infinite;
}
@property --a {
  syntax: '<angle>';
  initial-value: 0deg;
  inherits: false;
}
@keyframes rotateBorder {
  to { --a: 360deg; }
}
.ctx-item {
  width: 100%;
  text-align: left;
  padding: 0.32rem 0.45rem;
  border-radius: 0.4rem;
  transition: background-color 0.15s ease;
  display: grid;
  grid-template-columns: 1rem 1fr;
  grid-template-areas:
    "icon main"
    "icon sub";
  column-gap: 0.45rem;
  row-gap: 0.05rem;
}
.ctx-item:hover {
  background: color-mix(in srgb, var(--primary) 15%, transparent);
}
.ctx-icon {
  grid-area: icon;
  width: 1rem;
  text-align: center;
  line-height: 1rem;
}
.ctx-main {
  grid-area: main;
  font-size: 10px;
  line-height: 1.2;
}
.ctx-sub {
  grid-area: sub;
  font-size: 8px;
  line-height: 1.2;
  color: var(--muted-foreground);
}
.ctx-item-reset {
  grid-template-columns: 1rem 1fr auto;
  grid-template-areas:
    "icon main arrow"
    "sub sub";
}
.ctx-arrow {
  grid-area: arrow;
  align-self: center;
}
.ctx-item-sub {
  width: 100%;
  text-align: left;
  padding: 0.3rem 0.45rem;
  border-radius: 0.35rem;
  transition: background-color 0.15s ease;
  font-size: 9px;
  line-height: 1.25;
}
.ctx-item-sub:hover {
  background: color-mix(in srgb, var(--primary) 15%, transparent);
}
</style>


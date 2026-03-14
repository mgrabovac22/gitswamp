<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import type { CommitInfo, GraphNode, GraphEdge } from "@/types";

const LANE_WIDTH = 14;
const ROW_HEIGHT = 24;
const NODE_RADIUS = 5;
const BRANCH_COL = 130;
const AUTHOR_COL = 140;
const SHA_COL = 75;
const OVERSCAN = 15;
const CORNER_R = 5;
const COLORS = [
  "#8b5cf6", "#06b6d4", "#f59e0b", "#ef4444", "#10b981",
  "#ec4899", "#f97316", "#14b8a6", "#a855f7", "#22d3ee",
  "#84cc16", "#e879f9", "#fb923c", "#2dd4bf",
];

const props = defineProps<{
  commits: CommitInfo[];
  selected: CommitInfo | null;
  searchQuery?: string;
  hasWorkingChanges: boolean;
  currentBranch: string;
  hasMore?: boolean;
}>();

const emit = defineEmits<{
  select: [commit: CommitInfo | null];
  search: [query: string];
  clearSearch: [];
  selectWorkingChanges: [];
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
}>();

const searchInput = ref("");
const scrollContainer = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const viewportHeight = ref(600);

// Context menu state
const ctxVisible = ref(false);
const ctxX = ref(0);
const ctxY = ref(0);
const ctxCommit = ref<CommitInfo | null>(null);
const ctxResetSub = ref(false);

function avatarColor(name: string): string {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = name.charCodeAt(i) + ((h << 5) - h);
  return COLORS[Math.abs(h) % COLORS.length];
}

function avatarInitials(name: string): string {
  const p = name.trim().split(/\s+/);
  return p.length >= 2 ? (p[0][0] + p[p.length - 1][0]).toUpperCase() : name.substring(0, 2).toUpperCase();
}

const graph = computed(() => {
  const all = props.commits;
  if (!all.length) return { nodes: [] as GraphNode[], edges: [] as GraphEdge[], laneCount: 0, branchLanes: new Map<string, number>() };

  const shaIdx = new Map<string, number>();
  all.forEach((c, i) => shaIdx.set(c.sha, i));

  const branchHeads = new Map<string, number>();
  const commitBranch = new Map<number, string>();

  all.forEach((commit, idx) => {
    for (const r of commit.refs) {
      const name = r.replace(/^origin\//, "");
      if (name === "HEAD" || name.includes("->")) continue;
      if (!branchHeads.has(name)) {
        branchHeads.set(name, idx);
        if (!commitBranch.has(idx)) commitBranch.set(idx, name);
      }
    }
  });

  for (const [branchName, headIdx] of branchHeads) {
    let current = headIdx;
    const visited = new Set<number>();
    while (current !== undefined && !visited.has(current)) {
      visited.add(current);
      const commit = all[current];
      if (commit.parent_shas.length > 0) {
        const fp = shaIdx.get(commit.parent_shas[0]);
        if (fp !== undefined && !commitBranch.has(fp)) {
          commitBranch.set(fp, branchName);
          current = fp;
        } else break;
      } else break;
    }
  }

  all.forEach((_, idx) => {
    if (!commitBranch.has(idx)) commitBranch.set(idx, "__default");
  });

  // Order: current branch lane 0 (leftmost), then main/master, then others by first appearance
  const branchOrder: string[] = [];
  const add = (n: string) => { if (!branchOrder.includes(n)) branchOrder.push(n); };
  if (props.currentBranch && [...commitBranch.values()].includes(props.currentBranch)) add(props.currentBranch);
  for (const n of ["main", "master"]) { if ([...commitBranch.values()].includes(n)) add(n); }
  all.forEach((_, idx) => { const b = commitBranch.get(idx); if (b) add(b); });

  const laneMap = new Map<string, number>();
  branchOrder.forEach((name, i) => laneMap.set(name, i));

  const nodes: GraphNode[] = all.map((commit, idx) => {
    const branch = commitBranch.get(idx) || "__default";
    const lane = laneMap.get(branch) || 0;
    return { commit, lane, color: COLORS[lane % COLORS.length] };
  });

  const edges: GraphEdge[] = [];
  all.forEach((commit, idx) => {
    for (const parentSha of commit.parent_shas) {
      const pi = shaIdx.get(parentSha);
      if (pi !== undefined) {
        edges.push({
          fromIndex: idx, toIndex: pi,
          fromLane: nodes[idx].lane, toLane: nodes[pi].lane,
          color: nodes[idx].color,
        });
      }
    }
  });

  return { nodes, edges, laneCount: branchOrder.length, branchLanes: laneMap };
});

const graphWidth = computed(() => Math.max((graph.value.laneCount + 1) * LANE_WIDTH + 8, 40));
const wcOffset = computed(() => props.hasWorkingChanges ? ROW_HEIGHT : 0);
const totalH = computed(() => props.commits.length * ROW_HEIGHT + wcOffset.value);

const visibleRange = computed(() => {
  const startY = scrollTop.value - wcOffset.value;
  const first = Math.max(0, Math.floor(startY / ROW_HEIGHT) - OVERSCAN);
  const last = Math.min(
    props.commits.length - 1,
    Math.ceil((startY + viewportHeight.value) / ROW_HEIGHT) + OVERSCAN
  );
  return { first, last };
});

const visibleNodes = computed(() => {
  const { first, last } = visibleRange.value;
  const nodes = graph.value.nodes;
  const result: { node: GraphNode; idx: number }[] = [];
  for (let i = first; i <= last && i < nodes.length; i++) {
    result.push({ node: nodes[i], idx: i });
  }
  return result;
});

const visibleEdges = computed(() => {
  const { first, last } = visibleRange.value;
  return graph.value.edges.filter(e => {
    const minIdx = Math.min(e.fromIndex, e.toIndex);
    const maxIdx = Math.max(e.fromIndex, e.toIndex);
    return maxIdx >= first - 2 && minIdx <= last + 2;
  });
});

function lx(lane: number) { return lane * LANE_WIDTH + LANE_WIDTH / 2 + 4; }
function ry(index: number) { return index * ROW_HEIGHT + ROW_HEIGHT / 2 + wcOffset.value; }

// GitKraken-style edge: straight vertical lines with small radius corners at transitions
function ep(e: GraphEdge): string {
  const x1 = lx(e.fromLane), y1 = ry(e.fromIndex);
  const x2 = lx(e.toLane), y2 = ry(e.toIndex);
  if (e.fromLane === e.toLane) return 'M ' + x1 + ' ' + y1 + ' L ' + x2 + ' ' + y2;
  const r = Math.min(CORNER_R, Math.abs(x2 - x1) / 2, Math.abs(y2 - y1) / 4);
  const d = x2 > x1 ? 1 : -1;
  // Horizontal transition near the child (fromIndex, top), then vertical to parent
  const turnY = y1 + ROW_HEIGHT * 0.5;
  return 'M ' + x1 + ' ' + y1
    + ' L ' + x1 + ' ' + (turnY - r)
    + ' Q ' + x1 + ' ' + turnY + ' ' + (x1 + d * r) + ' ' + turnY
    + ' L ' + (x2 - d * r) + ' ' + turnY
    + ' Q ' + x2 + ' ' + turnY + ' ' + x2 + ' ' + (turnY + r)
    + ' L ' + x2 + ' ' + y2;
}

function wcEdge(): string {
  const cl = graph.value.branchLanes.get(props.currentBranch) ?? 0;
  const x = lx(cl);
  return 'M ' + x + ' ' + (ROW_HEIGHT / 2) + ' L ' + x + ' ' + ry(0);
}

function wcLaneX(): number { return lx(graph.value.branchLanes.get(props.currentBranch) ?? 0); }

function brefs(commit: CommitInfo): string[] {
  return commit.refs.filter(r => !r.includes("HEAD") && !r.includes("->"));
}

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

// Context menu
function onCtx(e: MouseEvent, commit: CommitInfo) {
  e.preventDefault();
  ctxCommit.value = commit;
  ctxX.value = e.clientX;
  ctxY.value = e.clientY;
  ctxResetSub.value = false;
  ctxVisible.value = true;
}

function closeCtx() {
  ctxVisible.value = false;
  ctxResetSub.value = false;
}

function ctxAction(action: string) {
  if (!ctxCommit.value) return;
  const sha = ctxCommit.value.sha;
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
  }
}

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
  <div class="flex-1 bg-[#111520] flex flex-col overflow-hidden min-w-0">
    <!-- Header -->
    <div class="flex-shrink-0 border-b border-[#8b5cf6]/10 bg-[#111520]/95 backdrop-blur-sm z-10">
      <div class="px-3 py-1.5 flex items-center gap-2">
        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-[#64748b] flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
        <input v-model="searchInput" @input="onSearch" placeholder="Search commits, messages, authors, SHA..." class="flex-1 bg-transparent text-xs text-[#e2e8f0] placeholder:text-[#475569] focus:outline-none" />
        <button v-if="searchInput" @click="clearSearch" class="p-0.5 rounded hover:bg-[#252b3d] text-[#64748b] hover:text-[#e2e8f0] transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
        <span v-if="searchQuery" class="text-[10px] text-[#64748b]">{{ commits.length }} results</span>
      </div>
      <div class="flex items-center py-0.5 text-[9px] text-[#64748b] uppercase tracking-wider font-medium border-t border-[#8b5cf6]/5">
        <div class="flex-shrink-0 px-2 text-right" :style="{ width: BRANCH_COL + 'px' }">Branch / Tag</div>
        <div class="flex-shrink-0 text-center" :style="{ width: graphWidth + 'px' }">Graph</div>
        <div class="flex-1 px-3">Commit Message</div>
        <div class="flex-shrink-0 px-1" :style="{ width: AUTHOR_COL + 'px' }">Author</div>
        <div class="flex-shrink-0 px-1" :style="{ width: SHA_COL + 'px' }">SHA</div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="!commits.length && !hasWorkingChanges" class="flex-1 flex items-center justify-center text-sm text-[#475569]">
      No commits to display
    </div>

    <!-- Scrollable content -->
    <div v-else ref="scrollContainer" class="flex-1 overflow-y-auto min-h-0" @scroll="onScroll">
      <div class="relative" :style="{ height: totalH + 'px' }">
        <!-- SVG graph layer -->
        <svg
          class="absolute top-0 pointer-events-none"
          :style="{ left: BRANCH_COL + 'px', width: graphWidth + 'px', height: totalH + 'px' }"
        >
          <path
            v-if="hasWorkingChanges && graph.nodes.length > 0"
            :d="wcEdge()"
            stroke="#8b5cf6" stroke-width="2" fill="none" opacity="0.4"
            stroke-dasharray="4 3" stroke-linecap="round"
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
            :x="wcLaneX() - 4" :y="ROW_HEIGHT / 2 - 4"
            width="8" height="8" rx="2"
            fill="#8b5cf6" opacity="0.9" class="animate-pulse"
          />
          <template v-for="item in visibleNodes" :key="'n' + item.node.commit.sha">
            <circle
              :cx="lx(item.node.lane)" :cy="ry(item.idx)"
              :r="NODE_RADIUS"
              :fill="avatarColor(item.node.commit.author_name)"
              :stroke="selected?.sha === item.node.commit.sha ? '#fff' : avatarColor(item.node.commit.author_name)"
              :stroke-width="selected?.sha === item.node.commit.sha ? 2 : 0.5"
              :opacity="selected?.sha === item.node.commit.sha ? 1 : 0.9"
            />
            <text
              :x="lx(item.node.lane)" :y="ry(item.idx) + 0.5"
              text-anchor="middle" dominant-baseline="middle"
              fill="#fff" font-size="5.5" font-weight="700"
              font-family="system-ui, -apple-system, sans-serif"
              style="pointer-events: none; user-select: none;"
            >{{ avatarInitials(item.node.commit.author_name) }}</text>
            <circle
              v-if="item.node.commit.refs.length > 0"
              :cx="lx(item.node.lane)" :cy="ry(item.idx)"
              :r="NODE_RADIUS + 3" fill="none"
              :stroke="item.node.color" stroke-width="1" opacity="0.3"
            />
          </template>
        </svg>

        <!-- Working changes row -->
        <div
          v-if="hasWorkingChanges"
          class="absolute left-0 right-0 flex items-center cursor-pointer transition-colors"
          :class="selected === null ? 'bg-[#8b5cf6]/10' : 'hover:bg-[#1a2030]'"
          :style="{ top: '0px', height: ROW_HEIGHT + 'px' }"
          @click="emit('selectWorkingChanges')"
        >
          <div class="flex-shrink-0" :style="{ width: BRANCH_COL + 'px' }" />
          <div class="flex-shrink-0" :style="{ width: graphWidth + 'px' }" />
          <div class="flex-1 flex items-center px-3 min-w-0">
            <span class="px-1.5 py-0.5 rounded text-[9px] font-semibold bg-[#8b5cf6]/20 text-[#a78bfa] border border-[#8b5cf6]/30">
              ● Working Changes
            </span>
          </div>
          <div class="flex-shrink-0 text-[10px] text-[#64748b] px-1" :style="{ width: AUTHOR_COL + 'px' }">—</div>
          <div class="flex-shrink-0 text-[9px] font-mono text-[#64748b] px-1" :style="{ width: SHA_COL + 'px' }">—</div>
        </div>

        <!-- Commit rows -->
        <div
          v-for="item in visibleNodes"
          :key="item.node.commit.sha"
          class="absolute left-0 right-0 flex items-center cursor-pointer transition-colors"
          :class="selected?.sha === item.node.commit.sha ? 'bg-[#8b5cf6]/10' : 'hover:bg-[#1a2030]'"
          :style="{ top: (item.idx * ROW_HEIGHT + wcOffset) + 'px', height: ROW_HEIGHT + 'px' }"
          @click="emit('select', item.node.commit)"
          @contextmenu="onCtx($event, item.node.commit)"
        >
          <div class="flex-shrink-0 flex items-center justify-end gap-0.5 overflow-hidden px-1" :style="{ width: BRANCH_COL + 'px' }">
            <span
              v-for="label in brefs(item.node.commit)"
              :key="label"
              class="flex-shrink-0 px-1.5 py-0.5 rounded text-[8px] font-medium truncate max-w-[120px]"
              :style="{ backgroundColor: item.node.color + '20', color: item.node.color, border: '1px solid ' + item.node.color + '30' }"
              :title="label"
            >{{ label }}</span>
          </div>
          <div class="flex-shrink-0 relative h-full" :style="{ width: graphWidth + 'px' }">
            <div
              class="absolute top-0 bottom-0 pointer-events-none"
              :style="{ left: lx(item.node.lane) + 'px', right: '-8px', background: 'linear-gradient(to right, ' + item.node.color + '10, transparent)', clipPath: 'polygon(0 20%, 100% 0%, 100% 100%, 0 80%)' }"
            />
          </div>
          <div class="flex-1 flex items-center px-3 min-w-0">
            <span class="text-[11px] text-[#cbd5e1] truncate">{{ item.node.commit.message.split('\n')[0] }}</span>
          </div>
          <div class="flex-shrink-0 flex items-center gap-1 px-1" :style="{ width: AUTHOR_COL + 'px' }">
            <div
              class="w-3.5 h-3.5 rounded-full flex items-center justify-center flex-shrink-0 text-[6px] font-bold text-white"
              :style="{ backgroundColor: avatarColor(item.node.commit.author_name) }"
            >{{ avatarInitials(item.node.commit.author_name) }}</div>
            <span class="text-[10px] text-[#64748b] truncate">{{ item.node.commit.author_name }}</span>
          </div>
          <div class="flex-shrink-0 px-1" :style="{ width: SHA_COL + 'px' }">
            <span class="text-[9px] font-mono" :style="{ color: item.node.color + 'cc' }">{{ item.node.commit.short_sha }}</span>
          </div>
        </div>

        <!-- Load more -->
        <div
          v-if="hasMore"
          class="absolute left-0 right-0 flex items-center justify-center text-[9px] text-[#64748b]"
          :style="{ top: totalH + 'px', height: ROW_HEIGHT + 'px' }"
        >
          Loading more commits...
        </div>
      </div>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <div
        v-if="ctxVisible"
        class="fixed z-[100] min-w-[200px] bg-[#1c2130] border border-[#8b5cf6]/20 rounded-lg shadow-2xl py-1 text-[11px] text-[#e2e8f0]"
        :style="{ left: ctxX + 'px', top: ctxY + 'px' }"
        @click.stop
      >
        <button class="w-full text-left px-3 py-1.5 hover:bg-[#8b5cf6]/15 transition-colors" @click="ctxAction('checkout')">Checkout this commit</button>
        <button class="w-full text-left px-3 py-1.5 hover:bg-[#8b5cf6]/15 transition-colors" @click="ctxAction('branch')">Create branch here</button>
        <button class="w-full text-left px-3 py-1.5 hover:bg-[#8b5cf6]/15 transition-colors" @click="ctxAction('cherry-pick')">Cherry pick commit</button>
        <div class="relative">
          <button
            class="w-full text-left px-3 py-1.5 hover:bg-[#8b5cf6]/15 transition-colors flex items-center justify-between"
            @click.stop="ctxResetSub = !ctxResetSub"
          >
            <span>Reset to this commit</span>
            <svg class="w-3 h-3 text-[#64748b]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>
          </button>
          <div
            v-if="ctxResetSub"
            class="absolute left-full top-0 ml-0.5 min-w-[200px] bg-[#1c2130] border border-[#8b5cf6]/20 rounded-lg shadow-2xl py-1"
          >
            <button class="w-full text-left px-3 py-1.5 hover:bg-[#8b5cf6]/15 transition-colors" @click="ctxAction('reset-soft')">Soft – keep all changes</button>
            <button class="w-full text-left px-3 py-1.5 hover:bg-[#8b5cf6]/15 transition-colors" @click="ctxAction('reset-mixed')">Mixed – keep working copy but reset index</button>
            <button class="w-full text-left px-3 py-1.5 hover:bg-[#ef4444]/15 text-[#ef4444] transition-colors" @click="ctxAction('reset-hard')">Hard – discard all changes</button>
          </div>
        </div>
        <button class="w-full text-left px-3 py-1.5 hover:bg-[#8b5cf6]/15 transition-colors" @click="ctxAction('revert')">Revert commit</button>
        <div class="border-t border-[#8b5cf6]/10 my-1" />
        <button class="w-full text-left px-3 py-1.5 hover:bg-[#8b5cf6]/15 transition-colors" @click="ctxAction('copy-sha')">Copy commit SHA</button>
        <div class="border-t border-[#8b5cf6]/10 my-1" />
        <button class="w-full text-left px-3 py-1.5 hover:bg-[#8b5cf6]/15 transition-colors" @click="ctxAction('tag')">Create tag here</button>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.overflow-y-auto::-webkit-scrollbar {
  width: 5px;
}
.overflow-y-auto::-webkit-scrollbar-track {
  background: transparent;
}
.overflow-y-auto::-webkit-scrollbar-thumb {
  background: rgba(139, 92, 246, 0.2);
  border-radius: 10px;
}
.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: rgba(139, 92, 246, 0.4);
}
</style>

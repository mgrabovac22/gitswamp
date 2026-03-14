<script setup lang="ts">
import { computed, ref, onMounted } from "vue";
import type { CommitInfo, GraphNode, GraphEdge } from "@/types";

const LANE_WIDTH = 16;
const ROW_HEIGHT = 32;
const NODE_RADIUS = 4;
const BRANCH_COL = 130;
const AUTHOR_COL = 140;
const SHA_COL = 75;
const OVERSCAN = 10;
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
}>();

const searchInput = ref("");
const scrollContainer = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const viewportHeight = ref(600);

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

const graphWidth = computed(() => Math.max((graph.value.laneCount + 1) * LANE_WIDTH + 8, 48));
const wcOffset = computed(() => props.hasWorkingChanges ? ROW_HEIGHT : 0);
const totalH = computed(() => props.commits.length * ROW_HEIGHT + wcOffset.value);

// Virtual scrolling: only render visible rows + overscan
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

// Only render edges that touch visible area
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

function ep(e: GraphEdge): string {
  const x1 = lx(e.fromLane), y1 = ry(e.fromIndex);
  const x2 = lx(e.toLane), y2 = ry(e.toIndex);
  if (e.fromLane === e.toLane) return 'M ' + x1 + ' ' + y1 + ' L ' + x2 + ' ' + y2;
  if (Math.abs(y2 - y1) <= ROW_HEIGHT * 2) {
    const cy = (y1 + y2) / 2;
    return 'M ' + x1 + ' ' + y1 + ' C ' + x1 + ' ' + cy + ', ' + x2 + ' ' + cy + ', ' + x2 + ' ' + y2;
  }
  const my = y1 + ROW_HEIGHT * 0.7;
  const r = Math.min(8, Math.abs(x2 - x1) / 2);
  const d = x2 > x1 ? 1 : -1;
  return 'M ' + x1 + ' ' + y1 + ' L ' + x1 + ' ' + (my - r)
    + ' Q ' + x1 + ' ' + my + ', ' + (x1 + d * r) + ' ' + my
    + ' L ' + (x2 - d * r) + ' ' + my
    + ' Q ' + x2 + ' ' + my + ', ' + x2 + ' ' + (my + r)
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
  // Load more when near bottom
  if (props.hasMore && el.scrollTop + el.clientHeight >= el.scrollHeight - 200) {
    emit("loadMore");
  }
}

onMounted(() => {
  if (scrollContainer.value) {
    viewportHeight.value = scrollContainer.value.clientHeight;
  }
});
</script>

<template>
  <div class="flex-1 bg-[#111520] flex flex-col overflow-hidden min-w-0">
    <!-- Header -->
    <div class="flex-shrink-0 border-b border-[#8b5cf6]/10 bg-[#111520]/95 backdrop-blur-sm z-10">
      <!-- Search -->
      <div class="px-3 py-1.5 flex items-center gap-2">
        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-[#64748b] flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
        <input v-model="searchInput" @input="onSearch" placeholder="Search commits, messages, authors, SHA..." class="flex-1 bg-transparent text-xs text-[#e2e8f0] placeholder:text-[#475569] focus:outline-none" />
        <button v-if="searchInput" @click="clearSearch" class="p-0.5 rounded hover:bg-[#252b3d] text-[#64748b] hover:text-[#e2e8f0] transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
        <span v-if="searchQuery" class="text-[10px] text-[#64748b]">{{ commits.length }} results</span>
      </div>
      <!-- Column headers -->
      <div class="flex items-center py-1 text-[10px] text-[#64748b] uppercase tracking-wider font-medium border-t border-[#8b5cf6]/5">
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

    <!-- Scrollable content with virtual scrolling -->
    <div v-else ref="scrollContainer" class="flex-1 overflow-y-auto min-h-0" @scroll="onScroll">
      <div class="relative" :style="{ height: totalH + 'px' }">
        <!-- SVG graph layer - only render visible edges + nodes -->
        <svg
          class="absolute top-0 pointer-events-none"
          :style="{ left: BRANCH_COL + 'px', width: graphWidth + 'px', height: totalH + 'px' }"
        >
          <!-- Working changes dashed line -->
          <path
            v-if="hasWorkingChanges && graph.nodes.length > 0"
            :d="wcEdge()"
            stroke="#8b5cf6" stroke-width="2" fill="none" opacity="0.4"
            stroke-dasharray="4 3" stroke-linecap="round"
          />

          <!-- Visible edge lines only -->
          <path
            v-for="(edge, i) in visibleEdges"
            :key="'e' + edge.fromIndex + '-' + edge.toIndex + '-' + i"
            :d="ep(edge)"
            :stroke="edge.color"
            stroke-width="2" fill="none" opacity="0.6"
            stroke-linecap="round" stroke-linejoin="round"
          />

          <!-- Working changes node -->
          <rect
            v-if="hasWorkingChanges"
            :x="wcLaneX() - 5" :y="ROW_HEIGHT / 2 - 5"
            width="10" height="10" rx="2"
            fill="#8b5cf6" opacity="0.9" class="animate-pulse"
          />

          <!-- Visible commit nodes only -->
          <template v-for="item in visibleNodes" :key="'n' + item.node.commit.sha">
            <circle
              :cx="lx(item.node.lane)" :cy="ry(item.idx)"
              :r="NODE_RADIUS" :fill="item.node.color"
              :stroke="selected?.sha === item.node.commit.sha ? '#fff' : item.node.color"
              :stroke-width="selected?.sha === item.node.commit.sha ? 2 : 1"
            />
            <circle
              v-if="item.node.commit.refs.length > 0"
              :cx="lx(item.node.lane)" :cy="ry(item.idx)"
              :r="NODE_RADIUS + 3" fill="none"
              :stroke="item.node.color" stroke-width="1.5" opacity="0.3"
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
            <span class="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-[#8b5cf6]/20 text-[#a78bfa] border border-[#8b5cf6]/30">
              ● Working Changes
            </span>
          </div>
          <div class="flex-shrink-0 text-xs text-[#64748b] px-1" :style="{ width: AUTHOR_COL + 'px' }">—</div>
          <div class="flex-shrink-0 text-[10px] font-mono text-[#64748b] px-1" :style="{ width: SHA_COL + 'px' }">—</div>
        </div>

        <!-- Visible commit rows only -->
        <div
          v-for="item in visibleNodes"
          :key="item.node.commit.sha"
          class="absolute left-0 right-0 flex items-center cursor-pointer transition-colors"
          :class="selected?.sha === item.node.commit.sha ? 'bg-[#8b5cf6]/10' : 'hover:bg-[#1a2030]'"
          :style="{ top: (item.idx * ROW_HEIGHT + wcOffset) + 'px', height: ROW_HEIGHT + 'px' }"
          @click="emit('select', item.node.commit)"
        >
          <!-- Branch / Tag labels -->
          <div class="flex-shrink-0 flex items-center justify-end gap-0.5 overflow-hidden px-1" :style="{ width: BRANCH_COL + 'px' }">
            <span
              v-for="label in brefs(item.node.commit)"
              :key="label"
              class="flex-shrink-0 px-1.5 py-0.5 rounded text-[9px] font-medium truncate max-w-[120px]"
              :style="{ backgroundColor: item.node.color + '20', color: item.node.color, border: '1px solid ' + item.node.color + '30' }"
              :title="label"
            >{{ label }}</span>
          </div>

          <!-- Graph column spacer + gradient spray -->
          <div class="flex-shrink-0 relative h-full" :style="{ width: graphWidth + 'px' }">
            <div
              class="absolute right-[-10px] top-0 bottom-0 w-5 pointer-events-none"
              :style="{ background: 'linear-gradient(to right, ' + item.node.color + '12, transparent)', clipPath: 'polygon(0 25%, 100% 0%, 100% 100%, 0 75%)' }"
            />
          </div>

          <!-- Commit message -->
          <div class="flex-1 flex items-center px-3 min-w-0">
            <span class="text-[12px] text-[#cbd5e1] truncate">{{ item.node.commit.message.split('\n')[0] }}</span>
          </div>

          <!-- Author with avatar -->
          <div class="flex-shrink-0 flex items-center gap-1.5 px-1" :style="{ width: AUTHOR_COL + 'px' }">
            <div
              class="w-4 h-4 rounded-full flex items-center justify-center flex-shrink-0 text-[7px] font-bold text-white"
              :style="{ backgroundColor: avatarColor(item.node.commit.author_name) }"
            >{{ avatarInitials(item.node.commit.author_name) }}</div>
            <span class="text-[11px] text-[#64748b] truncate">{{ item.node.commit.author_name }}</span>
          </div>

          <!-- SHA -->
          <div class="flex-shrink-0 px-1" :style="{ width: SHA_COL + 'px' }">
            <span class="text-[10px] font-mono" :style="{ color: item.node.color + 'cc' }">{{ item.node.commit.short_sha }}</span>
          </div>
        </div>

        <!-- Load more indicator -->
        <div
          v-if="hasMore"
          class="absolute left-0 right-0 flex items-center justify-center text-[10px] text-[#64748b]"
          :style="{ top: totalH + 'px', height: ROW_HEIGHT + 'px' }"
        >
          Loading more commits...
        </div>
      </div>
    </div>
  </div>
</template>

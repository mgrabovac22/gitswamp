<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch, nextTick } from "vue";
import type { CommitInfo, GraphNode, GraphEdge } from "@/types";

const LANE_WIDTH = 20;
const ROW_HEIGHT = 32;
const NODE_RADIUS = 4;
const BATCH_SIZE = 80;
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
}>();

const emit = defineEmits<{
  select: [commit: CommitInfo | null];
  search: [query: string];
  clearSearch: [];
  selectWorkingChanges: [];
}>();

const searchInput = ref("");
const scrollRef = ref<HTMLElement | null>(null);
const visibleCount = ref(BATCH_SIZE);
const drawnEdgeCount = ref(0);

function onScroll() {
  const el = scrollRef.value;
  if (!el) return;
  const scrollBottom = el.scrollTop + el.clientHeight;
  const totalH = totalHeight.value;
  if (scrollBottom >= totalH - 300 && visibleCount.value < props.commits.length) {
    visibleCount.value = Math.min(visibleCount.value + BATCH_SIZE, props.commits.length);
  }
  // Animate edge drawing based on scroll position
  const visibleBottom = Math.ceil(scrollBottom / ROW_HEIGHT) + 2;
  const targetEdges = graph.value.edges.filter(e => e.fromIndex < visibleBottom || e.toIndex < visibleBottom).length;
  if (targetEdges > drawnEdgeCount.value) {
    drawnEdgeCount.value = targetEdges;
  }
}

watch(() => props.commits, () => {
  visibleCount.value = BATCH_SIZE;
  drawnEdgeCount.value = 0;
  nextTick(() => {
    onScroll();
  });
});

onMounted(() => {
  scrollRef.value?.addEventListener("scroll", onScroll, { passive: true });
  nextTick(() => onScroll());
});
onUnmounted(() => {
  scrollRef.value?.removeEventListener("scroll", onScroll);
});

const visibleCommits = computed(() => props.commits.slice(0, visibleCount.value));

// Generate deterministic avatar color from name
function avatarColor(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  return COLORS[Math.abs(hash) % COLORS.length];
}

function avatarInitials(name: string): string {
  const parts = name.trim().split(/\s+/);
  if (parts.length >= 2) return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  return name.substring(0, 2).toUpperCase();
}

const graph = computed(() => {
  const list = visibleCommits.value;
  const allCommits = props.commits;
  if (!list.length) return { nodes: [] as GraphNode[], edges: [] as GraphEdge[], laneCount: 0, branchLanes: new Map<string, number>() };

  const shaIdx = new Map<string, number>();
  allCommits.forEach((c, i) => shaIdx.set(c.sha, i));

  // Assign branches to commits
  const branchHeads = new Map<string, number>();
  const commitBranch = new Map<number, string>();

  allCommits.forEach((commit, idx) => {
    for (const r of commit.refs) {
      const name = r.replace(/^origin\//, "");
      if (name === "HEAD" || name.includes("->")) continue;
      if (!branchHeads.has(name)) {
        branchHeads.set(name, idx);
        if (!commitBranch.has(idx)) commitBranch.set(idx, name);
      }
    }
  });

  // Trace first-parent chains to assign branch ownership
  for (const [branchName, headIdx] of branchHeads) {
    let current = headIdx;
    const visited = new Set<number>();
    while (current !== undefined && !visited.has(current)) {
      visited.add(current);
      const commit = allCommits[current];
      if (commit.parent_shas.length > 0) {
        const firstParent = shaIdx.get(commit.parent_shas[0]);
        if (firstParent !== undefined && !commitBranch.has(firstParent)) {
          commitBranch.set(firstParent, branchName);
          current = firstParent;
        } else {
          break;
        }
      } else {
        break;
      }
    }
  }

  allCommits.forEach((_, idx) => {
    if (!commitBranch.has(idx)) commitBranch.set(idx, "__default");
  });

  // Order branches: current branch first, then main/master, then by first appearance
  const branchOrder: string[] = [];
  const addBranch = (n: string) => {
    if (!branchOrder.includes(n)) branchOrder.push(n);
  };

  // Current branch gets lane 0
  if (props.currentBranch && [...commitBranch.values()].includes(props.currentBranch)) {
    addBranch(props.currentBranch);
  }
  for (const name of ["main", "master"]) {
    if ([...commitBranch.values()].includes(name)) addBranch(name);
  }
  allCommits.forEach((_, idx) => {
    const b = commitBranch.get(idx);
    if (b) addBranch(b);
  });

  const laneMap = new Map<string, number>();
  branchOrder.forEach((name, i) => laneMap.set(name, i));

  const nodes: GraphNode[] = list.map((commit, idx) => {
    const globalIdx = shaIdx.get(commit.sha) ?? idx;
    const branch = commitBranch.get(globalIdx) || "__default";
    const lane = laneMap.get(branch) || 0;
    return { commit, lane, color: COLORS[lane % COLORS.length] };
  });

  const edges: GraphEdge[] = [];
  list.forEach((commit, idx) => {
    for (const parentSha of commit.parent_shas) {
      const parentGlobal = shaIdx.get(parentSha);
      if (parentGlobal !== undefined && parentGlobal < list.length) {
        const parentLocalIdx = parentGlobal;
        if (parentLocalIdx < nodes.length) {
          edges.push({
            fromIndex: idx,
            toIndex: parentLocalIdx,
            fromLane: nodes[idx].lane,
            toLane: nodes[parentLocalIdx].lane,
            color: nodes[idx].color,
          });
        }
      }
    }
  });

  return { nodes, edges, laneCount: branchOrder.length, branchLanes: laneMap };
});

const graphWidth = computed(() => Math.max((graph.value.laneCount + 1) * LANE_WIDTH + 16, 60));
const workingChangesOffset = computed(() => props.hasWorkingChanges ? ROW_HEIGHT : 0);
const totalHeight = computed(() => (visibleCommits.value.length * ROW_HEIGHT) + workingChangesOffset.value);

function laneX(lane: number) {
  return lane * LANE_WIDTH + LANE_WIDTH / 2 + 8;
}

function rowY(index: number) {
  return index * ROW_HEIGHT + ROW_HEIGHT / 2 + workingChangesOffset.value;
}

function edgePath(edge: GraphEdge) {
  const x1 = laneX(edge.fromLane);
  const y1 = rowY(edge.fromIndex);
  const x2 = laneX(edge.toLane);
  const y2 = rowY(edge.toIndex);

  if (edge.fromLane === edge.toLane) {
    return `M ${x1} ${y1} L ${x2} ${y2}`;
  }

  // Smooth bezier curve for cross-lane connections
  const midY = (y1 + y2) / 2;
  const r = Math.min(10, Math.abs(x2 - x1) / 2);

  if (Math.abs(y2 - y1) <= ROW_HEIGHT * 1.5) {
    // Short distance: simple cubic bezier
    return `M ${x1} ${y1} C ${x1} ${midY}, ${x2} ${midY}, ${x2} ${y2}`;
  }

  // Long distance: go down, curve over, continue down
  const stepY = y1 + ROW_HEIGHT * 0.7;
  const dir = x2 > x1 ? 1 : -1;
  return `M ${x1} ${y1} L ${x1} ${stepY - r} Q ${x1} ${stepY}, ${x1 + dir * r} ${stepY} L ${x2 - dir * r} ${stepY} Q ${x2} ${stepY}, ${x2} ${stepY + r} L ${x2} ${y2}`;
}

// Working changes connection to the current branch lane
function workingChangesEdgePath(): string {
  const currentLane = graph.value.branchLanes.get(props.currentBranch) ?? 0;
  const x = laneX(currentLane);
  const y1 = ROW_HEIGHT / 2; // center of working changes row
  const y2 = rowY(0); // first commit
  return `M ${x} ${y1} L ${x} ${y2}`;
}

function workingChangesLaneX(): number {
  const currentLane = graph.value.branchLanes.get(props.currentBranch) ?? 0;
  return laneX(currentLane);
}

// Branch labels that should appear on the left side in the graph lane area
function branchLabels(commit: CommitInfo): string[] {
  return commit.refs.filter(
    (r) => !r.includes("HEAD") && !r.includes("->")
  );
}

// Edge animation: assign a draw delay based on index
function edgeDelay(index: number): string {
  return `${index * 0.02}s`;
}

function edgeLength(edge: GraphEdge): number {
  const x1 = laneX(edge.fromLane);
  const y1 = rowY(edge.fromIndex);
  const x2 = laneX(edge.toLane);
  const y2 = rowY(edge.toIndex);
  return Math.sqrt((x2 - x1) ** 2 + (y2 - y1) ** 2);
}

let searchTimeout: ReturnType<typeof setTimeout> | null = null;
function onSearchInput() {
  if (searchTimeout) clearTimeout(searchTimeout);
  searchTimeout = setTimeout(() => {
    if (searchInput.value.trim()) {
      emit("search", searchInput.value.trim());
    } else {
      emit("clearSearch");
    }
  }, 300);
}

function clearSearchInput() {
  searchInput.value = "";
  emit("clearSearch");
}
</script>

<template>
  <div class="flex-1 bg-[#111520] flex flex-col overflow-hidden">
    <div class="sticky top-0 bg-[#111520]/95 backdrop-blur-sm border-b border-[#8b5cf6]/10 z-10 flex-shrink-0">
      <!-- Search bar -->
      <div class="px-3 py-1.5 border-b border-[#8b5cf6]/5 flex items-center gap-2">
        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-[#64748b] flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
        <input
          v-model="searchInput"
          @input="onSearchInput"
          placeholder="Search commits, messages, authors, SHA..."
          class="flex-1 bg-transparent text-xs text-[#e2e8f0] placeholder:text-[#475569] focus:outline-none"
        />
        <button
          v-if="searchInput"
          @click="clearSearchInput"
          class="p-0.5 rounded hover:bg-[#252b3d] text-[#64748b] hover:text-[#e2e8f0] transition-colors"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
        <span v-if="searchQuery" class="text-[10px] text-[#64748b]">{{ commits.length }} results</span>
      </div>
      <div
        class="grid px-3 py-1.5 text-[10px] text-[#64748b] uppercase tracking-wider font-medium"
        :style="{ gridTemplateColumns: graphWidth + 'px 1fr 120px 70px' }"
      >
        <div>Graph</div>
        <div>Message</div>
        <div>Author</div>
        <div>SHA</div>
      </div>
    </div>

    <div v-if="!commits.length && !hasWorkingChanges" class="flex items-center justify-center h-64 text-[#64748b] text-sm">
      No commits to display
    </div>

    <div v-else class="flex-1 overflow-y-auto" ref="scrollRef">
      <div class="relative" :style="{ height: totalHeight + 'px', minHeight: '100%' }">
        <!-- SVG graph layer -->
        <svg
          class="absolute left-3 top-0 pointer-events-none"
          :width="graphWidth"
          :height="totalHeight"
        >
          <!-- Working changes connection line -->
          <path
            v-if="hasWorkingChanges && graph.nodes.length > 0"
            :d="workingChangesEdgePath()"
            stroke="#8b5cf6"
            stroke-width="2"
            fill="none"
            opacity="0.4"
            stroke-dasharray="4 3"
            stroke-linecap="round"
          />

          <!-- Edges with animation -->
          <path
            v-for="(edge, i) in graph.edges"
            :key="'e' + i"
            :d="edgePath(edge)"
            :stroke="edge.color"
            stroke-width="2"
            fill="none"
            opacity="0.55"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="graph-edge"
            :style="{
              strokeDasharray: edgeLength(edge),
              strokeDashoffset: edgeLength(edge),
              animationDelay: edgeDelay(i),
            }"
          />

          <!-- Working changes node -->
          <template v-if="hasWorkingChanges">
            <rect
              :x="workingChangesLaneX() - 5"
              :y="ROW_HEIGHT / 2 - 5"
              width="10"
              height="10"
              rx="2"
              fill="#8b5cf6"
              opacity="0.9"
              class="animate-pulse"
            />
          </template>

          <!-- Commit nodes -->
          <template v-for="(node, index) in graph.nodes" :key="'n' + node.commit.sha">
            <circle
              :cx="laneX(node.lane)"
              :cy="rowY(index)"
              :r="NODE_RADIUS"
              :fill="node.color"
              :stroke="selected?.sha === node.commit.sha ? '#fff' : node.color"
              :stroke-width="selected?.sha === node.commit.sha ? 2 : 1"
              class="graph-node"
              :style="{ animationDelay: (index * 0.01) + 's' }"
            />
            <!-- Branch head indicator ring -->
            <circle
              v-if="node.commit.refs.length > 0"
              :cx="laneX(node.lane)"
              :cy="rowY(index)"
              :r="NODE_RADIUS + 3"
              fill="none"
              :stroke="node.color"
              stroke-width="1.5"
              opacity="0.3"
            />
          </template>
        </svg>

        <!-- Working changes row -->
        <div
          v-if="hasWorkingChanges"
          class="absolute left-0 right-0 grid px-3 items-center cursor-pointer transition-colors"
          :class="selected === null ? 'bg-[#8b5cf6]/10' : 'hover:bg-[#1a2030]'"
          :style="{
            top: '0px',
            height: ROW_HEIGHT + 'px',
            gridTemplateColumns: graphWidth + 'px 1fr 120px 70px',
          }"
          @click="emit('selectWorkingChanges')"
        >
          <div></div>
          <div class="flex items-center gap-1.5 min-w-0">
            <span class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-[#8b5cf6]/20 text-[#a78bfa] border border-[#8b5cf6]/30">
              ● Working Changes
            </span>
          </div>
          <div class="text-xs text-[#64748b]">—</div>
          <div class="text-[11px] font-mono text-[#64748b]">—</div>
        </div>

        <!-- Commit rows -->
        <div
          v-for="(node, index) in graph.nodes"
          :key="node.commit.sha"
          class="absolute left-0 right-0 grid px-3 items-center cursor-pointer transition-colors"
          :class="selected?.sha === node.commit.sha ? 'bg-[#8b5cf6]/10' : 'hover:bg-[#1a2030]'"
          :style="{
            top: (index * ROW_HEIGHT + workingChangesOffset) + 'px',
            height: ROW_HEIGHT + 'px',
            gridTemplateColumns: graphWidth + 'px 1fr 120px 70px',
          }"
          @click="emit('select', node.commit)"
        >
          <!-- Branch labels in graph column -->
          <div class="flex items-center gap-0.5 overflow-hidden pl-1">
            <span
              v-for="label in branchLabels(node.commit)"
              :key="label"
              class="flex-shrink-0 px-1 py-0 rounded text-[9px] font-medium truncate max-w-[80px]"
              :style="{
                backgroundColor: node.color + '18',
                color: node.color,
                border: '1px solid ' + node.color + '30',
              }"
              :title="label"
            >
              {{ label }}
            </span>
          </div>

          <!-- Message -->
          <div class="flex items-center gap-1.5 min-w-0">
            <span class="text-[12px] text-[#cbd5e1] truncate leading-tight">
              {{ node.commit.message.split('\n')[0] }}
            </span>
          </div>

          <!-- Author with avatar -->
          <div class="flex items-center gap-1.5 min-w-0">
            <div
              class="w-4 h-4 rounded-full flex items-center justify-center flex-shrink-0 text-[7px] font-bold text-white"
              :style="{ backgroundColor: avatarColor(node.commit.author_name) }"
            >
              {{ avatarInitials(node.commit.author_name) }}
            </div>
            <span class="text-[11px] text-[#64748b] truncate">{{ node.commit.author_name }}</span>
          </div>

          <!-- SHA -->
          <div class="text-[10px] font-mono" :style="{ color: node.color + 'cc' }">
            {{ node.commit.short_sha }}
          </div>
        </div>

        <!-- Loading more indicator -->
        <div
          v-if="visibleCount < commits.length"
          class="absolute left-0 right-0 flex items-center justify-center py-2 text-xs text-[#64748b]"
          :style="{ top: totalHeight + 'px' }"
        >
          Scroll for more...
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.graph-edge {
  animation: drawEdge 0.4s ease-out forwards;
}

.graph-node {
  opacity: 0;
  animation: fadeNode 0.2s ease-out forwards;
}

@keyframes drawEdge {
  to {
    stroke-dashoffset: 0;
  }
}

@keyframes fadeNode {
  to {
    opacity: 1;
  }
}
</style>

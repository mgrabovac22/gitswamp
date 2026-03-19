<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import type { CommitInfo, GraphNode, GraphEdge, StashInfo, TagInfo } from "@/types";

const LANE_WIDTH = 20;
const ROW_HEIGHT_NORMAL = 28;
const ROW_HEIGHT_COMPACT = 22;
const NODE_RADIUS = 10;
const BRANCH_COL = 140;
const AUTHOR_COL = 140;
const SHA_COL = 75;
const OVERSCAN = 15;
const CORNER_R = 6;
const COLORS = [
  "#8b5cf6", "#06b6d4", "#f59e0b", "#ef4444", "#10b981",
  "#ec4899", "#f97316", "#14b8a6", "#a855f7", "#22d3ee",
  "#84cc16", "#e879f9", "#fb923c", "#2dd4bf",
];

const rowHeight = ref(ROW_HEIGHT_NORMAL);

function updateRowHeight() {
  rowHeight.value = document.documentElement.classList.contains("compact") 
    ? ROW_HEIGHT_COMPACT 
    : ROW_HEIGHT_NORMAL;
}

let compactObserver: MutationObserver | null = null;

onMounted(() => {
  updateRowHeight();
  compactObserver = new MutationObserver(() => {
    updateRowHeight();
  });
  compactObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["class"],
  });
});

onUnmounted(() => {
  compactObserver?.disconnect();
});

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

const isLight = ref(document.documentElement.classList.contains('light'));
const themeObserver = new MutationObserver(() => {
  isLight.value = document.documentElement.classList.contains('light');
});
onMounted(() => {
  themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
});
onUnmounted(() => {
  themeObserver.disconnect();
});
const svgBgOuter = computed(() => isLight.value ? '#bcc6d4' : '#111520');
const svgBgInner = computed(() => isLight.value ? '#cdd5e0' : '#1a1f30');

function nameHash(name: string): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = name.charCodeAt(i) + ((h << 5) - h);
  return Math.abs(h);
}

function avatarColor(name: string): string {
  return COLORS[nameHash(name) % COLORS.length];
}

function avatarSvg(name: string, cx: number, cy: number, r: number, branchColor: string): string {
  const h = nameHash(name);
  const c = avatarColor(name);
  const s = r * 0.48; // cell size
  const ox = cx - s * 1.5;
  const oy = cy - s * 1.5;
  let cells = '';
  for (let row = 0; row < 3; row++) {
    for (let col = 0; col < 2; col++) {
      const bit = (h >> (row * 2 + col)) & 1;
      if (bit) {
        const x1 = ox + col * s;
        const y1 = oy + row * s;
        const x2 = ox + (2 - col) * s;
        cells += '<rect x="' + x1 + '" y="' + y1 + '" width="' + s + '" height="' + s + '" rx="1" fill="' + c + '" opacity="0.85"/>';
        cells += '<rect x="' + x2 + '" y="' + y1 + '" width="' + s + '" height="' + s + '" rx="1" fill="' + c + '" opacity="0.85"/>';
      }
    }
    const centerBit = (h >> (row + 6)) & 1;
    if (centerBit) {
      cells += '<rect x="' + (ox + s) + '" y="' + (oy + row * s) + '" width="' + s + '" height="' + s + '" rx="1" fill="' + c + '" opacity="0.9"/>';
    }
  }
  return '<g class="commit-avatar"><circle cx="' + cx + '" cy="' + cy + '" r="' + (r + 1.5) + '" fill="' + svgBgOuter.value + '"/>'
    + '<circle cx="' + cx + '" cy="' + cy + '" r="' + r + '" fill="' + svgBgInner.value + '"/>'
    + cells
    + '<circle cx="' + cx + '" cy="' + cy + '" r="' + r + '" fill="none" stroke="' + branchColor + '" stroke-width="1.5" opacity="0.8"/></g>';
}

function mergeDotSvg(cx: number, cy: number, color: string): string {
  return '<circle cx="' + cx + '" cy="' + cy + '" r="5" fill="' + svgBgOuter.value + '"/>'
    + '<circle cx="' + cx + '" cy="' + cy + '" r="4" fill="' + color + '" opacity="0.9"/>';
}

function isMergeCommit(commit: CommitInfo): boolean {
  return commit.parent_shas.length > 1;
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

  const branchRange = new Map<string, { min: number; max: number }>();
  all.forEach((_, idx) => {
    const b = commitBranch.get(idx)!;
    const cur = branchRange.get(b);
    if (!cur) {
      branchRange.set(b, { min: idx, max: idx });
    } else {
      cur.min = Math.min(cur.min, idx);
      cur.max = Math.max(cur.max, idx);
    }
  });

  const branchOrder: string[] = [];
  const addBr = (n: string) => { if (!branchOrder.includes(n) && branchRange.has(n)) branchOrder.push(n); };
  if (props.currentBranch) addBr(props.currentBranch);
  for (const n of ["main", "master"]) addBr(n);
  all.forEach((_, idx) => { const b = commitBranch.get(idx); if (b) addBr(b); });

  const laneOccupied: Array<Array<{ min: number; max: number }>> = [];
  const laneMap = new Map<string, number>();

  function rangesOverlap(a: { min: number; max: number }, b: { min: number; max: number }): boolean {
    return a.min <= b.max && b.min <= a.max;
  }

  for (const branch of branchOrder) {
    const range = branchRange.get(branch)!;
    let assigned = -1;
    for (let lane = 0; lane < laneOccupied.length; lane++) {
      const conflicts = laneOccupied[lane].some(r => rangesOverlap(r, range));
      if (!conflicts) {
        assigned = lane;
        break;
      }
    }
    if (assigned === -1) {
      assigned = laneOccupied.length;
      laneOccupied.push([]);
    }
    laneOccupied[assigned].push(range);
    laneMap.set(branch, assigned);
  }

  const laneCount = laneOccupied.length;

  const nodes: GraphNode[] = all.map((commit, idx) => {
    const branch = commitBranch.get(idx) || "__default";
    const lane = laneMap.get(branch) ?? 0;
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

  return { nodes, edges, laneCount, branchLanes: laneMap };
});

const stashNodes = computed(() => {
  if (!props.stashes?.length) return [];
  const shaIdx = new Map<string, number>();
  props.commits.forEach((c, i) => shaIdx.set(c.sha, i));

  const parentCount = new Map<number, number>();

  return props.stashes.map(s => {
    const parentIdx = shaIdx.get(s.parent_sha) ?? -1;
    if (parentIdx < 0) return null;
    const parentLane = graph.value.nodes[parentIdx]?.lane ?? 0;

    const offsetIdx = parentCount.get(parentIdx) ?? 0;
    parentCount.set(parentIdx, offsetIdx + 1);

    const stashLane = parentLane + 1 + offsetIdx;
    return { ...s, parentIdx, lane: stashLane, offsetIdx };
  }).filter(Boolean) as (StashInfo & { parentIdx: number; lane: number; offsetIdx: number })[];
});

const wcLane = computed(() => {
  if (!props.hasWorkingChanges) return 0;
  const currentBranchLane = graph.value.branchLanes.get(props.currentBranch) ?? 0;
  return currentBranchLane;
});

const headCommitIndex = computed(() => {
  for (let i = 0; i < props.commits.length; i++) {
    const refs = mergedRefs(props.commits[i]);
    if (refs.some(r => r.name === props.currentBranch && r.local)) {
      return i;
    }
  }
  return 0;
});

const graphWidth = computed(() => {
  const maxStashLane = stashNodes.value.length > 0
    ? Math.max(...stashNodes.value.map(s => s.lane))
    : 0;
  const totalLanes = Math.max(graph.value.laneCount, maxStashLane + 1);
  return Math.max((totalLanes + 1) * LANE_WIDTH + 8, 40);
});
const wcRows = computed(() => {
  let n = 0;
  if (props.hasWorkingChanges) n += 1;
  if (props.hasConflicts) n += 1;
  return n;
});
const wcOffset = computed(() => wcRows.value * rowHeight.value);
const totalH = computed(() => props.commits.length * rowHeight.value + wcOffset.value);

const visibleRange = computed(() => {
  const startY = scrollTop.value - wcOffset.value;
  const first = Math.max(0, Math.floor(startY / rowHeight.value) - OVERSCAN);
  const last = Math.min(
    props.commits.length - 1,
    Math.ceil((startY + viewportHeight.value) / rowHeight.value) + OVERSCAN
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


const stashesByParent = computed(() => {
  const map = new Map<number, (StashInfo & { parentIdx: number; lane: number; offsetIdx: number })[]>();
  for (const s of stashNodes.value) {
    const arr = map.get(s.parentIdx);
    if (arr) arr.push(s);
    else map.set(s.parentIdx, [s]);
  }
  return map;
});

function stashesAtCommit(idx: number) {
  return stashesByParent.value.get(idx) ?? [];
}

function lx(lane: number) { return lane * LANE_WIDTH + LANE_WIDTH / 2 + 4; }
function ry(index: number) { return index * rowHeight.value + rowHeight.value / 2 + wcOffset.value; }


function ep(e: GraphEdge): string {
  const x1 = lx(e.fromLane), y1 = ry(e.fromIndex);
  const x2 = lx(e.toLane), y2 = ry(e.toIndex);
  if (e.fromLane === e.toLane) return 'M ' + x1 + ' ' + y1 + ' L ' + x2 + ' ' + y2;
  const r = Math.min(CORNER_R, Math.abs(x2 - x1) / 2, Math.abs(y2 - y1) / 4);
  const d = x2 > x1 ? 1 : -1;
  const turnY = y1 + rowHeight.value * 0.5;
  return 'M ' + x1 + ' ' + y1
    + ' L ' + x1 + ' ' + (turnY - r)
    + ' Q ' + x1 + ' ' + turnY + ' ' + (x1 + d * r) + ' ' + turnY
    + ' L ' + (x2 - d * r) + ' ' + turnY
    + ' Q ' + x2 + ' ' + turnY + ' ' + x2 + ' ' + (turnY + r)
    + ' L ' + x2 + ' ' + y2;
}

function wcEdge(): string {
  const x = lx(wcLane.value);
  const startY = rowHeight.value / 2 + 7;
  const endY = ry(headCommitIndex.value) - 10;
  return 'M ' + x + ' ' + startY + ' L ' + x + ' ' + endY;
}

function conflictEdge(): string {
  const conflictX = lx(wcLane.value) + 22;
  const conflictY = ((props.hasWorkingChanges ? 1 : 0) * rowHeight.value) + rowHeight.value / 2;
  if (props.hasWorkingChanges) {
    const workingX = lx(wcLane.value);
    const workingY = rowHeight.value / 2 + 7;
    const endX = conflictX - 7;
    const endY = conflictY;
    const turnY = workingY + Math.max(8, rowHeight.value * 0.28);
    const r = Math.min(CORNER_R, Math.abs(endX - workingX) / 2, Math.abs(endY - turnY) / 2);
    return 'M ' + workingX + ' ' + workingY
      + ' L ' + workingX + ' ' + (turnY - r)
      + ' Q ' + workingX + ' ' + turnY + ' ' + (workingX + r) + ' ' + turnY
      + ' L ' + (endX - r) + ' ' + turnY
      + ' Q ' + endX + ' ' + turnY + ' ' + endX + ' ' + (turnY + r)
      + ' L ' + endX + ' ' + endY;
  }
  const x2 = lx(wcLane.value) - 7;
  const y2 = ry(headCommitIndex.value) - 10;
  const r = Math.min(CORNER_R, Math.abs(conflictX - x2) / 2, Math.abs(conflictY - y2) / 4);
  const turnY = conflictY - Math.max(10, rowHeight.value * 0.35);
  return 'M ' + conflictX + ' ' + conflictY
    + ' L ' + conflictX + ' ' + (turnY + r)
    + ' Q ' + conflictX + ' ' + turnY + ' ' + (conflictX - r) + ' ' + turnY
    + ' L ' + (x2 + r) + ' ' + turnY
    + ' Q ' + x2 + ' ' + turnY + ' ' + x2 + ' ' + (turnY - r)
    + ' L ' + x2 + ' ' + y2;
}

function conflictSpineEdge(): string {
  if (!props.hasConflicts || !props.hasWorkingChanges) return "";
  const x = lx(wcLane.value);
  const startY = rowHeight.value / 2 + 7;
  const endY = rowHeight.value + rowHeight.value / 2 - 7;
  return 'M ' + x + ' ' + startY + ' L ' + x + ' ' + endY;
}

function wcLaneX(): number { return lx(wcLane.value); }

function brefs(commit: CommitInfo): string[] {
  return commit.refs.filter(r => !r.includes("HEAD") && !r.includes("->"));
}

const tagNameSet = computed(() => new Set((props.tags ?? []).map(t => t.name)));

function commitTags(commit: CommitInfo): TagInfo[] {
  const tags = props.tags ?? [];
  return tags.filter(t => t.sha === commit.sha);
}

function branchRefs(commit: CommitInfo): string[] {
  const tags = tagNameSet.value;
  return brefs(commit).filter(r => !tags.has(r));
}

interface MergedRef { name: string; local: boolean; remote: boolean; }
interface DisplayRef {
  kind: "branch" | "tag";
  key: string;
  name: string;
  local?: boolean;
  remote?: boolean;
}

function mergedRefs(commit: CommitInfo): MergedRef[] {
  const raw = branchRefs(commit);
  const map = new Map<string, MergedRef>();
  for (const r of raw) {
    if (r.startsWith("origin/")) {
      const name = r.substring(7);
      const existing = map.get(name);
      if (existing) { existing.remote = true; }
      else { map.set(name, { name, local: false, remote: true }); }
    } else {
      const existing = map.get(r);
      if (existing) { existing.local = true; }
      else { map.set(r, { name: r, local: true, remote: false }); }
    }
  }
  return Array.from(map.values());
}

function displayRefs(commit: CommitInfo): DisplayRef[] {
  const refs: DisplayRef[] = mergedRefs(commit).map(r => ({
    kind: "branch",
    key: "branch:" + r.name,
    name: r.name,
    local: r.local,
    remote: r.remote,
  }));
  for (const t of commitTags(commit)) {
    refs.push({
      kind: "tag",
      key: "tag:" + t.name,
      name: t.name,
    });
  }
  refs.sort((a, b) => {
    const aIsCurrent = a.kind === "branch" && a.local && a.name === props.currentBranch;
    const bIsCurrent = b.kind === "branch" && b.local && b.name === props.currentBranch;
    if (aIsCurrent && !bIsCurrent) return -1;
    if (!aIsCurrent && bIsCurrent) return 1;
    if (a.kind === "branch" && b.kind === "branch") {
      if (a.local && !b.local) return -1;
      if (!a.local && b.local) return 1;
    }
    return 0;
  });
  return refs;
}

function topDisplayRef(commit: CommitInfo): DisplayRef | null {
  const refs = displayRefs(commit);
  return refs.length > 0 ? refs[0] : null;
}

function extraDisplayRefCount(commit: CommitInfo): number {
  return Math.max(0, displayRefs(commit).length - 1);
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

function onStashClick(event: MouseEvent, stash: StashInfo & { parentIdx: number; lane: number; offsetIdx: number }) {
  event.stopPropagation();
  emit("selectStash", stash);
  emit("select", null);
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

function providerIconSvg(): string {
  switch (props.remoteProvider) {
    case "github":
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-75" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1.2a6.8 6.8 0 00-2.15 13.25c.34.06.46-.14.46-.33v-1.15c-1.88.4-2.27-.8-2.27-.8-.31-.76-.75-.96-.75-.96-.61-.42.05-.41.05-.41.67.04 1.02.69 1.02.69.6 1.03 1.57.73 1.95.56.06-.43.23-.73.41-.9-1.5-.18-3.08-.75-3.08-3.35 0-.74.26-1.34.69-1.81-.07-.17-.3-.88.06-1.83 0 0 .56-.18 1.85.69a6.4 6.4 0 013.36 0c1.28-.87 1.84-.69 1.84-.69.37.95.14 1.66.07 1.83.43.47.69 1.07.69 1.81 0 2.6-1.59 3.17-3.1 3.34.24.21.45.63.45 1.28v1.9c0 .19.12.39.47.33A6.8 6.8 0 008 1.2z"/></svg>';
    case "gitlab":
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-75" viewBox="0 0 16 16" fill="currentColor"><path d="M8 14.5l-2.3-7h4.6L8 14.5zM1.4 7.5L.5 10.3c-.1.2 0 .5.2.6L8 14.5 1.4 7.5zm.8-2.2L.5 10.3h3.6l1.1-3.4-3 1.6zm11.4 2.2L15.5 10.3c.1.2 0 .5-.2.6L8 14.5l6.6-7zm-.8-2.2l1.7 5h-3.6l-1.1-3.4 3-1.6z"/></svg>';
    case "bitbucket":
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-75" viewBox="0 0 16 16" fill="currentColor"><path d="M.8 1.5h14.4c.5 0 .8.4.8.8v.2l-2 12c-.1.4-.4.7-.8.7H3c-.4 0-.8-.3-.8-.7l-2-12c-.1-.4.2-.8.6-.8zm8.4 9H6.8L6 6.5h4l-.8 4z"/></svg>';
    case "azure":
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-75" viewBox="0 0 16 16" fill="currentColor"><path d="M4.8 1.5L1 6.3l2.3 8.2h9.4l2.3-8.2L11.2 1.5H4.8zM8 5l2 3.5H6L8 5z"/></svg>';
    default:
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-70" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1C5.2 1 3 3 3 5.5c0 .8.2 1.5.5 2.1C2.1 8.2 1 9.5 1 11c0 2 1.6 3.5 3.6 3.5h7.8c2 0 3.6-1.5 3.6-3.5 0-1.5-1.1-2.8-2.5-3.4.3-.6.5-1.3.5-2.1C13 3 10.8 1 8 1z"/></svg>';
  }
}

const providerIconMarkup = computed(() => providerIconSvg());

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
                :title="stashesAtCommit(item.idx).map(s => 'stash@{' + s.index + '}: ' + (s.message || '')).join('\n')"
                @contextmenu.stop.prevent="onStashBadgeContextMenu($event, stashesAtCommit(item.idx))"
              >
                <svg class="w-3 h-3 flex-shrink-0" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <rect x="1" y="5" width="12" height="8" rx="1.5" fill="rgba(245,158,11,0.25)" stroke="#f59e0b" stroke-width="1.2"/>
                  <rect x="1" y="2" width="12" height="3.5" rx="1.2" fill="rgba(245,158,11,0.40)" stroke="#f59e0b" stroke-width="1.2"/>
                  <rect x="4.5" y="1" width="5" height="2" rx="0.8" fill="rgba(245,158,11,0.55)" stroke="#f59e0b" stroke-width="1"/>
                  <line x1="3.5" y1="9" x2="10.5" y2="9" stroke="#f59e0b" stroke-width="1" opacity="0.6"/>
                </svg>
                <span class="text-[9px] font-semibold truncate max-w-[75px]" style="color:#f59e0b;">
                  {{ stashesAtCommit(item.idx).length > 1
                    ? stashesAtCommit(item.idx).length + '×'
                    : (stashesAtCommit(item.idx)[0].message || 'stash') }}
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
        class="fixed z-[120] min-w-[180px] bg-[var(--popover)] border border-[var(--border)] rounded-lg shadow-2xl py-1 text-[11px] text-[var(--foreground)]"
        :style="{ left: refCtxX + 'px', top: refCtxY + 'px' }"
        @click.stop
      >
        <template v-if="refCtxRef.kind === 'branch'">
          <button v-if="refCtxRef.local" class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="refAction('checkout-local')">Checkout {{ refCtxRef.name }}</button>
          <button v-if="!refCtxRef.local" class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="refAction('checkout-remote')">Checkout origin/{{ refCtxRef.name }}</button>
          <button v-if="canMergeRefIntoCurrent(refCtxRef)" class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="refAction('merge-into-current')">Merge into {{ currentBranch }}</button>
          <button v-if="refCtxRef.local" class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="refAction('delete-local')">Delete {{ refCtxRef.name }}</button>
          <button v-if="refCtxRef.remote" class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="refAction('delete-remote')">Delete origin/{{ refCtxRef.name }}</button>
          <div class="border-t border-[var(--border)] my-1" />
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="refAction('copy-name')">Copy name</button>
        </template>
        <template v-else>
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="refAction('delete-tag')">Delete tag {{ refCtxRef.name }}</button>
          <div class="border-t border-[var(--border)] my-1" />
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="refAction('copy-name')">Copy name</button>
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
        class="fixed z-[100] min-w-[240px] bg-[var(--popover)] border border-[var(--border)] rounded-lg shadow-2xl py-1 text-[11px] text-[var(--foreground)] max-h-[80vh] overflow-y-auto"
        :style="{ left: ctxX + 'px', top: ctxY + 'px' }"
        @click.stop
      >
        <template v-if="ctxHasBranch()">
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('pull')">Pull (fast-forward if possible)</button>
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('push')">Push</button>
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('set-upstream')">Set Upstream</button>
          <div class="border-t border-[var(--border)] my-1" />
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('checkout-branch')">Checkout</button>
          <div class="border-t border-[var(--border)] my-1" />
        </template>

        <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('checkout')">Checkout this commit</button>
        <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('branch')">Create branch here</button>
        <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('cherry-pick')">Cherry pick commit</button>

        <div class="relative">
          <button
            class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors flex items-center justify-between"
            @click.stop="ctxResetSub = !ctxResetSub"
          >
            <span>Reset {{ currentBranch }} to this commit</span>
            <svg :class="['w-3 h-3 text-[var(--muted-foreground)] transition-transform', ctxResetSub ? 'rotate-90' : '']" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>
          </button>
          <div
            v-if="ctxResetSub"
            class="bg-[var(--popover)]/80 py-1 pl-4"
          >
            <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('reset-soft')">Soft – keep all changes staged</button>
            <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('reset-mixed')">Mixed – keep working copy but reset index</button>
            <button class="w-full text-left px-3 py-1.5 hover:bg-[#ef4444]/15 text-[#ef4444] transition-colors" @click="ctxAction('reset-hard')">Hard – discard all changes</button>
          </div>
        </div>

        <template v-if="ctxHasBranch() && ctxIsHeadCommit()">
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('edit-message')">Edit commit message</button>
        </template>

        <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('revert')">Revert commit</button>

        <div class="border-t border-[var(--border)] my-1" />

        <template v-if="ctxHasBranch()">
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('rename-branch')">Rename {{ ctxBranchName() }}</button>
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('delete-branch')">Delete {{ ctxBranchName() }}</button>
          <button v-if="ctxBranchRef()?.remote" class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('delete-remote-branch')">Delete origin/{{ ctxBranchName() }}</button>
          <button v-if="ctxBranchRef()?.local && ctxBranchRef()?.remote" class="w-full text-left px-3 py-1.5 hover:bg-[#ef4444]/15 text-[#ef4444] transition-colors" @click="ctxAction('delete-both')">Delete {{ ctxBranchName() }} and origin/{{ ctxBranchName() }}</button>
          <div class="border-t border-[var(--border)] my-1" />
          <button v-if="ctxBranchRef()?.remote && ctxBranchRef()?.local" class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('reset-to-remote')">Reset {{ ctxBranchName() }} to origin/{{ ctxBranchName() }}</button>
        </template>

        <template v-if="ctxHasBranch()">
          <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('copy-branch-name')">Copy branch name</button>
        </template>
        <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('copy-sha')">Copy commit SHA</button>

        <div class="border-t border-[var(--border)] my-1" />

        <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('tag')">Create tag here</button>
        <button class="w-full text-left px-3 py-1.5 hover:bg-[var(--primary)]/15 transition-colors" @click="ctxAction('annotated-tag')">Create annotated tag here</button>
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
</style>


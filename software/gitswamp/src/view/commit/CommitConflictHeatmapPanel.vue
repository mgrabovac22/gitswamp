<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { FileCode2, Folder, FolderOpen } from "lucide-vue-next";
import logoCrocLoading from "@/assets/logo_croc_loading.gif";
import type { ConflictHotspot, ConflictPair } from "@/types";

const FULL_HISTORY_LIMIT = 60000;
const TREE_SCAN_LIMIT = 160000;

const MERGE_WINDOWS = [
  { key: "300", label: "300", maxCount: 300 },
  { key: "500", label: "500", maxCount: 500 },
  { key: "1000", label: "1000", maxCount: 1000 },
  { key: "all", label: "All", maxCount: FULL_HISTORY_LIMIT },
] as const;
const loadingLetters = "LOADING".split("");

const hotspotCache = new Map<string, ConflictHotspot[]>();
const pairCache = new Map<string, ConflictPair[]>();
const treeCache = new Map<string, string[]>();

type MergeWindowKey = (typeof MERGE_WINDOWS)[number]["key"];

type TreeNodeKind = "dir" | "file";

type RiskBand = "low" | "moderate" | "high" | "critical";

interface RepoTreeNode {
  id: string;
  name: string;
  path: string;
  kind: TreeNodeKind;
  riskScore: number;
  collisionIndex: number;
  children: RepoTreeNode[];
}

interface FlatTreeRow {
  id: string;
  name: string;
  path: string;
  kind: TreeNodeKind;
  depth: number;
  riskScore: number;
  collisionIndex: number;
  hasChildren: boolean;
  expanded: boolean;
}

interface BuildTreeNode {
  id: string;
  name: string;
  path: string;
  kind: TreeNodeKind;
  children: Map<string, BuildTreeNode>;
  ownRiskScore: number;
  ownCollisionIndex: number;
}

const props = defineProps<{
  repoPath: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

const query = ref("");
const selectedMergeWindow = ref<MergeWindowKey>("500");
const selectedTreePath = ref("");
const expandedNodeIds = ref<string[]>([]);

const hotspots = ref<ConflictHotspot[]>([]);
const hotspotsLoading = ref(false);
const hotspotsError = ref("");

const riskyPairs = ref<ConflictPair[]>([]);
const pairsLoading = ref(false);
const pairsError = ref("");

const repositoryPaths = ref<string[]>([]);
const treeLoading = ref(false);
const treeError = ref("");

let hotspotRunToken = 0;
let pairRunToken = 0;
let treeRunToken = 0;

const selectedMergeConfig = computed(
  () => MERGE_WINDOWS.find((item) => item.key === selectedMergeWindow.value) ?? MERGE_WINDOWS[0],
);

const maxHotspotScore = computed(() => hotspots.value.reduce((max, item) => Math.max(max, item.score), 0));
const maxPairScore = computed(() => riskyPairs.value.reduce((max, item) => Math.max(max, item.score), 0));

const filteredHotspots = computed(() => {
  const q = query.value.trim().toLowerCase();
  let items = hotspots.value;

  if (q) {
    items = items.filter((item) => item.path.toLowerCase().includes(q));
  }

  if (selectedTreePath.value) {
    items = items.filter(
      (item) => item.path === selectedTreePath.value || item.path.startsWith(selectedTreePath.value + "/"),
    );
  }

  return items;
});

const topHotspot = computed(() => hotspots.value[0] ?? null);

const totalMergeTouches = computed(() =>
  hotspots.value.reduce((sum, item) => sum + item.merge_touches, 0),
);

const totalConflictMentions = computed(() =>
  hotspots.value.reduce((sum, item) => sum + item.conflict_mentions, 0),
);

const totalCollisionIndex = computed(() =>
  hotspots.value.reduce((sum, item) => sum + item.collision_index, 0),
);

const averageCollisionIndex = computed(() => {
  if (hotspots.value.length === 0) return 0;
  return Math.round((totalCollisionIndex.value / hotspots.value.length) * 10) / 10;
});

const conflictMentionsPerTouch = computed(() => {
  if (totalMergeTouches.value === 0) {
    return 0;
  }
  return Math.round((totalConflictMentions.value / totalMergeTouches.value) * 100) / 100;
});

const highRiskHotspots = computed(() =>
  hotspots.value.filter((item) => riskRatio(item.score) >= 0.66).length,
);

const topPair = computed(() => riskyPairs.value[0] ?? null);

const topPairCoupling = computed(() => {
  if (!topPair.value) {
    return 0;
  }

  if (topPair.value.co_touches <= 0) {
    return 0;
  }

  return Math.round((topPair.value.conflict_touches / topPair.value.co_touches) * 100);
});

const dominantRiskDomain = computed(() => {
  const byRoot = new Map<string, number>();

  for (const item of hotspots.value) {
    const [root] = normalizePath(item.path).split("/");
    const key = root || item.path;
    byRoot.set(key, (byRoot.get(key) ?? 0) + item.score);
  }

  let name = "n/a";
  let score = 0;

  byRoot.forEach((value, key) => {
    if (value > score) {
      name = key;
      score = value;
    }
  });

  return { name, score };
});

const topHotspotReason = computed(() => {
  const top = topHotspot.value;
  if (!top) {
    return "No hotspot diagnostics yet.";
  }

  const perFileTouches = totalMergeTouches.value / Math.max(1, hotspots.value.length);
  const perFileMentions = totalConflictMentions.value / Math.max(1, hotspots.value.length);
  const perFileCollision = totalCollisionIndex.value / Math.max(1, hotspots.value.length);
  const reasons: string[] = [];

  if (top.merge_touches >= Math.max(6, perFileTouches * 1.3)) {
    reasons.push("frequent merge touches");
  }

  if (top.conflict_mentions >= Math.max(3, perFileMentions * 1.4)) {
    reasons.push("high conflict mention count");
  }

  if (top.collision_index >= Math.max(4, perFileCollision * 1.25)) {
    reasons.push("elevated collision index");
  }

  if (reasons.length === 0) {
    return "steady medium pressure across merge + conflict metrics";
  }

  return reasons.join(" + ");
});

function upsertBuildNode(
  container: Map<string, BuildTreeNode>,
  name: string,
  path: string,
  isFile: boolean,
): BuildTreeNode {
  let node = container.get(name);

  if (!node) {
    node = {
      id: path,
      name,
      path,
      kind: isFile ? "file" : "dir",
      children: new Map<string, BuildTreeNode>(),
      ownRiskScore: 0,
      ownCollisionIndex: 0,
    };
    container.set(name, node);
  }

  if (isFile) {
    node.kind = "file";
  }

  return node;
}

function buildTreeSkeleton(paths: string[]): {
  roots: Map<string, BuildTreeNode>;
  byPath: Map<string, BuildTreeNode>;
} {
  const roots = new Map<string, BuildTreeNode>();
  const byPath = new Map<string, BuildTreeNode>();

  for (const rawPath of paths) {
    const normalized = normalizePath(rawPath);
    if (!normalized) {
      continue;
    }

    const segments = normalized.split("/").filter(Boolean);
    if (segments.length === 0) {
      continue;
    }

    let childMap = roots;
    let currentPath = "";

    for (let idx = 0; idx < segments.length; idx += 1) {
      const name = segments[idx];
      currentPath = currentPath ? `${currentPath}/${name}` : name;
      const isFile = idx === segments.length - 1;

      const node = upsertBuildNode(childMap, name, currentPath, isFile);
      byPath.set(currentPath, node);
      childMap = node.children;
    }
  }

  return { roots, byPath };
}

function applyHotspotRiskToTree(byPath: Map<string, BuildTreeNode>, items: ConflictHotspot[]) {
  for (const hotspot of items) {
    const normalized = normalizePath(hotspot.path);
    const node = byPath.get(normalized);
    if (!node || node.kind !== "file") {
      continue;
    }

    node.ownRiskScore = hotspot.score;
    node.ownCollisionIndex = hotspot.collision_index;
  }
}

function finalizeTreeNodes(map: Map<string, BuildTreeNode>): RepoTreeNode[] {
  const nodes: RepoTreeNode[] = [];

  for (const node of map.values()) {
    const children = finalizeTreeNodes(node.children);
    const childRisk = children.reduce((max, child) => Math.max(max, child.riskScore), 0);
    const childCollision = children.reduce((sum, child) => sum + child.collisionIndex, 0);

    nodes.push({
      id: node.id,
      name: node.name,
      path: node.path,
      kind: node.kind,
      riskScore: Math.max(node.ownRiskScore, childRisk),
      collisionIndex: node.ownCollisionIndex + childCollision,
      children,
    });
  }

  nodes.sort((a, b) => {
    if (a.kind !== b.kind) {
      return a.kind === "dir" ? -1 : 1;
    }

    const riskOrder = b.riskScore - a.riskScore;
    if (riskOrder !== 0) {
      return riskOrder;
    }

    return a.name.localeCompare(b.name);
  });

  return nodes;
}

const repoTree = computed<RepoTreeNode[]>(() => {
  const { roots, byPath } = buildTreeSkeleton(repositoryPaths.value);
  applyHotspotRiskToTree(byPath, hotspots.value);
  return finalizeTreeNodes(roots);
});

const flatTreeRows = computed<FlatTreeRow[]>(() => {
  const rows: FlatTreeRow[] = [];
  const expanded = new Set(expandedNodeIds.value);

  const walk = (node: RepoTreeNode, depth: number) => {
    const hasChildren = node.children.length > 0;
    const isExpanded = hasChildren && expanded.has(node.id);

    rows.push({
      id: node.id,
      name: node.name,
      path: node.path,
      kind: node.kind,
      depth,
      riskScore: node.riskScore,
      collisionIndex: node.collisionIndex,
      hasChildren,
      expanded: isExpanded,
    });

    if (!hasChildren || !isExpanded) {
      return;
    }

    for (const child of node.children) {
      walk(child, depth + 1);
    }
  };

  for (const node of repoTree.value) {
    walk(node, 0);
  }

  return rows;
});

function normalizePath(value: string): string {
  return value.split("\\").join("/").replace(/^\/+/, "").replace(/\/+$/, "");
}

function riskRatio(score: number): number {
  if (maxHotspotScore.value <= 0) {
    return 0;
  }
  return Math.min(1, Math.max(0, score / maxHotspotScore.value));
}

function pairRatio(score: number): number {
  if (maxPairScore.value <= 0) {
    return 0;
  }
  return Math.min(1, Math.max(0, score / maxPairScore.value));
}

function heatRowStyle(score: number, selected = false): Record<string, string> {
  const ratio = riskRatio(score);
  const hue = Math.round(78 - ratio * 36);
  const saturation = Math.round(46 + ratio * 24);
  const lightness = Math.round(97 - ratio * 22);
  const alpha = selected ? 0.56 : 0.12 + ratio * 0.18;
  const borderAlpha = selected ? 0.75 : 0.24 + ratio * 0.3;

  return {
    background: `hsla(${hue}, ${saturation}%, ${lightness}%, ${alpha})`,
    borderColor: `hsla(${Math.max(18, hue - 12)}, ${Math.max(42, saturation - 8)}%, ${Math.max(28, lightness - 26)}%, ${borderAlpha})`,
  };
}

function pairRowStyle(score: number): Record<string, string> {
  const ratio = pairRatio(score);
  const hue = Math.round(72 - ratio * 28);
  const lightness = Math.round(96 - ratio * 18);

  return {
    background: `hsla(${hue}, 56%, ${lightness}%, ${0.11 + ratio * 0.15})`,
    borderColor: `hsla(${Math.max(20, hue - 10)}, 52%, ${Math.max(34, lightness - 18)}%, ${0.24 + ratio * 0.28})`,
  };
}

function riskBand(score: number): RiskBand {
  const ratio = riskRatio(score);
  if (ratio >= 0.82) return "critical";
  if (ratio >= 0.64) return "high";
  if (ratio >= 0.36) return "moderate";
  return "low";
}

function riskBandLabel(score: number): string {
  const band = riskBand(score);
  if (band === "critical") return "Critical";
  if (band === "high") return "High";
  if (band === "moderate") return "Moderate";
  return "Low";
}

function riskBandStyle(score: number): Record<string, string> {
  const band = riskBand(score);

  if (band === "critical") {
    return {
      background: "rgba(120, 53, 15, 0.28)",
      borderColor: "rgba(217, 119, 6, 0.58)",
      color: "rgba(254, 215, 170, 1)",
    };
  }

  if (band === "high") {
    return {
      background: "rgba(113, 63, 18, 0.25)",
      borderColor: "rgba(234, 179, 8, 0.56)",
      color: "rgba(254, 243, 199, 1)",
    };
  }

  if (band === "moderate") {
    return {
      background: "rgba(63, 98, 18, 0.22)",
      borderColor: "rgba(163, 230, 53, 0.5)",
      color: "rgba(236, 252, 203, 1)",
    };
  }

  return {
    background: "rgba(20, 83, 45, 0.22)",
    borderColor: "rgba(74, 222, 128, 0.44)",
    color: "rgba(187, 247, 208, 1)",
  };
}

function scoreBarStyle(score: number): Record<string, string> {
  const ratio = riskRatio(score);
  return {
    width: `${Math.max(8, Math.round(ratio * 100))}%`,
    background: `color-mix(in srgb, var(--chart-2) ${Math.round(30 + ratio * 60)}%, var(--primary))`,
  };
}

function pairBarStyle(score: number): Record<string, string> {
  const ratio = pairRatio(score);
  return {
    width: `${Math.max(6, Math.round(ratio * 100))}%`,
    background: `color-mix(in srgb, var(--chart-2) ${Math.round(24 + ratio * 62)}%, var(--primary))`,
  };
}

function isTreeSelected(path: string): boolean {
  return selectedTreePath.value === path;
}

function toggleNode(id: string) {
  if (expandedNodeIds.value.includes(id)) {
    expandedNodeIds.value = expandedNodeIds.value.filter((value) => value !== id);
    return;
  }

  expandedNodeIds.value = [...expandedNodeIds.value, id];
}

function selectTreePath(path: string) {
  if (selectedTreePath.value === path) {
    selectedTreePath.value = "";
    return;
  }

  selectedTreePath.value = path;
}

function clearTreeFilter() {
  selectedTreePath.value = "";
}

function treeBandForRow(score: number, kind: TreeNodeKind): RiskBand {
  const ratio = riskRatio(score);

  if (kind === "file") {
    if (ratio >= 0.62) return "critical";
    if (ratio >= 0.38) return "high";
    if (ratio >= 0.16) return "moderate";
    return "low";
  }

  return riskBand(score);
}

function treeRiskClassForRow(score: number, kind: TreeNodeKind): string {
  const band = treeBandForRow(score, kind);
  const prefix = kind === "file" ? "tree-file-risk" : "tree-risk";

  if (band === "critical") return `${prefix}-critical`;
  if (band === "high") return `${prefix}-high`;
  if (band === "moderate") return `${prefix}-moderate`;
  return `${prefix}-low`;
}

async function loadHotspots() {
  hotspotRunToken += 1;
  const runToken = hotspotRunToken;
  const cacheKey = `${props.repoPath}::${selectedMergeConfig.value.maxCount}`;

  if (!props.repoPath) {
    hotspots.value = [];
    hotspotsLoading.value = false;
    hotspotsError.value = "";
    return;
  }

  const cachedRows = hotspotCache.get(cacheKey);
  if (cachedRows) {
    hotspots.value = cachedRows;
    hotspotsLoading.value = false;
    hotspotsError.value = "";
    return;
  }

  hotspotsLoading.value = true;
  hotspotsError.value = "";

  try {
    const items = await invoke<ConflictHotspot[]>("get_conflict_hotspots", {
      path: props.repoPath,
      maxCount: selectedMergeConfig.value.maxCount,
    });

    if (runToken !== hotspotRunToken) {
      return;
    }

    hotspots.value = items;
    hotspotCache.set(cacheKey, items);
  } catch {
    if (runToken !== hotspotRunToken) {
      return;
    }

    hotspots.value = [];
    hotspotsError.value = "Could not load hotspot collision analytics.";
  } finally {
    if (runToken === hotspotRunToken) {
      hotspotsLoading.value = false;
    }
  }
}

async function loadRiskPairs() {
  pairRunToken += 1;
  const runToken = pairRunToken;
  const cacheKey = `${props.repoPath}::${selectedMergeConfig.value.maxCount}`;

  if (!props.repoPath) {
    riskyPairs.value = [];
    pairsLoading.value = false;
    pairsError.value = "";
    return;
  }

  const cachedRows = pairCache.get(cacheKey);
  if (cachedRows) {
    riskyPairs.value = cachedRows;
    pairsLoading.value = false;
    pairsError.value = "";
    return;
  }

  pairsLoading.value = true;
  pairsError.value = "";

  try {
    const rows = await invoke<ConflictPair[]>("get_conflict_pairs", {
      path: props.repoPath,
      maxCount: selectedMergeConfig.value.maxCount,
    });

    if (runToken !== pairRunToken) {
      return;
    }

    const limited = rows.slice(0, 36);
    riskyPairs.value = limited;
    pairCache.set(cacheKey, limited);
  } catch {
    if (runToken !== pairRunToken) {
      return;
    }

    riskyPairs.value = [];
    pairsError.value = "Could not load high-coupling conflict pairs.";
  } finally {
    if (runToken === pairRunToken) {
      pairsLoading.value = false;
    }
  }
}

async function loadRepositoryTree() {
  treeRunToken += 1;
  const runToken = treeRunToken;
  const cacheKey = props.repoPath;

  if (!props.repoPath) {
    repositoryPaths.value = [];
    treeLoading.value = false;
    treeError.value = "";
    return;
  }

  const cachedPaths = treeCache.get(cacheKey);
  if (cachedPaths) {
    repositoryPaths.value = cachedPaths;
    treeLoading.value = false;
    treeError.value = "";
    return;
  }

  treeLoading.value = true;
  treeError.value = "";

  try {
    const paths = await invoke<string[]>("get_repository_tree_paths", {
      path: props.repoPath,
      maxCount: TREE_SCAN_LIMIT,
    });

    if (runToken !== treeRunToken) {
      return;
    }

    repositoryPaths.value = paths;
    treeCache.set(cacheKey, paths);
  } catch {
    if (runToken !== treeRunToken) {
      return;
    }

    repositoryPaths.value = [];
    treeError.value = "Could not load repository tree structure.";
  } finally {
    if (runToken === treeRunToken) {
      treeLoading.value = false;
    }
  }
}

watch(
  () => props.repoPath,
  () => {
    query.value = "";
    selectedTreePath.value = "";
    expandedNodeIds.value = [];

    void loadHotspots();
    void loadRiskPairs();
    void loadRepositoryTree();
  },
  { immediate: true },
);

watch(selectedMergeWindow, () => {
  if (!props.repoPath) {
    return;
  }

  void loadHotspots();
  void loadRiskPairs();
});

watch(
  repoTree,
  (nodes) => {
    const validIds = new Set<string>();

    const walk = (node: RepoTreeNode) => {
      validIds.add(node.id);

      for (const child of node.children) {
        walk(child);
      }
    };

    for (const node of nodes) {
      walk(node);
    }

    const kept = expandedNodeIds.value.filter((id) => validIds.has(id));
    expandedNodeIds.value = kept;

    if (selectedTreePath.value) {
      const hasSelected = repositoryPaths.value.some(
        (path) => path === selectedTreePath.value || path.startsWith(selectedTreePath.value + "/"),
      );

      if (!hasSelected) {
        selectedTreePath.value = "";
      }
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="flex-1 min-h-0 overflow-y-auto bg-[var(--background)]">
    <div class="p-4 md:p-5 space-y-4">
      <section class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-4">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div>
            <p class="text-[10px] uppercase tracking-[0.16em] text-[var(--muted-foreground)]">Predictive Merge Risk</p>
            <h2 class="text-lg md:text-xl font-bold text-[var(--foreground)]">Integration Signals</h2>
            <p class="text-xs text-[var(--muted-foreground)] mt-1">Repository tree heatmap, collision index, and coupling pairs.</p>
          </div>
          <button
            class="h-7 w-7 rounded border border-[var(--border)] bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:border-[var(--destructive)]/50 transition-colors"
            title="Back to Git Graph"
            @click="emit('close')"
          >
            x
          </button>
        </div>
      </section>

      <section class="relative overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--card)] p-4 space-y-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-[10px] uppercase tracking-[0.08em] text-[var(--muted-foreground)]">Merge sample</span>
            <div class="window-selector">
              <button
                v-for="option in MERGE_WINDOWS"
                :key="option.key"
                class="window-btn"
                :class="selectedMergeWindow === option.key ? 'active' : ''"
                @click="selectedMergeWindow = option.key"
              >
                {{ option.label }}
              </button>
            </div>
          </div>
        </div>

        <div class="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-2">
          <article class="risk-stat-card">
            <div class="risk-stat-label">Hotspots</div>
            <div class="risk-stat-value">{{ hotspots.length }}</div>
          </article>
          <article class="risk-stat-card">
            <div class="risk-stat-label">High risk files</div>
            <div class="risk-stat-value">{{ highRiskHotspots }}</div>
          </article>
          <article class="risk-stat-card">
            <div class="risk-stat-label">Merge touches</div>
            <div class="risk-stat-value">{{ totalMergeTouches }}</div>
          </article>
          <article class="risk-stat-card">
            <div class="risk-stat-label">Conflict mentions</div>
            <div class="risk-stat-value">{{ totalConflictMentions }}</div>
          </article>
          <article class="risk-stat-card">
            <div class="risk-stat-label">Collision index</div>
            <div class="risk-stat-value">{{ totalCollisionIndex }}</div>
          </article>
          <article class="risk-stat-card">
            <div class="risk-stat-label">Avg collision/file</div>
            <div class="risk-stat-value">{{ averageCollisionIndex }}</div>
          </article>
          <article class="risk-stat-card">
            <div class="risk-stat-label">Mentions / touch</div>
            <div class="risk-stat-value">{{ conflictMentionsPerTouch }}</div>
          </article>
          <article class="risk-stat-card">
            <div class="risk-stat-label">Risky pairs</div>
            <div class="risk-stat-value">{{ riskyPairs.length }}</div>
          </article>
          <article class="risk-stat-card">
            <div class="risk-stat-label">Top pair coupling</div>
            <div class="risk-stat-value">{{ topPairCoupling }}%</div>
          </article>
        </div>

        <div class="grid grid-cols-1 xl:grid-cols-2 gap-2 text-[10px]">
          <div class="diag-card">
            <div class="diag-title">Top suspect</div>
            <div v-if="topHotspot" class="diag-value" :title="topHotspot.path">{{ topHotspot.path }}</div>
            <div v-else class="diag-value">n/a</div>
            <div class="diag-desc">Why: {{ topHotspotReason }}</div>
          </div>
          <div class="diag-card">
            <div class="diag-title">Cross-file pressure</div>
            <div v-if="topPair" class="diag-value" :title="`${topPair.left_path} + ${topPair.right_path}`">
              {{ topPair.left_path }} + {{ topPair.right_path }}
            </div>
            <div v-else class="diag-value">n/a</div>
            <div class="diag-desc">Main domain under pressure: <strong class="text-[var(--foreground)]">{{ dominantRiskDomain.name }}</strong></div>
          </div>
        </div>

        <div v-if="hotspotsError" class="text-xs text-[var(--destructive)]">{{ hotspotsError }}</div>

        <div v-if="hotspotsLoading" class="panel-loader-overlay">
          <img :src="logoCrocLoading" alt="Loading conflict diagnostics" class="panel-loader-logo" />
          <div class="panel-loader-text">Loading diagnostics</div>
          <div class="loader-wave" aria-hidden="true">
            <span
              v-for="(letter, index) in loadingLetters"
              :key="`summary-loader-${index}`"
              class="loader-letter"
              :style="{ animationDelay: `${index * 0.08}s` }"
            >
              {{ letter }}
            </span>
          </div>
        </div>
      </section>

      <section class="grid grid-cols-1 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)] gap-3">
        <article class="relative overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="flex flex-wrap items-center justify-between gap-2 mb-2">
            <h3 class="text-sm font-semibold text-[var(--foreground)]">Suspect Files</h3>
            <div class="flex items-center gap-2">
              <button
                v-if="selectedTreePath"
                class="h-7 px-2 rounded border border-[var(--primary)]/35 text-[11px] text-[var(--primary)] hover:bg-[var(--primary)]/10"
                @click="clearTreeFilter"
              >
                Clear tree scope
              </button>
              <span class="text-[11px] text-[var(--muted-foreground)]">{{ filteredHotspots.length }} result{{ filteredHotspots.length === 1 ? '' : 's' }}</span>
            </div>
          </div>

          <input
            v-model="query"
            placeholder="Search suspect file paths..."
            class="h-8 w-full px-3 rounded-md border border-[var(--border)] bg-[var(--input-background)] text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          />

          <div v-if="filteredHotspots.length === 0" class="text-xs text-[var(--muted-foreground)] py-4 text-center">No suspect files for this filter.</div>
          <div v-else class="space-y-1.5 max-h-[620px] overflow-y-auto pr-1 mt-3 hotspot-scroll">
            <div
              v-for="item in filteredHotspots"
              :key="item.path"
              class="rounded-md border px-2.5 py-1.5"
              :style="heatRowStyle(item.score)"
            >
              <div class="flex items-start gap-2 justify-between">
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span class="text-[10px] font-semibold text-[var(--foreground)] break-all">{{ item.path }}</span>
                    <span class="risk-pill" :style="riskBandStyle(item.score)">{{ riskBandLabel(item.score) }}</span>
                  </div>
                  <div class="mt-0.5 text-[10px] text-[var(--muted-foreground)]">
                    collision index: {{ item.collision_index }}
                    <span class="mx-1">|</span>
                    merge touches: {{ item.merge_touches }}
                    <span class="mx-1">|</span>
                    conflict mentions: {{ item.conflict_mentions }}
                    <span class="mx-1">|</span>
                    score: {{ item.score }}
                  </div>
                </div>
                <div class="w-[96px] h-1.5 rounded bg-[var(--secondary)] overflow-hidden self-center mt-1">
                  <div class="h-full rounded" :style="scoreBarStyle(item.score)" />
                </div>
              </div>
            </div>
          </div>

          <div v-if="hotspotsLoading" class="panel-loader-overlay">
            <img :src="logoCrocLoading" alt="Loading hotspots" class="panel-loader-logo" />
            <div class="panel-loader-text">Loading suspects</div>
            <div class="loader-wave" aria-hidden="true">
              <span
                v-for="(letter, index) in loadingLetters"
                :key="`suspects-loader-${index}`"
                class="loader-letter"
                :style="{ animationDelay: `${index * 0.08}s` }"
              >
                {{ letter }}
              </span>
            </div>
          </div>
        </article>

        <article class="relative overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="flex items-center justify-between gap-2 mb-2">
            <div>
              <h3 class="text-sm font-semibold text-[var(--foreground)]">Repository Tree Heatmap</h3>
              <p class="text-[10px] text-[var(--muted-foreground)]">Folders can collapse/expand. Risk rolls up from child files.</p>
            </div>
          </div>

          <div v-if="treeError" class="text-xs text-[var(--destructive)] py-3">{{ treeError }}</div>
          <div v-else-if="flatTreeRows.length === 0" class="text-xs text-[var(--muted-foreground)] py-3">No files found in repository tree.</div>
          <div v-else class="repo-tree-list pr-1">
            <div
              v-for="row in flatTreeRows"
              :key="row.id"
              class="repo-tree-row"
              :class="[
                isTreeSelected(row.path) ? 'repo-tree-row-selected' : '',
                treeRiskClassForRow(row.riskScore, row.kind),
              ]"
              @click="selectTreePath(row.path)"
            >
              <div class="repo-tree-row-main" :style="{ paddingLeft: `${6 + row.depth * 14}px` }">
                <button
                  v-if="row.hasChildren"
                  class="tree-toggle"
                  :title="row.expanded ? 'Collapse' : 'Expand'"
                  @click.stop="toggleNode(row.id)"
                >
                  {{ row.expanded ? "▾" : "▸" }}
                </button>
                <span v-else class="tree-toggle-placeholder" />

                <component
                  :is="row.kind === 'dir' ? (row.expanded ? FolderOpen : Folder) : FileCode2"
                  class="w-3.5 h-3.5 flex-shrink-0 tree-icon"
                  :class="row.kind === 'dir' ? 'text-[var(--chart-5)]' : 'text-[var(--primary)]'"
                />

                <span class="repo-tree-name" :title="row.path">{{ row.name }}</span>
                <span
                  v-if="treeBandForRow(row.riskScore, row.kind) !== 'low'"
                  class="tree-risk-badge"
                  :class="treeRiskClassForRow(row.riskScore, row.kind)"
                >
                  {{ treeBandForRow(row.riskScore, row.kind) === 'critical' ? '!!' : '!' }}
                </span>
              </div>
            </div>
          </div>

          <div v-if="treeLoading" class="panel-loader-overlay">
            <img :src="logoCrocLoading" alt="Loading repository tree" class="panel-loader-logo" />
            <div class="panel-loader-text">Loading repository tree</div>
            <div class="loader-wave" aria-hidden="true">
              <span
                v-for="(letter, index) in loadingLetters"
                :key="`tree-loader-${index}`"
                class="loader-letter"
                :style="{ animationDelay: `${index * 0.08}s` }"
              >
                {{ letter }}
              </span>
            </div>
          </div>
        </article>
      </section>

      <section class="relative overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--card)] p-3">
        <div class="flex items-center justify-between gap-2 mb-2">
          <h3 class="text-sm font-semibold text-[var(--foreground)]">Coupled Risk Pairs</h3>
          <span class="text-[10px] text-[var(--muted-foreground)]">Files that repeatedly collide in same merge windows</span>
        </div>

        <div v-if="pairsError" class="text-xs text-[var(--destructive)] py-2">{{ pairsError }}</div>
        <div v-else-if="riskyPairs.length === 0" class="text-xs text-[var(--muted-foreground)] py-2">No risky pairs in this window.</div>
        <div v-else class="pairs-scroll pr-1">
          <div
            v-for="pair in riskyPairs"
            :key="`${pair.left_path}__${pair.right_path}`"
            class="pair-compact-card"
            :style="pairRowStyle(pair.score)"
          >
            <div class="pair-path" :title="pair.left_path">{{ pair.left_path }}</div>
            <div class="pair-path" :title="pair.right_path">{{ pair.right_path }}</div>
            <div class="mt-0.5 text-[10px] text-[var(--muted-foreground)]">
              co-touches: {{ pair.co_touches }}
              <span class="mx-1">|</span>
              conflict merges: {{ pair.conflict_touches }}
              <span class="mx-1">|</span>
              score: {{ pair.score }}
            </div>
            <div class="mt-1 h-1 rounded bg-[var(--secondary)] overflow-hidden">
              <div class="h-full rounded" :style="pairBarStyle(pair.score)" />
            </div>
          </div>
        </div>

        <div v-if="pairsLoading" class="panel-loader-overlay">
          <img :src="logoCrocLoading" alt="Loading pair analytics" class="panel-loader-logo" />
          <div class="panel-loader-text">Loading conflict pairs</div>
          <div class="loader-wave" aria-hidden="true">
            <span
              v-for="(letter, index) in loadingLetters"
              :key="`pairs-loader-${index}`"
              class="loader-letter"
              :style="{ animationDelay: `${index * 0.08}s` }"
            >
              {{ letter }}
            </span>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.window-selector {
  display: inline-flex;
  align-items: center;
  border: 1px solid color-mix(in srgb, var(--border) 85%, transparent);
  border-radius: 0.5rem;
  overflow: hidden;
}

.window-btn {
  min-width: 46px;
  height: 28px;
  padding: 0 0.55rem;
  font-size: 11px;
  font-weight: 600;
  color: var(--muted-foreground);
  background: color-mix(in srgb, var(--secondary) 88%, transparent);
  border-right: 1px solid color-mix(in srgb, var(--border) 85%, transparent);
}

.window-btn:last-child {
  border-right: 0;
}

.window-btn.active {
  color: var(--foreground);
  background: color-mix(in srgb, var(--destructive) 16%, var(--card));
}

.risk-stat-card {
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  background: color-mix(in srgb, var(--card) 90%, transparent);
  padding: 0.55rem 0.62rem;
}

.risk-stat-label {
  font-size: 10px;
  color: var(--muted-foreground);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.risk-stat-value {
  margin-top: 0.22rem;
  font-size: 1rem;
  font-weight: 700;
  color: var(--foreground);
}

.diag-card {
  border: 1px solid color-mix(in srgb, var(--border) 88%, transparent);
  border-radius: 0.48rem;
  background: color-mix(in srgb, var(--secondary) 72%, transparent);
  padding: 0.5rem 0.58rem;
}

.diag-title {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted-foreground);
}

.diag-value {
  margin-top: 0.22rem;
  font-size: 11px;
  color: var(--foreground);
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.diag-desc {
  margin-top: 0.2rem;
  font-size: 10px;
  color: var(--muted-foreground);
}

.risk-pill {
  display: inline-flex;
  align-items: center;
  border: 1px solid transparent;
  border-radius: 999px;
  font-size: 9px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  font-weight: 700;
  padding: 0.12rem 0.4rem;
  white-space: nowrap;
}

.panel-loader-overlay {
  position: absolute;
  inset: 0;
  z-index: 20;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--background) 82%, transparent);
  backdrop-filter: blur(2px);
}

.panel-loader-logo {
  width: 44px;
  height: 44px;
  object-fit: contain;
  filter: drop-shadow(0 0 7px color-mix(in srgb, var(--primary) 34%, transparent));
}

.panel-loader-text {
  margin-top: 0.38rem;
  font-size: 11px;
  font-weight: 600;
  color: var(--foreground);
}

.loader-wave {
  margin-top: 0.18rem;
  display: inline-flex;
  gap: 1px;
}

.loader-letter {
  font-size: 10px;
  color: var(--primary);
  font-weight: 800;
  text-transform: uppercase;
  animation: panel-loader-wave 1.05s ease-in-out infinite;
}

.pairs-scroll {
  max-height: 318px;
  overflow-y: auto;
  display: grid;
  gap: 0.45rem;
  grid-template-columns: repeat(1, minmax(0, 1fr));
}

@media (min-width: 900px) {
  .pairs-scroll {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

.pair-compact-card {
  border: 1px solid color-mix(in srgb, var(--border) 85%, transparent);
  border-radius: 0.45rem;
  padding: 0.45rem 0.55rem;
  min-width: 0;
  display: grid;
  gap: 0.2rem;
}

.pair-path {
  font-size: 11px;
  font-weight: 600;
  color: var(--foreground);
  line-height: 1.3;
  word-break: break-all;
}

.repo-tree-list {
  max-height: 620px;
  overflow-y: auto;
  display: grid;
  gap: 2px;
}

.repo-tree-row {
  border: 1px solid color-mix(in srgb, var(--border) 90%, transparent);
  border-left-width: 4px;
  border-radius: 0.45rem;
  background: color-mix(in srgb, var(--card) 92%, transparent);
  transition: border-color 0.12s ease, background-color 0.12s ease;
  cursor: pointer;
}

.repo-tree-row:hover {
  border-color: color-mix(in srgb, var(--destructive) 52%, var(--border));
}

.repo-tree-row-selected {
  background: color-mix(in srgb, var(--destructive) 15%, var(--card));
  border-color: color-mix(in srgb, var(--destructive) 58%, var(--border));
}

.repo-tree-row-main {
  min-height: 26px;
  display: flex;
  align-items: center;
  gap: 0.33rem;
  padding: 0.26rem 0.4rem;
}

.repo-tree-name {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--foreground);
  font-size: 11px;
  font-weight: 500;
}

.tree-icon {
  opacity: 0.94;
}

.tree-toggle {
  width: 14px;
  height: 14px;
  border-radius: 0.25rem;
  border: 1px solid color-mix(in srgb, var(--border) 84%, transparent);
  background: color-mix(in srgb, var(--secondary) 85%, transparent);
  color: var(--muted-foreground);
  font-size: 9px;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
  flex-shrink: 0;
}

.tree-toggle-placeholder {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.tree-risk-badge {
  min-width: 16px;
  height: 16px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 800;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0 0.25rem;
}

.tree-risk-low {
  border-left-color: rgba(74, 222, 128, 0.34);
}

.tree-risk-moderate {
  border-left-color: rgba(253, 186, 116, 0.72);
}

.tree-risk-high {
  border-left-color: rgba(239, 68, 68, 0.88);
}

.tree-risk-critical {
  border-left-color: rgba(185, 28, 28, 0.97);
}

.tree-file-risk-low {
  border-left-color: rgba(96, 165, 250, 0.34);
}

.tree-file-risk-moderate {
  border-left-color: rgba(245, 158, 11, 0.64);
  background: color-mix(in srgb, var(--destructive) 7%, var(--card));
}

.tree-file-risk-high {
  border-left-color: rgba(239, 68, 68, 0.9);
  background: color-mix(in srgb, var(--destructive) 10%, var(--card));
}

.tree-file-risk-critical {
  border-left-color: rgba(185, 28, 28, 0.98);
  background: color-mix(in srgb, var(--destructive) 16%, var(--card));
}

.tree-risk-badge.tree-risk-moderate {
  color: rgba(180, 83, 9, 1);
  background: rgba(251, 191, 36, 0.2);
  border: 1px solid rgba(251, 191, 36, 0.48);
}

.tree-risk-badge.tree-risk-high {
  color: rgba(252, 165, 165, 1);
  background: rgba(185, 28, 28, 0.26);
  border: 1px solid rgba(248, 113, 113, 0.6);
}

.tree-risk-badge.tree-risk-critical {
  color: rgba(254, 226, 226, 1);
  background: rgba(127, 29, 29, 0.42);
  border: 1px solid rgba(252, 165, 165, 0.76);
}

.tree-risk-badge.tree-file-risk-moderate {
  color: rgba(180, 83, 9, 1);
  background: rgba(245, 158, 11, 0.22);
  border: 1px solid rgba(245, 158, 11, 0.5);
}

.tree-risk-badge.tree-file-risk-high {
  color: rgba(254, 202, 202, 1);
  background: rgba(153, 27, 27, 0.28);
  border: 1px solid rgba(248, 113, 113, 0.62);
}

.tree-risk-badge.tree-file-risk-critical {
  color: rgba(254, 226, 226, 1);
  background: rgba(127, 29, 29, 0.46);
  border: 1px solid rgba(254, 202, 202, 0.85);
}

@keyframes panel-loader-wave {
  0%,
  50%,
  100% {
    transform: translateY(0);
    opacity: 0.45;
  }

  25% {
    transform: translateY(-3px);
    opacity: 1;
  }
}
</style>

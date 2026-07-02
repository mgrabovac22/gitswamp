<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import type { BranchInfo, CommitInfo } from "@/types";

type ViewMode = "galaxy" | "tree";
type GalaxyShape = "spiral" | "layers" | "constellation";
type DepthPreset = "balanced" | "deep";
type GalaxyMotionMode = "threeD" | "balanced" | "circling";

type GalaxyNode = {
  commit: CommitInfo;
  galaxyX: number;
  galaxyY: number;
  galaxyZ: number;
  levelX: number;
  levelY: number;
  levelZ: number;
  treeX: number;
  treeY: number;
  treeZ: number;
  radius: number;
  color: string;
  branchLabel: string | null;
  laneName: string;
  laneOffset: number;
  ageProgress: number;
  treeBaseProgress: number;
  treeTipProgress: number;
  treeBranchProgress: number;
  isMainLane: boolean;
};

type GalaxyEdge = {
  from: GalaxyNode;
  to: GalaxyNode;
  color: string;
};

type GalaxyLane = {
  name: string;
  offset: number;
  color: string;
  baseProgress: number;
  tipProgress: number;
  isMain: boolean;
};

type GalaxyScene = {
  nodes: GalaxyNode[];
  edges: GalaxyEdge[];
  lanes: GalaxyLane[];
  historyHeight: number;
  treeHeight: number;
};

type ProjectedNode = GalaxyNode & {
  screenX: number;
  screenY: number;
  depth: number;
  screenRadius: number;
};

const props = defineProps<{
  commits: CommitInfo[];
  branches: BranchInfo[];
  currentBranch: string;
  selectedSha?: string | null;
  hasMore?: boolean;
}>();

const emit = defineEmits<{
  close: [];
  loadMore: [];
  loadAll: [];
  select: [payload: { commit: CommitInfo | null; additive?: boolean }];
}>();

const MAX_RENDERED_COMMITS = 50000;
const STARFIELD_COUNT = 180;
const MIN_ZOOM = 0.18;
const MAX_ZOOM = 5.5;
const MINI_MAP_MAX_POINTS = 1600;
const BRANCH_COLORS = [
  "#7dd3fc",
  "#c084fc",
  "#f9a8d4",
  "#86efac",
  "#fde68a",
  "#fca5a5",
  "#93c5fd",
  "#5eead4",
  "#d8b4fe",
  "#fdba74",
];

const containerRef = ref<HTMLElement | null>(null);
const canvasRef = ref<HTMLCanvasElement | null>(null);
const hoveredNode = shallowRef<ProjectedNode | null>(null);
const viewMode = ref<ViewMode>("galaxy");
const galaxyShape = ref<GalaxyShape>("spiral");
const galaxyMotionMode = ref<GalaxyMotionMode>("threeD");
const depthPreset = computed<DepthPreset>(() => (galaxyMotionMode.value === "balanced" ? "balanced" : "deep"));
const focusedSha = ref<string | null>(null);
const showLabels = ref(true);
const showGuides = ref(true);
const showTimeRings = ref(true);
const showBranchAura = ref(true);
const showDepthFog = ref(true);
const showMiniMap = ref(true);
const highlightMerges = ref(true);
const showViewOptionsMenu = ref(false);
const tooltip = ref({ x: 0, y: 0 });
const isDragging = ref(false);
const viewOptionsRef = ref<HTMLElement | null>(null);

let ctx: CanvasRenderingContext2D | null = null;
let resizeObserver: ResizeObserver | null = null;
let frameId: number | null = null;
let animationTimer: ReturnType<typeof setTimeout> | null = null;
let canvasWidth = 1;
let canvasHeight = 1;
let dpr = 1;
let lastPointerX = 0;
let lastPointerY = 0;
let pointerDownX = 0;
let pointerDownY = 0;
let draggedSincePointerDown = false;
let projectedNodes: ProjectedNode[] = [];
const projectedBySha = new Map<string, ProjectedNode>();
let galaxyAnimationTime = 0;
let circlingSpeedScale = 1;

const viewport = {
  zoom: 1,
  panX: 0,
  panY: 0,
  rotation: -0.48,
  tilt: 0.58,
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function mix(from: number, to: number, amount: number): number {
  return from + (to - from) * amount;
}

function smoothstep(edge0: number, edge1: number, value: number): number {
  const t = clamp((value - edge0) / Math.max(0.0001, edge1 - edge0), 0, 1);
  return t * t * (3 - 2 * t);
}

function hashText(value: string): number {
  let hash = 2166136261;
  for (let i = 0; i < value.length; i += 1) {
    hash ^= value.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function normalizeBranchName(value: string): string {
  return value
    .replace(/^refs\/heads\//, "")
    .replace(/^refs\/remotes\//, "")
    .replace(/^remotes\/[a-z0-9_-]+\//i, "")
    .replace(/^origin\//i, "")
    .replace(/^HEAD ->\s*/, "")
    .trim();
}

function commitHasBranchRef(commit: CommitInfo, branchName: string): boolean {
  const normalizedBranch = normalizeBranchName(branchName);
  return commit.refs.some((ref) => {
    const refParts = ref.split(",").map((part) => normalizeBranchName(part));
    return refParts.some((normalizedRef) =>
      normalizedRef === normalizedBranch || normalizedRef.endsWith(`/${normalizedBranch}`),
    );
  });
}

function colorForBranch(branchName: string): string {
  return BRANCH_COLORS[hashText(branchName) % BRANCH_COLORS.length];
}

function alternatingLaneOffset(index: number): number {
  if (index === 0) return 0;
  const magnitude = Math.ceil(index / 2);
  return index % 2 === 1 ? -magnitude : magnitude;
}

function trunkBranchPriority(name: string): number {
  const normalized = normalizeBranchName(name).toLowerCase();
  if (normalized === "main") return 0;
  if (normalized === "master") return 1;
  if (normalized === "trunk") return 2;
  if (normalized === "develop" || normalized === "dev") return 3;
  return 100;
}

function findBranchTip(commits: CommitInfo[], branchName: string): CommitInfo | null {
  for (let index = commits.length - 1; index >= 0; index -= 1) {
    if (commitHasBranchRef(commits[index], branchName)) {
      return commits[index];
    }
  }
  return null;
}

function branchPathFromTip(tip: CommitInfo, commitsBySha: Map<string, CommitInfo>, maxLength: number): CommitInfo[] {
  const path: CommitInfo[] = [];
  let cursor: CommitInfo | undefined = tip;
  let guard = 0;
  while (cursor && guard < maxLength) {
    path.push(cursor);
    cursor = commitsBySha.get(cursor.parent_shas[0] || "");
    guard += 1;
  }
  return path;
}

const currentBranchName = computed(() => normalizeBranchName(props.currentBranch));

const renderedCommits = computed(() =>
  props.commits
    .slice(0, MAX_RENDERED_COMMITS)
    .sort((a, b) => a.timestamp - b.timestamp),
);

const sortedBranches = computed(() => {
  const current = currentBranchName.value;
  return [...props.branches].sort((a, b) => {
    const aName = normalizeBranchName(a.name);
    const bName = normalizeBranchName(b.name);
    const aRank = trunkBranchPriority(aName) * 1000 + (aName === current ? -250 : 0) + (a.is_head ? -120 : 0) + (a.is_remote ? 80 : 0);
    const bRank = trunkBranchPriority(bName) * 1000 + (bName === current ? -250 : 0) + (b.is_head ? -120 : 0) + (b.is_remote ? 80 : 0);
    return aRank - bRank || aName.localeCompare(bName);
  });
});

const galaxyScene = computed<GalaxyScene>(() => {
  const commits = renderedCommits.value;
  const nodesBySha = new Map<string, GalaxyNode>();
  const commitsBySha = new Map(commits.map((commit) => [commit.sha, commit]));
  const progressBySha = new Map(commits.map((commit, index) => [
    commit.sha,
    commits.length <= 1 ? 0.5 : index / Math.max(1, commits.length - 1),
  ]));
  const branchTips = new Map<string, CommitInfo>();
  const branchPaths = new Map<string, CommitInfo[]>();
  const tipLabelsBySha = new Map<string, string[]>();
  const laneBySha = new Map<string, string>();

  for (const branch of sortedBranches.value) {
    const branchName = normalizeBranchName(branch.name);
    if (!branchName) continue;
    const tip = findBranchTip(commits, branch.name);
    if (!tip) continue;
    branchTips.set(branchName, tip);
    const labels = tipLabelsBySha.get(tip.sha) || [];
    labels.push(branchName);
    tipLabelsBySha.set(tip.sha, labels);
  }

  for (const [branchName, tip] of branchTips) {
    branchPaths.set(branchName, branchPathFromTip(tip, commitsBySha, commits.length));
  }

  const current = currentBranchName.value;
  const branchNamesWithTips = Array.from(branchTips.keys());
  const mainLaneName = branchNamesWithTips
    .slice()
    .sort((a, b) => {
      const aRank = trunkBranchPriority(a) * 100 + (a === current ? -1 : 0);
      const bRank = trunkBranchPriority(b) * 100 + (b === current ? -1 : 0);
      return aRank - bRank || a.localeCompare(b);
    })[0] || current || "history";

  const mainPath = branchPaths.get(mainLaneName) || [];
  const mainPathSet = new Set(mainPath.map((commit) => commit.sha));
  for (const commit of mainPath) {
    laneBySha.set(commit.sha, mainLaneName);
  }

  const orderedBranchNames = [
    mainLaneName,
    ...branchNamesWithTips.filter((name) => current && name === current && name !== mainLaneName),
    ...branchNamesWithTips.filter((name) => name !== mainLaneName && (!current || name !== current)),
  ];

  for (const branchName of orderedBranchNames) {
    if (branchName === mainLaneName) continue;
    const path = branchPaths.get(branchName) || [];
    for (const commit of path) {
      if (mainPathSet.has(commit.sha)) {
        break;
      }
      if (!laneBySha.has(commit.sha)) {
        laneBySha.set(commit.sha, branchName);
      }
    }
  }

  const usedLaneNames = orderedBranchNames.filter((name, index, names) =>
    names.indexOf(name) === index && commits.some((commit) => laneBySha.get(commit.sha) === name),
  );
  if (commits.some((commit) => !laneBySha.has(commit.sha))) {
    usedLaneNames.push("history");
  }

  const orderedLaneNames = usedLaneNames.filter((name, index, names) => names.indexOf(name) === index);
  if (!orderedLaneNames.length) {
    orderedLaneNames.push("history");
  }

  const laneOffsets = new Map<string, number>();
  orderedLaneNames.forEach((name, index) => {
    laneOffsets.set(name, name === mainLaneName ? 0 : alternatingLaneOffset(index));
  });

  const count = Math.max(1, commits.length - 1);
  const historyHeight = clamp(commits.length * 18, 760, 12000);
  const treeHeight = clamp(commits.length * 15, 780, 9000);
  const branchMeta = new Map<string, { baseProgress: number; tipProgress: number; isMain: boolean }>();

  for (const name of orderedLaneNames) {
    if (name === mainLaneName) {
      const path = branchPaths.get(name) || mainPath;
      const tipProgress = progressBySha.get(path[0]?.sha || "") ?? 1;
      const baseProgress = progressBySha.get(path[path.length - 1]?.sha || "") ?? 0;
      branchMeta.set(name, {
        baseProgress,
        tipProgress,
        isMain: true,
      });
      continue;
    }

    if (name === "history") {
      branchMeta.set(name, {
        baseProgress: 0.18,
        tipProgress: 1,
        isMain: false,
      });
      continue;
    }

    const path = branchPaths.get(name) || [];
    const baseCommit = path.find((commit) => mainPathSet.has(commit.sha));
    const tipProgress = progressBySha.get(path[0]?.sha || "") ?? 1;
    const baseProgress = progressBySha.get(baseCommit?.sha || "") ?? Math.max(0, tipProgress - 0.22);
    branchMeta.set(name, {
      baseProgress: Math.min(baseProgress, tipProgress - 0.02),
      tipProgress,
      isMain: false,
    });
  }

  const lanes: GalaxyLane[] = orderedLaneNames.map((name) => {
    const meta = branchMeta.get(name) || { baseProgress: 0, tipProgress: 1, isMain: false };
    return {
      name,
      offset: laneOffsets.get(name) || 0,
      color: colorForBranch(name),
      baseProgress: meta.baseProgress,
      tipProgress: meta.tipProgress,
      isMain: meta.isMain,
    };
  });

  const nodes = commits.map((commit, index) => {
    const ageProgress = commits.length <= 1 ? 0.5 : index / count;
    const laneName = laneBySha.get(commit.sha) || "history";
    const laneOffset = laneOffsets.get(laneName) ?? alternatingLaneOffset(orderedLaneNames.length);
    const tipLabels = tipLabelsBySha.get(commit.sha);
    const branchLabel = tipLabels?.slice(0, 3).join(", ") || null;
    const hash = hashText(commit.sha);
    const branchSeed = hashText(laneName) % 29;

    const spiralRadius = 28 + Math.sqrt(index + 1) * 21;
    const angle = index * 2.3999632297 + branchSeed * 0.075 + laneOffset * 0.14;
    const galaxyX = Math.cos(angle) * spiralRadius + laneOffset * 6;
    const galaxyY = Math.sin(angle) * spiralRadius;
    const galaxyZ = ((hash % 220) - 110) * (1 + index / Math.max(1, commits.length));

    const laneNoise = ((hash % 1000) / 1000 - 0.5) * (branchLabel ? 12 : 24);
    const levelX = laneOffset * 118 + laneNoise;
    const levelY = (0.52 - ageProgress) * historyHeight;
    const levelZ = ((Math.floor(hash / 17) % 90) - 45) + laneOffset * 10;

    const laneMeta = branchMeta.get(laneName) || { baseProgress: 0, tipProgress: 1, isMain: false };
    const branchSpan = Math.max(0.02, laneMeta.tipProgress - laneMeta.baseProgress);
    const treeBranchProgress = laneMeta.isMain
      ? ageProgress
      : smoothstep(0, 1, clamp((ageProgress - laneMeta.baseProgress) / branchSpan, 0, 1));
    const direction = laneOffset < 0 ? -1 : 1;
    const laneMagnitude = Math.max(1, Math.abs(laneOffset));
    const branchEase = smoothstep(0, 1, treeBranchProgress);
    const branchReach = 88 + laneMagnitude * 58;
    const treeNoise = ((Math.floor(hash / 31) % 1000) / 1000 - 0.5) * (laneMeta.isMain ? 7 : 18 + treeBranchProgress * 24);
    const trunkSway = Math.sin(ageProgress * Math.PI * 2 + branchSeed) * 8;
    const branchArc = Math.sin(branchEase * Math.PI) * (44 + laneMagnitude * 9);
    const forkWave = Math.sin(branchEase * Math.PI * (1.35 + (branchSeed % 5) * 0.08) + branchSeed) * (12 + laneMagnitude * 5);
    const canopyLift = Math.sin(branchEase * Math.PI) * (112 + laneMagnitude * 14);
    const branchYProgress = mix(laneMeta.baseProgress, laneMeta.tipProgress, branchEase);
    const treeX = laneMeta.isMain
      ? treeNoise + trunkSway * 0.4
      : direction * (branchReach * (0.12 + branchEase * 0.88) + branchArc) + forkWave + treeNoise;
    const treeY = laneMeta.isMain
      ? (0.66 - ageProgress) * treeHeight
      : (0.66 - branchYProgress) * treeHeight - canopyLift - (branchSeed % 6) * 10;
    const treeZ = laneMeta.isMain
      ? ((Math.floor(hash / 101) % 80) - 40) * 0.28
      : direction * laneMagnitude * 34 + ((Math.floor(hash / 101) % 140) - 70) * (0.22 + branchEase * 0.7);
    const colorKey = laneName === "history" && branchLabel ? branchLabel : laneName;

    const node: GalaxyNode = {
      commit,
      galaxyX,
      galaxyY,
      galaxyZ,
      levelX,
      levelY,
      levelZ,
      treeX,
      treeY,
      treeZ,
      radius: branchLabel || commit.refs.length ? 4.5 : 2.7,
      color: colorForBranch(colorKey),
      branchLabel,
      laneName,
      laneOffset,
      ageProgress,
      treeBaseProgress: laneMeta.baseProgress,
      treeTipProgress: laneMeta.tipProgress,
      treeBranchProgress,
      isMainLane: laneMeta.isMain,
    };
    nodesBySha.set(commit.sha, node);
    return node;
  });

  const edges: GalaxyEdge[] = [];
  for (const node of nodes) {
    for (const parentSha of node.commit.parent_shas) {
      const parent = nodesBySha.get(parentSha);
      if (!parent) continue;
      edges.push({
        from: parent,
        to: node,
        color: node.color,
      });
    }
  }

  return { nodes, edges, lanes, historyHeight, treeHeight };
});

const renderedCountLabel = computed(() => {
  const rendered = renderedCommits.value.length;
  const total = props.commits.length;
  if (total > rendered) {
    return `${rendered}/${total} commits rendered`;
  }
  return `${rendered} commits rendered`;
});

const viewTitle = computed(() => (viewMode.value === "tree" ? "Tree View" : "Galaxy View"));

const shapeLabel = computed(() => {
  if (viewMode.value === "tree") return "Canopy";
  if (galaxyShape.value === "layers") return "Branch Layers";
  if (galaxyShape.value === "constellation") return "Constellation";
  return "Spiral Galaxy";
});

const motionLabel = computed(() => {
  if (viewMode.value === "tree") return "Tree";
  if (galaxyMotionMode.value === "circling") return "Circling";
  if (galaxyMotionMode.value === "balanced") return "Balanced";
  return "3D";
});

const viewDetailLabel = computed(() => {
  const lanes = galaxyScene.value.lanes.length;
  return viewMode.value === "tree"
    ? `main trunk · ${lanes} branch canopy`
    : `${motionLabel.value} · ${shapeLabel.value} · ${lanes} branch levels · drag rotate`;
});

const tooltipStyle = computed(() => {
  const containerWidth = containerRef.value?.clientWidth || 0;
  return {
    left: `${Math.min(tooltip.value.x + 14, Math.max(12, containerWidth - 334))}px`,
    top: `${Math.max(12, tooltip.value.y + 14)}px`,
  };
});

function scheduleDraw() {
  if (frameId !== null) return;
  frameId = requestAnimationFrame(() => {
    frameId = null;
    drawScene();
  });
}

function isCirclingActive(): boolean {
  return viewMode.value === "galaxy" && galaxyMotionMode.value === "circling";
}

function circlingFrameDelayMs(): number {
  const count = galaxyScene.value.nodes.length;
  if (count > 30000) return 96;
  if (count > 14000) return 64;
  if (count > 6000) return 48;
  return 36;
}

function updateCirclingMetrics() {
  const count = galaxyScene.value.nodes.length;
  circlingSpeedScale = count > 30000
    ? 0.34
    : count > 14000
      ? 0.48
      : count > 6000
        ? 0.68
        : 1;
}

function scheduleCirclingFrame() {
  if (!isCirclingActive() || animationTimer !== null || frameId !== null) {
    return;
  }

  animationTimer = setTimeout(() => {
    animationTimer = null;
    scheduleDraw();
  }, circlingFrameDelayMs());
}

function stopCirclingFrame() {
  if (animationTimer !== null) {
    clearTimeout(animationTimer);
    animationTimer = null;
  }
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const clean = hex.replace("#", "");
  const value = Number.parseInt(clean.length === 3
    ? clean.split("").map((item) => item + item).join("")
    : clean, 16);
  return {
    r: (value >> 16) & 255,
    g: (value >> 8) & 255,
    b: value & 255,
  };
}

function colorWithAlpha(hex: string, alpha: number): string {
  const { r, g, b } = hexToRgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${clamp(alpha, 0, 1)})`;
}

function depthAlpha(depth: number): number {
  if (!showDepthFog.value || viewMode.value === "tree") return 1;
  const range = depthPreset.value === "deep" ? 1250 : 1650;
  return clamp(0.48 + (depth + range * 0.24) / range, 0.34, 1);
}

function depthLineWidth(depth: number, base: number): number {
  if (viewMode.value === "tree") return base;
  const strength = depthPreset.value === "deep" ? 0.52 : 0.34;
  return base * clamp(1 + (depth / 900) * strength, 0.62, 1.55);
}

function resizeCanvas() {
  const canvas = canvasRef.value;
  const container = containerRef.value;
  if (!canvas || !container) return;

  dpr = Math.min(globalThis.devicePixelRatio || 1, 2);
  const rect = container.getBoundingClientRect();
  canvasWidth = Math.max(1, Math.floor(rect.width));
  canvasHeight = Math.max(1, Math.floor(rect.height));
  canvas.width = Math.floor(canvasWidth * dpr);
  canvas.height = Math.floor(canvasHeight * dpr);
  canvas.style.width = `${canvasWidth}px`;
  canvas.style.height = `${canvasHeight}px`;
  ctx = canvas.getContext("2d", { alpha: false });
  scheduleDraw();
}

function zoomLayoutFocus(): number {
  return smoothstep(0.7, 2.65, viewport.zoom);
}

function layoutForNode(node: GalaxyNode): { x: number; y: number; z: number } {
  if (viewMode.value === "tree") {
    const focus = smoothstep(0.7, 3.2, viewport.zoom);
    return {
      x: node.treeX * (0.86 + focus * 0.36),
      y: node.treeY * (0.88 + focus * 0.2),
      z: node.treeZ * (0.72 + focus * 0.44),
    };
  }

  const focus = zoomLayoutFocus();
  const levelScale = 0.82 + focus * 0.24;
  const layerX = node.levelX;
  const layerY = node.levelY * levelScale;
  const layerZ = node.levelZ;

  if (galaxyMotionMode.value === "circling") {
    const baseRadius = Math.max(48, Math.hypot(node.galaxyX, node.galaxyY));
    const laneMagnitude = Math.max(1, Math.abs(node.laneOffset));
    const direction = node.laneOffset < 0 ? -1 : 1;
    const speed = (0.018 + laneMagnitude * 0.0018 + node.ageProgress * 0.009) * direction * circlingSpeedScale;
    const angle = Math.atan2(node.galaxyY, node.galaxyX) + galaxyAnimationTime * speed;
    const ringSquash = 0.58 + Math.min(0.16, laneMagnitude * 0.025);
    const orbitX = Math.cos(angle) * baseRadius + node.laneOffset * 16;
    const orbitY = Math.sin(angle) * baseRadius * ringSquash;
    const orbitZ = node.galaxyZ * 0.32 + Math.sin(angle + node.laneOffset * 0.45) * (82 + laneMagnitude * 9);
    return {
      x: mix(orbitX, layerX, focus * 0.68),
      y: mix(orbitY, layerY, focus * 0.64),
      z: mix(orbitZ, layerZ, focus * 0.5),
    };
  }

  if (galaxyShape.value === "layers") {
    const branchWave = Math.sin(node.ageProgress * Math.PI * 5 + node.laneOffset * 0.85) * 34;
    return {
      x: mix(layerX + branchWave * 0.26, node.galaxyX, 0.14 * (1 - focus)),
      y: layerY,
      z: layerZ + branchWave + node.laneOffset * 24,
    };
  }

  if (galaxyShape.value === "constellation") {
    const arc = node.ageProgress * Math.PI * 2.6 + node.laneOffset * 0.42;
    const laneMagnitude = Math.max(1, Math.abs(node.laneOffset));
    const branchRadius = 78 + laneMagnitude * 28;
    const x = node.laneOffset * 104 + Math.sin(arc) * branchRadius * 0.26;
    const y = mix(node.levelY * 0.92, node.galaxyY * 0.72, 0.22 * (1 - focus));
    const z = Math.cos(arc) * (115 + laneMagnitude * 18) + node.levelZ * 0.45;
    return {
      x: mix(x, layerX, focus * 0.58),
      y: mix(y, layerY, focus * 0.82),
      z: mix(z, layerZ, focus * 0.52),
    };
  }

  return {
    x: mix(node.galaxyX, layerX, focus),
    y: mix(node.galaxyY, layerY, focus),
    z: mix(node.galaxyZ, layerZ, focus),
  };
}

function projectLayoutPoint(x: number, y: number, z: number): { screenX: number; screenY: number; depth: number; scale: number } {
  if (viewMode.value === "tree") {
    const depthScale = 1 + clamp(z / 1400, -0.18, 0.22);
    return {
      screenX: canvasWidth / 2 + viewport.panX + x * viewport.zoom * depthScale,
      screenY: canvasHeight / 2 + viewport.panY + (y + z * 0.06) * viewport.zoom,
      depth: z,
      scale: depthScale,
    };
  }

  const focus = zoomLayoutFocus();
  const rotation = mix(viewport.rotation, -0.08, focus * 0.2);
  const pitch = mix(viewport.tilt, 0.46, focus * 0.36);
  const cosYaw = Math.cos(rotation);
  const sinYaw = Math.sin(rotation);
  const xYaw = x * cosYaw - y * sinYaw;
  const yYaw = x * sinYaw + y * cosYaw;
  const cosPitch = Math.cos(pitch);
  const sinPitch = Math.sin(pitch);
  const yPitch = yYaw * cosPitch - z * sinPitch;
  const depth = yYaw * sinPitch + z * cosPitch;
  const cameraDistance = depthPreset.value === "deep" ? 1120 : 1520;
  const depthScale = clamp(cameraDistance / Math.max(260, cameraDistance - depth), 0.54, depthPreset.value === "deep" ? 1.82 : 1.46);
  return {
    screenX: canvasWidth / 2 + viewport.panX + xYaw * viewport.zoom * depthScale,
    screenY: canvasHeight / 2 + viewport.panY + yPitch * viewport.zoom * depthScale,
    depth,
    scale: depthScale,
  };
}

function projectNodeInto(node: GalaxyNode, target?: ProjectedNode): ProjectedNode {
  const layout = layoutForNode(node);
  const projected = projectLayoutPoint(layout.x, layout.y, layout.z);
  const output = target || ({ ...node } as ProjectedNode);
  Object.assign(output, node);
  output.screenX = projected.screenX;
  output.screenY = projected.screenY;
  output.depth = projected.depth;
  output.screenRadius = Math.max(1.5, node.radius * viewport.zoom * projected.scale);
  return output;
}

function projectSceneNodes(): ProjectedNode[] {
  const nodes = galaxyScene.value.nodes;
  if (projectedNodes.length > nodes.length) {
    projectedNodes.length = nodes.length;
  }

  for (let index = 0; index < nodes.length; index += 1) {
    projectedNodes[index] = projectNodeInto(nodes[index], projectedNodes[index]);
  }

  projectedNodes.sort((a, b) => a.depth - b.depth);
  return projectedNodes;
}

function drawBackground(context: CanvasRenderingContext2D) {
  const gradient = viewMode.value === "tree"
    ? context.createLinearGradient(0, 0, 0, canvasHeight * dpr)
    : context.createRadialGradient(
      canvasWidth * 0.48 * dpr,
      canvasHeight * 0.42 * dpr,
      10,
      canvasWidth * 0.5 * dpr,
      canvasHeight * 0.5 * dpr,
      Math.max(canvasWidth, canvasHeight) * 0.78 * dpr,
    );

  if (viewMode.value === "tree") {
    gradient.addColorStop(0, "#06121f");
    gradient.addColorStop(0.48, "#07111f");
    gradient.addColorStop(1, "#03110d");
  } else {
    gradient.addColorStop(0, "#172554");
    gradient.addColorStop(0.42, "#07111f");
    gradient.addColorStop(1, "#020617");
  }

  context.fillStyle = gradient;
  context.fillRect(0, 0, canvasWidth * dpr, canvasHeight * dpr);

  context.save();
  context.scale(dpr, dpr);
  if (viewMode.value === "galaxy" && showBranchAura.value) {
    const nebulaCount = Math.min(6, galaxyScene.value.lanes.length || 3);
    for (let index = 0; index < nebulaCount; index += 1) {
      const lane = galaxyScene.value.lanes[index];
      const seed = hashText(`nebula:${lane?.name || index}`);
      const x = ((seed % 9000) / 9000) * canvasWidth;
      const y = ((Math.floor(seed / 9000) % 9000) / 9000) * canvasHeight;
      const radius = Math.max(canvasWidth, canvasHeight) * (0.18 + (seed % 7) * 0.012);
      const nebula = context.createRadialGradient(x, y, 0, x, y, radius);
      nebula.addColorStop(0, colorWithAlpha(lane?.color || "#7dd3fc", 0.1));
      nebula.addColorStop(0.48, colorWithAlpha(lane?.color || "#7dd3fc", 0.035));
      nebula.addColorStop(1, "rgba(2, 6, 23, 0)");
      context.globalAlpha = 1;
      context.fillStyle = nebula;
      context.beginPath();
      context.arc(x, y, radius, 0, Math.PI * 2);
      context.fill();
    }
  }

  const starCount = viewMode.value === "tree" ? Math.floor(STARFIELD_COUNT * 0.45) : STARFIELD_COUNT;
  for (let i = 0; i < starCount; i += 1) {
    const seed = hashText(`${i}:gitswamp-${viewMode.value}`);
    const x = (seed % 10000) / 10000 * canvasWidth;
    const y = (Math.floor(seed / 10000) % 10000) / 10000 * canvasHeight;
    const radius = 0.45 + (seed % 7) * 0.08;
    context.globalAlpha = viewMode.value === "tree" ? 0.08 + (seed % 11) / 120 : 0.18 + (seed % 17) / 90;
    context.fillStyle = i % 9 === 0 ? "#bae6fd" : "#e0f2fe";
    context.beginPath();
    context.arc(x, y, radius, 0, Math.PI * 2);
    context.fill();
  }
  context.restore();
}

function drawProjectedOrbit(
  context: CanvasRenderingContext2D,
  radiusX: number,
  radiusY: number,
  z: number,
  color: string,
  alpha: number,
  width: number,
) {
  const steps = 96;
  context.save();
  context.globalAlpha = alpha;
  context.strokeStyle = color;
  context.lineWidth = width;
  context.beginPath();
  for (let index = 0; index <= steps; index += 1) {
    const angle = (index / steps) * Math.PI * 2;
    const point = projectLayoutPoint(Math.cos(angle) * radiusX, Math.sin(angle) * radiusY, z);
    if (index === 0) {
      context.moveTo(point.screenX, point.screenY);
    } else {
      context.lineTo(point.screenX, point.screenY);
    }
  }
  context.closePath();
  context.stroke();
  context.restore();
}

function drawTimeRings(context: CanvasRenderingContext2D) {
  if (viewMode.value !== "galaxy" || !showTimeRings.value) return;

  const scene = galaxyScene.value;
  const maxRadius = Math.max(150, Math.sqrt(scene.nodes.length + 1) * 21 + 78);
  const circling = galaxyMotionMode.value === "circling";
  const rings = circling ? 7 : galaxyShape.value === "layers" ? 5 : 6;
  for (let index = 1; index <= rings; index += 1) {
    const progress = index / rings;
    const radius = maxRadius * progress;
    const color = circling && index % 2 === 0 ? "#22d3ee" : index === rings ? "#93c5fd" : "#60a5fa";
    const pulse = circling ? 0.015 * (1 + Math.sin(galaxyAnimationTime * 0.7 + index)) : 0;
    drawProjectedOrbit(
      context,
      radius,
      radius * (circling ? 0.56 : galaxyShape.value === "constellation" ? 0.42 : 0.72),
      (index % 2 === 0 ? -36 : 34) * (depthPreset.value === "deep" ? 1.4 : 0.9),
      color,
      0.035 + progress * 0.025 + pulse,
      Math.max(0.6, viewport.zoom * 0.55),
    );
  }
}

function drawGalaxyGuides(context: CanvasRenderingContext2D) {
  const scene = galaxyScene.value;
  const focus = zoomLayoutFocus();
  if (focus < 0.08) return;

  context.save();
  context.lineCap = "round";
  context.setLineDash([5, 12]);

  const guideTop = -scene.historyHeight * 0.48;
  const guideBottom = scene.historyHeight * 0.48;
  const visibleLanes = scene.lanes.slice(0, 16);
  for (const lane of visibleLanes) {
    const x = lane.offset * 118;
    const from = projectLayoutPoint(x, guideTop, 0);
    const to = projectLayoutPoint(x, guideBottom, 0);
    context.globalAlpha = 0.05 + focus * 0.11;
    context.strokeStyle = lane.name === currentBranchName.value ? "#fef08a" : lane.color;
    context.lineWidth = lane.name === currentBranchName.value ? 1.8 : 1;
    context.beginPath();
    context.moveTo(from.screenX, from.screenY);
    context.lineTo(to.screenX, to.screenY);
    context.stroke();
  }

  context.setLineDash([]);
  const horizontalWidth = Math.max(520, visibleLanes.length * 150);
  for (let index = 0; index <= 4; index += 1) {
    const y = mix(guideTop, guideBottom, index / 4);
    const from = projectLayoutPoint(-horizontalWidth / 2, y, -20);
    const to = projectLayoutPoint(horizontalWidth / 2, y, -20);
    context.globalAlpha = 0.03 + focus * 0.07;
    context.strokeStyle = "#bfdbfe";
    context.lineWidth = 0.8;
    context.beginPath();
    context.moveTo(from.screenX, from.screenY);
    context.lineTo(to.screenX, to.screenY);
    context.stroke();
  }

  context.restore();
}

function drawTreeGuides(context: CanvasRenderingContext2D) {
  const scene = galaxyScene.value;
  const trunkRoot = projectLayoutPoint(0, (0.66 - 0) * scene.treeHeight, -30);
  const trunkCrown = projectLayoutPoint(0, (0.66 - 1) * scene.treeHeight, 34);
  const crown = projectLayoutPoint(0, (0.66 - 0.9) * scene.treeHeight, 0);
  const canopyWidth = Math.max(230, (scene.lanes.length + 2) * 62) * viewport.zoom;
  const canopyHeight = Math.max(140, scene.treeHeight * 0.12) * viewport.zoom;

  context.save();
  context.globalAlpha = 0.07;
  context.fillStyle = "#86efac";
  context.beginPath();
  const canopyPoints = 18;
  for (let index = 0; index <= canopyPoints; index += 1) {
    const angle = (index / canopyPoints) * Math.PI * 2 - Math.PI / 2;
    const seed = hashText(`canopy:${index}:${scene.lanes.length}`);
    const wobble = 0.82 + (seed % 38) / 100;
    const lobe = 1 + Math.sin(angle * 3.2 + scene.lanes.length * 0.17) * 0.12;
    const x = crown.screenX + Math.cos(angle) * canopyWidth * wobble * lobe;
    const y = crown.screenY + Math.sin(angle) * canopyHeight * (0.72 + (seed % 23) / 100);
    if (index === 0) {
      context.moveTo(x, y);
    } else {
      const previousAngle = ((index - 0.5) / canopyPoints) * Math.PI * 2 - Math.PI / 2;
      const cx = crown.screenX + Math.cos(previousAngle) * canopyWidth * 1.02;
      const cy = crown.screenY + Math.sin(previousAngle) * canopyHeight * 0.86;
      context.quadraticCurveTo(cx, cy, x, y);
    }
  }
  context.closePath();
  context.fill();

  for (const lane of scene.lanes) {
    if (lane.isMain) continue;
    const direction = lane.offset < 0 ? -1 : 1;
    const laneMagnitude = Math.max(1, Math.abs(lane.offset));
    const branchReach = 74 + laneMagnitude * 64;
    const branchTip = projectLayoutPoint(direction * branchReach, (0.66 - lane.tipProgress) * scene.treeHeight - 20, direction * laneMagnitude * 28);
    context.globalAlpha = 0.045;
    context.fillStyle = lane.color;
    context.beginPath();
    context.ellipse(
      branchTip.screenX,
      branchTip.screenY,
      Math.max(58, 120 * viewport.zoom),
      Math.max(34, 70 * viewport.zoom),
      direction * 0.28,
      0,
      Math.PI * 2,
    );
    context.fill();
  }

  const trunkGradient = context.createLinearGradient(trunkRoot.screenX, trunkRoot.screenY, trunkCrown.screenX, trunkCrown.screenY);
  trunkGradient.addColorStop(0, "#5f3b1f");
  trunkGradient.addColorStop(0.48, "#8b5a2b");
  trunkGradient.addColorStop(1, "#b7791f");
  context.globalAlpha = 0.36;
  context.strokeStyle = trunkGradient;
  context.lineWidth = Math.max(7, Math.min(24, viewport.zoom * 10));
  context.lineCap = "round";
  context.beginPath();
  context.moveTo(trunkRoot.screenX, trunkRoot.screenY);
  context.bezierCurveTo(
    trunkRoot.screenX - 18 * viewport.zoom,
    mix(trunkRoot.screenY, trunkCrown.screenY, 0.35),
    trunkCrown.screenX + 18 * viewport.zoom,
    mix(trunkRoot.screenY, trunkCrown.screenY, 0.74),
    trunkCrown.screenX,
    trunkCrown.screenY,
  );
  context.stroke();

  const rootCount = 5;
  for (let index = 0; index < rootCount; index += 1) {
    const offset = index - Math.floor(rootCount / 2);
    if (offset === 0) continue;
    const direction = offset < 0 ? -1 : 1;
    const rootReach = (44 + Math.abs(offset) * 28) * viewport.zoom;
    const rootDip = (18 + Math.abs(offset) * 9) * viewport.zoom;
    context.globalAlpha = 0.2;
    context.strokeStyle = "#6b4423";
    context.lineWidth = Math.max(1.8, Math.min(8, viewport.zoom * (3.2 - Math.abs(offset) * 0.18)));
    context.beginPath();
    context.moveTo(trunkRoot.screenX, trunkRoot.screenY - 2 * viewport.zoom);
    context.quadraticCurveTo(
      trunkRoot.screenX + direction * rootReach * 0.42,
      trunkRoot.screenY + rootDip * 0.24,
      trunkRoot.screenX + direction * rootReach,
      trunkRoot.screenY + rootDip,
    );
    context.stroke();
  }

  for (const lane of scene.lanes) {
    if (lane.isMain || lane.name === "history") {
      continue;
    }

    const direction = lane.offset < 0 ? -1 : 1;
    const laneMagnitude = Math.max(1, Math.abs(lane.offset));
    const branchReach = 74 + laneMagnitude * 64;
    const startY = (0.66 - lane.baseProgress) * scene.treeHeight;
    const endY = (0.66 - lane.tipProgress) * scene.treeHeight - 20;
    const start = projectLayoutPoint(0, startY, 0);
    const end = projectLayoutPoint(direction * branchReach, endY, direction * laneMagnitude * 28);
    const midX = direction * branchReach * 0.42 * viewport.zoom;

    context.globalAlpha = 0.2;
    context.strokeStyle = "#6b4423";
    context.lineWidth = Math.max(2.8, Math.min(14, viewport.zoom * (5.2 - Math.min(2, laneMagnitude) * 0.42)));
    context.beginPath();
    context.moveTo(start.screenX, start.screenY);
    context.bezierCurveTo(
      start.screenX + midX,
      mix(start.screenY, end.screenY, 0.2),
      end.screenX - midX * 0.25,
      mix(start.screenY, end.screenY, 0.82),
      end.screenX,
      end.screenY,
    );
    context.stroke();

    context.globalAlpha = 0.22;
    context.strokeStyle = lane.color;
    context.lineWidth = Math.max(0.9, Math.min(4.5, viewport.zoom * 1.25));
    context.beginPath();
    context.moveTo(start.screenX, start.screenY);
    context.bezierCurveTo(
      start.screenX + midX,
      mix(start.screenY, end.screenY, 0.2),
      end.screenX - midX * 0.25,
      mix(start.screenY, end.screenY, 0.82),
      end.screenX,
      end.screenY,
    );
    context.stroke();
  }

  context.restore();
}

function drawBranchAuras(context: CanvasRenderingContext2D, nodes: ProjectedNode[]) {
  if (viewMode.value !== "galaxy" || !showBranchAura.value) return;

  const laneStats = new Map<string, {
    color: string;
    count: number;
    x: number;
    y: number;
    minX: number;
    maxX: number;
    minY: number;
    maxY: number;
    depth: number;
  }>();
  const step = Math.max(1, Math.floor(nodes.length / 9000));
  for (let index = 0; index < nodes.length; index += step) {
    const node = nodes[index];
    if (
      node.screenX < -120
      || node.screenX > canvasWidth + 120
      || node.screenY < -120
      || node.screenY > canvasHeight + 120
    ) {
      continue;
    }

    const weight = depthAlpha(node.depth);
    const stat = laneStats.get(node.laneName) || {
      color: node.color,
      count: 0,
      x: 0,
      y: 0,
      minX: node.screenX,
      maxX: node.screenX,
      minY: node.screenY,
      maxY: node.screenY,
      depth: 0,
    };
    stat.count += weight;
    stat.x += node.screenX * weight;
    stat.y += node.screenY * weight;
    stat.minX = Math.min(stat.minX, node.screenX);
    stat.maxX = Math.max(stat.maxX, node.screenX);
    stat.minY = Math.min(stat.minY, node.screenY);
    stat.maxY = Math.max(stat.maxY, node.screenY);
    stat.depth += node.depth * weight;
    laneStats.set(node.laneName, stat);
  }

  const orderedLaneNames = galaxyScene.value.lanes.map((lane) => lane.name);
  context.save();
  context.globalCompositeOperation = "lighter";
  for (const laneName of orderedLaneNames.slice(0, 18)) {
    const stat = laneStats.get(laneName);
    if (!stat || stat.count < 1.5) continue;

    const x = stat.x / stat.count;
    const y = stat.y / stat.count;
    const avgDepth = stat.depth / stat.count;
    const width = clamp((stat.maxX - stat.minX) * 0.64, 70, 360);
    const height = clamp((stat.maxY - stat.minY) * 0.32, 42, 220);
    const alpha = (laneName === currentBranchName.value ? 0.12 : 0.065) * depthAlpha(avgDepth);
    const aura = context.createRadialGradient(x, y, 0, x, y, Math.max(width, height));
    aura.addColorStop(0, colorWithAlpha(stat.color, alpha));
    aura.addColorStop(0.52, colorWithAlpha(stat.color, alpha * 0.36));
    aura.addColorStop(1, "rgba(2, 6, 23, 0)");
    context.fillStyle = aura;
    context.beginPath();
    context.ellipse(x, y, width, height, viewport.rotation * 0.28, 0, Math.PI * 2);
    context.fill();
  }
  context.restore();
}

function drawMiniMap(context: CanvasRenderingContext2D) {
  if (!showMiniMap.value || viewMode.value !== "galaxy" || canvasWidth < 420 || canvasHeight < 300) return;

  const nodes = galaxyScene.value.nodes;
  if (!nodes.length) return;

  const width = 168;
  const height = 104;
  const left = 12;
  const top = canvasHeight - height - 12;
  const padding = 9;
  const step = Math.max(1, Math.ceil(nodes.length / MINI_MAP_MAX_POINTS));
  const plotted: { node: GalaxyNode; x: number; y: number }[] = [];
  let minX = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (let index = 0; index < nodes.length; index += step) {
    const node = nodes[index];
    const layout = layoutForNode(node);
    plotted.push({ node, x: layout.x, y: layout.y });
    minX = Math.min(minX, layout.x);
    maxX = Math.max(maxX, layout.x);
    minY = Math.min(minY, layout.y);
    maxY = Math.max(maxY, layout.y);
  }

  const rangeX = Math.max(1, maxX - minX);
  const rangeY = Math.max(1, maxY - minY);

  context.save();
  context.globalAlpha = 1;
  context.fillStyle = "rgba(2, 6, 23, 0.58)";
  context.strokeStyle = "rgba(148, 163, 184, 0.18)";
  context.lineWidth = 1;
  context.beginPath();
  context.rect(left, top, width, height);
  context.fill();
  context.stroke();

  for (const item of plotted) {
    const x = left + padding + ((item.x - minX) / rangeX) * (width - padding * 2);
    const y = top + padding + ((item.y - minY) / rangeY) * (height - padding * 2);
    const selected = item.node.commit.sha === props.selectedSha || item.node.commit.sha === hoveredNode.value?.commit.sha;
    const currentBranch = normalizeBranchName(item.node.laneName) === currentBranchName.value;
    context.globalAlpha = selected ? 0.9 : currentBranch ? 0.5 : 0.28;
    context.fillStyle = selected ? "#ffffff" : currentBranch ? "#fef08a" : item.node.color;
    context.beginPath();
    context.arc(x, y, selected ? 2.3 : 1.25, 0, Math.PI * 2);
    context.fill();
  }
  context.restore();
}

function drawStraightEdge(context: CanvasRenderingContext2D, from: ProjectedNode, to: ProjectedNode, color: string) {
  const avgDepth = (from.depth + to.depth) / 2;
  const selectedEdge = from.commit.sha === props.selectedSha || to.commit.sha === props.selectedSha;
  const alpha = (selectedEdge ? 0.34 : 0.12 + Math.min(0.16, viewport.zoom * 0.035)) * depthAlpha(avgDepth);
  context.globalAlpha = alpha;
  context.strokeStyle = showDepthFog.value ? colorWithAlpha(color, 0.96) : color;
  context.lineWidth = Math.max(0.5, depthLineWidth(avgDepth, viewport.zoom * 0.8));
  context.beginPath();
  context.moveTo(from.screenX, from.screenY);
  if (galaxyShape.value === "constellation") {
    const midX = mix(from.screenX, to.screenX, 0.5);
    const midY = mix(from.screenY, to.screenY, 0.5);
    const curve = clamp((to.depth - from.depth) * 0.12, -36, 36);
    context.quadraticCurveTo(midX + curve, midY - Math.abs(curve) * 0.35, to.screenX, to.screenY);
  } else {
    context.lineTo(to.screenX, to.screenY);
  }
  context.stroke();
}

function drawTreeEdge(context: CanvasRenderingContext2D, from: ProjectedNode, to: ProjectedNode, color: string) {
  const midY = mix(from.screenY, to.screenY, 0.55);
  const curve = clamp((to.screenX - from.screenX) * 0.32, -90, 90);
  const mainEdge = from.isMainLane && to.isMainLane;
  const branchWidth = mainEdge
    ? Math.max(2.4, Math.min(8.5, viewport.zoom * 2.9))
    : Math.max(1.2, Math.min(6.5, viewport.zoom * (to.branchLabel ? 2.1 : 1.35)));

  context.globalAlpha = mainEdge ? 0.32 + Math.min(0.2, viewport.zoom * 0.035) : 0.2 + Math.min(0.2, viewport.zoom * 0.04);
  context.strokeStyle = mainEdge ? "#9a5f24" : "#6b4423";
  context.lineWidth = branchWidth;
  context.beginPath();
  context.moveTo(from.screenX, from.screenY);
  context.bezierCurveTo(from.screenX + curve, midY, to.screenX - curve, midY, to.screenX, to.screenY);
  context.stroke();

  if (!mainEdge) {
    context.globalAlpha = 0.28 + Math.min(0.22, viewport.zoom * 0.035);
    context.strokeStyle = color;
    context.lineWidth = Math.max(0.7, branchWidth * 0.34);
    context.beginPath();
    context.moveTo(from.screenX, from.screenY);
    context.bezierCurveTo(from.screenX + curve, midY, to.screenX - curve, midY, to.screenX, to.screenY);
    context.stroke();
  }
}

function drawNode(context: CanvasRenderingContext2D, node: ProjectedNode) {
  const selected = node.commit.sha === props.selectedSha;
  const hovered = hoveredNode.value?.commit.sha === node.commit.sha;
  const branch = !!node.branchLabel;
  const merge = node.commit.parent_shas.length > 1;
  const currentBranch = normalizeBranchName(node.laneName) === currentBranchName.value;
  const treeLeafBoost = viewMode.value === "tree" ? 1.12 + node.ageProgress * 0.28 : 1;
  const radius = node.screenRadius * treeLeafBoost * (selected ? 1.75 : hovered ? 1.55 : currentBranch ? 1.25 : 1);

  if (
    node.screenX < -radius
    || node.screenX > canvasWidth + radius
    || node.screenY < -radius
    || node.screenY > canvasHeight + radius
  ) {
    return;
  }

  const glow = context.createRadialGradient(node.screenX, node.screenY, 0, node.screenX, node.screenY, radius * 4.2);
  glow.addColorStop(0, selected ? "#ffffff" : node.color);
  glow.addColorStop(0.24, currentBranch ? "#fef08a" : node.color);
  glow.addColorStop(1, "rgba(2, 6, 23, 0)");
  const visibility = depthAlpha(node.depth);
  context.globalAlpha = (branch || selected || hovered || currentBranch ? 0.76 : viewMode.value === "tree" ? 0.48 : 0.34) * visibility;
  context.fillStyle = glow;
  context.beginPath();
  context.arc(node.screenX, node.screenY, radius * 4.2, 0, Math.PI * 2);
  context.fill();

  context.globalAlpha = 0.94 * visibility;
  context.fillStyle = selected ? "#ffffff" : currentBranch ? "#fef08a" : node.color;
  context.beginPath();
  if (viewMode.value === "tree" && !selected && !hovered) {
    context.ellipse(node.screenX, node.screenY, radius * 1.18, radius * 0.82, node.laneOffset * 0.08, 0, Math.PI * 2);
  } else {
    context.arc(node.screenX, node.screenY, radius, 0, Math.PI * 2);
  }
  context.fill();

  if (highlightMerges.value && merge) {
    context.globalAlpha = (selected || hovered ? 0.78 : 0.5) * visibility;
    context.strokeStyle = "#f97316";
    context.lineWidth = Math.max(1, depthLineWidth(node.depth, viewport.zoom * 0.55));
    context.beginPath();
    context.arc(node.screenX, node.screenY, radius * 1.85, 0, Math.PI * 2);
    context.stroke();

    if (viewMode.value === "galaxy") {
      context.globalAlpha = (selected || hovered ? 0.5 : 0.26) * visibility;
      context.strokeStyle = "#fed7aa";
      context.lineWidth = Math.max(0.7, viewport.zoom * 0.38);
      context.beginPath();
      context.moveTo(node.screenX, node.screenY - radius * 3.2);
      context.lineTo(node.screenX, node.screenY + radius * 3.2);
      context.stroke();
    }
  }

  if (showLabels.value && branch && viewport.zoom > 0.38) {
    context.globalAlpha = 0.86 * visibility;
    context.font = "11px ui-sans-serif, system-ui, sans-serif";
    context.fillStyle = currentBranch ? "#fef9c3" : "#e0f2fe";
    context.fillText(node.branchLabel || "", node.screenX + radius + 6, node.screenY + 3);
  }
}

function drawScene() {
  if (!ctx) return;
  if (isCirclingActive()) {
    galaxyAnimationTime = performance.now() * 0.001;
    updateCirclingMetrics();
  }
  const context = ctx;
  drawBackground(context);

  context.save();
  context.scale(dpr, dpr);
  projectedNodes = projectSceneNodes();
  projectedBySha.clear();
  for (const node of projectedNodes) {
    projectedBySha.set(node.commit.sha, node);
  }

  drawTimeRings(context);

  if (showGuides.value) {
    if (viewMode.value === "tree") {
      drawTreeGuides(context);
    } else {
      drawGalaxyGuides(context);
    }
  }

  drawBranchAuras(context, projectedNodes);

  context.lineCap = "round";
  for (const edge of galaxyScene.value.edges) {
    const from = projectedBySha.get(edge.from.commit.sha);
    const to = projectedBySha.get(edge.to.commit.sha);
    if (!from || !to) continue;
    if (
      (from.screenX < -80 && to.screenX < -80)
      || (from.screenX > canvasWidth + 80 && to.screenX > canvasWidth + 80)
      || (from.screenY < -80 && to.screenY < -80)
      || (from.screenY > canvasHeight + 80 && to.screenY > canvasHeight + 80)
    ) {
      continue;
    }

    if (viewMode.value === "tree") {
      drawTreeEdge(context, from, to, edge.color);
    } else {
      drawStraightEdge(context, from, to, edge.color);
    }
  }

  for (const node of projectedNodes) {
    drawNode(context, node);
  }

  drawMiniMap(context);

  context.globalAlpha = 1;
  context.restore();
  scheduleCirclingFrame();
}

function pointerPosition(event: MouseEvent | PointerEvent | WheelEvent): { x: number; y: number } {
  const rect = canvasRef.value?.getBoundingClientRect();
  if (!rect) return { x: 0, y: 0 };
  return {
    x: event.clientX - rect.left,
    y: event.clientY - rect.top,
  };
}

function nodeVisualRadius(node: ProjectedNode): number {
  const selected = node.commit.sha === props.selectedSha;
  const hovered = hoveredNode.value?.commit.sha === node.commit.sha;
  const currentBranch = normalizeBranchName(node.laneName) === currentBranchName.value;
  const treeLeafBoost = viewMode.value === "tree" ? 1.12 + node.ageProgress * 0.28 : 1;
  return node.screenRadius * treeLeafBoost * (selected ? 1.75 : hovered ? 1.55 : currentBranch ? 1.25 : 1);
}

function findNodeAtPointer(pointer: { x: number; y: number }, strict = false): ProjectedNode | null {
  let bestNode: ProjectedNode | null = null;
  let bestScore = Number.POSITIVE_INFINITY;
  const clickPadding = strict
    ? (viewMode.value === "tree" ? 5 : 4)
    : (viewMode.value === "tree" ? 8 : 7);
  const maxRadius = strict
    ? (viewMode.value === "tree" ? 14 : 12)
    : (viewMode.value === "tree" ? 19 : 17);

  for (let i = projectedNodes.length - 1; i >= 0; i -= 1) {
    const node = projectedNodes[i];
    const dx = node.screenX - pointer.x;
    const dy = node.screenY - pointer.y;
    const distance = Math.hypot(dx, dy);
    const radius = clamp(nodeVisualRadius(node) + clickPadding, 5, maxRadius);

    if (distance > radius) {
      continue;
    }

    const frontBias = (i / Math.max(1, projectedNodes.length - 1)) * 0.035;
    const focusBias = node.commit.sha === props.selectedSha ? 0.015 : 0;
    const score = distance / radius - frontBias - focusBias;
    if (score < bestScore) {
      bestScore = score;
      bestNode = node;
    }
  }

  return bestNode;
}

function updateHover(event: PointerEvent) {
  const pointer = pointerPosition(event);
  tooltip.value = { x: pointer.x, y: pointer.y };
  const nextHover = findNodeAtPointer(pointer);

  if (nextHover?.commit.sha !== hoveredNode.value?.commit.sha) {
    hoveredNode.value = nextHover;
    scheduleDraw();
  }
}

function onPointerDown(event: PointerEvent) {
  isDragging.value = true;
  lastPointerX = event.clientX;
  lastPointerY = event.clientY;
  pointerDownX = event.clientX;
  pointerDownY = event.clientY;
  draggedSincePointerDown = false;
  canvasRef.value?.setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
  if (isDragging.value) {
    if (Math.hypot(event.clientX - pointerDownX, event.clientY - pointerDownY) > 4) {
      draggedSincePointerDown = true;
    }
    const deltaX = event.clientX - lastPointerX;
    const deltaY = event.clientY - lastPointerY;
    if (viewMode.value === "galaxy") {
      viewport.rotation += deltaX * 0.006;
      viewport.tilt = clamp(viewport.tilt + deltaY * 0.0025, 0.28, 1.28);
    } else {
      viewport.panX += deltaX;
      viewport.panY += deltaY;
    }
    lastPointerX = event.clientX;
    lastPointerY = event.clientY;
    scheduleDraw();
  }
  updateHover(event);
}

function onPointerUp(event: PointerEvent) {
  isDragging.value = false;
  canvasRef.value?.releasePointerCapture(event.pointerId);
}

function focusNode(node: GalaxyNode, zoomIn = false) {
  focusedSha.value = node.commit.sha;
  if (zoomIn) {
    viewport.zoom = Math.min(MAX_ZOOM, Math.max(viewport.zoom * 1.18, viewMode.value === "galaxy" ? 2.25 : 1.35));
  }

  const layout = layoutForNode(node);
  const projected = projectLayoutPoint(layout.x, layout.y, layout.z);
  viewport.panX += canvasWidth / 2 - projected.screenX;
  viewport.panY += canvasHeight / 2 - projected.screenY;
  scheduleDraw();
}

function focusCommit(sha?: string | null, zoomIn = false) {
  const targetSha = sha || hoveredNode.value?.commit.sha || props.selectedSha;
  if (!targetSha) return;

  const node = galaxyScene.value.nodes.find((item) => item.commit.sha === targetSha);
  if (node) {
    focusNode(node, zoomIn);
  }
}

function onClick(event: MouseEvent) {
  if (draggedSincePointerDown) {
    return;
  }

  const clickedNode = findNodeAtPointer(pointerPosition(event), true);
  if (clickedNode) {
    hoveredNode.value = clickedNode;
    focusedSha.value = clickedNode.commit.sha;
    if (viewMode.value === "galaxy") {
      focusNode(clickedNode, true);
    }
    emit("select", { commit: clickedNode.commit, additive: event.ctrlKey || event.metaKey });
  }
}

function onWheel(event: WheelEvent) {
  event.preventDefault();
  const previousZoom = viewport.zoom;
  const nextZoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, previousZoom * Math.exp(-event.deltaY * 0.0012)));
  if (nextZoom === previousZoom) return;

  const zoomingIn = nextZoom > previousZoom;
  const zoomRatio = nextZoom / previousZoom;
  viewport.zoom = nextZoom;

  if (zoomingIn) {
    const targetSha = focusedSha.value || props.selectedSha || null;
    const target = targetSha ? galaxyScene.value.nodes.find((item) => item.commit.sha === targetSha) : null;
    if (target) {
      focusNode(target, false);
    } else {
      focusedSha.value = null;
      viewport.panX = 0;
      viewport.panY = viewMode.value === "tree" ? viewport.panY * zoomRatio : 0;
    }
  } else {
    focusedSha.value = null;
    viewport.panX = mix(viewport.panX * zoomRatio, 0, 0.45);
    viewport.panY = mix(viewport.panY * zoomRatio, viewMode.value === "tree" ? 28 : 0, 0.45);
    if (nextZoom <= 1.02) {
      viewport.panX = 0;
      viewport.panY = viewMode.value === "tree" ? 28 : 0;
    }
  }

  scheduleDraw();
}

function resetView() {
  viewport.zoom = viewMode.value === "tree" ? 0.92 : 1;
  viewport.panX = 0;
  viewport.panY = viewMode.value === "tree" ? 28 : 0;
  viewport.rotation = -0.48;
  viewport.tilt = 0.58;
  hoveredNode.value = null;
  focusedSha.value = null;
  scheduleDraw();
}

function toggleTreeView() {
  viewMode.value = viewMode.value === "tree" ? "galaxy" : "tree";
  if (viewMode.value === "tree") {
    stopCirclingFrame();
  }
  resetView();
}

function toggleLabels() {
  showLabels.value = !showLabels.value;
  scheduleDraw();
}

function toggleGuides() {
  showGuides.value = !showGuides.value;
  scheduleDraw();
}

function toggleTimeRings() {
  showTimeRings.value = !showTimeRings.value;
  scheduleDraw();
}

function toggleBranchAura() {
  showBranchAura.value = !showBranchAura.value;
  scheduleDraw();
}

function toggleDepthFog() {
  showDepthFog.value = !showDepthFog.value;
  scheduleDraw();
}

function toggleMiniMap() {
  showMiniMap.value = !showMiniMap.value;
  scheduleDraw();
}

function toggleMerges() {
  highlightMerges.value = !highlightMerges.value;
  scheduleDraw();
}

function toggleViewOptionsMenu() {
  showViewOptionsMenu.value = !showViewOptionsMenu.value;
}

function closeViewOptionsMenu() {
  showViewOptionsMenu.value = false;
}

function handleDocumentPointerDown(event: PointerEvent) {
  if (!showViewOptionsMenu.value) return;
  const target = event.target as Node | null;
  if (target && viewOptionsRef.value?.contains(target)) {
    return;
  }
  closeViewOptionsMenu();
}

onMounted(async () => {
  await nextTick();
  document.addEventListener("pointerdown", handleDocumentPointerDown);
  resizeCanvas();
  resizeObserver = new ResizeObserver(resizeCanvas);
  if (containerRef.value) {
    resizeObserver.observe(containerRef.value);
  }
});

onUnmounted(() => {
  document.removeEventListener("pointerdown", handleDocumentPointerDown);
  resizeObserver?.disconnect();
  stopCirclingFrame();
  if (frameId !== null) {
    cancelAnimationFrame(frameId);
    frameId = null;
  }
});

watch(galaxyScene, () => {
  hoveredNode.value = null;
  scheduleDraw();
});

watch(galaxyShape, () => {
  resetView();
});

watch(galaxyMotionMode, () => {
  if (!isCirclingActive()) {
    stopCirclingFrame();
  }
  scheduleDraw();
  scheduleCirclingFrame();
});

watch(viewMode, () => {
  if (!isCirclingActive()) {
    stopCirclingFrame();
  }
  scheduleDraw();
  scheduleCirclingFrame();
});

watch(depthPreset, scheduleDraw);

watch(() => props.selectedSha, scheduleDraw);
</script>

<template>
  <div ref="containerRef" class="relative h-full w-full overflow-hidden bg-[#020617] text-white">
    <canvas
      ref="canvasRef"
      class="block h-full w-full cursor-grab touch-none active:cursor-grabbing"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
      @click="onClick"
      @wheel="onWheel"
    />

    <div class="pointer-events-none absolute left-3 top-3 rounded border border-white/10 bg-slate-950/70 px-3 py-2 shadow-xl backdrop-blur">
      <div class="text-xs font-semibold text-sky-100">{{ viewTitle }}</div>
      <div class="mt-0.5 text-[10px] text-slate-300">
        {{ renderedCountLabel }} · {{ viewDetailLabel }}
      </div>
    </div>

    <div class="absolute right-3 top-3 flex max-w-[calc(100%-18rem)] flex-wrap items-center justify-end gap-2">
      <select
        v-if="viewMode === 'galaxy'"
        v-model="galaxyShape"
        class="h-[26px] rounded border border-white/10 bg-slate-950/80 px-2 text-[11px] text-slate-100 outline-none hover:bg-slate-800"
        title="Galaxy layout"
      >
        <option value="spiral">Spiral</option>
        <option value="layers">Branch Layers</option>
        <option value="constellation">Constellation</option>
      </select>
      <select
        v-if="viewMode === 'galaxy'"
        v-model="galaxyMotionMode"
        class="h-[26px] rounded border border-white/10 bg-slate-950/80 px-2 text-[11px] text-slate-100 outline-none hover:bg-slate-800"
        title="Galaxy motion"
      >
        <option value="threeD">3D</option>
        <option value="balanced">Balanced</option>
        <option value="circling">Circling</option>
      </select>
      <button
        type="button"
        class="rounded border border-white/10 bg-slate-950/70 px-2.5 py-1 text-[11px] text-slate-100 hover:bg-slate-800"
        @click="resetView"
      >
        Reset
      </button>
      <button
        type="button"
        class="rounded border border-white/10 bg-slate-950/70 px-2.5 py-1 text-[11px] text-slate-100 hover:bg-slate-800"
        @click="focusCommit(null, true)"
      >
        Focus
      </button>
      <button
        type="button"
        class="rounded border border-emerald-300/30 bg-emerald-500/15 px-2.5 py-1 text-[11px] text-emerald-100 hover:bg-emerald-500/25"
        @click="toggleTreeView"
      >
        {{ viewMode === "tree" ? "Galaxy View" : "Tree View" }}
      </button>
      <div ref="viewOptionsRef" class="relative">
        <button
          type="button"
          class="h-[26px] rounded border border-white/10 bg-slate-950/80 px-2.5 text-[11px] text-slate-100 outline-none hover:bg-slate-800"
          @click="toggleViewOptionsMenu"
        >
          Options
        </button>
        <div
          v-if="showViewOptionsMenu"
          class="absolute right-0 top-8 z-[20] w-44 rounded-md border border-white/10 bg-slate-950/95 p-2 text-[11px] text-slate-100 shadow-2xl backdrop-blur"
        >
          <label class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-white/5">
            <input type="checkbox" class="h-3 w-3 accent-sky-400" :checked="showLabels" @change="toggleLabels" />
            <span>Labels</span>
          </label>
          <label class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-white/5">
            <input type="checkbox" class="h-3 w-3 accent-emerald-400" :checked="showGuides" @change="toggleGuides" />
            <span>Guides</span>
          </label>
          <label v-if="viewMode === 'galaxy'" class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-white/5">
            <input type="checkbox" class="h-3 w-3 accent-cyan-400" :checked="showTimeRings" @change="toggleTimeRings" />
            <span>Rings</span>
          </label>
          <label v-if="viewMode === 'galaxy'" class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-white/5">
            <input type="checkbox" class="h-3 w-3 accent-fuchsia-400" :checked="showBranchAura" @change="toggleBranchAura" />
            <span>Aura</span>
          </label>
          <label v-if="viewMode === 'galaxy'" class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-white/5">
            <input type="checkbox" class="h-3 w-3 accent-violet-400" :checked="showDepthFog" @change="toggleDepthFog" />
            <span>Depth</span>
          </label>
          <label v-if="viewMode === 'galaxy'" class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-white/5">
            <input type="checkbox" class="h-3 w-3 accent-teal-400" :checked="showMiniMap" @change="toggleMiniMap" />
            <span>Map</span>
          </label>
          <label class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-white/5">
            <input type="checkbox" class="h-3 w-3 accent-orange-400" :checked="highlightMerges" @change="toggleMerges" />
            <span>Merges</span>
          </label>
        </div>
      </div>
      <button
        v-if="hasMore"
        type="button"
        class="rounded border border-sky-300/30 bg-sky-500/15 px-2.5 py-1 text-[11px] text-sky-100 hover:bg-sky-500/25"
        @click="emit('loadMore')"
      >
        Load More
      </button>
      <button
        v-if="hasMore"
        type="button"
        class="rounded border border-fuchsia-300/30 bg-fuchsia-500/15 px-2.5 py-1 text-[11px] text-fuchsia-100 hover:bg-fuchsia-500/25"
        @click="emit('loadAll')"
      >
        Load All
      </button>
      <button
        type="button"
        class="rounded border border-white/10 bg-slate-950/70 px-2.5 py-1 text-[11px] text-slate-100 hover:bg-slate-800"
        @click="emit('close')"
      >
        Close
      </button>
    </div>

    <div
      v-if="hoveredNode"
      class="pointer-events-none absolute max-w-[320px] rounded border border-sky-300/20 bg-slate-950/90 px-3 py-2 text-[11px] text-slate-100 shadow-2xl backdrop-blur"
      :style="tooltipStyle"
    >
      <div class="flex items-center gap-2">
        <span class="h-2 w-2 rounded-full" :style="{ backgroundColor: hoveredNode.color }" />
        <span class="font-mono text-sky-200">{{ hoveredNode.commit.short_sha }}</span>
        <span v-if="hoveredNode.branchLabel" class="rounded bg-sky-400/10 px-1.5 py-0.5 text-[10px] text-sky-100">
          {{ hoveredNode.branchLabel }}
        </span>
      </div>
      <div class="mt-1 line-clamp-3 font-medium">{{ hoveredNode.commit.message }}</div>
      <div class="mt-1 text-slate-400">{{ hoveredNode.commit.author_name }} · {{ hoveredNode.commit.time_ago }}</div>
      <div class="mt-1 flex flex-wrap gap-1.5 text-[10px] text-slate-300">
        <span class="rounded bg-white/5 px-1.5 py-0.5">lane: {{ hoveredNode.laneName }}</span>
        <span class="rounded bg-white/5 px-1.5 py-0.5">{{ hoveredNode.commit.parent_shas.length || 0 }} parent{{ hoveredNode.commit.parent_shas.length === 1 ? "" : "s" }}</span>
        <span v-if="hoveredNode.commit.parent_shas.length > 1" class="rounded bg-orange-400/15 px-1.5 py-0.5 text-orange-100">merge</span>
      </div>
      <div v-if="hoveredNode.commit.refs.length" class="mt-1 truncate text-slate-400">
        refs: {{ hoveredNode.commit.refs.join(", ") }}
      </div>
    </div>
  </div>
</template>

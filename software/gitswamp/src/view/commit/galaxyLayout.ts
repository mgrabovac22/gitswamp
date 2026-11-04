import type { BranchInfo, CommitInfo } from "@/types";

export type GalaxyLayout = "spiral" | "layers" | "constellation" | "tree";
export type GalaxyMotion = "threeD" | "balanced" | "circling";
export type GalaxyPoint = [number, number, number];
export type GalaxyNode = {
  commit: CommitInfo;
  lane: number;
  color: string;
  label: string;
  progress: number;
  positions: Record<GalaxyLayout, GalaxyPoint>;
};
export type GalaxyScene = {
  nodes: GalaxyNode[];
  edges: [number, number][];
  lanes: { name: string; color: string }[];
};

export const MAX_GALAXY_COMMITS = 50000;
export const MAX_GALAXY_EDGES = 100000;
export const GALAXY_RADIUS = 440;
export const GALAXY_ARMS = 4;
const COLORS = ["#8ed9f5", "#f5bc85", "#b3a5ec", "#80dbb5", "#f09bb5", "#dfdb98"];

export function hashGalaxy(value: string): number {
  let hash = 2166136261;
  for (let i = 0; i < value.length; i++) hash = Math.imul(hash ^ value.charCodeAt(i), 16777619);
  return hash >>> 0;
}

export function normalizeGalaxyBranch(value: string): string {
  return value.trim().replace(/^HEAD ->\s*/, "").replace(/^refs\/heads\//, "")
    .replace(/^refs\/remotes\//, "").replace(/^remotes\//, "").replace(/^origin\//, "");
}

function priority(name: string): number {
  const rank = ["main", "master", "trunk", "develop", "dev"].indexOf(name.toLowerCase());
  return rank < 0 ? 10 : rank;
}

export function spiralPoint(progress: number, lane: number, height = 0): GalaxyPoint {
  const radius = 36 + progress * (GALAXY_RADIUS - 36);
  const angle = lane % GALAXY_ARMS * Math.PI * 2 / GALAXY_ARMS
    + Math.floor(lane / GALAXY_ARMS) * 0.065 + 4.6 * progress - 0.8;
  return [Math.cos(angle) * radius, height, Math.sin(angle) * radius];
}

// Iterative parent-first traversal tolerates clock skew, shallow history and even
// malformed cycles, without recursive stacks or one ancestry copy per branch.
export function orderGalaxyCommits(input: CommitInfo[]): CommitInfo[] {
  const unique = new Map(input.slice(0, MAX_GALAXY_COMMITS).map(commit => [commit.sha, commit]));
  const commits = [...unique.values()].sort((a, b) => a.timestamp - b.timestamp || a.sha.localeCompare(b.sha));
  const state = new Map<string, number>();
  const result: CommitInfo[] = [];
  for (const commit of commits) {
    if (state.has(commit.sha)) continue;
    const stack: { commit: CommitInfo; parent: number }[] = [{ commit, parent: 0 }];
    state.set(commit.sha, 1);
    while (stack.length) {
      const item = stack[stack.length - 1];
      const parents = item.commit.parent_shas;
      if (item.parent < Math.min(parents.length, 16)) {
        const parent = unique.get(parents[item.parent++]);
        if (parent && !state.has(parent.sha)) {
          state.set(parent.sha, 1);
          stack.push({ commit: parent, parent: 0 });
        }
      } else {
        result.push(item.commit);
        state.set(item.commit.sha, 2);
        stack.pop();
      }
    }
  }
  return result;
}

export function buildGalaxyScene(input: CommitInfo[], branches: BranchInfo[], currentBranch: string): GalaxyScene {
  const commits = orderGalaxyCommits(input);
  const bySha = new Map(commits.map((commit, i) => [commit.sha, i]));
  const tips = new Map<string, number>();
  commits.forEach((commit, i) => {
    for (const ref of commit.refs) {
      for (const part of ref.split(",")) tips.set(normalizeGalaxyBranch(part), i);
    }
  });
  const current = normalizeGalaxyBranch(currentBranch);
  const names = [...new Set(branches.map(branch => normalizeGalaxyBranch(branch.name)))]
    .filter(name => name && tips.has(name))
    .sort((a, b) => priority(a) - priority(b) || Number(b === current) - Number(a === current) || a.localeCompare(b));
  const lanes: GalaxyScene["lanes"] = [];
  const laneByIndex = new Int32Array(commits.length).fill(-1);
  const labels = new Map<number, string[]>();
  const claim = (tip: number, name: string) => {
    if (laneByIndex[tip] >= 0) return;
    const lane = lanes.length;
    lanes.push({ name, color: COLORS[lane % COLORS.length] });
    let index: number | undefined = tip;
    while (index !== undefined && laneByIndex[index] < 0) {
      laneByIndex[index] = lane;
      index = bySha.get(commits[index].parent_shas[0]);
    }
  };
  for (const name of names) {
    const tip = tips.get(name)!;
    const existing = labels.get(tip) || [];
    if (existing.length < 3) existing.push(name);
    labels.set(tip, existing);
    claim(tip, name);
  }
  if (!lanes.length && commits.length) claim(commits.length - 1, current || "history");
  // Retain merged/deleted branch ancestry instead of scattering it into history.
  for (let i = commits.length - 1; i >= 0; i--) {
    for (const sha of commits[i].parent_shas.slice(1, 16)) {
      const parent = bySha.get(sha);
      if (parent !== undefined) claim(parent, `merged ${commits[parent].short_sha}`);
    }
  }
  for (let i = commits.length - 1; i >= 0; i--) claim(i, `history ${commits[i].short_sha}`);

  const laneBase = new Float64Array(lanes.length).fill(1);
  const laneTip = new Float64Array(lanes.length);
  commits.forEach((_, i) => {
    const progress = i / Math.max(1, commits.length - 1);
    laneBase[laneByIndex[i]] = Math.min(laneBase[laneByIndex[i]], progress);
    laneTip[laneByIndex[i]] = progress;
  });
  const nodes: GalaxyNode[] = commits.map((commit, i) => {
    const progress = commits.length === 1 ? 0.5 : i / (commits.length - 1);
    const lane = laneByIndex[i];
    const seed = hashGalaxy(commit.sha);
    const height = ((seed % 1000) / 1000 - 0.5) * (10 - progress * 6);
    const offset = lane === 0 ? 0 : Math.ceil(lane / 2) * (lane % 2 ? -1 : 1);
    const branchProgress = (progress - laneBase[lane]) / Math.max(0.01, laneTip[lane] - laneBase[lane]);
    const reach = offset * 65 * (0.18 + Math.max(0, branchProgress) * 0.82);
    const label = labels.get(i)?.join(", ") || commit.refs.filter(ref => ref.startsWith("tag:")).slice(0, 2).join(", ");
    return {
      commit, lane, color: lanes[lane].color, label, progress,
      positions: {
        spiral: spiralPoint(progress, lane, height),
        layers: [offset * 52, (progress - 0.5) * 760, height],
        constellation: [offset * 76 + Math.sin(progress * 5) * 30, (progress - 0.5) * 760,
          Math.cos(progress * 5 + lane) * 38],
        tree: [reach, (progress - 0.5) * 760, height * 0.4],
      },
    };
  });
  const edges: GalaxyScene["edges"] = [];
  for (let i = 0; i < commits.length && edges.length < MAX_GALAXY_EDGES; i++) {
    for (const sha of commits[i].parent_shas.slice(0, 16)) {
      const parent = bySha.get(sha);
      if (parent !== undefined && parent !== i && edges.length < MAX_GALAXY_EDGES) edges.push([parent, i]);
    }
  }
  return { nodes, edges, lanes };
}

export type OrbitState = {
  motion: GalaxyMotion; layout: GalaxyLayout; hidden: boolean;
  reducedMotion: boolean; interacting: boolean; hovering: boolean; resumeAt: number;
};

export function canOrbit(state: OrbitState, now: number): boolean {
  return state.motion === "circling" && state.layout !== "tree" && !state.hidden
    && !state.reducedMotion && !state.interacting && !state.hovering && now >= state.resumeAt;
}

export function orbitDelta(previous: number | null, now: number): number {
  return previous === null ? 0 : Math.max(0, Math.min(0.05, (now - previous) / 1000));
}

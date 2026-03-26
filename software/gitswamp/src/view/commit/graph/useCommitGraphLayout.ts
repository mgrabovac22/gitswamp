import { computed, type Ref } from "vue";

import type { CommitInfo, GraphEdge, GraphNode, StashInfo, TagInfo } from "@/types";

import { CORNER_R, GRAPH_COLORS, LANE_WIDTH, OVERSCAN } from "./graph.constants";

export interface MergedRef {
  name: string;
  local: boolean;
  remote: boolean;
}

export interface DisplayRef {
  kind: "branch" | "tag";
  key: string;
  name: string;
  local?: boolean;
  remote?: boolean;
}

type CommitGraphProps = {
  commits: CommitInfo[];
  currentBranch: string;
  hasWorkingChanges: boolean;
  hasConflicts?: boolean;
  stashes?: StashInfo[];
  tags?: TagInfo[];
};

function rangesOverlap(a: { min: number; max: number }, b: { min: number; max: number }): boolean {
  return a.min <= b.max && b.min <= a.max;
}

function laneX(lane: number): number {
  return lane * LANE_WIDTH + LANE_WIDTH / 2 + 4;
}

function refsWithoutHead(commit: CommitInfo): string[] {
  return commit.refs.filter(r => !r.includes("HEAD") && !r.includes("->"));
}

export function useCommitGraphLayout(
  props: CommitGraphProps,
  rowHeight: Ref<number>,
  scrollTop: Ref<number>,
  viewportHeight: Ref<number>,
) {
  const graph = computed(() => {
    const all = props.commits;
    if (!all.length) {
      return {
        nodes: [] as GraphNode[],
        edges: [] as GraphEdge[],
        laneCount: 0,
        branchLanes: new Map<string, number>(),
      };
    }

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
          } else {
            break;
          }
        } else {
          break;
        }
      }
    }

    all.forEach((_, idx) => {
      if (!commitBranch.has(idx)) commitBranch.set(idx, "__default");
    });

    const branchRange = new Map<string, { min: number; max: number }>();
    all.forEach((_, idx) => {
      const b = commitBranch.get(idx) ?? "__default";
      const cur = branchRange.get(b);
      if (cur) {
        cur.min = Math.min(cur.min, idx);
        cur.max = Math.max(cur.max, idx);
      } else {
        branchRange.set(b, { min: idx, max: idx });
      }
    });

    const branchOrder: string[] = [];
    const addBr = (n: string) => {
      if (!branchOrder.includes(n) && branchRange.has(n)) branchOrder.push(n);
    };

    if (props.currentBranch) addBr(props.currentBranch);
    for (const n of ["main", "master"]) addBr(n);
    all.forEach((_, idx) => {
      const b = commitBranch.get(idx);
      if (b) addBr(b);
    });

    const laneOccupied: Array<Array<{ min: number; max: number }>> = [];
    const laneMap = new Map<string, number>();

    for (const branch of branchOrder) {
      const range = branchRange.get(branch);
      if (!range) continue;
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
      return { commit, lane, color: GRAPH_COLORS[lane % GRAPH_COLORS.length] };
    });

    const edges: GraphEdge[] = [];
    all.forEach((commit, idx) => {
      for (const parentSha of commit.parent_shas) {
        const pi = shaIdx.get(parentSha);
        if (pi !== undefined) {
          edges.push({
            fromIndex: idx,
            toIndex: pi,
            fromLane: nodes[idx].lane,
            toLane: nodes[pi].lane,
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

    return props.stashes
      .map(s => {
        const parentIdx = shaIdx.get(s.parent_sha) ?? -1;
        if (parentIdx < 0) return null;
        const parentLane = graph.value.nodes[parentIdx]?.lane ?? 0;

        const offsetIdx = parentCount.get(parentIdx) ?? 0;
        parentCount.set(parentIdx, offsetIdx + 1);

        const stashLane = parentLane + 1 + offsetIdx;
        return { ...s, parentIdx, lane: stashLane, offsetIdx };
      })
      .filter(Boolean) as (StashInfo & { parentIdx: number; lane: number; offsetIdx: number })[];
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
    const maxStashLane = stashNodes.value.length > 0 ? Math.max(...stashNodes.value.map(s => s.lane)) : 0;
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
      Math.ceil((startY + viewportHeight.value) / rowHeight.value) + OVERSCAN,
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

  function ry(index: number) {
    return index * rowHeight.value + rowHeight.value / 2 + wcOffset.value;
  }

  function ep(e: GraphEdge): string {
    const x1 = laneX(e.fromLane);
    const y1 = ry(e.fromIndex);
    const x2 = laneX(e.toLane);
    const y2 = ry(e.toIndex);
    if (e.fromLane === e.toLane) return "M " + x1 + " " + y1 + " L " + x2 + " " + y2;
    const r = Math.min(CORNER_R, Math.abs(x2 - x1) / 2, Math.abs(y2 - y1) / 4);
    const d = x2 > x1 ? 1 : -1;
    const turnY = y1 + rowHeight.value * 0.5;
    return "M " + x1 + " " + y1
      + " L " + x1 + " " + (turnY - r)
      + " Q " + x1 + " " + turnY + " " + (x1 + d * r) + " " + turnY
      + " L " + (x2 - d * r) + " " + turnY
      + " Q " + x2 + " " + turnY + " " + x2 + " " + (turnY + r)
      + " L " + x2 + " " + y2;
  }

  function wcEdge(): string {
    const x = laneX(wcLane.value);
    const startY = rowHeight.value / 2 + 7;
    const endY = ry(headCommitIndex.value) - 10;
    return "M " + x + " " + startY + " L " + x + " " + endY;
  }

  function conflictEdge(): string {
    const conflictX = laneX(wcLane.value) + 22;
    const conflictY = (props.hasWorkingChanges ? 1 : 0) * rowHeight.value + rowHeight.value / 2;
    if (props.hasWorkingChanges) {
      const workingX = laneX(wcLane.value);
      const workingY = rowHeight.value / 2 + 7;
      const endX = conflictX - 7;
      const endY = conflictY;
      const turnY = workingY + Math.max(8, rowHeight.value * 0.28);
      const r = Math.min(CORNER_R, Math.abs(endX - workingX) / 2, Math.abs(endY - turnY) / 2);
      return "M " + workingX + " " + workingY
        + " L " + workingX + " " + (turnY - r)
        + " Q " + workingX + " " + turnY + " " + (workingX + r) + " " + turnY
        + " L " + (endX - r) + " " + turnY
        + " Q " + endX + " " + turnY + " " + endX + " " + (turnY + r)
        + " L " + endX + " " + endY;
    }
    const x2 = laneX(wcLane.value) - 7;
    const y2 = ry(headCommitIndex.value) - 10;
    const r = Math.min(CORNER_R, Math.abs(conflictX - x2) / 2, Math.abs(conflictY - y2) / 4);
    const turnY = conflictY - Math.max(10, rowHeight.value * 0.35);
    return "M " + conflictX + " " + conflictY
      + " L " + conflictX + " " + (turnY + r)
      + " Q " + conflictX + " " + turnY + " " + (conflictX - r) + " " + turnY
      + " L " + (x2 + r) + " " + turnY
      + " Q " + x2 + " " + turnY + " " + x2 + " " + (turnY - r)
      + " L " + x2 + " " + y2;
  }

  function conflictSpineEdge(): string {
    if (!props.hasConflicts || !props.hasWorkingChanges) return "";
    const x = laneX(wcLane.value);
    const startY = rowHeight.value / 2 + 7;
    const endY = rowHeight.value + rowHeight.value / 2 - 7;
    return "M " + x + " " + startY + " L " + x + " " + endY;
  }

  function wcLaneX(): number {
    return laneX(wcLane.value);
  }

  const tagNameSet = computed(() => new Set((props.tags ?? []).map(t => t.name)));

  function commitTags(commit: CommitInfo): TagInfo[] {
    const tags = props.tags ?? [];
    return tags.filter(t => t.sha === commit.sha);
  }

  function branchRefs(commit: CommitInfo): string[] {
    const tags = tagNameSet.value;
    return refsWithoutHead(commit).filter(r => !tags.has(r));
  }

  function mergedRefs(commit: CommitInfo): MergedRef[] {
    const raw = branchRefs(commit);
    const map = new Map<string, MergedRef>();
    for (const r of raw) {
      if (r.startsWith("origin/")) {
        const name = r.substring(7);
        const existing = map.get(name);
        if (existing) {
          existing.remote = true;
        } else {
          map.set(name, { name, local: false, remote: true });
        }
      } else {
        const existing = map.get(r);
        if (existing) {
          existing.local = true;
        } else {
          map.set(r, { name: r, local: true, remote: false });
        }
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

  return {
    graph,
    stashNodes,
    headCommitIndex,
    graphWidth,
    wcOffset,
    totalH,
    visibleNodes,
    visibleEdges,
    stashesAtCommit,
    lx: laneX,
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
  };
}

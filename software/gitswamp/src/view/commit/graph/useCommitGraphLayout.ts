import { computed, type Ref } from "vue";

import type { CommitInfo, GraphEdge, GraphNode, StashInfo, TagInfo } from "@/types";

import { CORNER_R, GRAPH_COLORS, LANE_WIDTH, NODE_RADIUS, OVERSCAN } from "./graph.constants";

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

function nextAvailableLane(used: Set<number>): number {
  let lane = 0;
  while (used.has(lane)) {
    lane += 1;
  }
  return lane;
}

function normalizedRefName(refName: string): string | null {
  const value = refName.trim();
  if (!value || value === "HEAD" || value.includes("->")) {
    return null;
  }

  return value.replace(/^origin\//, "");
}

function preferredLaneFromRefs(
  commit: CommitInfo,
  branchLaneMap: Map<string, number>,
  activeLanes: Set<number>,
): number | undefined {
  for (const ref of commit.refs) {
    const normalized = normalizedRefName(ref);
    if (!normalized) continue;
    const reservedLane = branchLaneMap.get(normalized);
    if (reservedLane !== undefined && !activeLanes.has(reservedLane)) {
      return reservedLane;
    }
  }

  return undefined;
}

function registerCommitRefs(commit: CommitInfo, lane: number, branchLaneMap: Map<string, number>) {
  for (const ref of commit.refs) {
    const normalized = normalizedRefName(ref);
    if (!normalized) continue;
    if (!branchLaneMap.has(normalized)) {
      branchLaneMap.set(normalized, lane);
    }
  }
}

function reserveParentLaneHints(commit: CommitInfo, lane: number, laneHintsBySha: Map<string, number>): number {
  let maxAssignedLane = lane;
  const firstParentSha = commit.parent_shas[0];
  if (firstParentSha && !laneHintsBySha.has(firstParentSha)) {
    laneHintsBySha.set(firstParentSha, lane);
  }

  for (const parentSha of commit.parent_shas.slice(1)) {
    if (!parentSha || laneHintsBySha.has(parentSha)) continue;

    const usedForMerge = new Set(laneHintsBySha.values());
    usedForMerge.add(lane);
    const mergeLane = nextAvailableLane(usedForMerge);
    laneHintsBySha.set(parentSha, mergeLane);
    maxAssignedLane = Math.max(maxAssignedLane, mergeLane);
  }

  return maxAssignedLane;
}

function buildGraphModel(commits: CommitInfo[], currentBranch: string) {
  if (!commits.length) {
    return {
      nodes: [] as GraphNode[],
      edges: [] as GraphEdge[],
      laneCount: 0,
      branchLanes: new Map<string, number>(),
    };
  }

  const shaIdx = new Map<string, number>();
  commits.forEach((commit, idx) => shaIdx.set(commit.sha, idx));

  const laneHintsBySha = new Map<string, number>();
  const branchLaneMap = new Map<string, number>();
  if (currentBranch.trim()) {
    branchLaneMap.set(currentBranch.trim(), 0);
  }

  const nodes: GraphNode[] = [];
  let maxLane = 0;

  for (const commit of commits) {
    const hintedLane = laneHintsBySha.get(commit.sha);
    if (hintedLane !== undefined) {
      laneHintsBySha.delete(commit.sha);
    }

    const activeLanes = new Set(laneHintsBySha.values());
    let lane = hintedLane;
    lane ??= preferredLaneFromRefs(commit, branchLaneMap, activeLanes);
    lane ??= nextAvailableLane(activeLanes);
    if (activeLanes.has(lane)) {
      lane = nextAvailableLane(activeLanes);
    }

    maxLane = Math.max(maxLane, lane);
    nodes.push({
      commit,
      lane,
      color: GRAPH_COLORS[lane % GRAPH_COLORS.length],
    });

    registerCommitRefs(commit, lane, branchLaneMap);
    maxLane = Math.max(maxLane, reserveParentLaneHints(commit, lane, laneHintsBySha));
  }

  const edges: GraphEdge[] = [];
  commits.forEach((commit, idx) => {
    const fromNode = nodes[idx];
    for (const parentSha of commit.parent_shas) {
      const parentIdx = shaIdx.get(parentSha);
      if (parentIdx === undefined) continue;

      edges.push({
        fromIndex: idx,
        toIndex: parentIdx,
        fromLane: fromNode.lane,
        toLane: nodes[parentIdx].lane,
        color: fromNode.color,
      });
    }
  });

  return {
    nodes,
    edges,
    laneCount: Math.max(1, maxLane + 1),
    branchLanes: branchLaneMap,
  };
}

function laneX(lane: number): number {
  return lane * LANE_WIDTH + LANE_WIDTH / 2 + 4;
}

function nodeConnectionGap(commit: CommitInfo): number {
  // Merge edges should connect to the center of the merge node.
  if (commit.parent_shas.length > 1) {
    return 0;
  }

  return NODE_RADIUS + 2;
}

function nodeShieldRadius(commit: CommitInfo): number {
  if (commit.parent_shas.length > 1) {
    return 8;
  }

  return NODE_RADIUS + 2;
}

function buildQuickVerticalPath(x: number, startY: number, endY: number, includeMove: boolean): string | null {
  if (Math.abs(endY - startY) < 0.1) {
    return `${includeMove ? "M" : "L"} ${x} ${startY}`;
  }

  if (endY < startY) {
    return `${includeMove ? "M" : "L"} ${x} ${startY} L ${x} ${endY}`;
  }

  return null;
}

function refsWithoutHead(commit: CommitInfo): string[] {
  return commit.refs.filter(r => !r.includes("HEAD") && !r.includes("->"));
}

function hasDetachedHeadRef(commit: CommitInfo, currentBranch: string): boolean {
  if (currentBranch !== "HEAD") return false;
  return commit.refs.some((ref) => ref.trim() === "HEAD");
}

export function useCommitGraphLayout(
  props: CommitGraphProps,
  rowHeight: Ref<number>,
  scrollTop: Ref<number>,
  viewportHeight: Ref<number>,
) {
  const graph = computed(() => buildGraphModel(props.commits, props.currentBranch));

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

  function firstBlockingNodeBlock(
    x: number,
    startY: number,
    endY: number,
    excludedIndexes: Set<number>,
  ): { start: number; end: number } | null {
    const minY = Math.min(startY, endY);
    const maxY = Math.max(startY, endY);

    const blocks: Array<{ start: number; end: number }> = [];
    for (let idx = 0; idx < graph.value.nodes.length; idx += 1) {
      if (excludedIndexes.has(idx)) continue;

      const node = graph.value.nodes[idx];
      const centerX = laneX(node.lane);
      const radius = nodeShieldRadius(node.commit);
      if (Math.abs(centerX - x) >= radius) {
        continue;
      }

      const centerY = ry(idx);
      const start = centerY - radius;
      const end = centerY + radius;
      if (end <= minY || start >= maxY) {
        continue;
      }

      blocks.push({
        start: Math.max(start, minY),
        end: Math.min(end, maxY),
      });
    }

    if (blocks.length === 0) {
      return null;
    }

    blocks.sort((a, b) => a.start - b.start);

    const first = { ...blocks[0] };
    for (const block of blocks.slice(1)) {
      if (block.start <= first.end) {
        first.end = Math.max(first.end, block.end);
      } else {
        break;
      }
    }

    return first;
  }

  function blockingNodeBlocksAtX(
    x: number,
    startY: number,
    endY: number,
    excludedIndexes: Set<number>,
  ): Array<{ start: number; end: number }> {
    const minY = Math.min(startY, endY);
    const maxY = Math.max(startY, endY);
    const blocks: Array<{ start: number; end: number }> = [];

    for (let idx = 0; idx < graph.value.nodes.length; idx += 1) {
      if (excludedIndexes.has(idx)) continue;

      const node = graph.value.nodes[idx];
      const centerX = laneX(node.lane);
      const radius = nodeShieldRadius(node.commit);
      if (Math.abs(centerX - x) >= radius) {
        continue;
      }

      const centerY = ry(idx);
      const start = centerY - radius;
      const end = centerY + radius;
      if (end <= minY || start >= maxY) {
        continue;
      }

      blocks.push({
        start: Math.max(minY, start),
        end: Math.min(maxY, end),
      });
    }

    blocks.sort((a, b) => a.start - b.start);
    return blocks;
  }

  function hasConsecutiveBlockingNodes(blocks: Array<{ start: number; end: number }>): boolean {
    if (blocks.length < 2) {
      return false;
    }

    const adjacencyGap = Math.max(2, rowHeight.value * 0.4);
    for (let idx = 1; idx < blocks.length; idx += 1) {
      if (blocks[idx].start - blocks[idx - 1].end <= adjacencyGap) {
        return true;
      }
    }

    return false;
  }

  function segmentIntersectsAnyForeignNode(
    x: number,
    startY: number,
    endY: number,
    excludedIndexes: Set<number>,
  ): boolean {
    const minY = Math.min(startY, endY);
    const maxY = Math.max(startY, endY);

    for (let idx = 0; idx < graph.value.nodes.length; idx += 1) {
      if (excludedIndexes.has(idx)) continue;

      const node = graph.value.nodes[idx];
      const centerX = laneX(node.lane);
      const centerY = ry(idx);
      const radius = nodeShieldRadius(node.commit);

      if (centerY + radius < minY || centerY - radius > maxY) {
        continue;
      }

      if (Math.abs(centerX - x) < radius) {
        return true;
      }
    }

    return false;
  }

  function findRightClearX(
    fromX: number,
    startY: number,
    endY: number,
    excludedIndexes: Set<number>,
  ): number {
    const unit = Math.max(6, Math.floor(LANE_WIDTH * 0.4));
    let candidate = fromX + unit;

    for (let attempt = 0; attempt < 40; attempt += 1) {
      if (!segmentIntersectsAnyForeignNode(candidate, startY, endY, excludedIndexes)) {
        return candidate;
      }
      candidate += unit;
    }

    return fromX + unit;
  }

  function routeStairDetours(
    path: string,
    startX: number,
    startY: number,
    endY: number,
    clearance: number,
    excludedIndexes: Set<number>,
  ): { path: string; cursorX: number; cursorY: number } {
    let nextPath = path;
    let cursorX = startX;
    let cursorY = startY;
    let guard = 0;

    while (cursorY < endY && guard < 50) {
      guard += 1;
      const block = firstBlockingNodeBlock(cursorX, cursorY, endY, excludedIndexes);
      if (!block) {
        nextPath += ` L ${cursorX} ${endY}`;
        cursorY = endY;
        break;
      }

      const approachY = Math.max(cursorY, block.start - clearance);
      if (approachY > cursorY) {
        nextPath += ` L ${cursorX} ${approachY}`;
      }

      const leaveY = Math.min(endY, block.end + clearance);
      const detourX = findRightClearX(cursorX, approachY, leaveY, excludedIndexes);
      nextPath += ` L ${detourX} ${approachY}`;
      nextPath += ` L ${detourX} ${leaveY}`;
      cursorX = detourX;
      cursorY = leaveY;
    }

    return { path: nextPath, cursorX, cursorY };
  }

  function settleStairPath(
    path: string,
    cursorX: number,
    cursorY: number,
    startX: number,
    startY: number,
    endY: number,
  ): string {
    let nextPath = path;
    if (cursorY < endY) {
      nextPath += ` L ${cursorX} ${endY}`;
    }

    if (Math.abs(cursorX - startX) <= 0.1) {
      return nextPath;
    }

    const settleY = Math.max(startY, endY - Math.max(8, rowHeight.value * 0.24));
    if (cursorY < settleY) {
      nextPath += ` L ${cursorX} ${settleY}`;
    }
    nextPath += ` L ${startX} ${settleY}`;
    nextPath += ` L ${startX} ${endY}`;
    return nextPath;
  }

  function stairStepVerticalPath(
    x: number,
    startY: number,
    endY: number,
    excludedIndexes: Set<number>,
    includeMove = true,
  ): string {
    const quickPath = buildQuickVerticalPath(x, startY, endY, includeMove);
    if (quickPath) return quickPath;

    const initialBlocks = blockingNodeBlocksAtX(x, startY, endY, excludedIndexes);
    if (!hasConsecutiveBlockingNodes(initialBlocks)) {
      return `${includeMove ? "M" : "L"} ${x} ${startY} L ${x} ${endY}`;
    }

    const clearance = Math.max(2, Math.floor(NODE_RADIUS * 0.25));
    const startCommand = `${includeMove ? "M" : "L"} ${x} ${startY}`;
    const routed = routeStairDetours(startCommand, x, startY, endY, clearance, excludedIndexes);
    return settleStairPath(routed.path, routed.cursorX, routed.cursorY, x, startY, endY);
  }

  function ep(e: GraphEdge): string {
    const x1 = laneX(e.fromLane);
    const y1 = ry(e.fromIndex);
    const x2 = laneX(e.toLane);
    const y2 = ry(e.toIndex);

    const fromCommit = props.commits[e.fromIndex];
    const toCommit = props.commits[e.toIndex];
    const sy = y1 + nodeConnectionGap(fromCommit);
    const ey = y2 - nodeConnectionGap(toCommit);
    const excludedIndexes = new Set<number>([e.fromIndex, e.toIndex]);

    if (e.fromLane === e.toLane) {
      return stairStepVerticalPath(
        x1,
        sy,
        ey,
        excludedIndexes,
      );
    }

    const d: -1 | 1 = x2 > x1 ? 1 : -1;
    const adx = Math.abs(x2 - x1);

    if (ey <= sy) {
      const bridgeY = (sy + ey) / 2;
      const up = Math.max(0, sy - bridgeY);
      const down = Math.max(0, bridgeY - ey);
      const r = Math.max(1, Math.min(CORNER_R, adx / 2, up, down));

      return `M ${x1} ${sy}`
        + ` L ${x1} ${bridgeY + r}`
        + ` Q ${x1} ${bridgeY} ${x1 + d * r} ${bridgeY}`
        + ` L ${x2 - d * r} ${bridgeY}`
        + ` Q ${x2} ${bridgeY} ${x2} ${bridgeY - r}`
        + ` L ${x2} ${ey}`;
    }

    const dy = ey - sy;

    if (dy < rowHeight.value) {
      const leadY = Math.max(2, Math.min(rowHeight.value * 0.2, dy * 0.4));
      const available = Math.max(1, dy - leadY);
      const r = Math.max(1, Math.min(CORNER_R, adx / 2, available / 2));
      const turnY = sy + leadY + r;

      return `M ${x1} ${sy}`
        + ` L ${x1} ${turnY - r}`
        + ` Q ${x1} ${turnY} ${x1 + d * r} ${turnY}`
        + ` L ${x2 - d * r} ${turnY}`
        + ` Q ${x2} ${turnY} ${x2} ${turnY + r}`
        + ` L ${x2} ${ey}`;
    }

    const minBottomGap = Math.max(8, rowHeight.value * 0.34);
    const preferredTurnY = sy + Math.max(10, rowHeight.value * 0.56);
    const maxTurnY = ey - minBottomGap;
    const turnY = Math.min(preferredTurnY, maxTurnY);

    // Use one consistent, symmetric corner radius so all lane changes
    // have the same horizontal<->vertical transition feel.
    const targetRadius = CORNER_R + 2;
    const availableUp = Math.max(1, turnY - sy);
    const availableDown = Math.max(1, ey - turnY);
    const r = Math.min(targetRadius, adx / 2, availableUp, availableDown);

    const firstVertical = stairStepVerticalPath(
      x1,
      sy,
      turnY - r,
      excludedIndexes,
    );

    const secondVertical = stairStepVerticalPath(
      x2,
      turnY + r,
      ey,
      excludedIndexes,
      false,
    );

    return firstVertical
      + ` Q ${x1} ${turnY} ${x1 + d * r} ${turnY}`
      + ` L ${x2 - d * r} ${turnY}`
      + ` Q ${x2} ${turnY} ${x2} ${turnY + r}`
      + secondVertical;
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
    const refs = refsWithoutHead(commit).filter(r => !tags.has(r));
    if (hasDetachedHeadRef(commit, props.currentBranch)) {
      return ["HEAD", ...refs];
    }
    return refs;
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

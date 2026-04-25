<script setup lang="ts">
import { ref, shallowRef, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { X, FileText, Pencil, ChevronUp, ChevronDown, Undo2, Eye, Edit3, Save, RotateCcw, Play, Pause, StepBack, StepForward, Share2, Users, Columns2 } from "lucide-vue-next";
import type { FileDiff, DiffLine, CommitInfo, CommitFileInfo, FileBlameLine } from "@/types";
import { highlightCodeLine, splitFilePath } from "@/shared/codeView";
import {
  getCachedDiffText,
  getCachedStructuredDiff,
  pruneDiffViewerCaches,
  setCachedDiffText,
  setCachedStructuredDiff,
} from "@/shared/config/diffViewCache";
import { useToast } from "@/shared/notifications/useToast";
import { useGit } from "@/domain/git/UseGit";
import logoCrocGif from "@/assets/logo_croc.gif";
import logoCrocLoadingGif from "@/assets/logo_croc_loading.gif";

const props = defineProps<{
  repoPath: string;
  filePath: string;
  commitSha?: string | null;
  staged?: boolean;
}>();

const emit = defineEmits<{
  close: [];
  refresh: [];
}>();

type ViewMode = "diff" | "split-diff" | "file-diff" | "edit" | "time-lapse";
const viewMode = ref<ViewMode>("diff");
const loadingLetters = ["L", "o", "a", "d", "i", "n", "g"];
const MAX_HIGHLIGHT_LINES = 6000;
const MAX_HIGHLIGHT_CHARS = 450000;

const diff = shallowRef<FileDiff | null>(null);
const fileContent = ref<string>("");
const editContent = ref<string>("");
const loading = ref(true);
const loadingFileContent = ref(false);
const error = ref<string | null>(null);
const showBlamePanel = ref(false);
const blameLoading = ref(false);
const blameError = ref<string | null>(null);
const blameLines = ref<FileBlameLine[]>([]);
const blameCache = ref(new Map<string, FileBlameLine[]>());
const splitPaneRefs = ref(new Map<string, HTMLElement>());
const currentHunkIndex = ref(0);
const saving = ref(false);
const hasUnsavedChanges = ref(false);
const highlightedLineCache = ref(new Map<string, string>());
const toast = useToast();
const git = useGit();

interface TimeLapseFrame {
  sha: string;
  shortSha: string;
  author: string;
  message: string;
  timestamp: number;
  content: string;
  diffLines: DiffLine[];
}

const timeLapseFrames = ref<TimeLapseFrame[]>([]);
const timeLapseLoading = ref(false);
const timeLapsePlaying = ref(false);
const timeLapseFrameIndex = ref(0);
let timeLapseTimer: ReturnType<typeof setInterval> | null = null;
const TIME_LAPSE_MAX_FRAMES = 22;
const DEFAULT_TIME_LAPSE_COMMIT_WINDOW = 100;
const MIN_TIME_LAPSE_COMMIT_WINDOW = 20;
const MAX_TIME_LAPSE_COMMIT_WINDOW = 2000;
const timeLapseCommitWindowOptions = [50, 75, 100, 150, 200, 300, 500, 800, 1000] as const;
const timeLapseCommitWindow = ref(DEFAULT_TIME_LAPSE_COMMIT_WINDOW);

const timeLapseCommitWindowSafe = computed(() => {
  const raw = Number(timeLapseCommitWindow.value);
  if (!Number.isFinite(raw)) {
    return DEFAULT_TIME_LAPSE_COMMIT_WINDOW;
  }

  const rounded = Math.round(raw);
  return Math.min(MAX_TIME_LAPSE_COMMIT_WINDOW, Math.max(MIN_TIME_LAPSE_COMMIT_WINDOW, rounded));
});

// Virtualization state
const scrollTop = ref(0);
const containerHeight = ref(400);
const LINE_HEIGHT = 18; // pixels per line
const OVERSCAN = 10;
const fileDiffScrollContainer = ref<HTMLElement | null>(null);
const FILE_DIFF_OVERVIEW_MIN_VISIBLE_RATIO = 1.08;
const FILE_DIFF_OVERVIEW_MIN_MARKER_PERCENT = 0.75;
const FILE_DIFF_OVERVIEW_MIN_VIEWPORT_PERCENT = 10;
const DEFAULT_MAX_HIGHLIGHT_CACHE_SIZE = 2000;
const MEMORY_SAVER_MAX_HIGHLIGHT_CACHE_SIZE = 900;
const DEFAULT_MAX_BLAME_CACHE_ENTRIES = 36;
const MEMORY_SAVER_MAX_BLAME_CACHE_ENTRIES = 14;

let fileWatchInterval: ReturnType<typeof setInterval> | null = null;
let lastFileHash = "";
let fileWatchRequestInFlight = false;
let fileContentLoadSequence = 0;
let cacheCleanupInterval: ReturnType<typeof setInterval> | null = null;

function isMemorySaverModeEnabled(): boolean {
  return localStorage.getItem("gitswamp-memory-saver-mode") === "true";
}

function currentHighlightCacheLimit(): number {
  return isMemorySaverModeEnabled()
    ? MEMORY_SAVER_MAX_HIGHLIGHT_CACHE_SIZE
    : DEFAULT_MAX_HIGHLIGHT_CACHE_SIZE;
}

function currentBlameCacheLimit(): number {
  return isMemorySaverModeEnabled()
    ? MEMORY_SAVER_MAX_BLAME_CACHE_ENTRIES
    : DEFAULT_MAX_BLAME_CACHE_ENTRIES;
}

function isBlameByDefaultEnabled(): boolean {
  return localStorage.getItem("gitswamp-prefer-blame-by-default") === "true";
}

function getFileContentCacheKey(): string | null {
  if (props.commitSha) {
    return `${props.repoPath}|${props.filePath}|commit:${props.commitSha}`;
  }

  if (props.staged) {
    return `${props.repoPath}|${props.filePath}|staged`;
  }

  return null;
}

function getDiffCacheKey(): string | null {
  return getFileContentCacheKey();
}

function getBlameCacheKey(): string {
  if (props.commitSha) {
    return `${props.repoPath}|${props.filePath}|blame:commit:${props.commitSha}`;
  }

  const mode = props.staged ? "staged" : "working";
  return `${props.repoPath}|${props.filePath}|blame:${mode}`;
}

function hashDiffStructure(value: FileDiff): string {
  return value.hunks
    .map((hunk) => `${hunk.old_start}:${hunk.old_lines}:${hunk.new_start}:${hunk.new_lines}:${hunk.lines.length}`)
    .join("|");
}

function estimateDiffChars(value: FileDiff): number {
  let chars = 0;
  for (const hunk of value.hunks) {
    for (const line of hunk.lines) {
      chars += line.content.length;
    }
  }
  return chars;
}

function maintainHighlightCache() {
  const cacheLimit = currentHighlightCacheLimit();
  if (highlightedLineCache.value.size > cacheLimit) {
    const toDelete = Math.floor(cacheLimit * 0.2);
    let deleted = 0;
    for (const key of highlightedLineCache.value.keys()) {
      if (deleted >= toDelete) break;
      highlightedLineCache.value.delete(key);
      deleted++;
    }
  }
}

function maintainBlameCache() {
  const cacheLimit = currentBlameCacheLimit();
  if (blameCache.value.size <= cacheLimit) {
    return;
  }

  const toDelete = Math.max(1, blameCache.value.size - cacheLimit);
  let deleted = 0;
  for (const key of blameCache.value.keys()) {
    blameCache.value.delete(key);
    deleted += 1;
    if (deleted >= toDelete) {
      break;
    }
  }
}

const isWorkingChanges = computed(() => !props.commitSha);
const isUnstaged = computed(() => isWorkingChanges.value && !props.staged);
const fileNameParts = computed(() => splitFilePath(props.filePath));
const blameSupportedView = computed(() => viewMode.value === "diff" || viewMode.value === "split-diff");
const blamePanelVisible = computed(() => showBlamePanel.value && blameSupportedView.value);

interface InlineDiffPair {
  compareText: string;
}

const INLINE_PAIR_MIN_SCORE = 0.25;
const INLINE_LCS_MAX_CELLS = 220000;
const INLINE_TOKEN_LCS_MAX_CELLS = 100000;
const INLINE_PAIR_MAX_CELLS = 24000;
const INLINE_PAIR_DIFF_LINE_LIMIT = 2400;
const INLINE_PAIR_DIFF_CHAR_LIMIT = 260000;
const LONG_LINE_PLAIN_TEXT_THRESHOLD = 4000;
const PLAIN_TEXT_FAST_PATH_PATTERN = /\.(rtf|txt|log|csv|tsv|jsonl|lock|patch)$/i;
const FILE_WATCH_INTERVAL_MS = 1800;
const CACHE_CLEANUP_INTERVAL_MS = 12_000;
const FULL_FILE_SIMPLIFY_LINE_LIMIT = 35000;
const FULL_FILE_SIMPLIFY_CHAR_LIMIT = 1200000;
const DIFF_COLORING_TIMEOUT_MS = 5000;

const rawDiffFallbackActive = ref(false);
const rawDiffFallbackReason = ref<string | null>(null);
let diffColoringDeadline = 0;
let diffFallbackNotified = false;

function resetDiffColoringBudget() {
  diffColoringDeadline = Date.now() + DIFF_COLORING_TIMEOUT_MS;
  rawDiffFallbackActive.value = false;
  rawDiffFallbackReason.value = null;
  diffFallbackNotified = false;
}

function shouldAbortDiffColoring(): boolean {
  return diffColoringDeadline > 0 && Date.now() > diffColoringDeadline;
}

function activateRawDiffFallback(reason = "Coloring exceeded 5 seconds. Showing regular Git diff.") {
  if (rawDiffFallbackActive.value) return;

  rawDiffFallbackActive.value = true;
  rawDiffFallbackReason.value = reason;
  highlightedLineCache.value.clear();

  if (!diffFallbackNotified) {
    diffFallbackNotified = true;
    toast.info("Diff coloring exceeded 5 seconds. Showing regular Git diff.");
  }
}

const isPlainTextFastPath = computed(() => PLAIN_TEXT_FAST_PATH_PATTERN.test(props.filePath));

function commonPrefixLength(a: string, b: string): number {
  const max = Math.min(a.length, b.length);
  let idx = 0;
  while (idx < max && a[idx] === b[idx]) {
    idx += 1;
  }
  return idx;
}

function commonSuffixLength(a: string, b: string, prefixLength: number): number {
  const max = Math.min(a.length - prefixLength, b.length - prefixLength);
  let idx = 0;
  while (idx < max && a[a.length - 1 - idx] === b[b.length - 1 - idx]) {
    idx += 1;
  }
  return idx;
}

function lineSimilarity(a: string, b: string): number {
  const left = a.trim();
  const right = b.trim();
  if (!left && !right) {
    return 1;
  }
  if (left === right) {
    return 1;
  }

  const prefix = commonPrefixLength(left, right);
  const suffix = commonSuffixLength(left, right, prefix);
  const overlap = prefix + suffix;
  const maxLen = Math.max(left.length, right.length, 1);
  return overlap / maxLen;
}

function buildInlineSimilarityMatrix(
  hunk: { lines: DiffLine[] },
  deletionIndexes: number[],
  additionIndexes: number[],
): number[][] {
  return Array.from({ length: deletionIndexes.length }, (_, row) => {
    const deletionText = hunk.lines[deletionIndexes[row]].content.replace(/\n$/, "");
    return Array.from({ length: additionIndexes.length }, (_, col) => {
      const additionText = hunk.lines[additionIndexes[col]].content.replace(/\n$/, "");
      return lineSimilarity(deletionText, additionText);
    });
  });
}

function buildInlinePairDp(scores: number[][]): number[][] {
  const rows = scores.length;
  const cols = scores[0]?.length ?? 0;
  const dp = Array.from({ length: rows + 1 }, () => Array.from({ length: cols + 1 }, () => 0));

  for (let row = 1; row <= rows; row += 1) {
    for (let col = 1; col <= cols; col += 1) {
      let best = Math.max(dp[row - 1][col], dp[row][col - 1]);
      const score = scores[row - 1][col - 1];
      if (score >= INLINE_PAIR_MIN_SCORE) {
        best = Math.max(best, dp[row - 1][col - 1] + score);
      }
      dp[row][col] = best;
    }
  }

  return dp;
}

function backtrackInlinePairs(
  scores: number[][],
  dp: number[][],
  deletionIndexes: number[],
  additionIndexes: number[],
): Array<[number, number]> {
  const pairs: Array<[number, number]> = [];
  let row = deletionIndexes.length;
  let col = additionIndexes.length;

  while (row > 0 && col > 0) {
    const score = scores[row - 1][col - 1];
    const matchValue = score >= INLINE_PAIR_MIN_SCORE ? dp[row - 1][col - 1] + score : Number.NEGATIVE_INFINITY;

    if (score >= INLINE_PAIR_MIN_SCORE && Math.abs(dp[row][col] - matchValue) < 1e-7) {
      pairs.push([deletionIndexes[row - 1], additionIndexes[col - 1]]);
      row -= 1;
      col -= 1;
      continue;
    }

    if (dp[row - 1][col] >= dp[row][col - 1]) {
      row -= 1;
    } else {
      col -= 1;
    }
  }

  pairs.reverse();
  return pairs;
}

function collectInlineLinePairs(hunk: { lines: DiffLine[] }, deletionIndexes: number[], additionIndexes: number[]): Array<[number, number]> {
  const rows = deletionIndexes.length;
  const cols = additionIndexes.length;
  if (rows === 0 || cols === 0) {
    return [];
  }

  if (rows * cols > INLINE_PAIR_MAX_CELLS) {
    const count = Math.min(rows, cols);
    return Array.from({ length: count }, (_, idx) => [deletionIndexes[idx], additionIndexes[idx]] as [number, number]);
  }

  const scores = buildInlineSimilarityMatrix(hunk, deletionIndexes, additionIndexes);
  const dp = buildInlinePairDp(scores);
  return backtrackInlinePairs(scores, dp, deletionIndexes, additionIndexes);
}

function buildPrefixSuffixInlineMarkup(base: string, compare: string, changedClass: string): string {
  const maxPrefix = Math.min(base.length, compare.length);
  let prefix = 0;
  while (prefix < maxPrefix && base[prefix] === compare[prefix]) {
    prefix += 1;
  }

  const maxSuffix = Math.min(base.length - prefix, compare.length - prefix);
  let suffix = 0;
  while (suffix < maxSuffix && base[base.length - 1 - suffix] === compare[compare.length - 1 - suffix]) {
    suffix += 1;
  }

  const changedEnd = base.length - suffix;
  const parts: string[] = [];

  if (prefix > 0) {
    parts.push(escapeHtml(base.slice(0, prefix)));
  }

  const changedPart = base.slice(prefix, changedEnd);
  if (changedPart) {
    parts.push(`<span class="${changedClass}">${escapeHtml(changedPart)}</span>`);
  }

  if (suffix > 0) {
    parts.push(escapeHtml(base.slice(changedEnd)));
  }

  if (parts.length === 0) {
    return escapeHtml(base);
  }

  return parts.join("");
}

function computeLcsKeepMask(base: string, compare: string): boolean[] | null {
  const rows = base.length;
  const cols = compare.length;
  if (rows === 0) {
    return [];
  }

  if ((rows + 1) * (cols + 1) > INLINE_LCS_MAX_CELLS) {
    return null;
  }

  const dp = buildLcsMatrix(base, compare);
  return backtrackLcsKeepMask(base, compare, dp);
}

function buildLcsMatrix(base: string, compare: string): Uint32Array[] {
  const rows = base.length;
  const cols = compare.length;
  const dp = Array.from({ length: rows + 1 }, () => new Uint32Array(cols + 1));

  for (let row = 1; row <= rows; row += 1) {
    for (let col = 1; col <= cols; col += 1) {
      if (base[row - 1] === compare[col - 1]) {
        dp[row][col] = dp[row - 1][col - 1] + 1;
      } else {
        dp[row][col] = Math.max(dp[row - 1][col], dp[row][col - 1]);
      }
    }
  }

  return dp;
}

function backtrackLcsKeepMask(base: string, compare: string, dp: Uint32Array[]): boolean[] {
  const keep = Array.from({ length: base.length }, () => false);
  let row = base.length;
  let col = compare.length;

  while (row > 0 && col > 0) {
    if (base[row - 1] === compare[col - 1]) {
      keep[row - 1] = true;
      row -= 1;
      col -= 1;
      continue;
    }

    if (dp[row - 1][col] >= dp[row][col - 1]) {
      row -= 1;
    } else {
      col -= 1;
    }
  }

  return keep;
}

const INLINE_TOKEN_SPLIT_PATTERN = /(\w+|\s+|[^\s\w]+)/g;

function isWhitespaceToken(value: string): boolean {
  return /^\s+$/.test(value);
}

function bridgeWhitespaceRuns(tokens: string[], changedMask: boolean[]): boolean[] {
  const bridged = [...changedMask];
  let idx = 0;

  while (idx < tokens.length) {
    if (!isWhitespaceToken(tokens[idx])) {
      idx += 1;
      continue;
    }

    const start = idx;
    while (idx < tokens.length && isWhitespaceToken(tokens[idx])) {
      idx += 1;
    }

    const left = start - 1;
    const right = idx;
    if (left >= 0 && right < tokens.length && bridged[left] && bridged[right]) {
      for (let run = start; run < right; run += 1) {
        bridged[run] = true;
      }
    }
  }

  return bridged;
}

function buildInlineSegmentsFromTokens(
  tokens: string[],
  changedMask: boolean[],
  changedClass: string,
): { parts: string[]; hasChanged: boolean } {
  const parts: string[] = [];
  let segment = "";
  let segmentChanged = changedMask[0] ?? false;
  let hasChanged = segmentChanged;

  for (let idx = 0; idx < tokens.length; idx += 1) {
    const changed = changedMask[idx] ?? false;
    if (idx > 0 && changed !== segmentChanged) {
      pushInlineSegment(parts, segment, segmentChanged, changedClass);
      segment = "";
      segmentChanged = changed;
    }

    segment += tokens[idx];
    if (changed) {
      hasChanged = true;
    }
  }

  pushInlineSegment(parts, segment, segmentChanged, changedClass);
  return { parts, hasChanged };
}

function tokenizeInlineText(value: string): string[] {
  const tokens = value.match(INLINE_TOKEN_SPLIT_PATTERN);
  if (!tokens || tokens.length === 0) {
    return [value];
  }
  return tokens;
}

function computeTokenLcsKeepMask(baseTokens: string[], compareTokens: string[]): boolean[] | null {
  const rows = baseTokens.length;
  const cols = compareTokens.length;
  if (rows === 0) {
    return [];
  }

  if ((rows + 1) * (cols + 1) > INLINE_TOKEN_LCS_MAX_CELLS) {
    return null;
  }

  const dp = buildTokenLcsMatrix(baseTokens, compareTokens);
  return backtrackTokenLcsKeepMask(baseTokens, compareTokens, dp);
}

function buildTokenLcsMatrix(baseTokens: string[], compareTokens: string[]): Uint32Array[] {
  const rows = baseTokens.length;
  const cols = compareTokens.length;

  const dp = Array.from({ length: rows + 1 }, () => new Uint32Array(cols + 1));

  for (let row = 1; row <= rows; row += 1) {
    for (let col = 1; col <= cols; col += 1) {
      if (baseTokens[row - 1] === compareTokens[col - 1]) {
        dp[row][col] = dp[row - 1][col - 1] + 1;
      } else {
        dp[row][col] = Math.max(dp[row - 1][col], dp[row][col - 1]);
      }
    }
  }

  return dp;
}

function backtrackTokenLcsKeepMask(baseTokens: string[], compareTokens: string[], dp: Uint32Array[]): boolean[] {
  const rows = baseTokens.length;
  const cols = compareTokens.length;

  const keep = Array.from({ length: rows }, () => false);
  let row = rows;
  let col = cols;

  while (row > 0 && col > 0) {
    if (baseTokens[row - 1] === compareTokens[col - 1]) {
      keep[row - 1] = true;
      row -= 1;
      col -= 1;
      continue;
    }

    if (dp[row - 1][col] >= dp[row][col - 1]) {
      row -= 1;
    } else {
      col -= 1;
    }
  }

  return keep;
}

function buildTokenInlineMarkup(base: string, compare: string, changedClass: string): string | null {
  const baseTokens = tokenizeInlineText(base);
  const compareTokens = tokenizeInlineText(compare);
  const keepMask = computeTokenLcsKeepMask(baseTokens, compareTokens);
  if (!keepMask || keepMask.length !== baseTokens.length) {
    return null;
  }

  const changedMask = bridgeWhitespaceRuns(
    baseTokens,
    keepMask.map((keep) => !keep),
  );
  const { parts, hasChanged } = buildInlineSegmentsFromTokens(baseTokens, changedMask, changedClass);

  if (!hasChanged) {
    return null;
  }

  return parts.join("");
}

watch(
  () => [props.filePath, props.staged, props.commitSha],
  () => {
    fileContentLoadSequence += 1;
    currentHunkIndex.value = 0;
    blameError.value = null;
    if (showBlamePanel.value) {
      void loadFileBlame(true);
    }
    reload();
  }
);

watch(viewMode, (mode) => {
  if (mode !== "time-lapse") {
    stopTimeLapsePlayback();
  }

  if (mode !== "diff" && mode !== "split-diff") {
    showBlamePanel.value = false;
  }

  if (mode === "diff" || mode === "split-diff" || mode === "file-diff") {
    resetDiffColoringBudget();
    if (showBlamePanel.value) {
      void loadFileBlame();
    }
  }

  if (mode === "edit") {
    loadFileForEdit();
  } else if (mode === "file-diff") {
    loadFileContentAsync();
    void nextTick(() => {
      syncFileDiffViewportMetrics();
    });
  } else if (mode === "time-lapse") {
    void loadTimeLapseFrames();
  } else {
    reload();
  }
});

watch(timeLapseCommitWindowSafe, () => {
  if (viewMode.value !== "time-lapse") return;
  void loadTimeLapseFrames();
});

watch(blamePanelVisible, (visible) => {
  if (!visible) {
    return;
  }

  void loadFileBlame();
});

async function reload() {
  if (viewMode.value === "edit") return;

  resetDiffColoringBudget();
  
  loading.value = true;
  error.value = null;
  
  try {
    const diffCacheKey = getDiffCacheKey();
    const cachedDiff = diffCacheKey ? getCachedStructuredDiff(diffCacheKey) : null;

    if (cachedDiff) {
      diff.value = cachedDiff;
    } else {
      if (props.commitSha) {
        diff.value = await invoke<FileDiff>("get_commit_diff", {
          path: props.repoPath,
          sha: props.commitSha,
          filePath: props.filePath,
        });
      } else {
        diff.value = await invoke<FileDiff>("get_working_diff", {
          path: props.repoPath,
          filePath: props.filePath,
          staged: props.staged ?? false,
        });
      }

      if (diff.value && diffCacheKey) {
        setCachedStructuredDiff(diffCacheKey, diff.value, estimateDiffChars(diff.value));
      }
    }

    if (diff.value) {
      lastFileHash = hashDiffStructure(diff.value);
    }

    highlightedLineCache.value.clear();

    if (blamePanelVisible.value) {
      void loadFileBlame();
    }
    
    // Don't load file content synchronously - do it lazily
    if (viewMode.value === "file-diff") {
      void loadFileContentAsync();
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function loadFileContentAsync(forceRefresh = false) {
  if (loadingFileContent.value) return;
  loadingFileContent.value = true;
  const loadSequence = ++fileContentLoadSequence;
  
  try {
    // Use requestAnimationFrame to prevent blocking
    await new Promise(resolve => requestAnimationFrame(resolve));
    if (loadSequence !== fileContentLoadSequence) return;
    
    const loadedContent = await loadDisplayedFileContent(forceRefresh);
    if (loadSequence !== fileContentLoadSequence) return;
    fileContent.value = loadedContent;
    
    // Allow UI to update
    await nextTick();
    await new Promise(resolve => requestAnimationFrame(resolve));
    if (loadSequence !== fileContentLoadSequence) return;
  } catch (e) {
    if (loadSequence === fileContentLoadSequence) {
      error.value = String(e);
    }
  } finally {
    if (loadSequence === fileContentLoadSequence && viewMode.value === "file-diff") {
      syncFileDiffViewportMetrics();
    }

    if (loadSequence === fileContentLoadSequence) {
      loadingFileContent.value = false;
    }
  }
}

async function loadDisplayedFileContent(forceRefresh = false): Promise<string> {
  const cacheKey = getFileContentCacheKey();
  if (cacheKey && !forceRefresh) {
    const cached = getCachedDiffText(cacheKey);
    if (cached !== null) {
      return cached;
    }
  }

  if (props.commitSha) {
    const content = await invoke<string>("get_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
      sha: props.commitSha,
    });
    if (cacheKey) {
      setCachedDiffText(cacheKey, content);
    }
    return content;
  }

  if (props.staged) {
    const content = await invoke<string>("get_staged_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
    });
    if (cacheKey) {
      setCachedDiffText(cacheKey, content);
    }
    return content;
  }

  return invoke<string>("get_file_content", {
    path: props.repoPath,
    filePath: props.filePath,
    sha: null,
  });
}

async function loadFileBlame(forceRefresh = false) {
  if (!blamePanelVisible.value || diff.value?.is_binary) {
    return;
  }
  if (blameLoading.value) {
    return;
  }

  blameLoading.value = true;
  blameError.value = null;

  try {
    const blameCacheKey = getBlameCacheKey();
    if (!forceRefresh) {
      const cached = blameCache.value.get(blameCacheKey);
      if (cached) {
        blameLines.value = cached;
        return;
      }
    }

    const lines = await invoke<FileBlameLine[]>("get_file_blame", {
      path: props.repoPath,
      filePath: props.filePath,
      sha: props.commitSha ?? null,
    });

    blameLines.value = lines;
    blameCache.value.set(blameCacheKey, lines);
    maintainBlameCache();
  } catch (e) {
    blameLines.value = [];
    blameError.value = String(e);
  } finally {
    blameLoading.value = false;
  }
}

function toggleBlamePanel() {
  if (!blameSupportedView.value || diff.value?.is_binary) {
    return;
  }

  showBlamePanel.value = !showBlamePanel.value;
}

function refreshBlame() {
  void loadFileBlame(true);
}

function formatBlameTimestamp(timestamp: number): string {
  if (!timestamp) {
    return "Unknown time";
  }

  const value = Math.abs(timestamp) < 1000000000000 ? timestamp * 1000 : timestamp;
  return new Date(value).toLocaleString();
}

interface InlineBlameEntry {
  entry: FileBlameLine;
  showHeader: boolean;
  groupSize: number;
}

const inlineBlameByLine = computed(() => {
  const sorted = [...blameLines.value].sort((a, b) => a.line_no - b.line_no);
  const map = new Map<number, InlineBlameEntry>();
  if (sorted.length === 0) {
    return map;
  }

  const showHeaderOnEveryLine = isBlameByDefaultEnabled();

  let groupStart = 0;
  const finalizeGroup = (startIdx: number, endIdx: number) => {
    const size = endIdx - startIdx + 1;
    for (let idx = startIdx; idx <= endIdx; idx += 1) {
      const line = sorted[idx];
      map.set(line.line_no, {
        entry: line,
        showHeader: showHeaderOnEveryLine || idx === startIdx,
        groupSize: size,
      });
    }
  };

  for (let idx = 1; idx <= sorted.length; idx += 1) {
    const previous = sorted[idx - 1];
    const current = sorted[idx];
    const sameGroup = !!current
      && current.line_no === previous.line_no + 1
      && current.author === previous.author
      && current.author_time === previous.author_time
      && current.commit_sha === previous.commit_sha;

    if (!sameGroup) {
      finalizeGroup(groupStart, idx - 1);
      groupStart = idx;
    }
  }

  return map;
});

function getInlineBlame(lineNo: number | null): InlineBlameEntry | null {
  if (!lineNo || !showBlamePanel.value) {
    return null;
  }

  return inlineBlameByLine.value.get(lineNo) || null;
}

function getDiffRowBlameLineNo(line: DiffLine): number | null {
  return line.new_line_no ?? null;
}

function getSplitRowBlameLineNo(row: SplitDiffRow): number | null {
  return row.newLineNo ?? null;
}

function splitPaneKey(hunkIdx: number, side: "old" | "new"): string {
  return `${hunkIdx}:${side}`;
}

function resolveSplitPaneElement(el: unknown): HTMLElement | null {
  if (el instanceof HTMLElement) {
    return el;
  }

  if (el && typeof el === "object" && "$el" in el) {
    const maybeElement = (el as { $el?: unknown }).$el;
    if (maybeElement instanceof HTMLElement) {
      return maybeElement;
    }
  }

  return null;
}

function setSplitPaneRef(hunkIdx: number, side: "old" | "new", el: unknown) {
  const key = splitPaneKey(hunkIdx, side);
  const element = resolveSplitPaneElement(el);

  if (!element) {
    splitPaneRefs.value.delete(key);
    return;
  }

  splitPaneRefs.value.set(key, element);
}

let splitScrollSyncLock = false;

function onSplitPaneScroll(hunkIdx: number, side: "old" | "new", event: Event) {
  if (splitScrollSyncLock) {
    return;
  }

  const source = event.target as HTMLElement;
  const partnerKey = splitPaneKey(hunkIdx, side === "old" ? "new" : "old");
  const partner = splitPaneRefs.value.get(partnerKey);
  if (!partner) {
    return;
  }

  splitScrollSyncLock = true;
  partner.scrollLeft = source.scrollLeft;
  splitScrollSyncLock = false;
}

async function loadFileForEdit() {
  loading.value = true;
  error.value = null;
  try {
    const content = await invoke<string>("get_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
      sha: null,
    });
    fileContent.value = content;
    editContent.value = content;
    hasUnsavedChanges.value = false;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function saveFile() {
  if (!hasUnsavedChanges.value) return;
  saving.value = true;
  try {
    await invoke("save_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
      content: editContent.value,
    });
    hasUnsavedChanges.value = false;
    fileContent.value = editContent.value;
    emit("refresh");
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function discardEditChanges() {
  editContent.value = fileContent.value;
  hasUnsavedChanges.value = false;
}

async function revertHunk(hunkIdx: number) {
  if (!diff.value || !isWorkingChanges.value) return;
  try {
    await invoke("revert_hunk", {
      path: props.repoPath,
      filePath: props.filePath,
      hunkIndex: hunkIdx,
      staged: props.staged ?? false,
    });
    emit("refresh");
    await reload();
  } catch (e) {
    error.value = `Failed to revert hunk: ${e}`;
  }
}

function startFileWatch() {
  if (fileWatchInterval) return;
  
  fileWatchInterval = setInterval(async () => {
    if (!isUnstaged.value || viewMode.value === "edit" || viewMode.value === "time-lapse") return;
    if (loading.value || loadingFileContent.value || fileWatchRequestInFlight) return;
    fileWatchRequestInFlight = true;
    
    try {
      const newDiff = await invoke<FileDiff>("get_working_diff", {
        path: props.repoPath,
        filePath: props.filePath,
        staged: false,
      });
      
      const newHash = hashDiffStructure(newDiff);
      if (newHash !== lastFileHash) {
        lastFileHash = newHash;
        diff.value = newDiff;
        highlightedLineCache.value.clear();
        if (blamePanelVisible.value) {
          await loadFileBlame(true);
        }
        if (viewMode.value === "file-diff") {
          await loadFileContentAsync(true);
        }
      }
    } catch {
    } finally {
      fileWatchRequestInFlight = false;
    }
  }, FILE_WATCH_INTERVAL_MS);
}

function stopFileWatch() {
  if (fileWatchInterval) {
    clearInterval(fileWatchInterval);
    fileWatchInterval = null;
  }
  fileWatchRequestInFlight = false;
}

watch(isUnstaged, (val) => {
  if (val) {
    startFileWatch();
  } else {
    stopFileWatch();
  }
}, { immediate: true });

watch(
  () => [props.filePath, props.commitSha, props.staged],
  () => {
    if (!isBlameByDefaultEnabled() || !blameSupportedView.value) {
      return;
    }

    showBlamePanel.value = true;
    void loadFileBlame();
  },
);

onMounted(() => {
  pruneDiffViewerCaches();
  cacheCleanupInterval = setInterval(() => {
    pruneDiffViewerCaches();
  }, CACHE_CLEANUP_INTERVAL_MS);
  window.addEventListener("resize", syncFileDiffViewportMetrics);

  const enableBlameByDefault = isBlameByDefaultEnabled();
  reload().finally(() => {
    if (enableBlameByDefault && !diff.value?.is_binary && blameSupportedView.value) {
      showBlamePanel.value = true;
      void loadFileBlame();
    }
  });
  if (isUnstaged.value) {
    startFileWatch();
  }
});

onUnmounted(() => {
  if (cacheCleanupInterval) {
    clearInterval(cacheCleanupInterval);
    cacheCleanupInterval = null;
  }

  window.removeEventListener("resize", syncFileDiffViewportMetrics);

  stopFileWatch();
  stopTimeLapsePlayback();
});

function navigateHunk(dir: "prev" | "next") {
  if (!diff.value) return;
  if (dir === "prev" && currentHunkIndex.value > 0) {
    currentHunkIndex.value--;
  } else if (dir === "next" && currentHunkIndex.value < diff.value.hunks.length - 1) {
    currentHunkIndex.value++;
  }
  const el = document.getElementById(`hunk-${currentHunkIndex.value}`);
  el?.scrollIntoView({ behavior: "auto", block: "start" });
}

interface DiffLineRef {
  hunkIdx: number;
  lineIdx: number;
  line: DiffLine;
}

interface SplitDiffRow {
  oldLineNo: number | null;
  newLineNo: number | null;
  oldType: "context" | "deletion" | "empty";
  newType: "context" | "addition" | "empty";
  oldContent: string;
  newContent: string;
  oldRef: DiffLineRef | null;
  newRef: DiffLineRef | null;
}

function normalizeDiffLineContent(line: DiffLine): string {
  return line.content.replace(/\n$/, "");
}

function toDiffLineRef(hunkIdx: number, lineIdx: number, line: DiffLine): DiffLineRef {
  return {
    hunkIdx,
    lineIdx,
    line,
  };
}

function collectContiguousDiffRefs(
  lines: DiffLine[],
  hunkIdx: number,
  startIndex: number,
  expectedType: "addition" | "deletion",
): { refs: DiffLineRef[]; nextIndex: number } {
  const refs: DiffLineRef[] = [];
  let cursor = startIndex;

  while (cursor < lines.length && lines[cursor].line_type === expectedType) {
    refs.push(toDiffLineRef(hunkIdx, cursor, lines[cursor]));
    cursor += 1;
  }

  return { refs, nextIndex: cursor };
}

function appendSplitContextRow(rows: SplitDiffRow[], ref: DiffLineRef) {
  const content = normalizeDiffLineContent(ref.line);
  rows.push({
    oldLineNo: ref.line.old_line_no,
    newLineNo: ref.line.new_line_no,
    oldType: "context",
    newType: "context",
    oldContent: content,
    newContent: content,
    oldRef: ref,
    newRef: ref,
  });
}

function appendSplitPairedRows(rows: SplitDiffRow[], deletions: DiffLineRef[], additions: DiffLineRef[]) {
  const pairCount = Math.max(deletions.length, additions.length);

  for (let pairIdx = 0; pairIdx < pairCount; pairIdx += 1) {
    const deletion = deletions[pairIdx] || null;
    const addition = additions[pairIdx] || null;

    rows.push({
      oldLineNo: deletion?.line.old_line_no ?? null,
      newLineNo: addition?.line.new_line_no ?? null,
      oldType: deletion ? "deletion" : "empty",
      newType: addition ? "addition" : "empty",
      oldContent: deletion ? normalizeDiffLineContent(deletion.line) : "",
      newContent: addition ? normalizeDiffLineContent(addition.line) : "",
      oldRef: deletion,
      newRef: addition,
    });
  }
}

function appendSplitAdditionRows(rows: SplitDiffRow[], additions: DiffLineRef[]) {
  for (const addition of additions) {
    rows.push({
      oldLineNo: null,
      newLineNo: addition.line.new_line_no,
      oldType: "empty",
      newType: "addition",
      oldContent: "",
      newContent: normalizeDiffLineContent(addition.line),
      oldRef: null,
      newRef: addition,
    });
  }
}

function buildSplitDiffRows(hunk: { lines: DiffLine[] }, hunkIdx: number): SplitDiffRow[] {
  const rows: SplitDiffRow[] = [];
  let lineIdx = 0;

  while (lineIdx < hunk.lines.length) {
    const line = hunk.lines[lineIdx];

    if (line.line_type === "context") {
      appendSplitContextRow(rows, toDiffLineRef(hunkIdx, lineIdx, line));
      lineIdx += 1;
      continue;
    }

    if (line.line_type === "deletion") {
      const deletionBlock = collectContiguousDiffRefs(hunk.lines, hunkIdx, lineIdx, "deletion");
      const additionBlock = collectContiguousDiffRefs(hunk.lines, hunkIdx, deletionBlock.nextIndex, "addition");
      appendSplitPairedRows(rows, deletionBlock.refs, additionBlock.refs);
      lineIdx = additionBlock.nextIndex;
      continue;
    }

    const additionBlock = collectContiguousDiffRefs(hunk.lines, hunkIdx, lineIdx, "addition");
    appendSplitAdditionRows(rows, additionBlock.refs);
    lineIdx = additionBlock.nextIndex;
  }

  return rows;
}

const splitDiffRowsByHunk = computed(() => {
  if (!diff.value) {
    return [] as SplitDiffRow[][];
  }

  return diff.value.hunks.map((hunk, hunkIdx) => buildSplitDiffRows(hunk, hunkIdx));
});

interface FullFileLine {
  lineNo: number;
  oldLineNo: number | null;
  content: string;
  type: 'context' | 'addition' | 'deletion';
  hunkIdx: number | null;
  hunkLineIdx: number | null;
  diffLine: DiffLine | null;
}

const fullFileLines = computed((): FullFileLine[] => {
  if (!diff.value || !fileContent.value) return [];
  
  const lines = fileContent.value.split('\n');
  if (lines.length > FULL_FILE_SIMPLIFY_LINE_LIMIT || fileContent.value.length > FULL_FILE_SIMPLIFY_CHAR_LIMIT) {
    return lines.map((content, idx) => ({
      lineNo: idx + 1,
      oldLineNo: idx + 1,
      content,
      type: 'context' as const,
      hunkIdx: null,
      hunkLineIdx: null,
      diffLine: null,
    }));
  }

  const result: FullFileLine[] = [];
  
  const additionLines = new Map<number, { hunkIdx: number; hunkLineIdx: number; line: DiffLine }>();
  const deletionsByHunk = new Map<number, Array<{ line: DiffLine; hunkLineIdx: number }>>();
  
  diff.value.hunks.forEach((hunk, hunkIdx) => {
    const deletions: Array<{ line: DiffLine; hunkLineIdx: number }> = [];
    hunk.lines.forEach((line, lineIdx) => {
      if (line.line_type === 'addition' && line.new_line_no) {
        additionLines.set(line.new_line_no, { hunkIdx, hunkLineIdx: lineIdx, line });
      } else if (line.line_type === 'deletion') {
        deletions.push({ line, hunkLineIdx: lineIdx });
      }
    });
    if (deletions.length > 0) {
      deletionsByHunk.set(hunkIdx, deletions);
    }
  });
  
  let deletionInsertPoint: number | null = null;
  
  for (let i = 0; i < lines.length; i++) {
    const lineNo = i + 1;
    const addInfo = additionLines.get(lineNo);
    
    if (addInfo && deletionInsertPoint !== addInfo.hunkIdx) {
      const deletions = deletionsByHunk.get(addInfo.hunkIdx);
      if (deletions) {
        deletions.forEach((del) => {
          result.push({
            lineNo: 0,
            oldLineNo: del.line.old_line_no,
            content: del.line.content.replace(/\n$/, ''),
            type: 'deletion',
            hunkIdx: addInfo.hunkIdx,
            hunkLineIdx: del.hunkLineIdx,
            diffLine: del.line,
          });
        });
        deletionInsertPoint = addInfo.hunkIdx;
      }
    }
    
    if (addInfo) {
      result.push({
        lineNo,
        oldLineNo: null,
        content: lines[i],
        type: 'addition',
        hunkIdx: addInfo.hunkIdx,
        hunkLineIdx: addInfo.hunkLineIdx,
        diffLine: addInfo.line,
      });
    } else {
      result.push({
        lineNo,
        oldLineNo: lineNo, 
        content: lines[i],
        type: 'context',
        hunkIdx: null,
        hunkLineIdx: null,
        diffLine: null,
      });
    }
  }
  
  return result;
});

const diffPayload = computed(() => {
  if (!diff.value) {
    return { lines: 0, chars: 0 };
  }

  let lines = 0;
  let chars = 0;

  for (const hunk of diff.value.hunks) {
    lines += hunk.lines.length;
    for (const line of hunk.lines) {
      chars += line.content.length;
    }
  }

  return { lines, chars };
});

const usePlainTextHighlighting = computed(() => {
  if (rawDiffFallbackActive.value) {
    return true;
  }

  if (viewMode.value === "file-diff") {
    return fullFileLines.value.length > MAX_HIGHLIGHT_LINES || fileContent.value.length > MAX_HIGHLIGHT_CHARS;
  }

  return diffPayload.value.lines > MAX_HIGHLIGHT_LINES || diffPayload.value.chars > MAX_HIGHLIGHT_CHARS;
});

const inlineDiffEnabled = computed(() => {
  if (rawDiffFallbackActive.value || isPlainTextFastPath.value || usePlainTextHighlighting.value) {
    return false;
  }

  return diffPayload.value.lines <= INLINE_PAIR_DIFF_LINE_LIMIT && diffPayload.value.chars <= INLINE_PAIR_DIFF_CHAR_LIMIT;
});

const loadingLabel = computed(() => {
  if (loadingFileContent.value) {
    return "Preparing full file view";
  }
  return "Parsing diff hunks";
});

const activeTimeLapseFrame = computed(() => {
  if (timeLapseFrames.value.length === 0) return null;
  const idx = Math.min(Math.max(0, timeLapseFrameIndex.value), timeLapseFrames.value.length - 1);
  return timeLapseFrames.value[idx] || null;
});

interface TimeLapseRenderLine {
  key: string;
  oldLineNo: number | null;
  newLineNo: number | null;
  type: "context" | "addition" | "deletion";
  content: string;
}

interface TimeLapseDecorations {
  additionsByLine: Map<number, DiffLine>;
  deletionsByAnchor: Map<number, DiffLine[]>;
}

function buildPlainTimeLapseRenderLines(lines: string[]): TimeLapseRenderLine[] {
  return lines.map((content, idx) => ({
    key: `ctx:${idx + 1}`,
    oldLineNo: idx + 1,
    newLineNo: idx + 1,
    type: "context",
    content,
  }));
}

function collectTimeLapseDecorations(diffLines: DiffLine[]): TimeLapseDecorations {
  const additionsByLine = new Map<number, DiffLine>();
  const deletionsByAnchor = new Map<number, DiffLine[]>();
  let currentNewLine = 1;

  for (const diffLine of diffLines) {
    if (diffLine.new_line_no !== null) {
      currentNewLine = diffLine.new_line_no;
    }

    if (diffLine.line_type === "addition" && diffLine.new_line_no !== null) {
      additionsByLine.set(diffLine.new_line_no, diffLine);
      continue;
    }

    if (diffLine.line_type === "deletion") {
      const anchor = Math.max(1, currentNewLine);
      const bucket = deletionsByAnchor.get(anchor) ?? [];
      bucket.push(diffLine);
      deletionsByAnchor.set(anchor, bucket);
    }
  }

  return { additionsByLine, deletionsByAnchor };
}

function appendTimeLapseDeletions(rendered: TimeLapseRenderLine[], anchor: number, deletions: DiffLine[], tail = false) {
  deletions.forEach((line, deleteIdx) => {
    rendered.push({
      key: `${tail ? "tail-del" : "del"}:${anchor}:${deleteIdx}:${line.old_line_no ?? 0}`,
      oldLineNo: line.old_line_no,
      newLineNo: null,
      type: "deletion",
      content: line.content.replace(/\n$/, ""),
    });
  });
}

function buildTimeLapseRenderLines(frame: TimeLapseFrame | null): TimeLapseRenderLine[] {
  if (!frame) return [];

  const lines = frame.content.split("\n");
  if (!frame.diffLines.length) {
    return buildPlainTimeLapseRenderLines(lines);
  }

  const { additionsByLine, deletionsByAnchor } = collectTimeLapseDecorations(frame.diffLines);

  const rendered: TimeLapseRenderLine[] = [];

  for (let idx = 0; idx < lines.length; idx += 1) {
    const lineNo = idx + 1;
    const pendingDeletions = deletionsByAnchor.get(lineNo);
    if (pendingDeletions && pendingDeletions.length) {
      appendTimeLapseDeletions(rendered, lineNo, pendingDeletions);
      deletionsByAnchor.delete(lineNo);
    }

    const addition = additionsByLine.get(lineNo);
    rendered.push({
      key: `line:${lineNo}`,
      oldLineNo: addition ? null : lineNo,
      newLineNo: lineNo,
      type: addition ? "addition" : "context",
      content: lines[idx],
    });
  }

  for (const [anchor, trailing] of deletionsByAnchor) {
    appendTimeLapseDeletions(rendered, anchor, trailing, true);
  }

  return rendered;
}

const activeTimeLapseRenderLines = computed(() => buildTimeLapseRenderLines(activeTimeLapseFrame.value));

function stopTimeLapsePlayback() {
  if (timeLapseTimer) {
    clearInterval(timeLapseTimer);
    timeLapseTimer = null;
  }
  timeLapsePlaying.value = false;
}

function nextTimeLapseFrame() {
  if (timeLapseFrames.value.length === 0) return;
  timeLapseFrameIndex.value = (timeLapseFrameIndex.value + 1) % timeLapseFrames.value.length;
}

function previousTimeLapseFrame() {
  if (timeLapseFrames.value.length === 0) return;
  timeLapseFrameIndex.value = (timeLapseFrameIndex.value - 1 + timeLapseFrames.value.length) % timeLapseFrames.value.length;
}

function toggleTimeLapsePlay() {
  if (timeLapseFrames.value.length <= 1) return;

  if (timeLapsePlaying.value) {
    stopTimeLapsePlayback();
    return;
  }

  timeLapsePlaying.value = true;
  timeLapseTimer = setInterval(() => {
    nextTimeLapseFrame();
  }, 850);
}

function formatTimeLapseTimestamp(timestamp: number): string {
  const value = Math.abs(timestamp) < 1000000000000 ? timestamp * 1000 : timestamp;
  return new Date(value).toLocaleString();
}

function normalizeCommitFilePath(path: string): string {
  return path.replace(/\\/g, "/").trim();
}

async function getCommitFilesSafe(sha: string): Promise<CommitFileInfo[] | null> {
  try {
    return await invoke<CommitFileInfo[]>("get_commit_files", {
      path: props.repoPath,
      sha,
    });
  } catch {
    return null;
  }
}

function commitTouchesFile(commitFiles: CommitFileInfo[], normalizedPath: string): boolean {
  return commitFiles.some((file) => normalizeCommitFilePath(file.path) === normalizedPath);
}

async function getFrameDiffLinesSafe(sha: string): Promise<DiffLine[]> {
  try {
    const frameDiff = await invoke<FileDiff>("get_commit_diff", {
      path: props.repoPath,
      filePath: props.filePath,
      sha,
    });

    const lines: DiffLine[] = [];
    for (const hunk of frameDiff.hunks) {
      for (const line of hunk.lines) {
        lines.push(line);
      }
    }
    return lines;
  } catch {
    return [];
  }
}

async function buildTimeLapseFrame(commit: CommitInfo, normalizedPath: string): Promise<TimeLapseFrame | null> {
  const commitFiles = await getCommitFilesSafe(commit.sha);
  if (!commitFiles || !commitTouchesFile(commitFiles, normalizedPath)) {
    return null;
  }

  try {
    const content = await invoke<string>("get_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
      sha: commit.sha,
    });

    const diffLines = await getFrameDiffLinesSafe(commit.sha);
    return {
      sha: commit.sha,
      shortSha: commit.short_sha,
      author: commit.author_name,
      message: commit.message.split("\n")[0],
      timestamp: commit.timestamp,
      content,
      diffLines,
    };
  } catch {
    return null;
  }
}

async function loadTimeLapseFrames() {
  if (timeLapseLoading.value) return;
  if (!props.repoPath) return;

  stopTimeLapsePlayback();
  timeLapseLoading.value = true;
  timeLapseFrames.value = [];
  timeLapseFrameIndex.value = 0;

  try {
    const commits = await invoke<CommitInfo[]>("get_commits", {
      path: props.repoPath,
      maxCount: timeLapseCommitWindowSafe.value,
    });

    const normalizedPath = normalizeCommitFilePath(props.filePath);
    const frames: TimeLapseFrame[] = [];
    let previousContent = "";

    for (const commit of commits) {
      if (frames.length >= TIME_LAPSE_MAX_FRAMES) break;

      const frame = await buildTimeLapseFrame(commit, normalizedPath);
      if (!frame) continue;
      if (frame.content === previousContent && frames.length > 0) continue;

      frames.push(frame);
      previousContent = frame.content;
    }

    timeLapseFrames.value = frames.reverse();
    timeLapseFrameIndex.value = 0;

    if (timeLapseFrames.value.length === 0) {
      toast.info("No historical frames found for this file in recent commits.");
    }
  } finally {
    timeLapseLoading.value = false;
  }
}

async function runExpertAdvisor() {
  if (!props.repoPath) return;

  try {
    const commits = await invoke<CommitInfo[]>("get_commits", {
      path: props.repoPath,
      maxCount: 220,
    });

    const scoreByAuthor = new Map<string, { commits: number; additions: number; deletions: number }>();
    const normalizedPath = props.filePath.replace(/\\/g, "/").trim();

    for (const commit of commits) {
      let commitFiles: CommitFileInfo[] = [];
      try {
        commitFiles = await invoke<CommitFileInfo[]>("get_commit_files", {
          path: props.repoPath,
          sha: commit.sha,
        });
      } catch {
        continue;
      }

      const target = commitFiles.find((file) => file.path.replace(/\\/g, "/").trim() === normalizedPath);
      if (!target) continue;

      const author = commit.author_name?.trim() || "Unknown";
      const existing = scoreByAuthor.get(author) || { commits: 0, additions: 0, deletions: 0 };
      existing.commits += 1;
      existing.additions += target.additions || 0;
      existing.deletions += target.deletions || 0;
      scoreByAuthor.set(author, existing);
    }

    const ranked = Array.from(scoreByAuthor.entries())
      .map(([author, stats]) => ({ author, ...stats }))
      .sort((a, b) => {
        const scoreA = a.commits * 3 + a.additions + a.deletions;
        const scoreB = b.commits * 3 + b.additions + b.deletions;
        return scoreB - scoreA;
      });

    if (ranked.length === 0) {
      toast.info("Expert advisor: no recent ownership data found for this file.");
      return;
    }

    const top = ranked[0];
    toast.info(`Expert advisor: ${top.author} (${top.commits} commits touching this file recently).`);
  } catch (e) {
    toast.error(`Expert advisor failed: ${String(e)}`);
  }
}

async function shareSelectionAsSnippet() {
  const selected = globalThis.getSelection?.()?.toString().trim() || "";
  if (!selected) {
    toast.warning("Select a code snippet first, then click Share Snippet.");
    return;
  }

  const fileName = fileNameParts.value.fileName || "snippet.txt";

  try {
    const gistUrl = await createGithubGistLink(fileName, selected);
    await navigator.clipboard.writeText(gistUrl);
    toast.success("Snippet shared as gist. Link copied to clipboard.");
  } catch {
    try {
      const pasteUrl = await createPasteRsLink(selected);
      if (pasteUrl) {
        await navigator.clipboard.writeText(pasteUrl);
        toast.success("Snippet shared. Link copied to clipboard.");
        return;
      }
    } catch {
    }

    await navigator.clipboard.writeText(selected);
    toast.warning("Could not create gist link. Selected snippet was copied to clipboard instead.");
  }
}

async function createGithubGistLink(fileName: string, selected: string): Promise<string> {
  const token = git.providerTokens.value.github || git.githubToken.value || "";
  const headers: Record<string, string> = {
    Accept: "application/vnd.github+json",
    "Content-Type": "application/json",
  };
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  const response = await fetch("https://api.github.com/gists", {
    method: "POST",
    headers,
    body: JSON.stringify({
      description: `GitSwamp snippet from ${fileName}`,
      public: true,
      files: {
        [fileName]: {
          content: selected,
        },
      },
    }),
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`);
  }

  const payload = await response.json() as Record<string, unknown>;
  const gistUrl = String(payload.html_url || payload.url || "").trim();
  if (!gistUrl) {
    throw new Error("Missing gist URL");
  }

  return gistUrl;
}

async function createPasteRsLink(selected: string): Promise<string | null> {
  const response = await fetch("https://paste.rs", {
    method: "POST",
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
    },
    body: selected,
  });

  if (!response.ok) {
    return null;
  }

  const link = (await response.text()).trim();
  if (!/^https?:\/\//i.test(link)) {
    return null;
  }

  return link;
}

// Virtualization for file-diff view
const visibleFileLines = computed(() => {
  const all = fullFileLines.value;
  if (!all.length) return [];
  
  const first = Math.max(0, Math.floor(scrollTop.value / LINE_HEIGHT) - OVERSCAN);
  const last = Math.min(all.length - 1, Math.ceil((scrollTop.value + containerHeight.value) / LINE_HEIGHT) + OVERSCAN);
  
  const result = [];
  for (let i = first; i <= last; i++) {
    result.push({ line: all[i], idx: i });
  }
  return result;
});

const totalFileHeight = computed(() => fullFileLines.value.length * LINE_HEIGHT);

interface FileDiffOverviewSegment {
  topPercent: number;
  heightPercent: number;
  type: "addition" | "deletion";
}

const showFileDiffOverview = computed(() => {
  if (viewMode.value !== "file-diff" || loadingFileContent.value) {
    return false;
  }

  const total = fullFileLines.value.length;
  if (total === 0) {
    return false;
  }

  return totalFileHeight.value > containerHeight.value * FILE_DIFF_OVERVIEW_MIN_VISIBLE_RATIO;
});

const fileDiffOverviewSegments = computed<FileDiffOverviewSegment[]>(() => {
  const lines = fullFileLines.value;
  const total = lines.length;
  if (total === 0) {
    return [];
  }

  const segments: FileDiffOverviewSegment[] = [];
  let idx = 0;

  while (idx < total) {
    const type = lines[idx].type;
    if (type !== "addition" && type !== "deletion") {
      idx += 1;
      continue;
    }

    const start = idx;
    while (idx < total && lines[idx].type === type) {
      idx += 1;
    }

    const length = idx - start;
    segments.push({
      topPercent: (start / total) * 100,
      heightPercent: Math.max(FILE_DIFF_OVERVIEW_MIN_MARKER_PERCENT, (length / total) * 100),
      type,
    });
  }

  return segments;
});

const fileDiffOverviewViewport = computed(() => {
  const totalHeight = totalFileHeight.value;
  const viewportHeight = containerHeight.value;

  if (totalHeight <= 0 || viewportHeight <= 0) {
    return {
      topPercent: 0,
      heightPercent: 100,
    };
  }

  const rawHeightPercent = (viewportHeight / totalHeight) * 100;
  const heightPercent = Math.min(100, Math.max(FILE_DIFF_OVERVIEW_MIN_VIEWPORT_PERCENT, rawHeightPercent));
  const maxScroll = Math.max(1, totalHeight - viewportHeight);
  const clampedScrollTop = Math.max(0, Math.min(scrollTop.value, maxScroll));
  const availableTrack = Math.max(0, 100 - heightPercent);
  const topPercent = availableTrack > 0
    ? (clampedScrollTop / maxScroll) * availableTrack
    : 0;

  return {
    topPercent,
    heightPercent,
  };
});

function syncFileDiffViewportMetrics() {
  const el = fileDiffScrollContainer.value;
  if (!el) {
    return;
  }

  scrollTop.value = el.scrollTop;
  containerHeight.value = el.clientHeight;
}

function onFileDiffScroll(event: Event) {
  const el = event.target as HTMLElement;
  scrollTop.value = el.scrollTop;
  containerHeight.value = el.clientHeight;
}

function jumpToFileDiffPosition(event: MouseEvent) {
  const el = fileDiffScrollContainer.value;
  if (!el || totalFileHeight.value <= 0) {
    return;
  }

  const target = event.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  const y = event.clientY - rect.top;
  const ratio = Math.max(0, Math.min(1, y / Math.max(1, rect.height)));
  const maxScroll = Math.max(0, el.scrollHeight - el.clientHeight);
  const nextScroll = Math.max(0, Math.min(maxScroll, ratio * totalFileHeight.value - el.clientHeight / 2));

  el.scrollTop = nextScroll;
  scrollTop.value = nextScroll;
}

function lineClass(type: string): string {
  switch (type) {
    case "addition": return "bg-[var(--diff-add-bg)]";
    case "deletion": return "bg-[var(--diff-del-bg)]";
    default: return "";
  }
}

function splitCellClass(type: "context" | "addition" | "deletion" | "empty"): string {
  switch (type) {
    case "addition":
      return "bg-[var(--diff-add-bg)] text-[var(--diff-add-fg)]";
    case "deletion":
      return "bg-[var(--diff-del-bg)] text-[var(--diff-del-fg)]";
    case "empty":
      return "opacity-45 text-[var(--muted-foreground)]";
    default:
      return "text-[var(--diff-text)]";
  }
}

function getSplitCellHtml(row: SplitDiffRow, side: "old" | "new"): string {
  const ref = side === "old" ? row.oldRef : row.newRef;
  const content = side === "old" ? row.oldContent : row.newContent;

  if (!ref || !content) {
    return "";
  }

  if (ref.line.line_type === "addition" || ref.line.line_type === "deletion") {
    return getHighlightedDiffLine(ref.hunkIdx, ref.lineIdx, ref.line);
  }

  return getHighlightedLine(content, `split:${side}:${ref.hunkIdx}:${ref.lineIdx}:${content}`);
}

function formatInlineBlameHeader(blame: InlineBlameEntry): string {
  const shaLabel = blame.entry.is_uncommitted ? "LOCAL" : blame.entry.short_sha;
  const authorLabel = (blame.entry.author || "Unknown").trim();
  return `${shaLabel} ${authorLabel}`.trim();
}

function formatInlineBlameMeta(blame: InlineBlameEntry): string {
  const when = formatBlameTimestamp(blame.entry.author_time);
  if (blame.groupSize > 1) {
    return `${when} • ${blame.groupSize} lines`;
  }

  return when;
}

function linePrefix(type: string): string {
  switch (type) {
    case "addition": return "+";
    case "deletion": return "-";
    default: return " ";
  }
}

function escapeHtml(value: string): string {
  return value
    .split("&").join("&amp;")
    .split("<").join("&lt;")
    .split(">").join("&gt;");
}

function getHighlightedLine(lineText: string, key: string): string {
  if (rawDiffFallbackActive.value) {
    return escapeHtml(lineText);
  }

  if (shouldAbortDiffColoring()) {
    activateRawDiffFallback();
    return escapeHtml(lineText);
  }

  const cached = highlightedLineCache.value.get(key);
  if (cached !== undefined) {
    return cached;
  }

  const shouldUsePlainText = usePlainTextHighlighting.value
    || isPlainTextFastPath.value
    || lineText.length > LONG_LINE_PLAIN_TEXT_THRESHOLD;
  const highlighted = shouldUsePlainText
    ? escapeHtml(lineText)
    : highlightCodeLine(lineText, props.filePath);
  highlightedLineCache.value.set(key, highlighted);
  maintainHighlightCache();
  return highlighted;
}

function getHighlightedDiffLine(hunkIdx: number, lineIdx: number, line: DiffLine): string {
  const lineText = line.content.replace(/\n$/, "");
  if (!inlineDiffEnabled.value) {
    const key = `${hunkIdx}:${lineIdx}:${line.line_type}:${lineText}`;
    return getHighlightedLine(lineText, key);
  }

  const pair = inlineDiffPairs.value.get(`${hunkIdx}:${lineIdx}`);
  if (pair) {
    const key = `${hunkIdx}:${lineIdx}:${line.line_type}:inline:${lineText}:${pair.compareText}`;
    const cached = highlightedLineCache.value.get(key);
    if (cached !== undefined) {
      return cached;
    }

    const inlineClass = line.line_type === "addition" ? "diff-inline-add" : "diff-inline-del";
    const html = buildInlineDiffMarkup(lineText, pair.compareText, inlineClass);
    highlightedLineCache.value.set(key, html);
    maintainHighlightCache();
    return html;
  }

  const key = `${hunkIdx}:${lineIdx}:${line.line_type}:${lineText}`;
  return getHighlightedLine(lineText, key);
}

function getHighlightedFileLine(rowIdx: number, line: FullFileLine): string {
  if (
    line.diffLine
    && line.hunkIdx !== null
    && line.hunkLineIdx !== null
    && (line.type === 'addition' || line.type === 'deletion')
  ) {
    return getHighlightedDiffLine(line.hunkIdx, line.hunkLineIdx, line.diffLine);
  }

  return getHighlightedLine(line.content, `file:${rowIdx}:${line.content}`);
}

function shouldStopInlineColoring(): boolean {
  if (rawDiffFallbackActive.value) {
    return true;
  }
  if (!shouldAbortDiffColoring()) {
    return false;
  }

  activateRawDiffFallback();
  return true;
}

interface InlineDiffBlock {
  deletionStart: number;
  additionStart: number;
  end: number;
}

function readInlineDiffBlock(lines: DiffLine[], startIndex: number): InlineDiffBlock | null {
  if (lines[startIndex]?.line_type !== "deletion") {
    return null;
  }

  let cursor = startIndex;
  const deletionStart = cursor;
  while (cursor < lines.length && lines[cursor].line_type === "deletion") {
    cursor += 1;
  }

  const additionStart = cursor;
  while (cursor < lines.length && lines[cursor].line_type === "addition") {
    cursor += 1;
  }

  return {
    deletionStart,
    additionStart,
    end: cursor,
  };
}

function applyInlinePairsForBlock(
  hunk: { lines: DiffLine[] },
  hunkIdx: number,
  block: InlineDiffBlock,
  pairs: Map<string, InlineDiffPair>,
): boolean {
  const deletionCount = block.additionStart - block.deletionStart;
  const additionCount = block.end - block.additionStart;
  if (deletionCount === 0 || additionCount === 0) {
    return true;
  }

  const deletionIndexes = Array.from({ length: deletionCount }, (_, offset) => block.deletionStart + offset);
  const additionIndexes = Array.from({ length: additionCount }, (_, offset) => block.additionStart + offset);
  const matchedPairs = collectInlineLinePairs(hunk, deletionIndexes, additionIndexes);

  for (const [delLineIdx, addLineIdx] of matchedPairs) {
    if (shouldStopInlineColoring()) {
      return false;
    }

    const deletionText = hunk.lines[delLineIdx].content.replace(/\n$/, "");
    const additionText = hunk.lines[addLineIdx].content.replace(/\n$/, "");
    pairs.set(`${hunkIdx}:${delLineIdx}`, { compareText: additionText });
    pairs.set(`${hunkIdx}:${addLineIdx}`, { compareText: deletionText });
  }

  return true;
}

function collectInlinePairsForHunk(
  hunk: { lines: DiffLine[] },
  hunkIdx: number,
  pairs: Map<string, InlineDiffPair>,
): boolean {
  let lineIdx = 0;
  while (lineIdx < hunk.lines.length) {
    if (shouldStopInlineColoring()) {
      return false;
    }

    const block = readInlineDiffBlock(hunk.lines, lineIdx);
    if (!block) {
      lineIdx += 1;
      continue;
    }

    const applied = applyInlinePairsForBlock(hunk, hunkIdx, block, pairs);
    if (!applied) {
      return false;
    }

    lineIdx = block.end;
  }

  return true;
}

const inlineDiffPairs = computed(() => {
  if (!diff.value || !inlineDiffEnabled.value || rawDiffFallbackActive.value) {
    return new Map<string, InlineDiffPair>();
  }

  if (shouldStopInlineColoring()) {
    return new Map<string, InlineDiffPair>();
  }

  const pairs = new Map<string, InlineDiffPair>();
  let aborted = false;

  for (const [hunkIdx, hunk] of diff.value.hunks.entries()) {
    if (shouldStopInlineColoring()) {
      aborted = true;
      break;
    }

    const collected = collectInlinePairsForHunk(hunk, hunkIdx, pairs);
    if (!collected) {
      aborted = true;
      break;
    }
  }

  if (aborted) {
    return new Map(pairs);
  }

  return pairs;
});

function buildInlineDiffMarkup(base: string, compare: string, changedClass: string): string {
  if (!base || base === compare) {
    return escapeHtml(base);
  }

  const tokenMarkup = buildTokenInlineMarkup(base, compare, changedClass);
  if (tokenMarkup) {
    return tokenMarkup;
  }

  const keepMask = computeLcsKeepMask(base, compare);
  if (!keepMask || keepMask.length !== base.length) {
    return buildPrefixSuffixInlineMarkup(base, compare, changedClass);
  }

  const { parts, hasChanged } = buildInlineSegmentsFromMask(base, keepMask, changedClass);

  if (!hasChanged) {
    return buildPrefixSuffixInlineMarkup(base, compare, changedClass);
  }

  return parts.join("");
}

function pushInlineSegment(parts: string[], value: string, changed: boolean, changedClass: string) {
  if (!value) return;
  if (changed) {
    parts.push(`<span class="${changedClass}">${escapeHtml(value)}</span>`);
    return;
  }
  parts.push(escapeHtml(value));
}

function buildInlineSegmentsFromMask(base: string, keepMask: boolean[], changedClass: string): { parts: string[]; hasChanged: boolean } {
  const baseChars = base.split("");
  const changedMask = bridgeWhitespaceRuns(
    baseChars,
    keepMask.map((keep) => !keep),
  );
  return buildInlineSegmentsFromTokens(baseChars, changedMask, changedClass);
}

function onEditInput() {
  hasUnsavedChanges.value = editContent.value !== fileContent.value;
}

watch(usePlainTextHighlighting, () => {
  highlightedLineCache.value.clear();
});
</script>

<template>
  <div class="h-full flex flex-col bg-[var(--card)] overflow-hidden">
    <div class="flex items-center justify-between px-4 py-2.5 border-b border-[var(--border)] bg-[var(--card)] flex-shrink-0">
      <div class="flex items-center gap-3 min-w-0">
        <Pencil v-if="isWorkingChanges" class="w-4 h-4 text-[var(--muted-foreground)] flex-shrink-0" />
        <FileText v-else class="w-4 h-4 text-[var(--muted-foreground)] flex-shrink-0" />
        <div class="min-w-0" :title="filePath">
          <div class="text-sm font-semibold text-[var(--foreground)] truncate">{{ fileNameParts.fileName }}</div>
          <div class="text-[10px] text-[var(--muted-foreground)] truncate">{{ fileNameParts.directory || '.' }}</div>
        </div>
        <span v-if="diff?.old_path" class="text-xs text-[var(--muted-foreground)] flex-shrink-0">
          (from {{ diff.old_path }})
        </span>
        <span v-if="hasUnsavedChanges" class="text-xs text-[var(--destructive)] flex-shrink-0">● Unsaved</span>
      </div>
      
      <div class="flex items-center gap-2 flex-shrink-0">
        <div class="flex items-center gap-1.5">
          <template v-if="viewMode === 'edit'">
            <button
              @click="saveFile"
              :disabled="!hasUnsavedChanges || saving"
              class="flex items-center gap-1 px-2 py-1 text-xs font-medium rounded bg-[var(--primary)] hover:opacity-90 text-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <Save class="w-3.5 h-3.5" />
              Save
            </button>
            <button
              @click="discardEditChanges"
              :disabled="!hasUnsavedChanges"
              class="flex items-center gap-1 px-2 py-1 text-xs font-medium rounded border border-[var(--diff-border)] bg-[var(--secondary)] hover:opacity-85 text-[var(--foreground)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <RotateCcw class="w-3.5 h-3.5" />
              Discard
            </button>
          </template>

          <div v-if="(viewMode === 'diff' || viewMode === 'split-diff') && diff && diff.hunks.length > 1" class="flex items-center gap-0.5">
            <button
              @click="navigateHunk('prev')"
              :disabled="currentHunkIndex === 0"
              class="p-1 rounded hover:bg-[var(--secondary)] disabled:opacity-30 disabled:cursor-not-allowed"
            >
              <ChevronUp class="w-4 h-4" />
            </button>
            <span class="text-[10px] text-[var(--muted-foreground)] min-w-[2.5rem] text-center">
              {{ currentHunkIndex + 1 }}/{{ diff.hunks.length }}
            </span>
            <button
              @click="navigateHunk('next')"
              :disabled="currentHunkIndex === diff.hunks.length - 1"
              class="p-1 rounded hover:bg-[var(--secondary)] disabled:opacity-30 disabled:cursor-not-allowed"
            >
              <ChevronDown class="w-4 h-4" />
            </button>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <div class="flex bg-[var(--secondary)] rounded-md overflow-hidden">
            <button
              @click="viewMode = 'diff'"
              :class="[
                'px-2 py-1.5 transition-colors',
                viewMode === 'diff' 
                  ? 'bg-[var(--primary)] text-white' 
                  : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
              ]"
              title="Diff view"
            >
              <FileText class="w-3.5 h-3.5" />
            </button>
            <button
              @click="viewMode = 'split-diff'"
              :class="[
                'px-2 py-1.5 transition-colors',
                viewMode === 'split-diff' 
                  ? 'bg-[var(--primary)] text-white' 
                  : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
              ]"
              title="Split view"
            >
              <Columns2 class="w-3.5 h-3.5" />
            </button>
            <button
              @click="viewMode = 'file-diff'"
              :class="[
                'px-2 py-1.5 transition-colors',
                viewMode === 'file-diff' 
                  ? 'bg-[var(--primary)] text-white' 
                  : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
              ]"
              title="File view"
            >
              <Eye class="w-3.5 h-3.5" />
            </button>
            <button
              @click="viewMode = 'time-lapse'"
              :class="[
                'px-2 py-1.5 transition-colors',
                viewMode === 'time-lapse'
                  ? 'bg-[var(--primary)] text-white'
                  : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
              ]"
              title="Play visual history of this file"
            >
              <Play class="w-3.5 h-3.5" />
            </button>
            <button
              v-if="isWorkingChanges"
              @click="viewMode = 'edit'"
              :class="[
                'px-2 py-1.5 transition-colors',
                viewMode === 'edit' 
                  ? 'bg-[var(--primary)] text-white' 
                  : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
              ]"
              title="Edit file"
            >
              <Edit3 class="w-3.5 h-3.5" />
            </button>
          </div>

          <button
            @click="toggleBlamePanel"
            :disabled="!blameSupportedView || !!diff?.is_binary"
            :class="[
              'px-2 py-1 text-[11px] rounded border transition-colors',
              showBlamePanel
                ? 'border-[var(--primary)] bg-[var(--primary)] text-white'
                : 'border-[var(--diff-border)] bg-[var(--secondary)] text-[var(--foreground)] hover:opacity-85',
              (!blameSupportedView || !!diff?.is_binary) ? 'opacity-50 cursor-not-allowed hover:opacity-50' : ''
            ]"
            title="Toggle blame panel"
          >
            Blame
          </button>

          <button
            class="px-2 py-1 text-[11px] rounded border border-[var(--diff-border)] bg-[var(--secondary)] hover:opacity-85 text-[var(--foreground)] transition-colors"
            title="Find who knows this file best"
            @click="runExpertAdvisor"
          >
            <Users class="w-3.5 h-3.5 inline mr-1" />
            Expert
          </button>

          <button
            class="px-2 py-1 text-[11px] rounded border border-[var(--diff-border)] bg-[var(--secondary)] hover:opacity-85 text-[var(--foreground)] transition-colors"
            title="Share selected code as gist snippet"
            @click="shareSelectionAsSnippet"
          >
            <Share2 class="w-3.5 h-3.5 inline mr-1" />
            Share Snippet
          </button>
        </div>

        <button
          @click="emit('close')"
          class="p-1 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
        >
          <X class="w-5 h-5" />
        </button>
      </div>
    </div>

    <div class="flex-1 min-h-0 flex bg-[var(--diff-bg)] font-mono text-[12px] leading-[1.4]">
      <div class="flex-1 min-w-0 overflow-auto">
      <div v-if="loading" class="flex items-center justify-center h-full">
        <div class="diff-loader-shell">
          <img :src="logoCrocGif" alt="Loading" class="diff-loader-logo" />
          <div class="diff-loader-wave" aria-label="Loading">
            <span
              v-for="(letter, idx) in loadingLetters"
              :key="letter + idx"
              :style="{ animationDelay: `${idx * 0.08}s` }"
            >
              {{ letter }}
            </span>
          </div>
          <p class="diff-loader-caption">{{ loadingLabel }}...</p>
        </div>
      </div>

      <div v-else-if="error" class="flex flex-col items-center justify-center h-full gap-2">
        <div class="text-red-400">{{ error }}</div>
        <button @click="reload" class="text-xs text-[var(--diff-link)] hover:underline">Retry</button>
      </div>

      <div v-else-if="diff?.is_binary" class="flex items-center justify-center h-full">
        <div class="text-[var(--muted-foreground)]">Binary file - cannot display diff</div>
      </div>

      <div v-else-if="viewMode === 'diff' && diff" class="min-w-fit">
        <div
          v-if="rawDiffFallbackActive"
          class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-[var(--secondary)]/90 text-[var(--muted-foreground)]"
        >
          {{ rawDiffFallbackReason || 'Coloring exceeded 5 seconds. Showing regular Git diff.' }}
        </div>

        <div
          v-if="showBlamePanel && blameLoading"
          class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-[var(--secondary)]/90 text-[var(--muted-foreground)]"
        >
          Loading blame data...
        </div>

        <div
          v-if="showBlamePanel && !blameLoading && blameError"
          class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-red-500/10 text-red-300 flex items-center gap-2"
        >
          <span class="truncate">Blame error: {{ blameError }}</span>
          <button @click="refreshBlame" class="text-[var(--diff-link)] hover:underline">Retry</button>
        </div>

        <div
          v-if="usePlainTextHighlighting"
          class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-[var(--secondary)]/90 text-[var(--muted-foreground)]"
        >
          Large diff mode: syntax coloring is simplified to keep scrolling auto.
        </div>

        <template v-for="(hunk, hunkIdx) in diff.hunks" :key="hunkIdx">
          <div :id="`hunk-${hunkIdx}`" class="flex items-center justify-between bg-[var(--diff-hunk-bg)] px-3 py-1.5 sticky top-0 z-10 border-y border-[var(--diff-border)]">
            <span class="text-xs text-[var(--diff-link)]">
              @@ -{{ hunk.old_start }},{{ hunk.old_lines }} +{{ hunk.new_start }},{{ hunk.new_lines }} @@
            </span>
            <button
              v-if="isWorkingChanges"
              @click="revertHunk(hunkIdx)"
              class="flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded border border-[var(--diff-border)] bg-[var(--secondary)] hover:opacity-85 text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors"
            >
              <Undo2 class="w-3 h-3" />
              Revert Hunk
            </button>
          </div>

          <div>
            <template v-for="(line, lineIdx) in hunk.lines" :key="lineIdx">
              <div :class="['flex', lineClass(line.line_type)]">
                <div
                  v-if="showBlamePanel"
                  class="w-[230px] flex-shrink-0 px-2 py-0.5 border-r border-[var(--diff-border)] bg-[var(--secondary)]/35 overflow-hidden"
                >
                  <template v-if="getInlineBlame(getDiffRowBlameLineNo(line))?.showHeader">
                    <div class="text-[10px] font-medium text-[var(--foreground)] truncate">
                      {{ formatInlineBlameHeader(getInlineBlame(getDiffRowBlameLineNo(line))!) }}
                    </div>
                    <div class="text-[10px] text-[var(--muted-foreground)] truncate">
                      {{ formatInlineBlameMeta(getInlineBlame(getDiffRowBlameLineNo(line))!) }}
                    </div>
                  </template>
                </div>
                <div class="diff-line-no w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px]">
                  {{ line.old_line_no ?? '' }}
                </div>
                <div class="diff-line-no w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px]">
                  {{ line.new_line_no ?? '' }}
                </div>
                <div
                  class="w-5 flex-shrink-0 text-center select-none font-bold"
                  :class="line.line_type === 'addition' ? 'text-[var(--diff-sign-add)]' : line.line_type === 'deletion' ? 'text-[var(--diff-sign-del)]' : 'text-[var(--diff-sign-neutral)]'"
                >
                  {{ linePrefix(line.line_type) }}
                </div>
                <pre
                  class="diff-code-line flex-1 px-1.5 whitespace-pre overflow-x-auto select-text"
                  :class="line.line_type === 'addition' ? 'text-[var(--diff-add-fg)]' : line.line_type === 'deletion' ? 'text-[var(--diff-del-fg)]' : 'text-[var(--diff-text)]'"
                ><code class="hljs bg-transparent" v-html="getHighlightedDiffLine(hunkIdx, lineIdx, line)"></code></pre>
              </div>
            </template>
          </div>
        </template>

        <div v-if="diff.hunks.length === 0" class="flex items-center justify-center h-64 text-[var(--muted-foreground)]">
          No changes in this file
        </div>
      </div>

      <div v-else-if="viewMode === 'split-diff' && diff" class="min-w-fit">
        <div
          v-if="rawDiffFallbackActive"
          class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-[var(--secondary)]/90 text-[var(--muted-foreground)]"
        >
          {{ rawDiffFallbackReason || 'Coloring exceeded 5 seconds. Showing regular Git diff.' }}
        </div>

        <div
          v-if="showBlamePanel && blameLoading"
          class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-[var(--secondary)]/90 text-[var(--muted-foreground)]"
        >
          Loading blame data...
        </div>

        <div
          v-if="showBlamePanel && !blameLoading && blameError"
          class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-red-500/10 text-red-300 flex items-center gap-2"
        >
          <span class="truncate">Blame error: {{ blameError }}</span>
          <button @click="refreshBlame" class="text-[var(--diff-link)] hover:underline">Retry</button>
        </div>

        <div
          v-if="usePlainTextHighlighting"
          class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-[var(--secondary)]/90 text-[var(--muted-foreground)]"
        >
          Large diff mode: syntax coloring is simplified to keep scrolling auto.
        </div>

        <div
          :class="[
            'split-column-head grid border-b border-[var(--diff-border)] bg-[var(--secondary)]/70 text-[10px] uppercase tracking-[0.06em] text-[var(--muted-foreground)]',
            showBlamePanel ? 'grid-cols-[230px_minmax(0,1fr)_minmax(0,1fr)]' : 'grid-cols-2'
          ]"
        >
          <div v-if="showBlamePanel" class="px-2 py-1.5 border-r border-[var(--diff-border)]">Blame</div>
          <div class="px-3 py-1.5 border-r border-[var(--diff-border)]">Previous Version</div>
          <div class="px-3 py-1.5">New Version</div>
        </div>

        <template v-for="(hunk, hunkIdx) in diff.hunks" :key="hunkIdx">
          <div :id="`hunk-${hunkIdx}`" class="flex items-center justify-between bg-[var(--diff-hunk-bg)] px-3 py-1.5 sticky top-0 z-10 border-y border-[var(--diff-border)]">
            <span class="text-xs text-[var(--diff-link)]">
              @@ -{{ hunk.old_start }},{{ hunk.old_lines }} +{{ hunk.new_start }},{{ hunk.new_lines }} @@
            </span>
            <button
              v-if="isWorkingChanges"
              @click="revertHunk(hunkIdx)"
              class="flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded border border-[var(--diff-border)] bg-[var(--secondary)] hover:opacity-85 text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors"
            >
              <Undo2 class="w-3 h-3" />
              Revert Hunk
            </button>
          </div>

          <div
            :class="[
              'border-b border-[var(--diff-border)] grid',
              showBlamePanel ? 'grid-cols-[230px_minmax(0,1fr)_minmax(0,1fr)]' : 'grid-cols-2'
            ]"
          >
            <div v-if="showBlamePanel" class="border-r border-[var(--diff-border)] bg-[var(--secondary)]/25">
              <div
                v-for="(row, rowIdx) in splitDiffRowsByHunk[hunkIdx]"
                :key="`${hunkIdx}-blame-${rowIdx}`"
                class="min-h-[18px] px-2 py-0.5 overflow-hidden"
              >
                <template v-if="getInlineBlame(getSplitRowBlameLineNo(row))?.showHeader">
                  <div class="text-[10px] font-medium text-[var(--foreground)] truncate">
                    {{ formatInlineBlameHeader(getInlineBlame(getSplitRowBlameLineNo(row))!) }}
                  </div>
                  <div class="text-[10px] text-[var(--muted-foreground)] truncate">
                    {{ formatInlineBlameMeta(getInlineBlame(getSplitRowBlameLineNo(row))!) }}
                  </div>
                </template>
              </div>
            </div>

            <div
              class="border-r border-[var(--diff-border)] overflow-x-auto overflow-y-hidden"
              :ref="(el) => setSplitPaneRef(hunkIdx, 'old', el)"
              @scroll="onSplitPaneScroll(hunkIdx, 'old', $event)"
            >
              <div class="min-w-max">
                <div
                  v-for="(row, rowIdx) in splitDiffRowsByHunk[hunkIdx]"
                  :key="`${hunkIdx}-old-${rowIdx}`"
                  :class="['flex min-h-[18px]', splitCellClass(row.oldType)]"
                >
                  <div class="diff-line-no w-11 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[10px] leading-[18px]">
                    {{ row.oldLineNo ?? '' }}
                  </div>
                  <pre class="diff-code-line flex-1 px-1.5 whitespace-pre overflow-hidden leading-[18px] m-0 select-text"><code class="hljs bg-transparent" v-html="getSplitCellHtml(row, 'old')"></code></pre>
                </div>
              </div>
            </div>

            <div
              class="overflow-x-auto overflow-y-hidden"
              :ref="(el) => setSplitPaneRef(hunkIdx, 'new', el)"
              @scroll="onSplitPaneScroll(hunkIdx, 'new', $event)"
            >
              <div class="min-w-max">
                <div
                  v-for="(row, rowIdx) in splitDiffRowsByHunk[hunkIdx]"
                  :key="`${hunkIdx}-new-${rowIdx}`"
                  :class="['flex min-h-[18px]', splitCellClass(row.newType)]"
                >
                  <div class="diff-line-no w-11 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[10px] leading-[18px]">
                    {{ row.newLineNo ?? '' }}
                  </div>
                  <pre class="diff-code-line flex-1 px-1.5 whitespace-pre overflow-hidden leading-[18px] m-0 select-text"><code class="hljs bg-transparent" v-html="getSplitCellHtml(row, 'new')"></code></pre>
                </div>
              </div>
            </div>
          </div>
        </template>

        <div v-if="diff.hunks.length === 0" class="flex items-center justify-center h-64 text-[var(--muted-foreground)]">
          No changes in this file
        </div>
      </div>

      <div v-else-if="viewMode === 'file-diff'" class="relative h-full min-w-0">
        <div
          ref="fileDiffScrollContainer"
          class="h-full min-w-fit overflow-auto pr-6"
          @scroll="onFileDiffScroll"
        >
          <div
            v-if="rawDiffFallbackActive"
            class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-[var(--secondary)]/90 text-[var(--muted-foreground)]"
          >
            {{ rawDiffFallbackReason || 'Coloring exceeded 5 seconds. Showing regular Git diff.' }}
          </div>

          <div v-if="loadingFileContent" class="flex items-center justify-center h-full">
            <div class="diff-loader-shell">
              <img :src="logoCrocGif" alt="Loading file content" class="diff-loader-logo" />
              <div class="diff-loader-wave" aria-label="Loading">
                <span
                  v-for="(letter, idx) in loadingLetters"
                  :key="'file-' + letter + idx"
                  :style="{ animationDelay: `${idx * 0.08}s` }"
                >
                  {{ letter }}
                </span>
              </div>
              <p class="diff-loader-caption">{{ loadingLabel }}...</p>
            </div>
          </div>
          <div v-else-if="fullFileLines.length > 0" class="h-full">
            <div
              v-if="usePlainTextHighlighting"
              class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-[var(--secondary)]/90 text-[var(--muted-foreground)]"
            >
              Large file mode: syntax coloring is simplified to reduce loading time.
            </div>
            <div class="relative" :style="{ height: totalFileHeight + 'px' }">
            <div
              v-for="item in visibleFileLines"
              :key="item.idx"
              :class="['flex absolute left-0 right-0', lineClass(item.line.type)]"
              :style="{ top: (item.idx * LINE_HEIGHT) + 'px', height: LINE_HEIGHT + 'px' }"
            >
              <div class="diff-line-no w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[10px] leading-[18px]">
                {{ item.line.oldLineNo ?? '' }}
              </div>
              <div class="diff-line-no w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[10px] leading-[18px]">
                {{ item.line.lineNo || '' }}
              </div>
              <div
                class="w-5 flex-shrink-0 text-center select-none font-bold leading-[20px]"
                :class="item.line.type === 'addition' ? 'text-[var(--diff-sign-add)]' : item.line.type === 'deletion' ? 'text-[var(--diff-sign-del)]' : 'text-[var(--diff-sign-neutral)]'"
              >
                {{ linePrefix(item.line.type) }}
              </div>
              <pre
                class="diff-code-line flex-1 px-1.5 whitespace-pre-wrap break-all overflow-hidden leading-[18px] m-0 select-text"
                :class="item.line.type === 'addition' ? 'text-[var(--diff-add-fg)]' : item.line.type === 'deletion' ? 'text-[var(--diff-del-fg)]' : 'text-[var(--diff-text)]'"
              ><code class="hljs bg-transparent" v-html="getHighlightedFileLine(item.idx, item.line)"></code></pre>
            </div>
            </div>
          </div>
          <div v-else-if="!loading && !loadingFileContent" class="flex items-center justify-center h-64 text-[var(--muted-foreground)]">
            No file content available
          </div>
        </div>

        <button
          v-if="showFileDiffOverview"
          class="file-diff-overview"
          type="button"
          title="Jump through changes"
          @click="jumpToFileDiffPosition"
        >
          <span
            v-for="(segment, idx) in fileDiffOverviewSegments"
            :key="`overview-${idx}`"
            class="file-diff-overview-segment"
            :class="segment.type === 'addition' ? 'file-diff-overview-segment-add' : 'file-diff-overview-segment-del'"
            :style="{ top: `${segment.topPercent}%`, height: `${segment.heightPercent}%` }"
          />
          <span
            class="file-diff-overview-viewport"
            :style="{ top: `${fileDiffOverviewViewport.topPercent}%`, height: `${fileDiffOverviewViewport.heightPercent}%` }"
          />
        </button>
      </div>

      <div v-else-if="viewMode === 'edit'" class="min-w-fit h-full">
        <textarea
          v-model="editContent"
          @input="onEditInput"
          class="w-full h-full bg-[var(--diff-bg)] text-[var(--diff-text)] p-3 resize-none outline-none font-mono text-[12px] leading-[1.4]"
          spellcheck="false"
        ></textarea>
      </div>

      <div v-else-if="viewMode === 'time-lapse'" class="h-full flex flex-col overflow-hidden">
        <div class="px-3 py-2 border-b border-[var(--diff-border)] bg-[var(--secondary)]/40 flex items-center justify-between gap-2">
          <div class="flex items-center gap-2 flex-wrap min-w-0">
            <div class="text-[11px] text-[var(--muted-foreground)]">
              File evolution timeline (recent frames)
            </div>
            <label class="flex items-center gap-1 text-[10px] text-[var(--muted-foreground)]">
              Commits back
              <select
                v-model.number="timeLapseCommitWindow"
                class="h-6 px-1.5 rounded border border-[var(--diff-border)] bg-[var(--card)] text-[var(--foreground)] text-[10px] focus:outline-none"
                title="How many commits back to scan"
              >
                <option v-for="count in timeLapseCommitWindowOptions" :key="count" :value="count">{{ count }}</option>
              </select>
            </label>
          </div>
          <div class="flex items-center gap-1">
            <button
              class="p-1 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
              title="Previous frame"
              @click="previousTimeLapseFrame"
            >
              <StepBack class="w-4 h-4" />
            </button>
            <button
              class="p-1 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
              :title="timeLapsePlaying ? 'Pause' : 'Play'"
              @click="toggleTimeLapsePlay"
            >
              <Pause v-if="timeLapsePlaying" class="w-4 h-4" />
              <Play v-else class="w-4 h-4" />
            </button>
            <button
              class="p-1 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
              title="Next frame"
              @click="nextTimeLapseFrame"
            >
              <StepForward class="w-4 h-4" />
            </button>
          </div>
        </div>

        <div v-if="timeLapseLoading" class="flex-1 flex items-center justify-center">
          <div class="time-lapse-loader-shell">
            <img :src="logoCrocLoadingGif" alt="Loading timeline" class="time-lapse-loader-logo" />
            <p class="diff-loader-caption">Loading file timeline...</p>
          </div>
        </div>

        <div v-else-if="!activeTimeLapseFrame" class="flex-1 flex items-center justify-center text-[var(--muted-foreground)] text-sm">
          No timeline frames available for this file in recent commits.
        </div>

        <div v-else class="flex-1 flex flex-col min-h-0">
          <div class="px-3 py-2 border-b border-[var(--diff-border)] bg-[var(--card)]">
            <div class="text-xs text-[var(--foreground)] font-semibold truncate">
              {{ activeTimeLapseFrame.message }}
            </div>
            <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5">
              {{ activeTimeLapseFrame.shortSha }} by {{ activeTimeLapseFrame.author }} • {{ formatTimeLapseTimestamp(activeTimeLapseFrame.timestamp) }}
            </div>
          </div>
          <div class="flex-1 overflow-auto">
            <div v-if="activeTimeLapseRenderLines.length === 0" class="h-full flex items-center justify-center text-[var(--muted-foreground)] text-sm">
              No timeline content available for this frame.
            </div>
            <div v-else class="font-mono text-[12px] leading-[1.4] min-w-fit">
              <div
                v-for="line in activeTimeLapseRenderLines"
                :key="line.key"
                class="flex"
                :class="line.type === 'addition'
                  ? 'bg-[var(--diff-add-bg)] text-[var(--diff-add-fg)]'
                  : line.type === 'deletion'
                    ? 'bg-[var(--diff-del-bg)] text-[var(--diff-del-fg)]'
                    : 'text-[var(--diff-text)]'"
              >
                <div class="diff-line-no w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px]">
                  {{ line.oldLineNo ?? '' }}
                </div>
                <div class="diff-line-no w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px]">
                  {{ line.newLineNo ?? '' }}
                </div>
                <div
                  class="w-5 flex-shrink-0 text-center select-none font-bold"
                  :class="line.type === 'addition' ? 'text-[var(--diff-sign-add)]' : line.type === 'deletion' ? 'text-[var(--diff-sign-del)]' : 'text-[var(--diff-sign-neutral)]'"
                >
                  {{ linePrefix(line.type) }}
                </div>
                <pre class="diff-code-line flex-1 px-1.5 whitespace-pre-wrap break-all overflow-hidden m-0 select-text"><code class="hljs bg-transparent">{{ line.content }}</code></pre>
              </div>
            </div>
          </div>
          <div class="px-3 py-1.5 border-t border-[var(--diff-border)] bg-[var(--secondary)]/40 text-[10px] text-[var(--muted-foreground)]">
            Frame {{ Math.min(timeLapseFrameIndex + 1, timeLapseFrames.length) }} / {{ timeLapseFrames.length }} • scanned last {{ timeLapseCommitWindowSafe }} commits
          </div>
        </div>
      </div>

      </div>
    </div>
  </div>
</template>

<style scoped>
.diff-loader-shell {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.55rem;
  padding: 1.1rem 1.25rem;
  border: 1px solid var(--diff-border);
  border-radius: 0.85rem;
  background: color-mix(in srgb, var(--card) 85%, transparent);
  box-shadow: 0 10px 30px color-mix(in srgb, var(--foreground) 7%, transparent);
}

.diff-loader-logo {
  width: 68px;
  height: 68px;
  object-fit: contain;
  filter: drop-shadow(0 6px 12px color-mix(in srgb, var(--foreground) 18%, transparent));
  animation: loaderFloat 1.35s ease-in-out infinite;
}

.diff-loader-wave {
  display: flex;
  align-items: baseline;
  gap: 0.04rem;
  font-size: 1rem;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: var(--foreground);
}

.diff-loader-wave span {
  display: inline-block;
  animation: loaderWave 1s ease-in-out infinite;
}

.diff-loader-caption {
  margin: 0;
  font-size: 11px;
  color: var(--muted-foreground);
}

.time-lapse-loader-shell {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.45rem;
}

.time-lapse-loader-logo {
  width: 54px;
  height: 54px;
  object-fit: contain;
  filter: drop-shadow(0 4px 10px color-mix(in srgb, var(--foreground) 16%, transparent));
}

@keyframes loaderFloat {
  0%,
  100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-5px);
  }
}

@keyframes loaderWave {
  0%,
  60%,
  100% {
    transform: translateY(0);
    opacity: 0.55;
  }
  30% {
    transform: translateY(-4px);
    opacity: 1;
  }
}

:global(body.gitswamp-reduced-motion) .diff-loader-logo,
:global(body.gitswamp-reduced-motion) .diff-loader-wave span {
  animation: none;
}

/* Performance hints */
.diff-code-line {
  contain: layout style paint;
}

:deep(.diff-inline-add) {
  background: color-mix(in srgb, var(--diff-inline-add-bg) 42%, transparent);
  border: 0;
  border-radius: 2px;
  color: var(--diff-inline-add-text);
  font-weight: inherit;
  font-style: normal;
  box-shadow:
    inset 0 -1px 0 color-mix(in srgb, var(--diff-sign-add) 18%, transparent),
    0 0 4px color-mix(in srgb, var(--diff-sign-add) 12%, transparent);
  text-decoration: none;
}

:deep(.diff-inline-del) {
  background: color-mix(in srgb, var(--diff-inline-del-bg) 42%, transparent);
  border: 0;
  border-radius: 2px;
  color: var(--diff-inline-del-text);
  font-weight: inherit;
  font-style: normal;
  box-shadow:
    inset 0 -1px 0 color-mix(in srgb, var(--diff-sign-del) 18%, transparent),
    0 0 4px color-mix(in srgb, var(--diff-sign-del) 12%, transparent);
  text-decoration: none;
}

.file-diff-overview {
  position: absolute;
  top: 14px;
  right: 10px;
  width: 28px;
  height: 136px;
  border: 1px solid color-mix(in srgb, var(--diff-border) 78%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--card) 88%, transparent);
  box-shadow: 0 8px 18px color-mix(in srgb, var(--foreground) 12%, transparent);
  overflow: hidden;
  cursor: pointer;
  z-index: 32;
}

.file-diff-overview-segment {
  position: absolute;
  left: 4px;
  right: 4px;
  border-radius: 999px;
}

.file-diff-overview-segment-add {
  background: color-mix(in srgb, var(--diff-sign-add) 80%, var(--card) 20%);
}

.file-diff-overview-segment-del {
  background: color-mix(in srgb, var(--diff-sign-del) 82%, var(--card) 18%);
}

.file-diff-overview-viewport {
  position: absolute;
  left: 2px;
  right: 2px;
  border-radius: 6px;
  border: 1px solid color-mix(in srgb, var(--diff-link) 72%, white 28%);
  background: color-mix(in srgb, var(--primary) 16%, transparent);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--foreground) 8%, transparent);
}
</style>


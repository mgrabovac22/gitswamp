<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { X, FileText, Pencil, ChevronUp, ChevronDown, Undo2, Eye, Edit3, Save, RotateCcw, Play, Pause, StepBack, StepForward, Share2, Users } from "lucide-vue-next";
import type { FileDiff, DiffLine, CommitInfo, CommitFileInfo } from "@/types";
import { highlightCodeLine, splitFilePath } from "@/shared/codeView";
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

type ViewMode = "diff" | "file-diff" | "edit" | "time-lapse";
const viewMode = ref<ViewMode>("diff");
const loadingLetters = ["L", "o", "a", "d", "i", "n", "g"];
const MAX_HIGHLIGHT_LINES = 6000;
const MAX_HIGHLIGHT_CHARS = 450000;

const diff = ref<FileDiff | null>(null);
const fileContent = ref<string>("");
const editContent = ref<string>("");
const loading = ref(true);
const loadingFileContent = ref(false);
const error = ref<string | null>(null);
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
}

const timeLapseFrames = ref<TimeLapseFrame[]>([]);
const timeLapseLoading = ref(false);
const timeLapsePlaying = ref(false);
const timeLapseFrameIndex = ref(0);
let timeLapseTimer: ReturnType<typeof setInterval> | null = null;
const TIME_LAPSE_MAX_FRAMES = 22;

// Virtualization state
const scrollTop = ref(0);
const containerHeight = ref(400);
const LINE_HEIGHT = 20; // pixels per line
const OVERSCAN = 10;

let fileWatchInterval: ReturnType<typeof setInterval> | null = null;
let lastFileHash = "";

const isWorkingChanges = computed(() => !props.commitSha);
const isUnstaged = computed(() => isWorkingChanges.value && !props.staged);
const fileNameParts = computed(() => splitFilePath(props.filePath));

interface InlineDiffPair {
  compareText: string;
}

const INLINE_PAIR_MIN_SCORE = 0.25;
const INLINE_LCS_MAX_CELLS = 220000;

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

watch(
  () => [props.filePath, props.staged, props.commitSha],
  () => {
    currentHunkIndex.value = 0;
    reload();
  }
);

watch(viewMode, (mode) => {
  if (mode !== "time-lapse") {
    stopTimeLapsePlayback();
  }

  if (mode === "edit") {
    loadFileForEdit();
  } else if (mode === "file-diff") {
    loadFileContentAsync();
  } else if (mode === "time-lapse") {
    void loadTimeLapseFrames();
  } else {
    reload();
  }
});

async function reload() {
  if (viewMode.value === "edit") return;
  
  loading.value = true;
  error.value = null;
  
  try {
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

    highlightedLineCache.value.clear();
    
    // Don't load file content synchronously - do it lazily
    if (viewMode.value === "file-diff") {
      await loadFileContentAsync();
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function loadFileContentAsync() {
  if (loadingFileContent.value) return;
  loadingFileContent.value = true;
  
  try {
    // Use requestAnimationFrame to prevent blocking
    await new Promise(resolve => requestAnimationFrame(resolve));
    
    fileContent.value = await loadDisplayedFileContent();
    
    // Allow UI to update
    await nextTick();
    await new Promise(resolve => requestAnimationFrame(resolve));
  } catch (e) {
    error.value = String(e);
  } finally {
    loadingFileContent.value = false;
  }
}

async function loadDisplayedFileContent(): Promise<string> {
  if (props.commitSha) {
    return invoke<string>("get_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
      sha: props.commitSha,
    });
  }

  if (props.staged) {
    return invoke<string>("get_staged_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
    });
  }

  return invoke<string>("get_file_content", {
    path: props.repoPath,
    filePath: props.filePath,
    sha: null,
  });
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
    if (!isUnstaged.value || viewMode.value === "edit") return;
    
    try {
      const newDiff = await invoke<FileDiff>("get_working_diff", {
        path: props.repoPath,
        filePath: props.filePath,
        staged: false,
      });
      
      const newHash = JSON.stringify(
        newDiff.hunks.map((hunk) => ({
          oldStart: hunk.old_start,
          newStart: hunk.new_start,
          lines: hunk.lines.map((line) => [line.line_type, line.old_line_no, line.new_line_no, line.content]),
        })),
      );
      if (newHash !== lastFileHash) {
        lastFileHash = newHash;
        diff.value = newDiff;
        highlightedLineCache.value.clear();
        if (viewMode.value === "file-diff") {
          fileContent.value = await loadDisplayedFileContent();
        }
      }
    } catch {
    }
  }, 1000);
}

function stopFileWatch() {
  if (fileWatchInterval) {
    clearInterval(fileWatchInterval);
    fileWatchInterval = null;
  }
}

watch(isUnstaged, (val) => {
  if (val) {
    startFileWatch();
  } else {
    stopFileWatch();
  }
}, { immediate: true });

onMounted(() => {
  reload();
  if (isUnstaged.value) {
    startFileWatch();
  }
});

onUnmounted(() => {
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
  el?.scrollIntoView({ behavior: "smooth", block: "start" });
}

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
  if (viewMode.value === "file-diff") {
    return fullFileLines.value.length > MAX_HIGHLIGHT_LINES || fileContent.value.length > MAX_HIGHLIGHT_CHARS;
  }

  return diffPayload.value.lines > MAX_HIGHLIGHT_LINES || diffPayload.value.chars > MAX_HIGHLIGHT_CHARS;
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
      maxCount: 180,
    });

    const normalizedPath = props.filePath.replace(/\\/g, "/").trim();
    const frames: TimeLapseFrame[] = [];
    let previousContent = "";

    for (const commit of commits) {
      if (frames.length >= TIME_LAPSE_MAX_FRAMES) break;

      let commitFiles: CommitFileInfo[] = [];
      try {
        commitFiles = await invoke<CommitFileInfo[]>("get_commit_files", {
          path: props.repoPath,
          sha: commit.sha,
        });
      } catch {
        continue;
      }

      const changedTarget = commitFiles.some((file) => file.path.replace(/\\/g, "/").trim() === normalizedPath);
      if (!changedTarget) continue;

      try {
        const content = await invoke<string>("get_file_content", {
          path: props.repoPath,
          filePath: props.filePath,
          sha: commit.sha,
        });

        if (content === previousContent && frames.length > 0) {
          continue;
        }

        frames.push({
          sha: commit.sha,
          shortSha: commit.short_sha,
          author: commit.author_name,
          message: commit.message.split("\n")[0],
          timestamp: commit.timestamp,
          content,
        });
        previousContent = content;
      } catch {
      }
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

function onFileDiffScroll(e: Event) {
  const el = e.target as HTMLElement;
  scrollTop.value = el.scrollTop;
  containerHeight.value = el.clientHeight;
}

function lineClass(type: string): string {
  switch (type) {
    case "addition": return "bg-[var(--diff-add-bg)]";
    case "deletion": return "bg-[var(--diff-del-bg)]";
    default: return "";
  }
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
  const cached = highlightedLineCache.value.get(key);
  if (cached !== undefined) {
    return cached;
  }

  const highlighted = usePlainTextHighlighting.value
    ? escapeHtml(lineText)
    : highlightCodeLine(lineText, props.filePath);
  highlightedLineCache.value.set(key, highlighted);
  return highlighted;
}

function getHighlightedDiffLine(hunkIdx: number, lineIdx: number, line: DiffLine): string {
  const lineText = line.content.replace(/\n$/, "");
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

const inlineDiffPairs = computed(() => {
  const pairs = new Map<string, InlineDiffPair>();
  if (!diff.value) {
    return pairs;
  }

  diff.value.hunks.forEach((hunk, hunkIdx) => {
    let lineIdx = 0;
    while (lineIdx < hunk.lines.length) {
      if (hunk.lines[lineIdx].line_type !== "deletion") {
        lineIdx += 1;
        continue;
      }

      const deletionStart = lineIdx;
      while (lineIdx < hunk.lines.length && hunk.lines[lineIdx].line_type === "deletion") {
        lineIdx += 1;
      }

      const additionStart = lineIdx;
      while (lineIdx < hunk.lines.length && hunk.lines[lineIdx].line_type === "addition") {
        lineIdx += 1;
      }

      const deletionCount = additionStart - deletionStart;
      const additionCount = lineIdx - additionStart;
      if (deletionCount === 0 || additionCount === 0) {
        continue;
      }

      const deletionIndexes = Array.from({ length: deletionCount }, (_, offset) => deletionStart + offset);
      const additionIndexes = Array.from({ length: additionCount }, (_, offset) => additionStart + offset);
      const matchedPairs = collectInlineLinePairs(hunk, deletionIndexes, additionIndexes);
      for (const [delLineIdx, addLineIdx] of matchedPairs) {
        const deletionText = hunk.lines[delLineIdx].content.replace(/\n$/, "");
        const additionText = hunk.lines[addLineIdx].content.replace(/\n$/, "");
        pairs.set(`${hunkIdx}:${delLineIdx}`, { compareText: additionText });
        pairs.set(`${hunkIdx}:${addLineIdx}`, { compareText: deletionText });
      }
    }
  });

  return pairs;
});

function buildInlineDiffMarkup(base: string, compare: string, changedClass: string): string {
  if (!base || base === compare) {
    return escapeHtml(base);
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
  const parts: string[] = [];
  let segment = "";
  let segmentChanged = !keepMask[0];
  let hasChanged = segmentChanged;

  for (let idx = 0; idx < base.length; idx += 1) {
    const changed = !keepMask[idx];
    if (idx > 0 && changed !== segmentChanged) {
      pushInlineSegment(parts, segment, segmentChanged, changedClass);
      segment = "";
      segmentChanged = changed;
    }

    segment += base[idx];
    if (changed) {
      hasChanged = true;
    }
  }

  pushInlineSegment(parts, segment, segmentChanged, changedClass);
  return { parts, hasChanged };
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
        <div class="flex bg-[var(--secondary)] rounded-md overflow-hidden">
          <button
            @click="viewMode = 'diff'"
            :class="[
              'px-2.5 py-1 text-xs font-medium transition-colors',
              viewMode === 'diff' 
                ? 'bg-[var(--primary)] text-white' 
                : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
            ]"
            title="View diff hunks only"
          >
            Diff
          </button>
          <button
            @click="viewMode = 'file-diff'"
            :class="[
              'px-2.5 py-1 text-xs font-medium transition-colors',
              viewMode === 'file-diff' 
                ? 'bg-[var(--primary)] text-white' 
                : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
            ]"
            title="View whole file with changes"
          >
            <Eye class="w-3.5 h-3.5 inline mr-1" />
            File
          </button>
          <button
            @click="viewMode = 'time-lapse'"
            :class="[
              'px-2.5 py-1 text-xs font-medium transition-colors',
              viewMode === 'time-lapse'
                ? 'bg-[var(--primary)] text-white'
                : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
            ]"
            title="Play visual history of this file"
          >
            <Play class="w-3.5 h-3.5 inline mr-1" />
            Time-Lapse
          </button>
          <button
            v-if="isWorkingChanges"
            @click="viewMode = 'edit'"
            :class="[
              'px-2.5 py-1 text-xs font-medium transition-colors',
              viewMode === 'edit' 
                ? 'bg-[var(--primary)] text-white' 
                : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
            ]"
            title="Edit file"
          >
            <Edit3 class="w-3.5 h-3.5 inline mr-1" />
            Edit
          </button>
        </div>

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

        <div v-if="viewMode === 'diff' && diff && diff.hunks.length > 1" class="flex items-center gap-0.5">
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

        <button
          @click="emit('close')"
          class="p-1 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
        >
          <X class="w-5 h-5" />
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-auto bg-[var(--diff-bg)] font-mono text-[13px] leading-[1.5]">
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
          v-if="usePlainTextHighlighting"
          class="sticky top-0 z-20 px-3 py-1 text-[11px] border-y border-[var(--diff-border)] bg-[var(--secondary)]/90 text-[var(--muted-foreground)]"
        >
          Large diff mode: syntax coloring is simplified to keep scrolling smooth.
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
                  class="diff-code-line flex-1 px-1.5 whitespace-pre overflow-x-auto"
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

      <div v-else-if="viewMode === 'file-diff'" class="min-w-fit h-full overflow-auto" @scroll="onFileDiffScroll">
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
            <div class="diff-line-no w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px] leading-[20px]">
              {{ item.line.oldLineNo ?? '' }}
            </div>
            <div class="diff-line-no w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px] leading-[20px]">
              {{ item.line.lineNo || '' }}
            </div>
            <div 
              class="w-5 flex-shrink-0 text-center select-none font-bold leading-[20px]"
              :class="item.line.type === 'addition' ? 'text-[var(--diff-sign-add)]' : item.line.type === 'deletion' ? 'text-[var(--diff-sign-del)]' : 'text-[var(--diff-sign-neutral)]'"
            >
              {{ linePrefix(item.line.type) }}
            </div>
            <pre 
              class="diff-code-line flex-1 px-1.5 whitespace-pre-wrap break-all overflow-hidden leading-[20px] m-0"
              :class="item.line.type === 'addition' ? 'text-[var(--diff-add-fg)]' : item.line.type === 'deletion' ? 'text-[var(--diff-del-fg)]' : 'text-[var(--diff-text)]'"
            ><code class="hljs bg-transparent" v-html="getHighlightedFileLine(item.idx, item.line)"></code></pre>
          </div>
          </div>
        </div>
        <div v-else-if="!loading && !loadingFileContent" class="flex items-center justify-center h-64 text-[var(--muted-foreground)]">
          No file content available
        </div>
      </div>

      <div v-else-if="viewMode === 'edit'" class="min-w-fit h-full">
        <textarea
          v-model="editContent"
          @input="onEditInput"
          class="w-full h-full bg-[var(--diff-bg)] text-[var(--diff-text)] p-3 resize-none outline-none font-mono text-[13px] leading-[1.5]"
          spellcheck="false"
        ></textarea>
      </div>

      <div v-else-if="viewMode === 'time-lapse'" class="h-full flex flex-col overflow-hidden">
        <div class="px-3 py-2 border-b border-[var(--diff-border)] bg-[var(--secondary)]/40 flex items-center justify-between gap-2">
          <div class="text-[11px] text-[var(--muted-foreground)]">
            File evolution timeline (recent frames)
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
            <pre class="m-0 p-3 font-mono text-[12px] leading-[1.45] whitespace-pre-wrap break-all text-[var(--diff-text)]">{{ activeTimeLapseFrame.content }}</pre>
          </div>
          <div class="px-3 py-1.5 border-t border-[var(--diff-border)] bg-[var(--secondary)]/40 text-[10px] text-[var(--muted-foreground)]">
            Frame {{ Math.min(timeLapseFrameIndex + 1, timeLapseFrames.length) }} / {{ timeLapseFrames.length }}
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

.diff-inline-add {
  background: color-mix(in srgb, var(--diff-sign-add) 42%, transparent);
  border: 1px solid color-mix(in srgb, var(--diff-sign-add) 82%, transparent);
  border-radius: 2px;
  color: var(--diff-sign-add);
  font-weight: 800;
  box-shadow: inset 0 -1px 0 color-mix(in srgb, var(--diff-sign-add) 78%, transparent);
}

.diff-inline-del {
  background: color-mix(in srgb, var(--diff-sign-del) 42%, transparent);
  border: 1px solid color-mix(in srgb, var(--diff-sign-del) 82%, transparent);
  border-radius: 2px;
  color: var(--diff-sign-del);
  font-weight: 800;
  box-shadow: inset 0 -1px 0 color-mix(in srgb, var(--diff-sign-del) 78%, transparent);
}
</style>


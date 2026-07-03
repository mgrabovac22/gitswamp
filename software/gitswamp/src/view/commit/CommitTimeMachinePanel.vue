<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch, type Component } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { FileArchive, FileCode2, FileImage, FileText, Folder } from "lucide-vue-next";
import logoCrocLoading from "@/assets/logo_croc_loading.gif";
import type { CommitInfo, CommitFileInfo } from "@/types";

const FULL_HISTORY_LIMIT = 60000;
const loadingLetters = ["L", "o", "a", "d", "i", "n", "g"];

const props = defineProps<{
  repoPath: string;
  focusSha?: string | null;
}>();

const emit = defineEmits<{
  close: [];
}>();

interface SnapshotEntry {
  name: string;
  path: string;
  type: "dir" | "file";
}

interface EntryVisual {
  icon: Component;
  toneClass: string;
  label: string;
}

const historyCommits = ref<CommitInfo[]>([]);
const historyLoading = ref(false);
const historyError = ref("");
let historyRunToken = 0;

const selectedIndex = ref(0);
const autoPlay = ref(false);
const autoPlayDirection = ref<1 | -1>(1);
const shaSearch = ref("");
const shaMatchCursor = ref(0);
const selectedFiles = ref<CommitFileInfo[]>([]);
const filesLoading = ref(false);
const filesError = ref("");
const selectedFilePath = ref("");
const snapshotPaths = ref<string[]>([]);
const treeLoading = ref(false);
const treeError = ref("");
const currentDirectory = ref("");
const selectedExplorerFilePath = ref("");
const selectedExplorerFileContent = ref("");
const explorerFileLoading = ref(false);
const explorerFileError = ref("");
const panelScrollContainer = ref<HTMLElement | null>(null);

const commitFilesCache = new Map<string, CommitFileInfo[]>();
const commitTreeCache = new Map<string, string[]>();
const fileContentCache = new Map<string, string>();
const COMMIT_FILES_CACHE_LIMIT = 12;
const COMMIT_TREE_CACHE_LIMIT = 3;
const FILE_CONTENT_CACHE_LIMIT = 8;
const FILE_CONTENT_CACHE_MAX_CHARS = 220_000;

function getCachedEntry<T>(cache: Map<string, T>, key: string): T | null {
  const value = cache.get(key);
  if (value === undefined) return null;
  cache.delete(key);
  cache.set(key, value);
  return value;
}

function setCachedEntry<T>(cache: Map<string, T>, key: string, value: T, limit: number) {
  if (cache.has(key)) {
    cache.delete(key);
  }
  cache.set(key, value);

  while (cache.size > limit) {
    const oldestKey = cache.keys().next().value;
    if (!oldestKey) break;
    cache.delete(oldestKey);
  }
}

let autoplayTimer: number | null = null;
let snapshotScheduleTimer: number | null = null;
let snapshotRunToken = 0;
let explorerRunToken = 0;
let pendingAutoplayScrollRestore = false;
let lastAutoplayScrollTop = 0;

const hasCommits = computed(() => historyCommits.value.length > 0);

const selectedCommit = computed(() => {
  if (!hasCommits.value) return null;
  const safeIndex = Math.max(0, Math.min(selectedIndex.value, historyCommits.value.length - 1));
  return historyCommits.value[safeIndex] ?? null;
});

function toMillis(timestamp: number): number {
  return Math.abs(timestamp) < 1000000000000 ? timestamp * 1000 : timestamp;
}

function formatTimelineDate(timestamp: number): string {
  return new Date(toMillis(timestamp)).toLocaleString(undefined, {
    weekday: "short",
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatTimelineMillis(millis: number): string {
  return new Date(millis).toLocaleString(undefined, {
    weekday: "short",
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

const selectedDateLabel = computed(() => {
  if (!selectedCommit.value) return "";
  return formatTimelineDate(selectedCommit.value.timestamp);
});

const normalizedShaSearch = computed(() => shaSearch.value.trim().toLowerCase());

const shaMatches = computed(() => {
  const query = normalizedShaSearch.value;
  if (!query) return [];

  return historyCommits.value.filter((commit) => {
    const full = commit.sha.toLowerCase();
    const short = commit.short_sha.toLowerCase();
    return full.includes(query) || short.includes(query);
  });
});

const shaMatchCount = computed(() => shaMatches.value.length);

const shaSearchStats = computed(() => {
  if (shaMatches.value.length === 0) return null;

  let oldest = Number.POSITIVE_INFINITY;
  let newest = Number.NEGATIVE_INFINITY;
  const authors = new Set<string>();

  for (const commit of shaMatches.value) {
    const millis = toMillis(commit.timestamp);
    oldest = Math.min(oldest, millis);
    newest = Math.max(newest, millis);
    if (commit.author_name.trim()) {
      authors.add(commit.author_name.trim());
    }
  }

  return {
    commits: shaMatches.value.length,
    authors: authors.size,
    oldest: formatTimelineMillis(oldest),
    newest: formatTimelineMillis(newest),
  };
});

const timelineProgress = computed(() => {
  if (!hasCommits.value || historyCommits.value.length <= 1) return 0;
  return Math.round((selectedIndex.value / (historyCommits.value.length - 1)) * 100);
});

const totalAdditions = computed(() => selectedFiles.value.reduce((sum, file) => sum + Math.max(0, file.additions || 0), 0));
const totalDeletions = computed(() => selectedFiles.value.reduce((sum, file) => sum + Math.max(0, file.deletions || 0), 0));

const rollbackCommand = computed(() => {
  if (!selectedCommit.value) return "";
  const sha = selectedCommit.value.short_sha || selectedCommit.value.sha.slice(0, 7);
  if (!selectedFilePath.value.trim()) {
    return `git checkout ${sha}`;
  }
  return `git checkout ${sha} -- ${selectedFilePath.value.trim()}`;
});

const directoryBreadcrumbs = computed(() => {
  const segments = currentDirectory.value ? currentDirectory.value.split("/").filter(Boolean) : [];
  const crumbs: { label: string; path: string }[] = [{ label: "root", path: "" }];
  let walking = "";
  for (const segment of segments) {
    walking = walking ? `${walking}/${segment}` : segment;
    crumbs.push({ label: segment, path: walking });
  }
  return crumbs;
});

function normalizePath(path: string): string {
  return path.split("\\").join("/").replace(/^\/+/, "").replace(/\/+$/, "");
}

function shouldIncludePath(path: string, prefix: string): boolean {
  if (!path) return false;
  if (!prefix) return true;
  return path.startsWith(prefix);
}

function buildEntryFromPath(path: string, currentDirectoryPath: string, prefix: string): SnapshotEntry | null {
  if (!shouldIncludePath(path, prefix)) {
    return null;
  }

  const remainder = prefix ? path.slice(prefix.length) : path;
  if (!remainder) {
    return null;
  }

  const parts = remainder.split("/").filter(Boolean);
  if (parts.length === 0) {
    return null;
  }

  const first = parts[0];
  const fullPath = currentDirectoryPath ? `${currentDirectoryPath}/${first}` : first;
  const type: "dir" | "file" = parts.length > 1 ? "dir" : "file";

  return {
    name: first,
    path: fullPath,
    type,
  };
}

function mergeEntry(entries: Map<string, SnapshotEntry>, candidate: SnapshotEntry | null) {
  if (!candidate) return;
  const existing = entries.get(candidate.path);
  if (!existing || existing.type === "file") {
    entries.set(candidate.path, candidate);
  }
}

function sortEntries(entries: SnapshotEntry[]): SnapshotEntry[] {
  return entries.sort((a, b) => {
    if (a.type !== b.type) {
      return a.type === "dir" ? -1 : 1;
    }
    return a.name.localeCompare(b.name);
  });
}

function fileExtension(name: string): string {
  const idx = name.lastIndexOf(".");
  if (idx <= 0 || idx >= name.length - 1) {
    return "";
  }
  return name.slice(idx + 1).toLowerCase();
}

function fileVisual(name: string): EntryVisual {
  const ext = fileExtension(name);

  if (["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif"].includes(ext)) {
    return {
      icon: FileImage,
      toneClass: "text-[#22c55e]",
      label: "Image",
    };
  }

  if (["zip", "rar", "7z", "gz", "tar", "tgz", "bz2", "xz"].includes(ext)) {
    return {
      icon: FileArchive,
      toneClass: "text-[#f59e0b]",
      label: "Archive",
    };
  }

  if (["md", "txt", "rst", "log", "yaml", "yml", "toml", "ini", "env"].includes(ext)) {
    return {
      icon: FileText,
      toneClass: "text-[#94a3b8]",
      label: ext ? ext.toUpperCase() : "Text",
    };
  }

  return {
    icon: FileCode2,
    toneClass: "text-[var(--primary)]",
    label: ext ? ext.toUpperCase() : "Code",
  };
}

function entryVisual(entry: SnapshotEntry): EntryVisual {
  if (entry.type === "dir") {
    return {
      icon: Folder,
      toneClass: "text-[var(--chart-5)]",
      label: "Folder",
    };
  }

  return fileVisual(entry.name);
}

function collectDirectoryEntries(paths: string[], currentPath: string): SnapshotEntry[] {
  const entries = new Map<string, SnapshotEntry>();
  const normalizedCurrent = normalizePath(currentPath);
  const prefix = normalizedCurrent ? `${normalizedCurrent}/` : "";

  for (const rawPath of paths) {
    const normalized = normalizePath(rawPath);
    const candidate = buildEntryFromPath(normalized, normalizedCurrent, prefix);
    mergeEntry(entries, candidate);
  }

  return sortEntries(Array.from(entries.values()));
}

const directoryEntries = computed<SnapshotEntry[]>(() => {
  return collectDirectoryEntries(snapshotPaths.value, currentDirectory.value);
});

function clampIndex(index: number): number {
  if (historyCommits.value.length === 0) return 0;
  return Math.max(0, Math.min(index, historyCommits.value.length - 1));
}

function moveFrame(step: number) {
  capturePanelScrollPosition();
  selectedIndex.value = clampIndex(selectedIndex.value + step);
}

function findCommitIndexBySha(sha: string): number {
  const query = sha.trim().toLowerCase();
  if (!query) return -1;

  return historyCommits.value.findIndex((commit) => {
    const full = commit.sha.toLowerCase();
    const short = commit.short_sha.toLowerCase();
    return full === query || short === query || full.startsWith(query) || short.startsWith(query);
  });
}

function focusCommitBySha(sha: string): boolean {
  const idx = findCommitIndexBySha(sha);
  if (idx < 0) return false;
  selectedIndex.value = idx;
  return true;
}

function jumpToShaMatch(direction: -1 | 1) {
  if (shaMatches.value.length === 0) return;

  const next = ((shaMatchCursor.value + direction) % shaMatches.value.length + shaMatches.value.length)
    % shaMatches.value.length;
  shaMatchCursor.value = next;
  selectedIndex.value = historyCommits.value.findIndex((commit) => commit.sha === shaMatches.value[next].sha);
}

function toggleAutoplay() {
  autoPlay.value = !autoPlay.value;
}

function toggleAutoplayDirection() {
  autoPlayDirection.value = autoPlayDirection.value === 1 ? -1 : 1;
}

function capturePanelScrollPosition() {
  const container = panelScrollContainer.value;
  if (!container) return;

  pendingAutoplayScrollRestore = true;
  lastAutoplayScrollTop = container.scrollTop;
}

async function restoreAutoplayScrollIfNeeded() {
  if (!pendingAutoplayScrollRestore) return;

  await nextTick();

  const container = panelScrollContainer.value;
  if (container) {
    container.scrollTop = lastAutoplayScrollTop;
  }

  pendingAutoplayScrollRestore = false;
}

function clearAutoplayTimer() {
  if (autoplayTimer !== null) {
    globalThis.clearInterval(autoplayTimer);
    autoplayTimer = null;
  }
}

function clearSnapshotScheduleTimer() {
  if (snapshotScheduleTimer !== null) {
    globalThis.clearTimeout(snapshotScheduleTimer);
    snapshotScheduleTimer = null;
  }
}

function advanceAutoplayFrame() {
  if (historyCommits.value.length === 0) {
    stopAutoplay();
    return;
  }

  capturePanelScrollPosition();

  if (autoPlayDirection.value === 1) {
    if (selectedIndex.value >= historyCommits.value.length - 1) {
      selectedIndex.value = 0;
      return;
    }

    selectedIndex.value += 1;
    return;
  }

  if (selectedIndex.value <= 0) {
    selectedIndex.value = historyCommits.value.length - 1;
    return;
  }

  selectedIndex.value -= 1;
}

function stopAutoplay() {
  autoPlay.value = false;
  clearAutoplayTimer();
  pendingAutoplayScrollRestore = false;
}

function startAutoplay() {
  clearAutoplayTimer();
  autoplayTimer = globalThis.setInterval(() => {
    advanceAutoplayFrame();
  }, 1200);
}

function scheduleSnapshotLoad() {
  clearSnapshotScheduleTimer();
  const delay = autoPlay.value ? 220 : 60;
  snapshotScheduleTimer = globalThis.setTimeout(() => {
    void loadSnapshotData();
  }, delay);
}

async function copyRollbackCommand() {
  if (!rollbackCommand.value) return;
  try {
    await navigator.clipboard.writeText(rollbackCommand.value);
  } catch {
  }
}

function resetExplorerSelection() {
  currentDirectory.value = "";
  selectedExplorerFilePath.value = "";
  selectedExplorerFileContent.value = "";
  explorerFileError.value = "";
  explorerFileLoading.value = false;
}

async function loadExplorerFile(path: string, shaOverride?: string) {
  if (!props.repoPath) return;

  const sha = shaOverride ?? selectedCommit.value?.sha;
  if (!sha) return;

  explorerRunToken += 1;
  const runToken = explorerRunToken;
  const cacheKey = `${props.repoPath}::${sha}::${path}`;

  selectedExplorerFilePath.value = path;
  explorerFileError.value = "";

  const cachedContent = getCachedEntry(fileContentCache, cacheKey);
  if (cachedContent !== null) {
    selectedExplorerFileContent.value = cachedContent;
    explorerFileLoading.value = false;
    return;
  }

  explorerFileLoading.value = true;

  try {
    const content = await invoke<string>("get_file_content", {
      path: props.repoPath,
      filePath: path,
      sha,
    });
    if (runToken !== explorerRunToken || selectedExplorerFilePath.value !== path) {
      return;
    }

    if (content.length <= FILE_CONTENT_CACHE_MAX_CHARS) {
      setCachedEntry(fileContentCache, cacheKey, content, FILE_CONTENT_CACHE_LIMIT);
    }
    selectedExplorerFileContent.value = content;
  } catch {
    if (runToken === explorerRunToken && selectedExplorerFilePath.value === path) {
      explorerFileError.value = "Could not preview this file snapshot.";
    }
  } finally {
    if (runToken === explorerRunToken && selectedExplorerFilePath.value === path) {
      explorerFileLoading.value = false;
    }
  }
}

function openDirectory(path: string) {
  currentDirectory.value = normalizePath(path);
}

function openDirectoryCrumb(path: string) {
  currentDirectory.value = normalizePath(path);
}

async function openEntry(entry: SnapshotEntry) {
  if (entry.type === "dir") {
    openDirectory(entry.path);
    return;
  }
  await loadExplorerFile(entry.path);
}

function snapshotCacheKey(sha: string): string {
  return `${props.repoPath}::${sha}`;
}

function pathExistsInSnapshot(path: string): boolean {
  if (!path) return false;
  return snapshotPaths.value.includes(path);
}

function directoryExistsInSnapshot(path: string): boolean {
  if (!path) return true;
  return snapshotPaths.value.some((entry) => entry === path || entry.startsWith(`${path}/`));
}

async function syncExplorerPreviewForCommit(sha: string) {
  if (!directoryExistsInSnapshot(currentDirectory.value)) {
    currentDirectory.value = "";
  }

  const preferredPath =
    (pathExistsInSnapshot(selectedExplorerFilePath.value) && selectedExplorerFilePath.value)
    || (pathExistsInSnapshot(selectedFilePath.value) && selectedFilePath.value)
    || snapshotPaths.value[0]
    || "";

  if (!preferredPath) {
    selectedExplorerFilePath.value = "";
    selectedExplorerFileContent.value = "";
    explorerFileError.value = "";
    explorerFileLoading.value = false;
    return;
  }

  selectedExplorerFilePath.value = preferredPath;

  if (autoPlay.value) {
    const cachedContent = getCachedEntry(fileContentCache, `${props.repoPath}::${sha}::${preferredPath}`);
    if (cachedContent !== null) {
      selectedExplorerFileContent.value = cachedContent;
      explorerFileError.value = "";
    }
    explorerFileLoading.value = false;
    return;
  }

  await loadExplorerFile(preferredPath, sha);
}

async function loadSnapshotData() {
  snapshotRunToken += 1;
  const runToken = snapshotRunToken;

  const commit = selectedCommit.value;
  if (!props.repoPath || !commit) {
    if (runToken === snapshotRunToken) {
      selectedFiles.value = [];
      snapshotPaths.value = [];
      filesLoading.value = false;
      treeLoading.value = false;
      filesError.value = "";
      treeError.value = "";
      selectedFilePath.value = "";
      resetExplorerSelection();
      await restoreAutoplayScrollIfNeeded();
    }
    return;
  }

  const cacheKey = snapshotCacheKey(commit.sha);
  const cachedFiles = getCachedEntry(commitFilesCache, cacheKey);
  const cachedTree = getCachedEntry(commitTreeCache, cacheKey);

  filesError.value = "";
  treeError.value = "";

  if (cachedFiles) {
    selectedFiles.value = cachedFiles;
    if (!cachedFiles.some((item) => item.path === selectedFilePath.value)) {
      selectedFilePath.value = cachedFiles[0]?.path || "";
    }
    filesLoading.value = false;
  } else {
    filesLoading.value = true;
  }

  if (cachedTree) {
    snapshotPaths.value = cachedTree;
    treeLoading.value = false;
  } else {
    treeLoading.value = true;
  }

  if (cachedFiles && cachedTree) {
    await syncExplorerPreviewForCommit(commit.sha);
    await restoreAutoplayScrollIfNeeded();
    return;
  }

  const [filesResult, treeResult] = await Promise.allSettled([
    cachedFiles
      ? Promise.resolve(cachedFiles)
      : invoke<CommitFileInfo[]>("get_commit_files", {
        path: props.repoPath,
        sha: commit.sha,
      }),
    cachedTree
      ? Promise.resolve(cachedTree)
      : invoke<string[]>("get_commit_tree_paths", {
        path: props.repoPath,
        sha: commit.sha,
      }),
  ]);

  if (runToken !== snapshotRunToken) {
    return;
  }

  if (filesResult.status === "fulfilled") {
    selectedFiles.value = filesResult.value;
    setCachedEntry(commitFilesCache, cacheKey, filesResult.value, COMMIT_FILES_CACHE_LIMIT);
    if (!filesResult.value.some((item) => item.path === selectedFilePath.value)) {
      selectedFilePath.value = filesResult.value[0]?.path || "";
    }
  } else {
    filesError.value = "Could not load commit snapshot files.";
  }

  if (treeResult.status === "fulfilled") {
    snapshotPaths.value = treeResult.value;
    setCachedEntry(commitTreeCache, cacheKey, treeResult.value, COMMIT_TREE_CACHE_LIMIT);
  } else {
    treeError.value = "Could not load directory snapshot for this commit.";
  }

  filesLoading.value = false;
  treeLoading.value = false;
  await syncExplorerPreviewForCommit(commit.sha);
  await restoreAutoplayScrollIfNeeded();
}

async function loadFullHistory() {
  historyRunToken += 1;
  const runToken = historyRunToken;

  if (!props.repoPath) {
    historyCommits.value = [];
    historyError.value = "";
    historyLoading.value = false;
    return;
  }

  historyLoading.value = true;
  historyError.value = "";

  try {
    const commits = await invoke<CommitInfo[]>("get_commits", {
      path: props.repoPath,
      maxCount: FULL_HISTORY_LIMIT,
    });

    if (runToken !== historyRunToken) {
      return;
    }

    historyCommits.value = commits;
    selectedIndex.value = clampIndex(selectedIndex.value);
  } catch {
    if (runToken === historyRunToken) {
      historyCommits.value = [];
      historyError.value = "Could not load full commit history for Time Machine.";
    }
  } finally {
    if (runToken === historyRunToken) {
      historyLoading.value = false;
    }
  }
}

watch(
  () => props.repoPath,
  () => {
    stopAutoplay();
    clearSnapshotScheduleTimer();
    commitFilesCache.clear();
    commitTreeCache.clear();
    fileContentCache.clear();
    selectedIndex.value = 0;
    resetExplorerSelection();
    void loadFullHistory();
  },
  { immediate: true },
);

watch(
  () => historyCommits.value.length,
  () => {
    selectedIndex.value = clampIndex(selectedIndex.value);
    if (historyCommits.value.length === 0) {
      stopAutoplay();
      return;
    }

    if (props.focusSha) {
      const focused = focusCommitBySha(props.focusSha);
      if (focused && !shaSearch.value) {
        shaSearch.value = props.focusSha.slice(0, 12);
      }
    }
  },
);

watch(
  () => normalizedShaSearch.value,
  () => {
    shaMatchCursor.value = 0;
    if (shaMatches.value.length === 0) return;
    selectedIndex.value = historyCommits.value.findIndex((commit) => commit.sha === shaMatches.value[0].sha);
  },
);

watch(
  () => selectedCommit.value?.sha,
  (sha) => {
    if (!sha || shaMatches.value.length === 0) return;
    const idx = shaMatches.value.findIndex((commit) => commit.sha === sha);
    if (idx >= 0) {
      shaMatchCursor.value = idx;
    }
  },
);

watch(
  () => props.focusSha,
  (sha) => {
    if (!sha) return;
    const focused = focusCommitBySha(sha);
    if (!focused) return;
    shaSearch.value = sha.slice(0, 12);
  },
  { immediate: true },
);

watch(
  () => autoPlay.value,
  (enabled) => {
    if (enabled) {
      startAutoplay();
    } else {
      clearAutoplayTimer();
      scheduleSnapshotLoad();
    }
  },
);

watch(
  () => autoPlayDirection.value,
  () => {
    if (autoPlay.value) {
      startAutoplay();
    }
  },
);

watch(
  () => selectedCommit.value?.sha,
  () => {
    scheduleSnapshotLoad();
  },
);

onUnmounted(() => {
  stopAutoplay();
  clearSnapshotScheduleTimer();
  historyCommits.value = [];
  selectedFiles.value = [];
  snapshotPaths.value = [];
  selectedExplorerFileContent.value = "";
  commitFilesCache.clear();
  commitTreeCache.clear();
  fileContentCache.clear();
});
</script>

<template>
  <div ref="panelScrollContainer" class="flex-1 min-h-0 overflow-y-auto time-machine-surface">
    <div class="p-4 md:p-5 space-y-4">
      <section class="tm-card">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <p class="text-[10px] uppercase tracking-[0.16em] text-[var(--muted-foreground)]">Visualise Commit History</p>
            <h2 class="text-lg md:text-xl font-bold text-[var(--foreground)]">Time Machine</h2>
            <p class="text-xs text-[var(--muted-foreground)] mt-1">Slide through history and preview rollback frames.</p>
          </div>
          <div class="flex items-center gap-2">
            <div class="px-3 py-1.5 rounded-md border border-[var(--border)] bg-[var(--secondary)]">
              <span class="text-[11px] font-semibold text-[var(--primary)]">Frame {{ hasCommits ? selectedIndex + 1 : 0 }} / {{ historyCommits.length }}</span>
            </div>
            <button
              class="tm-close"
              title="Back to Git Graph"
              @click="emit('close')"
            >
              x
            </button>
          </div>
        </div>
      </section>

      <section
        v-if="historyLoading"
        class="tm-card p-6 flex flex-col items-center justify-center gap-2 min-h-[240px]"
      >
        <img :src="logoCrocLoading" alt="Loading time machine" class="tm-loader-logo" />
        <div class="tm-loader-wave" aria-label="Loading">
          <span
            v-for="(letter, idx) in loadingLetters"
            :key="`tm-load-${idx}`"
            class="tm-loader-letter"
            :style="{ animationDelay: `${idx * 0.06}s` }"
          >
            {{ letter }}
          </span>
        </div>
        <p class="text-xs text-[var(--muted-foreground)]">Loading full history timeline...</p>
      </section>

      <section
        v-else-if="historyError"
        class="tm-card border-[var(--destructive)]/40 bg-[var(--destructive)]/10 p-4 text-sm text-[var(--destructive)]"
      >
        {{ historyError }}
      </section>

      <template v-else>

        <section v-if="hasCommits" class="tm-card space-y-3">
          <div class="relative">
            <input
              v-model.number="selectedIndex"
              class="w-full h-2 rounded-lg appearance-none timeline-slider"
              type="range"
              min="0"
              :max="Math.max(0, historyCommits.length - 1)"
              step="1"
            />
            <div class="mt-2 h-1 rounded bg-[var(--secondary)] overflow-hidden">
              <div class="h-full timeline-progress" :style="{ width: `${timelineProgress}%` }" />
            </div>
          </div>

          <div class="flex flex-wrap items-center gap-2">
            <button class="tm-btn" @click="moveFrame(-1)">Back</button>
            <button class="tm-btn" @click="moveFrame(1)">Forward</button>
            <button class="tm-btn" @click="toggleAutoplay">{{ autoPlay ? "Pause" : "Autoplay" }}</button>
            <button class="tm-btn" @click="toggleAutoplayDirection">{{ autoPlayDirection === 1 ? "Reverse autoplay" : "Forward autoplay" }}</button>
            <span class="text-xs text-[var(--muted-foreground)]">Scrub to inspect project state at each commit.</span>
          </div>
        </section>

        <section v-if="hasCommits" class="tm-card space-y-3">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <h3 class="text-sm font-semibold text-[var(--foreground)]">SHA Search</h3>
            <span class="text-[11px] text-[var(--muted-foreground)]">
              {{ shaMatchCount }} match{{ shaMatchCount === 1 ? "" : "es" }}
              <template v-if="shaMatchCount > 0"> • {{ shaMatchCursor + 1 }}/{{ shaMatchCount }}</template>
            </span>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <input
              v-model="shaSearch"
              placeholder="Search commit by SHA..."
              class="h-8 min-w-[240px] flex-1 px-3 rounded border border-[var(--border)] bg-[var(--input-background)] text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            />
            <button class="tm-btn" :disabled="shaMatchCount === 0" @click="jumpToShaMatch(-1)">Prev</button>
            <button class="tm-btn" :disabled="shaMatchCount === 0" @click="jumpToShaMatch(1)">Next</button>
          </div>

          <div v-if="shaSearchStats" class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-2">
            <div class="snapshot-chip">
              <span class="chip-label">Matches</span>
              <span class="chip-value">{{ shaSearchStats.commits }}</span>
            </div>
            <div class="snapshot-chip">
              <span class="chip-label">Authors</span>
              <span class="chip-value">{{ shaSearchStats.authors }}</span>
            </div>
            <div class="snapshot-chip">
              <span class="chip-label">Oldest</span>
              <span class="chip-value">{{ shaSearchStats.oldest }}</span>
            </div>
            <div class="snapshot-chip">
              <span class="chip-label">Newest</span>
              <span class="chip-value">{{ shaSearchStats.newest }}</span>
            </div>
          </div>
        </section>

        <section v-if="selectedCommit" class="grid grid-cols-1 xl:grid-cols-[1.1fr_0.9fr] gap-4">
          <article class="tm-card">
            <div class="text-[10px] uppercase tracking-[0.14em] text-[var(--muted-foreground)] mb-1">Selected Snapshot</div>
            <h3 class="text-sm md:text-base font-semibold text-[var(--foreground)] leading-snug">{{ selectedCommit.message }}</h3>
            <div class="mt-2 text-xs text-[var(--muted-foreground)]">{{ selectedDateLabel }}</div>

            <div class="mt-3 grid grid-cols-1 sm:grid-cols-3 gap-2.5 text-xs">
              <div class="snapshot-chip">
                <span class="chip-label">Author</span>
                <span class="chip-value">{{ selectedCommit.author_name }}</span>
              </div>
              <div class="snapshot-chip">
                <span class="chip-label">SHA</span>
                <span class="chip-value">{{ selectedCommit.short_sha }}</span>
              </div>
              <div class="snapshot-chip">
                <span class="chip-label">Refs</span>
                <span class="chip-value">{{ selectedCommit.refs.length }}</span>
              </div>
            </div>

            <div class="mt-4 rounded-xl border border-[var(--border)] bg-[var(--secondary)]/50 p-3">
              <div class="text-[10px] uppercase tracking-[0.14em] text-[var(--muted-foreground)] mb-1">Selective Rollback Command</div>
              <div class="font-mono text-xs text-[var(--foreground)] break-all select-text">{{ rollbackCommand }}</div>
              <div class="mt-2.5 flex flex-wrap items-center gap-2">
                <select
                  v-model="selectedFilePath"
                  class="h-8 min-w-[220px] max-w-full px-2 text-[11px] rounded border border-[var(--border)] bg-[var(--input-background)] text-[var(--foreground)] focus:outline-none"
                >
                  <option value="">Whole commit</option>
                  <option v-for="file in selectedFiles" :key="file.path" :value="file.path">{{ file.path }}</option>
                </select>
                <button class="tm-btn" @click="copyRollbackCommand">Copy command</button>
              </div>
            </div>
          </article>

          <article class="tm-card tm-subcard-shell">
            <div class="flex items-center justify-between gap-2 mb-2.5">
              <h3 class="text-sm font-semibold text-[var(--foreground)]">Snapshot file changes</h3>
              <div class="text-[11px] text-[var(--primary)]">+{{ totalAdditions }} / -{{ totalDeletions }}</div>
            </div>

            <div v-if="filesError && selectedFiles.length === 0" class="text-xs text-[var(--destructive)]">{{ filesError }}</div>
            <div v-else-if="selectedFiles.length === 0" class="text-xs text-[var(--muted-foreground)]">No changed files in this snapshot.</div>
            <div v-else class="space-y-2 max-h-[360px] overflow-y-auto pr-1">
              <div
                v-for="file in selectedFiles"
                :key="file.path"
                class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/45 px-2.5 py-2"
              >
                <div class="text-[11px] text-[var(--foreground)] truncate">{{ file.path }}</div>
                <div class="mt-1 text-[10px] text-[var(--muted-foreground)]">
                  <span class="text-[var(--chart-5)]">+{{ file.additions }}</span>
                  <span class="mx-1">|</span>
                  <span class="text-[var(--destructive)]">-{{ file.deletions }}</span>
                  <span class="mx-1">|</span>
                  <span>{{ file.status }}</span>
                </div>
              </div>
            </div>

            <div v-if="filesLoading" class="tm-subcard-overlay">
              <img :src="logoCrocLoading" alt="Loading snapshot files" class="mini-loader-logo" />
              <div class="text-[11px] text-[var(--foreground)] font-semibold">Loading snapshot files</div>
            </div>
          </article>
        </section>

      <section
        v-if="selectedCommit"
        class="tm-card"
      >
        <div class="flex flex-wrap items-center justify-between gap-2 mb-3">
          <h3 class="text-sm font-semibold text-[var(--foreground)]">Snapshot Directory Explorer</h3>
          <span class="text-[10px] text-[var(--muted-foreground)]">Browse repository tree exactly as it was in this commit</span>
        </div>

        <div class="mb-2.5 flex flex-wrap items-center gap-1.5">
          <button
            v-for="crumb in directoryBreadcrumbs"
            :key="crumb.path || 'root'"
            class="crumb-btn"
            @click="openDirectoryCrumb(crumb.path)"
          >
            {{ crumb.label }}
          </button>
        </div>

        <div class="grid grid-cols-1 xl:grid-cols-[0.85fr_1.15fr] gap-3">
          <article class="rounded-xl border border-[var(--border)] bg-[var(--secondary)]/45 p-2.5 tm-subcard-shell">
            <div v-if="treeError && directoryEntries.length === 0" class="text-xs text-[var(--destructive)]">{{ treeError }}</div>
            <div v-else-if="directoryEntries.length === 0" class="text-xs text-[var(--muted-foreground)]">No entries in this directory.</div>
            <div v-else class="max-h-[300px] overflow-y-auto space-y-1 pr-1">
              <button
                v-for="entry in directoryEntries"
                :key="entry.path"
                class="entry-row"
                :class="entry.type === 'dir' ? 'entry-dir' : 'entry-file'"
                @click="openEntry(entry)"
              >
                <span class="entry-icon-wrap">
                  <component :is="entryVisual(entry).icon" class="w-3.5 h-3.5 flex-shrink-0" :class="entryVisual(entry).toneClass" />
                </span>
                <span class="truncate">{{ entry.name }}</span>
                <span v-if="entry.type === 'file'" class="entry-lang">{{ entryVisual(entry).label }}</span>
              </button>
            </div>

            <div v-if="treeLoading" class="tm-subcard-overlay">
              <img :src="logoCrocLoading" alt="Loading tree" class="mini-loader-logo" />
              <div class="text-[11px] text-[var(--foreground)] font-semibold">Loading snapshot tree</div>
            </div>
          </article>

          <article class="rounded-xl border border-[var(--border)] bg-[var(--secondary)]/45 p-2.5 tm-subcard-shell">
            <div class="text-[11px] text-[var(--muted-foreground)] mb-2">
              {{ selectedExplorerFilePath ? selectedExplorerFilePath : "Select a file to preview snapshot content" }}
            </div>
            <div v-if="explorerFileError && !selectedExplorerFileContent" class="text-xs text-[var(--destructive)]">{{ explorerFileError }}</div>
            <pre v-else-if="selectedExplorerFileContent" class="snapshot-preview select-text">{{ selectedExplorerFileContent }}</pre>
            <div v-else class="text-xs text-[var(--muted-foreground)]">No file selected.</div>

            <div v-if="explorerFileLoading" class="tm-subcard-overlay">
              <img :src="logoCrocLoading" alt="Loading file" class="mini-loader-logo" />
              <div class="text-[11px] text-[var(--foreground)] font-semibold">Loading file snapshot</div>
            </div>
          </article>
        </div>
      </section>

      <section v-else class="tm-card text-sm text-[var(--muted-foreground)]">
        No commits loaded yet for Time Machine visualization.
      </section>
      </template>
    </div>
  </div>
</template>

<style scoped>
.time-machine-surface {
  background:
    radial-gradient(circle at 14% 16%, color-mix(in srgb, var(--primary) 16%, transparent), transparent 42%),
    radial-gradient(circle at 88% 18%, color-mix(in srgb, var(--chart-2) 14%, transparent), transparent 40%),
    linear-gradient(165deg, color-mix(in srgb, var(--background) 96%, black 4%) 0%, var(--background) 52%, color-mix(in srgb, var(--card) 70%, var(--background) 30%) 100%);
}

.tm-card {
  border-radius: 0.65rem;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--card) 90%, transparent);
  padding: 1rem;
}

.tm-subcard-shell {
  position: relative;
  overflow: hidden;
}

.tm-subcard-overlay {
  position: absolute;
  inset: 0;
  z-index: 12;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.35rem;
  background: color-mix(in srgb, var(--background) 76%, transparent);
  backdrop-filter: blur(1.5px);
}

.timeline-slider {
  background: linear-gradient(90deg, color-mix(in srgb, var(--secondary) 92%, transparent), color-mix(in srgb, var(--muted) 82%, transparent));
}

.timeline-progress {
  background: linear-gradient(90deg, color-mix(in srgb, var(--primary) 85%, white 15%), color-mix(in srgb, var(--ring) 78%, white 22%));
}

.timeline-slider::-webkit-slider-thumb {
  appearance: none;
  width: 15px;
  height: 15px;
  border-radius: 0.35rem;
  border: 2px solid color-mix(in srgb, var(--ring) 80%, white 20%);
  background: var(--primary);
  cursor: pointer;
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--ring) 28%, transparent);
}

.timeline-slider::-moz-range-thumb {
  width: 15px;
  height: 15px;
  border-radius: 0.35rem;
  border: 2px solid color-mix(in srgb, var(--ring) 80%, white 20%);
  background: var(--primary);
  cursor: pointer;
}

.tm-btn {
  height: 30px;
  padding: 0 11px;
  border-radius: 0.45rem;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--secondary) 88%, transparent);
  color: var(--foreground);
  font-size: 11px;
  font-weight: 600;
  transition: all 0.16s ease;
}

.tm-btn:hover {
  border-color: color-mix(in srgb, var(--primary) 52%, var(--border));
  color: var(--foreground);
  background: color-mix(in srgb, var(--secondary) 70%, var(--primary) 30%);
}

.tm-close {
  width: 28px;
  height: 28px;
  border-radius: 0.45rem;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--secondary) 78%, transparent);
  color: var(--muted-foreground);
  font-size: 12px;
  font-weight: 700;
  line-height: 1;
  text-transform: uppercase;
}

.tm-close:hover {
  color: var(--foreground);
  border-color: color-mix(in srgb, var(--destructive) 60%, var(--border));
  background: color-mix(in srgb, var(--destructive) 16%, var(--secondary));
}

.tm-loader-logo {
  width: 54px;
  height: 54px;
  object-fit: contain;
  filter: drop-shadow(0 0 7px color-mix(in srgb, var(--ring) 38%, transparent));
}

.mini-loader-logo {
  width: 18px;
  height: 18px;
  object-fit: contain;
}

.tm-loader-wave {
  display: inline-flex;
  gap: 0.5px;
}

.tm-loader-letter {
  font-size: 12px;
  font-weight: 700;
  color: var(--primary);
  text-transform: uppercase;
  animation: tm-loader-bounce 1s ease-in-out infinite;
}

@keyframes tm-loader-bounce {
  0%,
  50%,
  100% {
    transform: translateY(0);
    opacity: 0.5;
  }
  25% {
    transform: translateY(-3px);
    opacity: 1;
  }
}

.snapshot-chip {
  border-radius: 0.45rem;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--secondary) 74%, transparent);
  padding: 0.45rem 0.55rem;
}

.chip-label {
  display: block;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--muted-foreground);
}

.chip-value {
  display: block;
  margin-top: 0.2rem;
  font-size: 11px;
  color: var(--foreground);
  font-weight: 600;
}

.crumb-btn {
  height: 24px;
  padding: 0 8px;
  border-radius: 0.35rem;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--secondary) 88%, transparent);
  color: var(--muted-foreground);
  font-size: 10px;
}

.crumb-btn:hover {
  color: var(--foreground);
  border-color: color-mix(in srgb, var(--primary) 48%, var(--border));
}

.entry-row {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  border-radius: 0.4rem;
  padding: 6px 8px;
  font-size: 11px;
  border: 1px solid var(--border);
  color: var(--foreground);
  text-align: left;
}

.entry-row:hover {
  border-color: color-mix(in srgb, var(--primary) 55%, var(--border));
  background: color-mix(in srgb, var(--primary) 12%, var(--secondary));
}

.entry-icon-wrap {
  width: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.entry-lang {
  margin-left: auto;
  flex-shrink: 0;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.04em;
  padding: 1px 6px;
  border-radius: 0.35rem;
  border: 1px solid color-mix(in srgb, var(--primary) 35%, var(--border));
  background: color-mix(in srgb, var(--primary) 12%, var(--secondary));
  color: color-mix(in srgb, var(--primary) 75%, white);
}

.snapshot-preview {
  max-height: 300px;
  overflow: auto;
  background: color-mix(in srgb, var(--secondary) 84%, transparent);
  border: 1px solid var(--border);
  border-radius: 0.45rem;
  padding: 8px;
  color: var(--foreground);
  font-family: Consolas, "Courier New", monospace;
  font-size: 11px;
  line-height: 1.5;
  white-space: pre;
}
</style>

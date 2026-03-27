<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { X, FileText, Pencil, ChevronUp, ChevronDown, Undo2, Eye, Edit3, Save, RotateCcw } from "lucide-vue-next";
import type { FileDiff, DiffLine } from "@/types";
import { highlightCodeLine, splitFilePath } from "@/shared/codeView";

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

type ViewMode = "diff" | "file-diff" | "edit";
const viewMode = ref<ViewMode>("diff");

const diff = ref<FileDiff | null>(null);
const fileContent = ref<string>("");
const originalContent = ref<string>(""); // Content before changes (for full file view)
const editContent = ref<string>("");
const loading = ref(true);
const loadingFileContent = ref(false);
const error = ref<string | null>(null);
const currentHunkIndex = ref(0);
const saving = ref(false);
const hasUnsavedChanges = ref(false);
const highlightedLineCache = ref(new Map<string, string>());

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

watch(
  () => [props.filePath, props.staged, props.commitSha],
  () => {
    currentHunkIndex.value = 0;
    reload();
  }
);

watch(viewMode, (mode) => {
  if (mode === "edit") {
    loadFileForEdit();
  } else if (mode === "file-diff") {
    loadFileContentAsync();
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
    
    fileContent.value = await invoke<string>("get_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
      sha: null,
    });
    
    // Allow UI to update
    await nextTick();
    await new Promise(resolve => requestAnimationFrame(resolve));
    
    try {
      originalContent.value = await invoke<string>("get_file_content", {
        path: props.repoPath,
        filePath: props.filePath,
        sha: "HEAD",
      });
    } catch {
      originalContent.value = "";
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loadingFileContent.value = false;
  }
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
      
      const newHash = JSON.stringify(newDiff.hunks.map(h => h.lines.length));
      if (newHash !== lastFileHash) {
        lastFileHash = newHash;
        diff.value = newDiff;
        if (viewMode.value === "file-diff") {
          fileContent.value = await invoke<string>("get_file_content", {
            path: props.repoPath,
            filePath: props.filePath,
            sha: null,
          });
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
}

const fullFileLines = computed((): FullFileLine[] => {
  if (!diff.value || !fileContent.value) return [];
  
  const lines = fileContent.value.split('\n');
  const result: FullFileLine[] = [];
  
  const additionLines = new Map<number, { content: string; hunkIdx: number }>();
  const deletionsByHunk = new Map<number, DiffLine[]>();
  
  diff.value.hunks.forEach((hunk, hunkIdx) => {
    const deletions: DiffLine[] = [];
    hunk.lines.forEach(line => {
      if (line.line_type === 'addition' && line.new_line_no) {
        additionLines.set(line.new_line_no, { content: line.content, hunkIdx });
      } else if (line.line_type === 'deletion') {
        deletions.push(line);
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
        deletions.forEach(del => {
          result.push({
            lineNo: 0,
            oldLineNo: del.old_line_no,
            content: del.content.replace(/\n$/, ''),
            type: 'deletion',
            hunkIdx: addInfo.hunkIdx,
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
      });
    } else {
      result.push({
        lineNo,
        oldLineNo: lineNo, 
        content: lines[i],
        type: 'context',
        hunkIdx: null,
      });
    }
  }
  
  return result;
});

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

function getHighlightedLine(lineText: string, key: string): string {
  const cached = highlightedLineCache.value.get(key);
  if (cached !== undefined) {
    return cached;
  }

  const highlighted = highlightCodeLine(lineText, props.filePath);
  highlightedLineCache.value.set(key, highlighted);
  return highlighted;
}

function getHighlightedDiffLine(hunkIdx: number, lineIdx: number, line: DiffLine): string {
  const lineText = line.content.replace(/\n$/, "");
  const key = `${hunkIdx}:${lineIdx}:${line.line_type}:${lineText}`;
  return getHighlightedLine(lineText, key);
}

function getHighlightedFileLine(rowIdx: number, lineText: string): string {
  return getHighlightedLine(lineText, `file:${rowIdx}:${lineText}`);
}

function onEditInput() {
  hasUnsavedChanges.value = editContent.value !== fileContent.value;
}
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
        <div class="text-[var(--muted-foreground)]">Loading...</div>
      </div>

      <div v-else-if="error" class="flex flex-col items-center justify-center h-full gap-2">
        <div class="text-red-400">{{ error }}</div>
        <button @click="reload" class="text-xs text-[var(--diff-link)] hover:underline">Retry</button>
      </div>

      <div v-else-if="diff?.is_binary" class="flex items-center justify-center h-full">
        <div class="text-[var(--muted-foreground)]">Binary file - cannot display diff</div>
      </div>

      <div v-else-if="viewMode === 'diff' && diff" class="min-w-fit">
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
                <div class="w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px]">
                  {{ line.old_line_no ?? '' }}
                </div>
                <div class="w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px]">
                  {{ line.new_line_no ?? '' }}
                </div>
                <div 
                  class="w-5 flex-shrink-0 text-center select-none font-bold"
                  :class="line.line_type === 'addition' ? 'text-[var(--diff-sign-add)]' : line.line_type === 'deletion' ? 'text-[var(--diff-sign-del)]' : 'text-[var(--diff-sign-neutral)]'"
                >
                  {{ linePrefix(line.line_type) }}
                </div>
                <pre 
                  class="flex-1 px-1.5 whitespace-pre overflow-x-auto"
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
          <div class="text-[var(--muted-foreground)]">Loading file content...</div>
        </div>
        <div v-else-if="fullFileLines.length > 0" class="relative" :style="{ height: totalFileHeight + 'px' }">
          <div
            v-for="item in visibleFileLines"
            :key="item.idx"
            :class="['flex absolute left-0 right-0', lineClass(item.line.type)]"
            :style="{ top: (item.idx * LINE_HEIGHT) + 'px', height: LINE_HEIGHT + 'px' }"
          >
            <div class="w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px] leading-[20px]">
              {{ item.line.oldLineNo ?? '' }}
            </div>
            <div class="w-10 flex-shrink-0 text-right pr-1.5 text-[var(--diff-line-number)] select-none border-r border-[var(--diff-border)] text-[11px] leading-[20px]">
              {{ item.line.lineNo || '' }}
            </div>
            <div 
              class="w-5 flex-shrink-0 text-center select-none font-bold leading-[20px]"
              :class="item.line.type === 'addition' ? 'text-[var(--diff-sign-add)]' : item.line.type === 'deletion' ? 'text-[var(--diff-sign-del)]' : 'text-[var(--diff-sign-neutral)]'"
            >
              {{ linePrefix(item.line.type) }}
            </div>
            <pre 
              class="flex-1 px-1.5 whitespace-pre overflow-x-auto leading-[20px] m-0"
              :class="item.line.type === 'addition' ? 'text-[var(--diff-add-fg)]' : item.line.type === 'deletion' ? 'text-[var(--diff-del-fg)]' : 'text-[var(--diff-text)]'"
            ><code class="hljs bg-transparent" v-html="getHighlightedFileLine(item.idx, item.line.content)"></code></pre>
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
    </div>
  </div>
</template>


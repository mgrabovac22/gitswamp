<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { X, FileText, Pencil, ChevronUp, ChevronDown, Undo2, Eye, Edit3, Save, RotateCcw } from "lucide-vue-next";
import type { FileDiff, DiffLine } from "@/types";

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

// View modes: diff (hunks only), file-diff (whole file with highlights), edit
type ViewMode = "diff" | "file-diff" | "edit";
const viewMode = ref<ViewMode>("diff");

const diff = ref<FileDiff | null>(null);
const fileContent = ref<string>("");
const originalContent = ref<string>(""); // Content before changes (for full file view)
const editContent = ref<string>("");
const loading = ref(true);
const error = ref<string | null>(null);
const currentHunkIndex = ref(0);
const saving = ref(false);
const hasUnsavedChanges = ref(false);

// File watcher for live updates (only for unstaged working changes)
let fileWatchInterval: ReturnType<typeof setInterval> | null = null;
let lastFileHash = "";

const isWorkingChanges = computed(() => !props.commitSha);
const isUnstaged = computed(() => isWorkingChanges.value && !props.staged);

// Watch for file/staged prop changes - reload when switching files
watch(
  () => [props.filePath, props.staged, props.commitSha],
  () => {
    currentHunkIndex.value = 0;
    reload();
  }
);

// Watch for view mode changes
watch(viewMode, (mode) => {
  if (mode === "edit") {
    loadFileForEdit();
  } else {
    reload();
  }
});

async function reload() {
  if (viewMode.value === "edit") return;
  
  loading.value = true;
  error.value = null;
  
  try {
    // Always load diff
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
    
    // For file-diff mode, load both current and original content
    if (viewMode.value === "file-diff") {
      fileContent.value = await invoke<string>("get_file_content", {
        path: props.repoPath,
        filePath: props.filePath,
        sha: null,
      });
      // Get original content from HEAD for comparison
      try {
        originalContent.value = await invoke<string>("get_file_content", {
          path: props.repoPath,
          filePath: props.filePath,
          sha: "HEAD",
        });
      } catch {
        originalContent.value = "";
      }
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
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

// Live file watching for unstaged changes
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
      // Ignore errors during polling
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

// Build full file lines with diff information
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
  
  // Build a map of line numbers that are additions (new_line_no -> line info)
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
  
  // Process each line in the current file
  let deletionInsertPoint: number | null = null;
  
  for (let i = 0; i < lines.length; i++) {
    const lineNo = i + 1;
    const addInfo = additionLines.get(lineNo);
    
    // Check if we need to insert deletions before this addition
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

// Word-level diff helpers
function getWordDiffPairs(lines: DiffLine[]): Map<number, { del: DiffLine; add: DiffLine }[]> {
  const pairs = new Map<number, { del: DiffLine; add: DiffLine }[]>();
  let i = 0;
  while (i < lines.length) {
    if (lines[i].line_type === 'deletion') {
      const delStart = i;
      while (i < lines.length && lines[i].line_type === 'deletion') i++;
      const addStart = i;
      while (i < lines.length && lines[i].line_type === 'addition') i++;
      const addEnd = i;
      
      const numDels = addStart - delStart;
      const numAdds = addEnd - addStart;
      const numPairs = Math.min(numDels, numAdds);
      
      for (let j = 0; j < numPairs; j++) {
        const delIdx = delStart + j;
        const addIdx = addStart + j;
        const pair = { del: lines[delIdx], add: lines[addIdx] };
        if (!pairs.has(delIdx)) pairs.set(delIdx, []);
        pairs.get(delIdx)!.push(pair);
        if (!pairs.has(addIdx)) pairs.set(addIdx, []);
        pairs.get(addIdx)!.push(pair);
      }
    } else {
      i++;
    }
  }
  return pairs;
}

function computeInlineDiff(oldStr: string, newStr: string): { old: { text: string; highlight: boolean }[]; new: { text: string; highlight: boolean }[] } {
  const oldChars = oldStr.split('');
  const newChars = newStr.split('');
  
  let prefixLen = 0;
  while (prefixLen < oldChars.length && prefixLen < newChars.length && oldChars[prefixLen] === newChars[prefixLen]) {
    prefixLen++;
  }
  
  let suffixLen = 0;
  while (
    suffixLen < oldChars.length - prefixLen && 
    suffixLen < newChars.length - prefixLen && 
    oldChars[oldChars.length - 1 - suffixLen] === newChars[newChars.length - 1 - suffixLen]
  ) {
    suffixLen++;
  }
  
  const oldMiddle = oldStr.substring(prefixLen, oldStr.length - suffixLen);
  const newMiddle = newStr.substring(prefixLen, newStr.length - suffixLen);
  const prefix = oldStr.substring(0, prefixLen);
  const suffix = oldStr.substring(oldStr.length - suffixLen);
  
  const oldParts: { text: string; highlight: boolean }[] = [];
  const newParts: { text: string; highlight: boolean }[] = [];
  
  if (prefix) { oldParts.push({ text: prefix, highlight: false }); newParts.push({ text: prefix, highlight: false }); }
  if (oldMiddle) oldParts.push({ text: oldMiddle, highlight: true });
  if (newMiddle) newParts.push({ text: newMiddle, highlight: true });
  if (suffix) { oldParts.push({ text: suffix, highlight: false }); newParts.push({ text: suffix, highlight: false }); }
  
  return { 
    old: oldParts.length ? oldParts : [{ text: oldStr, highlight: false }], 
    new: newParts.length ? newParts : [{ text: newStr, highlight: false }] 
  };
}

function lineClass(type: string): string {
  switch (type) {
    case "addition": return "bg-[#1a4d2e]/50";
    case "deletion": return "bg-[#4d1a1a]/50";
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

function onEditInput() {
  hasUnsavedChanges.value = editContent.value !== fileContent.value;
}
</script>

<template>
  <div class="h-full flex flex-col bg-[var(--card)] overflow-hidden">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-2.5 border-b border-[var(--border)] bg-[var(--card)] flex-shrink-0">
      <div class="flex items-center gap-3 min-w-0">
        <Pencil v-if="isWorkingChanges" class="w-4 h-4 text-[var(--muted-foreground)] flex-shrink-0" />
        <FileText v-else class="w-4 h-4 text-[var(--muted-foreground)] flex-shrink-0" />
        <span class="text-sm font-medium text-[var(--foreground)] truncate">{{ filePath }}</span>
        <span v-if="diff?.old_path" class="text-xs text-[var(--muted-foreground)] flex-shrink-0">
          (from {{ diff.old_path }})
        </span>
        <span v-if="hasUnsavedChanges" class="text-xs text-[#f59e0b] flex-shrink-0">● Unsaved</span>
      </div>
      
      <div class="flex items-center gap-2 flex-shrink-0">
        <!-- View mode buttons -->
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

        <!-- Edit mode actions -->
        <template v-if="viewMode === 'edit'">
          <button
            @click="saveFile"
            :disabled="!hasUnsavedChanges || saving"
            class="flex items-center gap-1 px-2 py-1 text-xs font-medium rounded bg-[#238636] hover:bg-[#2ea043] text-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <Save class="w-3.5 h-3.5" />
            Save
          </button>
          <button
            @click="discardEditChanges"
            :disabled="!hasUnsavedChanges"
            class="flex items-center gap-1 px-2 py-1 text-xs font-medium rounded border border-[#30363d] bg-[#21262d] hover:bg-[#30363d] text-[#c9d1d9] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <RotateCcw class="w-3.5 h-3.5" />
            Discard
          </button>
        </template>

        <!-- Hunk navigation (only in diff mode) -->
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

        <!-- Close button -->
        <button
          @click="emit('close')"
          class="p-1 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
        >
          <X class="w-5 h-5" />
        </button>
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto bg-[#0d1117] font-mono text-[13px] leading-[1.5]">
      <!-- Loading -->
      <div v-if="loading" class="flex items-center justify-center h-full">
        <div class="text-[var(--muted-foreground)]">Loading...</div>
      </div>

      <!-- Error -->
      <div v-else-if="error" class="flex flex-col items-center justify-center h-full gap-2">
        <div class="text-red-400">{{ error }}</div>
        <button @click="reload" class="text-xs text-[#58a6ff] hover:underline">Retry</button>
      </div>

      <!-- Binary file -->
      <div v-else-if="diff?.is_binary" class="flex items-center justify-center h-full">
        <div class="text-[var(--muted-foreground)]">Binary file - cannot display diff</div>
      </div>

      <!-- DIFF VIEW: Show only hunks -->
      <div v-else-if="viewMode === 'diff' && diff" class="min-w-fit">
        <template v-for="(hunk, hunkIdx) in diff.hunks" :key="hunkIdx">
          <!-- Hunk header -->
          <div :id="`hunk-${hunkIdx}`" class="flex items-center justify-between bg-[#161b22] px-3 py-1.5 sticky top-0 z-10 border-y border-[#30363d]">
            <span class="text-xs text-[#58a6ff]">
              @@ -{{ hunk.old_start }},{{ hunk.old_lines }} +{{ hunk.new_start }},{{ hunk.new_lines }} @@
            </span>
            <button
              v-if="isWorkingChanges"
              @click="revertHunk(hunkIdx)"
              class="flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded border border-[#30363d] bg-[#21262d] hover:bg-[#30363d] text-[#8b949e] hover:text-[#c9d1d9] transition-colors"
            >
              <Undo2 class="w-3 h-3" />
              Revert Hunk
            </button>
          </div>

          <!-- Hunk lines -->
          <div>
            <template v-for="(line, lineIdx) in hunk.lines" :key="lineIdx">
              <div :class="['flex', lineClass(line.line_type)]">
                <div class="w-10 flex-shrink-0 text-right pr-1.5 text-[#484f58] select-none border-r border-[#21262d] text-[11px]">
                  {{ line.old_line_no ?? '' }}
                </div>
                <div class="w-10 flex-shrink-0 text-right pr-1.5 text-[#484f58] select-none border-r border-[#21262d] text-[11px]">
                  {{ line.new_line_no ?? '' }}
                </div>
                <div 
                  class="w-5 flex-shrink-0 text-center select-none font-bold"
                  :class="line.line_type === 'addition' ? 'text-[#3fb950]' : line.line_type === 'deletion' ? 'text-[#f85149]' : 'text-[#484f58]'"
                >
                  {{ linePrefix(line.line_type) }}
                </div>
                <pre 
                  class="flex-1 px-1.5 whitespace-pre overflow-x-auto"
                  :class="line.line_type === 'addition' ? 'text-[#aff5b4]' : line.line_type === 'deletion' ? 'text-[#ffa198]' : 'text-[#c9d1d9]'"
                ><template v-if="line.line_type === 'deletion' || line.line_type === 'addition'"><template v-for="(part, pIdx) in (() => {
                    const pairs = getWordDiffPairs(hunk.lines);
                    const pairData = pairs.get(lineIdx);
                    if (pairData && pairData.length > 0) {
                      const pair = pairData[0];
                      const inlineDiff = computeInlineDiff(
                        pair.del.content.replace(/\n$/, ''),
                        pair.add.content.replace(/\n$/, '')
                      );
                      return line.line_type === 'deletion' ? inlineDiff.old : inlineDiff.new;
                    }
                    return [{ text: line.content.replace(/\n$/, ''), highlight: false }];
                  })()" :key="pIdx"><span 
                      v-if="part.highlight" 
                      :class="line.line_type === 'addition' ? 'bg-[#2ea043]/60 rounded-sm' : 'bg-[#b62324]/60 rounded-sm'"
                    >{{ part.text }}</span><template v-else>{{ part.text }}</template></template></template><template v-else>{{ line.content.replace(/\n$/, '') }}</template></pre>
              </div>
            </template>
          </div>
        </template>

        <div v-if="diff.hunks.length === 0" class="flex items-center justify-center h-64 text-[var(--muted-foreground)]">
          No changes in this file
        </div>
      </div>

      <!-- FILE-DIFF VIEW: Whole file with change highlights -->
      <div v-else-if="viewMode === 'file-diff'" class="min-w-fit">
        <template v-for="(line, idx) in fullFileLines" :key="idx">
          <div :class="['flex', lineClass(line.type)]">
            <div class="w-10 flex-shrink-0 text-right pr-1.5 text-[#484f58] select-none border-r border-[#21262d] text-[11px]">
              {{ line.oldLineNo ?? '' }}
            </div>
            <div class="w-10 flex-shrink-0 text-right pr-1.5 text-[#484f58] select-none border-r border-[#21262d] text-[11px]">
              {{ line.lineNo || '' }}
            </div>
            <div 
              class="w-5 flex-shrink-0 text-center select-none font-bold"
              :class="line.type === 'addition' ? 'text-[#3fb950]' : line.type === 'deletion' ? 'text-[#f85149]' : 'text-[#484f58]'"
            >
              {{ linePrefix(line.type) }}
            </div>
            <pre 
              class="flex-1 px-1.5 whitespace-pre overflow-x-auto"
              :class="line.type === 'addition' ? 'text-[#aff5b4]' : line.type === 'deletion' ? 'text-[#ffa198]' : 'text-[#c9d1d9]'"
            >{{ line.content }}</pre>
          </div>
        </template>
        <div v-if="fullFileLines.length === 0 && !loading" class="flex items-center justify-center h-64 text-[var(--muted-foreground)]">
          No file content available
        </div>
      </div>

      <!-- EDIT VIEW -->
      <div v-else-if="viewMode === 'edit'" class="min-w-fit h-full">
        <textarea
          v-model="editContent"
          @input="onEditInput"
          class="w-full h-full bg-[#0d1117] text-[#c9d1d9] p-3 resize-none outline-none font-mono text-[13px] leading-[1.5]"
          spellcheck="false"
        ></textarea>
      </div>
    </div>
  </div>
</template>

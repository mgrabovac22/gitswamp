<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { X, Check, ChevronLeft, ChevronRight, Save, RotateCcw, AlertTriangle } from "lucide-vue-next";
import { highlightCodeLine, splitFilePath } from "@/shared/codeView";

const props = defineProps<{
  repoPath: string;
  filePath: string;
}>();

const emit = defineEmits<{
  close: [];
  resolved: [];
}>();

interface ConflictHunk {
  startLine: number;
  oursStartLine: number;
  theirsStartLine: number;
  oursLines: string[];
  theirsLines: string[];
  baseLines?: string[];
  contextBefore: string[];
  contextAfter: string[];
  contextBeforeStartLine: number;
  contextAfterStartLine: number;
}

interface LineSelection {
  side: 'ours' | 'theirs' | 'both' | 'none';
  oursSelected: boolean[];
  theirsSelected: boolean[];
}

const loading = ref(true);
const saving = ref(false);
const error = ref<string | null>(null);

const fileContent = ref<string>("");
const hunks = ref<ConflictHunk[]>([]);
const selections = ref<LineSelection[]>([]);
const hasMarkers = ref(false);
const simpleResolution = ref<'keep-modified' | 'keep-base' | 'delete' | null>(null);
const CONTEXT_LINES = 5;
const highlightedLineCache = ref(new Map<string, string>());
const fileNameParts = computed(() => splitFilePath(props.filePath));

// Parse conflict markers from file content
function parseConflicts(content: string): ConflictHunk[] {
  const lines = content.split('\n');
  const conflicts: ConflictHunk[] = [];
  
  let i = 0;
  while (i < lines.length) {
    if (lines[i].startsWith('<<<<<<<')) {
      const markerStartIdx = i;
      const startLine = markerStartIdx;
      // Displayed line numbers are 1-based for users.
      const oursStartLine = markerStartIdx + 2;
      const oursLines: string[] = [];
      const baseLines: string[] = [];
      const theirsLines: string[] = [];
      let theirsStartLine = markerStartIdx + 2;
      
      i++; // skip <<<<<<< marker
      
      // Read "ours" section
      while (i < lines.length && !lines[i].startsWith('|||||||') && !lines[i].startsWith('=======')) {
        oursLines.push(lines[i]);
        i++;
      }
      
      // Check for base section (diff3 style)
      if (i < lines.length && lines[i].startsWith('|||||||')) {
        i++; // skip ||||||| marker
        while (i < lines.length && !lines[i].startsWith('=======')) {
          baseLines.push(lines[i]);
          i++;
        }
      }
      
      // Skip ======= marker
      if (i < lines.length && lines[i].startsWith('=======')) {
        i++;
        theirsStartLine = i + 1;
      }
      
      // Read "theirs" section
      while (i < lines.length && !lines[i].startsWith('>>>>>>>')) {
        theirsLines.push(lines[i]);
        i++;
      }
      
      // Skip >>>>>>> marker
      if (i < lines.length && lines[i].startsWith('>>>>>>>')) {
        i++;
      }
      
      // Gather context before conflict (up to 3 lines)
      const beforeStartIdx = Math.max(0, startLine - CONTEXT_LINES);
      const contextBefore: string[] = [];
      for (let j = beforeStartIdx; j < startLine; j++) {
        contextBefore.push(lines[j]);
      }
      
      // Gather context after conflict (up to CONTEXT_LINES lines)
      const afterStartIdx = i;
      const contextAfter: string[] = [];
      for (let j = afterStartIdx; j < Math.min(lines.length, afterStartIdx + CONTEXT_LINES); j++) {
        contextAfter.push(lines[j]);
      }
      
      conflicts.push({
        startLine,
        oursStartLine,
        theirsStartLine,
        oursLines,
        theirsLines,
        baseLines: baseLines.length > 0 ? baseLines : undefined,
        contextBefore,
        contextAfter,
        contextBeforeStartLine: beforeStartIdx + 1,
        contextAfterStartLine: afterStartIdx + 1,
      });
    } else {
      i++;
    }
  }
  
  return conflicts;
}

// Initialize selections for each hunk
function initSelections(conflictHunks: ConflictHunk[]): LineSelection[] {
  return conflictHunks.map(hunk => ({
    side: 'none',
    oursSelected: new Array(hunk.oursLines.length).fill(false),
    theirsSelected: new Array(hunk.theirsLines.length).fill(false),
  }));
}

async function loadFile() {
  loading.value = true;
  error.value = null;
  simpleResolution.value = null;
  
  try {
    highlightedLineCache.value.clear();
    fileContent.value = await invoke<string>("get_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
      sha: null,
    });
    
    // Check if file has actual conflict markers
    hasMarkers.value = fileContent.value.includes('<<<<<<<') && 
                       fileContent.value.includes('=======') && 
                       fileContent.value.includes('>>>>>>>');
    
    if (hasMarkers.value) {
      hunks.value = parseConflicts(fileContent.value);
      selections.value = initSelections(hunks.value);
    } else {
      // No markers - show options to user
      hunks.value = [];
      selections.value = [];
    }
  } catch (e) {
    error.value = `Failed to load file: ${String(e)}`;
  } finally {
    loading.value = false;
  }
}

function toggleOursLine(hunkIdx: number, lineIdx: number) {
  const sel = selections.value[hunkIdx];
  sel.oursSelected[lineIdx] = !sel.oursSelected[lineIdx];
  sel.side = 'none'; // Manual selection
}

function selectAllOurs(hunkIdx: number) {
  const sel = selections.value[hunkIdx];
  sel.side = 'ours';
  sel.oursSelected = new Array(hunks.value[hunkIdx].oursLines.length).fill(true);
  sel.theirsSelected = new Array(hunks.value[hunkIdx].theirsLines.length).fill(false);
}

function selectAllTheirs(hunkIdx: number) {
  const sel = selections.value[hunkIdx];
  sel.side = 'theirs';
  sel.oursSelected = new Array(hunks.value[hunkIdx].oursLines.length).fill(false);
  sel.theirsSelected = new Array(hunks.value[hunkIdx].theirsLines.length).fill(true);
}

function selectBoth(hunkIdx: number) {
  const sel = selections.value[hunkIdx];
  sel.side = 'both';
  sel.oursSelected = new Array(hunks.value[hunkIdx].oursLines.length).fill(true);
  sel.theirsSelected = new Array(hunks.value[hunkIdx].theirsLines.length).fill(true);
}

function selectNone(hunkIdx: number) {
  const sel = selections.value[hunkIdx];
  sel.side = 'none';
  sel.oursSelected = new Array(hunks.value[hunkIdx].oursLines.length).fill(false);
  sel.theirsSelected = new Array(hunks.value[hunkIdx].theirsLines.length).fill(false);
}

function formatRange(start: number, length: number): string {
  if (length <= 1) return `${start}`;
  return `${start}-${start + length - 1}`;
}

function toggleTheirsLine(hunkIdx: number, lineIdx: number) {
  const sel = selections.value[hunkIdx];
  sel.theirsSelected[lineIdx] = !sel.theirsSelected[lineIdx];
  sel.side = 'none'; // Manual selection
}

// Build the resolved file content
const resolvedLines = computed(() => {
  const lines = fileContent.value.split('\n');
  const result: string[] = [];
  
  let lineIdx = 0;
  let hunkIdx = 0;
  
  while (lineIdx < lines.length) {
    if (hunkIdx < hunks.value.length && lineIdx === hunks.value[hunkIdx].startLine) {
      // Insert selected lines for this hunk
      const hunk = hunks.value[hunkIdx];
      const sel = selections.value[hunkIdx];
      
      // Add selected "ours" lines
      for (let i = 0; i < hunk.oursLines.length; i++) {
        if (sel.oursSelected[i]) {
          result.push(hunk.oursLines[i]);
        }
      }
      
      // Add selected "theirs" lines
      for (let i = 0; i < hunk.theirsLines.length; i++) {
        if (sel.theirsSelected[i]) {
          result.push(hunk.theirsLines[i]);
        }
      }
      
      // Skip conflict markers and content
      while (lineIdx < lines.length && !lines[lineIdx].startsWith('>>>>>>>')) {
        lineIdx++;
      }
      lineIdx++; // skip >>>>>>> marker
      hunkIdx++;
    } else {
      result.push(lines[lineIdx]);
      lineIdx++;
    }
  }
  
  return result;
});

const previewContent = computed(() => resolvedLines.value.join('\n'));

const simpleResolutionContent = computed(() => {
  if (!simpleResolution.value || hasMarkers.value) return fileContent.value;
  
  // For delete, return empty string
  if (simpleResolution.value === 'delete') {
    return '';
  }
  
  const lines = fileContent.value.split('\n');
  const result: string[] = [];
  let i = 0;
  
  while (i < lines.length) {
    if (lines[i].startsWith('<<<<<<<')) {
      const oursContent: string[] = [];
      const theirsContent: string[] = [];
      
      i++; // skip <<<<<<<
      while (i < lines.length && !lines[i].startsWith('=======')) {
        oursContent.push(lines[i]);
        i++;
      }
      
      i++; // skip =======
      while (i < lines.length && !lines[i].startsWith('>>>>>>>')) {
        theirsContent.push(lines[i]);
        i++;
      }
      
      i++; // skip >>>>>>>
      
      // Apply resolution
      if (simpleResolution.value === 'keep-modified') {
        // Keep our version (modified)
        result.push(...oursContent);
      } else if (simpleResolution.value === 'keep-base') {
        // Keep their version (base)
        result.push(...theirsContent);
      }
    } else {
      result.push(lines[i]);
      i++;
    }
  }
  
  return result.join('\n');
});

const displayContent = computed(() => {
  return hasMarkers.value ? previewContent.value : simpleResolutionContent.value;
});

const allHunksResolved = computed(() => {
  if (!hasMarkers.value) {
    return simpleResolution.value !== null;
  }
  return selections.value.every(sel => 
    sel.oursSelected.some(s => s) || sel.theirsSelected.some(s => s)
  );
});

async function saveResolution() {
  if (!allHunksResolved.value) return;
  
  saving.value = true;
  error.value = null;
  
  try {
    await invoke("save_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
      content: displayContent.value,
    });
    
    // Stage the resolved file
    await invoke("stage_file", {
      path: props.repoPath,
      filePath: props.filePath,
    });
    
    emit("resolved");
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function reset() {
  selections.value = initSelections(hunks.value);
}

function getHighlightedConflictLine(section: string, hunkIdx: number, lineIdx: number, lineText: string): string {
  const key = `${section}:${hunkIdx}:${lineIdx}:${lineText}`;
  const cached = highlightedLineCache.value.get(key);
  if (cached !== undefined) {
    return cached;
  }

  const highlighted = highlightCodeLine(lineText, props.filePath);
  highlightedLineCache.value.set(key, highlighted);
  return highlighted;
}

onMounted(() => {
  loadFile();
});

watch(() => props.filePath, () => {
  loadFile();
});
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm" @click.self="emit('close')">
    <div class="w-[98vw] max-w-[1600px] h-[95vh] bg-[var(--card)] rounded-xl border border-[var(--border)] shadow-2xl flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-3 border-b border-[var(--border)] bg-[var(--card)] flex-shrink-0">
        <div class="flex items-center gap-3">
          <AlertTriangle class="w-5 h-5 text-[#f59e0b]" />
          <div>
            <span class="text-sm font-semibold text-[var(--foreground)]">{{ hasMarkers ? 'Resolve Conflicts - Line by Line' : 'Resolve Conflicts - Choose Method' }}</span>
            <div class="text-xs text-[var(--foreground)] font-semibold truncate max-w-[60vw]" :title="filePath">{{ fileNameParts.fileName }}</div>
            <div class="text-[10px] text-[var(--muted-foreground)] truncate max-w-[60vw]">{{ fileNameParts.directory || '.' }}</div>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button
            @click="reset"
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded border border-[var(--border)] bg-[var(--secondary)] hover:bg-[var(--accent)]/10 text-[var(--foreground)] transition-colors"
          >
            <RotateCcw class="w-3.5 h-3.5" />
            Reset
          </button>
          <button
            @click="saveResolution"
            :disabled="!allHunksResolved || saving"
            class="flex items-center gap-1.5 px-4 py-1.5 text-xs font-medium rounded bg-[#238636] hover:bg-[#2ea043] text-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <Save class="w-3.5 h-3.5" />
            {{ saving ? 'Saving...' : 'Save & Stage' }}
          </button>
          <button
            @click="emit('close')"
            class="p-1.5 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors"
          >
            <X class="w-5 h-5" />
          </button>
        </div>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-hidden flex flex-col">
        <div v-if="loading" class="flex items-center justify-center h-full">
          <div class="text-[var(--muted-foreground)]">Loading conflict file...</div>
        </div>

        <div v-else-if="error" class="flex flex-col items-center justify-center h-full gap-2">
          <AlertTriangle class="w-8 h-8 text-[#ef4444]" />
          <div class="text-[#ef4444] font-medium">{{ error }}</div>
          <button @click="loadFile" class="text-xs text-[var(--primary)] hover:underline">Retry</button>
        </div>

        <div v-else class="flex-1 overflow-y-auto space-y-4 p-4">
          <!-- Simple Mode (No Markers) -->
          <div v-if="!hasMarkers" class="flex items-center justify-center h-full">
            <div class="bg-[var(--card)] border border-[#f59e0b] rounded-lg shadow-2xl p-8 max-w-md w-full">
              <div class="text-center mb-8">
                <div class="w-12 h-12 bg-[#f59e0b]/20 rounded-lg flex items-center justify-center mx-auto mb-4">
                  <AlertTriangle class="w-6 h-6 text-[#f59e0b]" />
                </div>
                <h3 class="text-lg font-semibold text-[var(--foreground)] mb-2">Resolve Conflict</h3>
                <p class="text-sm text-[var(--muted-foreground)]">{{ filePath }}</p>
              </div>
              
              <p class="text-xs text-[#f59e0b] text-center mb-6">
                No clear conflict markers found. Choose how to resolve:
              </p>

              <!-- Result Preview -->
              <div class="mb-6">
                <div class="text-xs font-semibold text-[#f59e0b] mb-2">PREVIEW (First 50 lines):</div>
                <div class="bg-[var(--background)] border border-[var(--border)] rounded p-3 font-mono text-[11px] max-h-64 overflow-y-auto">
                  <pre class="whitespace-pre-wrap text-[#8b949e]">{{ displayContent.split('\n').slice(0, 50).join('\n') }}</pre>
                  <div v-if="displayContent.split('\n').length > 50" class="text-xs text-[#f59e0b] mt-2 p-2 bg-[#f59e0b]/10 rounded">
                    ... and {{ displayContent.split('\n').length - 50 }} more lines
                  </div>
                </div>
              </div>

              <div class="flex flex-col gap-2">
                <div class="flex gap-2">
                  <button
                    @click="simpleResolution = 'keep-modified'; saveResolution()"
                    :disabled="saving"
                    class="flex-1 px-4 py-2 rounded bg-[#238636] hover:bg-[#2ea043] text-white text-xs font-semibold transition-colors disabled:opacity-50"
                  >
                    {{ saving && simpleResolution === 'keep-modified' ? 'Saving...' : 'Keep Modified' }}
                  </button>
                  <button
                    @click="simpleResolution = 'keep-base'; saveResolution()"
                    :disabled="saving"
                    class="flex-1 px-4 py-2 rounded bg-[#1f6feb] hover:bg-[#388bfd] text-white text-xs font-semibold transition-colors disabled:opacity-50"
                  >
                    {{ saving && simpleResolution === 'keep-base' ? 'Saving...' : 'Keep Base' }}
                  </button>
                </div>
                <div class="flex gap-2">
                  <button
                    @click="simpleResolution = 'delete'; saveResolution()"
                    :disabled="saving"
                    class="flex-1 px-4 py-2 rounded bg-[#da3633] hover:bg-[#f85149] text-white text-xs font-semibold transition-colors disabled:opacity-50"
                  >
                    {{ saving && simpleResolution === 'delete' ? 'Deleting...' : 'Delete File' }}
                  </button>
                  <button
                    @click="emit('close')"
                    class="flex-1 px-4 py-2 rounded border border-[var(--border)] bg-[var(--card)] hover:bg-[var(--secondary)] text-[var(--foreground)] text-xs font-medium transition-colors"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Line-by-Line Mode (With Markers) -->
          <div v-else class="space-y-4">
          <!-- Conflict Hunks -->
          <div v-for="(hunk, hunkIdx) in hunks" :key="hunkIdx" class="border border-[var(--border)] rounded-lg overflow-hidden">
            <!-- Hunk Header - Always Expanded -->
            <div 
              class="flex items-center justify-between px-4 py-3 bg-[var(--secondary)] border-b border-[var(--border)]"
            >
              <div class="flex items-center gap-3 flex-1">
                <span class="text-xs font-semibold text-[var(--foreground)]">Conflict {{ hunkIdx + 1 }} of {{ hunks.length }}</span>
                <span class="text-xs text-[var(--muted-foreground)]">
                  ({{ hunk.oursLines.length }} ours, {{ hunk.theirsLines.length }} theirs{{ hunk.baseLines ? `, ${hunk.baseLines.length} base` : '' }})
                </span>
                <span class="text-xs text-[var(--muted-foreground)]">around line {{ hunk.startLine + 1 }} • context ±{{ CONTEXT_LINES }}</span>
              </div>
              
              <!-- Quick Actions -->
              <div class="flex items-center gap-2" @click.stop>
                <button
                  @click.stop="selectAllOurs(hunkIdx)"
                  :class="[
                    'px-2.5 py-1 text-[10px] font-semibold rounded transition-colors',
                    selections[hunkIdx].side === 'ours'
                      ? 'bg-[#238636] text-white'
                      : 'bg-[var(--card)] text-[var(--muted-foreground)] hover:bg-[#238636]/20 hover:text-[#238636] border border-[var(--border)]'
                  ]"
                  title="Take all from your branch"
                >
                  <ChevronLeft class="w-3 h-3 inline mr-0.5" />
                  Ours
                </button>
                <button
                  @click.stop="selectBoth(hunkIdx)"
                  :class="[
                    'px-2.5 py-1 text-[10px] font-semibold rounded transition-colors',
                    selections[hunkIdx].side === 'both'
                      ? 'bg-[var(--primary)] text-white'
                      : 'bg-[var(--card)] text-[var(--muted-foreground)] hover:bg-[var(--primary)]/20 hover:text-[var(--primary)] border border-[var(--border)]'
                  ]"
                  title="Keep both sections"
                >
                  Both
                </button>
                <button
                  @click.stop="selectAllTheirs(hunkIdx)"
                  :class="[
                    'px-2.5 py-1 text-[10px] font-semibold rounded transition-colors',
                    selections[hunkIdx].side === 'theirs'
                      ? 'bg-[#1f6feb] text-white'
                      : 'bg-[var(--card)] text-[var(--muted-foreground)] hover:bg-[#1f6feb]/20 hover:text-[#1f6feb] border border-[var(--border)]'
                  ]"
                  title="Take all from incoming branch"
                >
                  Theirs
                  <ChevronRight class="w-3 h-3 inline ml-0.5" />
                </button>
                <button
                  @click.stop="selectNone(hunkIdx)"
                  :class="[
                    'px-2.5 py-1 text-[10px] font-semibold rounded transition-colors',
                    'bg-[var(--card)] text-[var(--muted-foreground)] hover:bg-[#ef4444]/20 hover:text-[#ef4444] border border-[var(--border)]'
                  ]"
                  title="Clear all selections"
                >
                  Clear
                </button>
              </div>
            </div>

            <!-- Content - Always Visible (Not Collapsable) -->
            <div class="bg-[var(--card)]">
              <!-- Side by side editor -->
              <div class="flex divide-x divide-[var(--border)] max-h-full overflow-hidden">
                <!-- OURS (Left) -->
                <div class="flex-1 flex flex-col min-w-0 bg-[#1a4d2e]/5">
                  <div class="px-3 py-2 text-[10px] font-semibold text-[#3fb950] border-b border-[var(--border)] bg-[#238636]/10 flex-shrink-0">
                    YOUR CHANGES (Lines {{ formatRange(hunk.oursStartLine, hunk.oursLines.length) }})
                  </div>
                  <div class="flex-1 overflow-y-auto font-mono text-[11px]">
                    <!-- Context before -->
                    <div v-if="hunk.contextBefore.length > 0" class="border-b border-[var(--border)]/20 bg-[#1a4d2e]/2">
                      <div v-for="(line, idx) in hunk.contextBefore" :key="'before-' + idx" class="flex items-start text-[#666]">
                        <div class="w-7 flex-shrink-0 text-right pr-1 text-[#484f58] select-none text-[10px] border-r border-[var(--border)]/20">
                          {{ hunk.contextBeforeStartLine + idx }}
                        </div>
                        <div class="w-7 flex-shrink-0"></div>
                        <pre class="flex-1 px-2 py-0.5 whitespace-pre-wrap text-[#666]"><code class="hljs bg-transparent" v-html="getHighlightedConflictLine('ours-context-before', hunkIdx, idx, line)"></code></pre>
                      </div>
                    </div>

                    <!-- Conflict lines -->
                    <div
                      v-for="(line, lineIdx) in hunk.oursLines"
                      :key="'o-' + lineIdx"
                      class="flex items-start hover:bg-[#238636]/10 transition-colors border-b border-[var(--border)]/30"
                      @click="toggleOursLine(hunkIdx, lineIdx)"
                    >
                      <div class="w-7 flex-shrink-0 flex items-center justify-center border-r border-[var(--border)]/30 cursor-pointer hover:bg-[#238636]/15">
                        <input
                          type="checkbox"
                          :checked="selections[hunkIdx].oursSelected[lineIdx]"
                          @click.stop
                          @change="toggleOursLine(hunkIdx, lineIdx)"
                          class="w-3 h-3 rounded border-[#3fb950] text-[#238636] cursor-pointer"
                        />
                      </div>
                      <div class="w-6 flex-shrink-0 text-right pr-1 text-[#484f58] select-none text-[10px] border-r border-[var(--border)]/30">
                        {{ hunk.oursStartLine + lineIdx }}
                      </div>
                      <pre 
                        class="flex-1 px-2 py-1 whitespace-pre-wrap word-break overflow-x-auto"
                        :class="selections[hunkIdx].oursSelected[lineIdx] ? 'text-[#aff5b4] bg-[#238636]/20' : 'text-[#8b949e]'"
                      ><code class="hljs bg-transparent" v-html="getHighlightedConflictLine('ours', hunkIdx, lineIdx, line)"></code></pre>
                    </div>

                    <!-- Context after -->
                    <div v-if="hunk.contextAfter.length > 0" class="border-t border-[var(--border)]/20 bg-[#1a4d2e]/2">
                      <div v-for="(line, idx) in hunk.contextAfter" :key="'after-' + idx" class="flex items-start text-[#666]">
                        <div class="w-7 flex-shrink-0 text-right pr-1 text-[#484f58] select-none text-[10px] border-r border-[var(--border)]/20">
                          {{ hunk.contextAfterStartLine + idx }}
                        </div>
                        <div class="w-7 flex-shrink-0"></div>
                        <pre class="flex-1 px-2 py-0.5 whitespace-pre-wrap text-[#666]"><code class="hljs bg-transparent" v-html="getHighlightedConflictLine('ours-context-after', hunkIdx, idx, line)"></code></pre>
                      </div>
                    </div>

                    <div v-if="hunk.oursLines.length === 0" class="px-4 py-4 text-xs text-[var(--muted-foreground)] italic">
                      (no content)
                    </div>
                  </div>
                </div>

                <!-- Base (Middle optional) -->
                <div v-if="hunk.baseLines && hunk.baseLines.length > 0" class="w-1/3 flex flex-col min-w-0 bg-[#f59e0b]/5">
                  <div class="px-3 py-2 text-[10px] font-semibold text-[#f59e0b] border-b border-[var(--border)] bg-[#f59e0b]/10 flex-shrink-0">
                    ORIGINAL (Base)
                  </div>
                  <div class="flex-1 overflow-y-auto font-mono text-[11px]">
                    <div
                      v-for="(line, lineIdx) in hunk.baseLines"
                      :key="'base-' + lineIdx"
                      class="flex items-start border-b border-[var(--border)]/30 last:border-0"
                    >
                      <div class="w-6 flex-shrink-0 text-right pr-1 text-[#484f58] select-none text-[10px] border-r border-[var(--border)]/30">
                        {{ lineIdx + 1 }}
                      </div>
                      <pre class="flex-1 px-2 py-1 whitespace-pre-wrap word-break overflow-x-auto text-[#999]"><code class="hljs bg-transparent" v-html="getHighlightedConflictLine('base', hunkIdx, lineIdx, line)"></code></pre>
                    </div>
                    <div v-if="hunk.baseLines.length === 0" class="px-4 py-4 text-xs text-[var(--muted-foreground)] italic">
                      (no content)
                    </div>
                  </div>
                </div>

                <!-- THEIRS (Right) -->
                <div :class="['flex-1 flex flex-col min-w-0 bg-[#1f6feb]/5', hunk.baseLines && hunk.baseLines.length > 0 ? '' : '']">
                  <div class="px-3 py-2 text-[10px] font-semibold text-[#58a6ff] border-b border-[var(--border)] bg-[#1f6feb]/10 flex-shrink-0">
                    INCOMING CHANGES (Lines {{ formatRange(hunk.theirsStartLine, hunk.theirsLines.length) }})
                  </div>
                  <div class="flex-1 overflow-y-auto font-mono text-[11px]">
                    <!-- Context before -->
                    <div v-if="hunk.contextBefore.length > 0" class="border-b border-[var(--border)]/20 bg-[#1f6feb]/2">
                      <div v-for="(line, idx) in hunk.contextBefore" :key="'theirs-before-' + idx" class="flex items-start text-[#666]">
                        <div class="w-7 flex-shrink-0 text-right pr-1 text-[#484f58] select-none text-[10px] border-r border-[var(--border)]/20">
                          {{ hunk.contextBeforeStartLine + idx }}
                        </div>
                        <div class="w-7 flex-shrink-0"></div>
                        <pre class="flex-1 px-2 py-0.5 whitespace-pre-wrap text-[#666]"><code class="hljs bg-transparent" v-html="getHighlightedConflictLine('theirs-context-before', hunkIdx, idx, line)"></code></pre>
                      </div>
                    </div>

                    <!-- Conflict lines -->
                    <div
                      v-for="(line, lineIdx) in hunk.theirsLines"
                      :key="'t-' + lineIdx"
                      class="flex items-start hover:bg-[#1f6feb]/10 transition-colors border-b border-[var(--border)]/30"
                      @click="toggleTheirsLine(hunkIdx, lineIdx)"
                    >
                      <div class="w-7 flex-shrink-0 flex items-center justify-center border-r border-[var(--border)]/30 cursor-pointer hover:bg-[#1f6feb]/15">
                        <input
                          type="checkbox"
                          :checked="selections[hunkIdx].theirsSelected[lineIdx]"
                          @click.stop
                          @change="toggleTheirsLine(hunkIdx, lineIdx)"
                          class="w-3 h-3 rounded border-[#58a6ff] text-[#1f6feb] cursor-pointer"
                        />
                      </div>
                      <div class="w-6 flex-shrink-0 text-right pr-1 text-[#484f58] select-none text-[10px] border-r border-[var(--border)]/30">
                        {{ hunk.theirsStartLine + lineIdx }}
                      </div>
                      <pre 
                        class="flex-1 px-2 py-1 whitespace-pre-wrap word-break overflow-x-auto"
                        :class="selections[hunkIdx].theirsSelected[lineIdx] ? 'text-[#a5d6ff] bg-[#1f6feb]/20' : 'text-[#8b949e]'"
                      ><code class="hljs bg-transparent" v-html="getHighlightedConflictLine('theirs', hunkIdx, lineIdx, line)"></code></pre>
                    </div>

                    <!-- Context after -->
                    <div v-if="hunk.contextAfter.length > 0" class="border-t border-[var(--border)]/20 bg-[#1f6feb]/2">
                      <div v-for="(line, idx) in hunk.contextAfter" :key="'theirs-after-' + idx" class="flex items-start text-[#666]">
                        <div class="w-7 flex-shrink-0 text-right pr-1 text-[#484f58] select-none text-[10px] border-r border-[var(--border)]/20">
                          {{ hunk.contextAfterStartLine + idx }}
                        </div>
                        <div class="w-7 flex-shrink-0"></div>
                        <pre class="flex-1 px-2 py-0.5 whitespace-pre-wrap text-[#666]"><code class="hljs bg-transparent" v-html="getHighlightedConflictLine('theirs-context-after', hunkIdx, idx, line)"></code></pre>
                      </div>
                    </div>

                    <div v-if="hunk.theirsLines.length === 0" class="px-4 py-4 text-xs text-[var(--muted-foreground)] italic">
                      (no content)
                    </div>
                  </div>
                </div>
              </div>

              <!-- Resolution Preview -->
              <div class="border-t border-[var(--border)]">
                <div class="px-3 py-2 text-[10px] font-semibold text-[#3fb950] border-b border-[var(--border)] bg-[#238636]/5">
                  RESULT
                </div>
                <div class="font-mono text-[11px] max-h-96 overflow-y-auto bg-[var(--background)]/50">
                  <div 
                    v-if="selections[hunkIdx].oursSelected.some(s => s) || selections[hunkIdx].theirsSelected.some(s => s)"
                    class="p-2"
                  >
                    <div v-for="(line, lineIdx) in [...hunk.oursLines.filter((_, i) => selections[hunkIdx].oursSelected[i]), ...hunk.theirsLines.filter((_, i) => selections[hunkIdx].theirsSelected[i])]" :key="'res-' + lineIdx" class="px-2 py-0.5 border-l-2 border-[#3fb950] text-[#aff5b4]">
                      <pre class="whitespace-pre-wrap"><code class="hljs bg-transparent" v-html="getHighlightedConflictLine('result', hunkIdx, lineIdx, line)"></code></pre>
                    </div>
                  </div>
                  <div v-else class="px-3 py-3 text-xs text-[#f59e0b]">
                    ⚠ No lines selected - conflict section will be empty
                  </div>
                </div>
              </div>
            </div>
          </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between px-5 py-2.5 border-t border-[var(--border)] bg-[var(--secondary)] flex-shrink-0">
        <div class="flex items-center gap-2">
          <span v-if="allHunksResolved" class="flex items-center gap-1.5 text-xs text-[#3fb950] font-medium">
            <Check class="w-4 h-4" />
            {{ hasMarkers ? 'All conflicts resolved ✓' : 'Resolution method selected ✓' }}
          </span>
          <span v-else class="flex items-center gap-1.5 text-xs text-[#f59e0b]">
            <AlertTriangle class="w-4 h-4" />
            {{ hasMarkers ? `${selections.filter(s => !s.oursSelected.some(x => x) && !s.theirsSelected.some(x => x)).length} unresolved conflict(s)` : 'Select resolution method' }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

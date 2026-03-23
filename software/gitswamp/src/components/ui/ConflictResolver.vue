<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { X, Check, ChevronLeft, ChevronRight, Save, RotateCcw, AlertTriangle } from "lucide-vue-next";

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
  oursLines: string[];
  theirsLines: string[];
  baseLines?: string[];
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

// Parse conflict markers from file content
function parseConflicts(content: string): ConflictHunk[] {
  const lines = content.split('\n');
  const conflicts: ConflictHunk[] = [];
  
  let i = 0;
  while (i < lines.length) {
    if (lines[i].startsWith('<<<<<<<')) {
      const startLine = i;
      const oursLines: string[] = [];
      const baseLines: string[] = [];
      const theirsLines: string[] = [];
      
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
      
      conflicts.push({
        startLine,
        oursLines,
        theirsLines,
        baseLines: baseLines.length > 0 ? baseLines : undefined,
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
  
  try {
    fileContent.value = await invoke<string>("get_file_content", {
      path: props.repoPath,
      filePath: props.filePath,
      sha: null,
    });
    
    hunks.value = parseConflicts(fileContent.value);
    selections.value = initSelections(hunks.value);
    
    if (hunks.value.length === 0) {
      error.value = "No conflict markers found in this file";
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
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

function toggleOursLine(hunkIdx: number, lineIdx: number) {
  const sel = selections.value[hunkIdx];
  sel.oursSelected[lineIdx] = !sel.oursSelected[lineIdx];
  sel.side = 'none'; // Manual selection
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

const allHunksResolved = computed(() => {
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
      content: previewContent.value,
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

onMounted(() => {
  loadFile();
});

watch(() => props.filePath, () => {
  loadFile();
});
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm" @click.self="emit('close')">
    <div class="w-[95vw] max-w-[1400px] h-[90vh] bg-[var(--card)] rounded-xl border border-[var(--border)] shadow-2xl flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-3 border-b border-[var(--border)] bg-[var(--card)] flex-shrink-0">
        <div class="flex items-center gap-3">
          <AlertTriangle class="w-5 h-5 text-[#f59e0b]" />
          <div>
            <span class="text-sm font-semibold text-[var(--foreground)]">Resolve Conflicts</span>
            <span class="text-xs text-[var(--muted-foreground)] ml-2">{{ filePath }}</span>
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
      <div class="flex-1 overflow-hidden flex">
        <!-- Conflict hunks -->
        <div class="flex-1 overflow-y-auto p-4 space-y-6">
          <div v-if="loading" class="flex items-center justify-center h-full">
            <div class="text-[var(--muted-foreground)]">Loading...</div>
          </div>

          <div v-else-if="error" class="flex flex-col items-center justify-center h-full gap-2">
            <div class="text-[#ef4444]">{{ error }}</div>
            <button @click="loadFile" class="text-xs text-[var(--primary)] hover:underline">Retry</button>
          </div>

          <div v-else v-for="(hunk, hunkIdx) in hunks" :key="hunkIdx" class="border border-[var(--border)] rounded-lg overflow-hidden">
            <!-- Hunk header -->
            <div class="flex items-center justify-between px-4 py-2 bg-[var(--secondary)] border-b border-[var(--border)]">
              <span class="text-xs font-medium text-[var(--foreground)]">Conflict {{ hunkIdx + 1 }} of {{ hunks.length }}</span>
              <div class="flex items-center gap-2">
                <button
                  @click="selectAllOurs(hunkIdx)"
                  :class="[
                    'px-2.5 py-1 text-[10px] font-semibold rounded transition-colors',
                    selections[hunkIdx].side === 'ours'
                      ? 'bg-[#238636] text-white'
                      : 'bg-[var(--card)] text-[var(--muted-foreground)] hover:bg-[#238636]/20 hover:text-[#238636] border border-[var(--border)]'
                  ]"
                >
                  <ChevronLeft class="w-3 h-3 inline mr-0.5" />
                  Keep Ours
                </button>
                <button
                  @click="selectBoth(hunkIdx)"
                  :class="[
                    'px-2.5 py-1 text-[10px] font-semibold rounded transition-colors',
                    selections[hunkIdx].side === 'both'
                      ? 'bg-[var(--primary)] text-white'
                      : 'bg-[var(--card)] text-[var(--muted-foreground)] hover:bg-[var(--primary)]/20 hover:text-[var(--primary)] border border-[var(--border)]'
                  ]"
                >
                  Keep Both
                </button>
                <button
                  @click="selectAllTheirs(hunkIdx)"
                  :class="[
                    'px-2.5 py-1 text-[10px] font-semibold rounded transition-colors',
                    selections[hunkIdx].side === 'theirs'
                      ? 'bg-[#1f6feb] text-white'
                      : 'bg-[var(--card)] text-[var(--muted-foreground)] hover:bg-[#1f6feb]/20 hover:text-[#1f6feb] border border-[var(--border)]'
                  ]"
                >
                  Keep Theirs
                  <ChevronRight class="w-3 h-3 inline ml-0.5" />
                </button>
              </div>
            </div>

            <!-- Split view -->
            <div class="flex divide-x divide-[var(--border)]">
              <!-- Ours (left) -->
              <div class="flex-1 bg-[#1a4d2e]/10">
                <div class="px-3 py-1.5 text-[10px] font-semibold text-[#3fb950] border-b border-[var(--border)] bg-[#238636]/10">
                  OURS (Current Branch)
                </div>
                <div class="font-mono text-[12px]">
                  <div
                    v-for="(line, lineIdx) in hunk.oursLines"
                    :key="'o-' + lineIdx"
                    class="flex items-center hover:bg-[#238636]/15 cursor-pointer transition-colors"
                    @click="toggleOursLine(hunkIdx, lineIdx)"
                  >
                    <div class="w-8 flex-shrink-0 flex items-center justify-center border-r border-[var(--border)]">
                      <input
                        type="checkbox"
                        :checked="selections[hunkIdx].oursSelected[lineIdx]"
                        @click.stop
                        @change="toggleOursLine(hunkIdx, lineIdx)"
                        class="w-3.5 h-3.5 rounded border-[#3fb950] text-[#238636] focus:ring-[#238636] cursor-pointer"
                      />
                    </div>
                    <div class="w-8 flex-shrink-0 text-right pr-1.5 text-[#484f58] select-none text-[10px]">
                      {{ lineIdx + 1 }}
                    </div>
                    <pre 
                      class="flex-1 px-2 py-0.5 whitespace-pre overflow-x-auto"
                      :class="selections[hunkIdx].oursSelected[lineIdx] ? 'text-[#aff5b4] bg-[#238636]/20' : 'text-[#8b949e]'"
                    >{{ line }}</pre>
                  </div>
                  <div v-if="hunk.oursLines.length === 0" class="px-4 py-3 text-xs text-[var(--muted-foreground)] italic">
                    (empty)
                  </div>
                </div>
              </div>

              <!-- Theirs (right) -->
              <div class="flex-1 bg-[#1f6feb]/5">
                <div class="px-3 py-1.5 text-[10px] font-semibold text-[#58a6ff] border-b border-[var(--border)] bg-[#1f6feb]/10">
                  THEIRS (Incoming)
                </div>
                <div class="font-mono text-[12px]">
                  <div
                    v-for="(line, lineIdx) in hunk.theirsLines"
                    :key="'t-' + lineIdx"
                    class="flex items-center hover:bg-[#1f6feb]/15 cursor-pointer transition-colors"
                    @click="toggleTheirsLine(hunkIdx, lineIdx)"
                  >
                    <div class="w-8 flex-shrink-0 flex items-center justify-center border-r border-[var(--border)]">
                      <input
                        type="checkbox"
                        :checked="selections[hunkIdx].theirsSelected[lineIdx]"
                        @click.stop
                        @change="toggleTheirsLine(hunkIdx, lineIdx)"
                        class="w-3.5 h-3.5 rounded border-[#58a6ff] text-[#1f6feb] focus:ring-[#1f6feb] cursor-pointer"
                      />
                    </div>
                    <div class="w-8 flex-shrink-0 text-right pr-1.5 text-[#484f58] select-none text-[10px]">
                      {{ lineIdx + 1 }}
                    </div>
                    <pre 
                      class="flex-1 px-2 py-0.5 whitespace-pre overflow-x-auto"
                      :class="selections[hunkIdx].theirsSelected[lineIdx] ? 'text-[#a5d6ff] bg-[#1f6feb]/20' : 'text-[#8b949e]'"
                    >{{ line }}</pre>
                  </div>
                  <div v-if="hunk.theirsLines.length === 0" class="px-4 py-3 text-xs text-[var(--muted-foreground)] italic">
                    (empty)
                  </div>
                </div>
              </div>
            </div>

            <!-- Resolution preview for this hunk -->
            <div class="border-t border-[var(--border)] bg-[var(--card)]">
              <div class="px-3 py-1.5 text-[10px] font-semibold text-[var(--primary)] border-b border-[var(--border)] bg-[var(--primary)]/5">
                RESOLUTION PREVIEW
              </div>
              <div class="font-mono text-[12px] max-h-32 overflow-y-auto">
                <template v-for="(line, lineIdx) in [...hunk.oursLines.filter((_, i) => selections[hunkIdx].oursSelected[i]), ...hunk.theirsLines.filter((_, i) => selections[hunkIdx].theirsSelected[i])]" :key="'p-' + lineIdx">
                  <div class="flex">
                    <div class="w-8 flex-shrink-0 text-right pr-1.5 text-[#484f58] select-none text-[10px] border-r border-[var(--border)]">
                      {{ lineIdx + 1 }}
                    </div>
                    <pre class="flex-1 px-2 py-0.5 whitespace-pre overflow-x-auto text-[var(--foreground)]">{{ line }}</pre>
                  </div>
                </template>
                <div 
                  v-if="!selections[hunkIdx].oursSelected.some(s => s) && !selections[hunkIdx].theirsSelected.some(s => s)" 
                  class="px-4 py-3 text-xs text-[#f59e0b] italic"
                >
                  ⚠ No lines selected - this conflict section will be empty
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer status -->
      <div class="flex items-center justify-between px-5 py-2.5 border-t border-[var(--border)] bg-[var(--secondary)] flex-shrink-0">
        <div class="flex items-center gap-2">
          <span v-if="allHunksResolved" class="flex items-center gap-1.5 text-xs text-[#3fb950]">
            <Check class="w-4 h-4" />
            All conflicts resolved
          </span>
          <span v-else class="flex items-center gap-1.5 text-xs text-[#f59e0b]">
            <AlertTriangle class="w-4 h-4" />
            {{ selections.filter(s => !s.oursSelected.some(x => x) && !s.theirsSelected.some(x => x)).length }} conflict(s) need resolution
          </span>
        </div>
        <div class="text-xs text-[var(--muted-foreground)]">
          Select lines from each side or use quick actions
        </div>
      </div>
    </div>
  </div>
</template>

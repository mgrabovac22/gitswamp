<script setup lang="ts">
import { ref, nextTick, watch, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Terminal, X, RotateCcw, Eraser } from "lucide-vue-next";

const props = defineProps<{
  output: string[];
  repoPath: string;
  allowAllCommands: boolean;
}>();

const emit = defineEmits<{
  run: [payload: { command: string; allowAll: boolean }];
  "update:allowAllCommands": [value: boolean];
  close: [];
}>();

const inputRef = ref<HTMLInputElement | null>(null);
const outputRef = ref<HTMLElement | null>(null);
const command = ref("");
const history = ref<string[]>([]);
const historyIndex = ref(-1);
const reverseSearchCursor = ref(-1);
const reverseSearchNeedle = ref("");
const MAX_HISTORY_ITEMS = 180;
const HISTORY_STORAGE_PREFIX = "gitswamp-terminal-history";
const OPEN_TOOLS_CACHE_KEY = "gitswamp-open-tools-cache-v2";
const OPEN_TOOL_SELECTED_KEY = "gitswamp-open-tool-selected-v1";

type ExternalToolId = "explorer" | "vscode" | "visualstudio" | "androidstudio" | "intellij";

interface ExternalToolOption {
  id: ExternalToolId;
  label: string;
}

const quickCommands = [
  { label: "Status", command: "git status -sb", title: "Show concise working tree status" },
  { label: "Branches", command: "git branch -vv", title: "Show local branches with tracking status" },
  { label: "Graph", command: "git log --oneline --graph --decorate -20", title: "Show recent commit graph" },
  { label: "Stash", command: "git stash list", title: "List stashes" },
  { label: "Fetch", command: "git fetch --all --prune", title: "Fetch all remotes and prune deleted refs" },
  { label: "Untrack", command: "git rm -r --cached .", title: "Stop tracking all currently tracked files in index" },
  { label: "Help", command: "help", title: "Show built-in terminal shortcuts" },
] as const;

const canRepeat = computed(() => history.value.length > 0);
const openToolOptions = ref<ExternalToolOption[]>([]);
const selectedOpenTool = ref<ExternalToolId | "">("");

const toolLabelMap: Record<ExternalToolId, string> = {
  explorer: "Folder Explorer",
  vscode: "VS Code",
  visualstudio: "Visual Studio",
  androidstudio: "Android Studio",
  intellij: "IntelliJ",
};

function normalizeToolId(input: string): ExternalToolId | null {
  const value = input.trim().toLowerCase();
  if (value === "explorer" || value === "folder" || value === "finder" || value === "file-explorer") return "explorer";
  if (value === "vscode" || value === "code") return "vscode";
  if (value === "visualstudio" || value === "vs" || value === "visual-studio") return "visualstudio";
  if (value === "androidstudio" || value === "android-studio" || value === "studio") return "androidstudio";
  if (value === "intellij" || value === "idea") return "intellij";
  return null;
}

function mapToolIds(ids: string[]): ExternalToolOption[] {
  const normalized = ids
    .map(normalizeToolId)
    .filter((value): value is ExternalToolId => value !== null);

  return Array.from(new Set(normalized)).map((id) => ({
    id,
    label: toolLabelMap[id],
  }));
}

function applyToolOptions(options: ExternalToolOption[]) {
  openToolOptions.value = options;

  let preferred = selectedOpenTool.value;
  if (!preferred) {
    try {
      const saved = localStorage.getItem(OPEN_TOOL_SELECTED_KEY);
      const normalized = saved ? normalizeToolId(saved) : null;
      if (normalized) {
        preferred = normalized;
      }
    } catch {
    }
  }

  if (preferred && options.some((option) => option.id === preferred)) {
    selectedOpenTool.value = preferred;
    return;
  }

  if (!options.some((option) => option.id === selectedOpenTool.value)) {
    selectedOpenTool.value = options[0]?.id ?? "";
  }
}

function loadOpenToolsFromCache(): ExternalToolOption[] {
  try {
    const raw = localStorage.getItem(OPEN_TOOLS_CACHE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return mapToolIds(parsed as string[]);
  } catch {
    return [];
  }
}

function cacheOpenTools(ids: string[]) {
  try {
    localStorage.setItem(OPEN_TOOLS_CACHE_KEY, JSON.stringify(ids));
  } catch {
  }
}

async function scanOpenToolsOnce() {
  const cached = loadOpenToolsFromCache();
  if (cached.length > 0) {
    applyToolOptions(cached);
    return;
  }

  try {
    const ids = await invoke<string[]>("get_available_external_tools");
    const mapped = mapToolIds(ids);
    if (mapped.length > 0) {
      cacheOpenTools(ids);
      applyToolOptions(mapped);
      return;
    }
  } catch {
  }

  try {
    const fallbackIds = await invoke<string[]>("get_available_external_editors");
    const mappedFallback = mapToolIds(fallbackIds);
    if (mappedFallback.length > 0) {
      cacheOpenTools(fallbackIds);
      applyToolOptions(mappedFallback);
      return;
    }
  } catch {
  }

  applyToolOptions([]);
}

function openWithSelectedTool() {
  if (!selectedOpenTool.value) return;
  runCommand(`open ${selectedOpenTool.value}`);
}

watch(selectedOpenTool, (value) => {
  if (!value) return;
  try {
    localStorage.setItem(OPEN_TOOL_SELECTED_KEY, value);
  } catch {
  }
});

function historyStorageKey() {
  return `${HISTORY_STORAGE_PREFIX}:${props.repoPath || "global"}`;
}

function persistHistory() {
  try {
    localStorage.setItem(historyStorageKey(), JSON.stringify(history.value.slice(-MAX_HISTORY_ITEMS)));
  } catch {
  }
}

function loadHistory() {
  historyIndex.value = -1;
  try {
    const raw = localStorage.getItem(historyStorageKey());
    if (!raw) {
      history.value = [];
      return;
    }
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      history.value = [];
      return;
    }
    history.value = parsed
      .filter((item): item is string => typeof item === "string" && item.trim().length > 0)
      .slice(-MAX_HISTORY_ITEMS);
  } catch {
    history.value = [];
  }
}

watch(() => props.repoPath, () => {
  loadHistory();
}, { immediate: true });

watch(() => props.output.length, () => {
  nextTick(() => {
    if (outputRef.value) {
      outputRef.value.scrollTop = outputRef.value.scrollHeight;
    }
  });
});

function pushHistory(cmd: string) {
  const last = history.value[history.value.length - 1];
  if (last !== cmd) {
    history.value.push(cmd);
  }
  if (history.value.length > MAX_HISTORY_ITEMS) {
    history.value.splice(0, history.value.length - MAX_HISTORY_ITEMS);
  }
  persistHistory();
}

function runCommand(raw: string) {
  const cmd = raw.trim();
  if (!cmd) return;
  pushHistory(cmd);
  historyIndex.value = -1;
  reverseSearchCursor.value = -1;
  reverseSearchNeedle.value = "";
  emit("run", { command: cmd, allowAll: props.allowAllCommands });
  command.value = "";
}

function submit() {
  runCommand(command.value);
}

function runQuickCommand(cmd: string) {
  command.value = cmd;
  runCommand(cmd);
}

function clearTerminal() {
  emit("run", { command: "clear", allowAll: props.allowAllCommands });
  command.value = "";
}

function repeatLast() {
  if (!canRepeat.value) return;
  runCommand("!!");
}

function resetReverseSearchState() {
  reverseSearchCursor.value = -1;
  reverseSearchNeedle.value = "";
}

function reverseSearchHistory() {
  if (history.value.length === 0) return;

  const needle = command.value.trim().toLowerCase();
  if (reverseSearchNeedle.value !== needle) {
    reverseSearchNeedle.value = needle;
    reverseSearchCursor.value = history.value.length;
  }

  for (let i = reverseSearchCursor.value - 1; i >= 0; i--) {
    const entry = history.value[i];
    if (!needle || entry.toLowerCase().includes(needle)) {
      reverseSearchCursor.value = i;
      command.value = entry;
      return;
    }
  }

  reverseSearchCursor.value = history.value.length;
}

function onKeyDown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key.toLowerCase() === "r") {
    e.preventDefault();
    reverseSearchHistory();
    return;
  }

  if (e.ctrlKey && e.key.toLowerCase() === "l") {
    e.preventDefault();
    clearTerminal();
    return;
  }

  if (e.key === "ArrowUp") {
    e.preventDefault();
    resetReverseSearchState();
    if (history.value.length === 0) return;
    if (historyIndex.value < 0) historyIndex.value = history.value.length;
    historyIndex.value = Math.max(0, historyIndex.value - 1);
    command.value = history.value[historyIndex.value];
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    resetReverseSearchState();
    if (historyIndex.value < 0) return;
    historyIndex.value++;
    if (historyIndex.value >= history.value.length) {
      historyIndex.value = -1;
      command.value = "";
    } else {
      command.value = history.value[historyIndex.value];
    }
  }
}

function focusInput() {
  inputRef.value?.focus();
}

function onContainerClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  if (target?.closest("button, input, select, option, label, textarea")) {
    return;
  }
  focusInput();
}

onMounted(() => {
  nextTick(focusInput);
  void scanOpenToolsOnce();
});
</script>

<template>
  <div class="bg-[var(--terminal-bg)] border-t border-[var(--border)] flex flex-col" @click="onContainerClick">
    <div class="flex items-center justify-between px-3 py-1.5 bg-[var(--background)] border-b border-[var(--border)] flex-shrink-0">
      <div class="flex items-center gap-2 text-xs text-[var(--muted-foreground)]">
        <Terminal class="w-3.5 h-3.5 text-[var(--primary)]" />
        <span class="font-medium text-[var(--foreground)]">Terminal</span>
        <span class="text-[10px] text-[var(--muted-foreground)]">{{ props.allowAllCommands ? 'all commands' : 'git commands only' }}</span>
      </div>
      <div class="flex items-center gap-1">
        <button
          class="px-2 py-1 rounded text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors disabled:opacity-40"
          :disabled="!canRepeat"
          title="Repeat last command (!!)"
          @click.stop="repeatLast"
        >
          <RotateCcw class="w-3 h-3 inline mr-1" />
          Repeat
        </button>
        <button
          class="px-2 py-1 rounded text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors"
          title="Clear terminal output (Ctrl+L)"
          @click.stop="clearTerminal"
        >
          <Eraser class="w-3 h-3 inline mr-1" />
          Clear
        </button>
        <label class="flex items-center gap-1.5 text-[10px] text-[var(--foreground)] mx-2 cursor-pointer">
          <input
            type="checkbox"
            :checked="props.allowAllCommands"
            @change="emit('update:allowAllCommands', ($event.target as HTMLInputElement).checked)"
            class="w-3.5 h-3.5 accent-[var(--primary)]"
          />
          Allow all commands
        </label>
        <button
          @click="emit('close')"
          class="p-1 rounded hover:bg-[var(--secondary)] transition-colors"
        >
          <X class="w-3.5 h-3.5 text-[var(--muted-foreground)]" />
        </button>
      </div>
    </div>

    <div class="px-3 py-1.5 border-b border-[var(--border)] bg-[var(--background)]/70 flex flex-wrap items-center gap-1.5">
      <button
        v-for="quick in quickCommands"
        :key="quick.label"
        class="px-2 py-0.5 rounded-full text-[10px] border border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:border-[var(--primary)]/45 hover:bg-[var(--primary)]/10 transition-colors"
        :title="quick.title"
        @click.stop="runQuickCommand(quick.command)"
      >
        {{ quick.label }}
      </button>
      <div class="ml-1 flex items-center gap-1.5">
        <select
          v-model="selectedOpenTool"
          class="h-6 min-w-[130px] px-1.5 text-[10px] rounded border border-[var(--border)] bg-[var(--card)] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/35"
          :disabled="openToolOptions.length === 0"
          title="Detected tools are scanned once and cached"
        >
          <option value="" disabled>{{ openToolOptions.length === 0 ? 'No tools detected' : 'Select tool' }}</option>
          <option v-for="tool in openToolOptions" :key="tool.id" :value="tool.id">{{ tool.label }}</option>
        </select>
        <button
          class="px-2 py-0.5 rounded-full text-[10px] border border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:border-[var(--primary)]/45 hover:bg-[var(--primary)]/10 transition-colors disabled:opacity-50"
          :disabled="!selectedOpenTool"
          title="Open current repository in selected tool"
          @click.stop="openWithSelectedTool"
        >
          Open
        </button>
      </div>
      <span class="text-[10px] text-[var(--muted-foreground)] ml-1">Shortcuts: ↑/↓ history, Ctrl+R reverse search, Ctrl+L clear, !! repeat</span>
    </div>

    <div ref="outputRef" class="flex-1 overflow-y-auto px-3 py-2 font-mono text-xs min-h-0">
      <div v-if="output.length === 0" class="text-[var(--muted-foreground)] py-2">
        {{ props.allowAllCommands ? 'Type any shell command (e.g., dir, npm run build, git status). Built-ins: help, clear, !!, tools, open <tool>' : 'Type a git command (e.g., git status, git log --oneline -5). Aliases: st, br, co, sw, lg, last. Built-ins: tools, open <tool>' }}
      </div>
      <div
        v-for="(line, i) in output"
        :key="i"
        class="whitespace-pre-wrap mb-2"
      >
        <template v-for="(part, _j) in line.split('\n')" :key="_j">
          <div :class="part.startsWith('$') ? 'text-[var(--primary)] font-medium' : part.startsWith('Error:') ? 'text-[#ef4444]' : 'text-[var(--foreground)]'">
            {{ part }}
          </div>
        </template>
      </div>
    </div>

    <div class="flex items-center gap-2 px-3 py-2 border-t border-[var(--border)] flex-shrink-0 bg-[var(--background)]">
      <span class="text-[var(--primary)] text-xs font-mono font-bold">$</span>
      <input
        ref="inputRef"
        v-model="command"
        @keyup.enter="submit"
        @keydown="onKeyDown"
        :placeholder="props.allowAllCommands ? 'command... (help, clear, !!, tools, open vscode)' : 'git ... (st, co, br, lg, last, clear, !!, tools, open vscode)'"
        class="flex-1 bg-transparent text-xs text-[var(--foreground)] font-mono placeholder:text-[var(--muted-foreground)] focus:outline-none"
      />
    </div>
  </div>
</template>


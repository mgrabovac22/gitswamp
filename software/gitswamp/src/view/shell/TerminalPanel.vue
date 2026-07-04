<script setup lang="ts">
import { ref, nextTick, watch, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, Terminal, RotateCcw, Eraser } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";

const props = defineProps<{
  output: string[];
  repoPath: string;
  allowAllCommands: boolean;
  stagedFileCount: number;
  unstagedFileCount: number;
  untrackedFileCount: number;
  conflictFileCount: number;
}>();

const emit = defineEmits<{
  run: [payload: { command: string; allowAll: boolean; safetyStashFirst?: boolean }];
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
const pendingSafety = ref<TerminalSafetyPrompt | null>(null);
const MAX_HISTORY_ITEMS = 180;
const HISTORY_STORAGE_PREFIX = "gitswamp-terminal-history";
const OPEN_TOOLS_CACHE_KEY = "gitswamp-open-tools-cache-v2";
const OPEN_TOOL_SELECTED_KEY = "gitswamp-open-tool-selected-v1";

type ExternalToolId = "explorer" | "vscode" | "visualstudio" | "androidstudio" | "intellij";

interface ExternalToolOption {
  id: ExternalToolId;
  label: string;
}

interface TerminalSafetyPrompt {
  command: string;
  headline: string;
  detail: string;
  impacts: string[];
  canCreateSafetyStash: boolean;
  safetyStashDisabledReason: string;
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
const hasLocalChanges = computed(() =>
  props.stagedFileCount + props.unstagedFileCount + props.untrackedFileCount > 0,
);
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

function parseCommandArgsForSafety(input: string): string[] | null {
  const args: string[] = [];
  let current = "";
  let quote: '"' | "'" | null = null;
  let escaping = false;

  for (const char of input) {
    if (escaping) {
      current += char;
      escaping = false;
      continue;
    }

    if (char === "\\") {
      escaping = true;
      continue;
    }

    if (quote) {
      if (char === quote) {
        quote = null;
      } else {
        current += char;
      }
      continue;
    }

    if (char === '"' || char === "'") {
      quote = char;
      continue;
    }

    if (char.trim().length === 0) {
      if (current.length > 0) {
        args.push(current);
        current = "";
      }
      continue;
    }

    current += char;
  }

  if (escaping || quote) return null;
  if (current.length > 0) args.push(current);
  return args;
}

function stripSudoPrefix(value: string): string {
  return value.replace(/^sudo(?:\s+|$)/i, "").trim();
}

function normalizeGitArgsForSafety(rawCommand: string, allowAll: boolean): string[] | null {
  const parsed = parseCommandArgsForSafety(stripSudoPrefix(rawCommand));
  if (!parsed || parsed.length === 0) return null;

  const first = parsed[0].toLowerCase();
  if (first === "git") {
    return parsed.slice(1);
  }

  return allowAll ? null : parsed;
}

function hasFlag(args: string[], flag: string): boolean {
  const target = flag.toLowerCase();
  return args.some((arg) => arg.toLowerCase() === target);
}

function hasCombinedShortFlag(args: string[], flag: string): boolean {
  const normalized = flag.toLowerCase().replace(/^-+/, "");
  return args.some((arg) => {
    const lower = arg.toLowerCase();
    return lower.startsWith("-") && !lower.startsWith("--") && lower.slice(1).includes(normalized);
  });
}

function hasForceFlag(args: string[]): boolean {
  return hasFlag(args, "--force") || hasCombinedShortFlag(args, "f");
}

function formatCount(count: number, label: string): string {
  return `${count} ${label}${count === 1 ? "" : "s"}`;
}

function localChangeImpacts(includeStaged: boolean, includeUnstaged: boolean, includeUntracked: boolean): string[] {
  const impacts: string[] = [];
  if (includeUnstaged) impacts.push(formatCount(props.unstagedFileCount, "unstaged file"));
  if (includeStaged) impacts.push(formatCount(props.stagedFileCount, "staged file"));
  if (includeUntracked) impacts.push(formatCount(props.untrackedFileCount, "untracked file"));
  if (props.conflictFileCount > 0) impacts.push(formatCount(props.conflictFileCount, "conflicted file"));
  return impacts.filter((impact) => !impact.startsWith("0 "));
}

function safetyStashDisabledReason(): string {
  if (props.conflictFileCount > 0) {
    return "Resolve conflicts before creating a safety stash.";
  }
  if (!hasLocalChanges.value) {
    return "No local changes to stash.";
  }
  return "";
}

function buildSafetyPrompt(commandToRun: string, args: string[]): TerminalSafetyPrompt | null {
  const subcommand = args[0]?.toLowerCase();
  if (!subcommand) return null;

  const disabledReason = safetyStashDisabledReason();
  const canCreateSafetyStash = hasLocalChanges.value && props.conflictFileCount === 0;

  if (subcommand === "reset" && hasFlag(args, "--hard")) {
    const impacts = localChangeImpacts(true, true, false);
    return {
      command: commandToRun,
      headline: "Hard reset",
      detail: "This can discard staged and unstaged work before moving HEAD/index.",
      impacts: impacts.length > 0 ? impacts : ["No staged or unstaged file changes detected"],
      canCreateSafetyStash,
      safetyStashDisabledReason: disabledReason,
    };
  }

  if (subcommand === "clean" && hasForceFlag(args)) {
    const includeIgnored = hasCombinedShortFlag(args, "x") || hasFlag(args, "-x");
    const impacts = localChangeImpacts(false, false, true);
    if (includeIgnored) impacts.push("ignored files matched by git clean -x");
    return {
      command: commandToRun,
      headline: "Git clean",
      detail: "This can permanently delete untracked files from the working tree.",
      impacts: impacts.length > 0 ? impacts : ["No untracked files currently shown in GitSwamp"],
      canCreateSafetyStash,
      safetyStashDisabledReason: disabledReason,
    };
  }

  if (subcommand === "restore") {
    const stagedOnly = hasFlag(args, "--staged") && !hasFlag(args, "--worktree");
    if (!stagedOnly) {
      const touchesStaged = hasFlag(args, "--staged");
      const impacts = localChangeImpacts(touchesStaged, true, false);
      return {
        command: commandToRun,
        headline: "Restore working tree",
        detail: "This can discard local file edits from the working tree.",
        impacts: impacts.length > 0 ? impacts : ["No unstaged file changes detected"],
        canCreateSafetyStash,
        safetyStashDisabledReason: disabledReason,
      };
    }
  }

  if (subcommand === "checkout" && (hasFlag(args, "--") || hasForceFlag(args))) {
    const impacts = localChangeImpacts(false, true, false);
    return {
      command: commandToRun,
      headline: "Checkout overwrite",
      detail: "This can overwrite local working-tree edits.",
      impacts: impacts.length > 0 ? impacts : ["No unstaged file changes detected"],
      canCreateSafetyStash,
      safetyStashDisabledReason: disabledReason,
    };
  }

  if (subcommand === "rm" && !hasFlag(args, "--cached")) {
    return {
      command: commandToRun,
      headline: "Remove tracked files",
      detail: "This can delete tracked files from the working tree.",
      impacts: ["tracked files matched by this command"],
      canCreateSafetyStash,
      safetyStashDisabledReason: disabledReason,
    };
  }

  if (subcommand === "stash" && ["pop", "drop", "clear"].includes(args[1]?.toLowerCase() || "")) {
    return {
      command: commandToRun,
      headline: `Stash ${args[1]?.toLowerCase()}`,
      detail: "This can remove stash entries or apply changes back into the worktree.",
      impacts: ["stash entry state can change"],
      canCreateSafetyStash: false,
      safetyStashDisabledReason: "Safety stash is not useful for stash-entry operations.",
    };
  }

  if (subcommand === "branch" && args.some((arg) => arg === "-D" || arg === "-d" || arg === "--delete")) {
    return {
      command: commandToRun,
      headline: "Delete branch",
      detail: "This can remove a local branch reference.",
      impacts: ["local branch reference matched by this command"],
      canCreateSafetyStash: false,
      safetyStashDisabledReason: "Safety stash protects files, not deleted branch refs.",
    };
  }

  if (subcommand === "push" && (hasFlag(args, "--force") || hasFlag(args, "--force-with-lease") || hasCombinedShortFlag(args, "f"))) {
    return {
      command: commandToRun,
      headline: "Force push",
      detail: "This can rewrite remote branch history.",
      impacts: ["remote branch history can be overwritten"],
      canCreateSafetyStash: false,
      safetyStashDisabledReason: "Safety stash protects local files, not remote history.",
    };
  }

  return null;
}

function safetyPromptForCommand(raw: string): TerminalSafetyPrompt | null {
  const commandToRun = raw.trim() === "!!" ? history.value[history.value.length - 1] || raw.trim() : raw.trim();
  const args = normalizeGitArgsForSafety(commandToRun, props.allowAllCommands);
  if (!args || args.length === 0) return null;
  return buildSafetyPrompt(commandToRun, args);
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

function executeCommand(raw: string, options?: { safetyStashFirst?: boolean }) {
  const cmd = raw.trim();
  if (!cmd) return;
  pushHistory(cmd);
  historyIndex.value = -1;
  reverseSearchCursor.value = -1;
  reverseSearchNeedle.value = "";
  pendingSafety.value = null;
  emit("run", {
    command: cmd,
    allowAll: props.allowAllCommands,
    safetyStashFirst: options?.safetyStashFirst,
  });
  command.value = "";
}

function runCommand(raw: string) {
  const cmd = raw.trim();
  if (!cmd) return;

  const safetyPrompt = safetyPromptForCommand(cmd);
  if (safetyPrompt) {
    pendingSafety.value = safetyPrompt;
    command.value = "";
    nextTick(focusInput);
    return;
  }

  executeCommand(cmd);
}

function continueDangerousCommand() {
  if (!pendingSafety.value) return;
  executeCommand(pendingSafety.value.command);
}

function createSafetyStashAndContinue() {
  if (!pendingSafety.value || !pendingSafety.value.canCreateSafetyStash) return;
  executeCommand(pendingSafety.value.command, { safetyStashFirst: true });
}

function cancelDangerousCommand() {
  pendingSafety.value = null;
  nextTick(focusInput);
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
        <CloseIconButton size="sm" title="Close terminal panel" @click="emit('close')" />
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

    <div
      v-if="pendingSafety"
      class="mx-3 mb-2 rounded-lg border border-[#f59e0b]/35 bg-[#f59e0b]/10 px-3 py-2 text-xs text-[var(--foreground)]"
    >
      <div class="flex items-start gap-2">
        <AlertTriangle class="w-4 h-4 text-[#f59e0b] mt-0.5 flex-shrink-0" />
        <div class="min-w-0 flex-1">
          <div class="flex items-center justify-between gap-2">
            <div class="font-semibold">{{ pendingSafety.headline }}</div>
            <code class="text-[10px] text-[var(--muted-foreground)] truncate max-w-[45%]">{{ pendingSafety.command }}</code>
          </div>
          <div class="mt-1 text-[11px] text-[var(--muted-foreground)]">{{ pendingSafety.detail }}</div>
          <div class="mt-2 text-[11px]">
            <div class="font-medium mb-1">This can affect:</div>
            <ul class="space-y-0.5 text-[var(--muted-foreground)]">
              <li v-for="impact in pendingSafety.impacts" :key="impact">- {{ impact }}</li>
            </ul>
          </div>
          <div
            v-if="!pendingSafety.canCreateSafetyStash && pendingSafety.safetyStashDisabledReason"
            class="mt-2 text-[10px] text-[var(--muted-foreground)]"
          >
            {{ pendingSafety.safetyStashDisabledReason }}
          </div>
        </div>
      </div>
      <div class="mt-3 flex items-center justify-end gap-2">
        <button
          type="button"
          class="px-2.5 py-1 rounded text-[11px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors"
          @click.stop="cancelDangerousCommand"
        >
          Cancel
        </button>
        <button
          type="button"
          class="px-2.5 py-1 rounded text-[11px] border border-[var(--border)] text-[var(--foreground)] hover:border-[var(--primary)]/45 hover:bg-[var(--primary)]/10 transition-colors disabled:opacity-45 disabled:cursor-not-allowed"
          :disabled="!pendingSafety.canCreateSafetyStash"
          @click.stop="createSafetyStashAndContinue"
        >
          Create safety stash first
        </button>
        <button
          type="button"
          class="px-2.5 py-1 rounded text-[11px] bg-[#f59e0b] text-black font-semibold hover:opacity-90 transition-opacity"
          @click.stop="continueDangerousCommand"
        >
          Continue
        </button>
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


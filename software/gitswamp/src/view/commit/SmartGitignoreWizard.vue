<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Check, FileWarning, Sparkles } from "lucide-vue-next";
import { useToast } from "@/shared/notifications/useToast";
import type { FileStatusInfo } from "@/types";

interface IgnoreDefinition {
  id: string;
  label: string;
  description: string;
  patterns: string[];
  match: (path: string, segments: string[], fileName: string) => boolean;
}

interface IgnoreSuggestion {
  id: string;
  label: string;
  description: string;
  patterns: string[];
  fileCount: number;
  samples: string[];
}

interface SuggestionState {
  suggestions: IgnoreSuggestion[];
  matchedFileCount: number;
}

const props = defineProps<{
  repoPath: string;
  unstagedFiles: FileStatusInfo[];
}>();

const emit = defineEmits<{
  applied: [];
  openGitignore: [];
}>();

const toast = useToast();
const selectedSuggestionIds = ref<Set<string>>(new Set());
const knownSuggestionIds = ref<Set<string>>(new Set());
const dismissedSuggestionIds = ref<Set<string>>(new Set());
const gitignoreContent = ref("");
const loadingGitignore = ref(false);
const applying = ref(false);
let gitignoreLoadSequence = 0;

const SAFE_ENV_FILES = new Set([".env.example", ".env.sample", ".env.template", ".env.defaults"]);

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/^\/+/, "");
}

function pathSegments(path: string): string[] {
  return normalizePath(path).toLowerCase().split("/").filter(Boolean);
}

function basename(path: string): string {
  const normalized = normalizePath(path);
  return normalized.split("/").filter(Boolean).pop()?.toLowerCase() ?? normalized.toLowerCase();
}

function hasSegment(segments: string[], segment: string): boolean {
  return segments.includes(segment.toLowerCase());
}

function hasSegmentSequence(segments: string[], first: string, second: string): boolean {
  const firstSegment = first.toLowerCase();
  const secondSegment = second.toLowerCase();
  return segments.some((segment, index) => segment === firstSegment && segments[index + 1] === secondSegment);
}

function isSafeEnvFile(fileName: string): boolean {
  return SAFE_ENV_FILES.has(fileName);
}

const IGNORE_DEFINITIONS: IgnoreDefinition[] = [
  {
    id: "dependencies-node",
    label: "Dependency folders",
    description: "Installed package directories",
    patterns: ["node_modules/"],
    match: (_path, segments) => hasSegment(segments, "node_modules"),
  },
  {
    id: "dependencies-pnpm",
    label: "Package manager cache",
    description: "Local pnpm store data",
    patterns: [".pnpm-store/"],
    match: (_path, segments) => hasSegment(segments, ".pnpm-store"),
  },
  {
    id: "dependencies-yarn-cache",
    label: "Package manager cache",
    description: "Yarn cache artifacts",
    patterns: [".yarn/cache/"],
    match: (_path, segments) => hasSegmentSequence(segments, ".yarn", "cache"),
  },
  {
    id: "secrets-env",
    label: "Environment secrets",
    description: "Local .env files",
    patterns: [".env", ".env.*", "!.env.example", "!.env.sample", "!.env.template"],
    match: (_path, _segments, fileName) =>
      (fileName === ".env" || fileName.startsWith(".env.")) && !isSafeEnvFile(fileName),
  },
  {
    id: "logs-files",
    label: "Log files",
    description: "Generated runtime logs",
    patterns: ["*.log"],
    match: (_path, _segments, fileName) => fileName.endsWith(".log"),
  },
  {
    id: "logs-folders",
    label: "Log folders",
    description: "Generated log directories",
    patterns: ["logs/"],
    match: (_path, segments) => hasSegment(segments, "logs"),
  },
  {
    id: "build-dist",
    label: "Build output",
    description: "Distribution bundles",
    patterns: ["dist/"],
    match: (_path, segments) => hasSegment(segments, "dist"),
  },
  {
    id: "build-build",
    label: "Build output",
    description: "Build directories",
    patterns: ["build/"],
    match: (_path, segments) => hasSegment(segments, "build"),
  },
  {
    id: "build-out",
    label: "Build output",
    description: "Output directories",
    patterns: ["out/"],
    match: (_path, segments) => hasSegment(segments, "out"),
  },
  {
    id: "build-target",
    label: "Build output",
    description: "Compiled target directories",
    patterns: ["target/"],
    match: (_path, segments) => hasSegment(segments, "target"),
  },
  {
    id: "coverage",
    label: "Coverage output",
    description: "Generated coverage reports",
    patterns: ["coverage/"],
    match: (_path, segments) => hasSegment(segments, "coverage"),
  },
  {
    id: "framework-next",
    label: "Framework cache",
    description: "Next.js build cache",
    patterns: [".next/"],
    match: (_path, segments) => hasSegment(segments, ".next"),
  },
  {
    id: "framework-nuxt",
    label: "Framework cache",
    description: "Nuxt build cache",
    patterns: [".nuxt/"],
    match: (_path, segments) => hasSegment(segments, ".nuxt"),
  },
  {
    id: "framework-svelte",
    label: "Framework cache",
    description: "SvelteKit build cache",
    patterns: [".svelte-kit/"],
    match: (_path, segments) => hasSegment(segments, ".svelte-kit"),
  },
  {
    id: "tool-cache-cache",
    label: "Tool cache",
    description: "Local cache folders",
    patterns: [".cache/"],
    match: (_path, segments) => hasSegment(segments, ".cache"),
  },
  {
    id: "tool-cache-vite",
    label: "Tool cache",
    description: "Vite cache folders",
    patterns: [".vite/"],
    match: (_path, segments) => hasSegment(segments, ".vite"),
  },
  {
    id: "tool-cache-turbo",
    label: "Tool cache",
    description: "Turbo cache folders",
    patterns: [".turbo/"],
    match: (_path, segments) => hasSegment(segments, ".turbo"),
  },
  {
    id: "python-cache",
    label: "Python cache",
    description: "Compiled Python artifacts",
    patterns: ["__pycache__/", "*.py[cod]"],
    match: (_path, segments, fileName) => hasSegment(segments, "__pycache__") || /\.py[cod]$/.test(fileName),
  },
  {
    id: "python-venv",
    label: "Virtual environments",
    description: "Local Python environments",
    patterns: [".venv/", "venv/"],
    match: (_path, segments) => hasSegment(segments, ".venv") || hasSegment(segments, "venv"),
  },
  {
    id: "gradle-cache",
    label: "Build cache",
    description: "Gradle local cache",
    patterns: [".gradle/"],
    match: (_path, segments) => hasSegment(segments, ".gradle"),
  },
  {
    id: "os-ds-store",
    label: "OS files",
    description: "macOS desktop metadata",
    patterns: [".DS_Store"],
    match: (_path, _segments, fileName) => fileName === ".ds_store",
  },
  {
    id: "os-thumbs",
    label: "OS files",
    description: "Windows thumbnail metadata",
    patterns: ["Thumbs.db"],
    match: (_path, _segments, fileName) => fileName === "thumbs.db",
  },
  {
    id: "editor-idea",
    label: "Editor metadata",
    description: "Local JetBrains project state",
    patterns: [".idea/"],
    match: (_path, segments) => hasSegment(segments, ".idea"),
  },
  {
    id: "temp-files",
    label: "Temporary files",
    description: "Scratch and swap files",
    patterns: ["*.tmp", "*.temp", "*.swp", "*~"],
    match: (_path, _segments, fileName) =>
      fileName.endsWith(".tmp") || fileName.endsWith(".temp") || fileName.endsWith(".swp") || fileName.endsWith("~"),
  },
];

function isUntrackedFile(file: FileStatusInfo): boolean {
  const status = file.status.trim().toLowerCase();
  return (
    !file.staged &&
    !file.conflicted &&
    (status === "new" || status === "added" || status === "untracked" || status === "??" || status.includes("untracked"))
  );
}

function compactPath(path: string): string {
  const normalized = normalizePath(path);
  return normalized.length > 54 ? `...${normalized.slice(-51)}` : normalized;
}

function addSuggestion(
  groups: Map<string, IgnoreSuggestion>,
  definition: IgnoreDefinition,
  path: string,
): void {
  const current = groups.get(definition.id);
  if (current) {
    current.fileCount += 1;
    if (current.samples.length < 3) {
      current.samples.push(compactPath(path));
    }
    return;
  }

  groups.set(definition.id, {
    id: definition.id,
    label: definition.label,
    description: definition.description,
    patterns: definition.patterns,
    fileCount: 1,
    samples: [compactPath(path)],
  });
}

const suggestionState = computed<SuggestionState>(() => {
  const groups = new Map<string, IgnoreSuggestion>();
  let matchedFileCount = 0;

  for (const file of props.unstagedFiles) {
    if (!isUntrackedFile(file)) continue;

    const normalizedPath = normalizePath(file.path);
    const segments = pathSegments(normalizedPath);
    const fileName = basename(normalizedPath);
    let matched = false;

    for (const definition of IGNORE_DEFINITIONS) {
      if (!definition.match(normalizedPath, segments, fileName)) continue;
      addSuggestion(groups, definition, normalizedPath);
      matched = true;
    }

    if (matched) {
      matchedFileCount += 1;
    }
  }

  return {
    suggestions: Array.from(groups.values()).sort((a, b) => b.fileCount - a.fileCount || a.label.localeCompare(b.label)),
    matchedFileCount,
  };
});

function normalizePatternForCompare(pattern: string): string {
  return pattern.trim().replace(/^\/+/, "").replace(/\/+$/, "").toLowerCase();
}

function parseIgnorePatterns(content: string): Set<string> {
  return new Set(
    content
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0 && !line.startsWith("#"))
      .map(normalizePatternForCompare),
  );
}

function isPatternCovered(pattern: string, existingPatterns: Set<string>): boolean {
  const normalized = normalizePatternForCompare(pattern);
  if (existingPatterns.has(normalized)) return true;
  if ((normalized === ".env" || normalized === ".env.*") && existingPatterns.has(".env*")) return true;
  return false;
}

const existingPatterns = computed(() => parseIgnorePatterns(gitignoreContent.value));

const visibleSuggestions = computed(() =>
  suggestionState.value.suggestions.filter((suggestion) =>
    !dismissedSuggestionIds.value.has(suggestion.id) &&
    suggestion.patterns.some((pattern) => !isPatternCovered(pattern, existingPatterns.value)),
  ),
);

const selectedSuggestions = computed(() =>
  visibleSuggestions.value.filter((suggestion) => selectedSuggestionIds.value.has(suggestion.id)),
);

function uniquePatterns(patterns: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];

  for (const pattern of patterns) {
    const key = normalizePatternForCompare(pattern);
    if (seen.has(key) || isPatternCovered(pattern, existingPatterns.value)) continue;
    seen.add(key);
    result.push(pattern);
  }

  return result;
}

const previewPatterns = computed(() => uniquePatterns(selectedSuggestions.value.flatMap((suggestion) => suggestion.patterns)));
const visibleMatchCount = computed(() => visibleSuggestions.value.reduce((total, suggestion) => total + suggestion.fileCount, 0));
const visibleGroupCount = computed(() => visibleSuggestions.value.length);

const summaryText = computed(() => {
  const fileLabel = visibleMatchCount.value === 1 ? "match" : "matches";
  const groupLabel = visibleGroupCount.value === 1 ? "group" : "groups";
  return `Found ${visibleMatchCount.value} risky untracked ${fileLabel} in ${visibleGroupCount.value} ${groupLabel}.`;
});

watch(
  () => props.repoPath,
  () => {
    dismissedSuggestionIds.value = new Set();
    knownSuggestionIds.value = new Set();
  },
);

watch(
  visibleSuggestions,
  (suggestions) => {
    const visibleIds = new Set(suggestions.map((suggestion) => suggestion.id));
    const nextSelected = new Set(
      Array.from(selectedSuggestionIds.value).filter((id) => visibleIds.has(id)),
    );

    for (const suggestion of suggestions) {
      if (!knownSuggestionIds.value.has(suggestion.id)) {
        nextSelected.add(suggestion.id);
      }
    }

    selectedSuggestionIds.value = nextSelected;
    knownSuggestionIds.value = visibleIds;
  },
  { immediate: true },
);

watch(
  () => [props.repoPath, suggestionState.value.matchedFileCount] as const,
  () => {
    void loadGitignore();
  },
  { immediate: true },
);

function toggleSuggestion(id: string): void {
  const next = new Set(selectedSuggestionIds.value);
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  selectedSuggestionIds.value = next;
}

function isSelected(id: string): boolean {
  return selectedSuggestionIds.value.has(id);
}

function openGitignore(): void {
  emit("openGitignore");
}

function keepTracking(): void {
  const suggestionsToDismiss = selectedSuggestions.value.length > 0
    ? selectedSuggestions.value
    : visibleSuggestions.value;

  if (suggestionsToDismiss.length === 0) return;

  dismissedSuggestionIds.value = new Set([
    ...dismissedSuggestionIds.value,
    ...suggestionsToDismiss.map((suggestion) => suggestion.id),
  ]);
  toast.info("Smart .gitignore suggestions dismissed for this session.");
}

async function readGitignore(): Promise<string> {
  if (!props.repoPath) return "";

  try {
    return await invoke<string>("get_file_content", {
      path: props.repoPath,
      filePath: ".gitignore",
      sha: null,
    });
  } catch {
    return "";
  }
}

async function loadGitignore(): Promise<void> {
  if (!props.repoPath || suggestionState.value.matchedFileCount === 0) {
    gitignoreContent.value = "";
    return;
  }

  const sequence = ++gitignoreLoadSequence;
  loadingGitignore.value = true;

  try {
    const content = await readGitignore();
    if (sequence === gitignoreLoadSequence) {
      gitignoreContent.value = content;
    }
  } finally {
    if (sequence === gitignoreLoadSequence) {
      loadingGitignore.value = false;
    }
  }
}

function appendPatterns(content: string, patterns: string[]): string {
  const normalized = content.length === 0 || content.endsWith("\n") ? content : `${content}\n`;
  const spacer = normalized.trim().length > 0 && !normalized.endsWith("\n\n") ? "\n" : "";
  return `${normalized}${spacer}# GitSwamp smart ignore\n${patterns.join("\n")}\n`;
}

async function applySelectedRules(): Promise<void> {
  const patterns = previewPatterns.value;
  if (!props.repoPath || patterns.length === 0 || applying.value) return;

  applying.value = true;
  try {
    const currentContent = await readGitignore();
    gitignoreContent.value = currentContent;

    const currentPatterns = parseIgnorePatterns(currentContent);
    const nextPatterns = patterns.filter((pattern) => !isPatternCovered(pattern, currentPatterns));
    if (nextPatterns.length === 0) {
      toast.info(".gitignore already has selected rules.");
      return;
    }

    const nextContent = appendPatterns(currentContent, nextPatterns);
    await invoke("save_file_content", {
      path: props.repoPath,
      filePath: ".gitignore",
      content: nextContent,
    });

    gitignoreContent.value = nextContent;
    toast.success(`Updated .gitignore with ${nextPatterns.length} rule${nextPatterns.length === 1 ? "" : "s"}.`);
    emit("applied");
  } catch (error) {
    console.error("Failed to update .gitignore", error);
    toast.error("Could not update .gitignore.");
  } finally {
    applying.value = false;
  }
}
</script>

<template>
  <div
    v-if="visibleSuggestions.length > 0"
    class="border-b border-[var(--border)] bg-[var(--card)]/70 px-3 py-3"
  >
    <div class="flex items-start justify-between gap-3">
      <div class="flex items-start gap-2 min-w-0">
        <div class="mt-0.5 w-7 h-7 rounded-md border border-[var(--primary)]/25 bg-[var(--primary)]/10 flex items-center justify-center flex-shrink-0">
          <Sparkles class="w-3.5 h-3.5 text-[var(--primary)]" />
        </div>
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <span class="text-xs font-semibold text-[var(--foreground)]">Smart .gitignore Assistant</span>
            <span class="text-[9px] uppercase tracking-wide text-[var(--primary)] bg-[var(--primary)]/10 border border-[var(--primary)]/20 rounded px-1.5 py-0.5">
              generated/private
            </span>
          </div>
          <p class="text-[10px] text-[var(--foreground)] mt-0.5">These look like generated/private files. Add to .gitignore?</p>
          <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">{{ summaryText }}</p>
        </div>
      </div>
      <div class="flex flex-wrap items-center justify-end gap-1.5 flex-shrink-0">
        <button
          type="button"
          class="px-2.5 py-1 rounded-md text-[10px] font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed bg-[var(--primary)] text-[var(--primary-foreground)] hover:bg-[var(--primary)]/90"
          :disabled="previewPatterns.length === 0 || applying || loadingGitignore"
          @click="applySelectedRules"
        >
          {{ applying ? "Saving..." : "Ignore selected" }}
        </button>
        <button
          type="button"
          class="px-2.5 py-1 rounded-md text-[10px] font-semibold border border-[var(--border)] text-[var(--foreground)] bg-[var(--background)] hover:bg-[var(--secondary)] transition-colors"
          @click="openGitignore"
        >
          Open .gitignore
        </button>
        <button
          type="button"
          class="px-2.5 py-1 rounded-md text-[10px] font-semibold border border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] bg-transparent hover:bg-[var(--secondary)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="visibleSuggestions.length === 0 || applying"
          @click="keepTracking"
        >
          Keep tracking
        </button>
      </div>
    </div>

    <TransitionGroup name="smart-ignore" tag="div" class="mt-3 space-y-1.5">
      <button
        v-for="suggestion in visibleSuggestions"
        :key="suggestion.id"
        type="button"
        class="w-full rounded-md border border-[var(--border)] bg-[var(--background)]/70 px-2.5 py-2 text-left transition-all hover:border-[var(--primary)]/35 hover:bg-[var(--secondary)]"
        @click="toggleSuggestion(suggestion.id)"
      >
        <div class="flex items-start gap-2">
          <span
            class="mt-0.5 w-4 h-4 rounded border flex items-center justify-center flex-shrink-0 transition-colors"
            :class="isSelected(suggestion.id) ? 'border-[var(--primary)] bg-[var(--primary)] text-[var(--primary-foreground)]' : 'border-[var(--border)] text-transparent'"
          >
            <Check class="w-3 h-3" />
          </span>
          <FileWarning class="mt-0.5 w-3.5 h-3.5 text-[#f59e0b] flex-shrink-0" />
          <div class="min-w-0 flex-1">
            <div class="flex items-center justify-between gap-2">
              <span class="text-[11px] font-medium text-[var(--foreground)] truncate">{{ suggestion.label }}</span>
              <span class="text-[10px] text-[var(--muted-foreground)] flex-shrink-0">{{ suggestion.fileCount }}</span>
            </div>
            <p class="text-[10px] text-[var(--muted-foreground)] truncate">{{ suggestion.description }}</p>
            <div class="mt-1 flex flex-wrap gap-1">
              <span
                v-for="pattern in suggestion.patterns.slice(0, 5)"
                :key="`${suggestion.id}-${pattern}`"
                class="font-mono text-[9px] rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-0.5 text-[var(--muted-foreground)]"
              >
                {{ pattern }}
              </span>
            </div>
            <div class="mt-1 text-[9px] text-[var(--muted-foreground)] truncate">
              {{ suggestion.samples.join(" | ") }}
            </div>
          </div>
        </div>
      </button>
    </TransitionGroup>

    <div v-if="previewPatterns.length > 0" class="mt-2 rounded-md border border-[var(--border)] bg-[var(--input-background)] px-2.5 py-2">
      <div class="flex items-center justify-between gap-2 mb-1">
        <span class="text-[10px] font-semibold text-[var(--foreground)]">.gitignore preview</span>
        <span class="text-[9px] text-[var(--muted-foreground)]">{{ previewPatterns.length }} rules</span>
      </div>
      <pre class="max-h-24 overflow-y-auto text-[10px] leading-4 font-mono text-[var(--muted-foreground)] whitespace-pre-wrap"># GitSwamp smart ignore
{{ previewPatterns.join("\n") }}</pre>
    </div>
  </div>
</template>

<style scoped>
.smart-ignore-enter-active,
.smart-ignore-leave-active {
  transition: opacity 160ms ease, transform 160ms ease;
}

.smart-ignore-enter-from,
.smart-ignore-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  FileSearch,
  Loader2,
  Search,
  SlidersHorizontal,
  TerminalSquare,
  X,
} from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import {
  parsePickaxeLog,
  parsePickaxePatchSnippets,
  type PickaxeCommitHit,
  type PickaxeFileChange,
  type PickaxeSnippetLine,
} from "@/features/repository/pickaxe/pickaxeSearch";

interface PickaxeResultRow {
  id: string;
  hit: PickaxeCommitHit;
  file: PickaxeFileChange;
}

const props = defineProps<{
  visible: boolean;
  repoPath: string;
}>();

const emit = defineEmits<{
  close: [];
  openResult: [payload: { sha: string; filePath: string; fileStatus: PickaxeFileChange["kind"]; oldPath?: string | null; hit: PickaxeCommitHit }];
}>();

const query = ref("");
const pathFilter = ref("");
const authorFilter = ref("");
const maxResults = ref(50);
const allResults = ref(false);
const allRefs = ref(true);
const includeMerges = ref(false);
const showFullCommitFiles = ref(false);
const caseSensitive = ref(true);
const searchMode = ref<"string" | "regex">("string");
const sinceFilter = ref("");
const searching = ref(false);
const detailLoading = ref(false);
const hasSearched = ref(false);
const error = ref("");
const hits = ref<PickaxeCommitHit[]>([]);
const selectedResultId = ref("");
const snippets = ref<PickaxeSnippetLine[]>([]);
const inputRef = ref<HTMLInputElement | null>(null);
let searchRunId = 0;
let detailRunId = 0;

const resultRows = computed<PickaxeResultRow[]>(() => {
  const rows: PickaxeResultRow[] = [];
  for (const hit of hits.value) {
    for (const file of hit.files) {
      rows.push({
        id: `${hit.sha}:${file.status}:${file.oldPath || ""}:${file.path}`,
        hit,
        file,
      });
    }
  }
  return rows;
});

const selectedRow = computed(() => resultRows.value.find((row) => row.id === selectedResultId.value) || null);
const fileCount = computed(() => {
  const files = new Set<string>();
  for (const row of resultRows.value) {
    files.add(row.file.path);
  }
  return files.size;
});

watch(
  () => props.visible,
  async (visible) => {
    if (!visible) return;
    error.value = "";
    await nextTick();
    inputRef.value?.focus();
  },
);

watch(selectedResultId, () => {
  void loadSelectedPatch();
});

function close(): void {
  emit("close");
}

function clearSearch(): void {
  query.value = "";
  error.value = "";
  hits.value = [];
  snippets.value = [];
  selectedResultId.value = "";
  hasSearched.value = false;
  void nextTick(() => inputRef.value?.focus());
}

function closeResults(): void {
  hasSearched.value = false;
  hits.value = [];
  snippets.value = [];
  selectedResultId.value = "";
  error.value = "";
}

function changeBadgeClass(kind: PickaxeFileChange["kind"]): string {
  if (kind === "added") return "text-[#10b981] bg-[#10b981]/10";
  if (kind === "deleted") return "text-[#ef4444] bg-[#ef4444]/10";
  if (kind === "renamed" || kind === "copied") return "text-[#8b5cf6] bg-[#8b5cf6]/10";
  if (kind === "modified") return "text-[#38bdf8] bg-[#38bdf8]/10";
  return "text-[var(--muted-foreground)] bg-[var(--secondary)]";
}

function snippetClass(kind: PickaxeSnippetLine["kind"]): string {
  if (kind === "added") return "text-[#10b981]";
  if (kind === "deleted") return "text-[#f87171]";
  return "text-[var(--muted-foreground)]";
}

function snippetPrefix(kind: PickaxeSnippetLine["kind"]): string {
  if (kind === "added") return "+";
  if (kind === "deleted") return "-";
  return " ";
}

function snippetLineNumber(snippet: PickaxeSnippetLine): string {
  if (snippet.kind === "added") return String(snippet.newLine || "");
  if (snippet.kind === "deleted") return String(snippet.oldLine || "");
  return String(snippet.newLine || snippet.oldLine || "");
}

function buildLogArgs(): string[] {
  const args = [
    "-c",
    "core.quotePath=false",
    "log",
    "--date=short",
    "--find-renames",
    searchMode.value === "regex" ? `-G${query.value.trim()}` : `-S${query.value.trim()}`,
  ];

  if (!allResults.value) {
    args.push(`--max-count=${maxResults.value}`);
  }
  if (allRefs.value) {
    args.push("--all");
  }
  if (!includeMerges.value) {
    args.push("--no-merges");
  }
  if (showFullCommitFiles.value) {
    args.push("--pickaxe-all");
  }
  if (!caseSensitive.value) {
    args.push("-i");
  }
  if (authorFilter.value.trim()) {
    args.push(`--author=${authorFilter.value.trim()}`);
  }
  if (sinceFilter.value) {
    args.push(`--since=${sinceFilter.value}`);
  }

  args.push("--format=%x1e%H%x1f%h%x1f%an%x1f%ct%x1f%ad%x1f%s", "--name-status", "--");
  if (pathFilter.value.trim()) {
    args.push(pathFilter.value.trim());
  }
  return args;
}

function buildShowArgs(row: PickaxeResultRow): string[] {
  const args = [
    "-c",
    "core.quotePath=false",
    "show",
    "--format=",
    "--find-renames",
    "--unified=4",
    searchMode.value === "regex" ? `-G${query.value.trim()}` : `-S${query.value.trim()}`,
  ];

  if (showFullCommitFiles.value) {
    args.push("--pickaxe-all");
  }
  if (!caseSensitive.value) {
    args.push("-i");
  }

  args.push(row.hit.sha, "--", row.file.path);
  return args;
}

async function runGit(args: string[]): Promise<string> {
  return invoke<string>("run_git_command", {
    path: props.repoPath,
    args,
  });
}

async function runSearch(): Promise<void> {
  const needle = query.value.trim();
  if (!needle) {
    error.value = "Enter an exact string to search through history.";
    hasSearched.value = true;
    return;
  }

  const runId = ++searchRunId;
  searching.value = true;
  detailLoading.value = false;
  hasSearched.value = true;
  error.value = "";
  hits.value = [];
  snippets.value = [];
  selectedResultId.value = "";

  try {
    const output = await runGit(buildLogArgs());
    if (runId !== searchRunId) return;

    hits.value = parsePickaxeLog(output);
    selectedResultId.value = resultRows.value[0]?.id || "";
    if (hits.value.length === 0) {
      error.value = "No commits changed that exact string.";
    }
  } catch (err) {
    if (runId === searchRunId) {
      error.value = String(err);
      hits.value = [];
      selectedResultId.value = "";
    }
  } finally {
    if (runId === searchRunId) {
      searching.value = false;
    }
  }
}

async function loadSelectedPatch(): Promise<void> {
  const row = selectedRow.value;
  const needle = query.value.trim();
  if (!row || !needle) {
    snippets.value = [];
    return;
  }

  const runId = ++detailRunId;
  detailLoading.value = true;
  snippets.value = [];

  try {
    const output = await runGit(buildShowArgs(row));
    if (runId !== detailRunId) return;
    snippets.value = parsePickaxePatchSnippets(output.slice(0, 160_000), needle, caseSensitive.value, 120);
  } catch {
    if (runId === detailRunId) {
      snippets.value = [];
    }
  } finally {
    if (runId === detailRunId) {
      detailLoading.value = false;
    }
  }
}

function openResult(row: PickaxeResultRow): void {
  selectedResultId.value = row.id;
  emit("openResult", {
    sha: row.hit.sha,
    filePath: row.file.path,
    fileStatus: row.file.kind,
    oldPath: row.file.oldPath || null,
    hit: row.hit,
  });
}

function onKeyDown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    void runSearch();
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="pointer-events-none fixed inset-0 z-[7350]">
      <section
        v-if="!hasSearched && !searching"
        class="pointer-events-auto absolute left-1/2 top-1/2 w-[720px] max-w-[94vw] -translate-x-1/2 -translate-y-1/2 rounded-xl bg-[var(--popover)]/98 p-3 shadow-2xl ring-1 ring-[var(--border)]"
        @keydown="onKeyDown"
      >
        <div class="mb-2 flex items-center justify-between gap-3">
          <div class="flex items-center gap-2">
            <FileSearch class="h-4 w-4 text-[var(--primary)]" />
            <div>
              <div class="text-sm font-semibold text-[var(--foreground)]">Smart Search</div>
              <div class="text-[10px] text-[var(--muted-foreground)]">Pickaxe search through Git history</div>
            </div>
          </div>
          <CloseIconButton size="sm" title="Close smart search" @click="close" />
        </div>

        <div class="flex items-center gap-2">
          <div class="relative min-w-0 flex-1">
            <Search class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--muted-foreground)]" />
            <input
              ref="inputRef"
              v-model="query"
              class="h-9 w-full rounded-md bg-[var(--input)] py-0 pl-8 pr-8 text-xs text-[var(--foreground)] outline-none ring-1 ring-[var(--border)] placeholder:text-[var(--muted-foreground)] focus:ring-[var(--primary)]"
              placeholder="Exact string, function, flag or variable..."
              @keydown="onKeyDown"
            >
            <button
              v-if="query"
              type="button"
              class="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]"
              title="Clear search"
              @click="clearSearch"
            >
              <X class="h-3.5 w-3.5" />
            </button>
          </div>

          <input
            v-model="pathFilter"
            class="h-9 w-[180px] rounded-md bg-[var(--input)] px-2.5 text-xs text-[var(--foreground)] outline-none ring-1 ring-[var(--border)] placeholder:text-[var(--muted-foreground)] focus:ring-[var(--primary)]"
            placeholder="Path filter"
            @keydown="onKeyDown"
          >

          <button
            type="button"
            class="inline-flex h-9 items-center gap-2 rounded-md bg-[var(--primary)] px-3 text-xs font-semibold text-[var(--primary-foreground)] hover:opacity-90 disabled:opacity-50"
            :disabled="searching"
            @click="runSearch"
          >
            <Loader2 v-if="searching" class="h-3.5 w-3.5 animate-spin" />
            <Search v-else class="h-3.5 w-3.5" />
            Search
          </button>
        </div>

        <div class="mt-2 grid grid-cols-2 gap-2">
          <input
            v-model="authorFilter"
            class="h-8 rounded-md bg-[var(--input)] px-2.5 text-[11px] text-[var(--foreground)] outline-none ring-1 ring-[var(--border)] placeholder:text-[var(--muted-foreground)] focus:ring-[var(--primary)]"
            placeholder="Author filter"
            @keydown="onKeyDown"
          >
          <select
            v-model="sinceFilter"
            class="h-8 rounded-md bg-[var(--input)] px-2.5 text-[11px] text-[var(--foreground)] outline-none ring-1 ring-[var(--border)]"
          >
            <option value="">Any time</option>
            <option value="30 days ago">Last 30 days</option>
            <option value="90 days ago">Last 90 days</option>
            <option value="1 year ago">Last year</option>
          </select>
        </div>

        <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1.5 text-[10px] text-[var(--muted-foreground)]">
          <span class="inline-flex items-center gap-1 text-[var(--foreground)]">
            <SlidersHorizontal class="h-3.5 w-3.5 text-[var(--primary)]" />
            Options
          </span>
          <label class="inline-flex items-center gap-1.5">
            <span>mode</span>
            <select v-model="searchMode" class="h-6 rounded bg-[var(--input)] px-1 text-[10px] text-[var(--foreground)] outline-none ring-1 ring-[var(--border)]">
              <option value="string">exact string</option>
              <option value="regex">regex diff</option>
            </select>
          </label>
          <label class="inline-flex items-center gap-1.5">
            <input v-model="allRefs" type="checkbox" class="accent-[var(--primary)]">
            all commits and refs
          </label>
          <label class="inline-flex items-center gap-1.5">
            <input v-model="allResults" type="checkbox" class="accent-[var(--primary)]">
            all results
          </label>
          <label v-if="!allResults" class="inline-flex items-center gap-1.5">
            <span>limit</span>
            <select v-model.number="maxResults" class="h-6 rounded bg-[var(--input)] px-1 text-[10px] text-[var(--foreground)] outline-none ring-1 ring-[var(--border)]">
              <option :value="25">25</option>
              <option :value="50">50</option>
              <option :value="100">100</option>
              <option :value="200">200</option>
            </select>
          </label>
          <label class="inline-flex items-center gap-1.5">
            <input v-model="includeMerges" type="checkbox" class="accent-[var(--primary)]">
            merges
          </label>
          <label class="inline-flex items-center gap-1.5">
            <input v-model="showFullCommitFiles" type="checkbox" class="accent-[var(--primary)]">
            full files
          </label>
          <label class="inline-flex items-center gap-1.5">
            <input v-model="caseSensitive" type="checkbox" class="accent-[var(--primary)]">
            case sensitive
          </label>
        </div>
      </section>

      <section
        v-if="hasSearched || searching"
        class="pointer-events-auto absolute inset-x-0 bottom-0 h-[38vh] min-h-[260px] overflow-hidden border-t border-[var(--border)] bg-[#070a10]/98 shadow-2xl"
      >
        <div class="flex h-10 items-center justify-between border-b border-white/10 bg-[var(--background)] px-3">
          <div class="flex items-center gap-2">
            <TerminalSquare class="h-4 w-4 text-[var(--primary)]" />
            <div>
              <div class="text-xs font-semibold text-[var(--foreground)]">Search results</div>
              <div class="text-[10px] text-[var(--muted-foreground)]">
                {{ searching ? "searching..." : `${hits.length} commit${hits.length === 1 ? "" : "s"}, ${fileCount} file${fileCount === 1 ? "" : "s"}` }}
              </div>
            </div>
          </div>
          <div class="flex items-center gap-2 text-[10px] text-[var(--muted-foreground)]">
            <span class="font-mono">git log {{ searchMode === "regex" ? "-G" : "-S" }} "{{ query || "string" }}"</span>
            <CloseIconButton size="sm" title="Close results" @click="closeResults" />
          </div>
        </div>

        <div class="grid h-[calc(100%-2.5rem)] grid-cols-[minmax(340px,42%)_minmax(0,1fr)] overflow-hidden">
          <div class="min-h-0 overflow-y-auto border-r border-white/10 p-2 no-scrollbar">
            <div v-if="searching" class="flex h-full items-center justify-center gap-2 text-xs text-[var(--muted-foreground)]">
              <Loader2 class="h-4 w-4 animate-spin" />
              Searching Git history...
            </div>

            <div v-else-if="error && resultRows.length === 0" class="p-4 text-xs text-[#f87171]">
              {{ error }}
            </div>

            <div v-else-if="resultRows.length === 0" class="p-4 text-xs text-[var(--muted-foreground)]">
              No results.
            </div>

            <div v-else class="space-y-1">
              <button
                v-for="row in resultRows"
                :key="row.id"
                type="button"
                class="w-full rounded-md px-2.5 py-2 text-left transition-colors"
                :class="selectedResultId === row.id ? 'bg-[var(--primary)]/14' : 'hover:bg-white/5'"
                @click="openResult(row)"
              >
                <div class="flex items-center justify-between gap-3">
                  <span class="min-w-0 truncate text-xs font-semibold text-[var(--foreground)]">{{ row.hit.subject }}</span>
                  <span class="font-mono text-[10px] text-[var(--primary)]">{{ row.hit.shortSha }}</span>
                </div>
                <div class="mt-1 flex items-center gap-2 text-[10px] text-[var(--muted-foreground)]">
                  <span class="truncate">{{ row.hit.author }}</span>
                  <span>{{ row.hit.date }}</span>
                </div>
                <div class="mt-1 flex items-center gap-2">
                  <span class="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase" :class="changeBadgeClass(row.file.kind)">
                    {{ row.file.kind }}
                  </span>
                  <span class="min-w-0 truncate font-mono text-[10px] text-[#94a3b8]" :title="row.file.path">{{ row.file.path }}</span>
                </div>
              </button>
            </div>
          </div>

          <div class="min-h-0 overflow-auto p-3 font-mono text-[11px] no-scrollbar">
            <div v-if="detailLoading" class="flex h-full items-center justify-center gap-2 text-[var(--muted-foreground)]">
              <Loader2 class="h-4 w-4 animate-spin" />
              Loading matching lines...
            </div>

            <div v-else-if="!selectedRow" class="text-[var(--muted-foreground)]">
              Click a result to select its commit, open its diff and inspect matching lines.
            </div>

            <div v-else-if="snippets.length === 0" class="text-[var(--muted-foreground)]">
              No direct matching patch lines were returned for this file. The commit and file still matched Git pickaxe.
            </div>

            <div v-else class="space-y-1">
              <div
                v-for="snippet in snippets"
                :key="snippet.id"
                class="grid grid-cols-[64px_minmax(0,1fr)_minmax(150px,260px)] gap-3 rounded px-2 py-1 hover:bg-white/5"
                :class="snippetClass(snippet.kind)"
              >
                <span class="text-right text-[#64748b]">{{ snippetPrefix(snippet.kind) }}{{ snippetLineNumber(snippet) }}</span>
                <span class="min-w-0 whitespace-pre-wrap break-words">{{ snippet.text }}</span>
                <span class="min-w-0 truncate text-right text-[#94a3b8]" :title="`${snippet.kind}: ${snippet.file}`">{{ snippet.kind }} · {{ snippet.file }}</span>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.no-scrollbar {
  scrollbar-width: none;
}

.no-scrollbar::-webkit-scrollbar {
  width: 0;
  height: 0;
}
</style>

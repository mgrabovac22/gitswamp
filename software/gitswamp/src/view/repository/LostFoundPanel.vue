<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  GitBranchPlus,
  LifeBuoy,
  Loader2,
  RefreshCw,
  Search,
} from "lucide-vue-next";
import type { CommitFileInfo, LostCommitInfo } from "@/types";

const LOST_FOUND_FILE_DISPLAY_LIMIT = 350;

const props = defineProps<{
  repoPath: string;
  lostCommits: LostCommitInfo[];
  loading?: boolean;
  rescuingSha?: string | null;
}>();

const emit = defineEmits<{
  close: [];
  refresh: [];
  rescue: [payload: { sha: string; branchName: string }];
  openDiff: [payload: { path: string; sha: string }];
}>();

const query = ref("");
const selectedSha = ref<string | null>(null);
const selectedFiles = ref<CommitFileInfo[]>([]);
const selectedFileTotals = ref({ additions: 0, deletions: 0 });
const filesTruncated = ref(false);
const filesLoading = ref(false);
const filesError = ref<string | null>(null);
let filesRequestId = 0;

const filteredCommits = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return props.lostCommits;

  return props.lostCommits.filter((commit) =>
    commit.sha.toLowerCase().includes(q)
    || commit.short_sha.toLowerCase().includes(q)
    || commit.message.toLowerCase().includes(q)
    || commit.author_name.toLowerCase().includes(q),
  );
});

const selectedCommit = computed(() =>
  props.lostCommits.find((commit) => commit.sha === selectedSha.value) || filteredCommits.value[0] || null,
);

watch(
  () => props.lostCommits.map((commit) => commit.sha).join("|"),
  () => {
    if (!props.lostCommits.some((commit) => commit.sha === selectedSha.value)) {
      selectedSha.value = props.lostCommits[0]?.sha || null;
    }
  },
  { immediate: true },
);

watch(
  () => selectedCommit.value?.sha || "",
  (sha) => {
    if (!sha) {
      selectedFiles.value = [];
      selectedFileTotals.value = { additions: 0, deletions: 0 };
      filesTruncated.value = false;
      filesError.value = null;
      return;
    }
    void loadCommitFiles(sha);
  },
  { immediate: true },
);

function safeBranchSegment(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 28);
}

function rescueBranchName(commit: LostCommitInfo): string {
  const message = safeBranchSegment(commit.message || "lost-commit");
  return `rescue/${commit.short_sha}${message ? `-${message}` : ""}`;
}

function formatDate(timestamp: number): string {
  if (!timestamp) return "Unknown date";
  return new Date(timestamp * 1000).toLocaleString();
}

function statusClass(status: string): string {
  const normalized = status.toLowerCase();
  if (normalized.includes("add")) return "text-[#10b981]";
  if (normalized.includes("delete")) return "text-[#ef4444]";
  if (normalized.includes("rename")) return "text-[#38bdf8]";
  return "text-[#f59e0b]";
}

async function loadCommitFiles(sha: string) {
  const requestId = ++filesRequestId;
  filesLoading.value = true;
  filesError.value = null;
  selectedFiles.value = [];
  selectedFileTotals.value = { additions: 0, deletions: 0 };
  filesTruncated.value = false;

  try {
    const files = await invoke<CommitFileInfo[]>("get_commit_files", { path: props.repoPath, sha });
    if (requestId !== filesRequestId) return;
    let additions = 0;
    let deletions = 0;
    for (const file of files) {
      additions += Number(file.additions) || 0;
      deletions += Number(file.deletions) || 0;
    }
    selectedFileTotals.value = { additions, deletions };
    filesTruncated.value = files.length > LOST_FOUND_FILE_DISPLAY_LIMIT;
    selectedFiles.value = filesTruncated.value ? files.slice(0, LOST_FOUND_FILE_DISPLAY_LIMIT) : files;
  } catch (error) {
    if (requestId !== filesRequestId) return;
    selectedFiles.value = [];
    selectedFileTotals.value = { additions: 0, deletions: 0 };
    filesTruncated.value = false;
    filesError.value = String(error);
  } finally {
    if (requestId === filesRequestId) {
      filesLoading.value = false;
    }
  }
}
</script>

<template>
  <div class="flex h-full min-w-0 flex-1 flex-col bg-[var(--background)]">
    <div class="flex h-12 flex-shrink-0 items-center justify-between border-b border-[var(--border)] bg-[var(--card)] px-4">
      <div class="flex min-w-0 items-center gap-3">
        <button
          class="flex h-8 w-8 items-center justify-center rounded border border-[var(--border)] text-[var(--muted-foreground)] transition-colors hover:bg-[var(--secondary)] hover:text-[var(--foreground)]"
          title="Back to graph"
          @click="emit('close')"
        >
          <ArrowLeft class="h-4 w-4" />
        </button>
        <div class="flex h-8 w-8 items-center justify-center rounded border border-[#10b981]/25 bg-[#10b981]/10 text-[#34d399]">
          <LifeBuoy class="h-4 w-4" />
        </div>
        <div class="min-w-0">
          <div class="truncate text-sm font-semibold text-[var(--foreground)]">Lost &amp; Found</div>
          <div class="truncate text-[10px] text-[var(--muted-foreground)]">
            Recover commits that are no longer attached to a branch.
          </div>
        </div>
      </div>

      <button
        class="inline-flex h-8 items-center gap-1.5 rounded border border-[var(--border)] px-2 text-[11px] text-[var(--muted-foreground)] transition-colors hover:bg-[var(--secondary)] hover:text-[var(--foreground)] disabled:cursor-wait disabled:opacity-60"
        :disabled="loading"
        @click="emit('refresh')"
      >
        <Loader2 v-if="loading" class="h-3.5 w-3.5 animate-spin" />
        <RefreshCw v-else class="h-3.5 w-3.5" />
        Rescan
      </button>
    </div>

    <div class="grid min-h-0 flex-1 grid-cols-[minmax(260px,34%)_1fr] overflow-hidden">
      <aside class="flex min-h-0 flex-col border-r border-[var(--border)] bg-[var(--card)]/55">
        <div class="border-b border-[var(--border)] p-3">
          <div class="relative">
            <Search class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--muted-foreground)]" />
            <input
              v-model="query"
              class="h-8 w-full rounded border border-[var(--border)] bg-[var(--background)] pl-8 pr-2 text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              placeholder="Search lost commits..."
            />
          </div>
        </div>

        <div class="min-h-0 flex-1 overflow-y-auto">
          <button
            v-for="commit in filteredCommits"
            :key="commit.sha"
            class="w-full border-b border-[var(--border)]/55 px-3 py-2.5 text-left transition-colors"
            :class="selectedCommit?.sha === commit.sha
              ? 'bg-[var(--primary)]/10 text-[var(--foreground)]'
              : 'text-[var(--muted-foreground)] hover:bg-[var(--secondary)]/70 hover:text-[var(--foreground)]'"
            @click="selectedSha = commit.sha"
          >
            <div class="flex items-center justify-between gap-2">
              <span class="min-w-0 truncate text-xs font-semibold">{{ commit.message || "No commit message" }}</span>
              <span class="flex-shrink-0 rounded border border-[#10b981]/25 bg-[#10b981]/10 px-1.5 py-0.5 font-mono text-[9px] text-[#34d399]">{{ commit.short_sha }}</span>
            </div>
            <div class="mt-1 flex items-center justify-between gap-2 text-[10px]">
              <span class="min-w-0 truncate">{{ commit.author_name }}</span>
              <span class="flex-shrink-0">{{ commit.time_ago }}</span>
            </div>
            <div class="mt-1 text-[9px] uppercase tracking-wide text-[var(--muted-foreground)]/75">
              {{ commit.source }}
            </div>
          </button>

          <div v-if="filteredCommits.length === 0" class="px-4 py-5 text-xs text-[var(--muted-foreground)]">
            No lost commits match this search.
          </div>
        </div>
      </aside>

      <main class="min-h-0 overflow-y-auto p-4">
        <div v-if="selectedCommit" class="mx-auto max-w-5xl space-y-4">
          <section class="rounded border border-[var(--border)] bg-[var(--card)] p-4">
            <div class="flex items-start justify-between gap-4">
              <div class="min-w-0">
                <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Recovery candidate</div>
                <h2 class="mt-1 truncate text-base font-semibold text-[var(--foreground)]">
                  {{ selectedCommit.message || "No commit message" }}
                </h2>
                <div class="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-[var(--muted-foreground)]">
                  <span>{{ selectedCommit.author_name }}</span>
                  <span class="font-mono">{{ selectedCommit.short_sha }}</span>
                  <span>{{ formatDate(selectedCommit.timestamp) }}</span>
                  <span>{{ selectedCommit.parent_shas.length }} parent{{ selectedCommit.parent_shas.length === 1 ? "" : "s" }}</span>
                </div>
              </div>

              <button
                class="inline-flex h-8 flex-shrink-0 items-center gap-1.5 rounded bg-[var(--primary)] px-3 text-xs font-semibold text-[var(--primary-foreground)] transition-opacity hover:opacity-90 disabled:cursor-wait disabled:opacity-60"
                :disabled="rescuingSha === selectedCommit.sha"
                @click="emit('rescue', { sha: selectedCommit.sha, branchName: rescueBranchName(selectedCommit) })"
              >
                <Loader2 v-if="rescuingSha === selectedCommit.sha" class="h-3.5 w-3.5 animate-spin" />
                <GitBranchPlus v-else class="h-3.5 w-3.5" />
                Rescue as branch
              </button>
            </div>
          </section>

          <section class="rounded border border-[var(--border)] bg-[var(--card)]">
            <div class="flex items-center justify-between border-b border-[var(--border)] px-4 py-3">
              <div>
                <div class="text-sm font-semibold text-[var(--foreground)]">Changed files</div>
                <div class="text-[10px] text-[var(--muted-foreground)]">
                  {{ filesTruncated ? `${selectedFiles.length}+` : selectedFiles.length }} files,
                  <span class="text-[#10b981]">+{{ selectedFileTotals.additions }}</span>
                  <span class="text-[#ef4444]">-{{ selectedFileTotals.deletions }}</span>
                </div>
              </div>
              <Loader2 v-if="filesLoading" class="h-4 w-4 animate-spin text-[var(--muted-foreground)]" />
            </div>

            <div v-if="filesError" class="px-4 py-3 text-xs text-[#ef4444]">
              Could not load commit files: {{ filesError }}
            </div>

            <div v-else-if="!filesLoading && selectedFiles.length === 0" class="px-4 py-4 text-xs text-[var(--muted-foreground)]">
              No file changes were reported for this commit.
            </div>

            <div v-else class="divide-y divide-[var(--border)]/65">
              <button
                v-for="file in selectedFiles"
                :key="`${selectedCommit.sha}-${file.path}`"
                class="grid w-full grid-cols-[minmax(0,1fr)_auto_auto_auto] items-center gap-3 px-4 py-2 text-left text-xs transition-colors hover:bg-[var(--secondary)]/60"
                @click="emit('openDiff', { path: file.path, sha: selectedCommit.sha })"
              >
                <span class="min-w-0 truncate text-[var(--foreground)]">{{ file.path }}</span>
                <span class="uppercase tracking-wide text-[9px]" :class="statusClass(file.status)">{{ file.status }}</span>
                <span class="font-mono text-[10px] text-[#10b981]">+{{ file.additions }}</span>
                <span class="font-mono text-[10px] text-[#ef4444]">-{{ file.deletions }}</span>
              </button>
              <div v-if="filesTruncated" class="px-4 py-2 text-[10px] text-[var(--muted-foreground)]">
                Showing the first {{ selectedFiles.length }} files to keep recovery lightweight.
              </div>
            </div>
          </section>

          <p class="text-[11px] leading-5 text-[var(--muted-foreground)]">
            Rescue creates a normal local branch at this commit. It does not checkout or modify your working tree.
          </p>
        </div>
      </main>
    </div>
  </div>
</template>

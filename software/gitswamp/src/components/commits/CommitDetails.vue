<script setup lang="ts">
import { ref, watch } from "vue";
import {
  Calendar,
  Hash,
  ChevronDown,
  FileText,
  GitBranch,
  ArrowUp,
  ArrowDown,
  Copy,
  Plus,
  Minus,
  Trash2,
  Archive,
  Eye,
} from "lucide-vue-next";
import AppButton from "@/components/ui/AppButton.vue";
import GitCommitIcon from "@/components/ui/GitCommitIcon.vue";
import type { CommitInfo, FileStatusInfo, CommitFileInfo, StashInfo } from "@/types";

const props = defineProps<{
  commit: CommitInfo | null;
  stagedFiles: FileStatusInfo[];
  unstagedFiles: FileStatusInfo[];
  commitFiles: CommitFileInfo[];
  isWorkingChanges: boolean;
  isStash?: boolean;
  selectedStash?: StashInfo | null;
  stashFiles?: CommitFileInfo[];
  repoPath: string;
}>();

const emit = defineEmits<{
  stage: [path: string];
  unstage: [path: string];
  stageAll: [];
  unstageAll: [];
  commit: [message: string];
  discard: [path: string];
  discardAll: [];
  stashPop: [index: number];
  stashApply: [index: number];
  stashDrop: [index: number];
  viewDiff: [{ path: string; sha: string | null; staged: boolean }];
}>();

const commitSummary = ref("");
const commitDescription = ref("");

function openDiff(filePath: string, commitSha: string | null, staged: boolean) {
  emit("viewDiff", { path: filePath, sha: commitSha, staged });
}

// Tab logic: working changes -> only "changes", commit/stash -> "changes" and "info"
const activeTab = ref<"changes" | "info">("changes");

watch(() => props.commit, (newVal) => {
  if (newVal) {
    activeTab.value = "info";
  } else {
    activeTab.value = "changes";
  }
});

watch(() => props.selectedStash, (newVal) => {
  if (newVal) {
    activeTab.value = "changes"; // Show files first for stash
  }
});

watch(() => props.isWorkingChanges, (val) => {
  if (val) activeTab.value = "changes";
});

const expandedStaged = ref(true);
const expandedUnstaged = ref(true);

function formatDate(timestamp: number) {
  const d = new Date(timestamp * 1000);
  return d.toLocaleDateString("en-GB", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  }) + " @ " + d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
}

function commitSubject(msg: string): string {
  const idx = msg.indexOf("\n");
  return idx > 0 ? msg.substring(0, idx) : msg;
}
function commitBody(msg: string): string {
  const idx = msg.indexOf("\n");
  return idx > 0 ? msg.substring(idx + 1).trim() : "";
}

function parentRefs(commit: CommitInfo): string[] {
  return commit.parent_shas.map((s) => s.substring(0, 7));
}

function branchRefs(commit: CommitInfo): string[] {
  return commit.refs.filter((r) => !r.includes("HEAD") && !r.includes("->"));
}

function refStatus(commit: CommitInfo): { local: boolean; remote: boolean } {
  const refs = commit.refs;
  const hasLocal = refs.some((r) => !r.startsWith("origin/") && !r.includes("HEAD") && !r.includes("->"));
  const hasRemote = refs.some((r) => r.startsWith("origin/"));
  return { local: hasLocal, remote: hasRemote };
}

function statusIcon(status: string): string {
  switch (status) {
    case "added": case "new": return "+";
    case "deleted": return "−";
    case "modified": return "M";
    case "renamed": return "R";
    default: return "?";
  }
}

function statusColor(status: string): string {
  switch (status) {
    case "added": case "new": return "#10b981";
    case "deleted": return "#ef4444";
    case "modified": return "#f59e0b";
    case "renamed": return "#06b6d4";
    default: return "#64748b";
  }
}

function onCommit() {
  if (!commitSummary.value.trim()) return;
  const msg = commitDescription.value.trim()
    ? `${commitSummary.value.trim()}\n\n${commitDescription.value.trim()}`
    : commitSummary.value.trim();
  emit("commit", msg);
  commitSummary.value = "";
  commitDescription.value = "";
}

function copyToClipboard(text: string) {
  navigator.clipboard.writeText(text);
}
</script>

<template>
  <div class="w-80 bg-[var(--card)] border-l border-[var(--border)] flex flex-col h-full overflow-hidden">
    <!-- Tabs -->
    <div class="border-b border-[var(--border)] flex-shrink-0">
      <div class="h-9 flex">
        <button
          @click="activeTab = 'changes'"
          :class="[
            'flex-1 text-xs font-medium tracking-wide transition-colors',
            activeTab === 'changes'
              ? 'text-[var(--primary)] border-b-2 border-[var(--primary)] bg-[var(--card)]'
              : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]',
          ]"
        >
          Changes
        </button>
        <button
          v-if="(commit && !isWorkingChanges) || (isStash && selectedStash)"
          @click="activeTab = 'info'"
          :class="[
            'flex-1 text-xs font-medium tracking-wide transition-colors',
            activeTab === 'info'
              ? 'text-[var(--primary)] border-b-2 border-[var(--primary)] bg-[var(--card)]'
              : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]',
          ]"
        >
          Info
        </button>
      </div>
    </div>

    <!-- Changes tab for working directory (uncommitted) -->
    <div v-show="activeTab === 'changes' && isWorkingChanges" class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 overflow-y-auto">
        <!-- Summary header -->
        <div v-if="stagedFiles.length > 0 || unstagedFiles.length > 0" class="px-3 py-2.5 border-b border-[var(--border)] bg-gradient-to-r from-[var(--primary)]/5 to-transparent">
          <div class="flex items-center gap-2 mb-1">
            <div class="w-2 h-2 rounded-full bg-[var(--primary)] animate-pulse" />
            <span class="text-[11px] font-semibold text-[var(--foreground)]">Working Changes</span>
          </div>
          <div class="flex items-center gap-3 text-[10px] text-[var(--muted-foreground)]">
            <span v-if="unstagedFiles.length > 0" class="flex items-center gap-1">
              <span class="w-1.5 h-1.5 rounded-full bg-[#f59e0b]" />
              {{ unstagedFiles.length }} unstaged
            </span>
            <span v-if="stagedFiles.length > 0" class="flex items-center gap-1">
              <span class="w-1.5 h-1.5 rounded-full bg-[#10b981]" />
              {{ stagedFiles.length }} staged
            </span>
          </div>
        </div>

        <!-- Unstaged (shown first) -->
        <div v-if="unstagedFiles.length > 0" class="border-b border-[var(--border)]">
          <div class="flex items-center justify-between px-3 py-2 bg-[var(--card)]">
            <button
              @click="expandedUnstaged = !expandedUnstaged"
              class="flex items-center gap-2 text-xs text-[var(--foreground)] hover:text-[var(--primary)] transition-all flex-1"
            >
              <ChevronDown
                :class="['w-3.5 h-3.5 transition-transform', !expandedUnstaged ? '-rotate-90' : '']"
              />
              <span class="font-medium">Unstaged</span>
              <span class="text-[10px] bg-[#f59e0b]/15 text-[#f59e0b] px-1.5 py-0.5 rounded-full font-semibold border border-[#f59e0b]/20">{{ unstagedFiles.length }}</span>
            </button>
            <div class="flex items-center gap-1">
              <button
                @click="emit('discardAll')"
                class="text-[10px] text-[#ef4444]/70 hover:text-[#ef4444] transition-colors px-1.5 py-0.5 rounded hover:bg-[#ef4444]/10"
                title="Discard all changes"
              >
                Discard All
              </button>
              <button
                @click="emit('stageAll')"
                class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors px-1.5 py-0.5 rounded hover:bg-[var(--secondary)]"
              >
                Stage All
              </button>
            </div>
          </div>
          <div v-if="expandedUnstaged">
            <div
              v-for="f in unstagedFiles"
              :key="'u-' + f.path"
              class="flex items-center gap-2 px-4 py-1.5 hover:bg-[var(--primary)]/5 transition-all group cursor-pointer"
            >
              <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
              <span class="text-xs text-[var(--foreground)] truncate flex-1 opacity-90" @click="openDiff(f.path, null, false)">{{ f.path }}</span>
              <button
                @click.stop="openDiff(f.path, null, false)"
                class="p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[var(--primary)]/20 transition-all"
                title="View diff"
              >
                <Eye class="w-3 h-3 text-[var(--muted-foreground)] hover:text-[var(--primary)]" />
              </button>
              <button
                @click.stop="emit('discard', f.path)"
                class="p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[#ef4444]/20 transition-all"
                title="Discard changes"
              >
                <Trash2 class="w-3 h-3 text-[var(--muted-foreground)] hover:text-[#ef4444]" />
              </button>
              <Plus class="w-3 h-3 text-[var(--muted-foreground)] opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer" @click.stop="emit('stage', f.path)" />
            </div>
          </div>
        </div>

        <!-- Staged -->
        <div v-if="stagedFiles.length > 0" class="border-b border-[var(--border)]">
          <div class="flex items-center justify-between px-3 py-2 bg-[var(--card)]">
            <button
              @click="expandedStaged = !expandedStaged"
              class="flex items-center gap-2 text-xs text-[var(--foreground)] hover:text-[var(--primary)] transition-all flex-1"
            >
              <ChevronDown
                :class="['w-3.5 h-3.5 transition-transform', !expandedStaged ? '-rotate-90' : '']"
              />
              <span class="font-medium">Staged</span>
              <span class="text-[10px] bg-[#10b981]/15 text-[#10b981] px-1.5 py-0.5 rounded-full font-semibold border border-[#10b981]/20">{{ stagedFiles.length }}</span>
            </button>
            <button
              @click="emit('unstageAll')"
              class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors px-1.5 py-0.5 rounded hover:bg-[var(--secondary)]"
            >
              Unstage All
            </button>
          </div>
          <div v-if="expandedStaged">
            <div
              v-for="f in stagedFiles"
              :key="'s-' + f.path"
              class="flex items-center gap-2 px-4 py-1.5 hover:bg-[var(--primary)]/5 transition-all group cursor-pointer"
            >
              <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
              <span class="text-xs text-[var(--foreground)] truncate flex-1 opacity-90" @click="openDiff(f.path, null, true)">{{ f.path }}</span>
              <button
                @click.stop="openDiff(f.path, null, true)"
                class="p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[var(--primary)]/20 transition-all"
                title="View diff"
              >
                <Eye class="w-3 h-3 text-[var(--muted-foreground)] hover:text-[var(--primary)]" />
              </button>
              <Minus class="w-3 h-3 text-[var(--muted-foreground)] opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer" @click.stop="emit('unstage', f.path)" />
            </div>
          </div>
        </div>

        <div v-if="stagedFiles.length === 0 && unstagedFiles.length === 0" class="px-4 py-12 flex flex-col items-center justify-center">
          <div class="w-14 h-14 rounded-full bg-[var(--card)] flex items-center justify-center mb-3 border border-[var(--border)]">
            <FileText class="w-6 h-6 text-[var(--muted-foreground)] opacity-40" />
          </div>
          <p class="text-xs text-[var(--muted-foreground)] font-medium">Working tree clean</p>
          <p class="text-[10px] text-[var(--muted-foreground)] opacity-60 mt-1">No uncommitted changes</p>
        </div>
      </div>

      <!-- Commit form -->
      <div class="border-t border-[var(--border)] p-3 bg-[var(--card)]/50 flex-shrink-0">
        <input
          v-model="commitSummary"
          type="text"
          placeholder="Commit message..."
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-2"
          @keyup.enter="onCommit"
        />
        <textarea
          v-model="commitDescription"
          placeholder="Description (optional)..."
          rows="2"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-none mb-2"
        />
        <AppButton
          class="w-full bg-[var(--primary)] hover:opacity-90 text-white text-xs font-medium h-8"
          :disabled="!commitSummary.trim() || stagedFiles.length === 0"
          @click="onCommit"
        >
          Commit Changes
        </AppButton>
      </div>
    </div>

    <!-- Changes tab for a selected commit -->
    <div v-show="activeTab === 'changes' && !isWorkingChanges && commit" class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 overflow-y-auto">
        <div v-if="commitFiles.length > 0">
          <div class="flex items-center gap-2 px-3 py-2 bg-[var(--card)] text-xs text-[var(--foreground)]">
            <span class="font-medium">Changed files</span>
            <span class="text-[10px] bg-[var(--primary)]/20 text-[var(--primary)] px-1.5 py-0.5 rounded-full">{{ commitFiles.length }}</span>
          </div>
          <div
            v-for="f in commitFiles"
            :key="f.path"
            class="flex items-center gap-2 px-4 py-1.5 hover:bg-[var(--primary)]/5 transition-all cursor-pointer group"
            @click="openDiff(f.path, commit!.sha, false)"
          >
            <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
            <span class="text-xs text-[var(--foreground)] truncate flex-1 opacity-90">{{ f.path }}</span>
            <Eye class="w-3 h-3 text-[var(--muted-foreground)] opacity-0 group-hover:opacity-100 transition-opacity" />
            <span v-if="f.additions > 0" class="text-[10px] text-[#10b981] font-mono">+{{ f.additions }}</span>
            <span v-if="f.deletions > 0" class="text-[10px] text-[#ef4444] font-mono">-{{ f.deletions }}</span>
          </div>
        </div>
        <div v-else class="px-4 py-12 flex flex-col items-center justify-center">
          <div class="w-12 h-12 rounded-full bg-[var(--card)] flex items-center justify-center mb-3 border border-[var(--border)]">
            <FileText class="w-6 h-6 text-[var(--muted-foreground)] opacity-40" />
          </div>
          <p class="text-xs text-[var(--muted-foreground)]">No file changes</p>
        </div>
      </div>
    </div>

    <!-- Info tab (only for commits) -->
    <div v-show="activeTab === 'info' && commit && !isWorkingChanges" class="flex-1 overflow-y-auto">
      <div v-if="commit" class="p-4 space-y-4">
        <!-- Commit message -->
        <div>
          <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">Commit Message</div>
          <div class="text-sm text-[var(--foreground)] leading-relaxed bg-[var(--input-background)] p-3 rounded border border-[var(--border)]">
            {{ commitSubject(commit.message) }}
          </div>
        </div>

        <!-- Description (body) -->
        <div v-if="commitBody(commit.message)">
          <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">Description</div>
          <div class="text-xs text-[var(--foreground)] opacity-80 leading-relaxed bg-[var(--input-background)] p-3 rounded border border-[var(--border)] whitespace-pre-wrap">
            {{ commitBody(commit.message) }}
          </div>
        </div>

        <div class="h-px bg-[var(--border)]" />

        <!-- Author -->
        <div class="flex items-center gap-3">
          <div class="w-8 h-8 rounded-full bg-gradient-to-br from-[var(--primary)] to-[var(--accent)] flex items-center justify-center text-white text-xs font-bold flex-shrink-0">
            {{ commit.author_name.charAt(0).toUpperCase() }}
          </div>
          <div>
            <div class="text-sm text-[var(--foreground)] font-medium">{{ commit.author_name }}</div>
            <div class="text-[10px] text-[var(--muted-foreground)]">{{ commit.author_email }}</div>
          </div>
        </div>

        <div class="h-px bg-[var(--border)]" />

        <!-- SHA -->
        <div>
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <Hash class="w-3 h-3" />
            SHA
          </div>
          <div class="flex items-center gap-2">
            <code class="text-xs text-[var(--primary)] bg-[var(--input-background)] px-2 py-1 rounded font-mono break-all">{{ commit.sha }}</code>
            <button
              @click="copyToClipboard(commit.sha)"
              class="p-1 rounded hover:bg-[var(--secondary)] transition-colors flex-shrink-0"
              title="Copy SHA"
            >
              <Copy class="w-3 h-3 text-[var(--muted-foreground)]" />
            </button>
          </div>
        </div>

        <!-- Date -->
        <div>
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <Calendar class="w-3 h-3" />
            Date
          </div>
          <div class="text-xs text-[var(--foreground)]">{{ formatDate(commit.timestamp) }}</div>
          <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5">{{ commit.time_ago }}</div>
        </div>

        <!-- Parents -->
        <div v-if="commit.parent_shas.length > 0">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <GitCommitIcon class="w-3 h-3" />
            {{ commit.parent_shas.length > 1 ? 'Parents' : 'Parent' }}
          </div>
          <div class="flex flex-wrap gap-1.5">
            <code
              v-for="p in parentRefs(commit)"
              :key="p"
              class="text-[11px] text-[var(--primary)] bg-[var(--input-background)] px-2 py-0.5 rounded font-mono cursor-pointer hover:bg-[var(--secondary)] transition-colors"
            >{{ p }}</code>
          </div>
        </div>

        <!-- Branches / Refs -->
        <div v-if="branchRefs(commit).length > 0">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <GitBranch class="w-3 h-3" />
            Branches
          </div>
          <div class="flex flex-wrap gap-1.5">
            <span
              v-for="r in branchRefs(commit)"
              :key="r"
              class="px-2 py-0.5 text-[10px] font-medium rounded bg-[var(--primary)]/15 text-[var(--primary)] border border-[var(--primary)]/20"
            >
              {{ r }}
            </span>
          </div>
          <!-- Push status -->
          <div class="mt-2 flex items-center gap-2 text-[10px]">
            <span v-if="refStatus(commit).local && refStatus(commit).remote" class="flex items-center gap-1 text-[#10b981]">
              <ArrowUp class="w-3 h-3" /><ArrowDown class="w-3 h-3" /> Local & Remote
            </span>
            <span v-else-if="refStatus(commit).local" class="flex items-center gap-1 text-[#f59e0b]">
              <ArrowUp class="w-3 h-3" /> Local only
            </span>
            <span v-else-if="refStatus(commit).remote" class="flex items-center gap-1 text-[#06b6d4]">
              <ArrowDown class="w-3 h-3" /> Remote only
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- Changes tab for a selected stash -->
    <div v-show="activeTab === 'changes' && isStash && selectedStash" class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 overflow-y-auto">
        <!-- Stash header -->
        <div v-if="selectedStash" class="px-3 py-2.5 border-b border-[var(--border)] bg-gradient-to-r from-[#f59e0b]/10 to-transparent">
          <div class="flex items-center gap-2 mb-1">
            <Archive class="w-4 h-4 text-[#f59e0b]" />
            <span class="text-[11px] font-semibold text-[var(--foreground)]">stash@{{ '{' + selectedStash.index + '}' }}</span>
          </div>
          <div class="text-[10px] text-[var(--muted-foreground)]">{{ selectedStash.message || 'No message' }}</div>
        </div>

        <!-- Stash files -->
        <div v-if="stashFiles && stashFiles.length > 0">
          <div class="flex items-center gap-2 px-3 py-2 bg-[var(--card)] text-xs text-[var(--foreground)]">
            <span class="font-medium">Stashed files</span>
            <span class="text-[10px] bg-[#f59e0b]/20 text-[#f59e0b] px-1.5 py-0.5 rounded-full">{{ stashFiles.length }}</span>
          </div>
          <div
            v-for="f in stashFiles"
            :key="f.path"
            class="flex items-center gap-2 px-4 py-1.5 hover:bg-[#f59e0b]/5 transition-all cursor-pointer"
          >
            <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
            <span class="text-xs text-[var(--foreground)] truncate flex-1 opacity-90">{{ f.path }}</span>
            <span v-if="f.additions > 0" class="text-[10px] text-[#10b981] font-mono">+{{ f.additions }}</span>
            <span v-if="f.deletions > 0" class="text-[10px] text-[#ef4444] font-mono">-{{ f.deletions }}</span>
          </div>
        </div>
        <div v-else class="px-4 py-12 flex flex-col items-center justify-center">
          <div class="w-12 h-12 rounded-full bg-[var(--card)] flex items-center justify-center mb-3 border border-[var(--border)]">
            <FileText class="w-6 h-6 text-[var(--muted-foreground)] opacity-40" />
          </div>
          <p class="text-xs text-[var(--muted-foreground)]">No stashed changes</p>
        </div>
      </div>

      <!-- Stash actions -->
      <div v-if="selectedStash" class="border-t border-[var(--border)] p-3 bg-[var(--card)]/50 flex-shrink-0 space-y-2">
        <AppButton
          class="w-full bg-[#f59e0b] hover:bg-[#f59e0b]/90 text-white text-xs font-medium h-8"
          @click="emit('stashPop', selectedStash.index)"
        >
          Pop Stash
        </AppButton>
        <div class="flex gap-2">
          <AppButton
            class="flex-1 bg-[var(--secondary)] hover:bg-[var(--secondary)]/80 text-[var(--foreground)] text-xs font-medium h-8"
            @click="emit('stashApply', selectedStash.index)"
          >
            Apply
          </AppButton>
          <AppButton
            class="flex-1 bg-[#ef4444]/10 hover:bg-[#ef4444]/20 text-[#ef4444] text-xs font-medium h-8"
            @click="emit('stashDrop', selectedStash.index)"
          >
            Drop
          </AppButton>
        </div>
      </div>
    </div>

    <!-- Info tab for stash -->
    <div v-show="activeTab === 'info' && isStash && selectedStash" class="flex-1 overflow-y-auto">
      <div v-if="selectedStash" class="p-4 space-y-4">
        <!-- Stash reference -->
        <div>
          <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">Stash Reference</div>
          <div class="flex items-center gap-2">
            <code class="text-xs text-[#f59e0b] bg-[#f59e0b]/10 px-2 py-1 rounded font-mono border border-[#f59e0b]/20">stash@{{ '{' + selectedStash.index + '}' }}</code>
          </div>
        </div>

        <!-- Message -->
        <div v-if="selectedStash.message">
          <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">Message</div>
          <div class="text-sm text-[var(--foreground)] leading-relaxed bg-[var(--input-background)] p-3 rounded border border-[var(--border)]">
            {{ selectedStash.message }}
          </div>
        </div>

        <div class="h-px bg-[var(--border)]" />

        <!-- Branch -->
        <div v-if="selectedStash.branch">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <GitBranch class="w-3 h-3" />
            Original Branch
          </div>
          <span class="px-2 py-0.5 text-[10px] font-medium rounded bg-[var(--primary)]/15 text-[var(--primary)] border border-[var(--primary)]/20">
            {{ selectedStash.branch }}
          </span>
        </div>

        <!-- Timestamp -->
        <div v-if="selectedStash.timestamp">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <Calendar class="w-3 h-3" />
            Created
          </div>
          <div class="text-xs text-[var(--foreground)]">{{ selectedStash.timestamp }}</div>
        </div>

        <!-- Parent SHA -->
        <div v-if="selectedStash.parent_sha">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <Hash class="w-3 h-3" />
            Parent Commit
          </div>
          <div class="flex items-center gap-2">
            <code class="text-xs text-[var(--primary)] bg-[var(--input-background)] px-2 py-1 rounded font-mono">{{ selectedStash.parent_sha.substring(0, 7) }}</code>
            <button
              @click="copyToClipboard(selectedStash.parent_sha)"
              class="p-1 rounded hover:bg-[var(--secondary)] transition-colors flex-shrink-0"
              title="Copy SHA"
            >
              <Copy class="w-3 h-3 text-[var(--muted-foreground)]" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- No selection placeholder -->
    <div v-if="!commit && !isWorkingChanges && !(isStash && selectedStash)" class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <GitCommitIcon class="w-8 h-8 text-[var(--muted-foreground)] opacity-30 mx-auto mb-2" />
        <p class="text-xs text-[var(--muted-foreground)]">Select a commit to view details</p>
      </div>
    </div>
  </div>
</template>

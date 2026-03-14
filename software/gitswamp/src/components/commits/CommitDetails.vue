<script setup lang="ts">
import { ref, computed, watch } from "vue";
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
} from "lucide-vue-next";
import AppButton from "@/components/ui/AppButton.vue";
import GitCommitIcon from "@/components/ui/GitCommitIcon.vue";
import type { CommitInfo, FileStatusInfo, CommitFileInfo } from "@/types";

const props = defineProps<{
  commit: CommitInfo | null;
  stagedFiles: FileStatusInfo[];
  unstagedFiles: FileStatusInfo[];
  commitFiles: CommitFileInfo[];
  isWorkingChanges: boolean;
}>();

const emit = defineEmits<{
  stage: [path: string];
  unstage: [path: string];
  stageAll: [];
  unstageAll: [];
  commit: [message: string];
  discard: [path: string];
  discardAll: [];
}>();

const commitSummary = ref("");
const commitDescription = ref("");

// Tab logic: working changes -> only "changes", commit -> "changes" and "info"
const activeTab = ref<"changes" | "info">("changes");

watch(() => props.commit, (newVal) => {
  if (newVal) {
    activeTab.value = "info";
  } else {
    activeTab.value = "changes";
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
  <div class="w-80 bg-[#0f1620] border-l border-[#8b5cf6]/15 flex flex-col h-full overflow-hidden">
    <!-- Tabs -->
    <div class="border-b border-[#8b5cf6]/15 flex-shrink-0">
      <div class="h-9 flex">
        <button
          @click="activeTab = 'changes'"
          :class="[
            'flex-1 text-xs font-medium tracking-wide transition-colors',
            activeTab === 'changes'
              ? 'text-[#a78bfa] border-b-2 border-[#8b5cf6] bg-[#151d28]'
              : 'text-[#64748b] hover:text-[#94a3b8]',
          ]"
        >
          Changes
        </button>
        <button
          v-if="commit && !isWorkingChanges"
          @click="activeTab = 'info'"
          :class="[
            'flex-1 text-xs font-medium tracking-wide transition-colors',
            activeTab === 'info'
              ? 'text-[#a78bfa] border-b-2 border-[#8b5cf6] bg-[#151d28]'
              : 'text-[#64748b] hover:text-[#94a3b8]',
          ]"
        >
          Info
        </button>
      </div>
    </div>

    <!-- Changes tab for working directory (uncommitted) -->
    <div v-show="activeTab === 'changes' && isWorkingChanges" class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 overflow-y-auto">
        <!-- Unstaged (shown first) -->
        <div v-if="unstagedFiles.length > 0" class="border-b border-[#8b5cf6]/10">
          <div class="flex items-center justify-between px-3 py-2 bg-[#151d28]">
            <button
              @click="expandedUnstaged = !expandedUnstaged"
              class="flex items-center gap-2 text-xs text-[#e2e8f0] hover:text-[#a78bfa] transition-all flex-1"
            >
              <ChevronDown
                :class="['w-3.5 h-3.5 transition-transform', !expandedUnstaged ? '-rotate-90' : '']"
              />
              <span class="font-medium">Unstaged</span>
              <span class="text-[10px] bg-[#1e293b] text-[#64748b] px-1.5 py-0.5 rounded-full">{{ unstagedFiles.length }}</span>
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
                class="text-[10px] text-[#64748b] hover:text-[#e2e8f0] transition-colors px-1.5 py-0.5 rounded hover:bg-[#1e293b]"
              >
                Stage All
              </button>
            </div>
          </div>
          <div v-if="expandedUnstaged">
            <div
              v-for="f in unstagedFiles"
              :key="'u-' + f.path"
              class="flex items-center gap-2 px-4 py-1.5 hover:bg-[#182028] transition-all group cursor-pointer"
            >
              <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
              <span class="text-xs text-[#e0eaf2] truncate flex-1" @click="emit('stage', f.path)">{{ f.path }}</span>
              <button
                @click.stop="emit('discard', f.path)"
                class="p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[#ef4444]/20 transition-all"
                title="Discard changes"
              >
                <Trash2 class="w-3 h-3 text-[#64748b] hover:text-[#ef4444]" />
              </button>
              <Plus class="w-3 h-3 text-[#64748b] opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer" @click="emit('stage', f.path)" />
            </div>
          </div>
        </div>

        <!-- Staged -->
        <div v-if="stagedFiles.length > 0" class="border-b border-[#8b5cf6]/10">
          <div class="flex items-center justify-between px-3 py-2 bg-[#151d28]">
            <button
              @click="expandedStaged = !expandedStaged"
              class="flex items-center gap-2 text-xs text-[#e2e8f0] hover:text-[#a78bfa] transition-all flex-1"
            >
              <ChevronDown
                :class="['w-3.5 h-3.5 transition-transform', !expandedStaged ? '-rotate-90' : '']"
              />
              <span class="font-medium">Staged</span>
              <span class="text-[10px] bg-[#10b981]/20 text-[#10b981] px-1.5 py-0.5 rounded-full">{{ stagedFiles.length }}</span>
            </button>
            <button
              @click="emit('unstageAll')"
              class="text-[10px] text-[#64748b] hover:text-[#e2e8f0] transition-colors px-1.5 py-0.5 rounded hover:bg-[#1e293b]"
            >
              Unstage All
            </button>
          </div>
          <div v-if="expandedStaged">
            <div
              v-for="f in stagedFiles"
              :key="'s-' + f.path"
              class="flex items-center gap-2 px-4 py-1.5 hover:bg-[#182028] transition-all group cursor-pointer"
              @click="emit('unstage', f.path)"
            >
              <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
              <span class="text-xs text-[#e0eaf2] truncate flex-1">{{ f.path }}</span>
              <Minus class="w-3 h-3 text-[#64748b] opacity-0 group-hover:opacity-100 transition-opacity" />
            </div>
          </div>
        </div>

        <div v-if="stagedFiles.length === 0 && unstagedFiles.length === 0" class="px-4 py-12 flex flex-col items-center justify-center">
          <div class="w-12 h-12 rounded-full bg-[#151d28] flex items-center justify-center mb-3">
            <FileText class="w-6 h-6 text-[#334155]" />
          </div>
          <p class="text-xs text-[#475569]">Working tree clean</p>
        </div>
      </div>

      <!-- Commit form -->
      <div class="border-t border-[#8b5cf6]/15 p-3 bg-[#0f1620] flex-shrink-0">
        <input
          v-model="commitSummary"
          type="text"
          placeholder="Commit message..."
          class="w-full px-3 py-2 bg-[#151d28] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] placeholder:text-[#334155] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40 mb-2"
          @keyup.enter="onCommit"
        />
        <textarea
          v-model="commitDescription"
          placeholder="Description (optional)..."
          rows="2"
          class="w-full px-3 py-2 bg-[#151d28] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] placeholder:text-[#334155] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40 resize-none mb-2"
        />
        <AppButton
          class="w-full bg-[#8b5cf6] hover:bg-[#7c3aed] text-white text-xs font-medium h-8"
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
          <div class="flex items-center gap-2 px-3 py-2 bg-[#151d28] text-xs text-[#e2e8f0]">
            <span class="font-medium">Changed files</span>
            <span class="text-[10px] bg-[#8b5cf6]/20 text-[#a78bfa] px-1.5 py-0.5 rounded-full">{{ commitFiles.length }}</span>
          </div>
          <div
            v-for="f in commitFiles"
            :key="f.path"
            class="flex items-center gap-2 px-4 py-1.5 hover:bg-[#182028] transition-all cursor-pointer"
          >
            <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
            <span class="text-xs text-[#e0eaf2] truncate flex-1">{{ f.path }}</span>
            <span v-if="f.additions > 0" class="text-[10px] text-[#10b981] font-mono">+{{ f.additions }}</span>
            <span v-if="f.deletions > 0" class="text-[10px] text-[#ef4444] font-mono">-{{ f.deletions }}</span>
          </div>
        </div>
        <div v-else class="px-4 py-12 flex flex-col items-center justify-center">
          <div class="w-12 h-12 rounded-full bg-[#151d28] flex items-center justify-center mb-3">
            <FileText class="w-6 h-6 text-[#334155]" />
          </div>
          <p class="text-xs text-[#475569]">No file changes</p>
        </div>
      </div>
    </div>

    <!-- Info tab (only for commits) -->
    <div v-show="activeTab === 'info' && commit && !isWorkingChanges" class="flex-1 overflow-y-auto">
      <div v-if="commit" class="p-4 space-y-4">
        <!-- Commit message -->
        <div>
          <div class="text-[10px] text-[#64748b] uppercase tracking-wider mb-1.5">Commit Message</div>
          <div class="text-sm text-[#e2e8f0] leading-relaxed bg-[#151d28] p-3 rounded border border-[#8b5cf6]/10">
            {{ commitSubject(commit.message) }}
          </div>
        </div>

        <!-- Description (body) -->
        <div v-if="commitBody(commit.message)">
          <div class="text-[10px] text-[#64748b] uppercase tracking-wider mb-1.5">Description</div>
          <div class="text-xs text-[#94a3b8] leading-relaxed bg-[#151d28] p-3 rounded border border-[#8b5cf6]/10 whitespace-pre-wrap">
            {{ commitBody(commit.message) }}
          </div>
        </div>

        <div class="h-px bg-[#8b5cf6]/10" />

        <!-- Author -->
        <div class="flex items-center gap-3">
          <div class="w-8 h-8 rounded-full bg-gradient-to-br from-[#8b5cf6] to-[#6d28d9] flex items-center justify-center text-white text-xs font-bold flex-shrink-0">
            {{ commit.author_name.charAt(0).toUpperCase() }}
          </div>
          <div>
            <div class="text-sm text-[#e2e8f0] font-medium">{{ commit.author_name }}</div>
            <div class="text-[10px] text-[#64748b]">{{ commit.author_email }}</div>
          </div>
        </div>

        <div class="h-px bg-[#8b5cf6]/10" />

        <!-- SHA -->
        <div>
          <div class="flex items-center gap-1.5 text-[10px] text-[#64748b] uppercase tracking-wider mb-1.5">
            <Hash class="w-3 h-3" />
            SHA
          </div>
          <div class="flex items-center gap-2">
            <code class="text-xs text-[#a78bfa] bg-[#151d28] px-2 py-1 rounded font-mono break-all">{{ commit.sha }}</code>
            <button
              @click="copyToClipboard(commit.sha)"
              class="p-1 rounded hover:bg-[#1e293b] transition-colors flex-shrink-0"
              title="Copy SHA"
            >
              <Copy class="w-3 h-3 text-[#64748b]" />
            </button>
          </div>
        </div>

        <!-- Date -->
        <div>
          <div class="flex items-center gap-1.5 text-[10px] text-[#64748b] uppercase tracking-wider mb-1.5">
            <Calendar class="w-3 h-3" />
            Date
          </div>
          <div class="text-xs text-[#e2e8f0]">{{ formatDate(commit.timestamp) }}</div>
          <div class="text-[10px] text-[#64748b] mt-0.5">{{ commit.time_ago }}</div>
        </div>

        <!-- Parents -->
        <div v-if="commit.parent_shas.length > 0">
          <div class="flex items-center gap-1.5 text-[10px] text-[#64748b] uppercase tracking-wider mb-1.5">
            <GitCommitIcon class="w-3 h-3" />
            {{ commit.parent_shas.length > 1 ? 'Parents' : 'Parent' }}
          </div>
          <div class="flex flex-wrap gap-1.5">
            <code
              v-for="p in parentRefs(commit)"
              :key="p"
              class="text-[11px] text-[#a78bfa] bg-[#151d28] px-2 py-0.5 rounded font-mono cursor-pointer hover:bg-[#1e293b] transition-colors"
            >{{ p }}</code>
          </div>
        </div>

        <!-- Branches / Refs -->
        <div v-if="branchRefs(commit).length > 0">
          <div class="flex items-center gap-1.5 text-[10px] text-[#64748b] uppercase tracking-wider mb-1.5">
            <GitBranch class="w-3 h-3" />
            Branches
          </div>
          <div class="flex flex-wrap gap-1.5">
            <span
              v-for="r in branchRefs(commit)"
              :key="r"
              class="px-2 py-0.5 text-[10px] font-medium rounded bg-[#8b5cf6]/15 text-[#a78bfa] border border-[#8b5cf6]/20"
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

    <!-- No selection placeholder -->
    <div v-if="!commit && !isWorkingChanges" class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <GitCommitIcon class="w-8 h-8 text-[#334155] mx-auto mb-2" />
        <p class="text-xs text-[#475569]">Select a commit to view details</p>
      </div>
    </div>
  </div>
</template>

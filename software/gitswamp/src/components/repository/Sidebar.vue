<script setup lang="ts">
import { reactive, ref, computed } from "vue";
import {
  Search,
  GitBranch,
  Archive,
  Tag,
  Globe,
  Plus,
  Trash2,
  Github,
} from "lucide-vue-next";
import SidebarSection from "./SidebarSection.vue";
import type { BranchInfo, StashInfo, TagInfo } from "@/types";

const props = defineProps<{
  branches: BranchInfo[];
  remoteBranches: BranchInfo[];
  currentBranch: string;
  stashes: StashInfo[];
  tags: TagInfo[];
  remoteProvider?: 'github' | 'gitlab' | 'bitbucket' | 'azure' | 'unknown';
}>();

const emit = defineEmits<{
  checkout: [branchName: string];
  createBranch: [name: string];
  deleteBranch: [name: string];
  stashPop: [index: number];
  stashApply: [index: number];
  stashDrop: [index: number];
}>();

interface SidebarSections {
  local: boolean;
  remote: boolean;
  stashes: boolean;
  tags: boolean;
}

const expandedSections = reactive<SidebarSections>({
  local: true,
  remote: false,
  stashes: false,
  tags: false,
});

const branchFilter = ref("");
const showNewBranch = ref(false);
const newBranchName = ref("");

function toggleSection(section: keyof SidebarSections) {
  expandedSections[section] = !expandedSections[section];
}

function submitNewBranch() {
  if (!newBranchName.value.trim()) return;
  emit("createBranch", newBranchName.value.trim());
  newBranchName.value = "";
  showNewBranch.value = false;
}

function filteredBranches(list: BranchInfo[]): BranchInfo[] {
  if (!branchFilter.value.trim()) return list;
  const q = branchFilter.value.toLowerCase();
  return list.filter((b) => b.name.toLowerCase().includes(q));
}

const remoteLabel = computed(() => {
  switch (props.remoteProvider) {
    case 'github': return 'GITHUB';
    case 'gitlab': return 'GITLAB';
    case 'bitbucket': return 'BITBUCKET';
    case 'azure': return 'AZURE';
    default: return 'REMOTE';
  }
});

const remoteIcon = computed(() => {
  return props.remoteProvider === 'github' ? Github : Globe;
});
</script>

<template>
  <div class="w-56 bg-[var(--sidebar)] border-r border-[var(--sidebar-border)] flex flex-col h-full flex-shrink-0">
    <div class="p-3 border-b border-[var(--sidebar-border)]">
      <div class="flex items-center gap-2.5 mb-2.5 px-2 py-2 rounded-lg bg-[var(--sidebar-primary)]/20 border-2 border-[var(--sidebar-primary)]/50 shadow-md shadow-[var(--sidebar-primary)]/10">
        <GitBranch class="w-5 h-5 text-[var(--sidebar-primary)] flex-shrink-0 drop-shadow-[0_0_6px_var(--sidebar-primary)]" />
        <div class="flex-1 min-w-0">
          <div class="text-[9px] text-[var(--sidebar-primary)]/70 uppercase tracking-wider font-semibold">Current Branch</div>
          <div class="text-sm text-[var(--sidebar-primary)] font-extrabold truncate drop-shadow-[0_0_8px_var(--sidebar-primary)]" :title="currentBranch">{{ currentBranch }}</div>
        </div>
      </div>
      <div class="relative">
        <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3 h-3 text-[var(--muted-foreground)]" />
        <input
          v-model="branchFilter"
          placeholder="Filter branches..."
          class="w-full h-7 pl-7 pr-2 bg-[var(--sidebar-accent)] border border-[var(--sidebar-border)] rounded text-[var(--sidebar-foreground)] placeholder:text-[var(--muted-foreground)]/50 text-[11px] focus:outline-none focus:ring-1 focus:ring-[var(--sidebar-ring)]/40"
        />
      </div>
    </div>

    <div class="flex-1 overflow-y-auto">
      <SidebarSection
        label="LOCAL"
        :count="branches.length"
        :icon="GitBranch"
        :expanded="expandedSections.local"
        @toggle="toggleSection('local')"
      >
        <div class="px-4 pb-1">
          <button
            v-if="!showNewBranch"
            @click="showNewBranch = true"
            class="w-full flex items-center gap-1.5 px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:text-[var(--sidebar-primary)] hover:bg-[var(--sidebar-accent)] rounded transition-all"
          >
            <Plus class="w-3 h-3" />
            New branch
          </button>
          <div v-else class="flex items-center gap-1">
            <input
              v-model="newBranchName"
              @keyup.enter="submitNewBranch"
              @keyup.escape="showNewBranch = false"
              placeholder="branch-name"
              class="flex-1 h-6 px-2 bg-[var(--sidebar-accent)] border border-[var(--sidebar-border)] rounded text-[10px] text-[var(--sidebar-foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--sidebar-ring)]/40"
              autofocus
            />
            <button @click="submitNewBranch" class="p-1 rounded hover:bg-[var(--sidebar-primary)]/20 text-[var(--sidebar-primary)]">
              <Plus class="w-3 h-3" />
            </button>
          </div>
        </div>

        <button
          v-for="branch in filteredBranches(branches)"
          :key="branch.name"
          @click="emit('checkout', branch.name)"
          :class="[
            'w-full flex items-center gap-2 px-4 py-1 pl-8 text-[11px] transition-all group',
            branch.is_head
              ? 'text-[var(--sidebar-primary)] bg-[var(--sidebar-primary)]/10 font-medium'
              : 'text-[var(--muted-foreground)] hover:text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)]',
          ]"
        >
          <span class="truncate flex-1 text-left">{{ branch.name }}</span>
          <span v-if="branch.upstream" class="flex items-center gap-0.5 flex-shrink-0">
            <span v-if="branch.ahead > 0" class="text-[9px] text-[#10b981]">↑{{ branch.ahead }}</span>
            <span v-if="branch.behind > 0" class="text-[9px] text-[#f59e0b]">↓{{ branch.behind }}</span>
            <span v-if="branch.ahead === 0 && branch.behind === 0" class="text-[9px] text-[var(--muted-foreground)]">✓</span>
          </span>
          <span v-else class="text-[9px] text-[var(--muted-foreground)] italic flex-shrink-0">local</span>
          <button
            v-if="!branch.is_head"
            @click.stop="emit('deleteBranch', branch.name)"
            class="p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[#ef4444]/20 transition-all flex-shrink-0"
            title="Delete branch"
          >
            <Trash2 class="w-2.5 h-2.5 text-[var(--muted-foreground)] hover:text-[#ef4444]" />
          </button>
        </button>
      </SidebarSection>

      <SidebarSection
        :label="remoteLabel"
        :count="remoteBranches.length"
        :icon="remoteIcon"
        :expanded="expandedSections.remote"
        @toggle="toggleSection('remote')"
      >
        <button
          v-for="branch in filteredBranches(remoteBranches)"
          :key="branch.name"
          @click="emit('checkout', branch.name)"
          class="w-full flex items-center gap-2 px-4 py-1 pl-10 text-[11px] text-[var(--muted-foreground)] hover:text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)] transition-all"
        >
          <span class="truncate">{{ branch.name }}</span>
        </button>
      </SidebarSection>

      <SidebarSection
        label="STASHES"
        :count="stashes.length"
        :icon="Archive"
        :expanded="expandedSections.stashes"
        @toggle="toggleSection('stashes')"
      >
        <div v-if="stashes.length === 0" class="px-4 py-2 text-[10px] text-[var(--muted-foreground)] italic">
          No stashes
        </div>
        <div
          v-for="stash in stashes"
          :key="stash.index"
          class="px-4 py-1.5 pl-8 hover:bg-[var(--sidebar-accent)] transition-all group"
        >
          <div class="text-[11px] text-[var(--sidebar-foreground)] truncate">{{ stash.message || ('stash@{' + stash.index + '}') }}</div>
          <div class="text-[9px] text-[var(--muted-foreground)]">on {{ stash.branch }}</div>
          <div class="flex items-center gap-1 mt-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              @click="emit('stashPop', stash.index)"
              class="text-[9px] text-[var(--sidebar-primary)] hover:opacity-80 px-1 py-0.5 rounded hover:bg-[var(--sidebar-primary)]/10"
            >Pop</button>
            <button
              @click="emit('stashApply', stash.index)"
              class="text-[9px] text-[#06b6d4] hover:text-[#22d3ee] px-1 py-0.5 rounded hover:bg-[#06b6d4]/10"
            >Apply</button>
            <button
              @click="emit('stashDrop', stash.index)"
              class="text-[9px] text-[#ef4444] hover:text-[#f87171] px-1 py-0.5 rounded hover:bg-[#ef4444]/10"
            >Drop</button>
          </div>
        </div>
      </SidebarSection>

      <SidebarSection
        label="TAGS"
        :count="tags.length"
        :icon="Tag"
        :expanded="expandedSections.tags"
        @toggle="toggleSection('tags')"
      >
        <div v-if="tags.length === 0" class="px-4 py-2 text-[10px] text-[var(--muted-foreground)] italic">
          No tags
        </div>
        <div
          v-for="tag in tags"
          :key="tag.name"
          class="px-4 py-1 pl-8 text-[11px] text-[var(--muted-foreground)] hover:text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)] transition-all"
        >
          <span class="truncate">{{ tag.name }}</span>
        </div>
      </SidebarSection>
    </div>
  </div>
</template>


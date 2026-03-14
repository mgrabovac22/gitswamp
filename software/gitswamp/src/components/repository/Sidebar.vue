<script setup lang="ts">
import { reactive, ref } from "vue";
import {
  Folder,
  Search,
  GitBranch,
  Archive,
  Tag,
  Globe,
  Plus,
  Trash2,
} from "lucide-vue-next";
import AppInput from "@/components/ui/AppInput.vue";
import SidebarSection from "./SidebarSection.vue";
import type { BranchInfo, StashInfo, TagInfo } from "@/types";

const props = defineProps<{
  branches: BranchInfo[];
  remoteBranches: BranchInfo[];
  currentBranch: string;
  stashes: StashInfo[];
  tags: TagInfo[];
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
</script>

<template>
  <div class="w-56 bg-[#0d1017] border-r border-[#8b5cf6]/10 flex flex-col h-full flex-shrink-0">
    <div class="p-3 border-b border-[#8b5cf6]/10">
      <div class="flex items-center gap-2 mb-2">
        <Folder class="w-3.5 h-3.5 text-[#a78bfa]" />
        <div>
          <div class="text-[10px] text-[#64748b]">Branch</div>
          <div class="text-xs text-[#e2e8f0] font-medium">{{ currentBranch }}</div>
        </div>
      </div>
      <div class="relative">
        <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3 h-3 text-[#64748b]" />
        <input
          v-model="branchFilter"
          placeholder="Filter branches..."
          class="w-full h-7 pl-7 pr-2 bg-[#151921] border border-[#8b5cf6]/15 rounded text-[#e2e8f0] placeholder:text-[#64748b]/50 text-[11px] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40"
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
        <!-- Create branch button -->
        <div class="px-4 pb-1">
          <button
            v-if="!showNewBranch"
            @click="showNewBranch = true"
            class="w-full flex items-center gap-1.5 px-2 py-1 text-[10px] text-[#64748b] hover:text-[#a78bfa] hover:bg-[#151921] rounded transition-all"
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
              class="flex-1 h-6 px-2 bg-[#151921] border border-[#8b5cf6]/20 rounded text-[10px] text-[#e2e8f0] placeholder:text-[#475569] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40"
              autofocus
            />
            <button @click="submitNewBranch" class="p-1 rounded hover:bg-[#8b5cf6]/20 text-[#a78bfa]">
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
              ? 'text-[#a78bfa] bg-[#8b5cf6]/10 font-medium'
              : 'text-[#64748b] hover:text-[#e2e8f0] hover:bg-[#151921]',
          ]"
        >
          <span class="truncate flex-1 text-left">{{ branch.name }}</span>
          <span v-if="branch.upstream" class="flex items-center gap-0.5 flex-shrink-0">
            <span v-if="branch.ahead > 0" class="text-[9px] text-[#10b981]">↑{{ branch.ahead }}</span>
            <span v-if="branch.behind > 0" class="text-[9px] text-[#f59e0b]">↓{{ branch.behind }}</span>
            <span v-if="branch.ahead === 0 && branch.behind === 0" class="text-[9px] text-[#475569]">✓</span>
          </span>
          <span v-else class="text-[9px] text-[#64748b] italic flex-shrink-0">local</span>
          <button
            v-if="!branch.is_head"
            @click.stop="emit('deleteBranch', branch.name)"
            class="p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[#ef4444]/20 transition-all flex-shrink-0"
            title="Delete branch"
          >
            <Trash2 class="w-2.5 h-2.5 text-[#64748b] hover:text-[#ef4444]" />
          </button>
        </button>
      </SidebarSection>

      <SidebarSection
        label="REMOTE"
        :count="remoteBranches.length"
        :icon="Globe"
        :expanded="expandedSections.remote"
        @toggle="toggleSection('remote')"
      >
        <button
          v-for="branch in filteredBranches(remoteBranches)"
          :key="branch.name"
          @click="emit('checkout', branch.name)"
          class="w-full flex items-center gap-2 px-4 py-1 pl-10 text-[11px] text-[#64748b] hover:text-[#e2e8f0] hover:bg-[#151921] transition-all"
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
        <div v-if="stashes.length === 0" class="px-4 py-2 text-[10px] text-[#475569] italic">
          No stashes
        </div>
        <div
          v-for="stash in stashes"
          :key="stash.index"
          class="px-4 py-1.5 pl-8 hover:bg-[#151921] transition-all group"
        >
          <div class="text-[11px] text-[#e2e8f0] truncate">{{ stash.message || ('stash@{' + stash.index + '}') }}</div>
          <div class="text-[9px] text-[#64748b]">on {{ stash.branch }}</div>
          <div class="flex items-center gap-1 mt-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              @click="emit('stashPop', stash.index)"
              class="text-[9px] text-[#a78bfa] hover:text-[#c4b5fd] px-1 py-0.5 rounded hover:bg-[#8b5cf6]/10"
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
        <div v-if="tags.length === 0" class="px-4 py-2 text-[10px] text-[#475569] italic">
          No tags
        </div>
        <div
          v-for="tag in tags"
          :key="tag.name"
          class="px-4 py-1 pl-8 text-[11px] text-[#64748b] hover:text-[#e2e8f0] hover:bg-[#151921] transition-all"
        >
          <span class="truncate">{{ tag.name }}</span>
        </div>
      </SidebarSection>
    </div>
  </div>
</template>

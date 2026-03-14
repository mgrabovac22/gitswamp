<script setup lang="ts">
import { Folder, Plus, X, Home } from "lucide-vue-next";
import type { RepoInfo } from "@/types";

const props = defineProps<{
  tabs: { id: string; repo: RepoInfo | null; label: string }[];
  activeTabId: string;
}>();

const emit = defineEmits<{
  selectTab: [id: string];
  closeTab: [id: string];
  newTab: [];
}>();
</script>

<template>
  <div class="h-9 bg-[#0d1017] flex items-end px-1 border-b border-[#8b5cf6]/10 gap-px overflow-x-auto flex-shrink-0">
    <button
      v-for="tab in tabs"
      :key="tab.id"
      @click="emit('selectTab', tab.id)"
      :class="[
        'h-8 px-3 rounded-t-md flex items-center gap-2 text-xs font-medium transition-colors relative group min-w-0 max-w-48 flex-shrink-0',
        activeTabId === tab.id
          ? 'bg-[#1c2130] text-[#e2e8f0] border-t border-x border-[#8b5cf6]/20'
          : 'bg-[#0d1017] text-[#64748b] hover:text-[#94a3b8] hover:bg-[#151921]',
      ]"
    >
      <Home v-if="!tab.repo" class="w-3 h-3 text-[#a78bfa] flex-shrink-0" />
      <Folder v-else class="w-3 h-3 text-[#a78bfa] flex-shrink-0" />
      <span class="truncate">{{ tab.label }}</span>
      <button
        v-if="tabs.length > 1"
        @click.stop="emit('closeTab', tab.id)"
        class="ml-1 p-0.5 rounded hover:bg-[#ef4444]/20 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0"
      >
        <X class="w-3 h-3" />
      </button>
    </button>

    <button
      @click="emit('newTab')"
      class="h-8 w-8 flex items-center justify-center rounded-t-md text-[#64748b] hover:text-[#a78bfa] hover:bg-[#151921] transition-colors flex-shrink-0"
      title="New Tab"
    >
      <Plus class="w-3.5 h-3.5" />
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { Folder, Plus, X } from "lucide-vue-next";
import { repositories } from "@/data/repositories";

const activeRepo = ref("1");

const activeRepoPath = computed(() =>
  repositories.find((r) => r.id === activeRepo.value)?.path
);
</script>

<template>
  <div class="h-10 bg-[#384d60] flex items-center justify-between px-2 border-b border-[#263342]/50 shadow-lg">
    <div class="flex items-center gap-1 flex-1 overflow-x-auto">
      <button
        v-for="repo in repositories"
        :key="repo.id"
        @click="activeRepo = repo.id"
        :class="[
          'h-8 px-3 rounded-t transition-all flex items-center gap-2 group relative',
          activeRepo === repo.id
            ? 'bg-[#263342] text-[#e8f0f8] shadow-md'
            : 'text-[#e8f0f8]/70 hover:bg-[#263342]/50 hover:text-[#e8f0f8]',
        ]"
      >
        <Folder class="w-3.5 h-3.5" />
        <span class="text-sm font-medium">{{ repo.name }}</span>
        <div
          v-if="activeRepo === repo.id"
          class="ml-1 opacity-0 group-hover:opacity-100 transition-opacity"
        >
          <X class="w-3.5 h-3.5 hover:text-[#ff6b6b]" />
        </div>
      </button>

      <button class="h-8 w-8 flex items-center justify-center rounded text-[#e8f0f8]/70 hover:bg-[#263342]/50 hover:text-[#e8f0f8] transition-all">
        <Plus class="w-4 h-4" />
      </button>
    </div>

    <div class="flex items-center gap-2 ml-4">
      <div class="text-xs text-[#e8f0f8]/50 font-mono">
        {{ activeRepoPath }}
      </div>
    </div>
  </div>
</template>

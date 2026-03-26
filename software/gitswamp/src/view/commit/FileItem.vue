<script setup lang="ts">
import { FileText, Plus, Minus, Trash2 } from "lucide-vue-next";

defineProps<{
  name: string;
  additions: number;
  deletions: number;
}>();
</script>

<template>
  <div class="px-4 py-2 hover:bg-[#182028] transition-all group cursor-pointer">
    <div class="flex items-center justify-between mb-1.5">
      <div class="flex items-center gap-2 flex-1 min-w-0">
        <FileText class="w-3.5 h-3.5 text-[#4ecdc4] flex-shrink-0" />
        <span class="text-xs text-[#e0eaf2] truncate group-hover:text-[#4ecdc4] transition-colors">
          {{ name }}
        </span>
      </div>
      <button
        class="opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-[#1e2c38]"
        title="Discard changes"
      >
        <Trash2 class="w-3 h-3 text-[#8a9fb2] hover:text-[#ff6b6b]" />
      </button>
    </div>
    <div class="flex items-center gap-3 text-xs pl-5">
      <div class="flex items-center gap-1 text-[#4ecdc4] font-medium">
        <Plus class="w-3 h-3" />
        <span>{{ additions }}</span>
      </div>
      <div class="flex items-center gap-1 text-[#ff6b6b] font-medium">
        <Minus class="w-3 h-3" />
        <span>{{ deletions }}</span>
      </div>
    </div>
  </div>
</template>

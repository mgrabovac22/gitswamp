<script setup lang="ts">
import { Minimize2, Maximize2, X } from "lucide-vue-next";
</script>

<template>
  <div class="h-8 bg-gradient-to-r from-[#263342] to-[#2e3e50] flex items-center justify-between px-3 border-b border-[#1a2634]/50 select-none">
    <div class="flex items-center gap-2">
      <div class="text-sm font-semibold text-[#e8f0f8] tracking-wide">
        🐊 GitSwamp
      </div>
    </div>

    <div class="flex items-center gap-1">
      <button
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[#1e2e3e]/70 transition-all group"
        title="Minimize"
      >
        <Minimize2 class="w-3.5 h-3.5 text-[#e8f0f8]/80 group-hover:text-[#e8f0f8]" />
      </button>
      <button
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[#1e2e3e]/70 transition-all group"
        title="Maximize"
      >
        <Maximize2 class="w-3.5 h-3.5 text-[#e8f0f8]/80 group-hover:text-[#e8f0f8]" />
      </button>
      <button
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[#e55555] transition-all group"
        title="Close"
      >
        <X class="w-4 h-4 text-[#e8f0f8]/80 group-hover:text-white" />
      </button>
    </div>
  </div>
</template>

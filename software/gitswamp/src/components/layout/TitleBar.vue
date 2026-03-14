<script setup lang="ts">
import { Minimize2, Maximize2, X } from "lucide-vue-next";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

function minimize() { appWindow.minimize(); }
function toggleMaximize() { appWindow.toggleMaximize(); }
function close() { appWindow.close(); }
</script>

<template>
  <div class="h-8 bg-gradient-to-r from-[#1c2130] to-[#252b3d] flex items-center justify-between px-3 border-b border-[#8b5cf6]/10 select-none" data-tauri-drag-region>
    <div class="flex items-center gap-2">
      <div class="text-sm font-semibold text-[#e2e8f0] tracking-wide">
        🐊 GitSwamp
      </div>
    </div>

    <div class="flex items-center gap-1">
      <button
        @click="minimize"
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[#252b3d]/70 transition-all group"
        title="Minimize"
      >
        <Minimize2 class="w-3.5 h-3.5 text-[#e2e8f0]/80 group-hover:text-[#e2e8f0]" />
      </button>
      <button
        @click="toggleMaximize"
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[#252b3d]/70 transition-all group"
        title="Maximize"
      >
        <Maximize2 class="w-3.5 h-3.5 text-[#e2e8f0]/80 group-hover:text-[#e2e8f0]" />
      </button>
      <button
        @click="close"
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[#ef4444] transition-all group"
        title="Close"
      >
        <X class="w-4 h-4 text-[#e2e8f0]/80 group-hover:text-white" />
      </button>
    </div>
  </div>
</template>

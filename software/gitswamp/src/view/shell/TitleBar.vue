<script setup lang="ts">
import { Minimize2, Maximize2, X } from "lucide-vue-next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import titleLogo from "@/assets/logo_croc.png";

const appWindow = getCurrentWindow();

function minimize() { appWindow.minimize(); }
function toggleMaximize() { appWindow.toggleMaximize(); }
function close() { appWindow.close(); }
</script>

<template>
  <div class="h-8 bg-[var(--header-bg)] flex items-center justify-between px-2 border-b border-[var(--border)] select-none [app-region:drag]" data-tauri-drag-region>
    <div class="flex items-center gap-2">
      <div class="text-sm font-semibold text-[var(--header-fg)] tracking-wide flex items-center gap-1.5">
        <div class="w-6 h-6 flex items-center justify-center">
          <img :src="titleLogo" alt="GitSwamp" class="w-full h-full object-contain" />
        </div>
        <span>GitSwamp</span>
      </div>
    </div>

    <div class="flex items-center gap-1">
      <button
        @click="minimize"
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[var(--header-hover)] transition-all group [app-region:no-drag]"
        title="Minimize"
      >
        <Minimize2 class="w-3.5 h-3.5 text-[var(--header-fg)]/80 group-hover:text-[var(--header-fg)]" />
      </button>
      <button
        @click="toggleMaximize"
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[var(--header-hover)] transition-all group [app-region:no-drag]"
        title="Maximize"
      >
        <Maximize2 class="w-3.5 h-3.5 text-[var(--header-fg)]/80 group-hover:text-[var(--header-fg)]" />
      </button>
      <button
        @click="close"
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[#ef4444] transition-all group [app-region:no-drag]"
        title="Close"
      >
        <X class="w-4 h-4 text-[var(--header-fg)]/80 group-hover:text-white" />
      </button>
    </div>
  </div>
</template>

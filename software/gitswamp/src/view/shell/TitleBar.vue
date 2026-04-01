<script setup lang="ts">
import { Minimize2, Maximize2, X } from "lucide-vue-next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMounted, onUnmounted, ref } from "vue";
import titleLogo from "@/assets/logo_croc.png";

const appWindow = getCurrentWindow();

const isLight = ref(document.documentElement.classList.contains("light"));
const themeObserver = new MutationObserver(() => {
  isLight.value = document.documentElement.classList.contains("light");
});

function minimize() { appWindow.minimize(); }
function toggleMaximize() { appWindow.toggleMaximize(); }
function close() { appWindow.close(); }

async function onTitleBarMouseDown(event: MouseEvent) {
  if (event.button !== 0) return;

  const target = event.target as HTMLElement | null;
  if (target?.closest("[data-no-window-drag='true']")) {
    return;
  }

  try {
    await appWindow.startDragging();
  } catch (error) {
    console.warn("Title bar drag failed", error);
  }
}

onMounted(() => {
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["class"],
  });
});

onUnmounted(() => {
  themeObserver.disconnect();
});
</script>

<template>
  <div
    class="relative z-[50] h-8 bg-[var(--header-bg)] flex items-center justify-between px-2 border-b border-[var(--border)] select-none [app-region:drag] [-webkit-app-region:drag]"
    data-tauri-drag-region
    @mousedown="onTitleBarMouseDown"
  >
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
        data-no-window-drag="true"
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[var(--header-hover)] transition-all group [app-region:no-drag] [-webkit-app-region:no-drag]"
        title="Minimize"
      >
        <Minimize2 class="w-3.5 h-3.5 text-[var(--header-fg)]/80 group-hover:text-[var(--header-fg)]" />
      </button>
      <button
        @click="toggleMaximize"
        data-no-window-drag="true"
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[var(--header-hover)] transition-all group [app-region:no-drag] [-webkit-app-region:no-drag]"
        title="Maximize"
      >
        <Maximize2 class="w-3.5 h-3.5 text-[var(--header-fg)]/80 group-hover:text-[var(--header-fg)]" />
      </button>
      <button
        @click="close"
        data-no-window-drag="true"
        class="w-8 h-6 flex items-center justify-center rounded hover:bg-[#ef4444] transition-all group [app-region:no-drag] [-webkit-app-region:no-drag]"
        title="Close"
      >
        <X class="w-4 h-4 text-[var(--header-fg)]/80 group-hover:text-white" />
      </button>
    </div>
  </div>
</template>

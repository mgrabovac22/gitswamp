<script setup lang="ts">
import { Minus, Square, X } from "lucide-vue-next";
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
    class="relative z-[50] h-8 bg-[var(--header-bg)] flex items-center justify-between pl-2 pr-0 border-b border-[var(--border)] select-none [app-region:drag] [-webkit-app-region:drag]"
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

    <div class="flex h-full items-stretch [app-region:no-drag] [-webkit-app-region:no-drag]" data-no-window-drag="true">
      <button
        type="button"
        @click="minimize"
        data-no-window-drag="true"
        class="flex h-full w-11 items-center justify-center text-[var(--header-fg)]/80 transition-colors hover:bg-[var(--header-hover)] hover:text-[var(--header-fg)] [app-region:no-drag] [-webkit-app-region:no-drag]"
        title="Minimize"
      >
        <Minus class="h-4 w-4" />
      </button>
      <button
        type="button"
        @click="toggleMaximize"
        data-no-window-drag="true"
        class="flex h-full w-11 items-center justify-center text-[var(--header-fg)]/80 transition-colors hover:bg-[var(--header-hover)] hover:text-[var(--header-fg)] [app-region:no-drag] [-webkit-app-region:no-drag]"
        title="Maximize"
      >
        <Square class="h-3.5 w-3.5" />
      </button>
      <button
        type="button"
        @click="close"
        data-no-window-drag="true"
        class="flex h-full w-11 items-center justify-center text-[var(--header-fg)]/80 transition-colors hover:bg-[#ef4444] hover:text-white [app-region:no-drag] [-webkit-app-region:no-drag]"
        title="Close"
      >
        <X class="h-4 w-4" />
      </button>
    </div>
  </div>
</template>

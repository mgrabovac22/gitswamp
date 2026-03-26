<script setup lang="ts">
import {
  Upload,
  RefreshCw,
  GitBranch,
  Archive,
  Terminal,
  Settings,
  Loader2,
  Download,
} from "lucide-vue-next";
import AppButton from "@/shared/ui/AppButton.vue";

defineProps<{
  loading: boolean;
}>();

const emit = defineEmits<{
  pull: [];
  push: [];
  fetch: [];
  branch: [];
  stash: [];
  terminal: [];
  settings: [];
}>();
</script>

<template>
  <div class="h-12 bg-gradient-to-b from-[var(--header-bg)] to-[var(--secondary)] border-b border-[var(--border)] flex items-center justify-between px-4 shadow-lg flex-shrink-0">
    <div class="flex items-center gap-3">
      <div class="flex items-center gap-1.5">
        <!-- Animated Crocodile Icon (SVG) -->
        <svg class="w-6 h-6 text-[var(--primary)]" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" style="animation: croc-bob 3s ease-in-out infinite">
          <!-- Crocodile body -->
          <path d="M3 12 Q5 10, 8 10 L16 10 Q19 10, 20 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          <!-- Crocodile snout -->
          <path d="M8 10 L7 12 L9 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
          <!-- Eyes -->
          <circle cx="10" cy="8" r="1.2" fill="currentColor" />
          <circle cx="16" cy="8" r="1.2" fill="currentColor" />
          <!-- Back spikes -->
          <line x1="11" y1="9" x2="11" y2="7.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          <line x1="14" y1="9" x2="14" y2="7.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          <line x1="17" y1="9" x2="17" y2="7.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          <!-- Tail -->
          <path d="M20 12 Q21 12, 22 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
        <!-- Animated GitSwamp Text -->
        <h1 class="text-[var(--foreground)] font-bold text-sm tracking-tight overflow-hidden whitespace-nowrap">
          <span v-for="(letter, idx) in 'GitSwamp'" :key="idx" class="inline-block" :style="{ animation: `fade-in-letter 0.5s ease-out ${idx * 0.08}s both` }">{{ letter }}</span>
        </h1>
      </div>

      <div class="w-px h-6 bg-[var(--foreground)]/10" />

      <div class="flex items-center gap-1">
        <AppButton
          variant="ghost"
          size="sm"
          class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
          :disabled="loading"
          @click="emit('pull')"
        >
          <Loader2 v-if="loading" class="w-3.5 h-3.5 animate-spin" />
          <Download v-else class="w-3.5 h-3.5" />
          Pull
        </AppButton>
        <AppButton
          variant="ghost"
          size="sm"
          class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
          :disabled="loading"
          @click="emit('push')"
        >
          <Upload class="w-3.5 h-3.5" />
          Push
        </AppButton>
        <AppButton
          variant="ghost"
          size="sm"
          class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
          :disabled="loading"
          @click="emit('fetch')"
        >
          <RefreshCw class="w-3.5 h-3.5" />
          Fetch
        </AppButton>
      </div>
    </div>

    <div class="flex items-center gap-1">
      <AppButton
        variant="ghost"
        size="sm"
        class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
        @click="emit('branch')"
      >
        <GitBranch class="w-3.5 h-3.5" />
        Branch
      </AppButton>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
        @click="emit('stash')"
      >
        <Archive class="w-3.5 h-3.5" />
        Stash
      </AppButton>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
        @click="emit('terminal')"
      >
        <Terminal class="w-3.5 h-3.5" />
        Terminal
      </AppButton>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
        @click="emit('settings')"
      >
        <Settings class="w-3.5 h-3.5" />
      </AppButton>
    </div>
  </div>
</template>

<style scoped>
@keyframes fade-in-letter {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes croc-bob {
  0%,
  100% {
    transform: translateY(0px);
  }
  50% {
    transform: translateY(-3px);
  }
}
</style>

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
import headerIcon from "@/assets/logo_git_croc.gif";
import headerTextLogo from "@/assets/logo_dark.png";

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
  <div class="h-12 bg-gradient-to-b from-[var(--header-bg)] to-[var(--secondary)] border-b border-[var(--border)] flex items-center justify-between px-2 shadow-lg flex-shrink-0">
    <div class="flex items-center gap-2">
      <div class="flex items-center gap-1.5">
        <div class="w-12 h-8 flex items-center justify-center">
          <img :src="headerIcon" alt="GitSwamp icon" class="w-full h-full object-contain" />
        </div>
        <h1 class="text-[var(--foreground)] font-bold text-sm tracking-tight overflow-hidden whitespace-nowrap">
          <div class="h-6 inline-flex items-center">
            <img :src="headerTextLogo" alt="GitSwamp" class="h-6 w-auto object-contain" style="transform: translateY(4px);" />
          </div>
          <span class="sr-only">GitSwamp</span>
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
</style>

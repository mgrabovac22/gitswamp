<script setup lang="ts">
import {
  Upload,
  RefreshCw,
  GitBranch,
  Archive,
  Terminal,
  Settings,
  Loader2,
} from "lucide-vue-next";
import AppButton from "@/components/ui/AppButton.vue";

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
        <div class="w-7 h-7 bg-gradient-to-br from-[var(--primary)] to-[var(--accent)] rounded-lg flex items-center justify-center shadow-md">
          <span class="text-white text-[10px] font-bold">GS</span>
        </div>
        <h1 class="text-[var(--foreground)] font-bold text-sm tracking-tight">GitSwamp</h1>
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

<script setup lang="ts">
import { ChevronDown, ChevronRight } from "lucide-vue-next";
import type { Component } from "vue";

defineProps<{
  label: string;
  count: number | string;
  icon: Component;
  expanded: boolean;
}>();

defineEmits<{
  toggle: [];
}>();
</script>

<template>
  <div class="border-b border-[var(--sidebar-border)]">
    <button
      @click="$emit('toggle')"
      class="w-full flex items-center gap-2 px-4 py-2.5 text-sm text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)] transition-all"
    >
      <ChevronDown v-if="expanded" class="w-4 h-4" />
      <ChevronRight v-else class="w-4 h-4" />
      <component :is="icon" class="w-4 h-4 text-[var(--sidebar-primary)]" />
      <span>{{ label }}</span>
      <span class="ml-auto text-xs bg-[var(--secondary)] px-2 py-0.5 rounded-full text-[var(--muted-foreground)]">{{ count }}</span>
    </button>
    <div v-if="expanded" class="pb-2">
      <slot />
    </div>
  </div>
</template>

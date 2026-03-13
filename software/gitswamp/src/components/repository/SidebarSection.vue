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
  <div class="border-b border-[#4ecdc4]/20">
    <button
      @click="$emit('toggle')"
      class="w-full flex items-center gap-2 px-4 py-2.5 text-sm text-[#e0eaf2] hover:bg-[#182028] transition-all"
    >
      <ChevronDown v-if="expanded" class="w-4 h-4" />
      <ChevronRight v-else class="w-4 h-4" />
      <component :is="icon" class="w-4 h-4 text-[#4ecdc4]" />
      <span>{{ label }}</span>
      <span class="ml-auto text-xs bg-[#1e2c38] px-2 py-0.5 rounded-full">{{ count }}</span>
    </button>
    <div v-if="expanded" class="pb-2">
      <slot />
    </div>
  </div>
</template>

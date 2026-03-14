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
  <div class="border-b border-[#8b5cf6]/10">
    <button
      @click="$emit('toggle')"
      class="w-full flex items-center gap-2 px-4 py-2.5 text-sm text-[#e2e8f0] hover:bg-[#151921] transition-all"
    >
      <ChevronDown v-if="expanded" class="w-4 h-4" />
      <ChevronRight v-else class="w-4 h-4" />
      <component :is="icon" class="w-4 h-4 text-[#a78bfa]" />
      <span>{{ label }}</span>
      <span class="ml-auto text-xs bg-[#1c2130] px-2 py-0.5 rounded-full text-[#64748b]">{{ count }}</span>
    </button>
    <div v-if="expanded" class="pb-2">
      <slot />
    </div>
  </div>
</template>

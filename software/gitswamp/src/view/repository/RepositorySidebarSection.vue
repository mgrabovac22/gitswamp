<script setup lang="ts">
import { ChevronDown, ChevronRight } from "lucide-vue-next";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { Component } from "vue";

const props = defineProps<{
  label: string;
  shortLabel?: string;
  count: number | string;
  icon: Component;
  expanded: boolean;
}>();

defineEmits<{
  toggle: [];
}>();

const labelSlotRef = ref<HTMLElement | null>(null);
const fullLabelMeasureRef = ref<HTMLElement | null>(null);
const useShortLabel = ref(false);
let resizeObserver: ResizeObserver | null = null;

const displayLabel = computed(() => useShortLabel.value && props.shortLabel ? props.shortLabel : props.label);

function updateLabelFit(): void {
  const slot = labelSlotRef.value;
  const measure = fullLabelMeasureRef.value;
  if (!slot || !measure || !props.shortLabel) {
    useShortLabel.value = false;
    return;
  }

  useShortLabel.value = measure.scrollWidth > slot.clientWidth + 1;
}

onMounted(() => {
  if (typeof ResizeObserver !== "undefined" && labelSlotRef.value) {
    resizeObserver = new ResizeObserver(() => updateLabelFit());
    resizeObserver.observe(labelSlotRef.value);
  }

  void nextTick(updateLabelFit);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});

watch(() => [props.label, props.shortLabel, props.count], () => {
  void nextTick(updateLabelFit);
});
</script>

<template>
  <div class="border-b border-[var(--sidebar-border)]">
    <button
      @click="$emit('toggle')"
      class="relative w-full flex items-center justify-start gap-2 px-4 py-2.5 text-left text-sm text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)] transition-all"
    >
      <ChevronDown v-if="expanded" class="w-4 h-4" />
      <ChevronRight v-else class="w-4 h-4" />
      <component :is="icon" class="w-4 h-4 text-[var(--sidebar-primary)]" />
      <span ref="labelSlotRef" class="min-w-0 flex-1 truncate whitespace-nowrap text-left">{{ displayLabel }}</span>
      <span ref="fullLabelMeasureRef" class="pointer-events-none invisible absolute whitespace-nowrap text-sm">{{ label }}</span>
      <span class="ml-auto text-xs bg-[var(--secondary)] px-2 py-0.5 rounded-full text-[var(--muted-foreground)]">{{ count }}</span>
    </button>
    <div v-if="expanded" class="pb-2">
      <slot />
    </div>
  </div>
</template>

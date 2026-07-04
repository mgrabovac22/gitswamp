<script setup lang="ts">
import { X } from "lucide-vue-next";

withDefaults(defineProps<{
  title?: string;
  size?: "sm" | "md";
  subtle?: boolean;
  disabled?: boolean;
}>(), {
  title: "Close",
  size: "md",
  subtle: false,
  disabled: false,
});

const emit = defineEmits<{
  click: [event: MouseEvent];
}>();
</script>

<template>
  <button
    type="button"
    :title="title"
    :aria-label="title"
    :disabled="disabled"
    :class="[
      'inline-flex shrink-0 items-center justify-center rounded-md border transition-colors focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/50',
      size === 'sm' ? 'h-7 w-7' : 'h-8 w-8',
      disabled ? 'cursor-not-allowed opacity-50' : '',
      subtle
        ? 'border-transparent bg-transparent text-[var(--muted-foreground)] hover:border-[var(--border)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]'
        : 'border-[var(--border)] bg-[var(--secondary)] text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--foreground)]',
    ]"
    @click="emit('click', $event)"
  >
    <X :class="size === 'sm' ? 'h-3.5 w-3.5' : 'h-4 w-4'" stroke-width="2.2" />
  </button>
</template>

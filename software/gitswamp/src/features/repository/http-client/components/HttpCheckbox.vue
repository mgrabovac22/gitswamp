<script setup lang="ts">
import { Check } from "lucide-vue-next";

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    disabled?: boolean;
  }>(),
  {
    disabled: false,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
}>();

function toggle(): void {
  if (props.disabled) return;
  emit("update:modelValue", !props.modelValue);
}
</script>

<template>
  <button
    type="button"
    role="checkbox"
    :aria-checked="modelValue"
    :disabled="disabled"
    class="flex h-4 w-4 shrink-0 items-center justify-center rounded border transition-colors"
    :class="
      modelValue
        ? 'border-[var(--primary)] bg-[var(--primary)] text-white'
        : 'border-[var(--border)] bg-[var(--input-background)] hover:border-[var(--primary)]'
    "
    @click.stop="toggle"
  >
    <Check v-if="modelValue" class="h-3 w-3" />
  </button>
</template>

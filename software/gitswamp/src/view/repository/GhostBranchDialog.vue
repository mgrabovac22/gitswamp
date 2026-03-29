<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{
  visible: boolean;
  loading: boolean;
  suggestedName: string;
}>();

const emit = defineEmits<{
  close: [];
  submit: [name: string];
}>();

const branchName = ref("");

watch(
  () => ({ visible: props.visible, suggested: props.suggestedName }),
  ({ visible, suggested }) => {
    if (!visible) return;
    branchName.value = suggested || "";
  },
  { immediate: true },
);

function submit() {
  const trimmed = branchName.value.trim();
  if (!trimmed) return;
  emit("submit", trimmed);
}
</script>

<template>
  <div
    v-if="props.visible"
    class="fixed inset-0 z-[2700] flex items-center justify-center bg-black/60 backdrop-blur-sm"
    @click.self="emit('close')"
  >
    <div class="w-[420px] max-w-[92vw] rounded-lg border border-[var(--border)] bg-[var(--popover)] shadow-2xl p-5">
      <h3 class="text-sm font-semibold text-[var(--foreground)]">Materialize Ghost Branch</h3>
      <p class="mt-1 text-[11px] text-[var(--muted-foreground)]">
        Give your ghost experiment a real branch name.
      </p>

      <input
        v-model="branchName"
        placeholder="feature/awesome-experiment"
        class="mt-4 w-full px-3 py-2 rounded border border-[var(--border)] bg-[var(--input-background)] text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
        @keyup.enter="submit"
        autofocus
      />

      <div class="mt-4 flex justify-end gap-2">
        <button
          class="px-3 py-1.5 rounded text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors"
          :disabled="props.loading"
          @click="emit('close')"
        >
          Cancel
        </button>
        <button
          class="px-3 py-1.5 rounded text-xs text-white bg-[var(--primary)] hover:opacity-90 disabled:opacity-50 transition-colors"
          :disabled="props.loading || !branchName.trim()"
          @click="submit"
        >
          Materialize
        </button>
      </div>
    </div>
  </div>
</template>

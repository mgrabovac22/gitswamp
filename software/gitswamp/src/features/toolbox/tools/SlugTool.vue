<script setup lang="ts">
import { computed, ref } from "vue";
import { Type, Copy, Check } from "lucide-vue-next";
import { slugify } from "../slugTool";

const input = ref("My Blog Post: Hello World!");
const lower = ref(true);
const copied = ref(false);

const output = computed(() => slugify(input.value, lower.value));

async function copy() {
  if (!output.value) return;
  try {
    await navigator.clipboard.writeText(output.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1200);
  } catch {
    copied.value = false;
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-2 overflow-y-auto pr-1">
    <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
      <div class="flex items-center gap-1.5">
        <Type class="h-3.5 w-3.5 text-[var(--primary)]" />
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Slugifier</span>
      </div>
      <textarea v-model="input" rows="3" placeholder="Paste a title or filename…" class="mt-1.5 w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-[11px] text-[var(--foreground)]" />
      <label class="mt-1.5 flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)]">
        <input v-model="lower" type="checkbox" class="h-3 w-3 accent-[var(--primary)]" /> Lowercase
      </label>
    </div>

    <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
      <div class="flex items-center justify-between">
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Slug</span>
        <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copy">
          <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? 'Copied' : 'Copy' }}
        </button>
      </div>
      <p class="mt-1.5 break-all rounded bg-[var(--input-background)] p-2 font-mono text-[12px] text-[var(--foreground)]">{{ output || '—' }}</p>
    </div>
  </div>
</template>

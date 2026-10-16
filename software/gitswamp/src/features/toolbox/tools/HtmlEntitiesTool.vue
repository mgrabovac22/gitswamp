<script setup lang="ts">
import { computed, ref } from "vue";
import { Code, Copy, Check } from "lucide-vue-next";
import { escapeHtml, unescapeHtml } from "../htmlEntitiesTool";

const input = ref(`<a href="x">Tom & Jerry's</a>`);
const mode = ref<"escape" | "unescape">("escape");
const copied = ref(false);

const output = computed(() => (mode.value === "escape" ? escapeHtml(input.value) : unescapeHtml(input.value)));

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
        <Code class="h-3.5 w-3.5 text-[var(--primary)]" />
        <span class="text-[11px] font-semibold text-[var(--foreground)]">HTML entities</span>
        <select v-model="mode" class="ml-auto rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px]">
          <option value="escape">Escape</option>
          <option value="unescape">Unescape</option>
        </select>
      </div>
      <div class="mt-1.5 grid grid-cols-2 gap-2">
        <textarea v-model="input" rows="5" spellcheck="false" class="resize-none rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px]" />
        <textarea :value="output" readonly rows="5" spellcheck="false" class="resize-none rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px]" />
      </div>
      <button class="mt-1.5 flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copy">
        <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? 'Copied' : 'Copy result' }}
      </button>
    </div>
  </div>
</template>

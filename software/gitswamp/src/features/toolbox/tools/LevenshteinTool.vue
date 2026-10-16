<script setup lang="ts">
import { computed, ref } from "vue";
import { Ruler, Copy, Check } from "lucide-vue-next";
import { levenshteinDistance, levenshteinRatio } from "../levenshteinTool";

const a = ref("kitten");
const b = ref("sitting");
const copied = ref(false);

const distance = computed(() => levenshteinDistance(a.value, b.value));
const ratio = computed(() => levenshteinRatio(a.value, b.value));

async function copy() {
  try {
    await navigator.clipboard.writeText(`distance=${distance.value} ratio=${ratio.value}`);
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
        <Ruler class="h-3.5 w-3.5 text-[var(--primary)]" />
        <span class="text-[11px] font-semibold text-[var(--foreground)]">String similarity (Levenshtein)</span>
      </div>
      <div class="mt-1.5 grid grid-cols-2 gap-2">
        <input v-model="a" spellcheck="false" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 font-mono text-[11px]" />
        <input v-model="b" spellcheck="false" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 font-mono text-[11px]" />
      </div>
    </div>

    <div class="grid grid-cols-2 gap-2">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <span class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Edit distance</span>
        <p class="mt-1 font-mono text-2xl text-[var(--foreground)]">{{ distance }}</p>
      </div>
      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <span class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Similarity</span>
        <p class="mt-1 font-mono text-2xl text-[var(--foreground)]">{{ Math.round(ratio * 100) }}%</p>
      </div>
    </div>
    <button class="flex w-fit items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copy">
      <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? 'Copied' : 'Copy result' }}
    </button>
  </div>
</template>

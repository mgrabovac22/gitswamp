<script setup lang="ts">
import { computed, ref } from "vue";
import { Binary, Copy, Check } from "lucide-vue-next";
import { BASES, convertNumber } from "../numberBaseTool";

function baseLabel(b: number) {
  if (b === 2) return "BIN";
  if (b === 8) return "OCT";
  if (b === 10) return "DEC";
  if (b === 16) return "HEX";
  return "BASE" + b;
}

const input = ref("255");
const fromBase = ref(10);
const toBase = ref(16);
const copied = ref(false);

const result = computed(() => convertNumber(input.value, fromBase.value, toBase.value));

async function copyResult() {
  if (!result.value.value) return;
  try {
    await navigator.clipboard.writeText(result.value.value);
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
        <Binary class="h-3.5 w-3.5 text-[var(--primary)]" />
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Number base converter</span>
      </div>
      <div class="mt-1.5 flex flex-wrap items-center gap-2">
        <input v-model="input" spellcheck="false" class="flex-1 min-w-[120px] rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 font-mono text-[11px] text-[var(--foreground)]" />
        <span class="text-[10px] text-[var(--muted-foreground)]">from</span>
        <select v-model.number="fromBase" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px]">
          <option v-for="b in BASES" :key="b" :value="b">{{ b }} ({{ baseLabel(b) }})</option>
        </select>
        <span class="text-[10px] text-[var(--muted-foreground)]">→ to</span>
        <select v-model.number="toBase" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px]">
          <option v-for="b in BASES" :key="b" :value="b">{{ b }} ({{ baseLabel(b) }})</option>
        </select>
      </div>
      <p v-if="result.error" class="mt-1 text-[10px] text-[var(--destructive)]">{{ result.error }}</p>
    </div>

    <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
      <div class="flex items-center justify-between">
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Result</span>
        <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copyResult">
          <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? 'Copied' : 'Copy' }}
        </button>
      </div>
      <p class="mt-1.5 break-all font-mono text-[14px] text-[var(--foreground)]">{{ result.value || '-' }}</p>
    </div>
  </div>
</template>

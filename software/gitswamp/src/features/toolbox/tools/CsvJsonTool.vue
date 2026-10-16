<script setup lang="ts">
import { computed, ref } from "vue";
import { Table, ArrowLeft, Copy, Check } from "lucide-vue-next";
import { csvToJson, jsonToCsv } from "../csvJsonTool";

const input = ref("id,name,age\n1,Alice,30\n2,Bob,25");
const dir = ref<"csv2json" | "json2csv">("csv2json");
const copied = ref(false);

const csvOut = computed(() => csvToJson(input.value));
const jsonOut = computed(() => jsonToCsv(input.value));
const out = computed(() => (dir.value === "csv2json" ? csvOut.value : jsonOut.value));
const output = computed(() => (dir.value === "csv2json" ? csvOut.value.json : jsonOut.value.csv));

function swap() {
  input.value = output.value;
  dir.value = dir.value === "csv2json" ? "json2csv" : "csv2json";
}

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
        <Table class="h-3.5 w-3.5 text-[var(--primary)]" />
        <span class="text-[11px] font-semibold text-[var(--foreground)]">CSV ⇄ JSON</span>
        <select v-model="dir" class="ml-auto rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px]">
          <option value="csv2json">CSV → JSON</option>
          <option value="json2csv">JSON → CSV</option>
        </select>
      </div>
      <div class="mt-1.5 flex flex-wrap items-center gap-1.5">
        <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="swap">
          <ArrowLeft class="h-3 w-3" /> Use output as input
        </button>
        <button class="rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copy">
          <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? 'Copied' : 'Copy result' }}
        </button>
      </div>
      <p v-if="out.error" class="mt-1 text-[10px] text-[var(--destructive)]">{{ out.error }}</p>
    </div>

    <div class="grid flex-1 min-h-0 grid-cols-2 gap-2">
      <div class="flex flex-col">
        <span class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">Input</span>
        <textarea v-model="input" rows="10" spellcheck="false" class="resize-none rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px]" />
      </div>
      <div class="flex flex-col">
        <span class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">Output</span>
        <textarea :value="output" readonly rows="10" spellcheck="false" class="resize-none rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px]" />
      </div>
    </div>
  </div>
</template>

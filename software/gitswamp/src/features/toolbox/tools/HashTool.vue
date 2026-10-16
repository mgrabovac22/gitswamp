<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Hash, Copy, Check, Save } from "lucide-vue-next";
import { hashAll } from "../hashTool";
import {
  addSnippetHistory,
  clearSnippetHistory,
  deleteSnippetHistory,
  formatHistoryTime,
  generateToolboxId,
  getSnippetHistory,
  type SnippetHistoryEntry,
} from "../toolboxStorage";
import ToolHistoryRail from "../ToolHistoryRail.vue";

interface HashResult {
  algorithm: string;
  hex: string;
  error: string | null;
}

const input = ref("");
const results = ref<HashResult[]>([]);
const copied = ref("");
const history = ref<SnippetHistoryEntry[]>(getSnippetHistory("hash"));

const historyItems = computed(() =>
  history.value.map((entry) => ({ id: entry.id, primary: entry.label, mono: true, tertiary: formatHistoryTime(entry.at) })),
);

let timer: ReturnType<typeof setTimeout> | null = null;
async function recompute() {
  results.value = input.value ? await hashAll(input.value) : [];
}

watch(input, () => {
  if (timer) clearTimeout(timer);
  timer = setTimeout(recompute, 180);
});

onMounted(recompute);

async function copyValue(key: string, value: string) {
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    copied.value = key;
    setTimeout(() => {
      if (copied.value === key) copied.value = "";
    }, 1400);
  } catch {
    copied.value = "";
  }
}

function saveCurrent() {
  const text = input.value.trim();
  if (!text) return;
  history.value = addSnippetHistory("hash", {
    id: generateToolboxId("hash"),
    label: text.slice(0, 56).replace(/\s+/g, " "),
    text,
    at: Date.now(),
  });
}

function loadEntry(id: string) {
  const entry = history.value.find((item) => item.id === id);
  if (entry) input.value = entry.text;
}

function removeEntry(id: string) {
  history.value = deleteSnippetHistory("hash", id);
}

function clearHistory() {
  history.value = clearSnippetHistory("hash");
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-2 overflow-y-auto pr-1">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
        <div class="flex items-center gap-1.5">
          <Hash class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Text to hash</span>
        </div>
        <textarea
          v-model="input"
          rows="4"
          spellcheck="false"
          placeholder="Type or paste text — every hash updates instantly…"
          class="mt-1.5 w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
      </div>

      <div v-for="result in results" :key="result.algorithm" class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-semibold text-[var(--foreground)]">{{ result.algorithm }}</span>
          <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copyValue(result.algorithm, result.hex)">
            <component :is="copied === result.algorithm ? Check : Copy" class="h-3 w-3" /> {{ copied === result.algorithm ? "Copied" : "Copy" }}
          </button>
        </div>
        <p v-if="result.error" class="mt-1 text-[10px] text-[var(--destructive)]">{{ result.error }}</p>
        <p v-else class="mt-1.5 break-all rounded bg-[var(--input-background)] p-2 font-mono text-[11px] text-[var(--foreground)]">{{ result.hex }}</p>
      </div>

      <p v-if="!input.trim()" class="px-1 text-[10px] text-[var(--muted-foreground)]">Enter some text to compute MD5, SHA-1, SHA-256, SHA-384 and SHA-512.</p>
    </div>

    <ToolHistoryRail
      title="Recent inputs"
      empty-text="Saved inputs appear here."
      :items="historyItems"
      @select="loadEntry"
      @remove="removeEntry"
      @clear="clearHistory"
    >
      <template #footer>
        <div class="border-t border-[var(--border)] p-1.5">
          <button class="flex w-full items-center justify-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85 disabled:opacity-40" :disabled="!input.trim()" @click="saveCurrent">
            <Save class="h-3 w-3" /> Save current
          </button>
        </div>
      </template>
    </ToolHistoryRail>
  </div>
</template>

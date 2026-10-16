<script setup lang="ts">
import { computed, ref } from "vue";
import { FileCode2, Copy, Check, Save } from "lucide-vue-next";
import { parseJson } from "../jsonTool";
import { jsonToTypescript } from "../jsonToTsTool";
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

const input = ref("");
const rootName = ref("Root");
const copied = ref(false);
const history = ref<SnippetHistoryEntry[]>(getSnippetHistory("schema"));

const parsed = computed(() => parseJson(input.value));
const generated = computed(() => (parsed.value.error || parsed.value.value === undefined ? null : jsonToTypescript(parsed.value.value, rootName.value || "Root")));
const historyItems = computed(() =>
  history.value.map((entry) => ({ id: entry.id, primary: entry.label, mono: true, tertiary: formatHistoryTime(entry.at) })),
);

async function copyCode() {
  const code = generated.value?.code;
  if (!code) return;
  try {
    await navigator.clipboard.writeText(code);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1400);
  } catch {
    copied.value = false;
  }
}

function saveCurrent() {
  const text = input.value.trim();
  if (!text) return;
  history.value = addSnippetHistory("schema", {
    id: generateToolboxId("schema"),
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
  history.value = deleteSnippetHistory("schema", id);
}

function clearHistory() {
  history.value = clearSnippetHistory("schema");
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-2 overflow-y-auto pr-1">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-1.5">
            <FileCode2 class="h-3.5 w-3.5 text-[var(--primary)]" />
            <span class="text-[11px] font-semibold text-[var(--foreground)]">JSON to TypeScript</span>
          </div>
          <label class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)]">
            Root name
            <input v-model="rootName" class="w-28 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)]" />
          </label>
        </div>
        <textarea
          v-model="input"
          rows="6"
          spellcheck="false"
          placeholder='Paste a JSON object or array, e.g. { "id": 1, "name": "Ada", "tags": ["a", "b"] }'
          class="mt-1.5 w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
        <p v-if="parsed.error" class="mt-1 text-[10px] text-[var(--destructive)]">Invalid JSON: {{ parsed.error }}</p>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Generated interfaces</span>
          <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)] disabled:opacity-40" :disabled="!generated?.code" @click="copyCode">
            <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? "Copied" : "Copy" }}
          </button>
        </div>
        <pre v-if="generated?.code" class="mt-1.5 max-h-[360px] overflow-auto rounded bg-[var(--input-background)] p-2 font-mono text-[11px] leading-5 text-[var(--foreground)]">{{ generated.code }}</pre>
        <p v-else class="mt-1.5 text-[10px] text-[var(--muted-foreground)]">Interfaces are generated automatically as you type valid JSON.</p>
      </div>
    </div>

    <ToolHistoryRail
      title="Recent JSON"
      empty-text="Saved JSON appears here."
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

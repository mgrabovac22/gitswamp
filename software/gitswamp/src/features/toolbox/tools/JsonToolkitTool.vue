<script setup lang="ts">
import { computed, ref } from "vue";
import { Braces, Wand2, Minimize2, ArrowDownAZ, Copy, Check, Save, Eraser, History, Trash2, Search } from "lucide-vue-next";
import { formatJson, jsonStats, minifyJson, parseJson, queryJsonPath, sortJsonKeys } from "../jsonTool";
import {
  addSnippetHistory,
  clearSnippetHistory,
  deleteSnippetHistory,
  formatHistoryTime,
  generateToolboxId,
  getSnippetHistory,
  type SnippetHistoryEntry,
} from "../toolboxStorage";

const input = ref("");
const output = ref("");
const queryPath = ref("");
const copied = ref(false);
const history = ref<SnippetHistoryEntry[]>(getSnippetHistory("json"));

const parsed = computed(() => parseJson(input.value));
const stats = computed(() => (parsed.value.value === undefined ? null : jsonStats(input.value, parsed.value.value)));
const queryResult = computed(() => {
  if (!queryPath.value.trim() || parsed.value.error || parsed.value.value === undefined) return null;
  return queryJsonPath(parsed.value.value, queryPath.value);
});

function apply(transform: (value: unknown) => string) {
  if (parsed.value.error || parsed.value.value === undefined) {
    output.value = parsed.value.error ? `Invalid JSON: ${parsed.value.error}` : "";
    return;
  }
  output.value = transform(parsed.value.value);
}

function doFormat() {
  apply((value) => formatJson(value, 2));
}
function doMinify() {
  apply((value) => minifyJson(value));
}
function doSort() {
  apply((value) => sortJsonKeys(value));
}

function clearAll() {
  input.value = "";
  output.value = "";
  queryPath.value = "";
}

async function copyOutput() {
  const text = output.value || input.value;
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1400);
  } catch {
    copied.value = false;
  }
}

function saveCurrent() {
  const text = (output.value || input.value).trim();
  if (!text) return;
  history.value = addSnippetHistory("json", {
    id: generateToolboxId("json"),
    label: text.slice(0, 48).replace(/\s+/g, " "),
    text,
    at: Date.now(),
  });
}

function loadEntry(entry: SnippetHistoryEntry) {
  input.value = entry.text;
  output.value = "";
}

function removeEntry(id: string) {
  history.value = deleteSnippetHistory("json", id);
}

function clearHistory() {
  history.value = clearSnippetHistory("json");
}

const prettyQuery = computed(() => {
  if (!queryResult.value) return "";
  if (!queryResult.value.found) return "";
  return formatJson(queryResult.value.value, 2);
});
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-2 overflow-y-auto pr-1">
      <div class="flex flex-wrap items-center gap-1.5">
        <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="doFormat">
          <Wand2 class="h-3 w-3" /> Format
        </button>
        <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="doMinify">
          <Minimize2 class="h-3 w-3" /> Minify
        </button>
        <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="doSort">
          <ArrowDownAZ class="h-3 w-3" /> Sort keys
        </button>
        <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="copyOutput">
          <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? "Copied" : "Copy" }}
        </button>
        <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="clearAll">
          <Eraser class="h-3 w-3" /> Clear
        </button>
        <button class="ml-auto flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)] disabled:opacity-40" :disabled="!input.trim() && !output.trim()" @click="saveCurrent">
          <Save class="h-3 w-3" /> Save
        </button>
      </div>

      <div class="flex items-center gap-2 rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 px-2.5 py-1.5">
        <Braces class="h-3.5 w-3.5 text-[var(--primary)]" />
        <span v-if="stats" class="text-[10px] text-[var(--muted-foreground)]">
          {{ stats.type }} · {{ stats.keys }} key{{ stats.keys === 1 ? "" : "s" }} · depth {{ stats.depth }} · {{ stats.size }} chars
        </span>
        <span v-else-if="input.trim()" class="text-[10px] text-[var(--destructive)]">Invalid JSON: {{ parsed.error }}</span>
        <span v-else class="text-[10px] text-[var(--muted-foreground)]">Paste JSON below to format, validate and inspect it.</span>
      </div>

      <div class="grid flex-1 min-h-0 grid-cols-2 gap-2">
        <div class="flex min-h-[220px] flex-col">
          <span class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">Input</span>
          <textarea
            v-model="input"
            spellcheck="false"
            placeholder='{ "hello": "world" }'
            class="flex-1 resize-none rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/50"
          />
        </div>
        <div class="flex min-h-[220px] flex-col">
          <span class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">Result</span>
          <textarea
            :value="output"
            readonly
            spellcheck="false"
            placeholder="Transformed output appears here"
            class="flex-1 resize-none rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/50"
          />
        </div>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center gap-1.5">
          <Search class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Find a value by path</span>
        </div>
        <input
          v-model="queryPath"
          placeholder="e.g. user.address.city or items[0].id"
          class="mt-1.5 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
        <pre v-if="prettyQuery" class="mt-1.5 max-h-32 overflow-auto rounded bg-[var(--input-background)] p-2 font-mono text-[11px] text-[var(--foreground)]">{{ prettyQuery }}</pre>
        <p v-else-if="queryResult && !queryResult.found" class="mt-1 text-[10px] text-[var(--destructive)]">{{ queryResult.error }}</p>
      </div>
    </div>

    <div class="w-[220px] flex-shrink-0 flex flex-col rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25">
      <div class="flex items-center justify-between border-b border-[var(--border)] px-2.5 py-2">
        <div class="flex items-center gap-1.5">
          <History class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Recent</span>
        </div>
        <button v-if="history.length" class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--destructive)]" @click="clearHistory">Clear</button>
      </div>
      <div class="flex-1 overflow-y-auto p-1.5">
        <p v-if="!history.length" class="px-1.5 py-2 text-[10px] text-[var(--muted-foreground)]">Saved JSON appears here.</p>
        <button
          v-for="entry in history"
          :key="entry.id"
          class="group mb-1 flex w-full items-start justify-between gap-1.5 rounded border border-transparent px-1.5 py-1.5 text-left hover:border-[var(--border)] hover:bg-[var(--card)]"
          @click="loadEntry(entry)"
        >
          <span class="min-w-0 flex-1">
            <span class="block truncate font-mono text-[10px] text-[var(--foreground)]">{{ entry.label }}</span>
            <span class="block text-[9px] text-[var(--muted-foreground)]/70">{{ formatHistoryTime(entry.at) }}</span>
          </span>
          <span class="rounded p-0.5 text-[var(--muted-foreground)] opacity-0 group-hover:opacity-100 hover:text-[var(--destructive)]" @click.stop="removeEntry(entry.id)">
            <Trash2 class="h-3 w-3" />
          </span>
        </button>
      </div>
    </div>
  </div>
</template>

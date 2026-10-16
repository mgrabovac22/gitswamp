<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Link2, Plus, Trash2, Copy, Check, Save, ArrowRightLeft, History } from "lucide-vue-next";
import { buildUrl, decodeBase64, encodeBase64, newParam, parseUrl, safeDecode, safeEncode, type UrlParam } from "../urlTool";
import {
  addSnippetHistory,
  clearSnippetHistory,
  deleteSnippetHistory,
  formatHistoryTime,
  generateToolboxId,
  getSnippetHistory,
  type SnippetHistoryEntry,
} from "../toolboxStorage";

const rawUrl = ref("");
const params = ref<UrlParam[]>([]);
const copied = ref(false);
const encodeInput = ref("");
const encodeOutput = ref("");
const history = ref<SnippetHistoryEntry[]>(getSnippetHistory("url"));

const parsed = computed(() => parseUrl(rawUrl.value));
const builtUrl = computed(() => buildUrl(rawUrl.value, params.value));

watch(rawUrl, () => {
  params.value = parsed.value.params.map((param) => newParam(param.key, param.value));
});

function addParam() {
  params.value = [...params.value, newParam()];
}

function removeParam(id: string) {
  params.value = params.value.filter((param) => param.id !== id);
}

async function copyBuilt() {
  if (!builtUrl.value) return;
  try {
    await navigator.clipboard.writeText(builtUrl.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1400);
  } catch {
    copied.value = false;
  }
}

function applyEncoder(fn: (input: string) => { value: string; error: string | null }) {
  const result = fn(encodeInput.value);
  encodeOutput.value = result.error ? `Error: ${result.error}` : result.value;
}

function swapEncode() {
  encodeInput.value = encodeOutput.value;
  encodeOutput.value = "";
}

function saveCurrent() {
  const text = rawUrl.value.trim();
  if (!text) return;
  history.value = addSnippetHistory("url", {
    id: generateToolboxId("url"),
    label: text.slice(0, 56),
    text,
    at: Date.now(),
  });
}

function loadEntry(entry: SnippetHistoryEntry) {
  rawUrl.value = entry.text;
}

function removeEntry(id: string) {
  history.value = deleteSnippetHistory("url", id);
}

function clearHistory() {
  history.value = clearSnippetHistory("url");
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-2 overflow-y-auto pr-1">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
        <div class="flex items-center gap-1.5">
          <Link2 class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">URL being inspected</span>
        </div>
        <input
          v-model="rawUrl"
          placeholder="https://api.example.com/v1/users?page=1&limit=20#results"
          class="mt-1.5 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
        <p v-if="parsed.error" class="mt-1 text-[10px] text-[var(--destructive)]">{{ parsed.error }}</p>
      </div>

      <div v-if="parsed.valid" class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Parts</span>
        <div class="mt-1.5 grid grid-cols-[90px_1fr] gap-x-3 gap-y-1 text-[11px]">
          <span class="text-[var(--muted-foreground)]">Protocol</span><span class="font-mono text-[var(--foreground)]">{{ parsed.protocol || "(none)" }}</span>
          <span class="text-[var(--muted-foreground)]">Host</span><span class="font-mono text-[var(--foreground)]">{{ parsed.host || "(none)" }}</span>
          <span class="text-[var(--muted-foreground)]">Path</span><span class="font-mono text-[var(--foreground)]">{{ parsed.pathname || "/" }}</span>
          <span class="text-[var(--muted-foreground)]">Port</span><span class="font-mono text-[var(--foreground)]">{{ parsed.port || "(default)" }}</span>
          <span class="text-[var(--muted-foreground)]">Hash</span><span class="font-mono text-[var(--foreground)]">{{ parsed.hash || "(none)" }}</span>
          <span class="text-[var(--muted-foreground)]">User</span><span class="font-mono text-[var(--foreground)]">{{ parsed.username || "(none)" }}</span>
        </div>
      </div>

      <div v-if="parsed.valid" class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Query parameters</span>
          <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="addParam">
            <Plus class="h-3 w-3" /> Add
          </button>
        </div>
        <p v-if="!params.length" class="mt-1.5 text-[10px] text-[var(--muted-foreground)]">No query parameters in this URL.</p>
        <div v-else class="mt-1.5 space-y-1">
          <div v-for="param in params" :key="param.id" class="flex items-center gap-1.5">
            <input v-model="param.key" placeholder="key" class="w-[38%] min-w-0 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 font-mono text-[10px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/50" />
            <input v-model="param.value" placeholder="value" class="flex-1 min-w-0 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 font-mono text-[10px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/50" />
            <button class="rounded p-1 text-[var(--muted-foreground)] hover:bg-[var(--destructive)]/15 hover:text-[var(--destructive)]" @click="removeParam(param.id)">
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        <div class="mt-2 flex items-center gap-2 border-t border-[var(--border)] pt-2">
          <span class="flex-1 truncate font-mono text-[10px] text-[var(--muted-foreground)]" :title="builtUrl">{{ builtUrl || "—" }}</span>
          <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)] disabled:opacity-40" :disabled="!builtUrl" @click="copyBuilt">
            <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? "Copied" : "Copy URL" }}
          </button>
        </div>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center gap-1.5">
          <ArrowRightLeft class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Text encoders</span>
        </div>
        <div class="mt-1.5 grid grid-cols-2 gap-2">
          <textarea v-model="encodeInput" rows="3" spellcheck="false" placeholder="Text to encode" class="resize-none rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/50" />
          <textarea v-model="encodeOutput" rows="3" spellcheck="false" placeholder="Result" class="resize-none rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/50" />
        </div>
        <div class="mt-1.5 flex flex-wrap items-center gap-1.5">
          <button class="rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="applyEncoder(safeEncode)">URL encode</button>
          <button class="rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="applyEncoder(safeDecode)">URL decode</button>
          <button class="rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="applyEncoder(encodeBase64)">Base64 encode</button>
          <button class="rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="applyEncoder(decodeBase64)">Base64 decode</button>
          <button class="ml-auto rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="swapEncode">Swap</button>
        </div>
      </div>
    </div>

    <div class="w-[220px] flex-shrink-0 flex flex-col rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25">
      <div class="flex items-center justify-between border-b border-[var(--border)] px-2.5 py-2">
        <div class="flex items-center gap-1.5">
          <History class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Recent URLs</span>
        </div>
        <button v-if="history.length" class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--destructive)]" @click="clearHistory">Clear</button>
      </div>
      <div class="flex-1 overflow-y-auto p-1.5">
        <p v-if="!history.length" class="px-1.5 py-2 text-[10px] text-[var(--muted-foreground)]">Saved URLs appear here.</p>
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
      <div class="border-t border-[var(--border)] p-1.5">
        <button class="flex w-full items-center justify-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85 disabled:opacity-40" :disabled="!rawUrl.trim()" @click="saveCurrent">
          <Save class="h-3 w-3" /> Save current URL
        </button>
      </div>
    </div>
  </div>
</template>

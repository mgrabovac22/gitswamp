<script setup lang="ts">
import { computed, ref } from "vue";
import { Fingerprint, KeyRound, Clock, Copy, Check, RefreshCw, Trash2, Save, History } from "lucide-vue-next";
import { describeTimestamp, generateBase64UrlToken, generateHexToken, generateNumericCode, generateUuid, parseTimestamp } from "../idTimeTool";
import {
  addSnippetHistory,
  clearSnippetHistory,
  deleteSnippetHistory,
  formatHistoryTime,
  generateToolboxId,
  getSnippetHistory,
  type SnippetHistoryEntry,
} from "../toolboxStorage";

const uuidCount = ref(5);
const uuids = ref<string[]>(Array.from({ length: 5 }, () => generateUuid()));
const tokenKind = ref<"hex" | "base64url" | "numeric">("hex");
const tokenBytes = ref(16);
const tokenValue = ref(generateHexToken(16));
const timestampInput = ref(String(Math.floor(Date.now() / 1000)));
const copiedKey = ref("");
const history = ref<SnippetHistoryEntry[]>(getSnippetHistory("ids"));

const timeResult = computed(() => parseTimestamp(timestampInput.value));
const timeParts = computed(() => (timeResult.value.valid && timeResult.value.ms !== null ? describeTimestamp(timeResult.value.ms) : null));

function regenerateUuids() {
  const count = Math.max(1, Math.min(Math.floor(uuidCount.value) || 1, 50));
  uuidCount.value = count;
  uuids.value = Array.from({ length: count }, () => generateUuid());
}

function regenerateToken() {
  if (tokenKind.value === "hex") tokenValue.value = generateHexToken(tokenBytes.value);
  else if (tokenKind.value === "base64url") tokenValue.value = generateBase64UrlToken(tokenBytes.value);
  else tokenValue.value = generateNumericCode(tokenBytes.value);
}

function useNow() {
  timestampInput.value = String(Math.floor(Date.now() / 1000));
}

async function copyValue(key: string, value: string) {
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    copiedKey.value = key;
    setTimeout(() => {
      if (copiedKey.value === key) copiedKey.value = "";
    }, 1400);
  } catch {
    copiedKey.value = "";
  }
}

function saveValue(value: string, label: string) {
  if (!value.trim()) return;
  history.value = addSnippetHistory("ids", {
    id: generateToolboxId("ids"),
    label,
    text: value,
    at: Date.now(),
  });
}

function removeEntry(id: string) {
  history.value = deleteSnippetHistory("ids", id);
}

function clearHistory() {
  history.value = clearSnippetHistory("ids");
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-2 overflow-y-auto pr-1">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-1.5">
            <Fingerprint class="h-3.5 w-3.5 text-[var(--primary)]" />
            <span class="text-[11px] font-semibold text-[var(--foreground)]">UUID v4 generator</span>
          </div>
          <div class="flex items-center gap-1.5">
            <input v-model.number="uuidCount" type="number" min="1" max="50" class="w-14 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)]" />
            <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="regenerateUuids">
              <RefreshCw class="h-3 w-3" /> Generate
            </button>
            <button class="rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copyValue('all-uuids', uuids.join('\n'))">
              <component :is="copiedKey === 'all-uuids' ? Check : Copy" class="inline h-3 w-3" />
            </button>
          </div>
        </div>
        <div class="mt-1.5 space-y-1">
          <div v-for="(uuid, index) in uuids" :key="index" class="group flex items-center gap-1.5 rounded bg-[var(--input-background)] px-2 py-1">
            <span class="flex-1 truncate font-mono text-[11px] text-[var(--foreground)]">{{ uuid }}</span>
            <button class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--primary)]" title="Copy" @click="copyValue('uuid-' + index, uuid)">
              <component :is="copiedKey === 'uuid-' + index ? Check : Copy" class="h-3 w-3" />
            </button>
            <button class="rounded p-0.5 text-[var(--muted-foreground)] opacity-0 group-hover:opacity-100 hover:text-[var(--foreground)]" title="Save to history" @click="saveValue(uuid, 'UUID ' + uuid.slice(0, 8))">
              <Save class="h-3 w-3" />
            </button>
          </div>
        </div>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center gap-1.5">
          <KeyRound class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Random token</span>
        </div>
        <div class="mt-1.5 flex flex-wrap items-center gap-1.5">
          <select v-model="tokenKind" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)]" @change="regenerateToken">
            <option value="hex">Hex</option>
            <option value="base64url">Base64 URL-safe</option>
            <option value="numeric">Numeric code</option>
          </select>
          <label class="flex items-center gap-1 text-[10px] text-[var(--muted-foreground)]">
            {{ tokenKind === "numeric" ? "Digits" : "Bytes" }}
            <input v-model.number="tokenBytes" type="number" min="1" max="64" class="w-14 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)]" />
          </label>
          <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="regenerateToken">
            <RefreshCw class="h-3 w-3" /> New
          </button>
        </div>
        <div class="mt-1.5 flex items-center gap-1.5 rounded bg-[var(--input-background)] px-2 py-1.5">
          <span class="flex-1 break-all font-mono text-[11px] text-[var(--foreground)]">{{ tokenValue }}</span>
          <button class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--primary)]" title="Copy" @click="copyValue('token', tokenValue)">
            <component :is="copiedKey === 'token' ? Check : Copy" class="h-3 w-3" />
          </button>
          <button class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--foreground)]" title="Save to history" @click="saveValue(tokenValue, tokenKind + ' token')">
            <Save class="h-3 w-3" />
          </button>
        </div>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-1.5">
            <Clock class="h-3.5 w-3.5 text-[var(--primary)]" />
            <span class="text-[11px] font-semibold text-[var(--foreground)]">Timestamp converter</span>
          </div>
          <button class="rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="useNow">Use now</button>
        </div>
        <input
          v-model="timestampInput"
          placeholder="Epoch seconds, epoch milliseconds or a date like 2024-01-31T10:00:00Z"
          class="mt-1.5 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
        <p v-if="timeResult.error" class="mt-1 text-[10px] text-[var(--destructive)]">{{ timeResult.error }}</p>
        <div v-if="timeParts" class="mt-1.5 grid grid-cols-[110px_1fr_auto] gap-x-3 gap-y-1 text-[11px]">
          <span class="text-[var(--muted-foreground)]">ISO 8601</span><span class="truncate font-mono text-[var(--foreground)]">{{ timeParts.iso }}</span>
          <button class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--primary)]" @click="copyValue('iso', timeParts.iso)"><component :is="copiedKey === 'iso' ? Check : Copy" class="h-3 w-3" /></button>
          <span class="text-[var(--muted-foreground)]">Local time</span><span class="truncate font-mono text-[var(--foreground)]">{{ timeParts.local }}</span>
          <button class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--primary)]" @click="copyValue('local', timeParts.local)"><component :is="copiedKey === 'local' ? Check : Copy" class="h-3 w-3" /></button>
          <span class="text-[var(--muted-foreground)]">Epoch (s)</span><span class="font-mono text-[var(--foreground)]">{{ timeParts.epochSeconds }}</span>
          <button class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--primary)]" @click="copyValue('secs', timeParts.epochSeconds)"><component :is="copiedKey === 'secs' ? Check : Copy" class="h-3 w-3" /></button>
          <span class="text-[var(--muted-foreground)]">Epoch (ms)</span><span class="font-mono text-[var(--foreground)]">{{ timeParts.epochMillis }}</span>
          <button class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--primary)]" @click="copyValue('millis', timeParts.epochMillis)"><component :is="copiedKey === 'millis' ? Check : Copy" class="h-3 w-3" /></button>
          <span class="text-[var(--muted-foreground)]">Relative</span><span class="text-[var(--foreground)]">{{ timeParts.relative }}</span><span />
        </div>
      </div>
    </div>

    <div class="w-[220px] flex-shrink-0 flex flex-col rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25">
      <div class="flex items-center justify-between border-b border-[var(--border)] px-2.5 py-2">
        <div class="flex items-center gap-1.5">
          <History class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Saved</span>
        </div>
        <button v-if="history.length" class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--destructive)]" @click="clearHistory">Clear</button>
      </div>
      <div class="flex-1 overflow-y-auto p-1.5">
        <p v-if="!history.length" class="px-1.5 py-2 text-[10px] text-[var(--muted-foreground)]">Saved IDs and timestamps appear here.</p>
        <div
          v-for="entry in history"
          :key="entry.id"
          class="group mb-1 flex items-start justify-between gap-1.5 rounded border border-transparent px-1.5 py-1.5 hover:border-[var(--border)] hover:bg-[var(--card)]"
        >
          <button class="min-w-0 flex-1 text-left" @click="copyValue('hist-' + entry.id, entry.text)">
            <span class="block truncate font-mono text-[10px] text-[var(--foreground)]">{{ entry.text }}</span>
            <span class="block truncate text-[9px] text-[var(--muted-foreground)]">{{ entry.label }} · {{ formatHistoryTime(entry.at) }}</span>
          </button>
          <span class="rounded p-0.5 text-[var(--muted-foreground)] opacity-0 group-hover:opacity-100 hover:text-[var(--destructive)]" @click="removeEntry(entry.id)">
            <Trash2 class="h-3 w-3" />
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { ShieldCheck, Copy, Check, Save } from "lucide-vue-next";
import { decodeJwt, jwtTimeClaims } from "../jwtTool";
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

const token = ref("");
const copied = ref("");
const history = ref<SnippetHistoryEntry[]>(getSnippetHistory("jwt"));

const decoded = computed(() => decodeJwt(token.value));
const claims = computed(() => (decoded.value.valid ? jwtTimeClaims(decoded.value.payload) : []));
const headerJson = computed(() => (decoded.value.header ? JSON.stringify(decoded.value.header, null, 2) : ""));
const payloadJson = computed(() => (decoded.value.payload ? JSON.stringify(decoded.value.payload, null, 2) : ""));
const historyItems = computed(() =>
  history.value.map((entry) => ({ id: entry.id, primary: entry.label, mono: true, tertiary: formatHistoryTime(entry.at) })),
);

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
  const text = token.value.trim();
  if (!text) return;
  history.value = addSnippetHistory("jwt", {
    id: generateToolboxId("jwt"),
    label: text.slice(0, 52),
    text,
    at: Date.now(),
  });
}

function loadEntry(id: string) {
  const entry = history.value.find((item) => item.id === id);
  if (entry) token.value = entry.text;
}

function removeEntry(id: string) {
  history.value = deleteSnippetHistory("jwt", id);
}

function clearHistory() {
  history.value = clearSnippetHistory("jwt");
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-2 overflow-y-auto pr-1">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
        <div class="flex items-center gap-1.5">
          <ShieldCheck class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">JWT token</span>
        </div>
        <textarea
          v-model="token"
          rows="3"
          spellcheck="false"
          placeholder="eyJhbGciOi…  paste a JSON Web Token here"
          class="mt-1.5 w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
        <p v-if="decoded.error" class="mt-1 text-[10px] text-[var(--destructive)]">{{ decoded.error }}</p>
        <p v-else-if="token.trim()" class="mt-1 text-[10px] text-[var(--primary)]">Decoded locally — signatures are not verified.</p>
      </div>

      <template v-if="decoded.valid">
        <div v-for="block in [{ key: 'header', title: 'Header', json: headerJson }, { key: 'payload', title: 'Payload', json: payloadJson }]" :key="block.key" class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
          <div class="flex items-center justify-between">
            <span class="text-[11px] font-semibold text-[var(--foreground)]">{{ block.title }}</span>
            <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copyValue(block.key, block.json)">
              <component :is="copied === block.key ? Check : Copy" class="h-3 w-3" /> {{ copied === block.key ? "Copied" : "Copy" }}
            </button>
          </div>
          <pre class="mt-1.5 max-h-52 overflow-auto rounded bg-[var(--input-background)] p-2 font-mono text-[11px] text-[var(--foreground)]">{{ block.json }}</pre>
        </div>

        <div v-if="claims.length" class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Time claims</span>
          <div class="mt-1.5 space-y-1">
            <div v-for="claim in claims" :key="claim.claim" class="flex items-center justify-between text-[11px]">
              <span class="text-[var(--muted-foreground)]">{{ claim.claim }}</span>
              <span :class="claim.expired ? 'text-[var(--destructive)]' : 'text-[var(--foreground)]'">{{ claim.date }} · {{ claim.relative }}</span>
            </div>
          </div>
        </div>

        <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Signature</span>
          <p class="mt-1 break-all rounded bg-[var(--input-background)] p-2 font-mono text-[10px] text-[var(--muted-foreground)]">{{ decoded.signature || "(none — unsigned token)" }}</p>
        </div>
      </template>
    </div>

    <ToolHistoryRail
      title="Recent tokens"
      empty-text="Saved tokens appear here."
      :items="historyItems"
      @select="loadEntry"
      @remove="removeEntry"
      @clear="clearHistory"
    >
      <template #footer>
        <div class="border-t border-[var(--border)] p-1.5">
          <button class="flex w-full items-center justify-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85 disabled:opacity-40" :disabled="!token.trim()" @click="saveCurrent">
            <Save class="h-3 w-3" /> Save token
          </button>
        </div>
      </template>
    </ToolHistoryRail>
  </div>
</template>

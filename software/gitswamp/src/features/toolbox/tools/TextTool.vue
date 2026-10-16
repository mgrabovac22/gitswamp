<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Type, Copy, Check, Save } from "lucide-vue-next";
import { applyLineOp, CASE_TRANSFORMS, LINE_OPS, textStats, type LineOp } from "../textTool";
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
const output = ref("");
const copied = ref(false);
const history = ref<SnippetHistoryEntry[]>(getSnippetHistory("text"));

const stats = computed(() => textStats(input.value));
const historyItems = computed(() =>
  history.value.map((entry) => ({ id: entry.id, primary: entry.label, mono: true, tertiary: formatHistoryTime(entry.at) })),
);

watch(input, () => {
  if (output.value) output.value = "";
});

function applyCase(transform: (value: string) => string) {
  output.value = transform(input.value);
}

function applyLine(op: LineOp) {
  output.value = applyLineOp(input.value, op);
}

async function copyOutput() {
  if (!output.value) return;
  try {
    await navigator.clipboard.writeText(output.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1400);
  } catch {
    copied.value = false;
  }
}

function useOutputAsInput() {
  if (!output.value) return;
  input.value = output.value;
  output.value = "";
}

function saveCurrent() {
  const text = (output.value || input.value).trim();
  if (!text) return;
  history.value = addSnippetHistory("text", {
    id: generateToolboxId("text"),
    label: text.slice(0, 56).replace(/\s+/g, " "),
    text,
    at: Date.now(),
  });
}

function loadEntry(id: string) {
  const entry = history.value.find((item) => item.id === id);
  if (entry) {
    input.value = entry.text;
    output.value = "";
  }
}

function removeEntry(id: string) {
  history.value = deleteSnippetHistory("text", id);
}

function clearHistory() {
  history.value = clearSnippetHistory("text");
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-2 overflow-y-auto pr-1">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
        <div class="flex items-center gap-1.5">
          <Type class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Text</span>
        </div>
        <textarea
          v-model="input"
          rows="4"
          spellcheck="false"
          placeholder="Paste or type text to transform…"
          class="mt-1.5 w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
        <div class="mt-1.5 flex flex-wrap gap-x-3 gap-y-0.5 text-[10px] text-[var(--muted-foreground)]">
          <span>{{ stats.characters }} chars</span>
          <span>{{ stats.charactersNoSpaces }} without spaces</span>
          <span>{{ stats.words }} words</span>
          <span>{{ stats.lines }} lines</span>
          <span>{{ stats.bytes }} bytes</span>
        </div>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Change case</span>
        <div class="mt-1.5 flex flex-wrap gap-1">
          <button
            v-for="preset in CASE_TRANSFORMS"
            :key="preset.id"
            class="rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/60 hover:bg-[var(--primary)]/15"
            @click="applyCase(preset.run)"
          >
            {{ preset.label }}
          </button>
        </div>

        <span class="mt-2.5 block text-[11px] font-semibold text-[var(--foreground)]">Work with lines</span>
        <div class="mt-1.5 flex flex-wrap gap-1">
          <button
            v-for="op in LINE_OPS"
            :key="op.id"
            class="rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/60 hover:bg-[var(--primary)]/15"
            @click="applyLine(op.id)"
          >
            {{ op.label }}
          </button>
        </div>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Result</span>
          <div class="flex items-center gap-1.5">
            <button class="rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)] disabled:opacity-40" :disabled="!output" @click="useOutputAsInput">Use as input</button>
            <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)] disabled:opacity-40" :disabled="!output" @click="copyOutput">
              <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? "Copied" : "Copy" }}
            </button>
          </div>
        </div>
        <textarea
          :value="output"
          readonly
          rows="4"
          spellcheck="false"
          placeholder="Transformed text appears here"
          class="mt-1.5 w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
      </div>
    </div>

    <ToolHistoryRail
      title="Recent text"
      empty-text="Saved snippets appear here."
      :items="historyItems"
      @select="loadEntry"
      @remove="removeEntry"
      @clear="clearHistory"
    >
      <template #footer>
        <div class="border-t border-[var(--border)] p-1.5">
          <button class="flex w-full items-center justify-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85 disabled:opacity-40" :disabled="!input.trim() && !output.trim()" @click="saveCurrent">
            <Save class="h-3 w-3" /> Save current
          </button>
        </div>
      </template>
    </ToolHistoryRail>
  </div>
</template>

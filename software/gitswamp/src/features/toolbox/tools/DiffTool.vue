<script setup lang="ts">
import { computed, ref } from "vue";
import { Columns2, Save, ArrowRightLeft } from "lucide-vue-next";
import { computeLineDiff, diffStats } from "../diffTool";
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

const SEP = "\n\u0000DIFF\u0000\n";

const left = ref("");
const right = ref("");
const ignoreCase = ref(false);
const ignoreWhitespace = ref(false);
const history = ref<SnippetHistoryEntry[]>(getSnippetHistory("diff"));

const diff = computed(() => computeLineDiff(left.value, right.value, { ignoreCase: ignoreCase.value, ignoreWhitespace: ignoreWhitespace.value }));
const stats = computed(() => diffStats(diff.value));
const historyItems = computed(() =>
  history.value.map((entry) => ({ id: entry.id, primary: entry.label, mono: true, tertiary: formatHistoryTime(entry.at) })),
);

function swap() {
  [left.value, right.value] = [right.value, left.value];
}

function saveCurrent() {
  if (!left.value.trim() && !right.value.trim()) return;
  const label = (left.value || right.value).slice(0, 50).replace(/\s+/g, " ");
  history.value = addSnippetHistory("diff", {
    id: generateToolboxId("diff"),
    label: label || "comparison",
    text: `${left.value}${SEP}${right.value}`,
    at: Date.now(),
  });
}

function loadEntry(id: string) {
  const entry = history.value.find((item) => item.id === id);
  if (!entry) return;
  const parts = entry.text.split(SEP);
  left.value = parts[0] ?? "";
  right.value = parts[1] ?? "";
}

function removeEntry(id: string) {
  history.value = deleteSnippetHistory("diff", id);
}

function clearHistory() {
  history.value = clearSnippetHistory("diff");
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-2 overflow-y-auto pr-1">
      <div class="flex items-center justify-between rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 px-2.5 py-1.5">
        <div class="flex items-center gap-1.5">
          <Columns2 class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Compare two texts</span>
        </div>
        <div class="flex items-center gap-2">
          <label class="flex items-center gap-1 text-[10px] text-[var(--muted-foreground)]"><input v-model="ignoreCase" type="checkbox" class="h-3 w-3 accent-[var(--primary)]" /> Ignore case</label>
          <label class="flex items-center gap-1 text-[10px] text-[var(--muted-foreground)]"><input v-model="ignoreWhitespace" type="checkbox" class="h-3 w-3 accent-[var(--primary)]" /> Ignore spacing</label>
          <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="swap">
            <ArrowRightLeft class="h-3 w-3" /> Swap
          </button>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-2">
        <div class="flex flex-col">
          <span class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-[var(--destructive)]">Original</span>
          <textarea v-model="left" rows="6" spellcheck="false" placeholder="Original text…" class="resize-y rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/50" />
        </div>
        <div class="flex flex-col">
          <span class="mb-1 text-[10px] font-semibold uppercase tracking-wide" style="color: var(--primary)">Changed</span>
          <textarea v-model="right" rows="6" spellcheck="false" placeholder="Changed text…" class="resize-y rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/50" />
        </div>
      </div>

      <div class="flex items-center gap-3 rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 px-2.5 py-1.5 text-[10px]">
        <span class="text-[var(--primary)]">+{{ stats.added }} added</span>
        <span class="text-[var(--destructive)]">−{{ stats.removed }} removed</span>
        <span class="text-[var(--muted-foreground)]">{{ stats.unchanged }} unchanged</span>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Difference</span>
        <p v-if="!diff.length" class="mt-1 text-[10px] text-[var(--muted-foreground)]">Both sides are empty.</p>
        <div v-else class="mt-1.5 max-h-[320px] overflow-auto rounded bg-[var(--input-background)] p-1 font-mono text-[11px] leading-5">
          <div
            v-for="(line, index) in diff"
            :key="index"
            class="flex items-start gap-2 rounded px-1"
            :class="line.kind === 'add' ? 'bg-[var(--primary)]/15' : line.kind === 'remove' ? 'bg-[var(--destructive)]/15' : ''"
          >
            <span class="w-7 flex-shrink-0 select-none text-right text-[9px] text-[var(--muted-foreground)]">{{ line.leftNo ?? "" }}</span>
            <span class="w-7 flex-shrink-0 select-none text-right text-[9px] text-[var(--muted-foreground)]">{{ line.rightNo ?? "" }}</span>
            <span class="w-3 flex-shrink-0 select-none text-center" :class="line.kind === 'add' ? 'text-[var(--primary)]' : line.kind === 'remove' ? 'text-[var(--destructive)]' : 'text-transparent'">{{ line.kind === "add" ? "+" : line.kind === "remove" ? "−" : "·" }}</span>
            <span class="whitespace-pre-wrap break-all" :class="line.kind === 'equal' ? 'text-[var(--muted-foreground)]' : 'text-[var(--foreground)]'">{{ line.text || " " }}</span>
          </div>
        </div>
      </div>
    </div>

    <ToolHistoryRail
      title="Recent comparisons"
      empty-text="Saved comparisons appear here."
      :items="historyItems"
      @select="loadEntry"
      @remove="removeEntry"
      @clear="clearHistory"
    >
      <template #footer>
        <div class="border-t border-[var(--border)] p-1.5">
          <button class="flex w-full items-center justify-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85 disabled:opacity-40" :disabled="!left.trim() && !right.trim()" @click="saveCurrent">
            <Save class="h-3 w-3" /> Save comparison
          </button>
        </div>
      </template>
    </ToolHistoryRail>
  </div>
</template>

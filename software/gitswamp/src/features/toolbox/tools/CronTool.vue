<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { CalendarClock, Copy, Check, Save } from "lucide-vue-next";
import { CRON_PRESETS, buildCron, explainCron, nextCronRuns, normalizeCron, parseCron } from "../cronTool";
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

const FIELD_LABELS = ["Minute", "Hour", "Day (month)", "Month", "Weekday"];
const FIELD_PLACEHOLDERS = ["* or 0-59", "* or 0-23", "* or 1-31", "* or 1-12", "* or 0-6 (Sun=0)"];

const expr = ref("0 9 * * 1-5");
const fields = ref<string[]>(["0", "9", "*", "*", "1-5"]);
const copied = ref(false);
const history = ref<SnippetHistoryEntry[]>(getSnippetHistory("cron"));

const normalized = computed(() => normalizeCron(expr.value));
const parsed = computed(() => parseCron(expr.value));
const explanation = computed(() => explainCron(expr.value));
const runs = computed(() => nextCronRuns(expr.value, 5));
const historyItems = computed(() =>
  history.value.map((entry) => ({ id: entry.id, primary: entry.label, mono: true, tertiary: formatHistoryTime(entry.at) })),
);

let syncing = false;
watch(expr, (value) => {
  if (syncing) return;
  const parts = normalizeCron(value).split(/\s+/);
  if (parts.length === 5) fields.value = [...parts];
});

function updateFromFields() {
  const next = buildCron(fields.value[0], fields.value[1], fields.value[2], fields.value[3], fields.value[4]);
  if (next === expr.value) return;
  syncing = true;
  expr.value = next;
  syncing = false;
}

function applyPreset(value: string) {
  expr.value = value;
  const parts = normalizeCron(value).split(/\s+/);
  if (parts.length === 5) fields.value = [...parts];
}

async function copyExpr() {
  if (!normalized.value) return;
  try {
    await navigator.clipboard.writeText(normalized.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1400);
  } catch {
    copied.value = false;
  }
}

function saveCurrent() {
  if (!normalized.value) return;
  history.value = addSnippetHistory("cron", {
    id: generateToolboxId("cron"),
    label: normalized.value,
    text: normalized.value,
    at: Date.now(),
  });
}

function loadEntry(id: string) {
  const entry = history.value.find((item) => item.id === id);
  if (entry) applyPreset(entry.text);
}

function removeEntry(id: string) {
  history.value = deleteSnippetHistory("cron", id);
}

function clearHistory() {
  history.value = clearSnippetHistory("cron");
}

function formatRun(date: Date): string {
  return date.toLocaleString();
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-2 overflow-y-auto pr-1">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-1.5">
            <CalendarClock class="h-3.5 w-3.5 text-[var(--primary)]" />
            <span class="text-[11px] font-semibold text-[var(--foreground)]">Cron expression</span>
          </div>
          <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copyExpr">
            <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? "Copied" : "Copy" }}
          </button>
        </div>
        <input
          v-model="expr"
          spellcheck="false"
          placeholder="minute hour day month weekday — e.g. 0 9 * * 1-5"
          class="mt-1.5 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[12px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
        <p v-if="parsed.error" class="mt-1 text-[10px] text-[var(--destructive)]">{{ parsed.error }}</p>
        <p v-else-if="normalized" class="mt-1 text-[10px] text-[var(--primary)]">Valid expression.</p>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Build it field by field</span>
        <div class="mt-1.5 grid grid-cols-5 gap-1.5">
          <div v-for="(label, index) in FIELD_LABELS" :key="label" class="flex flex-col gap-1">
            <span class="text-[9px] text-[var(--muted-foreground)]">{{ label }}</span>
            <input
              v-model="fields[index]"
              :placeholder="FIELD_PLACEHOLDERS[index]"
              spellcheck="false"
              class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-center font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/40"
              @input="updateFromFields"
            />
          </div>
        </div>
        <div class="mt-2 flex flex-wrap gap-1">
          <button
            v-for="preset in CRON_PRESETS"
            :key="preset.expr"
            class="rounded-full border border-[var(--border)] bg-[var(--secondary)] px-2 py-0.5 text-[10px] text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/60 hover:bg-[var(--primary)]/15"
            @click="applyPreset(preset.expr)"
          >
            {{ preset.label }}
          </button>
        </div>
      </div>

      <div v-if="!parsed.error && normalized" class="grid grid-cols-1 gap-2">
        <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
          <span class="text-[11px] font-semibold text-[var(--foreground)]">In plain English</span>
          <ul class="mt-1.5 space-y-0.5">
            <li v-for="(line, index) in explanation.lines" :key="index" class="flex gap-1.5 text-[11px] text-[var(--muted-foreground)]">
              <span class="text-[var(--primary)]">•</span><span>{{ line }}</span>
            </li>
          </ul>
        </div>
        <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Next runs</span>
          <p v-if="runs.error" class="mt-1 text-[10px] text-[var(--muted-foreground)]">{{ runs.error }}</p>
          <ul v-else class="mt-1.5 space-y-0.5">
            <li v-for="(run, index) in runs.runs" :key="index" class="font-mono text-[11px] text-[var(--foreground)]">{{ formatRun(run) }}</li>
          </ul>
        </div>
      </div>
    </div>

    <ToolHistoryRail
      title="Saved schedules"
      empty-text="Saved expressions appear here."
      :items="historyItems"
      @select="loadEntry"
      @remove="removeEntry"
      @clear="clearHistory"
    >
      <template #footer>
        <div class="border-t border-[var(--border)] p-1.5">
          <button class="flex w-full items-center justify-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85 disabled:opacity-40" :disabled="!normalized || !!parsed.error" @click="saveCurrent">
            <Save class="h-3 w-3" /> Save expression
          </button>
        </div>
      </template>
    </ToolHistoryRail>
  </div>
</template>

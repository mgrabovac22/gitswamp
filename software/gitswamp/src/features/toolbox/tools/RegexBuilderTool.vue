<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import {
  Plus,
  ArrowUp,
  ArrowDown,
  Trash2,
  Copy,
  CopyPlus,
  Check,
  Save,
  RotateCcw,
  Regex,
  ListOrdered,
  FlaskConical,
  History,
} from "lucide-vue-next";
import {
  buildRegex,
  createRule,
  testRegex,
  REGEX_RULE_KINDS,
  REGEX_RULE_MAP,
  type RegexOptions,
  type RegexRule,
  type RegexRuleKind,
} from "../regexBuilder";
import {
  addRegexHistory,
  clearRegexHistory,
  deleteRegexHistory,
  formatHistoryTime,
  generateToolboxId,
  getRegexDraft,
  getRegexHistory,
  setRegexDraft,
  type RegexHistoryEntry,
} from "../toolboxStorage";

interface RegexDraft {
  rules: RegexRule[];
  options: RegexOptions;
  sample: string;
}

let ruleSeed = 0;
function nextRuleId(): string {
  ruleSeed += 1;
  return `rule-${ruleSeed}-${Math.random().toString(36).slice(2, 7)}`;
}

const DEFAULT_OPTIONS: RegexOptions = { wholeMatch: false, caseInsensitive: false, multiline: false, global: true };

const rules = ref<RegexRule[]>([createRule("contains", nextRuleId())]);
const options = ref<RegexOptions>({ ...DEFAULT_OPTIONS });
const sample = ref("");
const nameInput = ref("");
const history = ref<RegexHistoryEntry[]>(getRegexHistory());
const copied = ref(false);
const newRuleKind = ref<RegexRuleKind>("contains");
const recipeEl = ref<HTMLElement | null>(null);

const build = computed(() => buildRegex(rules.value, options.value));
const test = computed(() => testRegex(build.value.pattern, build.value.flags, sample.value));

const recipeSummary = computed(() =>
  rules.value
    .filter((rule) => rule.enabled)
    .map((rule) => {
      const meta = REGEX_RULE_MAP[rule.kind];
      const value = rule.value.trim();
      if (value && meta.value) return `${meta.label}: ${value}`;
      if (rule.kind === "between") return `${meta.label} (${rule.min}-${rule.max})`;
      if (rule.kind === "atLeast") return `${meta.label} (${rule.min}+)`;
      return meta.label;
    }),
);

const sampleSegments = computed(() => {
  const src = sample.value;
  const res = test.value;
  const segments: { text: string; match: boolean }[] = [];
  if (!src) return segments;
  if (!res.matches.length) {
    segments.push({ text: src, match: false });
    return segments;
  }
  let cursor = 0;
  for (const match of res.matches) {
    if (match.index > cursor) segments.push({ text: src.slice(cursor, match.index), match: false });
    if (match.text.length === 0) continue;
    segments.push({ text: src.slice(match.index, match.index + match.text.length), match: true });
    cursor = match.index + match.text.length;
  }
  if (cursor < src.length) segments.push({ text: src.slice(cursor), match: false });
  return segments;
});

onMounted(() => {
  const draft = getRegexDraft<RegexDraft>();
  if (!draft || !Array.isArray(draft.rules) || !draft.rules.length) return;
  rules.value = draft.rules.map((rule) => ({
    id: rule.id || nextRuleId(),
    kind: REGEX_RULE_MAP[rule.kind] ? rule.kind : "contains",
    value: typeof rule.value === "string" ? rule.value : "",
    min: Number.isFinite(rule.min) ? rule.min : 1,
    max: Number.isFinite(rule.max) ? rule.max : 3,
    enabled: rule.enabled !== false,
  }));
  options.value = { ...DEFAULT_OPTIONS, ...(draft.options ?? {}) };
  sample.value = typeof draft.sample === "string" ? draft.sample : "";
});

let draftTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  [rules, options, sample],
  () => {
    if (draftTimer) clearTimeout(draftTimer);
    draftTimer = setTimeout(() => {
      setRegexDraft({ rules: rules.value, options: options.value, sample: sample.value });
    }, 400);
  },
  { deep: true },
);

async function focusRuleValue(id: string) {
  await nextTick();
  recipeEl.value?.querySelector<HTMLInputElement>(`[data-rule-value="${id}"]`)?.focus();
}

function addRule(kind?: RegexRuleKind) {
  const rule = createRule(kind ?? newRuleKind.value, nextRuleId());
  rules.value = [...rules.value, rule];
  if (REGEX_RULE_MAP[rule.kind].value) void focusRuleValue(rule.id);
}

function duplicateRule(index: number) {
  const source = rules.value[index];
  const clone: RegexRule = { ...source, id: nextRuleId(), value: source.value };
  const next = [...rules.value];
  next.splice(index + 1, 0, clone);
  rules.value = next;
  if (REGEX_RULE_MAP[clone.kind].value) void focusRuleValue(clone.id);
}

function removeRule(id: string) {
  rules.value = rules.value.filter((rule) => rule.id !== id);
}

function moveRule(index: number, direction: -1 | 1) {
  const target = index + direction;
  if (target < 0 || target >= rules.value.length) return;
  const next = [...rules.value];
  const [item] = next.splice(index, 1);
  next.splice(target, 0, item);
  rules.value = next;
}

function resetAll() {
  rules.value = [createRule("contains", nextRuleId())];
  options.value = { ...DEFAULT_OPTIONS };
  sample.value = "";
  nameInput.value = "";
}

function saveCurrent() {
  if (!build.value.pattern || build.value.error) return;
  const name = nameInput.value.trim() || build.value.pattern;
  history.value = addRegexHistory({
    id: generateToolboxId("regex"),
    name,
    pattern: build.value.pattern,
    flags: build.value.flags,
    at: Date.now(),
  });
  nameInput.value = "";
}

function loadEntry(entry: RegexHistoryEntry) {
  const rule = createRule("custom", nextRuleId());
  rule.value = entry.pattern;
  rules.value = [rule];
  options.value = {
    wholeMatch: false,
    caseInsensitive: entry.flags.includes("i"),
    multiline: entry.flags.includes("m"),
    global: entry.flags.includes("g"),
  };
}

function removeEntry(id: string) {
  history.value = deleteRegexHistory(id);
}

function clearHistory() {
  history.value = clearRegexHistory();
}

async function copyPattern() {
  if (!build.value.pattern) return;
  try {
    await navigator.clipboard.writeText(`/${build.value.pattern}/${build.value.flags}`);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1400);
  } catch {
    copied.value = false;
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex-1 min-w-0 flex flex-col gap-3 overflow-y-auto pr-1">
      <div ref="recipeEl" class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-center gap-1.5">
            <ListOrdered class="h-3.5 w-3.5 text-[var(--primary)]" />
            <span class="text-[11px] font-semibold text-[var(--foreground)]">Recipe — top step runs first (highest priority)</span>
          </div>
          <button
            class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]"
            @click="resetAll"
          >
            <RotateCcw class="h-3 w-3" /> Reset
          </button>
        </div>

        <div class="mb-2 rounded-md border border-dashed border-[var(--border)] bg-[var(--card)] p-2">
          <div class="flex items-center gap-2">
            <select v-model="newRuleKind" class="flex-1 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)]">
              <option v-for="meta in REGEX_RULE_KINDS" :key="meta.kind" :value="meta.kind">{{ meta.label }}</option>
            </select>
            <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85" @click="addRule()">
              <Plus class="h-3 w-3" /> Add step
            </button>
          </div>
        </div>

        <div v-if="recipeSummary.length" class="mb-2 flex flex-wrap items-center gap-1">
          <template v-for="(step, index) in recipeSummary" :key="index">
            <span v-if="index > 0" class="text-[10px] text-[var(--muted-foreground)]">→</span>
            <span class="rounded bg-[var(--primary)]/12 px-1.5 py-0.5 text-[10px] text-[var(--foreground)]">{{ step }}</span>
          </template>
        </div>

        <div class="space-y-1.5">
          <div
            v-for="(rule, index) in rules"
            :key="rule.id"
            class="rounded-md border border-[var(--border)] bg-[var(--card)] p-2"
          >
            <div class="flex items-start gap-2">
              <div class="flex flex-col items-center gap-0.5 pt-0.5">
                <span class="flex h-4 w-4 items-center justify-center rounded bg-[var(--primary)]/15 text-[9px] font-bold text-[var(--primary)]">{{ index + 1 }}</span>
                <button class="text-[var(--muted-foreground)] hover:text-[var(--foreground)] disabled:opacity-30" :disabled="index === 0" title="Move up" @click="moveRule(index, -1)">
                  <ArrowUp class="h-3 w-3" />
                </button>
                <button class="text-[var(--muted-foreground)] hover:text-[var(--foreground)] disabled:opacity-30" :disabled="index === rules.length - 1" title="Move down" @click="moveRule(index, 1)">
                  <ArrowDown class="h-3 w-3" />
                </button>
              </div>

              <div class="flex-1 min-w-0 space-y-1.5">
                <div class="flex items-center gap-2">
                  <input
                    type="checkbox"
                    class="h-3 w-3 accent-[var(--primary)]"
                    :checked="rule.enabled"
                    title="Enable or disable this step"
                    @change="rule.enabled = ($event.target as HTMLInputElement).checked"
                  />
                  <select
                    v-model="rule.kind"
                    class="min-w-0 flex-1 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)]"
                  >
                    <option v-for="meta in REGEX_RULE_KINDS" :key="meta.kind" :value="meta.kind">{{ meta.label }}</option>
                  </select>
                  <button class="rounded p-1 text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" title="Duplicate step" @click="duplicateRule(index)">
                    <CopyPlus class="h-3.5 w-3.5" />
                  </button>
                  <button class="rounded p-1 text-[var(--muted-foreground)] hover:bg-[var(--destructive)]/15 hover:text-[var(--destructive)]" title="Remove step" @click="removeRule(rule.id)">
                    <Trash2 class="h-3.5 w-3.5" />
                  </button>
                </div>

                <div v-if="REGEX_RULE_MAP[rule.kind].value" class="flex items-center gap-2">
                  <input
                    v-model="rule.value"
                    :data-rule-value="rule.id"
                    :placeholder="REGEX_RULE_MAP[rule.kind].placeholder"
                    class="flex-1 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
                  />
                </div>

                <div v-if="REGEX_RULE_MAP[rule.kind].range" class="flex items-center gap-2 text-[10px] text-[var(--muted-foreground)]">
                  <template v-if="rule.kind === 'between'">
                    <span>Between</span>
                    <input v-model.number="rule.min" type="number" min="0" class="w-14 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)]" />
                    <span>and</span>
                    <input v-model.number="rule.max" type="number" min="0" class="w-14 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)]" />
                    <span>times</span>
                  </template>
                  <template v-else>
                    <span>At least</span>
                    <input v-model.number="rule.min" type="number" min="0" class="w-14 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)]" />
                    <span>times</span>
                  </template>
                </div>

                <p class="text-[10px] leading-4 text-[var(--muted-foreground)]">{{ REGEX_RULE_MAP[rule.kind].hint }}</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
        <div class="flex flex-wrap items-center gap-3">
          <label class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)]">
            <input v-model="options.wholeMatch" type="checkbox" class="h-3 w-3 accent-[var(--primary)]" /> Whole input must match
          </label>
          <label class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)]">
            <input v-model="options.caseInsensitive" type="checkbox" class="h-3 w-3 accent-[var(--primary)]" /> Ignore letter case
          </label>
          <label class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)]">
            <input v-model="options.multiline" type="checkbox" class="h-3 w-3 accent-[var(--primary)]" /> Line-by-line (^ and $)
          </label>
          <label class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)]">
            <input v-model="options.global" type="checkbox" class="h-3 w-3 accent-[var(--primary)]" /> Find all matches
          </label>
        </div>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-1.5">
            <Regex class="h-3.5 w-3.5 text-[var(--primary)]" />
            <span class="text-[11px] font-semibold text-[var(--foreground)]">Generated pattern</span>
          </div>
          <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)] disabled:opacity-40" :disabled="!build.pattern" @click="copyPattern">
            <component :is="copied ? Check : Copy" class="h-3 w-3" /> {{ copied ? "Copied" : "Copy" }}
          </button>
        </div>
        <pre class="mt-1.5 overflow-x-auto rounded bg-[var(--input-background)] p-2 text-[11px] font-mono text-[var(--foreground)]">{{ build.pattern ? '/' + build.pattern + '/' + build.flags : 'Add steps to build your pattern…' }}</pre>
        <p v-if="build.error" class="mt-1 text-[10px] text-[var(--destructive)]">Invalid pattern: {{ build.error }}</p>

        <ul v-if="build.explanation.length" class="mt-2 space-y-0.5">
          <li v-for="(line, i) in build.explanation" :key="i" class="flex gap-1.5 text-[10px] text-[var(--muted-foreground)]">
            <span class="text-[var(--primary)]">•</span><span>{{ line }}</span>
          </li>
        </ul>

        <div class="mt-2.5 flex items-center gap-2 border-t border-[var(--border)] pt-2">
          <input v-model="nameInput" placeholder="Name this pattern (optional)" class="flex-1 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60" />
          <button class="flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1 text-[10px] text-[var(--foreground)] hover:opacity-85 disabled:opacity-40" :disabled="!build.pattern || !!build.error" @click="saveCurrent">
            <Save class="h-3 w-3" /> Save
          </button>
        </div>
      </div>

      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <div class="flex items-center gap-1.5">
          <FlaskConical class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[11px] font-semibold text-[var(--foreground)]">Try it out</span>
        </div>
        <textarea
          v-model="sample"
          rows="3"
          placeholder="Paste some text here to see what matches…"
          class="mt-1.5 w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-[11px] font-mono text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]/60"
        />
        <div v-if="sample" class="mt-1.5 rounded bg-[var(--input-background)] p-2 text-[11px] font-mono leading-5">
          <span v-if="!build.pattern" class="text-[var(--muted-foreground)]">No pattern yet.</span>
          <span v-else-if="test.error" class="text-[var(--destructive)]">{{ test.error }}</span>
          <template v-else>
            <span
              v-for="(segment, i) in sampleSegments"
              :key="i"
              :class="segment.match ? 'rounded bg-[var(--primary)]/30 text-[var(--foreground)]' : 'text-[var(--muted-foreground)]'"
            >{{ segment.text }}</span>
          </template>
        </div>
        <p v-if="sample && build.pattern && !test.error" class="mt-1 text-[10px] text-[var(--muted-foreground)]">
          {{ test.matches.length }} match{{ test.matches.length === 1 ? "" : "es" }}
          <span v-if="test.matches[0]"> — first: “{{ test.matches[0].text }}”</span>
        </p>
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
        <p v-if="!history.length" class="px-1.5 py-2 text-[10px] text-[var(--muted-foreground)]">Saved patterns appear here.</p>
        <button
          v-for="entry in history"
          :key="entry.id"
          class="group mb-1 flex w-full items-start justify-between gap-1.5 rounded border border-transparent px-1.5 py-1.5 text-left hover:border-[var(--border)] hover:bg-[var(--card)]"
          @click="loadEntry(entry)"
        >
          <span class="min-w-0 flex-1">
            <span class="block truncate text-[10px] font-medium text-[var(--foreground)]">{{ entry.name }}</span>
            <span class="block truncate font-mono text-[9px] text-[var(--muted-foreground)]">/{{ entry.pattern }}/{{ entry.flags }}</span>
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

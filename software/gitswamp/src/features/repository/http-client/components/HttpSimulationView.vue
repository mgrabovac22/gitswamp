<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  ArrowLeft,
  Play,
  Loader2,
  Trash2,
  FlaskConical,
  CheckCircle2,
  XCircle,
  FileText,
  ClipboardCopy,
  Settings2,
  Save,
  RotateCcw,
  Activity,
  HelpCircle,
} from "lucide-vue-next";
import type { HttpClientController } from "../useHttpClient";
import type { HttpSimulationConfig, HttpSimulationKind, HttpSimulationResult } from "../httpClient.types";
import { SIMULATION_PROFILES, defaultSimulationConfig, formatDuration } from "../httpClient.utils";
import HttpCheckbox from "./HttpCheckbox.vue";

const props = defineProps<{
  controller: HttpClientController;
}>();

const emit = defineEmits<{
  back: [];
}>();

const profiles = computed(() => SIMULATION_PROFILES);
const selectedKinds = ref<HttpSimulationKind[]>(["smoke"]);
const running = computed(() => props.controller.simulationActive.value);
const allResults = computed(() => props.controller.simulationResults.value);
const activeRequest = computed(() => props.controller.activeRequest.value);
const extracted = computed(() => {
  const record: Record<string, string> = {};
  for (const step of props.controller.runSteps.value) {
    Object.assign(record, step.extracted);
  }
  return { ...record, ...props.controller.sessionVariables.value };
});
const requests = computed(() => props.controller.visibleRequests.value);
const requestId = ref<string>(activeRequest.value?.id ?? requests.value[0]?.id ?? "");

watch(
  () => activeRequest.value?.id,
  (id) => {
    if (id && !requestId.value) requestId.value = id;
  },
);

const targetRequest = computed(() => requests.value.find((item) => item.id === requestId.value) ?? null);

const results = computed(() => allResults.value.filter((item) => item.requestId === requestId.value));

const displayResults = computed<HttpSimulationResult[]>(() => {
  if (results.value.length) return results.value;
  const smoke = allResults.value.filter((item) => item.kind === "smoke");
  const pool = smoke.length ? smoke : allResults.value;
  return pool.length ? [pool[0]] : [];
});

const usingFallback = computed(() => results.value.length === 0 && displayResults.value.length > 0);

const selectedResultId = ref<string | null>(null);

watch(
  displayResults,
  (list) => {
    if (list.length === 0) {
      selectedResultId.value = null;
      return;
    }
    if (!selectedResultId.value || !list.some((item) => item.id === selectedResultId.value)) {
      selectedResultId.value = list[0].id;
    }
  },
  { immediate: true },
);

const selectedResult = computed<HttpSimulationResult | null>(
  () => displayResults.value.find((item) => item.id === selectedResultId.value) ?? displayResults.value[0] ?? null,
);

const editingKind = ref<HttpSimulationKind | null>(null);
const draft = ref<HttpSimulationConfig>(defaultSimulationConfig("smoke"));

const editingProfile = computed(() =>
  editingKind.value ? profiles.value.find((item) => item.kind === editingKind.value) ?? null : null,
);

function configFor(kind: HttpSimulationKind): HttpSimulationConfig {
  return props.controller.simulationConfigFor(kind);
}

function toggleKind(kind: HttpSimulationKind): void {
  selectedKinds.value = selectedKinds.value.includes(kind)
    ? selectedKinds.value.filter((item) => item !== kind)
    : [...selectedKinds.value, kind];
}

function startEdit(kind: HttpSimulationKind): void {
  editingKind.value = kind;
  draft.value = { ...configFor(kind) };
}

function saveEdit(): void {
  if (!editingKind.value) return;
  props.controller.updateSimulationConfig(editingKind.value, { ...draft.value });
  editingKind.value = null;
}

function cancelEdit(): void {
  editingKind.value = null;
}

function resetEdit(): void {
  if (!editingKind.value) return;
  props.controller.resetSimulationConfig(editingKind.value);
  draft.value = defaultSimulationConfig(editingKind.value);
}

function runSimulations(): void {
  if (!targetRequest.value) return;
  editingKind.value = null;
  void props.controller.runSimulations([...selectedKinds.value], targetRequest.value.id);
}

function clear(): void {
  props.controller.clearSimulationResults();
  selectedResultId.value = null;
}

function passRate(result: HttpSimulationResult): string {
  if (!result.requests) return "0%";
  return `${Math.round((result.passed / result.requests) * 1000) / 10}%`;
}

function throughput(result: HttpSimulationResult): string {
  if (!result.simulatedDurationMs) return "—";
  return `${Math.round((result.requests / (result.simulatedDurationMs / 1000)) * 10) / 10}/s`;
}

function methodToneFor(method: string): string {
  switch (method) {
    case "GET":
      return "text-[#10b981]";
    case "POST":
      return "text-[#f59e0b]";
    case "PUT":
      return "text-[#3b82f6]";
    case "PATCH":
      return "text-[#a855f7]";
    case "DELETE":
      return "text-[#ef4444]";
    default:
      return "text-[var(--muted-foreground)]";
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <div class="flex items-center gap-2 border-b border-[var(--border)] bg-[var(--secondary)]/30 px-3 py-2">
      <button
        type="button"
        class="inline-flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[11px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
        @click="emit('back')"
      >
        <ArrowLeft class="h-3.5 w-3.5" /> Requests
      </button>
      <FlaskConical class="ml-1 h-4 w-4 text-[var(--primary)]" />
      <span class="text-sm font-semibold text-[var(--foreground)]">Simulated tests</span>
      <select
        v-model="requestId"
        class="ml-auto rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[11px] text-[var(--foreground)] focus:outline-none"
      >
        <option v-for="req in requests" :key="req.id" :value="req.id">{{ req.method }} {{ req.name }}</option>
      </select>
    </div>

    <div class="flex min-h-0 flex-1 overflow-hidden">
      <div class="flex w-[280px] shrink-0 flex-col overflow-hidden border-r border-[var(--border)]">
        <div class="flex items-center gap-1 border-b border-[var(--border)] px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
          Profiles
        </div>
        <div class="min-h-0 flex-1 overflow-y-auto p-1.5">
          <div
            v-for="profile in profiles"
            :key="profile.kind"
            class="flex cursor-pointer items-start gap-2 rounded px-1.5 py-1.5 transition-colors hover:bg-[var(--primary)]/8"
            :class="editingKind === profile.kind ? 'bg-[var(--primary)]/10 ring-1 ring-[var(--primary)]/40' : ''"
            @click="startEdit(profile.kind)"
          >
            <span class="mt-0.5">
              <HttpCheckbox
                :model-value="selectedKinds.includes(profile.kind)"
                @update:model-value="toggleKind(profile.kind)"
              />
            </span>
            <span class="min-w-0 flex-1">
              <span class="flex items-center gap-1 text-[11px] font-medium text-[var(--foreground)]">
                {{ profile.label }}
                <Settings2 class="h-3 w-3 text-[var(--muted-foreground)]" />
              </span>
              <span class="block text-[10px] leading-snug text-[var(--muted-foreground)]">{{ profile.description }}</span>
              <span class="block text-[9px] text-[var(--muted-foreground)]">
                {{ configFor(profile.kind).iterations }} iters · {{ configFor(profile.kind).thresholdMs ? formatDuration(configFor(profile.kind).thresholdMs) + " budget" : "no budget" }}
              </span>
            </span>
          </div>
        </div>
        <div class="border-t border-[var(--border)] p-2">
          <button
            type="button"
            class="inline-flex w-full items-center justify-center gap-1.5 rounded-md bg-[var(--primary)] px-3 py-1.5 text-[11px] font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50"
            :disabled="running || selectedKinds.length === 0 || !targetRequest"
            @click="runSimulations"
          >
            <Loader2 v-if="running" class="h-3.5 w-3.5 animate-spin" />
            <Play v-else class="h-3.5 w-3.5" />
            Run {{ selectedKinds.length }} profile(s)
          </button>
        </div>

        <div class="border-t border-[var(--border)] p-2">
          <div class="mb-1 flex items-center gap-1 text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">
            <ClipboardCopy class="h-3 w-3" /> Extracted
          </div>
          <div v-if="Object.keys(extracted).length === 0" class="text-[10px] text-[var(--muted-foreground)]">
            No values captured yet.
          </div>
          <div v-else class="max-h-32 overflow-y-auto text-[10px]">
            <div v-for="(value, name) in extracted" :key="name" class="flex items-baseline gap-1 py-0.5">
              <span class="font-mono text-[var(--primary)]">{{ name }}</span>
              <span class="text-[var(--muted-foreground)]">=</span>
              <span class="min-w-0 flex-1 truncate font-mono text-[var(--foreground)]" :title="String(value)">{{ value }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="flex min-w-0 flex-1 flex-col overflow-hidden">
        <template v-if="editingKind && editingProfile">
          <div class="flex items-center gap-1 border-b border-[var(--border)] px-2 py-1.5">
            <Settings2 class="h-3 w-3 text-[var(--primary)]" />
            <span class="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">Edit · {{ editingProfile.label }}</span>
          </div>
          <div class="min-h-0 flex-1 overflow-y-auto p-3">
            <p class="mb-3 text-[11px] leading-snug text-[var(--muted-foreground)]">{{ editingProfile.description }}</p>
            <div class="grid max-w-sm gap-3">
              <label class="block">
                <span class="mb-1 block text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Iterations</span>
                <input
                  v-model.number="draft.iterations"
                  type="number"
                  min="1"
                  max="500"
                  class="w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[12px] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--primary)]"
                >
                <span class="mt-1 block text-[9px] text-[var(--muted-foreground)]">Simulated requests in this profile (1–500).</span>
              </label>
              <label class="block">
                <span class="mb-1 block text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Expected status</span>
                <input
                  v-model.number="draft.expectedStatus"
                  type="number"
                  min="100"
                  max="599"
                  class="w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[12px] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--primary)]"
                >
                <span class="mt-1 block text-[9px] text-[var(--muted-foreground)]">HTTP status the endpoint is expected to return.</span>
              </label>
              <p class="rounded border border-[var(--border)] bg-[var(--secondary)]/20 p-2 text-[10px] leading-snug text-[var(--muted-foreground)]">
                Advanced latency settings are on the right. Press <span class="font-medium text-[var(--foreground)]">Save</span> to apply.
              </p>
            </div>
          </div>
          <div class="flex items-center gap-2 border-t border-[var(--border)] p-2">
            <button
              type="button"
              class="inline-flex items-center gap-1.5 rounded-md bg-[var(--primary)] px-3 py-1.5 text-[11px] font-medium text-white transition-opacity hover:opacity-90"
              @click="saveEdit"
            >
              <Save class="h-3.5 w-3.5" /> Save
            </button>
            <button
              type="button"
              class="inline-flex items-center gap-1.5 rounded-md border border-[var(--border)] px-3 py-1.5 text-[11px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
              @click="cancelEdit"
            >
              Cancel
            </button>
          </div>
        </template>

        <template v-else>
          <div class="flex items-center gap-1 border-b border-[var(--border)] px-2 py-1.5">
            <span class="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">Results</span>
            <span class="text-[10px] text-[var(--muted-foreground)]">{{ displayResults.length }}</span>
            <span v-if="usingFallback" class="rounded bg-[var(--secondary)]/50 px-1.5 py-0.5 text-[9px] text-[var(--muted-foreground)]">last run</span>
            <button v-if="results.length" type="button" class="ml-auto inline-flex items-center gap-1 rounded px-1.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[#ef4444]" @click="clear">
              <Trash2 class="h-3 w-3" /> Clear
            </button>
          </div>
          <div class="min-h-0 flex-1 overflow-y-auto p-2">
            <div v-if="displayResults.length === 0" class="flex h-full flex-col items-center justify-center gap-1.5 text-center text-[11px] text-[var(--muted-foreground)]">
              <FlaskConical class="h-6 w-6" />
              Pick one or more profiles and run them for the selected request.
            </div>
            <div
              v-for="result in displayResults"
              :key="result.id"
              class="mb-1.5 cursor-pointer rounded border p-2 transition-colors"
              :class="[result.id === selectedResultId ? 'border-[var(--primary)] bg-[var(--primary)]/8' : result.ok ? 'border-[#10b981]/30 hover:bg-[var(--primary)]/8' : 'border-[#ef4444]/30 hover:bg-[var(--primary)]/8']"
              @click="selectedResultId = result.id"
            >
              <div class="flex items-center gap-1.5">
                <component :is="result.ok ? CheckCircle2 : XCircle" class="h-3.5 w-3.5" :class="result.ok ? 'text-[#10b981]' : 'text-[#ef4444]'" />
                <span class="text-[11px] font-semibold text-[var(--foreground)]">{{ result.label }}</span>
                <span class="text-[9px] text-[var(--muted-foreground)]">{{ result.method }} {{ result.requestName }}</span>
                <span class="ml-auto text-[9px] text-[var(--muted-foreground)]">{{ new Date(result.at).toLocaleTimeString() }}</span>
              </div>
              <div class="mt-1 grid grid-cols-3 gap-1 text-center text-[10px] sm:grid-cols-6">
                <div><div class="font-semibold text-[var(--foreground)]">{{ result.requests }}</div><div class="text-[8px] text-[var(--muted-foreground)]">req</div></div>
                <div><div class="font-semibold text-[#10b981]">{{ result.passed }}</div><div class="text-[8px] text-[var(--muted-foreground)]">ok</div></div>
                <div><div class="font-semibold" :class="result.failed ? 'text-[#ef4444]' : 'text-[var(--foreground)]'">{{ result.failed }}</div><div class="text-[8px] text-[var(--muted-foreground)]">fail</div></div>
                <div><div class="font-semibold text-[var(--foreground)]">{{ formatDuration(result.avgMs) }}</div><div class="text-[8px] text-[var(--muted-foreground)]">avg</div></div>
                <div><div class="font-semibold text-[var(--foreground)]">{{ formatDuration(result.p50Ms) }}</div><div class="text-[8px] text-[var(--muted-foreground)]">p50</div></div>
                <div><div class="font-semibold text-[var(--foreground)]">{{ formatDuration(result.p95Ms) }}</div><div class="text-[8px] text-[var(--muted-foreground)]">p95</div></div>
              </div>
            </div>
          </div>
        </template>
      </div>

      <div class="flex w-[320px] shrink-0 flex-col overflow-hidden border-l border-[var(--border)]">
        <template v-if="editingKind && editingProfile">
          <div class="flex items-center gap-1 border-b border-[var(--border)] px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
            <Settings2 class="h-3 w-3" /> Advanced · {{ editingProfile.label }}
          </div>
          <div class="min-h-0 flex-1 overflow-y-auto p-3">
            <label class="block">
              <span class="mb-1 block text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Latency budget (p95, ms)</span>
              <input
                v-model.number="draft.thresholdMs"
                type="number"
                min="0"
                max="600000"
                class="w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[12px] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--primary)]"
              >
              <span class="mt-1 block text-[9px] leading-snug text-[var(--muted-foreground)]">
                The 95th percentile latency must stay under this value for the profile to pass. Set 0 to disable the latency check.
              </span>
            </label>
            <dl class="mt-3 space-y-1 text-[10px]">
              <div class="flex items-center justify-between gap-2">
                <dt class="text-[var(--muted-foreground)]">Iterations</dt>
                <dd class="font-mono text-[var(--foreground)]">{{ draft.iterations }}</dd>
              </div>
              <div class="flex items-center justify-between gap-2">
                <dt class="text-[var(--muted-foreground)]">Expected status</dt>
                <dd class="font-mono text-[var(--foreground)]">{{ draft.expectedStatus }}</dd>
              </div>
            </dl>
            <button
              type="button"
              class="mt-3 inline-flex items-center gap-1.5 rounded-md border border-[var(--border)] px-2.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
              @click="resetEdit"
            >
              <RotateCcw class="h-3 w-3" /> Reset to defaults
            </button>
          </div>
        </template>

        <template v-else>
          <div class="flex items-center gap-1 border-b border-[var(--border)] px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
            <FileText class="h-3 w-3" /> Detail
          </div>
          <div v-if="selectedResult" class="min-h-0 flex-1 overflow-y-auto p-2 text-[11px]">
            <div class="mb-2">
              <div class="flex items-center gap-1.5">
                <span class="font-mono text-[9px] font-semibold" :class="methodToneFor(selectedResult.method)">{{ selectedResult.method }}</span>
                <span class="min-w-0 flex-1 truncate text-[11px] font-semibold text-[var(--foreground)]">{{ selectedResult.requestName }}</span>
                <span class="text-[9px] text-[var(--muted-foreground)]">{{ selectedResult.label }}</span>
              </div>
              <div class="mt-0.5 truncate font-mono text-[9px] text-[var(--muted-foreground)]" :title="selectedResult.url">{{ selectedResult.url }}</div>
              <div class="mt-0.5 text-[9px] text-[var(--muted-foreground)]">{{ new Date(selectedResult.at).toLocaleString() }}</div>
            </div>

            <div class="grid grid-cols-2 gap-1.5 text-[10px]">
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Iterations</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ selectedResult.iterations }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Requests</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ selectedResult.requests }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Passed</div><div class="text-[16px] font-semibold text-[#10b981]">{{ selectedResult.passed }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Failed</div><div class="text-[16px] font-semibold" :class="selectedResult.failed ? 'text-[#ef4444]' : 'text-[var(--foreground)]'">{{ selectedResult.failed }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Pass rate</div><div class="text-[16px] font-semibold text-[#10b981]">{{ passRate(selectedResult) }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Error rate</div><div class="text-[16px] font-semibold" :class="selectedResult.errorRate ? 'text-[#ef4444]' : 'text-[var(--foreground)]'">{{ selectedResult.errorRate }}%</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Throughput</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ throughput(selectedResult) }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Simulated time</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(selectedResult.simulatedDurationMs) }}</div></div>
            </div>

            <div class="mt-2 text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Latency</div>
            <div class="mt-1 grid grid-cols-3 gap-1.5 text-[10px]">
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Min</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(selectedResult.minMs) }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">p50</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(selectedResult.p50Ms) }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Avg</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(selectedResult.avgMs) }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">p95</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(selectedResult.p95Ms) }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Max</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(selectedResult.maxMs) }}</div></div>
              <div class="rounded border border-[var(--border)] p-1.5"><div class="text-[8px] text-[var(--muted-foreground)]">Spread</div><div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(selectedResult.maxMs - selectedResult.minMs) }}</div></div>
            </div>

            <div class="mt-2 rounded border border-[var(--border)] bg-[var(--secondary)]/20 p-2">
              <div class="mb-1 text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Baseline vs simulated</div>
              <dl class="space-y-0.5 text-[10px]">
                <div class="flex items-center justify-between gap-2">
                  <dt class="text-[var(--muted-foreground)]">Baseline status</dt>
                  <dd class="font-mono text-[var(--foreground)]">{{ selectedResult.baselineStatus }}</dd>
                </div>
                <div class="flex items-center justify-between gap-2">
                  <dt class="text-[var(--muted-foreground)]">Expected status</dt>
                  <dd class="font-mono text-[var(--foreground)]">{{ selectedResult.expectedStatus }}</dd>
                </div>
                <div class="flex items-center justify-between gap-2">
                  <dt class="text-[var(--muted-foreground)]">Latency budget</dt>
                  <dd class="text-[var(--foreground)]">{{ selectedResult.thresholdMs ? formatDuration(selectedResult.thresholdMs) : "none" }}</dd>
                </div>
                <div class="flex items-center justify-between gap-2">
                  <dt class="text-[var(--muted-foreground)]">Verdict</dt>
                  <dd :class="selectedResult.ok ? 'text-[#10b981]' : 'text-[#ef4444]'">{{ selectedResult.ok ? "Pass" : "Fail" }}</dd>
                </div>
              </dl>
              <div class="mt-1.5 text-[9px] leading-snug text-[var(--muted-foreground)]">{{ selectedResult.note }}</div>
            </div>

            <div class="mt-2 flex items-start gap-1.5 rounded border border-[var(--border)]/70 bg-[var(--secondary)]/10 p-2">
              <HelpCircle class="mt-0.5 h-3 w-3 shrink-0 text-[var(--muted-foreground)]" />
              <p class="text-[9px] leading-snug text-[var(--muted-foreground)]">
                <span class="font-medium text-[var(--foreground)]">What is p95?</span> The 95th percentile latency: 95% of the
                simulated requests were as fast as this value, and only the slowest 5% were slower. It highlights worst-case
                behaviour better than the average. Lower is better; it should stay under the latency budget.
              </p>
            </div>
          </div>
          <div v-else class="flex flex-1 items-center justify-center gap-1.5 text-[10px] text-[var(--muted-foreground)]">
            <Activity class="h-3.5 w-3.5" /> Select a result.
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

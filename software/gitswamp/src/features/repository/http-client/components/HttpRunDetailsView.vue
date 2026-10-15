<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  ArrowLeft,
  Square,
  Clock,
  CheckCircle2,
  XCircle,
  Loader2,
  Trash2,
  Variable,
  Gauge,
  Braces,
  ShieldCheck,
  ListTree,
} from "lucide-vue-next";
import type { HttpClientController } from "../useHttpClient";
import type { HttpRunStep } from "../httpClient.types";
import { formatBytes, formatDuration, runStatsFor, statusTone } from "../httpClient.utils";

const props = defineProps<{
  controller: HttpClientController;
  collectionId?: string | null;
}>();

const emit = defineEmits<{
  back: [];
}>();

const steps = computed<HttpRunStep[]>(() => {
  const all = props.controller.runSteps.value;
  if (!props.collectionId) return all;
  return all.filter((step) => step.collectionId === props.collectionId);
});
const active = computed(() => props.controller.runActive.value);
const stats = computed(() => (steps.value.length ? runStatsFor(steps.value) : null));
const collectionName = computed(() => {
  if (!props.collectionId) return "";
  return props.controller.collections.value.find((item) => item.id === props.collectionId)?.name ?? "";
});

const selectedIndex = ref(0);
const detailTab = ref<"overview" | "assertions" | "headers" | "body">("overview");

watch(
  () => steps.value.length,
  (length) => {
    if (length === 0) selectedIndex.value = 0;
    else if (selectedIndex.value >= length) selectedIndex.value = length - 1;
  },
);

const selectedStep = computed<HttpRunStep | null>(() => steps.value[selectedIndex.value] ?? null);

const statusToneClass: Record<string, string> = {
  success: "text-[#10b981]",
  redirect: "text-[#3b82f6]",
  warning: "text-[#f59e0b]",
  danger: "text-[#ef4444]",
  muted: "text-[var(--muted-foreground)]",
};

const stepIcon: Record<string, unknown> = {
  running: Loader2,
  passed: CheckCircle2,
  failed: XCircle,
  skipped: Clock,
  pending: Clock,
};

function iconFor(status: string): unknown {
  return stepIcon[status] ?? Clock;
}

const testsPassed = computed(() =>
  selectedStep.value ? selectedStep.value.tests.filter((test) => test.passed).length : 0,
);

function prettyBody(body: string): string {
  const trimmed = body.trim();
  if (!trimmed) return "(empty body)";
  try {
    return JSON.stringify(JSON.parse(trimmed), null, 2).slice(0, 8000);
  } catch {
    return body.slice(0, 8000);
  }
}

function selectStep(index: number): void {
  selectedIndex.value = index;
  detailTab.value = "overview";
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
      <Gauge class="ml-1 h-4 w-4 text-[var(--primary)]" />
      <span class="text-sm font-semibold text-[var(--foreground)]">Run details</span>
      <span v-if="collectionName" class="rounded bg-[var(--secondary)]/60 px-1.5 py-0.5 text-[10px] text-[var(--muted-foreground)]">{{ collectionName }}</span>
      <span v-if="stats" class="text-[11px] text-[var(--muted-foreground)]">{{ stats.total }} request(s)</span>

      <div class="ml-auto flex items-center gap-1.5">
        <button
          v-if="active"
          type="button"
          class="inline-flex items-center gap-1 rounded border border-[#ef4444]/40 px-2 py-1 text-[11px] text-[#ef4444] transition-colors hover:bg-[#ef4444]/10"
          @click="controller.stopRun()"
        >
          <Square class="h-3 w-3" /> Stop
        </button>
        <button
          v-else
          type="button"
          class="inline-flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[11px] text-[var(--muted-foreground)] transition-colors hover:text-[#ef4444]"
          @click="controller.clearRun(); emit('back')"
        >
          <Trash2 class="h-3.5 w-3.5" /> Clear
        </button>
      </div>
    </div>

    <div v-if="stats" class="grid grid-cols-3 gap-2 border-b border-[var(--border)] bg-[var(--secondary)]/20 px-3 py-2 sm:grid-cols-6">
      <div class="text-center">
        <div class="text-[16px] font-semibold text-[var(--foreground)]">{{ stats.total }}</div>
        <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">requests</div>
      </div>
      <div class="text-center">
        <div class="text-[16px] font-semibold text-[#10b981]">{{ stats.passed }}</div>
        <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">passed</div>
      </div>
      <div class="text-center">
        <div class="text-[16px] font-semibold" :class="stats.failed ? 'text-[#ef4444]' : 'text-[var(--foreground)]'">{{ stats.failed }}</div>
        <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">failed</div>
      </div>
      <div class="text-center">
        <div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(stats.avgMs) }}</div>
        <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">average</div>
      </div>
      <div class="text-center">
        <div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(stats.p95Ms) }}</div>
        <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">p95</div>
      </div>
      <div class="text-center">
        <div class="text-[16px] font-semibold text-[var(--foreground)]">{{ formatDuration(stats.totalMs) }}</div>
        <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">total time</div>
      </div>
    </div>

    <div class="flex min-h-0 flex-1 overflow-hidden">
      <div class="flex w-[300px] shrink-0 flex-col overflow-hidden border-r border-[var(--border)]">
        <div class="flex items-center gap-1 border-b border-[var(--border)] px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
          <ListTree class="h-3 w-3" /> Requests
        </div>
        <div class="min-h-0 flex-1 overflow-y-auto">
          <div v-if="steps.length === 0" class="px-3 py-8 text-center text-[11px] text-[var(--muted-foreground)]">
            No run yet. Start a run from a collection or the Run all button.
          </div>
          <button
            v-for="(step, index) in steps"
            :key="step.requestId + index"
            type="button"
            class="flex w-full items-center gap-2 border-b border-[var(--border)]/40 px-2 py-1.5 text-left transition-colors"
            :class="selectedIndex === index ? 'bg-[var(--primary)]/10' : 'hover:bg-[var(--primary)]/8'"
            @click="selectStep(index)"
          >
            <component
              :is="iconFor(step.status)"
              class="h-3.5 w-3.5 shrink-0"
              :class="{
                'animate-spin text-[var(--primary)]': step.status === 'running',
                'text-[#10b981]': step.status === 'passed',
                'text-[#ef4444]': step.status === 'failed',
                'text-[var(--muted-foreground)]': step.status !== 'running' && step.status !== 'passed' && step.status !== 'failed',
                'ring-2 ring-[var(--primary)]/40 rounded-full': controller.runCursor.value === index,
              }"
            />
            <span class="w-10 shrink-0 font-mono text-[9px] font-semibold" :class="'text-[var(--foreground)]'">{{ step.method }}</span>
            <span class="min-w-0 flex-1 truncate text-[11px] text-[var(--foreground)]">{{ step.name }}</span>
            <span v-if="step.iterations > 1" class="shrink-0 text-[9px] text-[var(--muted-foreground)]">×{{ step.iterations }}</span>
            <span v-if="step.httpStatus" class="shrink-0 text-[10px]" :class="statusToneClass[statusTone(step.httpStatus)]">{{ step.httpStatus }}</span>
            <span class="shrink-0 text-[9px] text-[var(--muted-foreground)]">{{ formatDuration(step.durationMs) }}</span>
          </button>
        </div>
      </div>

      <div class="flex min-w-0 flex-1 flex-col overflow-hidden">
        <template v-if="selectedStep">
          <div class="flex items-center gap-1 border-b border-[var(--border)] px-2">
            <button
              v-for="tab in [['overview', 'Overview', ShieldCheck], ['assertions', 'Assertions', CheckCircle2], ['headers', 'Headers', ListTree], ['body', 'Body', Braces]] as const"
              :key="tab[0]"
              type="button"
              class="relative flex items-center gap-1 px-2.5 py-2 text-[11px] font-medium transition-colors"
              :class="detailTab === tab[0] ? 'text-[var(--primary)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
              @click="detailTab = tab[0]"
            >
              <component :is="tab[2]" class="h-3 w-3" />
              {{ tab[1] }}
              <span v-if="tab[0] === 'assertions' && selectedStep.tests.length" class="text-[9px] text-[var(--muted-foreground)]">({{ testsPassed }}/{{ selectedStep.tests.length }})</span>
              <span v-if="detailTab === tab[0]" class="absolute inset-x-1 bottom-0 h-0.5 rounded-full bg-[var(--primary)]"></span>
            </button>
          </div>

          <div class="min-h-0 flex-1 overflow-y-auto p-3">
            <div v-if="detailTab === 'overview'" class="flex flex-col gap-3">
              <div
                class="rounded-md border p-3"
                :class="selectedStep.status === 'failed' ? 'border-[#ef4444]/40 bg-[#ef4444]/5' : selectedStep.status === 'passed' ? 'border-[#10b981]/40 bg-[#10b981]/5' : 'border-[var(--border)] bg-[var(--secondary)]/20'"
              >
                <div class="flex items-center gap-2">
                  <component :is="iconFor(selectedStep.status)" class="h-4 w-4" :class="selectedStep.status === 'failed' ? 'text-[#ef4444]' : 'text-[#10b981]'" />
                  <span class="font-mono text-[11px] font-semibold text-[var(--foreground)]">{{ selectedStep.method }}</span>
                  <span class="truncate text-[11px] text-[var(--muted-foreground)]">{{ selectedStep.url }}</span>
                </div>
                <div v-if="selectedStep.error" class="mt-2 rounded border border-[#ef4444]/30 bg-[#ef4444]/10 px-2 py-1 text-[11px] text-[#ef4444]">{{ selectedStep.error }}</div>
              </div>

              <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
                <div class="rounded border border-[var(--border)] p-2">
                  <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Status</div>
                  <div class="text-[13px] font-semibold" :class="statusToneClass[statusTone(selectedStep.httpStatus)]">{{ selectedStep.httpStatus || "—" }}</div>
                </div>
                <div class="rounded border border-[var(--border)] p-2">
                  <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Time</div>
                  <div class="text-[13px] font-semibold text-[var(--foreground)]">{{ formatDuration(selectedStep.durationMs) }}</div>
                </div>
                <div class="rounded border border-[var(--border)] p-2">
                  <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Size</div>
                  <div class="text-[13px] font-semibold text-[var(--foreground)]">{{ selectedStep.sizeBytes ? formatBytes(selectedStep.sizeBytes) : "—" }}</div>
                </div>
                <div class="rounded border border-[var(--border)] p-2">
                  <div class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Iterations</div>
                  <div class="text-[13px] font-semibold text-[var(--foreground)]">{{ selectedStep.iterations }}</div>
                </div>
              </div>

              <div v-if="Object.keys(selectedStep.extracted).length" class="rounded border border-[var(--border)] bg-[var(--secondary)]/20 p-2">
                <div class="mb-1 flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
                  <Variable class="h-3 w-3" /> Extracted values
                </div>
                <div class="flex flex-wrap gap-1.5">
                  <span v-for="(value, name) in selectedStep.extracted" :key="name" class="rounded bg-[var(--primary)]/10 px-1.5 py-0.5 font-mono text-[10px] text-[var(--primary)]">{{ name }} = {{ value }}</span>
                </div>
              </div>
            </div>

            <div v-else-if="detailTab === 'assertions'" class="flex flex-col gap-2">
              <div v-if="selectedStep.tests.length === 0" class="py-10 text-center text-[11px] text-[var(--muted-foreground)]">
                This request has no assertions.
              </div>
              <div v-else>
                <div class="mb-2 text-[10px] text-[var(--muted-foreground)]">{{ testsPassed }} of {{ selectedStep.tests.length }} passed</div>
                <div class="divide-y divide-[var(--border)]/50 rounded border border-[var(--border)]">
                  <div v-for="test in selectedStep.tests" :key="test.name" class="flex items-start gap-2 px-2.5 py-2">
                    <component :is="test.passed ? CheckCircle2 : XCircle" class="mt-0.5 h-3.5 w-3.5 shrink-0" :class="test.passed ? 'text-[#10b981]' : 'text-[#ef4444]'" />
                    <div class="min-w-0">
                      <div class="text-[11px] text-[var(--foreground)]">{{ test.name }}</div>
                      <div v-if="test.message" class="text-[10px] text-[var(--muted-foreground)]">{{ test.message }}</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div v-else-if="detailTab === 'headers'" class="font-mono text-[11px]">
              <div v-if="selectedStep.responseHeaders.length === 0" class="py-10 text-center font-sans text-[11px] text-[var(--muted-foreground)]">
                No response headers captured.
              </div>
              <div v-else class="divide-y divide-[var(--border)]/40 rounded border border-[var(--border)]">
                <div v-for="header in selectedStep.responseHeaders" :key="header.name" class="flex gap-2 px-2 py-1">
                  <span class="w-48 shrink-0 truncate text-[var(--primary)]">{{ header.name }}</span>
                  <span class="min-w-0 flex-1 break-all text-[var(--muted-foreground)]">{{ header.value }}</span>
                </div>
              </div>
            </div>

            <div v-else class="flex flex-col gap-2">
              <pre class="max-h-full overflow-auto whitespace-pre-wrap break-all rounded border border-[var(--border)] bg-[var(--input-background)] p-2 font-mono text-[10px] text-[var(--foreground)]">{{ prettyBody(selectedStep.responseBody) }}</pre>
            </div>
          </div>
        </template>

        <div v-else class="flex flex-1 items-center justify-center text-[11px] text-[var(--muted-foreground)]">
          Select a request to inspect its result.
        </div>
      </div>
    </div>
  </div>
</template>

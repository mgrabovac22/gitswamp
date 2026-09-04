<script setup lang="ts">
import type {
  ManualBisectCommit,
  ManualBisectResult,
  ManualBisectSession,
} from "./useManualBisect";

const props = defineProps<{
  session: ManualBisectSession;
  busy: boolean;
  remaining: number;
  currentCommit: ManualBisectCommit | null;
  badBound: ManualBisectCommit | null;
  goodBound: ManualBisectCommit | null;
  culprit: ManualBisectCommit | null;
}>();

const emit = defineEmits<{
  cancel: [];
  retryCheckout: [];
  mark: [result: ManualBisectResult];
  checkoutCulprit: [];
  returnToOriginalBranch: [];
  close: [];
}>();

const steps = [
  { key: "broken", label: "Broken", description: "start point" },
  { key: "worked", label: "Worked", description: "old safe point" },
  { key: "test", label: "Test", description: "middle commit" },
  { key: "culprit", label: "Culprit", description: "final candidate" },
] as const;

function stepState(index: number): "done" | "active" | "pending" {
  if (props.session.phase === "select-good") {
    return index === 1 ? "active" : index === 0 ? "done" : "pending";
  }
  if (props.session.phase === "testing") {
    return index < 2 ? "done" : index === 2 ? "active" : "pending";
  }
  return "done";
}
</script>

<template>
  <div class="pointer-events-none absolute inset-x-4 top-4 z-[450] flex justify-center">
    <section class="pointer-events-auto w-[min(720px,100%)] rounded-lg border border-[var(--primary)]/35 bg-[var(--card)]/98 shadow-2xl shadow-black/35 backdrop-blur">
      <div class="flex items-start justify-between gap-3 border-b border-[var(--border)] px-4 py-3">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <span class="h-2.5 w-2.5 rounded-full bg-[var(--primary)] shadow-[0_0_14px_var(--primary)]" />
            <h3 class="text-sm font-semibold text-[var(--foreground)]">Bug Autopsy</h3>
            <span class="rounded-full bg-[var(--primary)]/12 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-[var(--primary)]">
              guided checkout
            </span>
          </div>
          <p class="mt-1 text-[11px] text-[var(--muted-foreground)]">
            GitSwamp moves through the history, you verify the app, then mark each checkout as Works or Broken.
          </p>
        </div>
        <button
          class="rounded-md border border-[var(--border)] px-2 py-1 text-[11px] font-medium text-[var(--muted-foreground)] transition-colors hover:bg-[var(--secondary)] hover:text-[var(--foreground)] disabled:opacity-50"
          :disabled="busy"
          @click="emit('cancel')"
        >
          Cancel
        </button>
      </div>

      <div class="grid grid-cols-4 gap-1.5 border-b border-[var(--border)] px-4 py-2">
        <div
          v-for="(step, index) in steps"
          :key="step.key"
          class="rounded-md border px-2 py-1.5"
          :class="{
            'border-[var(--primary)]/45 bg-[var(--primary)]/12 text-[var(--primary)]': stepState(index) === 'active',
            'border-[#10b981]/35 bg-[#10b981]/10 text-[#10b981]': stepState(index) === 'done',
            'border-[var(--border)] bg-[var(--input-background)] text-[var(--muted-foreground)]': stepState(index) === 'pending',
          }"
        >
          <div class="text-[10px] font-semibold">{{ index + 1 }}. {{ step.label }}</div>
          <div class="text-[9px] opacity-75">{{ step.description }}</div>
        </div>
      </div>

      <div v-if="session.phase === 'select-good'" class="space-y-3 px-4 py-3">
        <div class="rounded border border-[#ef4444]/25 bg-[#ef4444]/8 px-3 py-2">
          <div class="text-[10px] font-semibold uppercase tracking-wide text-[#ef4444]">Broken boundary</div>
          <div class="mt-1 truncate text-sm font-medium text-[var(--foreground)]">
            {{ badBound?.shortSha || session.badSha.slice(0, 7) }} · {{ badBound?.message || 'Selected commit' }}
          </div>
        </div>
        <p class="text-xs leading-relaxed text-[var(--foreground)]">
          Select an older commit in the graph where the app still worked, then click
          <span class="font-semibold text-[var(--primary)]">Use as worked commit</span> in the Info panel.
        </p>
      </div>

      <div v-else-if="session.phase === 'testing'" class="space-y-3 px-4 py-3">
        <div class="grid grid-cols-3 gap-2 text-[11px]">
          <div class="rounded border border-[var(--border)] bg-[var(--input-background)] px-3 py-2">
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Step</div>
            <div class="mt-1 text-sm font-semibold text-[var(--foreground)]">{{ session.step }}</div>
          </div>
          <div class="rounded border border-[var(--border)] bg-[var(--input-background)] px-3 py-2">
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Remaining</div>
            <div class="mt-1 text-sm font-semibold text-[var(--foreground)]">{{ remaining }}</div>
          </div>
          <div class="rounded border border-[var(--border)] bg-[var(--input-background)] px-3 py-2">
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Tested</div>
            <div class="mt-1 text-sm font-semibold text-[var(--foreground)]">{{ session.tested.length }}</div>
          </div>
        </div>

        <div class="rounded-lg border border-[var(--primary)]/30 bg-[var(--primary)]/8 px-3 py-3">
          <div class="text-[10px] font-semibold uppercase tracking-wide text-[var(--primary)]">
            {{ session.currentSha ? 'Now testing' : 'Checkout paused' }}
          </div>
          <div class="mt-1 truncate text-sm font-semibold text-[var(--foreground)]">
            {{ currentCommit?.shortSha || session.currentSha?.slice(0, 7) || 'waiting' }} · {{ currentCommit?.message || 'Checkout needs a clean working tree' }}
          </div>
          <div class="mt-1 text-[11px] text-[var(--muted-foreground)]">
            {{ session.currentSha ? 'Open or refresh your project, check if the bug is present, then classify this checkout.' : 'Clean or stash working changes, then retry this checkout.' }}
          </div>
          <button
            v-if="session.currentIndex !== null && !session.currentSha"
            class="mt-3 rounded-md border border-[var(--primary)]/35 bg-[var(--primary)]/12 px-3 py-1.5 text-[11px] font-semibold text-[var(--primary)] transition-colors hover:bg-[var(--primary)]/20 disabled:opacity-50"
            :disabled="busy"
            @click="emit('retryCheckout')"
          >
            Retry checkout
          </button>
        </div>

        <div class="flex gap-2">
          <button
            class="flex-1 rounded-md border border-[#10b981]/35 bg-[#10b981]/14 px-3 py-2 text-sm font-semibold text-[#10b981] transition-colors hover:bg-[#10b981]/22 disabled:opacity-50"
            :disabled="busy || !session.currentSha"
            @click="emit('mark', 'good')"
          >
            Works
          </button>
          <button
            class="flex-1 rounded-md border border-[#ef4444]/35 bg-[#ef4444]/14 px-3 py-2 text-sm font-semibold text-[#ef4444] transition-colors hover:bg-[#ef4444]/22 disabled:opacity-50"
            :disabled="busy || !session.currentSha"
            @click="emit('mark', 'bad')"
          >
            Broken
          </button>
        </div>

        <div class="grid grid-cols-2 gap-2 text-[10px]">
          <div class="rounded border border-[#10b981]/20 bg-[#10b981]/8 px-2 py-1.5">
            <span class="font-semibold text-[#10b981]">Known good</span>
            <div class="truncate text-[var(--muted-foreground)]">{{ goodBound?.shortSha }} · {{ goodBound?.message }}</div>
          </div>
          <div class="rounded border border-[#ef4444]/20 bg-[#ef4444]/8 px-2 py-1.5">
            <span class="font-semibold text-[#ef4444]">Known broken</span>
            <div class="truncate text-[var(--muted-foreground)]">{{ badBound?.shortSha }} · {{ badBound?.message }}</div>
          </div>
        </div>
      </div>

      <div v-else class="space-y-3 px-4 py-3">
        <div class="rounded-lg border border-[#f59e0b]/35 bg-[#f59e0b]/12 px-3 py-3">
          <div class="text-[10px] font-semibold uppercase tracking-wide text-[#f59e0b]">Likely breaking commit</div>
          <div class="mt-1 truncate text-base font-semibold text-[var(--foreground)]">
            {{ culprit?.shortSha || session.culpritSha?.slice(0, 7) }} · {{ culprit?.message || 'Culprit candidate' }}
          </div>
          <div class="mt-1 text-[11px] text-[var(--muted-foreground)]">
            The last known working commit is {{ goodBound?.shortSha || 'unknown' }} and the first known broken commit is {{ culprit?.shortSha || 'unknown' }}.
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            class="rounded-md bg-[var(--primary)] px-3 py-2 text-xs font-semibold text-white transition-opacity hover:opacity-90 disabled:opacity-50"
            :disabled="busy || !session.culpritSha"
            @click="emit('checkoutCulprit')"
          >
            Checkout culprit
          </button>
          <button
            v-if="session.originalBranch"
            class="rounded-md border border-[var(--border)] bg-[var(--input-background)] px-3 py-2 text-xs font-semibold text-[var(--foreground)] transition-colors hover:bg-[var(--secondary)] disabled:opacity-50"
            :disabled="busy"
            @click="emit('returnToOriginalBranch')"
          >
            Return to {{ session.originalBranch }}
          </button>
          <button
            class="rounded-md border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--muted-foreground)] transition-colors hover:bg-[var(--secondary)] hover:text-[var(--foreground)]"
            :disabled="busy"
            @click="emit('close')"
          >
            Close
          </button>
        </div>
      </div>
    </section>
  </div>
</template>

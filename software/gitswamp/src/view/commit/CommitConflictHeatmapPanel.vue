<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import logoCrocLoading from "@/assets/logo_croc_loading.gif";
import type { ConflictHotspot } from "@/types";

const PREVIEW_CONFLICT_LIMIT = 300;
const FULL_CONFLICT_LIMIT = 60000;

const props = defineProps<{
  repoPath: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

const loading = ref(false);
const error = ref("");
const query = ref("");
const items = ref<ConflictHotspot[]>([]);
const loadAll = ref(false);
const maxScore = computed(() => items.value.reduce((acc, item) => Math.max(acc, item.score), 0));

const filteredItems = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return items.value;
  return items.value.filter((item) => item.path.toLowerCase().includes(q));
});

function scoreRatio(score: number): number {
  if (maxScore.value <= 0) return 0;
  return Math.min(1, Math.max(0, score / maxScore.value));
}

function scoreBarStyle(score: number): Record<string, string> {
  const ratio = scoreRatio(score);
  const width = `${Math.max(10, Math.round(ratio * 100))}%`;
  const fill = `color-mix(in srgb, var(--destructive) ${Math.round(25 + ratio * 60)}%, transparent)`;
  return {
    width,
    background: fill,
  };
}

function rowStyle(score: number): Record<string, string> {
  const ratio = scoreRatio(score);
  return {
    borderColor: `color-mix(in srgb, var(--destructive) ${Math.round(18 + ratio * 55)}%, var(--border))`,
    background: `color-mix(in srgb, var(--destructive) ${Math.round(4 + ratio * 16)}%, var(--card))`,
  };
}

async function loadHeatmap() {
  if (!props.repoPath) {
    items.value = [];
    error.value = "";
    return;
  }

  loading.value = true;
  error.value = "";

  try {
    items.value = await invoke<ConflictHotspot[]>("get_conflict_hotspots", {
      path: props.repoPath,
      maxCount: loadAll.value ? FULL_CONFLICT_LIMIT : PREVIEW_CONFLICT_LIMIT,
    });
  } catch {
    items.value = [];
    error.value = "Could not load conflict hotspot analytics.";
  } finally {
    loading.value = false;
  }
}

function enableLoadAll() {
  if (loadAll.value) return;
  loadAll.value = true;
  void loadHeatmap();
}

watch(
  () => props.repoPath,
  () => {
    loadAll.value = false;
    void loadHeatmap();
  },
  { immediate: true },
);
</script>

<template>
  <div class="flex-1 min-h-0 overflow-y-auto bg-[var(--background)]">
    <div class="p-4 md:p-5 space-y-4">
      <section class="rounded-2xl border border-[var(--border)] bg-[var(--card)] p-4">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div>
            <p class="text-[10px] uppercase tracking-[0.16em] text-[var(--muted-foreground)]">Visualise Commit History</p>
            <h2 class="text-lg md:text-xl font-bold text-[var(--foreground)]">Usual Conflict Suspects</h2>
            <p class="text-xs text-[var(--muted-foreground)] mt-1">Files touched by merge-heavy history glow hotter in red.</p>
          </div>
          <button
            class="h-7 w-7 rounded-full border border-[var(--border)] bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:border-[var(--destructive)]/50 transition-colors"
            title="Back to Git Graph"
            @click="emit('close')"
          >
            x
          </button>
        </div>
      </section>

      <section v-if="loading" class="rounded-2xl border border-[var(--border)] bg-[var(--card)] p-6 flex flex-col items-center justify-center gap-2 min-h-[220px]">
        <img :src="logoCrocLoading" alt="Loading conflict heatmap" class="w-14 h-14 object-contain" />
        <p class="text-xs text-[var(--muted-foreground)]">Scanning merge history for recurring conflict hotspots...</p>
      </section>

      <section v-else-if="error" class="rounded-2xl border border-[var(--destructive)]/40 bg-[var(--destructive)]/10 p-4 text-sm text-[var(--destructive)]">
        {{ error }}
      </section>

      <template v-else>
        <section class="rounded-2xl border border-[var(--border)] bg-[var(--card)] p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <input
              v-model="query"
              placeholder="Filter hotspots by path..."
              class="h-8 min-w-[240px] flex-1 max-w-[580px] px-3 rounded border border-[var(--border)] bg-[var(--input-background)] text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            />
            <div class="flex items-center gap-2">
              <button
                v-if="!loadAll"
                class="h-7 px-3 rounded-full text-[11px] border border-[var(--primary)]/35 text-[var(--primary)] hover:border-[var(--primary)]/65 hover:bg-[var(--primary)]/10 transition-colors"
                :disabled="loading"
                @click="enableLoadAll"
              >
                Load all
              </button>
              <span v-else class="text-[10px] px-2 py-0.5 rounded-full border border-[var(--primary)]/25 bg-[var(--primary)]/10 text-[var(--primary)]">All history loaded</span>
              <div class="text-[11px] text-[var(--muted-foreground)]">
              {{ filteredItems.length }} hotspot{{ filteredItems.length === 1 ? "" : "s" }}
            </div>
            </div>
          </div>
          <p class="mt-2 text-[10px] text-[var(--muted-foreground)]">
            {{ loadAll ? "Showing full hotspot history." : `Showing latest ${PREVIEW_CONFLICT_LIMIT} merge commits.` }}
          </p>
        </section>

        <section class="rounded-2xl border border-[var(--border)] bg-[var(--card)] p-3">
          <div v-if="filteredItems.length === 0" class="px-2 py-6 text-xs text-[var(--muted-foreground)] text-center">
            No conflict suspects for this filter.
          </div>

          <div v-else class="space-y-2 max-h-[560px] overflow-y-auto pr-1">
            <div
              v-for="item in filteredItems"
              :key="item.path"
              class="rounded-lg border px-3 py-2"
              :style="rowStyle(item.score)"
            >
              <div class="flex flex-wrap items-start justify-between gap-2">
                <div class="min-w-0 flex-1">
                  <div class="text-[11px] font-semibold text-[var(--foreground)] break-all">{{ item.path }}</div>
                  <div class="mt-1 text-[10px] text-[var(--muted-foreground)]">
                    merge touches: {{ item.merge_touches }}
                    <span class="mx-1">|</span>
                    conflict mentions: {{ item.conflict_mentions }}
                    <span class="mx-1">|</span>
                    score: {{ item.score }}
                  </div>
                </div>
                <div class="w-[140px] h-2 rounded-full bg-[var(--secondary)] overflow-hidden self-center">
                  <div class="h-full rounded-full" :style="scoreBarStyle(item.score)" />
                </div>
              </div>
            </div>
          </div>
        </section>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import logoCrocLoading from "@/assets/logo_croc_loading.gif";
import type { CommitInfo } from "@/types";

const FULL_HISTORY_LIMIT = 60000;
const PREVIEW_HISTORY_LIMIT = 300;
const WEEKDAY_LABELS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const loadingLetters = ["L", "o", "a", "d", "i", "n", "g"];

const props = defineProps<{
  repoPath: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

interface HeatCell {
  key: string;
  dateLabel: string;
  count: number;
}

interface BugKillerRow {
  author: string;
  deletions: number;
  commits: number;
}

type AuthorDeletionTuple = [string, number, number];

const historyCommits = ref<CommitInfo[]>([]);
const historyLoading = ref(false);
const historyError = ref("");
const bugKillers = ref<BugKillerRow[]>([]);
const bugKillerLoading = ref(false);
const bugKillerError = ref("");
const loadAllHistory = ref(false);
let loadRunToken = 0;

const activeHistoryLimit = computed(() =>
  loadAllHistory.value ? FULL_HISTORY_LIMIT : PREVIEW_HISTORY_LIMIT,
);

function dayKeyFromDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function dayKeyFromUnixTimestamp(timestamp: number): string {
  const value = Math.abs(timestamp) < 1000000000000 ? timestamp * 1000 : timestamp;
  return dayKeyFromDate(new Date(value));
}

function parseDayKey(dayKey: string): Date {
  const [year, month, day] = dayKey.split("-").map(Number);
  return new Date(year, (month || 1) - 1, day || 1);
}

function compareDayAsc(a: string, b: string): number {
  return a.localeCompare(b);
}

function compareDayDesc(a: string, b: string): number {
  return b.localeCompare(a);
}

function shiftDay(dayKey: string, offsetDays: number): string {
  const date = parseDayKey(dayKey);
  date.setDate(date.getDate() + offsetDays);
  return dayKeyFromDate(date);
}

const totalCommits = computed(() => historyCommits.value.length);

const uniqueAuthorNames = computed(() => {
  const values = new Set<string>();
  for (const commit of historyCommits.value) {
    const name = commit.author_name?.trim() || "Unknown";
    values.add(name);
  }
  return Array.from(values);
});

const uniqueAuthors = computed(() => uniqueAuthorNames.value.length);

const commitsPerDay = computed(() => {
  const map = new Map<string, number>();
  for (const commit of historyCommits.value) {
    const key = dayKeyFromUnixTimestamp(commit.timestamp);
    map.set(key, (map.get(key) ?? 0) + 1);
  }
  return map;
});

const sortedDaysDesc = computed(() =>
  Array.from(commitsPerDay.value.keys()).sort(compareDayDesc),
);

const currentStreak = computed(() => {
  const uniqueDays = new Set(sortedDaysDesc.value);
  const latest = sortedDaysDesc.value[0];
  if (!latest) return 0;

  let streak = 0;
  let cursor = latest;
  while (uniqueDays.has(cursor)) {
    streak += 1;
    cursor = shiftDay(cursor, -1);
  }
  return streak;
});

const longestStreak = computed(() => {
  const sortedAsc = Array.from(commitsPerDay.value.keys()).sort(compareDayAsc);
  if (sortedAsc.length === 0) return 0;

  let best = 1;
  let run = 1;

  for (let i = 1; i < sortedAsc.length; i += 1) {
    const previous = sortedAsc[i - 1];
    const current = sortedAsc[i];
    run = shiftDay(previous, 1) === current ? run + 1 : 1;
    if (run > best) {
      best = run;
    }
  }

  return best;
});

const activeDays = computed(() => commitsPerDay.value.size);

const hottestDay = computed(() => {
  let selectedDay = "";
  let selectedCount = 0;

  commitsPerDay.value.forEach((count, day) => {
    if (count > selectedCount) {
      selectedCount = count;
      selectedDay = day;
    }
  });

  return {
    day: selectedDay,
    count: selectedCount,
  };
});

const firstCommitTimestamp = computed(() => {
  if (historyCommits.value.length === 0) return null;
  return historyCommits.value.reduce((min, commit) => Math.min(min, commit.timestamp), Number.POSITIVE_INFINITY);
});

const latestCommitTimestamp = computed(() => {
  if (historyCommits.value.length === 0) return null;
  return historyCommits.value.reduce((max, commit) => Math.max(max, commit.timestamp), Number.NEGATIVE_INFINITY);
});

const repoAgeDays = computed(() => {
  const first = firstCommitTimestamp.value;
  const latest = latestCommitTimestamp.value;
  if (first === null || latest === null) return 0;
  const seconds = Math.max(0, latest - first);
  return Math.max(1, Math.round(seconds / 86400));
});

const averageCommitsPerDay = computed(() => {
  const days = activeDays.value;
  if (days === 0) return 0;
  return Math.round((totalCommits.value / days) * 10) / 10;
});

const mergeCommits = computed(() => historyCommits.value.filter((commit) => commit.parent_shas.length > 1).length);

const mergeCommitRatio = computed(() => {
  if (totalCommits.value === 0) return 0;
  return Math.round((mergeCommits.value / totalCommits.value) * 100);
});

const weekendCommits = computed(() => {
  let total = 0;
  for (const commit of historyCommits.value) {
    const value = Math.abs(commit.timestamp) < 1000000000000 ? commit.timestamp * 1000 : commit.timestamp;
    const day = new Date(value).getDay();
    if (day === 0 || day === 6) {
      total += 1;
    }
  }
  return total;
});

const weekendCommitRatio = computed(() => {
  if (totalCommits.value === 0) return 0;
  return Math.round((weekendCommits.value / totalCommits.value) * 100);
});

const hourlyCounts = computed(() => {
  const values = Array.from({ length: 24 }, () => 0);
  for (const commit of historyCommits.value) {
    const value = Math.abs(commit.timestamp) < 1000000000000 ? commit.timestamp * 1000 : commit.timestamp;
    const hour = new Date(value).getHours();
    values[hour] += 1;
  }
  return values;
});

const peakHour = computed(() => {
  let index = 0;
  let maxValue = 0;
  hourlyCounts.value.forEach((count, hour) => {
    if (count > maxValue) {
      maxValue = count;
      index = hour;
    }
  });
  return `${String(index).padStart(2, "0")}:00`;
});

const weekdayCounts = computed(() => {
  const counts = Array.from({ length: 7 }, () => 0);
  for (const commit of historyCommits.value) {
    const value = Math.abs(commit.timestamp) < 1000000000000 ? commit.timestamp * 1000 : commit.timestamp;
    const day = new Date(value).getDay();
    counts[day] += 1;
  }
  return counts;
});

const topWeekday = computed(() => {
  let index = 0;
  let maxValue = 0;
  weekdayCounts.value.forEach((count, day) => {
    if (count > maxValue) {
      maxValue = count;
      index = day;
    }
  });
  return WEEKDAY_LABELS[index];
});

const authorCommitCounts = computed(() => {
  const byAuthor = new Map<string, number>();
  for (const commit of historyCommits.value) {
    const key = commit.author_name?.trim() || "Unknown";
    byAuthor.set(key, (byAuthor.get(key) ?? 0) + 1);
  }
  return byAuthor;
});

const topContributor = computed(() => {
  let selected = "n/a";
  let maxValue = 0;

  authorCommitCounts.value.forEach((count, author) => {
    if (count > maxValue) {
      maxValue = count;
      selected = author;
    }
  });

  return {
    name: selected,
    commits: maxValue,
  };
});

const heatCells = computed<HeatCell[]>(() => {
  const cells: HeatCell[] = [];
  const now = new Date();
  const totalDays = 12 * 7;

  for (let offset = totalDays - 1; offset >= 0; offset -= 1) {
    const date = new Date(now);
    date.setHours(0, 0, 0, 0);
    date.setDate(now.getDate() - offset);

    const key = dayKeyFromDate(date);
    const count = commitsPerDay.value.get(key) ?? 0;

    cells.push({
      key,
      dateLabel: date.toLocaleDateString(undefined, {
        day: "2-digit",
        month: "short",
      }),
      count,
    });
  }

  return cells;
});

const heatWeeks = computed(() => {
  const weeks: HeatCell[][] = [];
  const cells = heatCells.value;
  for (let i = 0; i < cells.length; i += 7) {
    weeks.push(cells.slice(i, i + 7));
  }
  return weeks;
});

const maxHeat = computed(() => {
  const maxValue = heatCells.value.reduce((max, cell) => Math.max(max, cell.count), 0);
  return maxValue <= 0 ? 1 : maxValue;
});

function heatCellStyle(cell: HeatCell): Record<string, string> {
  if (cell.count <= 0) {
    return {
      background: "rgba(148, 163, 184, 0.12)",
      borderColor: "rgba(148, 163, 184, 0.2)",
    };
  }

  const ratio = cell.count / maxHeat.value;
  const alpha = Math.min(0.95, 0.2 + ratio * 0.75);
  return {
    background: `rgba(20, 184, 166, ${alpha})`,
    borderColor: `rgba(45, 212, 191, ${Math.min(1, alpha + 0.1)})`,
  };
}

const topBugKiller = computed(() => bugKillers.value[0] ?? null);
const maxKillerDeletes = computed(() => {
  const maxValue = bugKillers.value.reduce((max, row) => Math.max(max, row.deletions), 0);
  return maxValue <= 0 ? 1 : maxValue;
});

const productivityRank = computed(() => {
  const score = currentStreak.value * 3 + Math.floor(totalCommits.value / 20);
  if (score >= 70) return "Croc Legend";
  if (score >= 45) return "Swamp Commander";
  if (score >= 25) return "Bug Hunter";
  if (score >= 10) return "Momentum Builder";
  return "Rookie Explorer";
});

const arenaHealthScore = computed(() => {
  const streakPart = Math.min(40, currentStreak.value * 2);
  const mergePart = Math.min(20, Math.max(0, 20 - Math.abs(mergeCommitRatio.value - 18)));
  const activityPart = Math.min(40, Math.round(averageCommitsPerDay.value * 8));
  return Math.min(100, streakPart + mergePart + activityPart);
});

function normalizeBugKillerRows(rows: AuthorDeletionTuple[]): BugKillerRow[] {
  return rows
    .map(([author, deletions, commits]) => ({
      author: (author || "Unknown").trim() || "Unknown",
      deletions: Number(deletions) || 0,
      commits: Number(commits) || 0,
    }))
    .filter((row) => row.deletions > 0)
    .slice(0, 8);
}

async function loadArenaData() {
  loadRunToken += 1;
  const runToken = loadRunToken;
  const maxCount = activeHistoryLimit.value;

  if (!props.repoPath) {
    historyCommits.value = [];
    bugKillers.value = [];
    historyLoading.value = false;
    bugKillerLoading.value = false;
    historyError.value = "";
    bugKillerError.value = "";
    return;
  }

  historyLoading.value = true;
  bugKillerLoading.value = true;
  historyError.value = "";
  bugKillerError.value = "";

  const [commitsResult, killerResult] = await Promise.allSettled([
    invoke<CommitInfo[]>("get_commits", {
      path: props.repoPath,
      maxCount,
    }),
    invoke<AuthorDeletionTuple[]>("get_author_deletion_stats", {
      path: props.repoPath,
      maxCount,
    }),
  ]);

  if (runToken !== loadRunToken) {
    return;
  }

  if (commitsResult.status === "fulfilled") {
    historyCommits.value = commitsResult.value;
  } else {
    historyCommits.value = [];
    historyError.value = "Could not load full commit history.";
  }

  if (killerResult.status === "fulfilled") {
    bugKillers.value = normalizeBugKillerRows(killerResult.value);
  } else {
    bugKillers.value = [];
    bugKillerError.value = "Could not load full-history deletion stats.";
  }

  historyLoading.value = false;
  bugKillerLoading.value = false;
}

function enableLoadAllHistory() {
  if (loadAllHistory.value) return;
  loadAllHistory.value = true;
  void loadArenaData();
}

watch(
  () => props.repoPath,
  () => {
    loadAllHistory.value = false;
    void loadArenaData();
  },
  { immediate: true },
);
</script>

<template>
  <div class="flex-1 overflow-y-auto min-h-0 relative productivity-surface">
    <div class="absolute inset-0 pointer-events-none productivity-radial" />
    <div class="relative z-10 p-4 md:p-5 space-y-4">
      <section class="rounded-2xl border border-[#1f2937] bg-[#08151a]/88 backdrop-blur-sm p-4">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <p class="text-[10px] uppercase tracking-[0.18em] text-[#7dd3fc]/80">Visualise Commit History</p>
            <h2 class="text-lg md:text-xl font-bold text-[#d1fae5]">Swamp Productivity Arena</h2>
            <p class="text-xs text-[#9ca3af] mt-1">Gamified pulse of your repository rhythm from full history.</p>
          </div>
          <div class="flex items-center gap-2">
            <div class="px-3 py-1.5 rounded-full border border-[#14b8a6]/35 bg-[#0f2a2a]/70">
              <span class="text-[11px] font-semibold text-[#5eead4]">Rank: {{ productivityRank }}</span>
            </div>
            <button
              v-if="!loadAllHistory"
              class="arena-load-all"
              :disabled="historyLoading || bugKillerLoading"
              @click="enableLoadAllHistory"
            >
              Load all
            </button>
            <span v-else class="arena-mode-pill">All history loaded</span>
            <button
              class="arena-close"
              title="Back to Git Graph"
              @click="emit('close')"
            >
              x
            </button>
          </div>
        </div>
      </section>

      <section
        v-if="historyLoading"
        class="rounded-2xl border border-[#1f2937] bg-[#071118]/88 p-6 flex flex-col items-center justify-center gap-2 min-h-[220px]"
      >
        <img :src="logoCrocLoading" alt="Loading productivity arena" class="arena-loader-logo" />
        <div class="arena-loader-wave" aria-label="Loading">
          <span
            v-for="(letter, idx) in loadingLetters"
            :key="`arena-load-${idx}`"
            class="arena-loader-letter"
            :style="{ animationDelay: `${idx * 0.06}s` }"
          >
            {{ letter }}
          </span>
        </div>
        <p class="text-xs text-[#94a3b8]">
          {{ loadAllHistory ? "Crunching full history stats..." : `Crunching latest ${PREVIEW_HISTORY_LIMIT} commits...` }}
        </p>
      </section>

      <section
        v-else-if="historyError"
        class="rounded-2xl border border-[#7f1d1d] bg-[#2a1316]/85 p-4 text-sm text-[#fca5a5]"
      >
        {{ historyError }}
      </section>

      <template v-else>
        <section class="rounded-2xl border border-[#1f2937] bg-[#071118]/72 px-4 py-2.5">
          <p class="text-[11px] text-[#94a3b8]">
            {{ loadAllHistory ? "Showing full loaded history." : `Showing latest ${PREVIEW_HISTORY_LIMIT} commits.` }}
          </p>
        </section>

        <section class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
          <article class="metric-card">
            <div class="metric-label">Current streak</div>
            <div class="metric-value">{{ currentStreak }}</div>
            <div class="metric-foot">consecutive commit days</div>
          </article>
          <article class="metric-card">
            <div class="metric-label">Longest streak</div>
            <div class="metric-value">{{ longestStreak }}</div>
            <div class="metric-foot">best run in full history</div>
          </article>
          <article class="metric-card">
            <div class="metric-label">Total commits</div>
            <div class="metric-value">{{ totalCommits }}</div>
            <div class="metric-foot">loaded from full history</div>
          </article>
          <article class="metric-card">
            <div class="metric-label">Arena health</div>
            <div class="metric-value">{{ arenaHealthScore }}%</div>
            <div class="metric-foot">streak + merge + activity score</div>
          </article>
        </section>

        <section class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-5 gap-3">
          <article class="metric-card compact">
            <div class="metric-label">Unique authors</div>
            <div class="metric-mini">{{ uniqueAuthors }}</div>
          </article>
          <article class="metric-card compact">
            <div class="metric-label">Merge ratio</div>
            <div class="metric-mini">{{ mergeCommitRatio }}%</div>
          </article>
          <article class="metric-card compact">
            <div class="metric-label">Weekend commits</div>
            <div class="metric-mini">{{ weekendCommitRatio }}%</div>
          </article>
          <article class="metric-card compact">
            <div class="metric-label">Peak coding hour</div>
            <div class="metric-mini">{{ peakHour }}</div>
          </article>
          <article class="metric-card compact">
            <div class="metric-label">Top weekday</div>
            <div class="metric-mini">{{ topWeekday }}</div>
          </article>
        </section>

        <section class="rounded-2xl border border-[#1f2937] bg-[#071118]/85 p-4">
          <div class="flex flex-wrap items-center justify-between gap-2 mb-3">
            <h3 class="text-sm font-semibold text-[#e2e8f0]">Consistency and Momentum</h3>
            <span class="text-[10px] text-[#94a3b8]">{{ repoAgeDays }} day repository age</span>
          </div>
          <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
            <div class="momentum-card">
              <div class="momentum-label">Active days</div>
              <div class="momentum-value">{{ activeDays }}</div>
            </div>
            <div class="momentum-card">
              <div class="momentum-label">Average commits/day</div>
              <div class="momentum-value">{{ averageCommitsPerDay }}</div>
            </div>
            <div class="momentum-card">
              <div class="momentum-label">Top contributor</div>
              <div class="momentum-value text-sm truncate">{{ topContributor.name }}</div>
              <div class="momentum-sub">{{ topContributor.commits }} commits</div>
            </div>
            <div class="momentum-card">
              <div class="momentum-label">Hottest day</div>
              <div class="momentum-value">{{ hottestDay.count }}</div>
              <div class="momentum-sub">{{ hottestDay.day || "n/a" }}</div>
            </div>
          </div>
        </section>

        <section class="rounded-2xl border border-[#1f2937] bg-[#071118]/85 p-4">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-[#e2e8f0]">Activity Heat Map</h3>
            <span class="text-[10px] text-[#94a3b8]">Last 12 weeks</span>
          </div>

          <div class="overflow-x-auto">
            <div class="inline-flex gap-1 min-w-[420px]">
              <div
                v-for="(week, weekIndex) in heatWeeks"
                :key="`week-${weekIndex}`"
                class="grid grid-rows-7 gap-1"
              >
                <div
                  v-for="cell in week"
                  :key="cell.key"
                  class="w-3.5 h-3.5 rounded-[4px] border transition-transform hover:scale-110"
                  :style="heatCellStyle(cell)"
                  :title="`${cell.dateLabel}: ${cell.count} commit${cell.count === 1 ? '' : 's'}`"
                />
              </div>
            </div>
          </div>
        </section>

        <section class="rounded-2xl border border-[#1f2937] bg-[#081118]/86 p-4">
          <div class="flex flex-wrap items-center justify-between gap-2 mb-3">
            <h3 class="text-sm font-semibold text-[#e2e8f0]">Bug Killer Leaderboard</h3>
            <span class="text-[10px] text-[#94a3b8]">Deletion stats across loaded history window</span>
          </div>

          <div v-if="bugKillerLoading" class="flex items-center gap-2 text-xs text-[#5eead4]">
            <img :src="logoCrocLoading" alt="Loading deletions" class="mini-loader-logo" />
            {{ loadAllHistory ? "Loading full-history deletion stats..." : `Loading latest ${PREVIEW_HISTORY_LIMIT} commit stats...` }}
          </div>
          <div v-else-if="bugKillerError" class="text-xs text-[#f87171]">{{ bugKillerError }}</div>
          <div v-else-if="bugKillers.length === 0" class="text-xs text-[#94a3b8]">No deletion-heavy commits found yet.</div>
          <div v-else class="space-y-2">
            <div class="rounded-xl border border-[#14b8a6]/30 bg-[#0f172a]/65 px-3 py-2.5">
              <div class="text-[10px] uppercase tracking-[0.14em] text-[#5eead4]/85">Top Slayer</div>
              <div class="mt-1 text-sm font-semibold text-[#d1fae5]">{{ topBugKiller?.author }}</div>
              <div class="text-[11px] text-[#9ca3af]">{{ topBugKiller?.deletions }} lines removed across {{ topBugKiller?.commits }} commits</div>
            </div>

            <div
              v-for="row in bugKillers"
              :key="row.author"
              class="rounded-xl border border-[#1f2937] bg-[#0b1520]/75 px-3 py-2"
            >
              <div class="flex items-center justify-between gap-2 text-[11px] mb-1.5">
                <span class="font-medium text-[#e2e8f0] truncate">{{ row.author }}</span>
                <span class="text-[#5eead4] font-semibold">{{ row.deletions }} del</span>
              </div>
              <div class="h-1.5 rounded-full bg-[#1f2937] overflow-hidden">
                <div
                  class="h-full rounded-full bg-gradient-to-r from-[#14b8a6] to-[#06b6d4]"
                  :style="{ width: `${Math.max(8, Math.round((row.deletions / maxKillerDeletes) * 100))}%` }"
                />
              </div>
            </div>
          </div>
        </section>
      </template>
    </div>
  </div>
</template>

<style scoped>
.productivity-surface {
  background:
    radial-gradient(circle at 15% 18%, rgba(20, 184, 166, 0.12), transparent 45%),
    radial-gradient(circle at 82% 12%, rgba(14, 116, 144, 0.16), transparent 40%),
    linear-gradient(145deg, rgba(2, 6, 23, 0.96) 0%, rgba(3, 12, 20, 0.94) 52%, rgba(4, 16, 24, 0.97) 100%);
}

.productivity-radial {
  background-image: repeating-linear-gradient(
    125deg,
    transparent 0,
    transparent 14px,
    rgba(20, 184, 166, 0.035) 14px,
    rgba(20, 184, 166, 0.035) 15px
  );
}

.metric-card {
  border: 1px solid rgba(31, 41, 55, 0.9);
  border-radius: 0.9rem;
  background: rgba(8, 18, 28, 0.84);
  padding: 0.8rem 0.9rem;
}

.metric-card.compact {
  padding: 0.7rem 0.8rem;
}

.metric-label {
  font-size: 10px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: rgba(148, 163, 184, 0.9);
}

.metric-value {
  margin-top: 0.28rem;
  font-size: 1.4rem;
  line-height: 1;
  font-weight: 700;
  color: #d1fae5;
}

.metric-foot {
  margin-top: 0.22rem;
  font-size: 11px;
  color: #94a3b8;
}

.metric-mini {
  margin-top: 0.22rem;
  font-size: 1.02rem;
  font-weight: 700;
  color: #d1fae5;
}

.momentum-card {
  border: 1px solid rgba(51, 65, 85, 0.88);
  border-radius: 0.75rem;
  background: rgba(15, 23, 42, 0.72);
  padding: 0.65rem 0.75rem;
}

.momentum-label {
  font-size: 10px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: #94a3b8;
}

.momentum-value {
  margin-top: 0.2rem;
  color: #e2e8f0;
  font-size: 1rem;
  font-weight: 700;
}

.momentum-sub {
  margin-top: 0.14rem;
  font-size: 11px;
  color: #94a3b8;
}

.arena-close {
  width: 28px;
  height: 28px;
  border-radius: 999px;
  border: 1px solid rgba(51, 65, 85, 0.9);
  background: rgba(15, 23, 42, 0.75);
  color: #cbd5e1;
  font-size: 12px;
  font-weight: 700;
  line-height: 1;
  text-transform: uppercase;
}

.arena-load-all {
  height: 28px;
  border-radius: 999px;
  border: 1px solid rgba(94, 234, 212, 0.35);
  background: rgba(15, 42, 42, 0.62);
  color: #5eead4;
  font-size: 11px;
  font-weight: 600;
  padding: 0 10px;
}

.arena-load-all:hover {
  border-color: rgba(94, 234, 212, 0.7);
  background: rgba(20, 78, 78, 0.45);
}

.arena-load-all:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.arena-mode-pill {
  height: 28px;
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  border: 1px solid rgba(45, 212, 191, 0.28);
  background: rgba(8, 28, 28, 0.62);
  color: #99f6e4;
  font-size: 11px;
  font-weight: 600;
  padding: 0 10px;
}

.arena-close:hover {
  color: #f8fafc;
  border-color: rgba(248, 113, 113, 0.8);
  background: rgba(127, 29, 29, 0.5);
}

.arena-loader-logo {
  width: 54px;
  height: 54px;
  object-fit: contain;
  filter: drop-shadow(0 0 7px rgba(45, 212, 191, 0.38));
}

.mini-loader-logo {
  width: 18px;
  height: 18px;
  object-fit: contain;
}

.arena-loader-wave {
  display: inline-flex;
  gap: 0.5px;
}

.arena-loader-letter {
  font-size: 12px;
  font-weight: 700;
  color: #5eead4;
  text-transform: uppercase;
  animation: arena-loader-bounce 1s ease-in-out infinite;
}

@keyframes arena-loader-bounce {
  0%,
  50%,
  100% {
    transform: translateY(0);
    opacity: 0.5;
  }
  25% {
    transform: translateY(-3px);
    opacity: 1;
  }
}
</style>

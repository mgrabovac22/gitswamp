<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import logoCrocLoading from "@/assets/logo_croc_loading.gif";
import type { CommitInfo, ConflictHotspot } from "@/types";

const FULL_HISTORY_LIMIT = 60000;
const PREVIEW_HISTORY_LIMIT = 300;
const WEEKDAY_LABELS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const REGRESSION_SIGNAL_RE = /\b(regression|regress|bug|fix|hotfix|rollback|revert)\b/i;
const loadingLetters = "LOADING".split("");

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
const conflictHotspots = ref<ConflictHotspot[]>([]);
const conflictLoading = ref(false);
const conflictError = ref("");
const loadAllHistory = ref(false);
const selectedAuthor = ref("all");

const commitCache = new Map<string, CommitInfo[]>();
const killerCache = new Map<string, BugKillerRow[]>();
const conflictCache = new Map<string, ConflictHotspot[]>();

let loadRunToken = 0;

const activeHistoryLimit = computed(() =>
  loadAllHistory.value ? FULL_HISTORY_LIMIT : PREVIEW_HISTORY_LIMIT,
);

const firstArenaError = computed(
  () => historyError.value || bugKillerError.value || conflictError.value,
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

function startOfDay(date: Date): Date {
  const value = new Date(date);
  value.setHours(0, 0, 0, 0);
  return value;
}

const uniqueAuthorNames = computed(() => {
  const values = new Set<string>();
  for (const commit of historyCommits.value) {
    const name = commit.author_name?.trim() || "Unknown";
    values.add(name);
  }
  return Array.from(values).sort((a, b) => a.localeCompare(b));
});

const filteredHistoryCommits = computed(() => {
  if (selectedAuthor.value === "all") {
    return historyCommits.value;
  }

  return historyCommits.value.filter(
    (commit) => (commit.author_name?.trim() || "Unknown") === selectedAuthor.value,
  );
});

const totalCommits = computed(() => filteredHistoryCommits.value.length);

const uniqueAuthors = computed(() => {
  const values = new Set<string>();
  for (const commit of filteredHistoryCommits.value) {
    const name = commit.author_name?.trim() || "Unknown";
    values.add(name);
  }
  return values.size;
});

const commitsPerDay = computed(() => {
  const map = new Map<string, number>();
  for (const commit of filteredHistoryCommits.value) {
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
  if (filteredHistoryCommits.value.length === 0) return null;
  return filteredHistoryCommits.value.reduce((min, commit) => Math.min(min, commit.timestamp), Number.POSITIVE_INFINITY);
});

const latestCommitTimestamp = computed(() => {
  if (filteredHistoryCommits.value.length === 0) return null;
  return filteredHistoryCommits.value.reduce((max, commit) => Math.max(max, commit.timestamp), Number.NEGATIVE_INFINITY);
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

const mergeCommits = computed(() => filteredHistoryCommits.value.filter((commit) => commit.parent_shas.length > 1).length);

const mergeCommitRatio = computed(() => {
  if (totalCommits.value === 0) return 0;
  return Math.round((mergeCommits.value / totalCommits.value) * 100);
});

const weekendCommits = computed(() => {
  let total = 0;
  for (const commit of filteredHistoryCommits.value) {
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

const offHoursCommits = computed(() => {
  let total = 0;
  for (const commit of filteredHistoryCommits.value) {
    const value = Math.abs(commit.timestamp) < 1000000000000 ? commit.timestamp * 1000 : commit.timestamp;
    const hour = new Date(value).getHours();
    if (hour < 8 || hour >= 19) {
      total += 1;
    }
  }
  return total;
});

const offHoursCommitRatio = computed(() => {
  if (totalCommits.value === 0) return 0;
  return Math.round((offHoursCommits.value / totalCommits.value) * 100);
});

const hourlyCounts = computed(() => {
  const values = Array.from({ length: 24 }, () => 0);
  for (const commit of filteredHistoryCommits.value) {
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
  for (const commit of filteredHistoryCommits.value) {
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

const top3DayLoadShare = computed(() => {
  if (totalCommits.value === 0) return 0;
  const dailyLoads = Array.from(commitsPerDay.value.values()).sort((a, b) => b - a);
  const topLoad = dailyLoads.slice(0, 3).reduce((sum, count) => sum + count, 0);
  return Math.round((topLoad / totalCommits.value) * 100);
});

const throughputVolatility = computed(() => {
  const dailyLoads = Array.from(commitsPerDay.value.values());
  if (dailyLoads.length === 0) return 0;
  const average = dailyLoads.reduce((sum, count) => sum + count, 0) / dailyLoads.length;
  const variance = dailyLoads.reduce((sum, count) => sum + (count - average) ** 2, 0) / dailyLoads.length;
  return Math.round(Math.sqrt(variance) * 10) / 10;
});

const authorCommitCounts = computed(() => {
  const byAuthor = new Map<string, number>();
  for (const commit of filteredHistoryCommits.value) {
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

const topContributorShare = computed(() => {
  if (totalCommits.value === 0) return 0;
  return Math.round((topContributor.value.commits / totalCommits.value) * 100);
});

const contributionBalanceScore = computed(() => {
  const values = Array.from(authorCommitCounts.value.values());
  if (values.length <= 1) return 100;

  const total = values.reduce((sum, count) => sum + count, 0);
  if (total <= 0) return 100;

  let entropy = 0;
  for (const count of values) {
    const p = count / total;
    entropy += -p * Math.log2(p);
  }

  const maxEntropy = Math.log2(values.length);
  if (maxEntropy <= 0) return 100;
  return Math.round((entropy / maxEntropy) * 100);
});

const heatCells = computed<HeatCell[]>(() => {
  const cells: HeatCell[] = [];
  const today = startOfDay(new Date());
  const rangeStart = startOfDay(today);
  rangeStart.setMonth(rangeStart.getMonth() - 12);
  rangeStart.setDate(rangeStart.getDate() + 1);

  const alignedStart = startOfDay(rangeStart);
  alignedStart.setDate(alignedStart.getDate() - alignedStart.getDay());

  const alignedEnd = startOfDay(today);
  alignedEnd.setDate(alignedEnd.getDate() + (6 - alignedEnd.getDay()));

  const cursor = new Date(alignedStart);
  while (cursor <= alignedEnd) {
    const key = dayKeyFromDate(cursor);
    const inWindow = cursor >= rangeStart && cursor <= today;

    cells.push({
      key,
      dateLabel: cursor.toLocaleDateString(undefined, {
        day: "2-digit",
        month: "short",
      }),
      count: inWindow ? (commitsPerDay.value.get(key) ?? 0) : 0,
    });

    cursor.setDate(cursor.getDate() + 1);
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

const displayedBugKillers = computed(() => {
  if (selectedAuthor.value === "all") {
    return bugKillers.value;
  }

  return bugKillers.value.filter((row) => row.author === selectedAuthor.value);
});

const leaderboardRows = computed(() => displayedBugKillers.value.slice(0, 8));

const topBugKiller = computed(() => leaderboardRows.value[0] ?? null);
const maxKillerDeletes = computed(() => {
  const maxValue = leaderboardRows.value.reduce((max, row) => Math.max(max, row.deletions), 0);
  return maxValue <= 0 ? 1 : maxValue;
});

const totalDeletedLines = computed(() =>
  displayedBugKillers.value.reduce((sum, row) => sum + row.deletions, 0),
);

const deletionsPerCommit = computed(() => {
  if (totalCommits.value === 0) return 0;
  return Math.round(totalDeletedLines.value / totalCommits.value);
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

const regressionSignalCommits = computed(() =>
  filteredHistoryCommits.value.filter((commit) => REGRESSION_SIGNAL_RE.test(commit.message || "")).length,
);

const regressionSignalRate = computed(() => {
  if (totalCommits.value === 0) return 0;
  return Math.round((regressionSignalCommits.value / totalCommits.value) * 100);
});

const totalConflictMentions = computed(() =>
  conflictHotspots.value.reduce((sum, item) => sum + item.conflict_mentions, 0),
);

const totalConflictMergeTouches = computed(() =>
  conflictHotspots.value.reduce((sum, item) => sum + item.merge_touches, 0),
);

const conflictMentionDensity = computed(() => {
  if (totalConflictMergeTouches.value === 0) return 0;
  return Math.round((totalConflictMentions.value / totalConflictMergeTouches.value) * 100);
});

const highRiskConflictFiles = computed(() => {
  if (conflictHotspots.value.length === 0) return 0;
  const maxScore = conflictHotspots.value.reduce((best, item) => Math.max(best, item.score), 0);
  const threshold = maxScore * 0.65;
  return conflictHotspots.value.filter((item) => item.score >= threshold).length;
});

const collaborationIntensityScore = computed(() => {
  const authorPart = Math.min(35, uniqueAuthors.value * 6);
  const mergePart = Math.min(35, mergeCommitRatio.value);
  const activityPart = Math.min(30, Math.round(averageCommitsPerDay.value * 9));
  return Math.min(100, authorPart + mergePart + activityPart);
});

const bottleneckScore = computed(() => {
  const burstPart = Math.min(35, Math.round(top3DayLoadShare.value * 0.35));
  const ownershipPart = Math.min(30, Math.round(topContributorShare.value * 0.3));
  const offHoursPart = Math.min(20, Math.round(offHoursCommitRatio.value * 0.2));
  const volatilityPart = Math.min(15, Math.round(throughputVolatility.value * 3));
  return Math.min(100, burstPart + ownershipPart + offHoursPart + volatilityPart);
});

const stabilityRiskScore = computed(() => {
  const regressionPart = Math.min(35, Math.round(regressionSignalRate.value * 0.6));
  const conflictPart = Math.min(35, Math.round(conflictMentionDensity.value * 0.8));
  const bottleneckPart = Math.min(30, Math.round(bottleneckScore.value * 0.3));
  return Math.min(100, regressionPart + conflictPart + bottleneckPart);
});

const mergePerActiveDay = computed(() => {
  if (activeDays.value === 0) return 0;
  return Math.round((mergeCommits.value / activeDays.value) * 100) / 100;
});

const busFactorRisk = computed(() => Math.min(100, topContributorShare.value));

const recoveryPressure = computed(() =>
  Math.min(100, Math.round(regressionSignalRate.value * 0.55 + conflictMentionDensity.value * 0.45)),
);

const contextSwitchPressure = computed(() =>
  Math.min(100, Math.round(offHoursCommitRatio.value * 0.42 + top3DayLoadShare.value * 0.58)),
);

function scoreBand(score: number): string {
  if (score >= 75) return "Critical";
  if (score >= 55) return "High";
  if (score >= 30) return "Moderate";
  return "Low";
}

function scoreBandStyle(score: number): Record<string, string> {
  if (score >= 75) {
    return {
      borderColor: "rgba(239, 68, 68, 0.72)",
      color: "rgba(254, 202, 202, 1)",
      background: "rgba(127, 29, 29, 0.45)",
    };
  }

  if (score >= 55) {
    return {
      borderColor: "rgba(251, 146, 60, 0.62)",
      color: "rgba(254, 215, 170, 1)",
      background: "rgba(124, 45, 18, 0.38)",
    };
  }

  if (score >= 30) {
    return {
      borderColor: "rgba(250, 204, 21, 0.55)",
      color: "rgba(254, 243, 199, 1)",
      background: "rgba(113, 63, 18, 0.34)",
    };
  }

  return {
    borderColor: "rgba(74, 222, 128, 0.45)",
    color: "rgba(187, 247, 208, 1)",
    background: "rgba(20, 83, 45, 0.34)",
  };
}

function scoreMeterStyle(score: number): Record<string, string> {
  return {
    width: `${Math.max(4, Math.min(100, Math.round(score)))}%`,
    background: `color-mix(in srgb, var(--primary) ${Math.min(90, 25 + Math.round(score * 0.6))}%, var(--chart-2))`,
  };
}

function normalizeBugKillerRows(rows: AuthorDeletionTuple[]): BugKillerRow[] {
  return rows
    .map(([author, deletions, commits]) => ({
      author: (author || "Unknown").trim() || "Unknown",
      deletions: Number(deletions) || 0,
      commits: Number(commits) || 0,
    }))
    .filter((row) => row.deletions > 0);
}

async function loadArenaData() {
  loadRunToken += 1;
  const runToken = loadRunToken;
  const maxCount = activeHistoryLimit.value;

  if (!props.repoPath) {
    resetArenaState();
    return;
  }

  startArenaLoading();

  void loadCommitsForRun(runToken, maxCount);
  void loadBugKillerStatsForRun(runToken, maxCount);
  void loadConflictSignalsForRun(runToken, maxCount);
}

function resetArenaState() {
  historyCommits.value = [];
  bugKillers.value = [];
  conflictHotspots.value = [];
  historyLoading.value = false;
  bugKillerLoading.value = false;
  conflictLoading.value = false;
  historyError.value = "";
  bugKillerError.value = "";
  conflictError.value = "";
}

function startArenaLoading() {
  historyLoading.value = true;
  bugKillerLoading.value = true;
  conflictLoading.value = true;
  historyError.value = "";
  bugKillerError.value = "";
  conflictError.value = "";
}

function isRunActive(runToken: number): boolean {
  return runToken === loadRunToken;
}

async function loadCommitsForRun(runToken: number, maxCount: number) {
  const cacheKey = `${props.repoPath}::${maxCount}`;

  const cachedCommits = commitCache.get(cacheKey);
  if (cachedCommits) {
    historyCommits.value = cachedCommits;
    historyLoading.value = false;
    historyError.value = "";
    return;
  }

  try {
    const commits = await invoke<CommitInfo[]>("get_commits", {
      path: props.repoPath,
      maxCount,
    });
    if (!isRunActive(runToken)) return;
    historyCommits.value = commits;
    commitCache.set(cacheKey, commits);
  } catch {
    if (!isRunActive(runToken)) return;
    historyCommits.value = [];
    historyError.value = "Could not load full commit history.";
  } finally {
    if (isRunActive(runToken)) {
      historyLoading.value = false;
    }
  }
}

async function loadBugKillerStatsForRun(runToken: number, maxCount: number) {
  const cacheKey = `${props.repoPath}::${maxCount}`;

  const cachedRows = killerCache.get(cacheKey);
  if (cachedRows) {
    bugKillers.value = cachedRows;
    bugKillerLoading.value = false;
    bugKillerError.value = "";
    return;
  }

  try {
    const killerRows = await invoke<AuthorDeletionTuple[]>("get_author_deletion_stats", {
      path: props.repoPath,
      maxCount,
    });
    if (!isRunActive(runToken)) return;
    const normalizedRows = normalizeBugKillerRows(killerRows);
    bugKillers.value = normalizedRows;
    killerCache.set(cacheKey, normalizedRows);
  } catch {
    if (!isRunActive(runToken)) return;
    bugKillers.value = [];
    bugKillerError.value = "Could not load full-history deletion stats.";
  } finally {
    if (isRunActive(runToken)) {
      bugKillerLoading.value = false;
    }
  }
}

async function loadConflictSignalsForRun(runToken: number, maxCount: number) {
  const cacheKey = `${props.repoPath}::${maxCount}`;

  const cachedRows = conflictCache.get(cacheKey);
  if (cachedRows) {
    conflictHotspots.value = cachedRows;
    conflictLoading.value = false;
    conflictError.value = "";
    return;
  }

  try {
    const conflictRows = await invoke<ConflictHotspot[]>("get_conflict_hotspots", {
      path: props.repoPath,
      maxCount,
    });
    if (!isRunActive(runToken)) return;
    conflictHotspots.value = conflictRows;
    conflictCache.set(cacheKey, conflictRows);
  } catch {
    if (!isRunActive(runToken)) return;
    conflictHotspots.value = [];
    conflictError.value = "Could not load collaboration conflict pressure signals.";
  } finally {
    if (isRunActive(runToken)) {
      conflictLoading.value = false;
    }
  }
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
    selectedAuthor.value = "all";
    void loadArenaData();
  },
  { immediate: true },
);

watch(uniqueAuthorNames, (authorNames) => {
  if (selectedAuthor.value === "all") return;
  if (!authorNames.includes(selectedAuthor.value)) {
    selectedAuthor.value = "all";
  }
});
</script>

<template>
  <div class="flex-1 overflow-y-auto min-h-0 relative productivity-surface">
    <div class="absolute inset-0 pointer-events-none productivity-radial" />
    <div class="relative z-10 p-4 md:p-5 space-y-4">
      <section class="rounded-lg border border-[var(--border)] bg-[var(--card)]/90 backdrop-blur-sm p-4">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <p class="text-[10px] uppercase tracking-[0.18em] text-[var(--muted-foreground)]">Visualise Commit History</p>
            <h2 class="text-lg md:text-xl font-bold text-[var(--foreground)]">Swamp Productivity Arena</h2>
            <p class="text-xs text-[var(--muted-foreground)] mt-1">Gamified pulse of your repository rhythm from full history.</p>
          </div>
          <div class="flex items-center gap-2 flex-wrap justify-end">
            <div class="arena-author-filter">
              <label for="arena-author-filter">Author</label>
              <select id="arena-author-filter" v-model="selectedAuthor">
                <option value="all">All contributors</option>
                <option
                  v-for="author in uniqueAuthorNames"
                  :key="`author-${author}`"
                  :value="author"
                >
                  {{ author }}
                </option>
              </select>
            </div>
            <div class="px-3 py-1.5 rounded-md border border-[var(--primary)]/35 bg-[var(--secondary)] min-w-[150px] text-center">
              <span class="text-[11px] font-semibold text-[var(--primary)]">Rank: {{ productivityRank }}</span>
            </div>
            <button
              v-if="!loadAllHistory"
              class="arena-load-all"
              :disabled="historyLoading || bugKillerLoading || conflictLoading"
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

      <section class="rounded-lg border border-[var(--border)] bg-[var(--card)]/78 px-4 py-2.5 space-y-2">
        <p class="text-[11px] text-[var(--muted-foreground)]">
          {{
            selectedAuthor === "all"
              ? (loadAllHistory ? "Showing full loaded history." : `Showing latest ${PREVIEW_HISTORY_LIMIT} commits.`)
              : `Filtered by ${selectedAuthor}.`
          }}
        </p>
        <p v-if="firstArenaError" class="text-xs text-[var(--destructive)]">{{ firstArenaError }}</p>
      </section>

      <section class="section-shell grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
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
            <div class="metric-foot">within active author filter</div>
          </article>
          <article class="metric-card">
            <div class="metric-label">Arena health</div>
            <div class="metric-value">{{ arenaHealthScore }}%</div>
            <div class="metric-foot">streak + merge + activity score</div>
          </article>
        <div v-if="historyLoading" class="section-loader-overlay">
          <img :src="logoCrocLoading" alt="Loading headline metrics" class="section-loader-logo" />
          <div class="section-loader-text">Loading headline metrics</div>
          <div class="loader-wave" aria-hidden="true">
            <span
              v-for="(letter, index) in loadingLetters"
              :key="`headline-loader-${index}`"
              class="loader-letter"
              :style="{ animationDelay: `${index * 0.08}s` }"
            >
              {{ letter }}
            </span>
          </div>
        </div>
      </section>

      <section class="section-shell grid grid-cols-1 md:grid-cols-2 xl:grid-cols-5 gap-3">
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
        <div v-if="historyLoading" class="section-loader-overlay">
          <img :src="logoCrocLoading" alt="Loading team metrics" class="section-loader-logo" />
          <div class="section-loader-text">Loading team metrics</div>
          <div class="loader-wave" aria-hidden="true">
            <span
              v-for="(letter, index) in loadingLetters"
              :key="`team-loader-${index}`"
              class="loader-letter"
              :style="{ animationDelay: `${index * 0.08}s` }"
            >
              {{ letter }}
            </span>
          </div>
        </div>
      </section>

      <section class="grid grid-cols-1 xl:grid-cols-3 gap-3">
          <article class="section-shell focus-card">
            <div class="focus-kicker">Flow Metrics</div>
            <h3 class="focus-title">Commit rhythm and change volume</h3>
            <div class="focus-metrics-grid">
              <div class="focus-metric">
                <div class="focus-metric-label">Commits/day</div>
                <div class="focus-metric-value">{{ averageCommitsPerDay }}</div>
              </div>
              <div class="focus-metric">
                <div class="focus-metric-label">Deleted lines</div>
                <div class="focus-metric-value">{{ totalDeletedLines }}</div>
              </div>
              <div class="focus-metric">
                <div class="focus-metric-label">Delete intensity</div>
                <div class="focus-metric-value">{{ deletionsPerCommit }}</div>
              </div>
              <div class="focus-metric">
                <div class="focus-metric-label">Balance score</div>
                <div class="focus-metric-value">{{ contributionBalanceScore }}%</div>
              </div>
            </div>
            <div class="focus-foot">
              Top contributor share: {{ topContributorShare }}%
            </div>
            <div v-if="historyLoading || bugKillerLoading" class="section-loader-overlay section-loader-overlay-card">
              <img :src="logoCrocLoading" alt="Loading flow metrics" class="section-loader-logo" />
              <div class="section-loader-text">Loading flow metrics</div>
              <div class="loader-wave" aria-hidden="true">
                <span
                  v-for="(letter, index) in loadingLetters"
                  :key="`flow-loader-${index}`"
                  class="loader-letter"
                  :style="{ animationDelay: `${index * 0.08}s` }"
                >
                  {{ letter }}
                </span>
              </div>
            </div>
          </article>

          <article class="section-shell focus-card">
            <div class="focus-kicker">Delivery Pressure</div>
            <h3 class="focus-title">Cycle pressure model</h3>
            <div class="focus-score-row">
              <span class="focus-score-label">Bottleneck index</span>
              <span class="score-pill" :style="scoreBandStyle(bottleneckScore)">{{ scoreBand(bottleneckScore) }}</span>
              <span class="focus-score-value">{{ bottleneckScore }}%</span>
            </div>
            <div class="focus-meter"><div class="focus-meter-fill" :style="scoreMeterStyle(bottleneckScore)" /></div>
            <div class="focus-list">
              <div>Top 3 day load share: <strong>{{ top3DayLoadShare }}%</strong></div>
              <div>Off-hours commits: <strong>{{ offHoursCommitRatio }}%</strong></div>
              <div>Throughput volatility: <strong>{{ throughputVolatility }}</strong></div>
              <div>Ownership concentration: <strong>{{ topContributorShare }}%</strong></div>
            </div>
            <div v-if="historyLoading" class="section-loader-overlay section-loader-overlay-card">
              <img :src="logoCrocLoading" alt="Loading delivery pressure" class="section-loader-logo" />
              <div class="section-loader-text">Loading delivery pressure</div>
              <div class="loader-wave" aria-hidden="true">
                <span
                  v-for="(letter, index) in loadingLetters"
                  :key="`pressure-loader-${index}`"
                  class="loader-letter"
                  :style="{ animationDelay: `${index * 0.08}s` }"
                >
                  {{ letter }}
                </span>
              </div>
            </div>
          </article>

          <article class="section-shell focus-card">
            <div class="focus-kicker">Stability Pulse</div>
            <h3 class="focus-title">Intensity connected to regression signals</h3>
            <div class="focus-score-row">
              <span class="focus-score-label">Stability risk</span>
              <span class="score-pill" :style="scoreBandStyle(stabilityRiskScore)">{{ scoreBand(stabilityRiskScore) }}</span>
              <span class="focus-score-value">{{ stabilityRiskScore }}%</span>
            </div>
            <div class="focus-meter"><div class="focus-meter-fill" :style="scoreMeterStyle(stabilityRiskScore)" /></div>
            <div class="focus-list">
              <div>Collaboration intensity: <strong>{{ collaborationIntensityScore }}%</strong></div>
              <div>Regression-signal commits: <strong>{{ regressionSignalCommits }} ({{ regressionSignalRate }}%)</strong></div>
              <div>Conflict mention density: <strong>{{ conflictMentionDensity }}%</strong></div>
              <div>High-risk conflict files: <strong>{{ highRiskConflictFiles }}</strong></div>
            </div>
            <div v-if="conflictError" class="focus-foot text-[var(--destructive)]">{{ conflictError }}</div>
            <div v-if="historyLoading || conflictLoading" class="section-loader-overlay section-loader-overlay-card">
              <img :src="logoCrocLoading" alt="Loading stability pulse" class="section-loader-logo" />
              <div class="section-loader-text">Loading stability pulse</div>
              <div class="loader-wave" aria-hidden="true">
                <span
                  v-for="(letter, index) in loadingLetters"
                  :key="`stability-loader-${index}`"
                  class="loader-letter"
                  :style="{ animationDelay: `${index * 0.08}s` }"
                >
                  {{ letter }}
                </span>
              </div>
            </div>
          </article>
        </section>

      <section class="section-shell rounded-lg border border-[var(--border)] bg-[var(--card)]/90 p-4">
          <div class="flex flex-wrap items-center justify-between gap-2 mb-3">
            <h3 class="text-sm font-semibold text-[var(--foreground)]">Engineering Diagnostics</h3>
            <span class="text-[10px] text-[var(--muted-foreground)]">Signals for process improvement</span>
          </div>
          <div class="grid grid-cols-2 xl:grid-cols-4 gap-3">
            <div class="momentum-card">
              <div class="momentum-label">Bus factor risk</div>
              <div class="momentum-value">{{ busFactorRisk }}%</div>
              <div class="momentum-sub">ownership concentration</div>
            </div>
            <div class="momentum-card">
              <div class="momentum-label">Merge/day</div>
              <div class="momentum-value">{{ mergePerActiveDay }}</div>
              <div class="momentum-sub">integration density</div>
            </div>
            <div class="momentum-card">
              <div class="momentum-label">Recovery pressure</div>
              <div class="momentum-value">{{ recoveryPressure }}%</div>
              <div class="momentum-sub">regression + conflict blend</div>
            </div>
            <div class="momentum-card">
              <div class="momentum-label">Context switch pressure</div>
              <div class="momentum-value">{{ contextSwitchPressure }}%</div>
              <div class="momentum-sub">bursts + off-hours work</div>
            </div>
          </div>
        <div v-if="historyLoading || conflictLoading" class="section-loader-overlay">
          <img :src="logoCrocLoading" alt="Loading diagnostics" class="section-loader-logo" />
          <div class="section-loader-text">Loading diagnostics</div>
          <div class="loader-wave" aria-hidden="true">
            <span
              v-for="(letter, index) in loadingLetters"
              :key="`diagnostics-loader-${index}`"
              class="loader-letter"
              :style="{ animationDelay: `${index * 0.08}s` }"
            >
              {{ letter }}
            </span>
          </div>
        </div>
      </section>

      <section class="section-shell rounded-lg border border-[var(--border)] bg-[var(--card)]/90 p-4">
          <div class="flex flex-wrap items-center justify-between gap-2 mb-3">
            <h3 class="text-sm font-semibold text-[var(--foreground)]">Consistency and Momentum</h3>
            <span class="text-[10px] text-[var(--muted-foreground)]">{{ repoAgeDays }} day repository age</span>
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
        <div v-if="historyLoading" class="section-loader-overlay">
          <img :src="logoCrocLoading" alt="Loading consistency metrics" class="section-loader-logo" />
          <div class="section-loader-text">Loading consistency metrics</div>
          <div class="loader-wave" aria-hidden="true">
            <span
              v-for="(letter, index) in loadingLetters"
              :key="`momentum-loader-${index}`"
              class="loader-letter"
              :style="{ animationDelay: `${index * 0.08}s` }"
            >
              {{ letter }}
            </span>
          </div>
        </div>
      </section>

      <section class="section-shell rounded-lg border border-[var(--border)] bg-[var(--card)]/90 p-4">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-[var(--foreground)]">Activity Heat Map</h3>
            <span class="text-[10px] text-[var(--muted-foreground)]">Last 12 months</span>
          </div>

          <div>
            <div class="heatmap-grid">
              <div
                v-for="(week, weekIndex) in heatWeeks"
                :key="`week-${weekIndex}`"
                class="heatmap-week"
              >
                <div
                  v-for="cell in week"
                  :key="cell.key"
                  class="heatmap-cell"
                  :style="heatCellStyle(cell)"
                  :title="`${cell.dateLabel}: ${cell.count} commit${cell.count === 1 ? '' : 's'}`"
                />
              </div>
            </div>
          </div>
        <div v-if="historyLoading" class="section-loader-overlay">
          <img :src="logoCrocLoading" alt="Loading activity heatmap" class="section-loader-logo" />
          <div class="section-loader-text">Loading activity heatmap</div>
          <div class="loader-wave" aria-hidden="true">
            <span
              v-for="(letter, index) in loadingLetters"
              :key="`heat-loader-${index}`"
              class="loader-letter"
              :style="{ animationDelay: `${index * 0.08}s` }"
            >
              {{ letter }}
            </span>
          </div>
        </div>
      </section>

      <section class="section-shell rounded-lg border border-[var(--border)] bg-[var(--card)]/90 p-4">
          <div class="flex flex-wrap items-center justify-between gap-2 mb-3">
            <h3 class="text-sm font-semibold text-[var(--foreground)]">Bug Killer Leaderboard</h3>
            <span class="text-[10px] text-[var(--muted-foreground)]">Deletion stats across loaded history window</span>
          </div>

          <div v-if="bugKillerError" class="text-xs text-[var(--destructive)]">{{ bugKillerError }}</div>
          <div v-else-if="leaderboardRows.length === 0" class="text-xs text-[var(--muted-foreground)]">No deletion-heavy commits found yet.</div>
          <div v-else class="space-y-2">
            <div class="rounded-md border border-[var(--primary)]/30 bg-[var(--secondary)]/80 px-3 py-2.5">
              <div class="text-[10px] uppercase tracking-[0.14em] text-[var(--primary)]">Top Slayer</div>
              <div class="mt-1 text-sm font-semibold text-[var(--foreground)]">{{ topBugKiller?.author }}</div>
              <div class="text-[11px] text-[var(--muted-foreground)]">{{ topBugKiller?.deletions }} lines removed across {{ topBugKiller?.commits }} commits</div>
            </div>

            <div
              v-for="row in leaderboardRows"
              :key="row.author"
              class="rounded-md border border-[var(--border)] bg-[var(--secondary)]/70 px-3 py-2"
            >
              <div class="flex items-center justify-between gap-2 text-[11px] mb-1.5">
                <span class="font-medium text-[var(--foreground)] truncate">{{ row.author }}</span>
                <span class="text-[var(--primary)] font-semibold">{{ row.deletions }} del</span>
              </div>
              <div class="h-1.5 rounded bg-[var(--muted)] overflow-hidden">
                <div
                  class="h-full rounded bg-gradient-to-r from-[var(--primary)] to-[var(--chart-2)]"
                  :style="{ width: `${Math.max(8, Math.round((row.deletions / maxKillerDeletes) * 100))}%` }"
                />
              </div>
            </div>
          </div>
        <div v-if="bugKillerLoading" class="section-loader-overlay">
          <img :src="logoCrocLoading" alt="Loading deletion leaderboard" class="section-loader-logo" />
          <div class="section-loader-text">Loading bug killer leaderboard</div>
          <div class="loader-wave" aria-hidden="true">
            <span
              v-for="(letter, index) in loadingLetters"
              :key="`leaderboard-loader-${index}`"
              class="loader-letter"
              :style="{ animationDelay: `${index * 0.08}s` }"
            >
              {{ letter }}
            </span>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.productivity-surface {
  background:
    radial-gradient(circle at 15% 18%, color-mix(in srgb, var(--primary) 16%, transparent), transparent 45%),
    radial-gradient(circle at 82% 12%, color-mix(in srgb, var(--chart-2) 12%, transparent), transparent 40%),
    linear-gradient(145deg, color-mix(in srgb, var(--background) 94%, black 6%) 0%, var(--background) 52%, color-mix(in srgb, var(--card) 72%, var(--background) 28%) 100%);
}

.productivity-radial {
  background-image: repeating-linear-gradient(
    125deg,
    transparent 0,
    transparent 14px,
    color-mix(in srgb, var(--primary) 10%, transparent) 14px,
    color-mix(in srgb, var(--primary) 10%, transparent) 15px
  );
}

.metric-card {
  border: 1px solid var(--border);
  border-radius: 0.55rem;
  background: color-mix(in srgb, var(--card) 90%, transparent);
  padding: 0.8rem 0.9rem;
}

.metric-card.compact {
  padding: 0.7rem 0.8rem;
}

.metric-label {
  font-size: 10px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: color-mix(in srgb, var(--muted-foreground) 90%, transparent);
}

.metric-value {
  margin-top: 0.28rem;
  font-size: 1.4rem;
  line-height: 1;
  font-weight: 700;
  color: var(--foreground);
}

.metric-foot {
  margin-top: 0.22rem;
  font-size: 11px;
  color: var(--muted-foreground);
}

.metric-mini {
  margin-top: 0.22rem;
  font-size: 1.02rem;
  font-weight: 700;
  color: var(--foreground);
}

.momentum-card {
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  background: color-mix(in srgb, var(--secondary) 78%, transparent);
  padding: 0.65rem 0.75rem;
}

.momentum-label {
  font-size: 10px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.momentum-value {
  margin-top: 0.2rem;
  color: var(--foreground);
  font-size: 1rem;
  font-weight: 700;
}

.momentum-sub {
  margin-top: 0.14rem;
  font-size: 11px;
  color: var(--muted-foreground);
}

.focus-card {
  border: 1px solid var(--border);
  border-radius: 0.55rem;
  background: color-mix(in srgb, var(--card) 92%, transparent);
  padding: 0.85rem;
}

.focus-kicker {
  font-size: 10px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.focus-title {
  margin-top: 0.26rem;
  font-size: 13px;
  font-weight: 700;
  color: var(--foreground);
}

.focus-metrics-grid {
  margin-top: 0.55rem;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.45rem;
}

.focus-metric {
  border: 1px solid color-mix(in srgb, var(--border) 88%, transparent);
  border-radius: 0.45rem;
  background: color-mix(in srgb, var(--secondary) 74%, transparent);
  padding: 0.45rem 0.5rem;
}

.focus-metric-label {
  font-size: 10px;
  color: var(--muted-foreground);
}

.focus-metric-value {
  margin-top: 0.2rem;
  font-size: 1rem;
  font-weight: 700;
  color: var(--foreground);
}

.focus-score-row {
  margin-top: 0.55rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.focus-score-label {
  font-size: 11px;
  color: var(--muted-foreground);
}

.focus-score-value {
  margin-left: auto;
  font-size: 13px;
  font-weight: 700;
  color: var(--foreground);
}

.score-pill {
  border: 1px solid transparent;
  border-radius: 999px;
  font-size: 9px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  font-weight: 700;
  padding: 0.12rem 0.45rem;
}

.focus-meter {
  margin-top: 0.45rem;
  height: 7px;
  border-radius: 999px;
  overflow: hidden;
  background: color-mix(in srgb, var(--secondary) 80%, transparent);
}

.focus-meter-fill {
  height: 100%;
  border-radius: 999px;
}

.focus-list {
  margin-top: 0.58rem;
  display: grid;
  gap: 0.24rem;
  font-size: 11px;
  color: var(--muted-foreground);
}

.focus-foot {
  margin-top: 0.52rem;
  font-size: 10px;
  color: var(--muted-foreground);
}

.section-shell {
  position: relative;
  overflow: hidden;
}

.section-loader-overlay {
  position: absolute;
  inset: 0;
  z-index: 18;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--background) 82%, transparent);
  backdrop-filter: blur(2px);
}

.section-loader-overlay-card {
  border-radius: 0.55rem;
}

.section-loader-logo {
  width: 50px;
  height: 50px;
  object-fit: contain;
  filter: drop-shadow(0 0 8px color-mix(in srgb, var(--primary) 36%, transparent));
}

.section-loader-text {
  margin-top: 0.35rem;
  font-size: 11px;
  color: var(--foreground);
  font-weight: 600;
}

.loader-wave {
  margin-top: 0.18rem;
  display: inline-flex;
  gap: 1px;
}

.loader-letter {
  font-size: 10px;
  color: var(--primary);
  font-weight: 800;
  text-transform: uppercase;
  animation: productivity-loader-wave 1.08s ease-in-out infinite;
}

.arena-close {
  width: 28px;
  height: 28px;
  border-radius: 0.45rem;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--secondary) 78%, transparent);
  color: var(--muted-foreground);
  font-size: 12px;
  font-weight: 700;
  line-height: 1;
  text-transform: uppercase;
}

.arena-load-all {
  height: 28px;
  border-radius: 0.45rem;
  border: 1px solid color-mix(in srgb, var(--primary) 45%, var(--border));
  background: color-mix(in srgb, var(--secondary) 72%, transparent);
  color: var(--primary);
  font-size: 11px;
  font-weight: 600;
  padding: 0 10px;
}

.arena-load-all:hover {
  border-color: color-mix(in srgb, var(--primary) 70%, var(--border));
  background: color-mix(in srgb, var(--primary) 14%, var(--secondary));
}

.arena-load-all:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.arena-mode-pill {
  height: 28px;
  display: inline-flex;
  align-items: center;
  border-radius: 0.45rem;
  border: 1px solid color-mix(in srgb, var(--primary) 35%, var(--border));
  background: color-mix(in srgb, var(--primary) 10%, var(--secondary));
  color: var(--primary);
  font-size: 11px;
  font-weight: 600;
  padding: 0 10px;
}

.arena-close:hover {
  color: var(--foreground);
  border-color: color-mix(in srgb, var(--destructive) 62%, var(--border));
  background: color-mix(in srgb, var(--destructive) 14%, var(--secondary));
}

.arena-author-filter {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.2rem 0.55rem;
  border-radius: 0.5rem;
  border: 1px solid color-mix(in srgb, var(--primary) 30%, var(--border));
  background: color-mix(in srgb, var(--secondary) 88%, transparent);
}

.arena-author-filter label {
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.arena-author-filter select {
  background: var(--input-background);
  border: 1px solid var(--border);
  border-radius: 0.45rem;
  height: 24px;
  min-width: 150px;
  padding: 0 0.65rem;
  font-size: 11px;
  color: var(--foreground);
  outline: none;
}

.arena-author-filter select:focus {
  border-color: color-mix(in srgb, var(--primary) 75%, var(--border));
}

.heatmap-grid {
  width: 100%;
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(0, 1fr);
  gap: 4px;
}

.heatmap-week {
  min-width: 0;
  display: grid;
  grid-template-rows: repeat(7, minmax(0, 1fr));
  gap: 4px;
}

.heatmap-cell {
  width: 100%;
  aspect-ratio: 1 / 1;
  border-radius: 2px;
  border-width: 1px;
  transition: transform 0.15s ease;
}

.heatmap-cell:hover {
  transform: scale(1.07);
}

@keyframes productivity-loader-wave {
  0%,
  50%,
  100% {
    transform: translateY(0);
    opacity: 0.45;
  }

  25% {
    transform: translateY(-3px);
    opacity: 1;
  }
}
</style>

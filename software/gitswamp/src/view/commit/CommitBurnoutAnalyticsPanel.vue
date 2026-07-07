<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, Clock3, Flame, GitCommit, Users, Zap } from "lucide-vue-next";
import logoCrocLoading from "@/assets/logo_croc_loading.gif";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import type { CommitInfo } from "@/types";

const FULL_HISTORY_LIMIT = 60000;
const HOT_FILE_SCAN_LIMIT = 2500;
const RECENT_WEEK_COUNT = 12;
const HISTORY_FORMAT = "__GITSWAMP_ANALYTICS__%x1f%an%x1f%ae%x1f%ct%x1f%s";
const HOT_FILE_FORMAT = "__GITSWAMP_COMMIT__%x1f%an%x1f%ae%x1f%s";
const FIELD_SEPARATOR = "\x1f";
const DAY_LABELS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAY_ORDER = [1, 2, 3, 4, 5, 6, 0];
const AFTER_HOURS_HOURS = [20, 21, 22, 23, 0, 1, 2, 3, 4, 5, 6];

const historyCache = new Map<string, AnalyticsCommit[]>();
const hotFileCache = new Map<string, HotFileInsight[]>();

const props = defineProps<{
  repoPath: string;
  commits: CommitInfo[];
}>();

const emit = defineEmits<{
  close: [];
}>();

interface HotFileInsight {
  path: string;
  touches: number;
  fixTouches: number;
  ownerKey: string;
  ownerName: string;
  ownerTouches: number;
  ownerShare: number;
}

interface AnalyticsCommit {
  authorName: string;
  authorEmail: string;
  timestamp: number;
  subject: string;
}

interface AuthorStats {
  key: string;
  name: string;
  email: string;
  commits: number;
  firstCommit: number;
  lastCommit: number;
  activeDays: number;
  lateNightCommits: number;
  afterHoursCommits: number;
  weekendCommits: number;
  saturdayNightCommits: number;
  lateNightWeekStreak: number;
  hotFileCount: number;
  hotFileTouches: number;
  riskScore: number;
  riskLevel: "low" | "medium" | "high";
  afterHoursHourly: number[];
  afterHoursWeekdays: number[];
  peakAfterHoursHour: number | null;
  peakAfterHoursWeekday: number | null;
  maxAfterHoursHourCount: number;
  recentWeeks: number[];
  recentAfterHoursWeeks: number[];
}

const historyCommits = ref<AnalyticsCommit[]>([]);
const authors = ref<AuthorStats[]>([]);
const totalCommitCount = ref(0);
const recentWeekStarts = ref<Date[]>([]);
const hotFiles = ref<HotFileInsight[]>([]);
const historyLoading = ref(false);
const hotFilesLoading = ref(false);
const statsLoading = ref(false);
const historyError = ref("");
const hotFilesError = ref("");
let historyLoadToken = 0;
let hotFilesLoadToken = 0;
let statsBuildToken = 0;

function toMillis(timestamp: number): number {
  return Math.abs(timestamp) < 1000000000000 ? timestamp * 1000 : timestamp;
}

function authorKey(name: string, email?: string): string {
  return (email || name || "unknown").trim().toLowerCase();
}

function dayKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function shiftDays(base: Date, offset: number): Date {
  const next = new Date(base);
  next.setDate(next.getDate() + offset);
  return next;
}

function weekStart(date: Date): Date {
  const next = new Date(date);
  next.setHours(0, 0, 0, 0);
  const day = next.getDay();
  const offset = day === 0 ? -6 : 1 - day;
  next.setDate(next.getDate() + offset);
  return next;
}

function weekKey(date: Date): string {
  return dayKey(weekStart(date));
}

function formatDate(timestamp: number): string {
  if (!timestamp) return "-";
  return new Date(timestamp).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "2-digit",
  });
}

function formatShortDate(date: Date): string {
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}

function formatWeekRange(start: Date): string {
  const end = shiftDays(start, 6);
  return `${formatShortDate(start)} - ${formatShortDate(end)}`;
}

function formatHourRange(hour: number): string {
  const next = (hour + 1) % 24;
  return `${String(hour).padStart(2, "0")}:00-${String(next).padStart(2, "0")}:00`;
}

function percent(part: number, total: number): number {
  if (total <= 0) return 0;
  return Math.round((part / total) * 100);
}

function peakIndex(values: number[]): number | null {
  let max = 0;
  let index: number | null = null;
  values.forEach((value, currentIndex) => {
    if (value > max) {
      max = value;
      index = currentIndex;
    }
  });
  return index;
}

function yieldToUi(): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, 0);
  });
}

function capCache<T>(cache: Map<string, T>, key: string, value: T) {
  if (cache.has(key)) {
    cache.delete(key);
  }
  cache.set(key, value);
  while (cache.size > 1) {
    const oldestKey = cache.keys().next().value;
    if (!oldestKey) break;
    cache.delete(oldestKey);
  }
}

function commitToAnalytics(commit: CommitInfo): AnalyticsCommit {
  return {
    authorName: commit.author_name || "Unknown",
    authorEmail: commit.author_email || "",
    timestamp: toMillis(commit.timestamp),
    subject: (commit.message || "(no subject)").split(/\r?\n/)[0] || "(no subject)",
  };
}

async function parseHistoryLog(output: string, token: number): Promise<AnalyticsCommit[] | null> {
  const rows: AnalyticsCommit[] = [];
  const lines = output.split(/\r?\n/);

  for (let index = 0; index < lines.length; index += 1) {
    if (token !== historyLoadToken) return null;
    if (index > 0 && index % 2500 === 0) {
      await yieldToUi();
    }

    const rawLine = lines[index];
    const line = rawLine.trim();
    if (!line.startsWith("__GITSWAMP_ANALYTICS__")) continue;

    const [, authorName, authorEmail, epochSeconds, subject] = line.split(FIELD_SEPARATOR);
    const parsedSeconds = Number.parseInt(epochSeconds || "", 10);
    if (!Number.isFinite(parsedSeconds)) continue;

    rows.push({
      authorName: authorName || "Unknown",
      authorEmail: authorEmail || "",
      timestamp: parsedSeconds * 1000,
      subject: subject || "(no subject)",
    });
  }
  return rows;
}

function setInitialCommits() {
  if (!props.repoPath) {
    historyCommits.value = [];
    authors.value = [];
    totalCommitCount.value = 0;
    return;
  }

  const cached = historyCache.get(props.repoPath);
  historyCommits.value = cached || props.commits.map(commitToAnalytics);
  void rebuildAuthorStats();
}

async function loadFullHistory() {
  const repoPath = props.repoPath;
  if (!repoPath) return;

  const cached = historyCache.get(repoPath);
  if (cached) {
    historyCommits.value = cached;
    void rebuildAuthorStats();
    return;
  }

  const token = ++historyLoadToken;
  historyLoading.value = true;
  historyError.value = "";

  try {
    const output = await invoke<string>("run_git_command", {
      path: repoPath,
      args: [
        "log",
        "--all",
        `--max-count=${FULL_HISTORY_LIMIT}`,
        `--format=${HISTORY_FORMAT}`,
      ],
    });
    if (token !== historyLoadToken || repoPath !== props.repoPath) return;
    const commits = await parseHistoryLog(output, token);
    if (!commits) return;
    historyCommits.value = commits;
    capCache(historyCache, repoPath, commits);
    void rebuildAuthorStats();
  } catch {
    if (token !== historyLoadToken) return;
    historyError.value = "Could not load full repository history.";
  } finally {
    if (token === historyLoadToken) {
      historyLoading.value = false;
    }
  }
}

function isFixLikeSubject(subject: string): boolean {
  return /\b(fix|bug|hotfix|regression|revert|crash|broken|repair|patch)\b/i.test(subject);
}

function parseHotFileLog(output: string): HotFileInsight[] {
  const files = new Map<string, {
    touches: number;
    fixTouches: number;
    authorTouches: Map<string, { name: string; touches: number }>;
  }>();
  let currentAuthorKey = "";
  let currentAuthorName = "Unknown";
  let currentFixLike = false;

  for (const rawLine of output.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line) continue;

    if (line.startsWith("__GITSWAMP_COMMIT__")) {
      const [, authorName, authorEmail, subject] = line.split(FIELD_SEPARATOR);
      currentAuthorName = authorName || "Unknown";
      currentAuthorKey = authorKey(currentAuthorName, authorEmail);
      currentFixLike = isFixLikeSubject(subject || "");
      continue;
    }

    const path = line.replace(/\\/g, "/");
    const entry = files.get(path) || {
      touches: 0,
      fixTouches: 0,
      authorTouches: new Map<string, { name: string; touches: number }>(),
    };
    entry.touches += 1;
    if (currentFixLike) {
      entry.fixTouches += 1;
    }

    const author = entry.authorTouches.get(currentAuthorKey) || { name: currentAuthorName, touches: 0 };
    author.touches += 1;
    entry.authorTouches.set(currentAuthorKey, author);
    files.set(path, entry);
  }

  return Array.from(files.entries())
    .map(([path, entry]) => {
      const owner = Array.from(entry.authorTouches.entries())
        .sort((a, b) => b[1].touches - a[1].touches || a[1].name.localeCompare(b[1].name))[0];
      const ownerKey = owner?.[0] || "";
      const ownerName = owner?.[1].name || "Unknown";
      const ownerTouches = owner?.[1].touches || 0;
      return {
        path,
        touches: entry.touches,
        fixTouches: entry.fixTouches,
        ownerKey,
        ownerName,
        ownerTouches,
        ownerShare: percent(ownerTouches, entry.touches),
      };
    })
    .filter((file) => file.touches >= 3)
    .sort((a, b) => (b.touches + b.fixTouches * 2) - (a.touches + a.fixTouches * 2) || a.path.localeCompare(b.path))
    .slice(0, 18);
}

async function loadHotFiles() {
  const repoPath = props.repoPath;
  if (!repoPath) return;

  const cached = hotFileCache.get(repoPath);
  if (cached) {
    hotFiles.value = cached;
    void rebuildAuthorStats();
    return;
  }

  const token = ++hotFilesLoadToken;
  hotFilesLoading.value = true;
  hotFilesError.value = "";

  try {
    const output = await invoke<string>("run_git_command", {
      path: repoPath,
      args: [
        "log",
        "--all",
        "--no-merges",
        `--max-count=${HOT_FILE_SCAN_LIMIT}`,
        "--name-only",
        `--format=${HOT_FILE_FORMAT}`,
      ],
    });
    if (token !== hotFilesLoadToken || repoPath !== props.repoPath) return;
    const parsed = parseHotFileLog(output);
    hotFiles.value = parsed;
    capCache(hotFileCache, repoPath, parsed);
    void rebuildAuthorStats();
  } catch {
    if (token !== hotFilesLoadToken) return;
    hotFilesError.value = "Hot file scan skipped.";
  } finally {
    if (token === hotFilesLoadToken) {
      hotFilesLoading.value = false;
    }
  }
}

function weekStartsForCommits(commits: AnalyticsCommit[]): Date[] {
  let newest = 0;
  for (const commit of commits) {
    newest = Math.max(newest, toMillis(commit.timestamp));
  }
  if (newest <= 0) {
    newest = Date.now();
  }

  const start = weekStart(new Date(newest));
  return Array.from({ length: RECENT_WEEK_COUNT }, (_, index) =>
    shiftDays(start, (index - RECENT_WEEK_COUNT + 1) * 7),
  );
}

async function buildAuthorStats(
  commits: AnalyticsCommit[],
  hotFileRows: HotFileInsight[],
  token: number,
): Promise<AuthorStats[] | null> {
  if (commits.length === 0) return [];

  const recentWeeks = weekStartsForCommits(commits).map(weekKey);

  const hotByOwner = new Map<string, { files: number; touches: number }>();
  for (const file of hotFileRows) {
    const current = hotByOwner.get(file.ownerKey) || { files: 0, touches: 0 };
    current.files += 1;
    current.touches += file.ownerTouches;
    hotByOwner.set(file.ownerKey, current);
  }

  const byAuthor = new Map<string, AuthorStats & {
    activeDaySet: Set<string>;
    lateWeeks: Set<string>;
  }>();

  for (let commitIndex = 0; commitIndex < commits.length; commitIndex += 1) {
    if (token !== statsBuildToken) return null;
    if (commitIndex > 0 && commitIndex % 2500 === 0) {
      await yieldToUi();
    }

    const commit = commits[commitIndex];
    const millis = toMillis(commit.timestamp);
    const date = new Date(millis);
    const key = authorKey(commit.authorName, commit.authorEmail);
    const week = weekKey(date);
    const day = dayKey(date);
    const hour = date.getHours();
    const weekday = date.getDay();
    const recentWeekIndex = recentWeeks.indexOf(week);
    const afterHours = hour < 7 || hour >= 20;
    const lateNight = hour < 5;
    const weekend = weekday === 0 || weekday === 6;

    const stats = byAuthor.get(key) || {
      key,
      name: commit.authorName || "Unknown",
      email: commit.authorEmail || "",
      commits: 0,
      firstCommit: millis,
      lastCommit: millis,
      activeDays: 0,
      lateNightCommits: 0,
      afterHoursCommits: 0,
      weekendCommits: 0,
      saturdayNightCommits: 0,
      lateNightWeekStreak: 0,
      hotFileCount: 0,
      hotFileTouches: 0,
      riskScore: 0,
      riskLevel: "low",
      afterHoursHourly: Array.from({ length: 24 }, () => 0),
      afterHoursWeekdays: Array.from({ length: 7 }, () => 0),
      peakAfterHoursHour: null,
      peakAfterHoursWeekday: null,
      maxAfterHoursHourCount: 0,
      recentWeeks: Array.from({ length: RECENT_WEEK_COUNT }, () => 0),
      recentAfterHoursWeeks: Array.from({ length: RECENT_WEEK_COUNT }, () => 0),
      activeDaySet: new Set<string>(),
      lateWeeks: new Set<string>(),
    };

    stats.commits += 1;
    stats.firstCommit = Math.min(stats.firstCommit, millis);
    stats.lastCommit = Math.max(stats.lastCommit, millis);
    stats.activeDaySet.add(day);
    if (afterHours) {
      stats.afterHoursCommits += 1;
      stats.afterHoursHourly[hour] += 1;
      stats.afterHoursWeekdays[weekday] += 1;
    }
    if (lateNight) {
      stats.lateNightCommits += 1;
      stats.lateWeeks.add(week);
    }
    if (weekend) stats.weekendCommits += 1;
    if (weekday === 6 && hour >= 20) stats.saturdayNightCommits += 1;
    if (recentWeekIndex >= 0) {
      stats.recentWeeks[recentWeekIndex] += 1;
      if (afterHours) stats.recentAfterHoursWeeks[recentWeekIndex] += 1;
    }
    byAuthor.set(key, stats);
  }

  const rows: AuthorStats[] = [];
  const rawStats = Array.from(byAuthor.values());
  for (let statsIndex = 0; statsIndex < rawStats.length; statsIndex += 1) {
    if (token !== statsBuildToken) return null;
    if (statsIndex > 0 && statsIndex % 40 === 0) {
      await yieldToUi();
    }

    const stats = rawStats[statsIndex];
      let streak = 0;
      for (let index = recentWeeks.length - 1; index >= 0; index -= 1) {
        if (!stats.lateWeeks.has(recentWeeks[index])) break;
        streak += 1;
      }

      const hot = hotByOwner.get(stats.key) || { files: 0, touches: 0 };
      const riskScore = Math.min(100, Math.round(
        percent(stats.afterHoursCommits, stats.commits) * 0.3
        + percent(stats.weekendCommits, stats.commits) * 0.24
        + percent(stats.lateNightCommits, stats.commits) * 0.28
        + Math.min(24, streak * 8)
        + Math.min(18, hot.files * 3),
      ));
      const riskLevel: AuthorStats["riskLevel"] = riskScore >= 65 ? "high" : riskScore >= 35 ? "medium" : "low";
      const maxAfterHoursHourCount = Math.max(0, ...stats.afterHoursHourly);

      rows.push({
        key: stats.key,
        name: stats.name,
        email: stats.email,
        commits: stats.commits,
        firstCommit: stats.firstCommit,
        lastCommit: stats.lastCommit,
        activeDays: stats.activeDaySet.size,
        lateNightCommits: stats.lateNightCommits,
        afterHoursCommits: stats.afterHoursCommits,
        weekendCommits: stats.weekendCommits,
        saturdayNightCommits: stats.saturdayNightCommits,
        lateNightWeekStreak: streak,
        hotFileCount: hot.files,
        hotFileTouches: hot.touches,
        riskScore,
        riskLevel,
        afterHoursHourly: stats.afterHoursHourly,
        afterHoursWeekdays: stats.afterHoursWeekdays,
        peakAfterHoursHour: peakIndex(stats.afterHoursHourly),
        peakAfterHoursWeekday: peakIndex(stats.afterHoursWeekdays),
        maxAfterHoursHourCount,
        recentWeeks: stats.recentWeeks,
        recentAfterHoursWeeks: stats.recentAfterHoursWeeks,
      });
  }

  return rows.sort((a, b) => b.riskScore - a.riskScore || b.commits - a.commits || a.name.localeCompare(b.name));
}

async function rebuildAuthorStats() {
  const token = ++statsBuildToken;
  const commits = historyCommits.value;
  totalCommitCount.value = commits.length;
  recentWeekStarts.value = weekStartsForCommits(commits);

  if (commits.length === 0) {
    authors.value = [];
    statsLoading.value = false;
    return;
  }

  statsLoading.value = true;
  await yieldToUi();
  const rows = await buildAuthorStats(commits, hotFiles.value, token);
  if (token !== statsBuildToken || rows === null) return;
  authors.value = rows;
  statsLoading.value = false;
}

const totalCommits = computed(() => totalCommitCount.value);
const highRiskAuthors = computed(() => authors.value.filter((author) => author.riskLevel === "high").length);
const mediumRiskAuthors = computed(() => authors.value.filter((author) => author.riskLevel === "medium").length);
const afterHoursCommitCount = computed(() => authors.value.reduce((sum, author) => sum + author.afterHoursCommits, 0));
const weekendCommitCount = computed(() => authors.value.reduce((sum, author) => sum + author.weekendCommits, 0));
const aggregateWeeks = computed(() => {
  const weeks = Array.from({ length: RECENT_WEEK_COUNT }, () => 0);
  for (const author of authors.value) {
    author.recentWeeks.forEach((count, index) => {
      weeks[index] += count;
    });
  }
  return weeks;
});
const aggregateAfterHoursWeeks = computed(() => {
  const weeks = Array.from({ length: RECENT_WEEK_COUNT }, () => 0);
  for (const author of authors.value) {
    author.recentAfterHoursWeeks.forEach((count, index) => {
      weeks[index] += count;
    });
  }
  return weeks;
});
const aggregateAfterHoursHours = computed(() => {
  const hours = Array.from({ length: 24 }, () => 0);
  for (const author of authors.value) {
    author.afterHoursHourly.forEach((count, index) => {
      hours[index] += count;
    });
  }
  return hours;
});
const aggregateAfterHoursWeekdays = computed(() => {
  const days = Array.from({ length: 7 }, () => 0);
  for (const author of authors.value) {
    author.afterHoursWeekdays.forEach((count, index) => {
      days[index] += count;
    });
  }
  return days;
});
const maxWeekCount = computed(() => Math.max(1, ...aggregateWeeks.value));
const maxAfterHoursCount = computed(() => Math.max(1, ...aggregateAfterHoursHours.value));
const maxAfterHoursWeekdayCount = computed(() => Math.max(1, ...aggregateAfterHoursWeekdays.value));
const isUsingInitialHistory = computed(() =>
  (historyLoading.value || statsLoading.value) && historyCommits.value.length > 0 && !historyCache.has(props.repoPath),
);

function barHeight(value: number, max: number, min = 6): string {
  return `${Math.max(min, Math.round((value / Math.max(1, max)) * 100))}%`;
}

function barWidth(value: number, max: number): string {
  return `${Math.max(2, Math.round((value / Math.max(1, max)) * 100))}%`;
}

function riskClass(level: AuthorStats["riskLevel"]): string {
  if (level === "high") return "risk-high";
  if (level === "medium") return "risk-medium";
  return "risk-low";
}

function riskLabel(level: AuthorStats["riskLevel"]): string {
  if (level === "high") return "High";
  if (level === "medium") return "Watch";
  return "Steady";
}

function peakAfterHoursLabel(author: AuthorStats): string {
  if (author.peakAfterHoursHour === null) {
    return "No after-hours pattern";
  }

  const day = author.peakAfterHoursWeekday === null ? "mixed days" : DAY_LABELS[author.peakAfterHoursWeekday];
  return `${formatHourRange(author.peakAfterHoursHour)} · ${day}`;
}

function compactPath(path: string): string {
  return path.length > 64 ? `...${path.slice(-61)}` : path;
}

watch(
  () => [props.repoPath, props.commits.length] as const,
  () => {
    historyLoadToken += 1;
    hotFilesLoadToken += 1;
    hotFiles.value = [];
    historyError.value = "";
    hotFilesError.value = "";
    setInitialCommits();
    void loadFullHistory();
    void loadHotFiles();
  },
  { immediate: true },
);

onUnmounted(() => {
  historyLoadToken += 1;
  hotFilesLoadToken += 1;
  statsBuildToken += 1;
  historyCommits.value = [];
  authors.value = [];
  hotFiles.value = [];
});
</script>

<template>
  <div class="burnout-surface flex-1 min-h-0 overflow-y-auto">
    <div class="p-4 md:p-5 space-y-4">
      <section class="burnout-hero">
        <div class="min-w-0">
          <p class="eyebrow">Team Focus</p>
          <h2>Burnout Analytics</h2>
          <p class="hero-copy">Repository-wide rhythm, after-hours load, and ownership pressure.</p>
        </div>
        <div class="hero-actions">
          <div v-if="historyLoading || hotFilesLoading || statsLoading" class="loading-pill">
            <img :src="logoCrocLoading" alt="" />
            <span>{{ isUsingInitialHistory ? "Refining" : "Loading" }}</span>
          </div>
          <CloseIconButton title="Back to Git Graph" @click="emit('close')" />
        </div>
      </section>

      <section class="metric-grid">
        <div class="metric-tile">
          <GitCommit class="metric-icon" />
          <span class="metric-label">Commits</span>
          <strong>{{ totalCommits }}</strong>
        </div>
        <div class="metric-tile">
          <Users class="metric-icon" />
          <span class="metric-label">Contributors</span>
          <strong>{{ authors.length }}</strong>
        </div>
        <div class="metric-tile">
          <Clock3 class="metric-icon" />
          <span class="metric-label">After hours</span>
          <strong>{{ percent(afterHoursCommitCount, totalCommits) }}%</strong>
        </div>
        <div class="metric-tile">
          <AlertTriangle class="metric-icon" />
          <span class="metric-label">High risk</span>
          <strong>{{ highRiskAuthors }}</strong>
        </div>
      </section>

      <section class="explain-grid">
        <div>
          <strong>Risk score</strong>
          <span>Weighted signal from after-hours work, weekend work, repeated late-night streaks and hot-file ownership.</span>
        </div>
        <div>
          <strong>After hours</strong>
          <span>Commits before 07:00 or after 20:00, using local time from Git history.</span>
        </div>
        <div>
          <strong>Hot files</strong>
          <span>Recently touched files that appear often, especially in fix-like commits, so ownership pressure is visible.</span>
        </div>
      </section>

      <section class="analytics-section">
        <div class="section-head">
          <div>
            <p class="eyebrow">Focus Load</p>
            <h3>Team After-Hours Pattern</h3>
          </div>
          <span class="section-note">{{ percent(weekendCommitCount, totalCommits) }}% weekend commits</span>
        </div>
        <p class="chart-explain">
          This only shows work outside normal hours, so it stays focused on team fatigue instead of repeating Productivity Arena.
        </p>
        <div class="stress-grid">
          <div>
            <h4>Late work by hour</h4>
            <div class="team-hour-chart" aria-label="After-hours commits by hour">
              <div v-for="hour in AFTER_HOURS_HOURS" :key="hour" class="team-hour-column">
                <div class="team-hour-bar-wrap">
                  <div class="team-hour-bar" :style="{ height: barHeight(aggregateAfterHoursHours[hour], maxAfterHoursCount, 5) }" />
                </div>
                <span>{{ hour }}</span>
              </div>
            </div>
          </div>

          <div>
            <h4>Late work by day</h4>
            <div class="late-day-chart" aria-label="After-hours commits by weekday">
              <div v-for="dayIndex in WEEKDAY_ORDER" :key="dayIndex" class="late-day-row">
                <span>{{ DAY_LABELS[dayIndex] }}</span>
                <div class="late-day-track">
                  <div class="late-day-fill" :style="{ width: barWidth(aggregateAfterHoursWeekdays[dayIndex], maxAfterHoursWeekdayCount) }" />
                </div>
                <strong>{{ aggregateAfterHoursWeekdays[dayIndex] }}</strong>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section class="analytics-section">
        <div class="section-head">
          <div>
            <p class="eyebrow">Trend</p>
            <h3>Repository Burnout Pulse</h3>
          </div>
          <span class="section-note">last {{ RECENT_WEEK_COUNT }} weeks</span>
        </div>
        <p class="chart-explain">
          Each row is a Monday-Sunday week, the cool line is total work, the warm line is after-hours load.
        </p>
        <div class="week-bars">
          <div v-for="(count, index) in aggregateWeeks" :key="index" class="week-row">
            <span class="week-label">
              <strong>{{ formatWeekRange(recentWeekStarts[index]) }}</strong>
              <small>{{ index === RECENT_WEEK_COUNT - 1 ? "current" : `${RECENT_WEEK_COUNT - index - 1}w ago` }}</small>
            </span>
            <div class="week-track">
              <div class="week-fill" :style="{ width: barWidth(count, maxWeekCount) }" />
              <div
                v-if="aggregateAfterHoursWeeks[index] > 0"
                class="week-after-fill"
                :style="{ width: barWidth(aggregateAfterHoursWeeks[index], maxWeekCount) }"
              />
            </div>
            <strong>{{ count }}</strong>
            <span class="week-after-label">{{ aggregateAfterHoursWeeks[index] }} after</span>
          </div>
        </div>
      </section>

      <section class="analytics-section">
        <div class="section-head">
          <div>
            <p class="eyebrow">People</p>
            <h3>Contributor Focus Risk</h3>
          </div>
          <span class="section-note">{{ mediumRiskAuthors }} watch signals</span>
        </div>

        <div v-if="authors.length === 0 && !historyLoading" class="empty-state">
          No commits available for this repository.
        </div>

        <div v-else class="author-list">
          <article v-for="author in authors" :key="author.key" class="author-row">
            <div class="author-main">
              <div class="author-avatar">{{ author.name.slice(0, 2).toUpperCase() }}</div>
              <div class="min-w-0">
                <div class="author-name">{{ author.name }}</div>
                <div class="author-meta">
                  {{ author.commits }} commits · {{ author.activeDays }} active days · {{ formatDate(author.lastCommit) }}
                </div>
              </div>
            </div>

            <div class="risk-block">
              <span :class="['risk-badge', riskClass(author.riskLevel)]">{{ riskLabel(author.riskLevel) }}</span>
              <strong>{{ author.riskScore }}</strong>
            </div>

            <div class="signal-grid">
              <span><Clock3 class="h-3 w-3" /> {{ percent(author.afterHoursCommits, author.commits) }}% after hours</span>
              <span><Flame class="h-3 w-3" /> {{ author.lateNightWeekStreak }} late-week streak</span>
              <span><Zap class="h-3 w-3" /> {{ author.hotFileCount }} hot files</span>
              <span><Clock3 class="h-3 w-3" /> Peak {{ peakAfterHoursLabel(author) }}</span>
            </div>

            <div class="focus-debt" title="Recent after-hours commits">
              <span
                v-for="(count, index) in author.recentAfterHoursWeeks"
                :key="`${author.key}-debt-${index}`"
                :class="{ active: count > 0 }"
              />
            </div>

            <div class="author-detail-row">
              <div class="author-insight">
                <strong>Most common off-hour work time</strong>
                <span>{{ author.afterHoursCommits }} after-hours commits, most often {{ peakAfterHoursLabel(author) }}, {{ author.saturdayNightCommits }} on Saturday night</span>
              </div>

              <div class="after-hour-bars" title="After-hours distribution">
                <span
                  v-for="hour in AFTER_HOURS_HOURS"
                  :key="`${author.key}-after-hour-${hour}`"
                  :style="{ height: barHeight(author.afterHoursHourly[hour], Math.max(1, author.maxAfterHoursHourCount), 5) }"
                >
                  <small>{{ hour }}</small>
                </span>
              </div>
            </div>
          </article>
        </div>
      </section>

      <section class="analytics-section">
        <div class="section-head">
          <div>
            <p class="eyebrow">Ownership Pressure</p>
            <h3>Hot Files And Bottlenecks</h3>
          </div>
          <span class="section-note">{{ hotFilesLoading ? "scanning" : `${hotFiles.length} files` }}</span>
        </div>

        <div v-if="hotFilesError" class="empty-state">{{ hotFilesError }}</div>
        <div v-else-if="hotFiles.length === 0 && !hotFilesLoading" class="empty-state">
          No recurring hot files detected in the recent scan.
        </div>
        <div v-else class="hot-file-list">
          <div v-for="file in hotFiles" :key="file.path" class="hot-file-row">
            <div class="min-w-0">
              <div class="hot-path">{{ compactPath(file.path) }}</div>
              <div class="hot-meta">{{ file.ownerName }} owns {{ file.ownerShare }}% of touches</div>
            </div>
            <div class="hot-stats">
              <span>{{ file.touches }} touches</span>
              <strong>{{ file.fixTouches }} fixes</strong>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.burnout-surface {
  background:
    radial-gradient(circle at top left, color-mix(in srgb, var(--primary) 12%, transparent), transparent 34%),
    var(--background);
  color: var(--foreground);
}

.burnout-hero,
.analytics-section {
  border-bottom: 1px solid var(--border);
  background: color-mix(in srgb, var(--card) 86%, transparent);
  padding: 16px;
}

.burnout-hero {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.eyebrow {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--muted-foreground);
  margin: 0 0 4px;
}

h2,
h3 {
  margin: 0;
  color: var(--foreground);
}

h2 {
  font-size: 22px;
  font-weight: 800;
}

h3 {
  font-size: 14px;
  font-weight: 750;
}

.hero-copy,
.section-note,
.author-meta,
.hot-meta {
  color: var(--muted-foreground);
  font-size: 11px;
}

.hero-actions,
.section-head,
.author-main,
.signal-grid span,
.hot-file-row,
.risk-block {
  display: flex;
  align-items: center;
}

.hero-actions,
.section-head {
  justify-content: space-between;
  gap: 12px;
}

.loading-pill {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 5px 9px;
  color: var(--muted-foreground);
  font-size: 11px;
}

.loading-pill img {
  width: 18px;
  height: 18px;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.metric-tile {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 9px;
  align-items: center;
  border-bottom: 1px solid var(--border);
  background: color-mix(in srgb, var(--card) 82%, transparent);
  padding: 12px;
}

.metric-icon {
  grid-row: span 2;
  width: 18px;
  height: 18px;
  color: var(--primary);
}

.metric-label {
  font-size: 10px;
  color: var(--muted-foreground);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.metric-tile strong {
  font-size: 20px;
  line-height: 1;
}

.explain-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.explain-grid div {
  border-bottom: 1px solid var(--border);
  background: color-mix(in srgb, var(--card) 78%, transparent);
  padding: 10px 12px;
}

.explain-grid strong,
.author-insight strong {
  display: block;
  color: var(--foreground);
  font-size: 11px;
  margin-bottom: 3px;
}

.explain-grid span,
.chart-explain,
.author-insight span {
  color: var(--muted-foreground);
  font-size: 11px;
  line-height: 1.45;
}

.chart-explain {
  margin: 8px 0 0;
}

.stress-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(220px, 0.7fr);
  gap: 18px;
  align-items: stretch;
}

.stress-grid h4 {
  margin: 12px 0 8px;
  color: var(--foreground);
  font-size: 11px;
  font-weight: 750;
}

.team-hour-chart {
  height: 118px;
  display: grid;
  grid-template-columns: repeat(11, minmax(14px, 1fr));
  gap: 8px;
  align-items: end;
}

.team-hour-column {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 5px;
  font-size: 9px;
  color: var(--muted-foreground);
}

.team-hour-bar-wrap {
  flex: 1;
  width: 100%;
  display: flex;
  align-items: end;
}

.team-hour-bar {
  width: 100%;
  border-radius: 5px 5px 0 0;
  background: linear-gradient(180deg, #fbbf24 0%, #fb7185 52%, #be123c 100%);
  box-shadow: 0 0 12px rgba(251, 113, 133, 0.18);
}

.late-day-chart {
  display: grid;
  align-content: center;
  gap: 8px;
}

.late-day-row {
  display: grid;
  grid-template-columns: 30px minmax(0, 1fr) 36px;
  align-items: center;
  gap: 8px;
  color: var(--muted-foreground);
  font-size: 10px;
}

.late-day-track {
  height: 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--secondary) 82%, #0f172a);
  overflow: hidden;
}

.late-day-fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #f59e0b 0%, #f43f5e 65%, #a855f7 100%);
}

.week-bars {
  margin-top: 12px;
  display: grid;
  gap: 8px;
}

.week-row {
  display: grid;
  grid-template-columns: 112px minmax(0, 1fr) 44px 64px;
  align-items: center;
  gap: 8px;
  color: var(--muted-foreground);
  font-size: 10px;
}

.week-label strong,
.week-label small {
  display: block;
  white-space: nowrap;
}

.week-label strong {
  color: var(--foreground);
  font-size: 10px;
  font-weight: 700;
}

.week-label small,
.week-after-label {
  color: var(--muted-foreground);
  font-size: 9px;
}

.week-track {
  height: 9px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--secondary) 84%, #111827);
  overflow: hidden;
  position: relative;
}

.week-fill,
.week-after-fill {
  position: absolute;
  left: 0;
  border-radius: inherit;
}

.week-fill {
  top: 0;
  height: 100%;
  background: linear-gradient(90deg, #67e8f9 0%, #38bdf8 38%, #6366f1 100%);
  box-shadow: 0 0 10px rgba(56, 189, 248, 0.18);
}

.week-after-fill {
  bottom: 0;
  height: 4px;
  background: linear-gradient(90deg, #fbbf24 0%, #fb7185 54%, #be123c 100%);
  box-shadow: 0 0 10px rgba(251, 113, 133, 0.2);
}

.author-list,
.hot-file-list {
  margin-top: 12px;
  display: grid;
  gap: 8px;
}

.author-row {
  display: grid;
  grid-template-columns: minmax(190px, 1.15fr) 90px minmax(260px, 1.35fr) 124px;
  align-items: center;
  gap: 12px;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--background) 72%, transparent);
  padding: 10px;
}

.author-main {
  gap: 10px;
  min-width: 0;
}

.author-avatar {
  width: 34px;
  height: 34px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 800;
  background: color-mix(in srgb, var(--primary) 16%, var(--background));
  color: var(--primary);
  border: 1px solid color-mix(in srgb, var(--primary) 25%, var(--border));
}

.author-name,
.hot-path {
  font-size: 12px;
  font-weight: 700;
  color: var(--foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.risk-block {
  justify-content: flex-end;
  gap: 8px;
}

.risk-badge {
  border-radius: 999px;
  padding: 3px 8px;
  font-size: 10px;
  font-weight: 800;
  border: 1px solid var(--border);
}

.risk-low {
  color: #22c55e;
  background: rgba(34, 197, 94, 0.1);
}

.risk-medium {
  color: #eab308;
  background: rgba(234, 179, 8, 0.12);
}

.risk-high {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.12);
}

.signal-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 5px 10px;
  font-size: 10px;
  color: var(--muted-foreground);
}

.signal-grid span {
  gap: 5px;
  min-width: 0;
}

.focus-debt {
  display: grid;
  grid-template-columns: repeat(12, 1fr);
  gap: 3px;
}

.focus-debt span {
  height: 18px;
  border-radius: 4px;
  background: var(--secondary);
  border: 1px solid var(--border);
}

.focus-debt span.active {
  background: rgba(239, 68, 68, 0.22);
  border-color: rgba(239, 68, 68, 0.35);
}

.author-detail-row {
  grid-column: 1 / -1;
  display: grid;
  grid-template-columns: minmax(190px, 1fr) minmax(180px, 0.9fr) minmax(240px, 1.1fr);
  align-items: end;
  gap: 14px;
  padding-top: 8px;
  border-top: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
}

.after-hour-bars {
  height: 42px;
  display: flex;
  align-items: end;
  gap: 4px;
}

.after-hour-bars span {
  width: 14px;
  min-height: 5px;
  border-radius: 4px 4px 0 0;
  background: linear-gradient(180deg, #fbbf24, #fb7185 62%, #be123c);
  position: relative;
}

.after-hour-bars small {
  position: absolute;
  left: 50%;
  bottom: -13px;
  transform: translateX(-50%);
  color: var(--muted-foreground);
  font-size: 8px;
}

.hot-file-row {
  justify-content: space-between;
  gap: 12px;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--background) 74%, transparent);
  padding: 10px 12px;
}

.hot-stats {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--muted-foreground);
  font-size: 10px;
  flex-shrink: 0;
}

.hot-stats strong {
  color: #ef4444;
}

.empty-state {
  margin-top: 12px;
  color: var(--muted-foreground);
  font-size: 12px;
  border: 1px dashed var(--border);
  padding: 16px;
  text-align: center;
}

@media (max-width: 1180px) {
  .metric-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .explain-grid,
  .stress-grid,
  .author-detail-row {
    grid-template-columns: minmax(0, 1fr);
  }

  .author-row {
    grid-template-columns: minmax(0, 1fr);
  }

  .risk-block {
    justify-content: flex-start;
  }
}
</style>

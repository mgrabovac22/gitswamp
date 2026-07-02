<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  AlertCircle,
  CircleDot,
  Clock,
  ExternalLink,
  FolderOpen,
  GitPullRequest,
  RefreshCw,
  Settings,
  UserRound,
} from "lucide-vue-next";

interface LandingRepositorySummary {
  name: string;
  path: string;
  branch: string;
  owner?: string;
}

interface DashboardItem {
  number: number;
  title: string;
  repo: string;
  url: string;
  updatedAt: string;
}

const props = defineProps<{
  githubToken?: string | null;
  openRepos: LandingRepositorySummary[];
  recentRepos: LandingRepositorySummary[];
}>();

const emit = defineEmits<{
  settings: [];
}>();

const loading = ref(false);
const error = ref("");
const githubLogin = ref("");
const pullRequests = ref<DashboardItem[]>([]);
const issues = ref<DashboardItem[]>([]);
const lastLoadedAt = ref("");
let loadSequence = 0;

const hasGithubToken = computed(() => !!props.githubToken?.trim());
const localStats = computed(() => [
  { label: "Open repos", value: props.openRepos.length },
  { label: "Recent repos", value: props.recentRepos.length },
  { label: "Active branches", value: new Set(props.openRepos.map((repo) => repo.branch).filter(Boolean)).size },
]);

const primaryLocalRepo = computed(() => props.openRepos[0] || props.recentRepos[0] || null);

function githubHeaders(token: string): Record<string, string> {
  return {
    Accept: "application/vnd.github+json",
    Authorization: `Bearer ${token}`,
    "X-GitHub-Api-Version": "2022-11-28",
  };
}

async function fetchGithubJson(url: string, token: string, signal: AbortSignal): Promise<unknown> {
  const response = await fetch(url, {
    headers: githubHeaders(token),
    signal,
  });

  if (!response.ok) {
    throw new Error(`GitHub HTTP ${response.status}`);
  }

  return response.json();
}

function repoFromApiUrl(value: unknown): string {
  if (typeof value !== "string") {
    return "";
  }

  const parts = value.split("/").filter(Boolean);
  if (parts.length < 2) {
    return "";
  }

  return `${parts[parts.length - 2]}/${parts[parts.length - 1]}`;
}

function mapDashboardItem(item: unknown, allowPullRequest: boolean): DashboardItem | null {
  if (!item || typeof item !== "object") {
    return null;
  }

  const value = item as Record<string, unknown>;
  if (!allowPullRequest && value.pull_request) {
    return null;
  }

  const number = Number(value.number);
  const title = String(value.title || "").trim();
  const url = String(value.html_url || "");
  if (!Number.isFinite(number) || !title || !url) {
    return null;
  }

  const repository = value.repository as Record<string, unknown> | undefined;
  const repo = String(repository?.full_name || "") || repoFromApiUrl(value.repository_url);

  return {
    number,
    title,
    repo: repo || "GitHub",
    url,
    updatedAt: String(value.updated_at || ""),
  };
}

function searchPullRequestUrl(login: string): string {
  const params = new URLSearchParams({
    q: `is:pr is:open author:${login}`,
    sort: "updated",
    order: "desc",
    per_page: "6",
  });
  return `https://api.github.com/search/issues?${params.toString()}`;
}

async function loadGithubDashboard() {
  const token = props.githubToken?.trim() || "";
  const sequence = ++loadSequence;

  if (!token) {
    githubLogin.value = "";
    pullRequests.value = [];
    issues.value = [];
    error.value = "";
    loading.value = false;
    return;
  }

  loading.value = true;
  error.value = "";
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(() => controller.abort(), 5500);

  try {
    const user = await fetchGithubJson("https://api.github.com/user", token, controller.signal) as Record<string, unknown>;
    const login = String(user.login || "").trim();
    if (!login) {
      throw new Error("GitHub user could not be read.");
    }

    const [assignedPayload, pullRequestPayload] = await Promise.all([
      fetchGithubJson("https://api.github.com/issues?filter=assigned&state=open&sort=updated&per_page=6", token, controller.signal),
      fetchGithubJson(searchPullRequestUrl(login), token, controller.signal),
    ]);

    if (sequence !== loadSequence) {
      return;
    }

    const assignedItems = Array.isArray(assignedPayload) ? assignedPayload : [];
    const searchItems = Array.isArray((pullRequestPayload as { items?: unknown }).items)
      ? ((pullRequestPayload as { items: unknown[] }).items)
      : [];

    githubLogin.value = login;
    issues.value = assignedItems
      .map((item) => mapDashboardItem(item, false))
      .filter((item): item is DashboardItem => item !== null)
      .slice(0, 6);
    pullRequests.value = searchItems
      .map((item) => mapDashboardItem(item, true))
      .filter((item): item is DashboardItem => item !== null)
      .slice(0, 6);
    lastLoadedAt.value = new Date().toLocaleTimeString();
  } catch (loadError) {
    if (sequence === loadSequence) {
      githubLogin.value = "";
      pullRequests.value = [];
      issues.value = [];
      error.value = loadError instanceof Error && loadError.name === "AbortError"
        ? "GitHub dashboard timed out."
        : String(loadError);
    }
  } finally {
    globalThis.clearTimeout(timeoutId);
    if (sequence === loadSequence) {
      loading.value = false;
    }
  }
}

function formatDate(value: string): string {
  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) {
    return "recent";
  }

  const deltaMinutes = Math.max(1, Math.round((Date.now() - timestamp) / 60000));
  if (deltaMinutes < 60) {
    return `${deltaMinutes}m ago`;
  }

  const deltaHours = Math.round(deltaMinutes / 60);
  if (deltaHours < 48) {
    return `${deltaHours}h ago`;
  }

  return `${Math.round(deltaHours / 24)}d ago`;
}

function openDashboardItem(item: DashboardItem) {
  openUrl(item.url).catch(() => {});
}

watch(() => props.githubToken, () => {
  void loadGithubDashboard();
}, { immediate: true });
</script>

<template>
  <aside class="min-w-0 space-y-3">
    <div class="rounded-lg border border-[var(--border)] bg-[var(--card)]">
      <div class="px-4 py-3 border-b border-[var(--border)] flex items-center justify-between gap-3">
        <div class="min-w-0">
          <div class="flex items-center gap-2 text-sm font-semibold text-[var(--foreground)]">
            <UserRound class="w-4 h-4 text-[var(--primary)]" />
            <span>Work Dashboard</span>
          </div>
          <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5 truncate">
            <span v-if="hasGithubToken && githubLogin">GitHub: @{{ githubLogin }}</span>
            <span v-else-if="hasGithubToken">GitHub work queue</span>
            <span v-else>Local workspace overview</span>
          </div>
        </div>
        <button
          v-if="hasGithubToken"
          class="p-1.5 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors disabled:opacity-50"
          :disabled="loading"
          title="Refresh dashboard"
          @click="loadGithubDashboard"
        >
          <RefreshCw class="w-3.5 h-3.5" :class="loading ? 'animate-spin' : ''" />
        </button>
        <button
          v-else
          class="p-1.5 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors"
          title="Open settings"
          @click="emit('settings')"
        >
          <Settings class="w-3.5 h-3.5" />
        </button>
      </div>

      <div class="p-3 space-y-3">
        <div class="grid grid-cols-3 gap-2">
          <div class="rounded border border-[var(--border)] bg-[var(--background)] px-2 py-2">
            <div class="text-[10px] text-[var(--muted-foreground)]">My PRs</div>
            <div class="text-lg font-semibold text-[var(--foreground)]">{{ hasGithubToken ? pullRequests.length : "-" }}</div>
          </div>
          <div class="rounded border border-[var(--border)] bg-[var(--background)] px-2 py-2">
            <div class="text-[10px] text-[var(--muted-foreground)]">Assigned</div>
            <div class="text-lg font-semibold text-[var(--foreground)]">{{ hasGithubToken ? issues.length : "-" }}</div>
          </div>
          <div class="rounded border border-[var(--border)] bg-[var(--background)] px-2 py-2">
            <div class="text-[10px] text-[var(--muted-foreground)]">Repos</div>
            <div class="text-lg font-semibold text-[var(--foreground)]">{{ recentRepos.length }}</div>
          </div>
        </div>

        <div v-if="error" class="flex items-start gap-2 rounded border border-[#f59e0b]/30 bg-[#f59e0b]/10 px-2.5 py-2 text-[11px] text-[var(--foreground)]">
          <AlertCircle class="w-3.5 h-3.5 mt-0.5 text-[#f59e0b] flex-shrink-0" />
          <span class="min-w-0">{{ error }}</span>
        </div>

        <template v-if="hasGithubToken">
          <section class="space-y-1.5">
            <div class="flex items-center justify-between gap-2 text-[11px] font-semibold text-[var(--foreground)]">
              <span class="flex items-center gap-1.5"><GitPullRequest class="w-3.5 h-3.5 text-[var(--primary)]" />My open pull requests</span>
              <span v-if="lastLoadedAt" class="text-[9px] font-normal text-[var(--muted-foreground)]">{{ lastLoadedAt }}</span>
            </div>
            <button
              v-for="item in pullRequests"
              :key="`pr-${item.repo}-${item.number}`"
              class="w-full text-left rounded border border-[var(--border)] bg-[var(--background)] px-2.5 py-2 hover:border-[var(--primary)]/45 hover:bg-[var(--secondary)] transition-colors"
              @click="openDashboardItem(item)"
            >
              <div class="flex items-center justify-between gap-2 text-[10px]">
                <span class="text-[var(--primary)] font-semibold">#{{ item.number }} {{ item.repo }}</span>
                <span class="text-[var(--muted-foreground)]">{{ formatDate(item.updatedAt) }}</span>
              </div>
              <div class="mt-0.5 text-[11px] text-[var(--foreground)] truncate">{{ item.title }}</div>
            </button>
            <div v-if="!loading && pullRequests.length === 0" class="rounded border border-[var(--border)] bg-[var(--background)] px-2.5 py-2 text-[11px] text-[var(--muted-foreground)]">
              No open pull requests authored by you.
            </div>
          </section>

          <section class="space-y-1.5">
            <div class="text-[11px] font-semibold text-[var(--foreground)] flex items-center gap-1.5">
              <CircleDot class="w-3.5 h-3.5 text-[var(--primary)]" />
              Issues assigned to me
            </div>
            <button
              v-for="item in issues"
              :key="`issue-${item.repo}-${item.number}`"
              class="w-full text-left rounded border border-[var(--border)] bg-[var(--background)] px-2.5 py-2 hover:border-[var(--primary)]/45 hover:bg-[var(--secondary)] transition-colors"
              @click="openDashboardItem(item)"
            >
              <div class="flex items-center justify-between gap-2 text-[10px]">
                <span class="text-[var(--primary)] font-semibold">#{{ item.number }} {{ item.repo }}</span>
                <span class="text-[var(--muted-foreground)]">{{ formatDate(item.updatedAt) }}</span>
              </div>
              <div class="mt-0.5 text-[11px] text-[var(--foreground)] truncate">{{ item.title }}</div>
            </button>
            <div v-if="!loading && issues.length === 0" class="rounded border border-[var(--border)] bg-[var(--background)] px-2.5 py-2 text-[11px] text-[var(--muted-foreground)]">
              No open issues assigned to you.
            </div>
          </section>
        </template>

        <template v-else>
          <section class="space-y-2">
            <div class="grid grid-cols-3 gap-2">
              <div v-for="stat in localStats" :key="stat.label" class="rounded border border-[var(--border)] bg-[var(--background)] px-2 py-2">
                <div class="text-[10px] text-[var(--muted-foreground)] truncate">{{ stat.label }}</div>
                <div class="text-base font-semibold text-[var(--foreground)]">{{ stat.value }}</div>
              </div>
            </div>

            <div class="rounded border border-[var(--border)] bg-[var(--background)] px-3 py-2">
              <div class="text-[11px] font-semibold text-[var(--foreground)] flex items-center gap-1.5">
                <FolderOpen class="w-3.5 h-3.5 text-[var(--primary)]" />
                {{ primaryLocalRepo ? primaryLocalRepo.name : 'No active repository' }}
              </div>
              <div class="mt-1 text-[10px] text-[var(--muted-foreground)] truncate">
                {{ primaryLocalRepo ? primaryLocalRepo.path : 'Browse, clone, or init a repository to start.' }}
              </div>
            </div>

            <button
              class="w-full flex items-center justify-between gap-2 rounded border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-left hover:border-[var(--primary)]/45 hover:bg-[var(--secondary)] transition-colors"
              @click="emit('settings')"
            >
              <span class="min-w-0">
                <span class="block text-[11px] font-semibold text-[var(--foreground)]">Connect GitHub for assigned work</span>
                <span class="block text-[10px] text-[var(--muted-foreground)] truncate">Show your PRs and assigned issues here.</span>
              </span>
              <ExternalLink class="w-3.5 h-3.5 text-[var(--muted-foreground)] flex-shrink-0" />
            </button>
          </section>
        </template>

        <div v-if="loading" class="flex items-center gap-2 text-[11px] text-[var(--muted-foreground)]">
          <RefreshCw class="w-3.5 h-3.5 animate-spin" />
          Loading GitHub dashboard...
        </div>
      </div>
    </div>

    <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] px-4 py-3">
      <div class="flex items-center gap-2 text-[11px] font-semibold text-[var(--foreground)]">
        <Clock class="w-3.5 h-3.5 text-[var(--primary)]" />
        Current Session
      </div>
      <div class="mt-2 space-y-1 text-[10px] text-[var(--muted-foreground)]">
        <div class="flex justify-between gap-2"><span>Open repositories</span><span class="text-[var(--foreground)]">{{ openRepos.length }}</span></div>
        <div class="flex justify-between gap-2"><span>Recent repositories</span><span class="text-[var(--foreground)]">{{ recentRepos.length }}</span></div>
        <div class="flex justify-between gap-2"><span>GitHub dashboard</span><span class="text-[var(--foreground)]">{{ hasGithubToken ? 'Connected' : 'Local only' }}</span></div>
      </div>
    </div>
  </aside>
</template>

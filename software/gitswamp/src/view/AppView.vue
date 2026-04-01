<script setup lang="ts">
import TitleBar from "@/view/shell/TitleBar.vue";
import AppHeader from "@/view/shell/AppHeader.vue";
import RepositoryTabs from "@/view/repository/RepositoryTabs.vue";
import RepositoryWorkspace from "@/view/repository/RepositoryWorkspace.vue";
import LandingPage from "@/view/repository/LandingPage.vue";
import RepositoryActionDialogs from "@/view/repository/RepositoryActionDialogs.vue";
import RepositoryAuthDialogs from "@/view/repository/RepositoryAuthDialogs.vue";
import CloneDialog from "@/view/repository/CloneDialog.vue";
import InitDialog from "@/view/repository/InitDialog.vue";
import GhostBranchDialog from "@/view/repository/GhostBranchDialog.vue";
import MultiPlatformPushDialog from "@/view/repository/MultiPlatformPushDialog.vue";
import SettingsDialog from "@/view/shell/SettingsDialog.vue";
import ToastContainer from "@/shared/ui/ToastContainer.vue";
import {
  applyAppPalettePreference,
  applyThemeModePreference,
  getStoredAppPalettePreference,
  getStoredThemeModePreference,
} from "@/shared/themePreferences";
import { useGit } from "@/domain/git/UseGit";
import { useToast } from "@/shared/notifications/useToast";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { ref, watch, onMounted, onUnmounted, computed } from "vue";
import type { RepoInfo, CommitInfo, StashInfo, RemoteInfo, IssueInfo, PullRequestInfo } from "@/types";

const git = useGit();
const toast = useToast();
const appWindow = getCurrentWindow();

applyThemeModePreference(getStoredThemeModePreference());
applyAppPalettePreference(getStoredAppPalettePreference());

const savedFontSize = localStorage.getItem("gitswamp-font-size");
if (savedFontSize) {
  const fontSizes: Record<string, string> = { small: "14px", medium: "16px", large: "18px" };
  document.documentElement.style.setProperty("--font-size", fontSizes[savedFontSize] || "16px");
}
const savedCompact = localStorage.getItem("gitswamp-compact-mode");
if (savedCompact === "true") {
  document.documentElement.classList.add("compact");
}
const savedDummyMode = localStorage.getItem("gitswamp-dummy-mode");
if (savedDummyMode === "true") {
  document.documentElement.classList.add("dummy-mode");
}
const savedAvatars = localStorage.getItem("gitswamp-show-avatars");
if (savedAvatars === "false") {
  document.documentElement.classList.add("hide-avatars");
}
const savedReducedMotion = localStorage.getItem("gitswamp-reduced-motion");
if (savedReducedMotion === "true") {
  document.documentElement.classList.add("reduced-motion");
}
const savedWrapDiffLines = localStorage.getItem("gitswamp-wrap-diff-lines");
if (savedWrapDiffLines === "true") {
  document.documentElement.classList.add("diff-wrap-lines");
}
const savedShowDiffLineNumbers = localStorage.getItem("gitswamp-show-diff-line-numbers");
if (savedShowDiffLineNumbers === "false") {
  document.documentElement.classList.add("hide-diff-line-numbers");
}

const STARTUP_FULLSCREEN_KEY = "gitswamp-fullscreen-on-start";
const RESTORE_SESSION_KEY = "gitswamp-restore-session";

function shouldStartMaximized(): boolean {
  const saved = localStorage.getItem(STARTUP_FULLSCREEN_KEY);
  if (saved === null) return true;
  return saved !== "false";
}

function shouldRestoreSession(): boolean {
  const saved = localStorage.getItem(RESTORE_SESSION_KEY);
  if (saved === null) {
    localStorage.setItem(RESTORE_SESSION_KEY, "true");
    return true;
  }
  return saved === "true";
}

interface Tab {
  id: string;
  repo: RepoInfo | null;
  label: string;
  path: string;
}

type HistoryViewMode = "graph" | "productivity" | "time-machine" | "conflict-heatmap" | "remote-insights" | "conflict-resolve";
type RemoteInsightsViewMode = "pull-request-detail" | "pull-request-create" | "issue-detail" | "issue-create";

const tabs = ref<Tab[]>([
  { id: "landing", repo: null, label: "Start", path: "" },
]);
const activeTabId = ref("landing");
const historyViewMode = ref<HistoryViewMode>("graph");
const timeMachineFocusSha = ref<string | null>(null);
const showCloneDialog = ref(false);
const showInitDialog = ref(false);
const showTerminal = ref(false);
const terminalAllowAll = ref(localStorage.getItem("gitswamp-terminal-allow-all") === "true");
const activeRemoteAction = ref<"pull" | "push" | "fetch" | null>(null);
const showBranchDialog = ref(false);
const showGhostMaterializeDialog = ref(false);
const ghostMaterializeName = ref("");
const showStashDialog = ref(false);
const showSettings = ref(false);
const newBranchName = ref("");
const stashMessage = ref("");
const showEditMessageDialog = ref(false);
const editMessageSha = ref("");
const editMessageText = ref("");
const showRenameDialog = ref(false);
const renameBranchOld = ref("");
const renameBranchNew = ref("");
const showAnnotatedTagDialog = ref(false);
const annotatedTagSha = ref("");
const annotatedTagName = ref("");
const annotatedTagMessage = ref("");

const viewingWorkingChanges = ref(false);
const viewingStash = ref(false);

const showDiffViewer = ref(false);
const diffFilePath = ref("");
const diffCommitSha = ref<string | null>(null);
const diffStaged = ref(false);

const conflictResolverPath = ref("");

const showMultiPlatformPushDialog = ref(false);
const multiPlatformPushRepoName = ref("");
const showPushUsernameDialog = ref(false);
const pushPlatform = ref("");
const pushUsername = ref("");
const pushDomain = ref("");
const detailsPanelCollapsed = ref(true);
const openPullRequestBranches = ref<string[]>([]);
const githubIssues = ref<IssueInfo[]>([]);
const githubPullRequests = ref<PullRequestInfo[]>([]);
const selectedIssueNumber = ref<number | null>(null);
const selectedPullRequestNumber = ref<number | null>(null);
const remoteInsightsMode = ref<RemoteInsightsViewMode>("pull-request-detail");
let pullRequestFetchSequence = 0;
let pullRequestFetchTimer: ReturnType<typeof setTimeout> | null = null;

const showAuthRequiredDialog = ref(false);
const authProvider = ref<"github" | "gitlab" | "gitlab-self">("github");
const authTokenInput = ref("");
const authDomainInput = ref("");
const authEmailInput = ref("");
const authKeyNameInput = ref("gitswamp");
const authSubmitting = ref(false);

function openDiffViewer(filePath: string, commitSha: string | null, staged: boolean) {
  diffFilePath.value = filePath;
  diffCommitSha.value = commitSha;
  diffStaged.value = staged;
  showDiffViewer.value = true;
}

function closeDiffViewer() {
  showDiffViewer.value = false;
}

async function openConflictResolver(filePath: string) {
  try {
    toast.action(
      "warning",
      `Resolve conflict: ${filePath}`,
      [
        {
          label: "Resolve Manually",
          style: "warning",
          onClick: async () => {
            conflictResolverPath.value = filePath;
            setHistoryViewMode("conflict-resolve");
          },
        },
        {
          label: "Keep Modified",
          style: "success",
          onClick: async () => {
            await resolveConflict(filePath, "keep-modified");
          },
        },
        {
          label: "Keep Base",
          style: "primary",
          onClick: async () => {
            await resolveConflict(filePath, "keep-base");
          },
        },
        {
          label: "Delete File",
          style: "danger",
          onClick: async () => {
            await resolveConflict(filePath, "delete");
          },
        },
        {
          label: "Cancel",
          style: "neutral",
          onClick: async () => {},
        },
      ],
      30000,
    );
  } catch (e) {
    toast.error(`Error opening conflict resolver: ${String(e)}`);
  }
}

function closeConflictResolver() {
  conflictResolverPath.value = "";
  if (historyViewMode.value === "conflict-resolve") {
    setHistoryViewMode("graph");
  }
}

function onConflictResolved() {
  conflictResolverPath.value = "";
  if (historyViewMode.value === "conflict-resolve") {
    setHistoryViewMode("graph");
  }
  git.refreshAll();
  toast.success("Conflict resolved and file staged");
}

async function resolveConflict(filePath: string, resolution: 'keep-modified' | 'keep-base' | 'delete') {
  try {
    let strategy: "ours" | "theirs" | "delete";
    if (resolution === "keep-modified") {
      strategy = "ours";
    } else if (resolution === "keep-base") {
      strategy = "theirs";
    } else {
      strategy = "delete";
    }

    // Use backend to resolve conflict properly
    await invoke('resolve_conflict_file', {
      path: git.repoPath.value,
      filePath: filePath,
      strategy: strategy,
    });

    await git.refreshAll();
    toast.success(`Conflict resolved: ${filePath}`);
  } catch (e) {
    toast.error(`Failed to resolve conflict: ${String(e)}`);
  }
}

function normalizeHostInput(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";

  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  try {
    const parsed = new URL(candidate);
    return parsed.host.toLowerCase();
  } catch {
    const withoutProtocol = trimmed.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "");
    return (withoutProtocol.split("/")[0] || "").trim().toLowerCase();
  }
}

function parseStoredGitlabSelfToken(): { domain: string; token: string } | null {
  const raw = git.providerTokens.value["gitlab-self"];
  if (!raw) return null;

  const separatorIndex = raw.indexOf("|");
  if (separatorIndex <= 0) return null;

  const domain = normalizeHostInput(raw.slice(0, separatorIndex));
  const token = raw.slice(separatorIndex + 1).trim();
  if (!domain || !token) return null;

  return { domain, token };
}

function remoteMatchesDomain(host: string, hostWithPort: string, domainRaw: string): boolean {
  const normalizedDomain = normalizeHostInput(domainRaw);
  const normalizedHost = normalizeHostInput(host);
  const normalizedHostWithPort = normalizeHostInput(hostWithPort);

  if (!normalizedDomain || !normalizedHost || !normalizedHostWithPort) {
    return false;
  }

  if (normalizedDomain === normalizedHost || normalizedDomain === normalizedHostWithPort) {
    return true;
  }

  const domainHostOnly = normalizedDomain.split(":")[0];
  const hostOnly = normalizedHost.split(":")[0];
  const hostWithPortOnly = normalizedHostWithPort.split(":")[0];
  return domainHostOnly === hostOnly || domainHostOnly === hostWithPortOnly;
}

function detectAuthProviderFromOrigin(): "github" | "gitlab" | "gitlab-self" {
  const origin = git.repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url || "";
  const originLower = origin.toLowerCase();
  const parsed = parseRemoteHostAndPath(origin);

  if ((parsed?.host || "") === "gitlab.com" || originLower.includes("gitlab.com")) {
    return "gitlab";
  }

  if ((parsed?.host || "").includes("gitlab.") || originLower.includes("gitlab.")) {
    return "gitlab-self";
  }

  const selfHostedToken = parseStoredGitlabSelfToken();
  if (parsed && selfHostedToken && remoteMatchesDomain(parsed.host, parsed.hostWithPort, selfHostedToken.domain)) {
    return "gitlab-self";
  }

  return "github";
}

function parseAuthDomainFromOrigin(): string {
  const origin = git.repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url || "";
  const parsed = parseRemoteHostAndPath(origin);
  if (parsed && parsed.hostWithPort) return parsed.hostWithPort;

  const noProto = origin.replace(/^https?:\/\//i, "");
  const noCreds = noProto.includes("@") ? noProto.split("@")[1] : noProto;
  const firstSegment = (noCreds.split("/")[0] || "").trim();
  if (!firstSegment) return "";

  const parts = firstSegment.split(":");
  if (parts.length >= 2 && /^\d+$/.test(parts[1])) {
    return `${parts[0].toLowerCase()}:${parts[1]}`;
  }

  return parts[0].toLowerCase();
}

function normalizeBranchName(name: string): string {
  return name
    .replace(/^origin\//i, "")
    .replace(/^remotes\/[a-z0-9_-]+\//i, "")
    .trim();
}

function uniqueNormalizedBranches(branches: string[]): string[] {
  const values = new Set<string>();
  for (const branch of branches) {
    const normalized = normalizeBranchName(branch).toLowerCase();
    if (normalized) {
      values.add(normalized);
    }
  }
  return Array.from(values);
}

function stripGitSuffix(path: string): string {
  return path.replace(/\.git$/i, "");
}

function parseRemoteHostAndPath(remoteUrl: string): { host: string; hostWithPort: string; path: string } | null {
  const value = remoteUrl.trim();
  if (!value) return null;

  const scpLikeMatch = value.match(/^[^@]+@([^:]+):(.+)$/);
  if (scpLikeMatch) {
    const host = scpLikeMatch[1].toLowerCase();
    return {
      host,
      hostWithPort: host,
      path: stripGitSuffix(scpLikeMatch[2].replace(/^\/+/, "")),
    };
  }

  try {
    const parsed = new URL(value);
    return {
      host: parsed.hostname.toLowerCase(),
      hostWithPort: parsed.host.toLowerCase(),
      path: stripGitSuffix(parsed.pathname.replace(/^\/+/, "")),
    };
  } catch {
    return null;
  }
}

function parseGithubRemote(remoteUrl: string): { host: string; hostWithPort: string; owner: string; repo: string } | null {
  const parsed = parseRemoteHostAndPath(remoteUrl);
  if (!parsed) return null;

  const segments = parsed.path.split("/").filter(Boolean);
  if (segments.length < 2) return null;

  return {
    host: parsed.host,
    hostWithPort: parsed.hostWithPort,
    owner: segments[segments.length - 2],
    repo: segments[segments.length - 1],
  };
}

function parseGitlabRemote(remoteUrl: string): { host: string; hostWithPort: string; projectPath: string } | null {
  const parsed = parseRemoteHostAndPath(remoteUrl);
  if (!parsed) return null;

  const segments = parsed.path.split("/").filter(Boolean);
  if (segments.length < 2) return null;

  return {
    host: parsed.host,
    hostWithPort: parsed.hostWithPort,
    projectPath: segments.join("/"),
  };
}

function getPrimaryRemote(remotes: RemoteInfo[]): RemoteInfo | null {
  return remotes.find((remote) => remote.name === "origin") || remotes[0] || null;
}

function getGitlabTokenForHost(host: string, hostWithPort: string = host): string | null {
  if (host === "gitlab.com") {
    return git.providerTokens.value.gitlab || null;
  }

  const selfHostedToken = parseStoredGitlabSelfToken();
  if (selfHostedToken && remoteMatchesDomain(host, hostWithPort, selfHostedToken.domain)) {
    return selfHostedToken.token;
  }

  const selfHostedLegacy = git.providerTokens.value["gitlab-self"];
  if (selfHostedLegacy && !selfHostedToken) {
    return selfHostedLegacy;
  }

  return git.providerTokens.value.gitlab || null;
}

function collectChangedWorkingPaths(): string[] {
  const values = new Set<string>();
  const addPath = (input: string | undefined) => {
    const normalized = (input || "").replace(/\\/g, "/").trim();
    if (normalized) {
      values.add(normalized);
    }
  };

  for (const file of git.stagedFiles.value) addPath(file.path);
  for (const file of git.unstagedFiles.value) addPath(file.path);
  for (const file of git.conflictFiles.value) addPath(file.path);

  return Array.from(values);
}

const originConflictRisk = computed(() => {
  const current = git.currentBranch.value;
  if (!current) {
    return { level: "none" as const, label: "No active branch" };
  }

  const local = git.localBranches.value.find((branch) => branch.name === current || branch.is_head);
  const behind = local?.behind || 0;
  const ahead = local?.ahead || 0;
  const changedFiles = collectChangedWorkingPaths().length;

  if (git.hasConflicts.value) {
    return { level: "high" as const, label: "Unresolved conflicts are present." };
  }

  if (behind > 0 && changedFiles > 0) {
    return {
      level: "high" as const,
      label: `Current branch is behind origin by ${behind} commit(s) while ${changedFiles} local file(s) are modified.`,
    };
  }

  if (behind > 0 || (ahead > 0 && changedFiles > 0)) {
    return {
      level: "medium" as const,
      label: `Branch divergence detected (ahead ${ahead}, behind ${behind}).`,
    };
  }

  return { level: "none" as const, label: "No origin conflict risk." };
});

interface GithubApiContext {
  apiBase: string;
  owner: string;
  repo: string;
  token: string;
}

async function fetchJsonWithTimeout(url: string, headers: Record<string, string>): Promise<unknown> {
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(() => controller.abort(), 8000);

  try {
    const response = await fetch(url, {
      headers,
      signal: controller.signal,
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    return await response.json();
  } finally {
    globalThis.clearTimeout(timeoutId);
  }
}

function getGithubApiContext(remoteUrl: string): GithubApiContext | null {
  const parsed = parseGithubRemote(remoteUrl);
  if (!parsed) return null;

  const apiBase = parsed.host === "github.com"
    ? "https://api.github.com"
    : `https://${parsed.hostWithPort}/api/v3`;

  return {
    apiBase,
    owner: parsed.owner,
    repo: parsed.repo,
    token: git.providerTokens.value.github || git.githubToken.value || "",
  };
}

function githubHeaders(token: string): Record<string, string> {
  const headers: Record<string, string> = {
    Accept: "application/vnd.github+json",
  };

  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  return headers;
}

function mapGithubPullRequest(item: unknown): PullRequestInfo | null {
  if (!item || typeof item !== "object") return null;
  const value = item as Record<string, unknown>;
  const head = (value.head as Record<string, unknown> | undefined) || {};
  const base = (value.base as Record<string, unknown> | undefined) || {};
  const user = (value.user as Record<string, unknown> | undefined) || {};

  const id = Number(value.id);
  const number = Number(value.number);
  const title = String(value.title || "").trim();
  if (!Number.isFinite(id) || !Number.isFinite(number) || !title) {
    return null;
  }

  return {
    id,
    number,
    title,
    state: String(value.state || "unknown"),
    draft: Boolean(value.draft),
    author: String(user.login || "Unknown"),
    sourceBranch: String(head.ref || ""),
    targetBranch: String(base.ref || ""),
    createdAt: String(value.created_at || ""),
    updatedAt: String(value.updated_at || ""),
    url: String(value.html_url || ""),
    description: String(value.body || ""),
  };
}

function mapGithubIssue(item: unknown): IssueInfo | null {
  if (!item || typeof item !== "object") return null;
  const value = item as Record<string, unknown>;
  if (value.pull_request) {
    return null;
  }

  const user = (value.user as Record<string, unknown> | undefined) || {};
  const id = Number(value.id);
  const number = Number(value.number);
  const title = String(value.title || "").trim();
  if (!Number.isFinite(id) || !Number.isFinite(number) || !title) {
    return null;
  }

  return {
    id,
    number,
    title,
    state: String(value.state || "unknown"),
    author: String(user.login || "Unknown"),
    createdAt: String(value.created_at || ""),
    updatedAt: String(value.updated_at || ""),
    url: String(value.html_url || ""),
    description: String(value.body || ""),
  };
}

async function fetchGithubPullRequests(remoteUrl: string): Promise<PullRequestInfo[]> {
  const context = getGithubApiContext(remoteUrl);
  if (!context) return [];

  const payload = await fetchJsonWithTimeout(
    `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/pulls?state=open&per_page=100`,
    githubHeaders(context.token),
  );

  if (!Array.isArray(payload)) return [];
  return payload
    .map(mapGithubPullRequest)
    .filter((item): item is PullRequestInfo => item !== null);
}

async function fetchGithubIssues(remoteUrl: string): Promise<IssueInfo[]> {
  const context = getGithubApiContext(remoteUrl);
  if (!context) return [];

  const issues: IssueInfo[] = [];

  for (let page = 1; page <= 5 && issues.length < 100; page += 1) {
    const { mapped, rawCount } = await fetchGithubIssuesPage(context, page);
    if (rawCount === 0) break;

    issues.push(...mapped);
    if (rawCount < 100) break;
  }

  return issues.slice(0, 100);
}

async function fetchGithubIssuesPage(context: GithubApiContext, page: number): Promise<{ mapped: IssueInfo[]; rawCount: number }> {
  const payload = await fetchJsonWithTimeout(
    `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/issues?state=open&per_page=100&page=${page}`,
    githubHeaders(context.token),
  );

  if (!Array.isArray(payload)) {
    return { mapped: [], rawCount: 0 };
  }

  return {
    mapped: payload
      .map(mapGithubIssue)
      .filter((item): item is IssueInfo => item !== null),
    rawCount: payload.length,
  };
}

async function fetchOpenGitlabMergeRequestBranches(remoteUrl: string): Promise<string[]> {
  const parsed = parseGitlabRemote(remoteUrl);
  if (!parsed) return [];

  const token = getGitlabTokenForHost(parsed.host, parsed.hostWithPort) || "";
  const headers: Record<string, string> = {
    Accept: "application/json",
  };

  if (token) {
    headers["PRIVATE-TOKEN"] = token;
  }

  const payload = await fetchJsonWithTimeout(
    `https://${parsed.hostWithPort}/api/v4/projects/${encodeURIComponent(parsed.projectPath)}/merge_requests?state=opened&per_page=100`,
    headers,
  );

  if (!Array.isArray(payload)) return [];
  return payload
    .map((item) => {
      if (!item || typeof item !== "object") return "";
      const candidate = (item as { source_branch?: unknown }).source_branch;
      return typeof candidate === "string" ? candidate : "";
    })
    .filter((name) => name.length > 0);
}

function ensureRemoteInsightSelection() {
  if (!githubPullRequests.value.some((pr) => pr.number === selectedPullRequestNumber.value)) {
    selectedPullRequestNumber.value = githubPullRequests.value[0]?.number ?? null;
  }

  if (!githubIssues.value.some((issue) => issue.number === selectedIssueNumber.value)) {
    selectedIssueNumber.value = githubIssues.value[0]?.number ?? null;
  }
}

async function refreshOpenPullRequestBranches() {
  const remotes = git.repoInfo.value?.remotes || [];
  const primaryRemote = getPrimaryRemote(remotes);

  if (!primaryRemote || (primaryRemote.provider !== "github" && primaryRemote.provider !== "gitlab")) {
    openPullRequestBranches.value = [];
    githubPullRequests.value = [];
    githubIssues.value = [];
    selectedPullRequestNumber.value = null;
    selectedIssueNumber.value = null;
    return;
  }

  const sequence = ++pullRequestFetchSequence;

  try {
    if (primaryRemote.provider === "github") {
      const [pullRequests, issues] = await Promise.all([
        fetchGithubPullRequests(primaryRemote.url),
        fetchGithubIssues(primaryRemote.url),
      ]);

      if (sequence !== pullRequestFetchSequence) return;

      githubPullRequests.value = pullRequests;
      githubIssues.value = issues;
      ensureRemoteInsightSelection();
      openPullRequestBranches.value = uniqueNormalizedBranches(
        pullRequests.map((item) => item.sourceBranch),
      );
      return;
    }

    const branches = await fetchOpenGitlabMergeRequestBranches(primaryRemote.url);

    if (sequence !== pullRequestFetchSequence) return;
    githubPullRequests.value = [];
    githubIssues.value = [];
    selectedPullRequestNumber.value = null;
    selectedIssueNumber.value = null;
    openPullRequestBranches.value = uniqueNormalizedBranches(branches);
  } catch {
    if (sequence !== pullRequestFetchSequence) return;
    openPullRequestBranches.value = [];
    githubPullRequests.value = [];
    githubIssues.value = [];
    selectedPullRequestNumber.value = null;
    selectedIssueNumber.value = null;
  }
}

function scheduleOpenPullRequestRefresh() {
  if (pullRequestFetchTimer) {
    clearTimeout(pullRequestFetchTimer);
  }

  pullRequestFetchTimer = setTimeout(() => {
    void refreshOpenPullRequestBranches();
  }, 250);
}

function promptGithubInsightsAuth() {
  authProvider.value = "github";
  authDomainInput.value = "";
  authTokenInput.value = "";
  showAuthRequiredDialog.value = true;
}

function getPrimaryGithubContext(): { apiBase: string; owner: string; repo: string; token: string } | null {
  const remotes = git.repoInfo.value?.remotes || [];
  const primaryRemote = getPrimaryRemote(remotes);
  if (!primaryRemote || primaryRemote.provider !== "github") {
    return null;
  }

  return getGithubApiContext(primaryRemote.url);
}

function openIssueDetails(number: number) {
  selectedIssueNumber.value = number;
  remoteInsightsMode.value = "issue-detail";
  setHistoryViewMode("remote-insights");
}

function openPullRequestDetails(number: number) {
  selectedPullRequestNumber.value = number;
  remoteInsightsMode.value = "pull-request-detail";
  setHistoryViewMode("remote-insights");
}

function openCreateIssuePanel() {
  remoteInsightsMode.value = "issue-create";
  setHistoryViewMode("remote-insights");
}

function openCreatePullRequestPanel() {
  remoteInsightsMode.value = "pull-request-create";
  setHistoryViewMode("remote-insights");
}

async function readGithubError(response: Response): Promise<string> {
  try {
    const payload = await response.json() as { message?: unknown };
    if (typeof payload.message === "string" && payload.message.trim()) {
      return payload.message.trim();
    }
  } catch {
    // Ignore JSON parse errors and fall back to HTTP status.
  }

  return `HTTP ${response.status}`;
}

async function createRemoteIssue(payload: { title: string; description: string }) {
  const context = getPrimaryGithubContext();
  if (!context) {
    toast.warning("GitHub remote is required for issue creation.");
    return;
  }

  if (!context.token) {
    promptGithubInsightsAuth();
    toast.warning("GitHub token is required to create issues.");
    return;
  }

  try {
    const response = await fetch(
      `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/issues`,
      {
        method: "POST",
        headers: {
          ...githubHeaders(context.token),
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          title: payload.title,
          body: payload.description || undefined,
        }),
      },
    );

    if (!response.ok) {
      if (response.status === 401 || response.status === 403) {
        promptGithubInsightsAuth();
      }
      throw new Error(await readGithubError(response));
    }

    const created = mapGithubIssue(await response.json());
    if (!created) {
      throw new Error("Created issue payload could not be parsed.");
    }

    githubIssues.value = [created, ...githubIssues.value.filter((item) => item.number !== created.number)];
    selectedIssueNumber.value = created.number;
    remoteInsightsMode.value = "issue-detail";
    setHistoryViewMode("remote-insights");
    toast.success(`Issue #${created.number} created.`);
    scheduleOpenPullRequestRefresh();
  } catch (e) {
    toast.error("Create issue failed: " + String(e));
  }
}

async function createRemotePullRequest(payload: { title: string; description: string; sourceBranch: string; targetBranch: string }) {
  const context = getPrimaryGithubContext();
  if (!context) {
    toast.warning("GitHub remote is required for pull request creation.");
    return;
  }

  if (!context.token) {
    promptGithubInsightsAuth();
    toast.warning("GitHub token is required to create pull requests.");
    return;
  }

  try {
    const response = await fetch(
      `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/pulls`,
      {
        method: "POST",
        headers: {
          ...githubHeaders(context.token),
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          title: payload.title,
          body: payload.description || undefined,
          head: payload.sourceBranch,
          base: payload.targetBranch,
        }),
      },
    );

    if (!response.ok) {
      if (response.status === 401 || response.status === 403) {
        promptGithubInsightsAuth();
      }
      throw new Error(await readGithubError(response));
    }

    const created = mapGithubPullRequest(await response.json());
    if (!created) {
      throw new Error("Created pull request payload could not be parsed.");
    }

    githubPullRequests.value = [
      created,
      ...githubPullRequests.value.filter((item) => item.number !== created.number),
    ];
    openPullRequestBranches.value = uniqueNormalizedBranches(
      githubPullRequests.value.map((item) => item.sourceBranch),
    );
    selectedPullRequestNumber.value = created.number;
    remoteInsightsMode.value = "pull-request-detail";
    setHistoryViewMode("remote-insights");
    toast.success(`Pull request #${created.number} created.`);
    scheduleOpenPullRequestRefresh();
  } catch (e) {
    toast.error("Create pull request failed: " + String(e));
  }
}

function isAuthenticationRequiredError(message: string | null | undefined): boolean {
  if (!message) return false;
  const m = message.toLowerCase();
  return m.includes("auth_required:") || m.includes("authentication") || m.includes("requires authentication") || m.includes("permission denied") || m.includes("http 401") || m.includes("status code: 401") || m.includes("401") || m.includes("http 403") || m.includes("status code: 403") || m.includes("forbidden") || m.includes("unauthorized");
}

function maybeShowAuthDialogFromGitError() {
  if (!isAuthenticationRequiredError(git.error.value)) return;
  authProvider.value = detectAuthProviderFromOrigin();
  authDomainInput.value = authProvider.value === "gitlab-self" ? parseAuthDomainFromOrigin() : "";
  authTokenInput.value = "";
  showAuthRequiredDialog.value = true;
}

async function syncAuthStateAfterChange() {
  await git.reloadAuthTokens();
  scheduleOpenPullRequestRefresh();
}

async function handleSaveProviderToken(provider: string, token: string) {
  await git.saveProviderToken(provider, token);
  await syncAuthStateAfterChange();
}

async function handleDeleteProviderToken(provider: string) {
  await git.deleteProviderToken(provider);
  await syncAuthStateAfterChange();
}

async function handleSaveGithubTokenFromSettings(token: string) {
  await handleSaveProviderToken("github", token);
  showSettings.value = false;
}

async function handlePull() {
  activeRemoteAction.value = "pull";
  try {
    await git.pull();
    maybeShowAuthDialogFromGitError();
  } finally {
    activeRemoteAction.value = null;
  }
}

async function handleFetch() {
  activeRemoteAction.value = "fetch";
  try {
    await git.fetchAll();
    maybeShowAuthDialogFromGitError();
  } finally {
    activeRemoteAction.value = null;
  }
}

function toggleTerminalPanel() {
  showTerminal.value = !showTerminal.value;
}

function setHistoryViewMode(mode: HistoryViewMode) {
  historyViewMode.value = mode;

  if (mode !== "time-machine") {
    timeMachineFocusSha.value = null;
  }

  if (mode !== "graph") {
    detailsPanelCollapsed.value = true;
    showDiffViewer.value = false;
    viewingWorkingChanges.value = false;
    viewingStash.value = false;
    git.selectedCommit.value = null;
    git.clearStashSelection();
  }

  if (mode !== "conflict-resolve") {
    conflictResolverPath.value = "";
  }
}

function dispatchFocusCommitSearch() {
  globalThis.dispatchEvent(new Event("gitswamp-focus-commit-search"));
}

function hasActiveRepositoryPath(): boolean {
  return !!git.repoPath.value;
}

async function openRepoInVsCode() {
  if (!hasActiveRepositoryPath()) {
    toast.warning("Open a repository first.");
    return;
  }

  try {
    await invoke("open_path_with_tool", {
      path: git.repoPath.value,
      tool: "vscode",
    });
    toast.success("Opened repository in VS Code");
  } catch (e) {
    toast.error("Failed to open in VS Code: " + String(e));
  }
}

async function openRepoInExplorer() {
  if (!hasActiveRepositoryPath()) {
    toast.warning("Open a repository first.");
    return;
  }

  try {
    await invoke("open_path_with_tool", {
      path: git.repoPath.value,
      tool: "explorer",
    });
    toast.success("Opened repository in folder explorer");
  } catch (e) {
    toast.error("Failed to open folder explorer: " + String(e));
  }
}

async function createGistFromRepo() {
  if (!hasActiveRepositoryPath()) {
    toast.warning("Open a repository first.");
    return;
  }

  try {
    await openUrl("https://gist.github.com/");
    toast.info("Opened GitHub Gist creator in browser");
  } catch (e) {
    toast.error("Failed to open Gist page: " + String(e));
  }
}

async function refreshCurrentRepo() {
  if (!hasActiveRepositoryPath()) {
    return;
  }

  await git.refreshAll();
  if (git.error.value) {
    toast.error(git.error.value);
    return;
  }

  toast.success("Repository refreshed");
}

function isEditableTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  if (!element) return false;
  const tag = element.tagName.toLowerCase();
  return element.isContentEditable || tag === "input" || tag === "textarea" || tag === "select" || tag === "option";
}

function isAltOnlyShortcut(event: KeyboardEvent): boolean {
  return event.altKey && !event.ctrlKey && !event.shiftKey;
}

function handleHistoryViewShortcut(event: KeyboardEvent, key: string): boolean {
  if (!isAltOnlyShortcut(event)) {
    return false;
  }

  const viewByKey: Partial<Record<string, HistoryViewMode>> = {
    "1": "graph",
    "2": "productivity",
    "3": "time-machine",
    "4": "conflict-heatmap",
  };

  const nextMode = viewByKey[key];
  if (!nextMode) {
    return false;
  }

  event.preventDefault();
  setHistoryViewMode(nextMode);
  return true;
}

function handleRepositoryShortcut(event: KeyboardEvent, key: string): boolean {
  if (event.ctrlKey && event.shiftKey && key === "o") {
    event.preventDefault();
    void openRepoInVsCode();
    return true;
  }

  if (isAltOnlyShortcut(event) && key === "o") {
    event.preventDefault();
    void openRepoInExplorer();
    return true;
  }

  if (event.ctrlKey && !event.shiftKey && key === "r") {
    event.preventDefault();
    dispatchFocusCommitSearch();
    return true;
  }

  if (event.ctrlKey && event.shiftKey && key === "g") {
    event.preventDefault();
    void createGistFromRepo();
    return true;
  }

  if (event.ctrlKey && !event.shiftKey && key === ",") {
    event.preventDefault();
    showSettings.value = true;
    return true;
  }

  return false;
}

function handleGlobalShortcuts(event: KeyboardEvent) {
  if (isEditableTarget(event.target)) {
    return;
  }

  const key = event.key.toLowerCase();

  if (event.ctrlKey && event.code === "Backquote") {
    event.preventDefault();
    toggleTerminalPanel();
    return;
  }

  if (handleHistoryViewShortcut(event, key)) {
    return;
  }

  handleRepositoryShortcut(event, key);
}

const recentRepos = ref<{ name: string; path: string; branch: string; owner?: string }[]>([]);

onMounted(() => {
  globalThis.addEventListener("keydown", handleGlobalShortcuts);

  const restoreSession = shouldRestoreSession();

  try {
    if (restoreSession) {
      const saved = localStorage.getItem("gitswamp-tabs");
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.tabs?.length) {
          tabs.value = parsed.tabs;
          activeTabId.value = parsed.activeTabId || tabs.value[0].id;
        }
      }
    }

    const savedRecent = localStorage.getItem("gitswamp-recent");
    if (savedRecent) {
      recentRepos.value = JSON.parse(savedRecent);
    }
  } catch {}

  if (restoreSession) {
    const active = tabs.value.find((t) => t.id === activeTabId.value);
    if (active?.path) {
      git.openRepository(active.path);
    }
  }

  if (shouldStartMaximized()) {
    appWindow.maximize().catch(() => {});
    setTimeout(() => {
      appWindow.maximize().catch(() => {});
    }, 120);
  }

});

onUnmounted(() => {
  globalThis.removeEventListener("keydown", handleGlobalShortcuts);
  pullRequestFetchSequence++;
  if (pullRequestFetchTimer) {
    clearTimeout(pullRequestFetchTimer);
    pullRequestFetchTimer = null;
  }

});

watch([tabs, activeTabId], () => {
  try {
    localStorage.setItem(
      "gitswamp-tabs",
      JSON.stringify({ tabs: tabs.value, activeTabId: activeTabId.value })
    );
  } catch {}
}, { deep: true });

// Collapse side panel and close diff when switching tabs
watch(activeTabId, () => {
  detailsPanelCollapsed.value = true;
  showDiffViewer.value = false;
});

watch(recentRepos, () => {
  try {
    localStorage.setItem("gitswamp-recent", JSON.stringify(recentRepos.value));
  } catch {}
}, { deep: true });

watch(
  () => [
    git.repoInfo.value?.path || "",
    (git.repoInfo.value?.remotes || [])
      .map((remote) => `${remote.name}:${remote.provider}:${remote.url}`)
      .join("|"),
    git.providerTokens.value.github || git.githubToken.value || "",
    git.providerTokens.value.gitlab || "",
    git.providerTokens.value["gitlab-self"] || "",
  ],
  () => {
    scheduleOpenPullRequestRefresh();
  },
  { immediate: true },
);

watch(() => git.selectedCommit.value, (commit) => {
  if (commit) {
    viewingWorkingChanges.value = false;
    git.getCommitFiles(commit.sha);
  }
});

const selectedIssue = computed(() =>
  githubIssues.value.find((item) => item.number === selectedIssueNumber.value) || null,
);

const selectedPullRequest = computed(() =>
  githubPullRequests.value.find((item) => item.number === selectedPullRequestNumber.value) || null,
);

const activeTab = computed(() => tabs.value.find((t) => t.id === activeTabId.value));
const isLanding = computed(() => !activeTab.value?.repo);

function selectTab(id: string) {
  activeTabId.value = id;
  const tab = tabs.value.find((t) => t.id === id);
  if (tab?.path) {
    git.openRepository(tab.path);
  }
}

function closeTab(id: string) {
  const idx = tabs.value.findIndex((t) => t.id === id);
  if (idx < 0 || tabs.value.length <= 1) return;
  tabs.value.splice(idx, 1);
  if (activeTabId.value === id) {
    activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)].id;
    const active = tabs.value.find((t) => t.id === activeTabId.value);
    if (active?.path) git.openRepository(active.path);
  }
}

function newTab() {
  const id = "tab-" + Date.now();
  tabs.value.push({ id, repo: null, label: "Start", path: "" });
  activeTabId.value = id;
}

async function openRepo(path: string) {
  try {
    await git.openRepository(path);
    if (git.repoInfo.value) {
      const repo = git.repoInfo.value;
      const tab = tabs.value.find((t) => t.id === activeTabId.value);
      if (tab) {
        tab.repo = repo;
        tab.label = repo.name;
        tab.path = repo.path;
      }
      addToRecent(repo);
      if (git.stagedFiles.value.length > 0 || git.unstagedFiles.value.length > 0) {
        viewingWorkingChanges.value = true;
        git.selectedCommit.value = null;
      } else {
        viewingWorkingChanges.value = false;
        git.selectedCommit.value = null;
      }
    }
  } catch (e) {
    console.error("Failed to open repository:", e);
    toast.error(`Failed to open repository: ${String(e)}`);
  }
}

async function browseAndOpen() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "Open Repository",
    });
    if (selected) {
      await openRepo(selected as string);
    }
  } catch {}
}

async function handleClone(url: string, path: string, shallow: boolean, done?: (ok: boolean, error?: string) => void) {
  try {
    toast.info("Cloning repository...");
    const clonedPath = await git.cloneRepo(url, path, shallow);
    if (!clonedPath) {
      done?.(false, git.error.value || "Clone failed.");
      toast.error("Clone failed: " + (git.error.value || "Unknown error"));
      return;
    }
    showCloneDialog.value = false;
    await openRepo(clonedPath);
    done?.(true);
    toast.success("Repository cloned successfully");
  } catch (e) {
    done?.(false, String(e));
    toast.error("Clone failed: " + String(e));
  }
}

async function handleInit(path: string, branchName: string) {
  const ok = await git.initRepo(path, branchName);
  if (ok) {
    showInitDialog.value = false;
    await openRepo(path);
    toast.success("Repository initialized successfully");
  } else {
    toast.error(`Failed to initialize repository: ${git.error.value || "Unknown error"}`);
  }
}

async function handlePush() {
  // First, check if origin exists
  const hasOrigin = await git.checkOriginExists();
  if (hasOrigin) {
    // Origin exists, push normally
    activeRemoteAction.value = "push";
    try {
      await git.push();
      maybeShowAuthDialogFromGitError();
    } finally {
      activeRemoteAction.value = null;
    }
  } else {
    // No origin, show dialog to select platform
    const repoName = git.repoInfo.value?.name || "repository";
    multiPlatformPushRepoName.value = repoName;
    showMultiPlatformPushDialog.value = true;
  }
}

function handleMultiPlatformPush(platform: string) {
  // For GitHub/GitLab and self-hosted, ask for username and domain first
  if (platform === 'github' || platform === 'gitlab' || platform === 'github-enterprise' || platform === 'gitlab-self-hosted' || platform === 'gitlab-self') {
    pushPlatform.value = platform;
    pushUsername.value = "";
    pushDomain.value = "";
    showPushUsernameDialog.value = true;
  } else {
    // For other platforms, push directly
    performPush(platform, "");
  }
}

async function performPush(platform: string, username: string) {
  let repoName: string;
  const needsDomain = platform === "gitlab-self-hosted" || platform === "gitlab-self" || platform === "github-enterprise";
  
  if (username) {
    if (needsDomain) {
      const normalizedDomain = normalizeHostInput(pushDomain.value);
      if (!normalizedDomain) {
        toast.error("Domain is required for this platform");
        return;
      }

      // For self-hosted instances: username/repo@domain.com
      repoName = `${username}/${git.repoInfo.value?.name || multiPlatformPushRepoName.value}@${normalizedDomain}`;
    } else {
      // For standard platforms: username/repo
      repoName = `${username}/${git.repoInfo.value?.name || multiPlatformPushRepoName.value}`;
    }
  } else {
    repoName = git.repoInfo.value?.name || multiPlatformPushRepoName.value;
  }
  
  await git.pushToMultiplePlatforms(platform, repoName);
  maybeShowAuthDialogFromGitError();
  if (!git.error.value) {
    showMultiPlatformPushDialog.value = false;
    showPushUsernameDialog.value = false;
  }
}

async function saveAuthToken() {
  if (!authTokenInput.value.trim()) return;
  try {
    authSubmitting.value = true;
    const token = authTokenInput.value.trim();

    if (authProvider.value === "github") {
      await git.saveProviderToken("github", token);
    } else if (authProvider.value === "gitlab") {
      await git.saveProviderToken("gitlab", token);
    } else {
        const domainRaw = authDomainInput.value;
        const domainClean = normalizeHostInput(domainRaw);
        if (!domainClean) {
          toast.error("Domain is required for self-hosted GitLab");
          return;
        }
        authDomainInput.value = domainClean;
        await git.saveProviderToken("gitlab-self", `${domainClean}|${token}`);
    }

    await syncAuthStateAfterChange();

    showAuthRequiredDialog.value = false;
    toast.success("Authentication token saved and loaded.");
  } catch (e) {
    toast.error("Failed to save token: " + String(e));
  } finally {
    authSubmitting.value = false;
  }
}

async function generateAndPushGitlabKey() {
  if (authProvider.value !== "gitlab-self") return;
  if (!authTokenInput.value.trim() || !authDomainInput.value.trim() || !authEmailInput.value.trim()) {
    toast.error("Domain, token and email are required to generate and push SSH key");
    return;
  }
  try {
    authSubmitting.value = true;
    const keyName = authKeyNameInput.value.trim() || "gitswamp";
    const generated = await invoke<[string, string]>("generate_ssh_key", {
      email: authEmailInput.value.trim(),
      keyName,
    });
    const publicKey = generated[1];
    const domainClean = normalizeHostInput(authDomainInput.value);
    if (!domainClean) {
      toast.error("Domain is required for self-hosted GitLab");
      return;
    }

    authDomainInput.value = domainClean;
    await invoke("add_gitlab_ssh_key", {
      domain: domainClean,
      token: authTokenInput.value.trim(),
      title: `gitswamp-${Date.now()}`,
      key: publicKey,
    });

    await git.saveProviderToken("gitlab-self", `${domainClean}|${authTokenInput.value.trim()}`);
    await syncAuthStateAfterChange();
    showAuthRequiredDialog.value = false;
    toast.success("SSH key generated and pushed to self-hosted GitLab");
  } catch (e) {
    toast.error("Failed to generate/push SSH key: " + String(e));
  } finally {
    authSubmitting.value = false;
  }
}

function addToRecent(repo: RepoInfo) {
  const existing = recentRepos.value.findIndex((r) => r.path === repo.path);
  if (existing >= 0) recentRepos.value.splice(existing, 1);
  recentRepos.value.unshift({
    name: repo.name,
    path: repo.path,
    branch: repo.current_branch,
  });
  if (recentRepos.value.length > 20) recentRepos.value.length = 20;
}

function removeRecent(path: string) {
  recentRepos.value = recentRepos.value.filter((r) => r.path !== path);
}

function clearRecent() {
  recentRepos.value = [];
}

function onSelectCommit(commit: CommitInfo | null) {
  viewingWorkingChanges.value = false;
  viewingStash.value = false;
  git.selectedCommit.value = commit;
  git.clearStashSelection();
  detailsPanelCollapsed.value = false;
}

function onSelectWorkingChanges() {
  selectWorkingChangesState();
}

function onSelectConflicts() {
  selectWorkingChangesState();
}

function selectWorkingChangesState() {
  viewingWorkingChanges.value = true;
  viewingStash.value = false;
  git.selectedCommit.value = null;
  git.clearStashSelection();
  detailsPanelCollapsed.value = false;
}

function onSelectStash(stash: StashInfo) {
  viewingWorkingChanges.value = false;
  viewingStash.value = true;
  git.selectedCommit.value = null;
  git.selectStash(stash);
  detailsPanelCollapsed.value = false;
}

function handleRequestMerge(payload: { source: string; sourceRemote: boolean; target: string }) {
  if (!payload.source || !payload.target || payload.source === payload.target) return;
  toast.action(
    "warning",
    `Merge ${payload.sourceRemote ? "origin/" + payload.source : payload.source} into ${payload.target}?`,
    [
      {
        label: "Merge",
        style: "primary",
        onClick: () => git.mergeBranchIntoCurrent(payload.source, payload.sourceRemote, payload.target),
      },
      {
        label: "Cancel",
        style: "neutral",
        onClick: () => {},
      },
    ],
    18000
  );
}

async function handleCheckoutRemoteBranch(name: string) {
  const localBranch = git.localBranches.value.find(b => b.name === name);
  if (localBranch) {
    await git.resetBranchToRemote(name);
  } else {
    try {
      await git.createBranch(name, "origin/" + name);
      await git.checkoutBranch(name);
    } catch {
      await git.checkoutBranch(name);
    }
  }
  if (!git.error.value) {
    toast.success(`Remote branch "${name}" checked out`);
  }
}

function handleCreateBranch() {
  showBranchDialog.value = true;
}

function normalizeGhostNameFragment(value: string): string {
  let out = "";
  let previousDash = false;

  for (const ch of value.toLowerCase()) {
    const allowed = (ch >= "a" && ch <= "z") || (ch >= "0" && ch <= "9") || ch === "_" || ch === "-";
    if (!allowed || ch === "-") {
      if (previousDash || out.length === 0) {
        continue;
      }
      out += "-";
      previousDash = true;
      continue;
    }

    out += ch;
    previousDash = false;
  }

  if (out.endsWith("-")) {
    out = out.slice(0, -1);
  }

  return out;
}

function suggestedMaterializedBranchName(): string {
  const source = git.currentBranch.value || "feature";
  const withoutOrigin = source.toLowerCase().startsWith("origin/")
    ? source.slice(7)
    : source;
  const flattened = withoutOrigin.split("/").filter(Boolean).join("-");
  const normalized = normalizeGhostNameFragment(flattened);
  return normalized ? `${normalized}-materialized` : "materialized-ghost";
}

function handleOpenGhostMaterializeDialog() {
  ghostMaterializeName.value = suggestedMaterializedBranchName();
  showGhostMaterializeDialog.value = true;
}

async function handleStartGhostBranch() {
  await git.startGhostBranch();
}

async function handleSubmitGhostMaterialize(name: string) {
  await git.materializeGhostBranch(name);
  showGhostMaterializeDialog.value = false;
}

function handleDiscardGhostBranch() {
  toast.action(
    "warning",
    "Discard active Ghost Branch experiment?",
    [
      {
        label: "Discard",
        style: "danger",
        onClick: () => git.discardGhostBranch(),
      },
      {
        label: "Cancel",
        style: "neutral",
        onClick: () => {},
      },
    ],
    18000,
  );
}

function handleTimeMachineBlame(sha: string) {
  timeMachineFocusSha.value = sha;
  setHistoryViewMode("time-machine");
}

function submitCreateBranch(name: string) {
  submitCreateBranchFromDialog(name);
}

function handleStash() {
  showStashDialog.value = true;
}

function submitStash() {
  git.stashPush(stashMessage.value || undefined);
  showStashDialog.value = false;
  stashMessage.value = "";
}

function handleEditCommitMessage(sha: string) {
  const commit = git.displayedCommits.value.find(c => c.sha === sha);
  editMessageSha.value = sha;
  editMessageText.value = commit ? commit.message : "";
  showEditMessageDialog.value = true;
}

async function submitEditMessage() {
  if (editMessageText.value.trim() && editMessageSha.value) {
    await git.editCommitMessage(editMessageSha.value, editMessageText.value.trim());
  }
  showEditMessageDialog.value = false;
  editMessageSha.value = "";
  editMessageText.value = "";
}

function handleRenameBranch(name: string) {
  renameBranchOld.value = name;
  renameBranchNew.value = name;
  showRenameDialog.value = true;
}

async function submitRenameBranch() {
  if (renameBranchNew.value.trim() && renameBranchOld.value) {
    await git.renameBranch(renameBranchOld.value, renameBranchNew.value.trim());
  }
  showRenameDialog.value = false;
  renameBranchOld.value = "";
  renameBranchNew.value = "";
}

async function handleDeleteBranchAndRemote(name: string) {
  await git.deleteBranch(name);
  await git.deleteRemoteBranch(name);
}

function handleCreateAnnotatedTag(sha: string) {
  annotatedTagSha.value = sha;
  annotatedTagName.value = "";
  annotatedTagMessage.value = "";
  showAnnotatedTagDialog.value = true;
}

async function submitAnnotatedTag() {
  if (annotatedTagName.value.trim() && annotatedTagSha.value && annotatedTagMessage.value.trim()) {
    await git.createAnnotatedTag(annotatedTagName.value.trim(), annotatedTagSha.value, annotatedTagMessage.value.trim());
  }
  showAnnotatedTagDialog.value = false;
  annotatedTagName.value = "";
  annotatedTagSha.value = "";
  annotatedTagMessage.value = "";
}

const branchAtSha = ref("");
const tagAtSha = ref("");
const showTagDialog = ref(false);
const tagName = ref("");

const isAnyDialogOpen = computed(() =>
  showCloneDialog.value ||
  showInitDialog.value ||
  showBranchDialog.value ||
  showStashDialog.value ||
  showSettings.value ||
  showEditMessageDialog.value ||
  showRenameDialog.value ||
  showAnnotatedTagDialog.value ||
  showTagDialog.value ||
  showMultiPlatformPushDialog.value ||
  showPushUsernameDialog.value ||
  showAuthRequiredDialog.value ||
  showGhostMaterializeDialog.value,
);

watch(isAnyDialogOpen, (opened) => {
  if (opened) {
    detailsPanelCollapsed.value = true;
  }
});

watch(terminalAllowAll, (value) => {
  localStorage.setItem("gitswamp-terminal-allow-all", String(value));
});

function handleCreateBranchAtCommit(sha: string) {
  branchAtSha.value = sha;
  showBranchDialog.value = true;
}

function submitCreateBranchFromDialog(name: string) {
  if (branchAtSha.value) {
    git.createBranch(name, branchAtSha.value);
    branchAtSha.value = "";
  } else {
    git.createBranch(name);
  }
  showBranchDialog.value = false;
  newBranchName.value = "";
}

function handleCreateTagAtCommit(sha: string) {
  tagAtSha.value = sha;
  tagName.value = "";
  showTagDialog.value = true;
}

function submitCreateTag() {
  if (tagName.value.trim() && tagAtSha.value) {
    git.createTagAt(tagName.value.trim(), tagAtSha.value);
  }
  showTagDialog.value = false;
  tagName.value = "";
  tagAtSha.value = "";
}

const openReposList = computed(() =>
  tabs.value
    .filter((t) => t.repo)
    .map((t) => ({
      name: t.repo!.name,
      path: t.repo!.path,
      branch: t.repo!.current_branch,
    }))
);
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-[var(--background)] overflow-hidden">
    <TitleBar />
    <RepositoryTabs
      :tabs="tabs"
      :active-tab-id="activeTabId"
      @select-tab="selectTab"
      @close-tab="closeTab"
      @new-tab="newTab"
      @open-repository="browseAndOpen"
      @toggle-terminal="toggleTerminalPanel"
      @set-history-view="setHistoryViewMode($event)"
      @open-settings="showSettings = true"
      @refresh-repository="refreshCurrentRepo()"
      @open-in-vs-code="openRepoInVsCode()"
      @open-in-explorer="openRepoInExplorer()"
      @create-gist="createGistFromRepo()"
    />

    <template v-if="isLanding">
      <LandingPage
        :open-repos="openReposList"
        :recent-repos="recentRepos"
        @open="openRepo"
        @browse="browseAndOpen"
        @clone="showCloneDialog = true"
        @init="showInitDialog = true"
        @settings="showSettings = true"
        @remove-recent="removeRecent"
        @clear-recent="clearRecent"
      />
    </template>

    <template v-else-if="git.repoInfo.value">
      <AppHeader
        :loading="git.loading.value"
        :active-action="activeRemoteAction"
        :ghost-active="git.ghostBranchState.value.active"
        :origin-conflict-risk="originConflictRisk"
        @pull="handlePull"
        @push="handlePush"
        @fetch="handleFetch"
        @branch="handleCreateBranch"
        @ghost-branch="handleStartGhostBranch"
        @materialize-ghost-branch="handleOpenGhostMaterializeDialog"
        @discard-ghost-branch="handleDiscardGhostBranch"
        @stash="handleStash"
        @terminal="toggleTerminalPanel"
        @settings="showSettings = true"
      />
      <RepositoryWorkspace
        :git="git"
        :show-terminal="showTerminal"
        :terminal-allow-all="terminalAllowAll"
        :open-pull-request-branches="openPullRequestBranches"
        :issues="githubIssues"
        :pull-requests="githubPullRequests"
        :selected-issue="selectedIssue"
        :selected-pull-request="selectedPullRequest"
        :remote-insights-mode="remoteInsightsMode"
        :show-diff-viewer="showDiffViewer"
        :diff-file-path="diffFilePath"
        :diff-commit-sha="diffCommitSha"
        :diff-staged="diffStaged"
        :conflict-resolver-path="conflictResolverPath"
        :details-panel-collapsed="detailsPanelCollapsed"
        :history-view-mode="historyViewMode"
        :time-machine-focus-sha="timeMachineFocusSha"
        :viewing-working-changes="viewingWorkingChanges"
        :viewing-stash="viewingStash"
        @set-history-view="setHistoryViewMode($event)"
        @update:show-terminal="showTerminal = $event"
        @update:terminal-allow-all="terminalAllowAll = $event"
        @update:details-panel-collapsed="detailsPanelCollapsed = $event"
        @close-diff-viewer="closeDiffViewer"
        @open-diff-viewer="openDiffViewer($event.path, $event.sha, $event.staged)"
        @open-conflict-resolver="openConflictResolver($event)"
        @close-conflict-resolver="closeConflictResolver"
        @conflict-resolved="onConflictResolved"
        @select-commit="onSelectCommit($event)"
        @select-working-changes="onSelectWorkingChanges"
        @select-conflicts="onSelectConflicts"
        @select-stash="onSelectStash($event)"
        @select-issue="openIssueDetails($event)"
        @select-pull-request="openPullRequestDetails($event)"
        @open-create-issue="openCreateIssuePanel"
        @open-create-pull-request="openCreatePullRequestPanel"
        @create-issue="createRemoteIssue($event)"
        @create-pull-request="createRemotePullRequest($event)"
        @request-merge="handleRequestMerge($event)"
        @checkout-remote-branch="handleCheckoutRemoteBranch($event)"
        @pull="handlePull"
        @push="handlePush"
        @create-branch-at="handleCreateBranchAtCommit($event)"
        @create-tag-at="handleCreateTagAtCommit($event)"
        @create-annotated-tag-at="handleCreateAnnotatedTag($event)"
        @edit-commit-message="handleEditCommitMessage($event)"
        @rename-branch="handleRenameBranch($event)"
        @delete-branch-and-remote="handleDeleteBranchAndRemote($event)"
        @time-machine-blame="handleTimeMachineBlame($event)"
        @create-gist="createGistFromRepo()"
      />
    </template>

    <GhostBranchDialog
      :visible="showGhostMaterializeDialog"
      :loading="git.loading.value"
      :suggested-name="ghostMaterializeName"
      @close="showGhostMaterializeDialog = false"
      @submit="handleSubmitGhostMaterialize"
    />

    <RepositoryActionDialogs
      :show-branch-dialog="showBranchDialog"
      :new-branch-name="newBranchName"
      :show-stash-dialog="showStashDialog"
      :stash-message="stashMessage"
      :show-tag-dialog="showTagDialog"
      :tag-name="tagName"
      :show-annotated-tag-dialog="showAnnotatedTagDialog"
      :annotated-tag-name="annotatedTagName"
      :annotated-tag-message="annotatedTagMessage"
      :show-edit-message-dialog="showEditMessageDialog"
      :edit-message-text="editMessageText"
      :show-rename-dialog="showRenameDialog"
      :rename-branch-old="renameBranchOld"
      :rename-branch-new="renameBranchNew"
      @update:new-branch-name="newBranchName = $event"
      @update:stash-message="stashMessage = $event"
      @update:tag-name="tagName = $event"
      @update:annotated-tag-name="annotatedTagName = $event"
      @update:annotated-tag-message="annotatedTagMessage = $event"
      @update:edit-message-text="editMessageText = $event"
      @update:rename-branch-new="renameBranchNew = $event"
      @close:branch="showBranchDialog = false"
      @submit:branch="submitCreateBranch(newBranchName)"
      @close:stash="showStashDialog = false"
      @submit:stash="submitStash()"
      @close:tag="showTagDialog = false"
      @submit:tag="submitCreateTag()"
      @close:annotated-tag="showAnnotatedTagDialog = false"
      @submit:annotated-tag="submitAnnotatedTag()"
      @close:edit-message="showEditMessageDialog = false"
      @submit:edit-message="submitEditMessage()"
      @close:rename="showRenameDialog = false"
      @submit:rename="submitRenameBranch()"
    />

    <CloneDialog
      :visible="showCloneDialog"
      :token="git.githubToken.value"
      :provider-tokens="git.providerTokens.value"
      @close="showCloneDialog = false"
      @clone="handleClone"
      @save-provider-token="handleSaveProviderToken"
    />
    <InitDialog
      :visible="showInitDialog"
      :provider-tokens="git.providerTokens.value"
      @close="showInitDialog = false"
      @init="handleInit"
      @save-provider-token="handleSaveProviderToken"
    />
    <MultiPlatformPushDialog
      :visible="showMultiPlatformPushDialog"
      :repo-name="multiPlatformPushRepoName"
      :available-platforms="git.providerTokens.value"
      :pushing="git.loading.value"
      @close="showMultiPlatformPushDialog = false"
      @pushTo="handleMultiPlatformPush"
    />
    <SettingsDialog
      v-if="showSettings"
      :token="git.githubToken.value"
      :git-path="git.gitPath.value"
      @save="handleSaveGithubTokenFromSettings"
      @delete="handleDeleteProviderToken('github')"
      @close="showSettings = false"
    />
    <RepositoryAuthDialogs
      :show-push-username-dialog="showPushUsernameDialog"
      :push-platform="pushPlatform"
      :push-username="pushUsername"
      :push-domain="pushDomain"
      :show-auth-required-dialog="showAuthRequiredDialog"
      :auth-provider="authProvider"
      :auth-domain-input="authDomainInput"
      :auth-token-input="authTokenInput"
      :auth-email-input="authEmailInput"
      :auth-key-name-input="authKeyNameInput"
      :auth-submitting="authSubmitting"
      @update:show-push-username-dialog="showPushUsernameDialog = $event"
      @update:push-username="pushUsername = $event"
      @update:push-domain="pushDomain = $event"
      @push="performPush(pushPlatform, pushUsername)"
      @update:show-auth-required-dialog="showAuthRequiredDialog = $event"
      @update:auth-provider="authProvider = $event"
      @update:auth-domain-input="authDomainInput = $event"
      @update:auth-token-input="authTokenInput = $event"
      @update:auth-email-input="authEmailInput = $event"
      @update:auth-key-name-input="authKeyNameInput = $event"
      @save-auth-token="saveAuthToken()"
      @generate-and-push-gitlab-key="generateAndPushGitlabKey()"
    />

    <ToastContainer />
  </div>
</template>

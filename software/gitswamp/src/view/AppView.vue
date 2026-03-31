<script setup lang="ts">
import TitleBar from "@/view/shell/TitleBar.vue";
import AppHeader from "@/view/shell/AppHeader.vue";
import RepositoryTabs from "@/view/repository/RepositoryTabs.vue";
import RepositoryWorkspace from "@/view/repository/RepositoryWorkspace.vue";
import ConflictResolver from "@/shared/ui/ConflictResolver.vue";
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
import type { RepoInfo, CommitInfo, StashInfo, RemoteInfo } from "@/types";

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

type HistoryViewMode = "graph" | "productivity" | "time-machine" | "conflict-heatmap";

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

const showConflictResolver = ref(false);
const conflictResolverPath = ref("");

const showMultiPlatformPushDialog = ref(false);
const multiPlatformPushRepoName = ref("");
const showPushUsernameDialog = ref(false);
const pushPlatform = ref("");
const pushUsername = ref("");
const pushDomain = ref("");
const detailsPanelCollapsed = ref(true);
const openPullRequestBranches = ref<string[]>([]);
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
    const hasMarkers = await invoke<boolean>("has_conflict_markers", {
      path: git.repoPath.value,
      filePath: filePath,
    });

    if (hasMarkers) {
      // File has markers - open the large window with line-by-line resolver
      conflictResolverPath.value = filePath;
      showConflictResolver.value = true;
    } else {
      // File has no markers - show toast with 4 options
      const actions = [
        {
          label: 'Keep Modified',
          style: 'primary' as const,
          onClick: async () => {
            await resolveConflict(filePath, 'keep-modified');
          }
        },
        {
          label: 'Keep Base',
          style: 'neutral' as const,
          onClick: async () => {
            await resolveConflict(filePath, 'keep-base');
          }
        },
        {
          label: 'Delete File',
          style: 'danger' as const,
          onClick: async () => {
            await resolveConflict(filePath, 'delete');
          }
        },
        {
          label: 'Cancel',
          style: 'neutral' as const,
          onClick: async () => {
            // Just close the toast, do nothing
          }
        }
      ];

      toast.action(
        'warning',
        `Resolve conflict: ${filePath}`,
        actions,
        30000 // 30 seconds for user to decide
      );
    }
  } catch (e) {
    toast.error(`Error opening conflict resolver: ${String(e)}`);
  }
}

function closeConflictResolver() {
  showConflictResolver.value = false;
}

function onConflictResolved() {
  showConflictResolver.value = false;
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

function detectAuthProviderFromOrigin(): "github" | "gitlab" | "gitlab-self" {
  const origin = git.repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url?.toLowerCase() || "";
  if (origin.includes("gitlab.com")) return "gitlab";
  if (origin.includes("gitlab.")) return "gitlab-self";
  return "github";
}

function parseAuthDomainFromOrigin(): string {
  const origin = git.repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url || "";
  const parsed = parseRemoteHostAndPath(origin);
  if (parsed && parsed.host) return parsed.host;

  const noProto = origin.replace(/^https?:\/\//i, "");
  const noCreds = noProto.includes("@") ? noProto.split("@")[1] : noProto;
  const firstSegment = noCreds.split("/")[0] || "";
  // Handle scp-like URLs (git@host:owner/repo) or host:port by stripping trailing ":..."
  const hostOnly = firstSegment.split(":")[0];
  return hostOnly;
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

function parseRemoteHostAndPath(remoteUrl: string): { host: string; path: string } | null {
  const value = remoteUrl.trim();
  if (!value) return null;

  const scpLikeMatch = value.match(/^[^@]+@([^:]+):(.+)$/);
  if (scpLikeMatch) {
    return {
      host: scpLikeMatch[1].toLowerCase(),
      path: stripGitSuffix(scpLikeMatch[2].replace(/^\/+/, "")),
    };
  }

  try {
    const parsed = new URL(value);
    return {
      host: parsed.hostname.toLowerCase(),
      path: stripGitSuffix(parsed.pathname.replace(/^\/+/, "")),
    };
  } catch {
    return null;
  }
}

function parseGithubRemote(remoteUrl: string): { host: string; owner: string; repo: string } | null {
  const parsed = parseRemoteHostAndPath(remoteUrl);
  if (!parsed) return null;

  const segments = parsed.path.split("/").filter(Boolean);
  if (segments.length < 2) return null;

  return {
    host: parsed.host,
    owner: segments[segments.length - 2],
    repo: segments[segments.length - 1],
  };
}

function parseGitlabRemote(remoteUrl: string): { host: string; projectPath: string } | null {
  const parsed = parseRemoteHostAndPath(remoteUrl);
  if (!parsed) return null;

  const segments = parsed.path.split("/").filter(Boolean);
  if (segments.length < 2) return null;

  return {
    host: parsed.host,
    projectPath: segments.join("/"),
  };
}

function getPrimaryRemote(remotes: RemoteInfo[]): RemoteInfo | null {
  return remotes.find((remote) => remote.name === "origin") || remotes[0] || null;
}

function getGitlabTokenForHost(host: string): string | null {
  if (host === "gitlab.com") {
    return git.providerTokens.value.gitlab || null;
  }

  const selfHosted = git.providerTokens.value["gitlab-self"];
  if (selfHosted) {
    const separatorIndex = selfHosted.indexOf("|");
    if (separatorIndex > 0) {
      const domain = selfHosted.slice(0, separatorIndex).toLowerCase();
      const token = selfHosted.slice(separatorIndex + 1);
      if (domain === host && token) {
        return token;
      }
    } else {
      return selfHosted;
    }
  }

  return git.providerTokens.value.gitlab || null;
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

async function fetchOpenGithubPullRequestBranches(remoteUrl: string): Promise<string[]> {
  const parsed = parseGithubRemote(remoteUrl);
  if (!parsed) return [];

  const apiBase = parsed.host === "github.com"
    ? "https://api.github.com"
    : `https://${parsed.host}/api/v3`;

  const token = git.providerTokens.value.github || git.githubToken.value || "";
  const headers: Record<string, string> = {
    Accept: "application/vnd.github+json",
  };

  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  const payload = await fetchJsonWithTimeout(
    `${apiBase}/repos/${encodeURIComponent(parsed.owner)}/${encodeURIComponent(parsed.repo)}/pulls?state=open&per_page=100`,
    headers,
  );

  if (!Array.isArray(payload)) return [];
  return payload
    .map((item) => {
      if (!item || typeof item !== "object") return "";
      const candidate = (item as { head?: { ref?: unknown } }).head?.ref;
      return typeof candidate === "string" ? candidate : "";
    })
    .filter((name) => name.length > 0);
}

async function fetchOpenGitlabMergeRequestBranches(remoteUrl: string): Promise<string[]> {
  const parsed = parseGitlabRemote(remoteUrl);
  if (!parsed) return [];

  const token = getGitlabTokenForHost(parsed.host) || "";
  const headers: Record<string, string> = {
    Accept: "application/json",
  };

  if (token) {
    headers["PRIVATE-TOKEN"] = token;
  }

  const payload = await fetchJsonWithTimeout(
    `https://${parsed.host}/api/v4/projects/${encodeURIComponent(parsed.projectPath)}/merge_requests?state=opened&per_page=100`,
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

async function refreshOpenPullRequestBranches() {
  const remotes = git.repoInfo.value?.remotes || [];
  const primaryRemote = getPrimaryRemote(remotes);

  if (!primaryRemote || (primaryRemote.provider !== "github" && primaryRemote.provider !== "gitlab")) {
    openPullRequestBranches.value = [];
    return;
  }

  const sequence = ++pullRequestFetchSequence;

  try {
    const branches = primaryRemote.provider === "github"
      ? await fetchOpenGithubPullRequestBranches(primaryRemote.url)
      : await fetchOpenGitlabMergeRequestBranches(primaryRemote.url);

    if (sequence !== pullRequestFetchSequence) return;
    openPullRequestBranches.value = uniqueNormalizedBranches(branches);
  } catch {
    if (sequence !== pullRequestFetchSequence) return;
    openPullRequestBranches.value = [];
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
  if (platform === 'github' || platform === 'gitlab' || platform === 'github-enterprise' || platform === 'gitlab-self-hosted') {
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
  
  if (username) {
    if (pushDomain.value) {
      // For self-hosted instances: username/repo@domain.com
      repoName = `${username}/${git.repoInfo.value?.name || multiPlatformPushRepoName.value}@${pushDomain.value}`;
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
      await git.saveToken(token);
    } else if (authProvider.value === "gitlab") {
      await git.saveProviderToken("gitlab", token);
    } else {
        const domainRaw = authDomainInput.value.trim();
        if (!domainRaw) {
          toast.error("Domain is required for self-hosted GitLab");
          return;
        }
        const domainClean = domainRaw.replace(/^https?:\/\//i, "").replace(/\/+$/i, "");
        await git.saveProviderToken("gitlab-self", `${domainClean}|${token}`);
    }

    showAuthRequiredDialog.value = false;
    toast.success("Authentication token saved. Retry the remote action.");
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
    const domainRaw = authDomainInput.value.trim();
    const domainClean = domainRaw.replace(/^https?:\/\//i, "").replace(/\/+$/i, "");
    await invoke("add_gitlab_ssh_key", {
      domain: domainClean,
      token: authTokenInput.value.trim(),
      title: `gitswamp-${Date.now()}`,
      key: publicKey,
    });

    await git.saveProviderToken("gitlab-self", `${authDomainInput.value.trim()}|${authTokenInput.value.trim()}`);
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
  showConflictResolver.value ||
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
        :show-diff-viewer="showDiffViewer"
        :diff-file-path="diffFilePath"
        :diff-commit-sha="diffCommitSha"
        :diff-staged="diffStaged"
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
        @select-commit="onSelectCommit($event)"
        @select-working-changes="onSelectWorkingChanges"
        @select-conflicts="onSelectConflicts"
        @select-stash="onSelectStash($event)"
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
      @save-provider-token="(provider: string, token: string) => git.saveProviderToken(provider, token)"
    />
    <InitDialog
      :visible="showInitDialog"
      :provider-tokens="git.providerTokens.value"
      @close="showInitDialog = false"
      @init="handleInit"
      @save-provider-token="(provider: string, token: string) => git.saveProviderToken(provider, token)"
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
      @save="git.saveProviderToken('github', $event); showSettings = false"
      @delete="git.deleteProviderToken('github')"
      @close="showSettings = false"
    />
    <ConflictResolver
      v-if="showConflictResolver && git.repoPath.value"
      :repo-path="git.repoPath.value"
      :file-path="conflictResolverPath"
      @close="closeConflictResolver"
      @resolved="onConflictResolved"
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

<script setup lang="ts">
import TitleBar from "@/view/shell/TitleBar.vue";
import AppHeader from "@/view/shell/AppHeader.vue";
import CommandPalette from "@/view/shell/CommandPalette.vue";
import RepositoryTabs from "@/view/repository/RepositoryTabs.vue";
import RepositoryWorkspace from "@/view/repository/RepositoryWorkspace.vue";
import LandingPage from "@/view/repository/LandingPage.vue";
import RepositoryActionDialogs from "@/view/repository/RepositoryActionDialogs.vue";
import RepositoryAuthDialogs from "@/view/repository/RepositoryAuthDialogs.vue";
import CloneDialog from "@/view/repository/CloneDialog.vue";
import InitDialog from "@/view/repository/InitDialog.vue";
import GhostBranchDialog from "@/view/repository/GhostBranchDialog.vue";
import MultiPlatformPushDialog from "@/view/repository/MultiPlatformPushDialog.vue";
import OptionsDialog from "@/view/shell/OptionsDialog.vue";
import LogsPanel from "@/view/shell/LogsPanel.vue";
import ToastContainer from "@/shared/ui/ToastContainer.vue";
import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";
import { shouldRestoreSession } from "@/app/preferences/sessionPreferences";
import { useAppAppearance } from "@/app/preferences/useAppAppearance";
import { useGit } from "@/domain/git/UseGit";
import { useToast } from "@/shared/notifications/useToast";
import { useUndoableDestructiveAction } from "@/shared/notifications/useUndoableDestructiveAction";
import { clearDiffViewerCaches } from "@/shared/config/diffViewCache";
import { handleRepositoryTabShortcut } from "@/features/repository/tabs/repositoryTabShortcuts";
import { useRepositoryTabs } from "@/features/repository/tabs/useRepositoryTabs";
import { useRecentRepositories } from "@/features/repository/recent/useRecentRepositories";
import { useAppLogs } from "@/features/shell/useAppLogs";
import { isEditableTarget } from "@/shared/dom/keyboardTargets";
import {
  SMART_GITIGNORE_WIZARD_EVENT,
  getStoredSmartGitignoreWizardEnabled,
} from "@/shared/config/gitignoreWizardPreferences";
import {
  BUG_AUTOPSY_EVENT,
  getStoredBugAutopsyEnabled,
} from "@/shared/config/bugAutopsyPreferences";
import {
  IDENTITY_GUARD_EVENT,
  getStoredIdentityGuardEnabled,
} from "@/shared/config/identityGuardPreferences";
import {
  BACKGROUND_MAINTENANCE_EVENT,
  getStoredBackgroundMaintenanceSettings,
  hasBackgroundMaintenanceEnabled,
  updateBackgroundMaintenanceSettings,
  type BackgroundMaintenanceSettings,
} from "@/shared/config/backgroundMaintenancePreferences";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  RELEASE_NOTES_LOG_FORMAT,
  buildReleaseNotesMarkdown,
  defaultReleaseNotesFileName,
  parseReleaseNotesLog,
} from "@/features/release-notes/releaseNotes";
import {
  buildGitRpgProfile,
  gitRpgProfileDetailLines,
  summarizeCommitFilesForRpg,
  type GitRpgCommitStats,
  type GitRpgProfile,
} from "@/features/repository/rpg/gitRpgProfiler";
import type { IdentityGuardMismatch } from "@/features/repository/identity/identityGuard";
import type { PickaxeCommitHit } from "@/features/repository/pickaxe/pickaxeSearch";

import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, watch } from "vue";
import type {
  BranchInfo,
  CommitFileInfo,
  CommitInfo,
  StashInfo,
  RemoteInfo,
  IssueInfo,
  PullRequestInfo,
  GistInfo,
  LostCommitInfo,
  RemoteReferenceInfo,
  RemoteLabelInfo,
  RemoteMilestoneInfo,
  RemoteUserInfo,
  RemoteIssueCreatePayload,
  RemotePullRequestCreatePayload,
} from "@/types";

const git = useGit();
const toast = useToast();
const PickaxeSearchPanel = defineAsyncComponent(() => import("@/features/repository/pickaxe/PickaxeSearchPanel.vue"));
const { scheduleDestructiveAction } = useUndoableDestructiveAction();
const {
  generalFontSize,
  commitNumeric,
  applyGeneralFontSize,
  applyCommitNumeric,
} = useAppAppearance();
const {
  showLogsPanel,
  appLogs,
  userLogs,
  errorLogs,
  appendLog,
  toggleLogsPanel,
} = useAppLogs();
const {
  recentRepos,
  addToRecent,
  removeRecent,
  clearRecent,
} = useRecentRepositories();
const {
  tabs,
  activeTabId,
  isLanding,
  canCloseActiveTab,
  canReopenClosedTab,
  openReposList,
  restoreTabs,
  selectTab,
  selectAdjacentTab,
  closeTab,
  newTab,
  reopenClosedTab,
  closeActiveTab,
  setActiveTabRepository,
} = useRepositoryTabs({
  openRepository: (path, options) => git.openRepository(path, options),
});

type HistoryViewMode = "graph" | "galaxy" | "city" | "productivity" | "time-machine" | "conflict-heatmap" | "burnout" | "remote-insights" | "conflict-resolve" | "lost-found";
type RemoteInsightsViewMode = "pull-request-detail" | "pull-request-create" | "issue-detail" | "issue-create";
type CommitSelectionPayload = CommitInfo | { commit: CommitInfo | null; additive?: boolean } | null;
type OptionsInitialSection = "integrations" | "git" | "preferences" | "advanced" | "organisations";
type CommandPaletteTone = "default" | "success" | "warning" | "danger";

interface CommandPaletteAction {
  id: string;
  label: string;
  description?: string;
  shortcut?: string;
  keywords?: string[];
  disabled?: boolean;
  tone?: CommandPaletteTone;
  run: () => void | Promise<void>;
}

const AUTO_FETCH_SETTINGS_EVENT = "gitswamp:auto-fetch-settings-changed";
const AUTO_FETCH_ENABLED_KEY = "gitswamp-auto-fetch-enabled";
const AUTO_FETCH_INTERVAL_KEY = "gitswamp-auto-fetch-interval-minutes";
const AUTO_FETCH_DEFAULT_INTERVAL_MINUTES = 3;
const BACKGROUND_STALE_WORK_MIN_MS = 30 * 60 * 1000;
const BACKGROUND_STALE_WORK_TOAST_COOLDOWN_MS = 45 * 60 * 1000;
const BACKGROUND_FOCUS_REMOTE_COOLDOWN_MS = 5 * 60 * 1000;
const BACKGROUND_COMMIT_PRELOAD_LIMIT = 3;
const BACKGROUND_IDLE_MIN_MS = 20 * 1000;
const BACKGROUND_BRANCH_REMINDER_COOLDOWN_MS = 30 * 60 * 1000;
const BACKGROUND_LARGE_CHANGE_REMINDER_COOLDOWN_MS = 30 * 60 * 1000;
const BACKGROUND_CONFLICT_REMINDER_COOLDOWN_MS = 20 * 60 * 1000;
const GITHUB_INITIAL_INSIGHT_LIMIT = 50;
const GITHUB_DETAIL_REFERENCE_LIMIT = 16;
const GIT_RPG_PROFILE_CACHE_LIMIT = 8;
const GIT_RPG_COMMIT_SCAN_LIMIT = 160;
const GIT_RPG_STAT_SCAN_LIMIT = 70;
const GIT_RPG_ANALYZE_DELAY_MS = 850;
const LOST_FOUND_SCAN_DELAY_MS = 1200;
const LOST_FOUND_SCAN_LIMIT = 50;

interface CloneProgressEventPayload {
  url: string;
  destination: string;
  phase: string;
  percent: number;
  message: string;
  received_objects: number;
  total_objects: number;
  indexed_objects: number;
  received_bytes: number;
  indexed_deltas: number;
  total_deltas: number;
}

const historyViewMode = ref<HistoryViewMode>("graph");
const timeMachineFocusSha = ref<string | null>(null);
const showCloneDialog = ref(false);
const showInitDialog = ref(false);
const showTerminal = ref(false);
const terminalAllowAll = ref(safeStorageGet("gitswamp-terminal-allow-all") === "true");
const activeRemoteAction = ref<"pull" | "push" | "fetch" | null>(null);
const showBranchDialog = ref(false);
const showGhostMaterializeDialog = ref(false);
const ghostMaterializeName = ref("");
const showStashDialog = ref(false);
const showOptions = ref(false);
const optionsMounted = ref(false);
const optionsInitialSection = ref<OptionsInitialSection>("integrations");
const autoFetchTimerId = ref<number | null>(null);
const autoFetchInFlight = ref(false);
const backgroundMaintenanceTimerId = ref<number | null>(null);
const backgroundMaintenanceInFlight = ref(false);
const newBranchName = ref("");
const stashMessage = ref("");
const showEditMessageDialog = ref(false);
const editMessageSha = ref("");
const editMessageText = ref("");
const smartGitignoreWizardEnabled = ref(getStoredSmartGitignoreWizardEnabled());
const bugAutopsyEnabled = ref(getStoredBugAutopsyEnabled());
const identityGuardEnabled = ref(getStoredIdentityGuardEnabled());
const identityGuardMismatch = ref<IdentityGuardMismatch | null>(null);
const showRenameDialog = ref(false);
const renameBranchOld = ref("");
const renameBranchNew = ref("");
const showAnnotatedTagDialog = ref(false);
const annotatedTagSha = ref("");
const annotatedTagName = ref("");
const annotatedTagMessage = ref("");
const showRebaseConflictDialog = ref(false);
const rebaseConflictSource = ref("");
const rebaseConflictTarget = ref("");
const rebaseConflictBusy = ref(false);

const viewingWorkingChanges = ref(false);
const viewingStash = ref(false);
const showCommandPalette = ref(false);
const showPickaxeSearch = ref(false);

const showDiffViewer = ref(false);
const diffFilePath = ref("");
const diffCommitSha = ref<string | null>(null);
const diffStaged = ref(false);
const diffFallbackStatus = ref<string | null>(null);
const diffFallbackOldPath = ref<string | null>(null);

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
const githubGists = ref<GistInfo[]>([]);
const githubIssuesHasMore = ref(false);
const githubPullRequestsHasMore = ref(false);
const githubIssuesLoadingAll = ref(false);
const githubPullRequestsLoadingAll = ref(false);
const githubIssueDetail = ref<IssueInfo | null>(null);
const githubPullRequestDetail = ref<PullRequestInfo | null>(null);
const remoteInsightDetailLoading = ref(false);
const remoteCreateOptions = ref<RemoteCreateOptions>(createEmptyRemoteCreateOptions());
const remoteCreateOptionsLoading = ref(false);
const gitRpgProfile = ref<GitRpgProfile | null>(null);
const gitRpgLoading = ref(false);
const lostCommits = ref<LostCommitInfo[]>([]);
const lostCommitsLoading = ref(false);
const rescuingLostCommitSha = ref<string | null>(null);
const selectedIssueNumber = ref<number | null>(null);
const selectedPullRequestNumber = ref<number | null>(null);
const remoteInsightsMode = ref<RemoteInsightsViewMode>("pull-request-detail");
let pullRequestFetchSequence = 0;
let pullRequestFetchTimer: ReturnType<typeof setTimeout> | null = null;
let remoteInsightDetailSequence = 0;
let remoteCreateOptionsSequence = 0;
let gitRpgProfileSequence = 0;
let gitRpgProfileTimer: ReturnType<typeof setTimeout> | null = null;
let lostFoundScanSequence = 0;
let lostFoundScanTimer: ReturnType<typeof setTimeout> | null = null;
let identityGuardCheckSequence = 0;
const identityGuardPromptedKeys = new Set<string>();

const showAuthRequiredDialog = ref(false);
const authProvider = ref<"github" | "gitlab" | "gitlab-self" | "bitbucket" | "azure">("github");
const authTokenInput = ref("");
const authDomainInput = ref("");
const authEmailInput = ref("");
const authKeyNameInput = ref("gitswamp");
const authSubmitting = ref(false);
let multiCommitFilesRunToken = 0;
let singleCommitLoadKey: string | null = null;
let selectedCommitWatcherSkipSha: string | null = null;
let backgroundRemoteHygieneLastRunAt = 0;
let backgroundStaleWorkSignature = "";
let backgroundStaleWorkStartedAt = 0;
let backgroundStaleWorkLastToastAt = 0;
let backgroundCommitPreloadRunKey = "";
let backgroundLastUserActivityAt = Date.now();
let backgroundBehindBranchSignature = "";
let backgroundBehindBranchLastToastAt = 0;
let backgroundLargeChangeSignature = "";
let backgroundLargeChangeLastToastAt = 0;
let backgroundConflictSignature = "";
let backgroundConflictLastToastAt = 0;
const COMMIT_FILES_CACHE_LIMIT = 12;
const COMMIT_FILES_CACHE_MAX_FILES = 350;
const commitFilesCache = new Map<string, CommitFileInfo[]>();
const gitRpgProfileCache = new Map<string, GitRpgProfile>();

function getCommitFilesCacheKey(repoPath: string, sha: string) {
  return `${repoPath}\u0000${sha}`;
}

function getCachedCommitFiles(repoPath: string, sha: string) {
  const key = getCommitFilesCacheKey(repoPath, sha);
  const cached = commitFilesCache.get(key);
  if (!cached) return null;
  commitFilesCache.delete(key);
  commitFilesCache.set(key, cached);
  return cached;
}

function cacheCommitFiles(repoPath: string, sha: string, files: CommitFileInfo[]) {
  if (files.length > COMMIT_FILES_CACHE_MAX_FILES) return;
  const key = getCommitFilesCacheKey(repoPath, sha);
  if (commitFilesCache.has(key)) {
    commitFilesCache.delete(key);
  }
  commitFilesCache.set(key, files);
  while (commitFilesCache.size > COMMIT_FILES_CACHE_LIMIT) {
    const oldestKey = commitFilesCache.keys().next().value;
    if (!oldestKey) break;
    commitFilesCache.delete(oldestKey);
  }
}

function clearCommitFilesCacheForOtherRepos(repoPath: string) {
  const activePrefix = `${repoPath}\u0000`;
  for (const key of commitFilesCache.keys()) {
    if (!key.startsWith(activePrefix)) {
      commitFilesCache.delete(key);
    }
  }
}

function openDiffViewer(
  filePath: string,
  commitSha: string | null,
  staged: boolean,
  fallback?: { status?: string | null; oldPath?: string | null },
) {
  diffFilePath.value = filePath;
  diffCommitSha.value = commitSha;
  diffStaged.value = staged;
  diffFallbackStatus.value = fallback?.status || null;
  diffFallbackOldPath.value = fallback?.oldPath || null;
  showDiffViewer.value = true;
}

function closeDiffViewer() {
  showDiffViewer.value = false;
  diffFallbackStatus.value = null;
  diffFallbackOldPath.value = null;
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

function normalizeAzureDomainInput(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";

  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  try {
    const parsed = new URL(candidate);
    const host = parsed.host.toLowerCase();
    const pathSegments = parsed.pathname.split("/");
    const firstPathSegment = pathSegments.find((segment) => segment.length > 0) || "";
    if (host === "dev.azure.com") {
      const organization = firstPathSegment;
      return organization ? `${host}/${organization.toLowerCase()}` : host;
    }
    return host;
  } catch {
    const withoutProtocol = trimmed.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "");
    const parts = withoutProtocol.split("/");
    const hostPart = parts.find((segment) => segment.trim().length > 0) || "";
    if (!hostPart) return "";
    const hostPartLower = hostPart.toLowerCase();
    if (hostPartLower === "dev.azure.com") {
      const orgPart = parts.find((segment, index) => index > 0 && segment.trim().length > 0) || "";
      return orgPart
        ? `${hostPartLower}/${orgPart.toLowerCase()}`
        : hostPartLower;
    }
    return hostPartLower;
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

function detectAuthProviderFromOrigin(): "github" | "gitlab" | "gitlab-self" | "bitbucket" | "azure" {
  const origin = git.repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url || "";
  const originLower = origin.toLowerCase();
  const parsed = parseRemoteHostAndPath(origin);

  if ((parsed?.host || "") === "bitbucket.org" || originLower.includes("bitbucket.org") || (parsed?.host || "").includes("bitbucket.")) {
    return "bitbucket";
  }

  if ((parsed?.host || "") === "dev.azure.com" || originLower.includes("dev.azure.com") || (parsed?.host || "").endsWith("visualstudio.com")) {
    return "azure";
  }

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

function parseAzureDomainFromOrigin(): string {
  const origin = git.repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url || "";
  const parsed = parseRemoteHostAndPath(origin);
  if (!parsed) return "";

  if (parsed.host === "dev.azure.com") {
    const pathSegments = parsed.path.split("/");
    const firstPath = pathSegments.find((segment) => segment.length > 0) || "";
    return firstPath ? `${parsed.host}/${firstPath}` : parsed.host;
  }

  if (parsed.host.endsWith("visualstudio.com")) {
    return parsed.host;
  }

  return "";
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

function pluralCount(count: number, singular: string, plural = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : plural}`;
}

function currentBranchDivergence(): { ahead: number; behind: number } {
  const current = git.currentBranch.value;
  const local = git.localBranches.value.find((branch) => branch.name === current || branch.is_head);
  return {
    ahead: local?.ahead || 0,
    behind: local?.behind || 0,
  };
}

function explainGitState() {
  const branch = git.currentBranch.value || "detached HEAD";
  const staged = git.stagedFiles.value.length;
  const unstaged = git.unstagedFiles.value.length;
  const conflicts = git.conflictFiles.value.length;
  const { ahead, behind } = currentBranchDivergence();
  const changedFiles = collectChangedWorkingPaths().length;
  const hasOrigin = (git.repoInfo.value?.remotes || []).some((remote: RemoteInfo) => remote.name === "origin");
  const clean = staged === 0 && unstaged === 0 && conflicts === 0;
  const state = conflicts > 0
    ? "conflicts"
    : ahead > 0 && behind > 0
      ? "diverged"
      : behind > 0
        ? "behind origin"
        : ahead > 0
          ? "ahead of origin"
          : clean
            ? "clean"
            : "local changes";
  const rpgLines = gitRpgProfileDetailLines(gitRpgProfile.value);
  const lines = [
    ...rpgLines,
    "",
    `State: ${state}.`,
    `Branch: ${branch}.`,
    `Files: ${pluralCount(staged, "staged file")}, ${pluralCount(unstaged, "unstaged file")}, ${pluralCount(conflicts, "conflict")}.`,
    hasOrigin
      ? `Origin: ${pluralCount(ahead, "commit")} ahead, ${pluralCount(behind, "commit")} behind.`
      : "Origin: no origin remote is configured.",
  ];

  if (conflicts > 0) {
    lines.push("Next: open conflict resolver, fix conflicted files, then stage resolved files.");
    lines.push("Avoid: reset, pull, or commit until conflicts are resolved.");
  } else if (ahead > 0 && behind > 0) {
    lines.push("Next: rebase or merge remote changes carefully, then push after testing.");
    if (changedFiles > 0) lines.push("Tip: commit/stash local changes before syncing.");
  } else if (behind > 0 && changedFiles > 0) {
    lines.push("Next: commit or stash local changes, then pull/rebase from origin.");
  } else if (staged > 0) {
    lines.push("Next: commit staged changes, or stage related unstaged files first.");
  } else if (unstaged > 0) {
    lines.push("Next: review the diff, then stage wanted files or discard only unstaged changes.");
  } else if (behind > 0) {
    lines.push("Next: pull/rebase before starting new work.");
  } else if (ahead > 0) {
    lines.push("Next: push when tests and review look good.");
  } else if (!hasOrigin) {
    lines.push("Next: add a remote if you want to push or sync this repository.");
  } else {
    lines.push("Next: continue working, create a branch, or pull/fetch to check for updates.");
  }

  if (originConflictRisk.value.level !== "none") {
    lines.push(`Warning: ${originConflictRisk.value.label}`);
  }

  const title = gitRpgProfile.value
    ? `Git RPG: ${gitRpgProfile.value.primaryRole.title}`
    : `Git State: ${branch}`;
  toast.infoDetail(title, lines.join("\n"), 16000);
}

function gitRpgProfileCacheKey(repoPath: string, commits: CommitInfo[]): string {
  const head = commits[0]?.sha || "empty";
  const sampleSize = Math.min(commits.length, GIT_RPG_COMMIT_SCAN_LIMIT);
  return `${repoPath}\u0000${head}\u0000${sampleSize}`;
}

function rememberGitRpgProfile(key: string, profile: GitRpgProfile): void {
  if (gitRpgProfileCache.has(key)) {
    gitRpgProfileCache.delete(key);
  }
  gitRpgProfileCache.set(key, profile);
  while (gitRpgProfileCache.size > GIT_RPG_PROFILE_CACHE_LIMIT) {
    const oldestKey = gitRpgProfileCache.keys().next().value;
    if (!oldestKey) break;
    gitRpgProfileCache.delete(oldestKey);
  }
}

function scheduleGitRpgProfileRefresh(delay = GIT_RPG_ANALYZE_DELAY_MS): void {
  if (gitRpgProfileTimer) {
    clearTimeout(gitRpgProfileTimer);
  }

  gitRpgProfileTimer = setTimeout(() => {
    void refreshGitRpgProfile();
  }, delay);
}

async function getGitRpgCommitSample(repoPath: string, seed: CommitInfo[], sequence: number): Promise<CommitInfo[]> {
  if (seed.length >= GIT_RPG_COMMIT_SCAN_LIMIT || seed.length >= 50) {
    return seed.slice(0, GIT_RPG_COMMIT_SCAN_LIMIT);
  }

  try {
    const commits = await invoke<CommitInfo[]>("get_commits", {
      path: repoPath,
      maxCount: GIT_RPG_COMMIT_SCAN_LIMIT,
      quick: true,
    });
    if (sequence !== gitRpgProfileSequence || git.repoPath.value !== repoPath) {
      return [];
    }
    return commits.length > 0 ? commits.slice(0, GIT_RPG_COMMIT_SCAN_LIMIT) : seed;
  } catch {
    return seed.slice(0, GIT_RPG_COMMIT_SCAN_LIMIT);
  }
}

async function collectGitRpgStats(repoPath: string, commits: CommitInfo[], sequence: number): Promise<Map<string, GitRpgCommitStats>> {
  const statsBySha = new Map<string, GitRpgCommitStats>();
  const sample = commits.slice(0, GIT_RPG_STAT_SCAN_LIMIT);

  for (let index = 0; index < sample.length; index += 1) {
    if (sequence !== gitRpgProfileSequence || git.repoPath.value !== repoPath) {
      return statsBySha;
    }

    const commit = sample[index];
    if (!commit) continue;

    try {
      const files = await invoke<CommitFileInfo[]>("get_commit_files", { path: repoPath, sha: commit.sha });
      statsBySha.set(commit.sha, summarizeCommitFilesForRpg(files));
    } catch {
      // Best-effort RPG metadata: message/time based roles still work if a commit diff is unavailable.
    }

    if (index > 0 && index % 8 === 0) {
      await new Promise<void>((resolve) => globalThis.setTimeout(resolve, 0));
    }
  }

  return statsBySha;
}

async function refreshGitRpgProfile(): Promise<void> {
  const repoPath = git.repoPath.value;
  const seedCommits = git.commits.value;

  if (!repoPath || seedCommits.length === 0) {
    gitRpgProfileSequence += 1;
    gitRpgProfile.value = null;
    gitRpgLoading.value = false;
    return;
  }

  const cacheKey = gitRpgProfileCacheKey(repoPath, seedCommits);
  const cached = gitRpgProfileCache.get(cacheKey);
  if (cached) {
    gitRpgProfile.value = cached;
    gitRpgLoading.value = false;
    return;
  }

  const sequence = ++gitRpgProfileSequence;
  gitRpgLoading.value = true;

  try {
    const commits = await getGitRpgCommitSample(repoPath, seedCommits, sequence);
    if (sequence !== gitRpgProfileSequence || git.repoPath.value !== repoPath || commits.length === 0) return;

    const statsBySha = await collectGitRpgStats(repoPath, commits, sequence);
    if (sequence !== gitRpgProfileSequence || git.repoPath.value !== repoPath) return;

    const profile = buildGitRpgProfile({
      repoPath,
      commits,
      statsBySha,
    });
    rememberGitRpgProfile(cacheKey, profile);
    gitRpgProfile.value = profile;
  } finally {
    if (sequence === gitRpgProfileSequence) {
      gitRpgLoading.value = false;
    }
  }
}

interface GithubApiContext {
  apiBase: string;
  owner: string;
  repo: string;
  token: string;
}

interface GithubListResult<T> {
  items: T[];
  hasMore: boolean;
}

interface RemoteCreateOptions {
  labels: RemoteLabelInfo[];
  milestones: RemoteMilestoneInfo[];
  assignees: RemoteUserInfo[];
  reviewers: RemoteUserInfo[];
}

function createEmptyRemoteCreateOptions(): RemoteCreateOptions {
  return {
    labels: [],
    milestones: [],
    assignees: [],
    reviewers: [],
  };
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

function githubUserLogin(value: unknown): string {
  if (!value || typeof value !== "object") return "";
  return String((value as Record<string, unknown>).login || "").trim();
}

function githubUserLogins(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .map(githubUserLogin)
    .filter((login) => login.length > 0);
}

function mapGithubLabels(value: unknown): NonNullable<IssueInfo["labels"]> {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => {
      if (!item || typeof item !== "object") return null;
      const label = item as Record<string, unknown>;
      const name = String(label.name || "").trim();
      if (!name) return null;
      return {
        name,
        color: String(label.color || "64748b").replace(/^#/, ""),
        description: String(label.description || ""),
      };
    })
    .filter((item): item is NonNullable<IssueInfo["labels"]>[number] => item !== null);
}

function mapGithubMilestone(value: unknown): IssueInfo["milestone"] {
  if (!value || typeof value !== "object") return null;
  const milestone = value as Record<string, unknown>;
  const title = String(milestone.title || "").trim();
  if (!title) return null;
  return {
    number: Number(milestone.number) || undefined,
    title,
    state: String(milestone.state || ""),
    description: String(milestone.description || ""),
    dueOn: String(milestone.due_on || ""),
    openIssues: Number(milestone.open_issues) || 0,
    closedIssues: Number(milestone.closed_issues) || 0,
  };
}

function mapGithubUserInfo(item: unknown): RemoteUserInfo | null {
  if (!item || typeof item !== "object") return null;
  const user = item as Record<string, unknown>;
  const login = String(user.login || "").trim();
  if (!login) return null;
  return {
    login,
    avatarUrl: String(user.avatar_url || ""),
    url: String(user.html_url || ""),
  };
}

function uniqueGithubReferences(items: RemoteReferenceInfo[]): RemoteReferenceInfo[] {
  const seen = new Set<string>();
  const unique: RemoteReferenceInfo[] = [];
  for (const item of items) {
    const key = `${item.kind}:${item.number ?? item.sha ?? item.url}`;
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push(item);
  }
  return unique.slice(0, GITHUB_DETAIL_REFERENCE_LIMIT);
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
    assignees: githubUserLogins(value.assignees),
    requestedReviewers: githubUserLogins(value.requested_reviewers),
    labels: mapGithubLabels(value.labels),
    milestone: mapGithubMilestone(value.milestone),
    comments: Number(value.comments) || 0,
    reviewComments: Number(value.review_comments) || 0,
    commitsCount: Number(value.commits) || 0,
    changedFiles: Number(value.changed_files) || 0,
    additions: Number(value.additions) || 0,
    deletions: Number(value.deletions) || 0,
    mergeable: typeof value.mergeable === "boolean" ? value.mergeable : null,
    mergeableState: String(value.mergeable_state || ""),
    merged: Boolean(value.merged),
    mergedAt: String(value.merged_at || ""),
    mergedBy: githubUserLogin(value.merged_by),
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
    assignees: githubUserLogins(value.assignees),
    labels: mapGithubLabels(value.labels),
    milestone: mapGithubMilestone(value.milestone),
    comments: Number(value.comments) || 0,
    closedAt: String(value.closed_at || ""),
    stateReason: String(value.state_reason || ""),
  };
}

function mapGithubGist(item: unknown): GistInfo | null {
  if (!item || typeof item !== "object") return null;
  const value = item as Record<string, unknown>;
  const id = String(value.id || "").trim();
  const url = String(value.html_url || "").trim();
  if (!id || !url) return null;

  const files = value.files && typeof value.files === "object"
    ? Object.values(value.files as Record<string, unknown>)
    : [];
  const filenames = files
    .map((file) => {
      if (!file || typeof file !== "object") return "";
      return String((file as Record<string, unknown>).filename || "").trim();
    })
    .filter(Boolean);

  return {
    id,
    description: String(value.description || "").trim(),
    filename: filenames[0] || "gist",
    fileCount: filenames.length,
    public: Boolean(value.public),
    updatedAt: String(value.updated_at || ""),
    url,
  };
}

async function fetchGithubPullRequests(remoteUrl: string): Promise<GithubListResult<PullRequestInfo>> {
  const context = getGithubApiContext(remoteUrl);
  if (!context) return { items: [], hasMore: false };

  const payload = await fetchJsonWithTimeout(
    `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/pulls?state=open&per_page=${GITHUB_INITIAL_INSIGHT_LIMIT + 1}`,
    githubHeaders(context.token),
  );

  if (!Array.isArray(payload)) return { items: [], hasMore: false };
  const mapped = payload
    .map(mapGithubPullRequest)
    .filter((item): item is PullRequestInfo => item !== null);

  return {
    items: mapped.slice(0, GITHUB_INITIAL_INSIGHT_LIMIT),
    hasMore: mapped.length > GITHUB_INITIAL_INSIGHT_LIMIT,
  };
}

async function fetchAllGithubPullRequests(remoteUrl: string): Promise<PullRequestInfo[]> {
  const context = getGithubApiContext(remoteUrl);
  if (!context) return [];

  const pullRequests: PullRequestInfo[] = [];
  for (let page = 1; page <= 20; page += 1) {
    const payload = await fetchJsonWithTimeout(
      `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/pulls?state=open&per_page=100&page=${page}`,
      githubHeaders(context.token),
    );
    if (!Array.isArray(payload) || payload.length === 0) break;
    pullRequests.push(
      ...payload
        .map(mapGithubPullRequest)
        .filter((item): item is PullRequestInfo => item !== null),
    );
    if (payload.length < 100) break;
  }

  return pullRequests;
}

async function fetchGithubIssues(remoteUrl: string): Promise<GithubListResult<IssueInfo>> {
  const context = getGithubApiContext(remoteUrl);
  if (!context) return { items: [], hasMore: false };

  const params = new URLSearchParams({
    q: `repo:${context.owner}/${context.repo} is:issue is:open`,
    sort: "updated",
    order: "desc",
    per_page: String(GITHUB_INITIAL_INSIGHT_LIMIT),
  });

  const payload = await fetchJsonWithTimeout(
    `${context.apiBase}/search/issues?${params.toString()}`,
    githubHeaders(context.token),
  );

  if (!payload || typeof payload !== "object") return { items: [], hasMore: false };
  const value = payload as Record<string, unknown>;
  const rawItems = Array.isArray(value.items) ? value.items : [];
  const totalCount = Number(value.total_count) || rawItems.length;
  const items = rawItems
    .map(mapGithubIssue)
    .filter((item): item is IssueInfo => item !== null)
    .slice(0, GITHUB_INITIAL_INSIGHT_LIMIT);

  return {
    items,
    hasMore: totalCount > items.length,
  };
}

async function fetchAllGithubIssues(remoteUrl: string): Promise<IssueInfo[]> {
  const context = getGithubApiContext(remoteUrl);
  if (!context) return [];

  const issues: IssueInfo[] = [];
  for (let page = 1; page <= 10; page += 1) {
    const params = new URLSearchParams({
      q: `repo:${context.owner}/${context.repo} is:issue is:open`,
      sort: "updated",
      order: "desc",
      per_page: "100",
      page: String(page),
    });
    const payload = await fetchJsonWithTimeout(
      `${context.apiBase}/search/issues?${params.toString()}`,
      githubHeaders(context.token),
    );

    if (!payload || typeof payload !== "object") break;
    const value = payload as Record<string, unknown>;
    const rawItems = Array.isArray(value.items) ? value.items : [];
    if (rawItems.length === 0) break;
    issues.push(
      ...rawItems
        .map(mapGithubIssue)
        .filter((item): item is IssueInfo => item !== null),
    );
    if (rawItems.length < 100) break;
  }

  return issues;
}

async function fetchGithubGists(remoteUrl: string): Promise<GistInfo[]> {
  const context = getGithubApiContext(remoteUrl);
  if (!context || !context.token) return [];

  const payload = await fetchJsonWithTimeout(
    `${context.apiBase}/gists?per_page=30`,
    githubHeaders(context.token),
  );

  if (!Array.isArray(payload)) return [];
  return payload
    .map(mapGithubGist)
    .filter((item): item is GistInfo => item !== null);
}

function mapGithubMilestoneOptions(payload: unknown): RemoteMilestoneInfo[] {
  if (!Array.isArray(payload)) return [];
  return payload
    .map(mapGithubMilestone)
    .filter((item): item is RemoteMilestoneInfo => item != null && typeof item.number === "number");
}

function mapGithubUserOptions(payload: unknown): RemoteUserInfo[] {
  if (!Array.isArray(payload)) return [];
  const seen = new Set<string>();
  const users: RemoteUserInfo[] = [];
  for (const item of payload) {
    const user = mapGithubUserInfo(item);
    if (!user || seen.has(user.login.toLowerCase())) continue;
    seen.add(user.login.toLowerCase());
    users.push(user);
  }
  return users;
}

async function fetchGithubCreateOptions(): Promise<RemoteCreateOptions> {
  const context = getPrimaryGithubContext();
  if (!context) return createEmptyRemoteCreateOptions();

  const headers = githubHeaders(context.token);
  const repoPath = `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}`;
  const [labelsPayload, milestonesPayload, assigneesPayload, collaboratorsPayload] = await Promise.all([
    fetchJsonWithTimeout(`${repoPath}/labels?per_page=100`, headers).catch(() => []),
    fetchJsonWithTimeout(`${repoPath}/milestones?state=open&per_page=100`, headers).catch(() => []),
    fetchJsonWithTimeout(`${repoPath}/assignees?per_page=100`, headers).catch(() => []),
    fetchJsonWithTimeout(`${repoPath}/collaborators?affiliation=direct&per_page=100`, headers).catch(() => []),
  ]);

  const assignees = mapGithubUserOptions(assigneesPayload);
  const collaboratorUsers = mapGithubUserOptions(collaboratorsPayload);
  const reviewerMap = new Map<string, RemoteUserInfo>();
  for (const user of [...collaboratorUsers, ...assignees]) {
    reviewerMap.set(user.login.toLowerCase(), user);
  }

  return {
    labels: mapGithubLabels(labelsPayload),
    milestones: mapGithubMilestoneOptions(milestonesPayload),
    assignees,
    reviewers: Array.from(reviewerMap.values()),
  };
}

function mapGithubTimelineReferences(payload: unknown): {
  pullRequests: NonNullable<IssueInfo["linkedPullRequests"]>;
  commits: NonNullable<IssueInfo["linkedCommits"]>;
  issues: NonNullable<IssueInfo["linkedIssues"]>;
} {
  const pullRequests: NonNullable<IssueInfo["linkedPullRequests"]> = [];
  const commits: NonNullable<IssueInfo["linkedCommits"]> = [];
  const issues: NonNullable<IssueInfo["linkedIssues"]> = [];
  if (!Array.isArray(payload)) {
    return { pullRequests, commits, issues };
  }

  for (const item of payload) {
    if (!item || typeof item !== "object") continue;
    const value = item as Record<string, unknown>;
    const createdAt = String(value.created_at || "");
    const actor = githubUserLogin(value.actor);
    const source = value.source && typeof value.source === "object" ? value.source as Record<string, unknown> : null;
    const sourceIssue = source?.issue && typeof source.issue === "object" ? source.issue as Record<string, unknown> : null;

    if (sourceIssue) {
      const number = Number(sourceIssue.number);
      const title = String(sourceIssue.title || "").trim();
      const url = String(sourceIssue.html_url || "").trim();
      if (Number.isFinite(number) && title && url) {
        const entry = {
          kind: sourceIssue.pull_request ? "pull_request" as const : "issue" as const,
          number,
          title,
          state: String(sourceIssue.state || ""),
          url,
          author: githubUserLogin(sourceIssue.user) || actor,
          occurredAt: createdAt,
        };
        if (entry.kind === "pull_request") {
          pullRequests.push(entry);
        } else {
          issues.push(entry);
        }
      }
    }

    const commitSha = String(value.commit_id || "").trim();
    const commitUrl = String(value.html_url || value.commit_url || "").trim();
    if (commitSha && commitUrl) {
      commits.push({
        kind: "commit",
        sha: commitSha,
        title: commitSha.slice(0, 7),
        url: commitUrl,
        author: actor,
        occurredAt: createdAt,
      });
    }
  }

  return {
    pullRequests: uniqueGithubReferences(pullRequests),
    commits: uniqueGithubReferences(commits),
    issues: uniqueGithubReferences(issues),
  };
}

function mapGithubPullRequestCommitReferences(payload: unknown): NonNullable<PullRequestInfo["linkedCommits"]> {
  if (!Array.isArray(payload)) return [];
  const commits: RemoteReferenceInfo[] = [];
  for (const item of payload) {
    if (!item || typeof item !== "object") continue;
    const value = item as Record<string, unknown>;
    const commit = value.commit && typeof value.commit === "object" ? value.commit as Record<string, unknown> : {};
    const message = String(commit.message || "").split("\n")[0]?.trim() || String(value.sha || "").slice(0, 7);
    const sha = String(value.sha || "").trim();
    const url = String(value.html_url || "").trim();
    if (!sha || !url) continue;
    const author = commit.author && typeof commit.author === "object"
      ? String((commit.author as Record<string, unknown>).name || "")
      : githubUserLogin(value.author);
    commits.push({
      kind: "commit",
      sha,
      title: message,
      url,
      author,
      occurredAt: commit.author && typeof commit.author === "object"
        ? String((commit.author as Record<string, unknown>).date || "")
        : "",
    });
  }

  return uniqueGithubReferences(commits);
}

function mapGithubPullRequestFiles(payload: unknown): NonNullable<PullRequestInfo["files"]> {
  if (!Array.isArray(payload)) return [];
  return payload
    .map((item) => {
      if (!item || typeof item !== "object") return null;
      const value = item as Record<string, unknown>;
      const filename = String(value.filename || "").trim();
      if (!filename) return null;
      return {
        filename,
        status: String(value.status || ""),
        additions: Number(value.additions) || 0,
        deletions: Number(value.deletions) || 0,
        changes: Number(value.changes) || 0,
        url: String(value.blob_url || value.raw_url || ""),
      };
    })
    .filter((item): item is NonNullable<PullRequestInfo["files"]>[number] => item !== null)
    .slice(0, GITHUB_DETAIL_REFERENCE_LIMIT);
}

async function fetchGithubIssueDetail(number: number): Promise<IssueInfo | null> {
  const context = getPrimaryGithubContext();
  if (!context) return null;

  const headers = githubHeaders(context.token);
  const [issuePayload, timelinePayload] = await Promise.all([
    fetchJsonWithTimeout(
      `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/issues/${number}`,
      headers,
    ),
    fetchJsonWithTimeout(
      `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/issues/${number}/timeline?per_page=100`,
      headers,
    ).catch(() => []),
  ]);

  const issue = mapGithubIssue(issuePayload);
  if (!issue) return null;
  const references = mapGithubTimelineReferences(timelinePayload);
  return {
    ...issue,
    linkedPullRequests: references.pullRequests,
    linkedCommits: references.commits,
    linkedIssues: references.issues,
  };
}

async function fetchGithubPullRequestDetail(number: number): Promise<PullRequestInfo | null> {
  const context = getPrimaryGithubContext();
  if (!context) return null;

  const headers = githubHeaders(context.token);
  const repoPath = `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}`;
  const [pullPayload, issuePayload, commitsPayload, filesPayload, timelinePayload] = await Promise.all([
    fetchJsonWithTimeout(`${repoPath}/pulls/${number}`, headers),
    fetchJsonWithTimeout(`${repoPath}/issues/${number}`, headers).catch(() => null),
    fetchJsonWithTimeout(`${repoPath}/pulls/${number}/commits?per_page=30`, headers).catch(() => []),
    fetchJsonWithTimeout(`${repoPath}/pulls/${number}/files?per_page=30`, headers).catch(() => []),
    fetchJsonWithTimeout(`${repoPath}/issues/${number}/timeline?per_page=100`, headers).catch(() => []),
  ]);

  const pullRequest = mapGithubPullRequest(pullPayload);
  if (!pullRequest) return null;
  const issueValue = issuePayload && typeof issuePayload === "object" ? issuePayload as Record<string, unknown> : {};
  const references = mapGithubTimelineReferences(timelinePayload);
  return {
    ...pullRequest,
    assignees: githubUserLogins(issueValue.assignees),
    labels: mapGithubLabels(issueValue.labels),
    milestone: mapGithubMilestone(issueValue.milestone),
    comments: Number(issueValue.comments) || pullRequest.comments || 0,
    linkedIssues: references.issues,
    linkedCommits: mapGithubPullRequestCommitReferences(commitsPayload),
    files: mapGithubPullRequestFiles(filesPayload),
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
    githubGists.value = [];
    githubPullRequestsHasMore.value = false;
    githubIssuesHasMore.value = false;
    clearRemoteInsightDetails();
    selectedPullRequestNumber.value = null;
    selectedIssueNumber.value = null;
    return;
  }

  const sequence = ++pullRequestFetchSequence;

  try {
    if (primaryRemote.provider === "github") {
      const [pullRequestsResult, issuesResult, gists] = await Promise.all([
        fetchGithubPullRequests(primaryRemote.url),
        fetchGithubIssues(primaryRemote.url),
        fetchGithubGists(primaryRemote.url),
      ]);

      if (sequence !== pullRequestFetchSequence) return;

      githubPullRequests.value = pullRequestsResult.items;
      githubPullRequestsHasMore.value = pullRequestsResult.hasMore;
      githubIssues.value = issuesResult.items;
      githubIssuesHasMore.value = issuesResult.hasMore;
      githubGists.value = gists;
      clearRemoteInsightDetails();
      ensureRemoteInsightSelection();
      loadRemoteInsightDetailForCurrentSelection();
      openPullRequestBranches.value = uniqueNormalizedBranches(
        pullRequestsResult.items.map((item) => item.sourceBranch),
      );
      return;
    }

    const branches = await fetchOpenGitlabMergeRequestBranches(primaryRemote.url);

    if (sequence !== pullRequestFetchSequence) return;
    githubPullRequests.value = [];
    githubIssues.value = [];
    githubGists.value = [];
    githubPullRequestsHasMore.value = false;
    githubIssuesHasMore.value = false;
    clearRemoteInsightDetails();
    selectedPullRequestNumber.value = null;
    selectedIssueNumber.value = null;
    openPullRequestBranches.value = uniqueNormalizedBranches(branches);
  } catch {
    if (sequence !== pullRequestFetchSequence) return;
    openPullRequestBranches.value = [];
    githubPullRequests.value = [];
    githubIssues.value = [];
    githubGists.value = [];
    githubPullRequestsHasMore.value = false;
    githubIssuesHasMore.value = false;
    clearRemoteInsightDetails();
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

function clearRemoteInsightDetails(): void {
  remoteInsightDetailSequence += 1;
  remoteInsightDetailLoading.value = false;
  githubIssueDetail.value = null;
  githubPullRequestDetail.value = null;
}

function clearRemoteCreateOptions(): void {
  remoteCreateOptionsSequence += 1;
  remoteCreateOptionsLoading.value = false;
  remoteCreateOptions.value = createEmptyRemoteCreateOptions();
}

async function loadRemoteCreateOptions(): Promise<void> {
  const sequence = ++remoteCreateOptionsSequence;
  remoteCreateOptionsLoading.value = true;

  try {
    const options = await fetchGithubCreateOptions();
    if (sequence !== remoteCreateOptionsSequence) return;
    remoteCreateOptions.value = options;
  } catch {
    if (sequence === remoteCreateOptionsSequence) {
      remoteCreateOptions.value = createEmptyRemoteCreateOptions();
    }
  } finally {
    if (sequence === remoteCreateOptionsSequence) {
      remoteCreateOptionsLoading.value = false;
    }
  }
}

async function loadSelectedIssueDetail(number: number): Promise<void> {
  const sequence = ++remoteInsightDetailSequence;
  remoteInsightDetailLoading.value = true;
  githubIssueDetail.value = null;

  try {
    const detail = await fetchGithubIssueDetail(number);
    if (sequence !== remoteInsightDetailSequence || selectedIssueNumber.value !== number) return;
    githubIssueDetail.value = detail;
  } catch {
    if (sequence === remoteInsightDetailSequence) {
      githubIssueDetail.value = null;
    }
  } finally {
    if (sequence === remoteInsightDetailSequence) {
      remoteInsightDetailLoading.value = false;
    }
  }
}

async function loadSelectedPullRequestDetail(number: number): Promise<void> {
  const sequence = ++remoteInsightDetailSequence;
  remoteInsightDetailLoading.value = true;
  githubPullRequestDetail.value = null;

  try {
    const detail = await fetchGithubPullRequestDetail(number);
    if (sequence !== remoteInsightDetailSequence || selectedPullRequestNumber.value !== number) return;
    githubPullRequestDetail.value = detail;
  } catch {
    if (sequence === remoteInsightDetailSequence) {
      githubPullRequestDetail.value = null;
    }
  } finally {
    if (sequence === remoteInsightDetailSequence) {
      remoteInsightDetailLoading.value = false;
    }
  }
}

function loadRemoteInsightDetailForCurrentSelection(): void {
  if (historyViewMode.value !== "remote-insights") return;

  if (remoteInsightsMode.value === "issue-detail" && selectedIssueNumber.value !== null) {
    void loadSelectedIssueDetail(selectedIssueNumber.value);
    return;
  }

  if (remoteInsightsMode.value === "pull-request-detail" && selectedPullRequestNumber.value !== null) {
    void loadSelectedPullRequestDetail(selectedPullRequestNumber.value);
    return;
  }

  clearRemoteInsightDetails();
}

function openIssueDetails(number: number) {
  clearRemoteCreateOptions();
  selectedIssueNumber.value = number;
  remoteInsightsMode.value = "issue-detail";
  setHistoryViewMode("remote-insights");
}

function openPullRequestDetails(number: number) {
  clearRemoteCreateOptions();
  selectedPullRequestNumber.value = number;
  remoteInsightsMode.value = "pull-request-detail";
  setHistoryViewMode("remote-insights");
}

function openCreateIssuePanel() {
  clearRemoteInsightDetails();
  remoteInsightsMode.value = "issue-create";
  setHistoryViewMode("remote-insights");
  void loadRemoteCreateOptions();
}

function openCreatePullRequestPanel() {
  clearRemoteInsightDetails();
  remoteInsightsMode.value = "pull-request-create";
  setHistoryViewMode("remote-insights");
  void loadRemoteCreateOptions();
}

async function loadAllGithubIssues(): Promise<void> {
  const remotes = git.repoInfo.value?.remotes || [];
  const primaryRemote = getPrimaryRemote(remotes);
  if (!primaryRemote || primaryRemote.provider !== "github" || githubIssuesLoadingAll.value) return;

  const sequence = ++pullRequestFetchSequence;
  githubIssuesLoadingAll.value = true;
  try {
    const issues = await fetchAllGithubIssues(primaryRemote.url);
    if (sequence !== pullRequestFetchSequence) return;
    githubIssues.value = issues;
    githubIssuesHasMore.value = false;
  } catch (error) {
    toast.error(`Could not load all issues: ${String(error)}`);
  } finally {
    if (sequence === pullRequestFetchSequence) {
      githubIssuesLoadingAll.value = false;
    }
  }
}

async function loadAllGithubPullRequests(): Promise<void> {
  const remotes = git.repoInfo.value?.remotes || [];
  const primaryRemote = getPrimaryRemote(remotes);
  if (!primaryRemote || primaryRemote.provider !== "github" || githubPullRequestsLoadingAll.value) return;

  const sequence = ++pullRequestFetchSequence;
  githubPullRequestsLoadingAll.value = true;
  try {
    const pullRequests = await fetchAllGithubPullRequests(primaryRemote.url);
    if (sequence !== pullRequestFetchSequence) return;
    githubPullRequests.value = pullRequests;
    githubPullRequestsHasMore.value = false;
    openPullRequestBranches.value = uniqueNormalizedBranches(
      pullRequests.map((item) => item.sourceBranch),
    );
  } catch (error) {
    toast.error(`Could not load all pull requests: ${String(error)}`);
  } finally {
    if (sequence === pullRequestFetchSequence) {
      githubPullRequestsLoadingAll.value = false;
    }
  }
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

function createIssueMetadataPayload(payload: RemoteIssueCreatePayload): Record<string, unknown> {
  const body: Record<string, unknown> = {};
  if (payload.labels.length > 0) body.labels = payload.labels;
  if (payload.assignees.length > 0) body.assignees = payload.assignees;
  if (payload.milestone !== null) body.milestone = payload.milestone;
  return body;
}

async function patchGithubIssueMetadata(context: GithubApiContext, number: number, payload: RemoteIssueCreatePayload): Promise<void> {
  const metadata = createIssueMetadataPayload(payload);
  if (Object.keys(metadata).length === 0) return;

  const response = await fetch(
    `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/issues/${number}`,
    {
      method: "PATCH",
      headers: {
        ...githubHeaders(context.token),
        "Content-Type": "application/json",
      },
      body: JSON.stringify(metadata),
    },
  );

  if (!response.ok) {
    throw new Error(await readGithubError(response));
  }
}

async function requestGithubPullRequestReviewers(context: GithubApiContext, number: number, reviewers: string[]): Promise<void> {
  if (reviewers.length === 0) return;

  const response = await fetch(
    `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}/pulls/${number}/requested_reviewers`,
    {
      method: "POST",
      headers: {
        ...githubHeaders(context.token),
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ reviewers }),
    },
  );

  if (!response.ok) {
    throw new Error(await readGithubError(response));
  }
}

async function createRemoteIssue(payload: RemoteIssueCreatePayload) {
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
          ...createIssueMetadataPayload(payload),
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

    const nextIssues = [created, ...githubIssues.value.filter((item) => item.number !== created.number)];
    githubIssues.value = githubIssuesHasMore.value ? nextIssues.slice(0, GITHUB_INITIAL_INSIGHT_LIMIT) : nextIssues;
    selectedIssueNumber.value = created.number;
    remoteInsightsMode.value = "issue-detail";
    setHistoryViewMode("remote-insights");
    githubIssueDetail.value = created;
    clearRemoteCreateOptions();
    toast.success(`Issue #${created.number} created.`);
    scheduleOpenPullRequestRefresh();
  } catch (e) {
    toast.error("Create issue failed: " + String(e));
  }
}

async function createRemotePullRequest(payload: RemotePullRequestCreatePayload) {
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

    const metadataWarnings: string[] = [];
    try {
      await patchGithubIssueMetadata(context, created.number, payload);
    } catch (error) {
      metadataWarnings.push(`metadata: ${String(error)}`);
    }

    try {
      await requestGithubPullRequestReviewers(context, created.number, payload.reviewers);
    } catch (error) {
      metadataWarnings.push(`reviewers: ${String(error)}`);
    }

    const detailed = await fetchGithubPullRequestDetail(created.number).catch(() => null);
    const nextCreated = detailed || created;
    const nextPullRequests = [
      nextCreated,
      ...githubPullRequests.value.filter((item) => item.number !== nextCreated.number),
    ];
    githubPullRequests.value = githubPullRequestsHasMore.value
      ? nextPullRequests.slice(0, GITHUB_INITIAL_INSIGHT_LIMIT)
      : nextPullRequests;
    openPullRequestBranches.value = uniqueNormalizedBranches(
      githubPullRequests.value.map((item) => item.sourceBranch),
    );
    selectedPullRequestNumber.value = nextCreated.number;
    remoteInsightsMode.value = "pull-request-detail";
    setHistoryViewMode("remote-insights");
    githubPullRequestDetail.value = nextCreated;
    clearRemoteCreateOptions();
    if (metadataWarnings.length > 0) {
      toast.warning(`Pull request #${nextCreated.number} created, but some metadata was not applied.`);
    } else {
      toast.success(`Pull request #${nextCreated.number} created.`);
    }
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
  if (authProvider.value === "gitlab-self") {
    authDomainInput.value = parseAuthDomainFromOrigin();
  } else if (authProvider.value === "azure") {
    authDomainInput.value = parseAzureDomainFromOrigin();
  } else {
    authDomainInput.value = "";
  }
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

function openOptions(section: OptionsInitialSection = "integrations") {
  optionsInitialSection.value = section;
  if (!optionsMounted.value) {
    optionsMounted.value = true;
  }
  showOptions.value = true;
}

function readAutoFetchSettings(): { enabled: boolean; intervalMinutes: number } {
  const enabledRaw = safeStorageGet(AUTO_FETCH_ENABLED_KEY);
  const intervalRaw = safeStorageGet(AUTO_FETCH_INTERVAL_KEY);
  const parsedInterval = Number.parseInt(intervalRaw || "", 10);

  return {
    enabled: enabledRaw === null ? true : enabledRaw !== "false",
    intervalMinutes: Number.isFinite(parsedInterval)
      ? Math.max(1, Math.min(60, parsedInterval))
      : AUTO_FETCH_DEFAULT_INTERVAL_MINUTES,
  };
}

function stopAutoFetchTimer(): void {
  if (autoFetchTimerId.value === null) {
    return;
  }

  clearInterval(autoFetchTimerId.value);
  autoFetchTimerId.value = null;
}

async function runBackgroundAutoFetch(): Promise<void> {
  const { enabled } = readAutoFetchSettings();
  if (!enabled || !git.repoPath.value || autoFetchInFlight.value) {
    return;
  }

  autoFetchInFlight.value = true;
  try {
    await git.backgroundFetchAll();
  } catch {
    // Keep silent in background mode to avoid toast spam.
  } finally {
    autoFetchInFlight.value = false;
  }
}

function restartAutoFetchTimer(): void {
  stopAutoFetchTimer();

  const { enabled, intervalMinutes } = readAutoFetchSettings();
  if (!enabled) {
    return;
  }

  autoFetchTimerId.value = globalThis.setInterval(() => {
    void runBackgroundAutoFetch();
  }, intervalMinutes * 60 * 1000);
}

function handleAutoFetchSettingsChanged(): void {
  restartAutoFetchTimer();
  void runBackgroundAutoFetch();
}

function handleSmartGitignoreWizardChanged(event: Event): void {
  if (event instanceof CustomEvent) {
    smartGitignoreWizardEnabled.value = Boolean(event.detail);
    return;
  }

  smartGitignoreWizardEnabled.value = getStoredSmartGitignoreWizardEnabled();
}

function handleBugAutopsyChanged(event: Event): void {
  if (event instanceof CustomEvent) {
    bugAutopsyEnabled.value = Boolean(event.detail);
    return;
  }

  bugAutopsyEnabled.value = getStoredBugAutopsyEnabled();
}

function handleIdentityGuardChanged(event: Event): void {
  if (event instanceof CustomEvent) {
    identityGuardEnabled.value = Boolean(event.detail);
  } else {
    identityGuardEnabled.value = getStoredIdentityGuardEnabled();
  }

  if (!identityGuardEnabled.value) {
    identityGuardMismatch.value = null;
    return;
  }

  if (identityGuardEnabled.value) {
    void checkGitIdentityGuard("settings");
  }
}

function getIdentityGuardRemoteUrl(): string {
  const remotes = git.repoInfo.value?.remotes || [];
  return getPrimaryRemote(remotes)?.url || "";
}

function shortenIdentityRemoteLabel(remoteUrl: string): string {
  return remoteUrl.length > 96 ? `${remoteUrl.slice(0, 93)}...` : remoteUrl;
}

async function getGitIdentityConfigValue(args: string[]): Promise<string> {
  const repoPath = git.repoPath.value;
  if (!repoPath) {
    return "";
  }

  try {
    const value = await invoke<string>("run_git_command", {
      path: repoPath,
      args,
    });
    return value.trim();
  } catch {
    return "";
  }
}

async function setRepositoryIdentityEmail(email: string): Promise<void> {
  const repoPath = git.repoPath.value;
  const { normalizeIdentityEmail } = await import("@/features/repository/identity/identityGuard");
  const cleanEmail = normalizeIdentityEmail(email);
  if (!repoPath || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(cleanEmail)) {
    toast.warning("Enter a valid Git email address.");
    return;
  }

  try {
    await invoke<string>("run_git_command", {
      path: repoPath,
      args: ["config", "user.email", cleanEmail],
    });
    identityGuardPromptedKeys.clear();
    toast.success(`Repository Git email set to ${cleanEmail}`);
    void checkGitIdentityGuard("settings");
  } catch (error) {
    toast.error(`Failed to set repository Git email: ${String(error)}`);
  }
}

function promptForRepositoryIdentityEmail(defaultEmail: string): void {
  const nextEmail = globalThis.prompt("Git email for this repository", defaultEmail);
  if (nextEmail === null) {
    return;
  }
  void setRepositoryIdentityEmail(nextEmail);
}

async function checkGitIdentityGuard(reason: "repo" | "settings" = "repo", forcePrompt = false): Promise<boolean> {
  if (!identityGuardEnabled.value || !git.repoPath.value || !git.repoInfo.value) {
    identityGuardMismatch.value = null;
    return false;
  }

  const repoPath = git.repoPath.value;
  const remoteUrl = getIdentityGuardRemoteUrl();
  if (!remoteUrl) {
    identityGuardMismatch.value = null;
    return false;
  }

  const runId = ++identityGuardCheckSequence;
  const [currentEmail, globalEmail] = await Promise.all([
    getGitIdentityConfigValue(["config", "user.email"]),
    getGitIdentityConfigValue(["config", "--global", "user.email"]),
  ]);

  if (runId !== identityGuardCheckSequence || repoPath !== git.repoPath.value || !identityGuardEnabled.value) {
    return false;
  }

  const { detectGitIdentityMismatch } = await import("@/features/repository/identity/identityGuard");
  const mismatch = detectGitIdentityMismatch(currentEmail, globalEmail, remoteUrl);
  if (!mismatch) {
    identityGuardMismatch.value = null;
    return false;
  }
  identityGuardMismatch.value = mismatch;

  const promptKey = [
    repoPath,
    mismatch.remoteUrl,
    mismatch.currentEmail,
    mismatch.globalEmail,
    mismatch.reason,
  ].join("\u0000");
  if (!forcePrompt && identityGuardPromptedKeys.has(promptKey)) {
    return true;
  }
  identityGuardPromptedKeys.add(promptKey);

  const fallbackEmail = mismatch.suggestedEmail || mismatch.globalEmail || mismatch.currentEmail;
  const actions = [
    ...(mismatch.suggestedEmail
      ? [{
          label: "Use suggested email",
          style: "success" as const,
          onClick: async () => setRepositoryIdentityEmail(mismatch.suggestedEmail || ""),
        }]
      : []),
    {
      label: "Set repo email",
      style: "primary" as const,
      onClick: async () => promptForRepositoryIdentityEmail(fallbackEmail),
    },
    {
      label: "Ignore",
      style: "neutral" as const,
      onClick: async () => {},
    },
  ];

  toast.action(
    "warning",
    reason === "settings" ? "Git Identity Guard is active." : "Git identity mismatch detected.",
    actions,
    22000,
    [
      `Current: ${mismatch.currentEmail || "not configured"}`,
      mismatch.suggestedEmail ? `Suggested: ${mismatch.suggestedEmail}` : null,
      `Remote: ${shortenIdentityRemoteLabel(mismatch.remoteUrl)}`,
      mismatch.reason,
    ].filter(Boolean).join("\n"),
  );
  return true;
}

async function explainIdentityGuardState(): Promise<void> {
  if (!identityGuardEnabled.value) {
    toast.info("Git Identity Guard is disabled.");
    return;
  }

  const hasMismatch = await checkGitIdentityGuard("settings", true);
  if (!hasMismatch) {
    toast.info(
      getIdentityGuardRemoteUrl()
        ? "Git identity looks aligned for this repository."
        : "Git Identity Guard is enabled, but this repository has no remote to compare.",
    );
  }
}

function isAppHidden(): boolean {
  return typeof document !== "undefined" && document.visibilityState === "hidden";
}

function isGitOperationBusyForBackground(): boolean {
  return autoFetchInFlight.value
    || activeRemoteAction.value !== null
    || git.loading.value
    || git.loadingMore.value;
}

function isBackgroundMaintenanceBusy(): boolean {
  return backgroundMaintenanceInFlight.value || isGitOperationBusyForBackground();
}

function hasTimedBackgroundMaintenanceEnabled(settings: BackgroundMaintenanceSettings): boolean {
  return settings.healthRefreshEnabled
    || settings.remoteHygieneEnabled
    || settings.staleWorkReminderEnabled
    || settings.behindBranchReminderEnabled
    || settings.largeChangeReminderEnabled
    || settings.conflictReminderEnabled
    || settings.commitDetailsPreloadEnabled;
}

function stopBackgroundMaintenanceTimer(): void {
  if (backgroundMaintenanceTimerId.value === null) {
    return;
  }

  clearInterval(backgroundMaintenanceTimerId.value);
  backgroundMaintenanceTimerId.value = null;
}

function restartBackgroundMaintenanceTimer(): void {
  stopBackgroundMaintenanceTimer();

  const settings = getStoredBackgroundMaintenanceSettings();
  if (!hasTimedBackgroundMaintenanceEnabled(settings)) {
    return;
  }

  backgroundMaintenanceTimerId.value = globalThis.setInterval(() => {
    void runBackgroundMaintenance("timer");
  }, settings.intervalMinutes * 60 * 1000);
}

function handleBackgroundMaintenanceSettingsChanged(): void {
  primeBackgroundMaintenanceBaselines();
  restartBackgroundMaintenanceTimer();
}

function handleVisibilityChanged(): void {
  if (!isAppHidden()) {
    void runBackgroundMaintenance("visible");
  }
}

function handleBackgroundUserActivity(): void {
  backgroundLastUserActivityAt = Date.now();
}

function primeBackgroundMaintenanceBaselines(): void {
  const now = Date.now();
  const settings = getStoredBackgroundMaintenanceSettings();
  const dirtySignature = dirtyWorkSignature();
  if (dirtySignature) {
    backgroundStaleWorkSignature = dirtySignature;
    backgroundStaleWorkStartedAt = now;
    backgroundLargeChangeSignature = `${settings.largeChangeThreshold}\u0000${dirtySignature}`;
    backgroundLargeChangeLastToastAt = now;
  }

  const currentBranch = git.branches.value.find((branch) => branch.is_head && !branch.is_remote);
  if (currentBranch && currentBranch.behind > 0) {
    backgroundBehindBranchSignature = behindBranchSignature(currentBranch);
    backgroundBehindBranchLastToastAt = now;
  }

  if (git.conflictFiles.value.length > 0) {
    backgroundConflictSignature = conflictSignature();
    backgroundConflictLastToastAt = now;
  }
}

async function runBackgroundMaintenance(reason: "timer" | "settings" | "visible" | "repo"): Promise<void> {
  const settings = getStoredBackgroundMaintenanceSettings();
  if (
    !hasBackgroundMaintenanceEnabled(settings)
    || (reason !== "visible" && !hasTimedBackgroundMaintenanceEnabled(settings))
    || !git.repoPath.value
    || isAppHidden()
    || isBackgroundMaintenanceBusy()
  ) {
    return;
  }

  if (settings.idleOnlyEnabled && Date.now() - backgroundLastUserActivityAt < BACKGROUND_IDLE_MIN_MS) {
    return;
  }

  backgroundMaintenanceInFlight.value = true;
  try {
    const remoteRan = await maybeRunBackgroundRemoteHygiene(settings, reason);
    const shouldRefreshLightweightState = settings.healthRefreshEnabled
      || (settings.focusSyncEnabled && reason === "visible")
      || settings.staleWorkReminderEnabled
      || settings.behindBranchReminderEnabled
      || settings.largeChangeReminderEnabled
      || settings.conflictReminderEnabled;

    if (shouldRefreshLightweightState) {
      await runBackgroundHealthRefresh(remoteRan, settings, reason);
    }

    const allowReminderToasts = reason === "timer" || reason === "visible";
    if (!allowReminderToasts) {
      primeBackgroundMaintenanceBaselines();
    }

    if (allowReminderToasts && settings.staleWorkReminderEnabled) {
      maybeShowStaleWorkReminder();
    }

    if (allowReminderToasts && settings.behindBranchReminderEnabled) {
      maybeShowBehindBranchReminder();
    }

    if (allowReminderToasts && settings.largeChangeReminderEnabled) {
      maybeShowLargeChangeReminder(settings.largeChangeThreshold);
    }

    if (allowReminderToasts && settings.conflictReminderEnabled) {
      maybeShowConflictReminder();
    }

    if (settings.commitDetailsPreloadEnabled) {
      await warmCommitDetailsCache();
    }
  } finally {
    backgroundMaintenanceInFlight.value = false;
  }
}

async function maybeRunBackgroundRemoteHygiene(
  settings: BackgroundMaintenanceSettings,
  reason: "timer" | "settings" | "visible" | "repo",
): Promise<boolean> {
  if (!settings.remoteHygieneEnabled || !git.repoPath.value) {
    return false;
  }

  const now = Date.now();
  const intervalMs = settings.intervalMinutes * 60 * 1000;
  const cooldownMs = reason === "visible" ? Math.min(intervalMs, BACKGROUND_FOCUS_REMOTE_COOLDOWN_MS) : intervalMs;
  if (backgroundRemoteHygieneLastRunAt > 0 && now - backgroundRemoteHygieneLastRunAt < cooldownMs) {
    return false;
  }

  try {
    await git.backgroundFetchAll();
    backgroundRemoteHygieneLastRunAt = now;
    return true;
  } catch {
    return false;
  }
}

async function runBackgroundHealthRefresh(
  remoteAlreadyRefreshed: boolean,
  settings: BackgroundMaintenanceSettings,
  reason: "timer" | "settings" | "visible" | "repo",
): Promise<void> {
  if (!git.repoPath.value) {
    return;
  }

  const focusSyncActive = settings.focusSyncEnabled && reason === "visible";
  const needsStatus = settings.healthRefreshEnabled
    || focusSyncActive
    || settings.staleWorkReminderEnabled
    || settings.largeChangeReminderEnabled
    || settings.conflictReminderEnabled;
  const needsBranches = settings.healthRefreshEnabled
    || focusSyncActive
    || settings.behindBranchReminderEnabled;
  const needsTags = settings.healthRefreshEnabled || focusSyncActive;
  const tasks: Promise<void>[] = [];

  if (needsStatus) {
    tasks.push(git.refreshStatus());
  }

  if (settings.healthRefreshEnabled) {
    tasks.push(git.refreshStashes());
  }

  if (!remoteAlreadyRefreshed && needsBranches) {
    tasks.push(git.refreshBranches());
  }

  if (!remoteAlreadyRefreshed && needsTags) {
    tasks.push(git.refreshTags());
  }

  if (tasks.length === 0) {
    return;
  }

  try {
    await Promise.all(tasks);
  } catch {
    // Individual refresh actions already capture errors on the git state.
  }
}

function dirtyWorkSignature(): string {
  if (!git.repoPath.value || git.fileStatuses.value.length === 0) {
    return "";
  }

  return buildStatusSignature(
    git.fileStatuses.value.map((file) => `${file.staged ? "S" : "W"}:${file.status}:${file.path}`),
  );
}

function buildStatusSignature(values: string[]): string {
  let hash = 2166136261;
  const sorted = [...values].sort();

  for (const value of sorted) {
    for (let index = 0; index < value.length; index += 1) {
      hash ^= value.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
  }

  return `${git.repoPath.value || ""}\u0000${sorted.length}\u0000${(hash >>> 0).toString(16)}`;
}

function maybeShowStaleWorkReminder(): void {
  const signature = dirtyWorkSignature();
  const now = Date.now();

  if (!signature) {
    backgroundStaleWorkSignature = "";
    backgroundStaleWorkStartedAt = 0;
    return;
  }

  if (signature !== backgroundStaleWorkSignature) {
    backgroundStaleWorkSignature = signature;
    backgroundStaleWorkStartedAt = now;
    return;
  }

  if (
    now - backgroundStaleWorkStartedAt < BACKGROUND_STALE_WORK_MIN_MS
    || now - backgroundStaleWorkLastToastAt < BACKGROUND_STALE_WORK_TOAST_COOLDOWN_MS
  ) {
    return;
  }

  backgroundStaleWorkLastToastAt = now;
  const stagedCount = git.stagedFiles.value.length;
  const unstagedCount = git.unstagedFiles.value.length;
  const conflictCount = git.conflictFiles.value.length;
  toast.action(
    "info",
    "Same uncommitted work has been sitting for 30 minutes.",
    [
      { label: "Review", style: "primary", onClick: onSelectWorkingChanges },
      {
        label: "Stash",
        style: "neutral",
        onClick: () => {
          stashMessage.value = `WIP ${new Date().toLocaleString()}`;
          showStashDialog.value = true;
        },
      },
      { label: "Mute", style: "neutral", onClick: () => muteBackgroundReminder({ staleWorkReminderEnabled: false }, "Stale work") },
    ],
    16000,
    `${stagedCount} staged, ${unstagedCount} unstaged, ${conflictCount} conflicts.`,
  );
}

function behindBranchSignature(branch: BranchInfo): string {
  return `${git.repoPath.value}\u0000${branch.name}\u0000${branch.upstream || ""}\u0000${branch.behind}`;
}

function conflictSignature(): string {
  return buildStatusSignature(git.conflictFiles.value.map((file) => `C:${file.status}:${file.path}`));
}

function muteBackgroundReminder(partial: Partial<BackgroundMaintenanceSettings>, label: string): void {
  updateBackgroundMaintenanceSettings(partial);
  toast.info(`${label} reminder muted.`);
}

function maybeShowBehindBranchReminder(): void {
  const currentBranch = git.branches.value.find((branch) => branch.is_head && !branch.is_remote);
  if (!git.repoPath.value || !currentBranch || currentBranch.behind <= 0) {
    backgroundBehindBranchSignature = "";
    return;
  }

  const now = Date.now();
  const signature = behindBranchSignature(currentBranch);
  if (signature === backgroundBehindBranchSignature && now - backgroundBehindBranchLastToastAt < BACKGROUND_BRANCH_REMINDER_COOLDOWN_MS) {
    return;
  }

  backgroundBehindBranchSignature = signature;
  backgroundBehindBranchLastToastAt = now;
  toast.action(
    "warning",
    `${currentBranch.name} is ${currentBranch.behind} commit${currentBranch.behind === 1 ? "" : "s"} behind.`,
    [
      { label: "Pull", style: "primary", onClick: () => void handlePull() },
      { label: "Fetch", style: "neutral", onClick: () => void handleFetch() },
      { label: "Mute", style: "neutral", onClick: () => muteBackgroundReminder({ behindBranchReminderEnabled: false }, "Behind branch") },
    ],
    15000,
    currentBranch.upstream ? `Upstream: ${currentBranch.upstream}` : "No upstream metadata was reported.",
  );
}

function maybeShowLargeChangeReminder(threshold: number): void {
  const changedCount = git.fileStatuses.value.length;
  if (!git.repoPath.value || changedCount < threshold) {
    backgroundLargeChangeSignature = "";
    return;
  }

  const now = Date.now();
  const signature = `${threshold}\u0000${dirtyWorkSignature()}`;
  if (signature === backgroundLargeChangeSignature && now - backgroundLargeChangeLastToastAt < BACKGROUND_LARGE_CHANGE_REMINDER_COOLDOWN_MS) {
    return;
  }

  backgroundLargeChangeSignature = signature;
  backgroundLargeChangeLastToastAt = now;
  toast.action(
    "warning",
    `${changedCount} changed files are waiting in this worktree.`,
    [
      { label: "Review", style: "primary", onClick: onSelectWorkingChanges },
      {
        label: "Stash",
        style: "neutral",
        onClick: () => {
          stashMessage.value = `WIP ${new Date().toLocaleString()}`;
          showStashDialog.value = true;
        },
      },
      { label: "Mute", style: "neutral", onClick: () => muteBackgroundReminder({ largeChangeReminderEnabled: false }, "Large worktree") },
    ],
    16000,
    `Current threshold: ${threshold} files.`,
  );
}

function maybeShowConflictReminder(): void {
  const conflicts = git.conflictFiles.value;
  if (!git.repoPath.value || conflicts.length === 0) {
    backgroundConflictSignature = "";
    return;
  }

  const now = Date.now();
  const signature = conflictSignature();
  if (signature === backgroundConflictSignature && now - backgroundConflictLastToastAt < BACKGROUND_CONFLICT_REMINDER_COOLDOWN_MS) {
    return;
  }

  backgroundConflictSignature = signature;
  backgroundConflictLastToastAt = now;
  toast.action(
    "warning",
    `${conflicts.length} unresolved conflict${conflicts.length === 1 ? "" : "s"} detected.`,
    [
      { label: "Resolve", style: "primary", onClick: onSelectConflicts },
      { label: "Review", style: "neutral", onClick: onSelectWorkingChanges },
      { label: "Mute", style: "neutral", onClick: () => muteBackgroundReminder({ conflictReminderEnabled: false }, "Conflict") },
    ],
    18000,
    conflicts.slice(0, 3).map((file) => file.path).join(", "),
  );
}

async function warmCommitDetailsCache(): Promise<void> {
  const repoPath = git.repoPath.value;
  if (!repoPath || git.commits.value.length === 0) {
    return;
  }

  const commitsToInspect = git.commits.value.slice(0, BACKGROUND_COMMIT_PRELOAD_LIMIT);
  const runKey = `${repoPath}\u0000${commitsToInspect.map((commit) => commit.sha).join(",")}`;
  if (runKey === backgroundCommitPreloadRunKey) {
    return;
  }

  backgroundCommitPreloadRunKey = runKey;
  for (const commit of commitsToInspect) {
    if (isAppHidden() || git.repoPath.value !== repoPath || isGitOperationBusyForBackground()) {
      return;
    }

    if (getCachedCommitFiles(repoPath, commit.sha)) {
      continue;
    }

    try {
      const files = await invoke<CommitFileInfo[]>("get_commit_files", { path: repoPath, sha: commit.sha });
      if (git.repoPath.value !== repoPath) {
        return;
      }
      cacheCommitFiles(repoPath, commit.sha, files);
    } catch {
      // Best-effort preloader; selected-commit loading still works normally on click.
    }
  }
}

async function handlePull() {
  appendLog("user", "Pull triggered.");
  activeRemoteAction.value = "pull";
  try {
    await git.pull();
    if (git.error.value) {
      appendLog("error", `Pull failed: ${git.error.value}`);
    } else {
      appendLog("app", "Pull completed.");
    }
    maybeShowAuthDialogFromGitError();
  } finally {
    activeRemoteAction.value = null;
  }
}

async function handleFetch() {
  appendLog("user", "Fetch triggered.");
  activeRemoteAction.value = "fetch";
  try {
    await git.fetchAll();
    if (git.error.value) {
      appendLog("error", `Fetch failed: ${git.error.value}`);
    } else {
      appendLog("app", "Fetch completed.");
    }
    maybeShowAuthDialogFromGitError();
  } finally {
    activeRemoteAction.value = null;
  }
}

function toggleTerminalPanel() {
  showTerminal.value = !showTerminal.value;
  appendLog("user", `Terminal ${showTerminal.value ? "opened" : "closed"}.`);
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
    git.selectedCommits.value = [];
    git.selectedCommitFiles.value = [];
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

async function openGistInBrowser(url: string) {
  if (!url) return;
  try {
    await openUrl(url);
  } catch (e) {
    toast.error("Failed to open Gist: " + String(e));
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
  scheduleLostFoundScan(0);
}

function scheduleLostFoundScan(delay = LOST_FOUND_SCAN_DELAY_MS): void {
  if (lostFoundScanTimer) {
    clearTimeout(lostFoundScanTimer);
    lostFoundScanTimer = null;
  }

  const repoPath = git.repoPath.value;
  if (!repoPath) {
    lostFoundScanSequence++;
    lostCommits.value = [];
    lostCommitsLoading.value = false;
    return;
  }

  lostFoundScanTimer = setTimeout(() => {
    void refreshLostFound(false);
  }, delay);
}

async function refreshLostFound(showToast = true): Promise<void> {
  const repoPath = git.repoPath.value;
  if (!repoPath) {
    lostCommits.value = [];
    return;
  }

  const sequence = ++lostFoundScanSequence;
  lostCommitsLoading.value = true;

  try {
    const items = await invoke<LostCommitInfo[]>("get_lost_commits", {
      path: repoPath,
      maxCount: LOST_FOUND_SCAN_LIMIT,
    });
    if (sequence !== lostFoundScanSequence || git.repoPath.value !== repoPath) return;

    lostCommits.value = items;
    if (items.length === 0 && historyViewMode.value === "lost-found") {
      setHistoryViewMode("graph");
    }
    if (showToast) {
      toast.info(items.length > 0
        ? `Found ${items.length} recoverable commit${items.length === 1 ? "" : "s"}.`
        : "No lost commits found.");
    }
  } catch (error) {
    if (sequence !== lostFoundScanSequence || git.repoPath.value !== repoPath) return;
    lostCommits.value = [];
    appendLog("error", `Lost & Found scan failed: ${String(error)}`);
    if (showToast) {
      toast.error(`Lost & Found scan failed: ${String(error)}`);
    }
  } finally {
    if (sequence === lostFoundScanSequence && git.repoPath.value === repoPath) {
      lostCommitsLoading.value = false;
    }
  }
}

function uniqueLostFoundBranchName(baseName: string): string {
  const cleanBase = (baseName || "rescue/lost-commit")
    .replace(/\/+/g, "/")
    .replace(/^\/+|\/+$/g, "")
    || "rescue/lost-commit";
  const existing = new Set(git.localBranches.value.map((branch: BranchInfo) => branch.name.toLowerCase()));
  if (!existing.has(cleanBase.toLowerCase())) {
    return cleanBase;
  }

  for (let index = 2; index < 1000; index++) {
    const candidate = `${cleanBase}-${index}`;
    if (!existing.has(candidate.toLowerCase())) {
      return candidate;
    }
  }

  return `${cleanBase}-${Date.now()}`;
}

async function rescueLostCommit(payload: { sha: string; branchName: string }): Promise<void> {
  if (!hasActiveRepositoryPath()) {
    toast.warning("Open a repository first.");
    return;
  }

  const branchName = uniqueLostFoundBranchName(payload.branchName);
  rescuingLostCommitSha.value = payload.sha;

  try {
    await invoke("create_branch", {
      path: git.repoPath.value,
      name: branchName,
      startPoint: payload.sha,
    });
    await Promise.all([
      git.refreshBranches(),
      git.refreshCommits(),
    ]);
    toast.success(`Recovered as ${branchName}`);
    await refreshLostFound(false);
  } catch (error) {
    toast.error(`Recovery failed: ${String(error)}`);
  } finally {
    if (rescuingLostCommitSha.value === payload.sha) {
      rescuingLostCommitSha.value = null;
    }
  }
}

function openCommandPalette() {
  showCommandPalette.value = true;
}

function closeCommandPalette() {
  showCommandPalette.value = false;
}

function openPickaxeSearch() {
  if (!hasActiveRepositoryPath()) {
    toast.warning("Open a repository before using Pickaxe Explorer.");
    return;
  }
  showPickaxeSearch.value = true;
}

function closePickaxeSearch() {
  showPickaxeSearch.value = false;
}

function pickaxeHitToCommit(hit: PickaxeCommitHit): CommitInfo {
  return {
    sha: hit.sha,
    short_sha: hit.shortSha,
    message: hit.subject,
    author_name: hit.author,
    author_email: "",
    committer_name: "",
    committer_email: "",
    timestamp: hit.timestamp || 0,
    time_ago: hit.date || "",
    parent_shas: [],
    refs: [],
  };
}

async function openPickaxeResult(payload: {
  sha: string;
  filePath: string;
  fileStatus: string;
  oldPath?: string | null;
  hit: PickaxeCommitHit;
}) {
  if (!hasActiveRepositoryPath()) {
    return;
  }

  let commit = git.commits.value.find((item: CommitInfo) => item.sha === payload.sha) || null;
  if (!commit) {
    await git.ensureCommitLoaded(payload.sha);
    commit = git.commits.value.find((item: CommitInfo) => item.sha === payload.sha) || null;
  }

  await onSelectCommit({ commit: commit || pickaxeHitToCommit(payload.hit), additive: false });
  setHistoryViewMode("graph");
  openDiffViewer(payload.filePath, payload.sha, false, {
    status: payload.fileStatus,
    oldPath: payload.oldPath || null,
  });
}

function branchExists(name: string): boolean {
  return git.localBranches.value.some((branch) => branch.name === name || branch.name.endsWith(`/${name}`));
}

async function checkoutBranchFromPalette(name: string) {
  if (!hasActiveRepositoryPath()) return;
  await git.checkoutBranch(name);
  if (git.error.value) {
    toast.error(git.error.value);
    return;
  }
  toast.success(`Checked out ${name}`);
}

function confirmDiscardUnstagedFromPalette() {
  const count = git.unstagedFiles.value.length;
  if (count === 0) {
    toast.info("No unstaged changes to discard.");
    return;
  }

  scheduleDestructiveAction({
    message: `Discard ${count} unstaged change${count === 1 ? "" : "s"} in 5 seconds.`,
    detail: "Staged changes stay staged. Click Undo to cancel.",
    run: async () => {
      await git.discardAll();
      if (git.error.value) {
        toast.error(git.error.value);
      } else {
        toast.success("Unstaged changes discarded");
      }
    },
  });
}

const commandPaletteActions = computed<CommandPaletteAction[]>(() => {
  const hasRepo = hasActiveRepositoryPath();
  const hasUnstaged = git.unstagedFiles.value.length > 0;
  const hasStaged = git.stagedFiles.value.length > 0;
  const hasChanges = hasUnstaged || hasStaged;

  return [
    {
      id: "stage-all",
      label: "Stage all",
      description: `${git.unstagedFiles.value.length} unstaged file${git.unstagedFiles.value.length === 1 ? "" : "s"}`,
      keywords: ["add", "git add", "changes"],
      disabled: !hasRepo || !hasUnstaged,
      tone: "success",
      run: () => git.stageAll(),
    },
    {
      id: "unstage-all",
      label: "Unstage all",
      description: `${git.stagedFiles.value.length} staged file${git.stagedFiles.value.length === 1 ? "" : "s"}`,
      keywords: ["restore staged", "reset", "changes"],
      disabled: !hasRepo || !hasStaged,
      tone: "warning",
      run: () => git.unstageAll(),
    },
    {
      id: "discard-unstaged",
      label: "Discard unstaged",
      description: "Discard only unstaged working-tree changes.",
      keywords: ["restore", "clean", "changes"],
      disabled: !hasRepo || !hasUnstaged,
      tone: "danger",
      run: confirmDiscardUnstagedFromPalette,
    },
    {
      id: "working-changes",
      label: "Show working changes",
      description: "Open the changes panel for current staged and unstaged files.",
      keywords: ["status", "files"],
      disabled: !hasRepo || !hasChanges,
      run: onSelectWorkingChanges,
    },
    {
      id: "pickaxe-search",
      label: "Open Pickaxe Explorer",
      description: "Search exact strings across Git history with git log -S.",
      shortcut: "Ctrl Shift F",
      keywords: ["smart search", "history", "string", "pickaxe", "archaeology"],
      disabled: !hasRepo,
      run: openPickaxeSearch,
    },
    {
      id: "refresh",
      label: "Refresh repository",
      description: "Reload status, branches and commit history.",
      shortcut: "Ctrl Shift R",
      keywords: ["reload", "status"],
      disabled: !hasRepo,
      run: refreshCurrentRepo,
    },
    {
      id: "fetch",
      label: "Fetch all remotes",
      description: "Update remote refs without changing local branches.",
      keywords: ["remote", "origin"],
      disabled: !hasRepo,
      run: handleFetch,
    },
    {
      id: "pull",
      label: "Pull current branch",
      description: "Pull latest changes for the current branch.",
      keywords: ["remote", "origin"],
      disabled: !hasRepo,
      run: handlePull,
    },
    {
      id: "push",
      label: "Push current branch",
      description: "Push local commits or set up a remote.",
      keywords: ["remote", "origin"],
      disabled: !hasRepo,
      run: handlePush,
    },
    {
      id: "checkout-main",
      label: "Checkout main",
      description: "Switch to main branch.",
      keywords: ["switch branch"],
      disabled: !hasRepo || !branchExists("main"),
      run: () => checkoutBranchFromPalette("main"),
    },
    {
      id: "checkout-master",
      label: "Checkout master",
      description: "Switch to master branch.",
      keywords: ["switch branch"],
      disabled: !hasRepo || !branchExists("master"),
      run: () => checkoutBranchFromPalette("master"),
    },
    {
      id: "checkout-develop",
      label: "Checkout develop",
      description: "Switch to develop branch.",
      keywords: ["switch branch dev"],
      disabled: !hasRepo || !branchExists("develop"),
      run: () => checkoutBranchFromPalette("develop"),
    },
    {
      id: "create-branch",
      label: "Create branch",
      description: "Open branch creation dialog.",
      keywords: ["new branch"],
      disabled: !hasRepo,
      run: handleCreateBranch,
    },
    {
      id: "stash",
      label: "Stash changes",
      description: "Save current working-tree changes into a stash.",
      keywords: ["save changes"],
      disabled: !hasRepo || !hasChanges,
      run: handleStash,
    },
    {
      id: "terminal",
      label: "Toggle terminal",
      description: "Show or hide the integrated terminal.",
      shortcut: "Ctrl `",
      keywords: ["shell"],
      run: toggleTerminalPanel,
    },
    {
      id: "open-repo",
      label: "Open repository",
      description: "Browse and open another local repository.",
      shortcut: "Ctrl O",
      keywords: ["folder"],
      run: browseAndOpen,
    },
    {
      id: "open-vscode",
      label: "Open in VS Code",
      description: "Open the current repository in Visual Studio Code.",
      keywords: ["editor"],
      disabled: !hasRepo,
      run: openRepoInVsCode,
    },
    {
      id: "open-explorer",
      label: "Open in folder explorer",
      description: "Open the current repository folder.",
      shortcut: "Alt O",
      keywords: ["files", "folder"],
      disabled: !hasRepo,
      run: openRepoInExplorer,
    },
    {
      id: "settings",
      label: "Open settings",
      description: "Open preferences and app options.",
      shortcut: "Ctrl ,",
      keywords: ["options", "preferences"],
      run: () => openOptions("preferences"),
    },
    {
      id: "git-settings",
      label: "Open Git integration",
      description: "Configure Git path and background auto-fetch.",
      shortcut: "Ctrl Shift K",
      keywords: ["settings", "fetch"],
      run: () => openOptions("git"),
    },
    {
      id: "integrations",
      label: "Open integrations",
      description: "Manage GitHub, GitLab, Bitbucket and Azure connections.",
      shortcut: "Ctrl Shift I",
      keywords: ["tokens", "remote"],
      run: () => openOptions("integrations"),
    },
    {
      id: "logs",
      label: "Toggle logs",
      description: "Show or hide app, user and error logs.",
      shortcut: "Ctrl Shift L",
      keywords: ["debug"],
      run: toggleLogsPanel,
    },
    {
      id: "view-graph",
      label: "Graph view",
      description: "Return to the standard commit graph.",
      shortcut: "Alt 1",
      disabled: !hasRepo,
      run: () => setHistoryViewMode("graph"),
    },
    {
      id: "view-galaxy",
      label: "Galaxy view",
      description: "Open the interactive commit galaxy.",
      shortcut: "Alt 2",
      disabled: !hasRepo,
      run: () => setHistoryViewMode("galaxy"),
    },
    {
      id: "view-city",
      label: "Repository City",
      description: "Explore files, hotspots and contributor activity as an interactive city.",
      shortcut: "Alt 7",
      disabled: !hasRepo,
      run: () => setHistoryViewMode("city"),
    },
    {
      id: "view-burnout",
      label: "Burnout Analytics",
      description: "Show contributor focus, after-hours rhythm and ownership pressure.",
      shortcut: "Alt 6",
      disabled: !hasRepo,
      run: () => setHistoryViewMode("burnout"),
    },
    ...(lostCommits.value.length > 0 ? [{
      id: "view-lost-found",
      label: "Lost & Found",
      description: "Inspect and rescue dangling commits that are no longer attached to a branch.",
      keywords: ["recover", "rescue", "dangling", "lost", "reset"],
      disabled: !hasRepo,
      run: () => setHistoryViewMode("lost-found"),
    }] : []),
  ];
});

function isAltOnlyShortcut(event: KeyboardEvent): boolean {
  return event.altKey && !event.ctrlKey && !event.shiftKey;
}

function handleHistoryViewShortcut(event: KeyboardEvent, key: string): boolean {
  if (!isAltOnlyShortcut(event)) {
    return false;
  }

  const viewByKey: Partial<Record<string, HistoryViewMode>> = {
    "1": "graph",
    "2": "galaxy",
    "3": "productivity",
    "4": "time-machine",
    "5": "conflict-heatmap",
    "6": "burnout",
    "7": "city",
  };

  const nextMode = viewByKey[key];
  if (!nextMode) {
    return false;
  }

  event.preventDefault();
  setHistoryViewMode(nextMode);
  return true;
}

function handleRepositoryZoomShortcut(event: KeyboardEvent, key: string): boolean {
  if (!event.ctrlKey || event.shiftKey) {
    return false;
  }

  if (key !== "+" && key !== "=" && key !== "-" && key !== "0") {
    return false;
  }

  event.preventDefault();

  if (key === "-") {
    applyGeneralFontSize(generalFontSize.value - 2);
    applyCommitNumeric(commitNumeric.value - 4);
    return true;
  }

  if (key === "+" || key === "=") {
    applyGeneralFontSize(generalFontSize.value + 2);
    applyCommitNumeric(commitNumeric.value + 4);
    return true;
  }

  applyGeneralFontSize(16);
  applyCommitNumeric(4);
  return true;
}

function handleRepositoryDialogShortcut(event: KeyboardEvent, key: string): boolean {
  if (event.ctrlKey && event.shiftKey && key === "o") {
    event.preventDefault();
    void openRepoInVsCode();
    return true;
  }

  if (event.ctrlKey && event.shiftKey && key === "g") {
    event.preventDefault();
    void createGistFromRepo();
    return true;
  }

  if (event.ctrlKey && event.shiftKey && key === "i") {
    event.preventDefault();
    openOptions("integrations");
    return true;
  }

  if (event.ctrlKey && event.shiftKey && key === "k") {
    event.preventDefault();
    openOptions("git");
    return true;
  }

  if (event.ctrlKey && event.shiftKey && key === "a") {
    event.preventDefault();
    openOptions("advanced");
    return true;
  }

  if (event.ctrlKey && event.shiftKey && key === "y") {
    event.preventDefault();
    openOptions("organisations");
    return true;
  }

  if (event.ctrlKey && event.shiftKey && key === "l") {
    event.preventDefault();
    toggleLogsPanel();
    return true;
  }

  if (event.ctrlKey && event.shiftKey && key === "r") {
    event.preventDefault();
    void refreshCurrentRepo();
    return true;
  }

  return false;
}

function handleRepositoryNavigationShortcut(event: KeyboardEvent, key: string): boolean {
  if (event.ctrlKey && !event.shiftKey && key === "o") {
    event.preventDefault();
    void browseAndOpen();
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

  if (event.ctrlKey && !event.shiftKey && key === ",") {
    event.preventDefault();
    openOptions("preferences");
    return true;
  }

  return false;
}

function handleRepositoryShortcut(event: KeyboardEvent, key: string): boolean {
  if (handleRepositoryZoomShortcut(event, key)) {
    return true;
  }

  if (handleRepositoryNavigationShortcut(event, key)) {
    return true;
  }

  if (handleRepositoryDialogShortcut(event, key)) {
    return true;
  }

  return false;
}

function handleGlobalShortcuts(event: KeyboardEvent) {
  handleBackgroundUserActivity();
  const key = event.key.toLowerCase();

  if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && key === "k") {
    event.preventDefault();
    openCommandPalette();
    return;
  }

  if ((event.ctrlKey || event.metaKey) && event.shiftKey && !event.altKey && key === "f") {
    event.preventDefault();
    openPickaxeSearch();
    return;
  }

  if (handleRepositoryTabShortcut(event, {
    newTab,
    closeActiveTab,
    reopenClosedTab,
    selectAdjacentTab,
    canCloseActiveTab: () => canCloseActiveTab.value,
    canReopenClosedTab: () => canReopenClosedTab.value,
  })) {
    return;
  }

  if (isEditableTarget(event.target)) {
    return;
  }

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

onMounted(() => {
  globalThis.addEventListener("keydown", handleGlobalShortcuts);
  globalThis.addEventListener("pointerdown", handleBackgroundUserActivity);
  globalThis.addEventListener("wheel", handleBackgroundUserActivity);
  globalThis.addEventListener(AUTO_FETCH_SETTINGS_EVENT, handleAutoFetchSettingsChanged as EventListener);
  globalThis.addEventListener(SMART_GITIGNORE_WIZARD_EVENT, handleSmartGitignoreWizardChanged);
  globalThis.addEventListener(BUG_AUTOPSY_EVENT, handleBugAutopsyChanged);
  globalThis.addEventListener(IDENTITY_GUARD_EVENT, handleIdentityGuardChanged);
  globalThis.addEventListener(BACKGROUND_MAINTENANCE_EVENT, handleBackgroundMaintenanceSettingsChanged as EventListener);
  if (typeof document !== "undefined") {
    document.addEventListener("visibilitychange", handleVisibilityChanged);
  }
  appendLog("app", "Application started.");

  const restoreSession = shouldRestoreSession();

  try {
    if (restoreSession) {
      const saved = safeStorageGet("gitswamp-tabs");
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.tabs?.length) {
          restoreTabs(parsed.tabs, parsed.activeTabId);
        }
      }
    }
  } catch {}

  if (restoreSession) {
    const active = tabs.value.find((t) => t.id === activeTabId.value);
    if (active?.path) {
      git.openRepository(active.path, { optimisticRepoInfo: active.repo, background: true });
    }
  }

  restartAutoFetchTimer();
  restartBackgroundMaintenanceTimer();

});

onUnmounted(() => {
  globalThis.removeEventListener("keydown", handleGlobalShortcuts);
  globalThis.removeEventListener("pointerdown", handleBackgroundUserActivity);
  globalThis.removeEventListener("wheel", handleBackgroundUserActivity);
  globalThis.removeEventListener(AUTO_FETCH_SETTINGS_EVENT, handleAutoFetchSettingsChanged as EventListener);
  globalThis.removeEventListener(SMART_GITIGNORE_WIZARD_EVENT, handleSmartGitignoreWizardChanged);
  globalThis.removeEventListener(BUG_AUTOPSY_EVENT, handleBugAutopsyChanged);
  globalThis.removeEventListener(IDENTITY_GUARD_EVENT, handleIdentityGuardChanged);
  globalThis.removeEventListener(BACKGROUND_MAINTENANCE_EVENT, handleBackgroundMaintenanceSettingsChanged as EventListener);
  if (typeof document !== "undefined") {
    document.removeEventListener("visibilitychange", handleVisibilityChanged);
  }
  commitFilesCache.clear();
  clearDiffViewerCaches();
  stopAutoFetchTimer();
  stopBackgroundMaintenanceTimer();
  pullRequestFetchSequence++;
  if (pullRequestFetchTimer) {
    clearTimeout(pullRequestFetchTimer);
    pullRequestFetchTimer = null;
  }
  gitRpgProfileSequence++;
  if (gitRpgProfileTimer) {
    clearTimeout(gitRpgProfileTimer);
    gitRpgProfileTimer = null;
  }
  lostFoundScanSequence++;
  if (lostFoundScanTimer) {
    clearTimeout(lostFoundScanTimer);
    lostFoundScanTimer = null;
  }

});

watch([tabs, activeTabId], () => {
  safeStorageSet(
    "gitswamp-tabs",
    JSON.stringify({ tabs: tabs.value, activeTabId: activeTabId.value })
  );
}, { deep: true });

// Collapse side panel and close diff when switching tabs
watch(activeTabId, () => {
  detailsPanelCollapsed.value = true;
  showDiffViewer.value = false;
  diffFallbackStatus.value = null;
  diffFallbackOldPath.value = null;
});

watch(
  () => [
    git.repoPath.value,
    git.commits.value[0]?.sha || "",
    Math.min(git.commits.value.length, GIT_RPG_COMMIT_SCAN_LIMIT),
  ],
  () => scheduleGitRpgProfileRefresh(),
  { immediate: true },
);

watch(
  () => [
    git.repoPath.value,
    git.repoInfo.value?.head_sha || "",
    git.localBranches.value.map((branch: BranchInfo) => `${branch.name}:${branch.is_head}:${branch.upstream || ""}`).join("|"),
    git.stashes.value.length,
  ],
  () => {
    if (git.repoPath.value) {
      scheduleLostFoundScan(LOST_FOUND_SCAN_DELAY_MS);
    }
  },
);

watch(() => git.error.value, (value) => {
  if (!value) return;
  appendLog("error", value);
});

watch(
  () => git.terminalOutput.value.length,
  (length, previousLength) => {
    if (length <= 0) return;
    const start = Math.max(previousLength ?? 0, 0);
    const recent = git.terminalOutput.value.slice(start);
    for (const row of recent) {
      appendLog("app", `Terminal: ${row}`);
    }
  },
);

watch(
  () => git.repoPath.value,
  (repoPath, previousRepoPath) => {
    restartAutoFetchTimer();
    restartBackgroundMaintenanceTimer();
    if (repoPath !== previousRepoPath) {
      identityGuardMismatch.value = null;
      lostCommits.value = [];
      lostCommitsLoading.value = false;
      scheduleLostFoundScan();
      clearCommitFilesCacheForOtherRepos(repoPath);
      clearDiffViewerCaches();
      backgroundCommitPreloadRunKey = "";
      backgroundRemoteHygieneLastRunAt = 0;
      backgroundStaleWorkSignature = "";
      backgroundStaleWorkStartedAt = 0;
      backgroundBehindBranchSignature = "";
      backgroundLargeChangeSignature = "";
      backgroundConflictSignature = "";
      void runBackgroundMaintenance("repo");
    }
  },
);

watch(
  () => [
    git.repoPath.value,
    git.repoInfo.value?.path || "",
    (git.repoInfo.value?.remotes || [])
      .map((remote) => `${remote.name}:${remote.url}`)
      .join("|"),
  ],
  () => {
    if (identityGuardEnabled.value) {
      void checkGitIdentityGuard("repo");
    }
  },
);

watch(
  () => [
    git.repoInfo.value?.path || "",
    (git.repoInfo.value?.remotes || [])
      .map((remote) => `${remote.name}:${remote.provider}:${remote.url}`)
      .join("|"),
    git.providerTokens.value.github || git.githubToken.value || "",
    git.providerTokens.value.gitlab || "",
    git.providerTokens.value["gitlab-self"] || "",
    git.providerTokens.value.bitbucket || "",
    git.providerTokens.value.azure || "",
    git.providerTokens.value["azure-domain"] || "",
  ],
  () => {
    scheduleOpenPullRequestRefresh();
  },
  { immediate: true },
);

watch(() => git.selectedCommit.value, (commit) => {
  if (git.selectedCommits.value.length > 1) {
    return;
  }

  if (commit) {
    if (selectedCommitWatcherSkipSha === commit.sha) {
      selectedCommitWatcherSkipSha = null;
      return;
    }
    viewingWorkingChanges.value = false;
    git.selectedCommits.value = [commit];
    git.selectedCommitFiles.value = [];
    void loadSingleSelectedCommitFiles(commit);
  } else if (git.selectedCommits.value.length <= 1) {
    selectedCommitWatcherSkipSha = null;
    git.selectedCommits.value = [];
  }
});

const selectedIssue = computed(() =>
  githubIssueDetail.value?.number === selectedIssueNumber.value
    ? githubIssueDetail.value
    : githubIssues.value.find((item) => item.number === selectedIssueNumber.value) || null,
);

const selectedPullRequest = computed(() =>
  githubPullRequestDetail.value?.number === selectedPullRequestNumber.value
    ? githubPullRequestDetail.value
    : githubPullRequests.value.find((item) => item.number === selectedPullRequestNumber.value) || null,
);

const identityGuardHeaderStatus = computed(() => {
  if (!identityGuardEnabled.value || !git.repoPath.value) {
    return null;
  }

  if (identityGuardMismatch.value) {
    return {
      enabled: true,
      mismatch: true,
      label: "Identity mismatch",
      detail: [
        "Git Identity Guard",
        `Current: ${identityGuardMismatch.value.currentEmail || "not configured"}`,
        identityGuardMismatch.value.suggestedEmail ? `Suggested: ${identityGuardMismatch.value.suggestedEmail}` : null,
        `Remote: ${shortenIdentityRemoteLabel(identityGuardMismatch.value.remoteUrl)}`,
      ].filter(Boolean).join("\n"),
    };
  }

  return {
    enabled: true,
    mismatch: false,
    label: "Identity OK",
    detail: "Git Identity Guard is watching this repository.",
  };
});

watch(
  [historyViewMode, remoteInsightsMode, selectedIssueNumber, selectedPullRequestNumber],
  ([viewMode, mode]) => {
    if (viewMode !== "remote-insights") {
      clearRemoteInsightDetails();
      clearRemoteCreateOptions();
      return;
    }

    if (mode !== "issue-create" && mode !== "pull-request-create") {
      clearRemoteCreateOptions();
    }

    loadRemoteInsightDetailForCurrentSelection();
  },
);

async function openRepo(path: string) {
  try {
    await git.openRepository(path);
    if (git.repoInfo.value) {
      const repo = git.repoInfo.value;
      appendLog("user", `Opened repository: ${repo.path}`);
      setActiveTabRepository(repo);
      addToRecent(repo);
      git.selectedCommits.value = [];
      git.selectedCommitFiles.value = [];
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

function formatCloneProgressDetail(payload: CloneProgressEventPayload): string {
  if (payload.total_objects > 0) {
    const completedObjects = Math.max(payload.received_objects, payload.indexed_objects);
    const transferredMb = (payload.received_bytes / (1024 * 1024)).toFixed(1);
    return `${payload.phase}: ${completedObjects}/${payload.total_objects} objects | ${transferredMb} MB`;
  }

  return payload.message || "Preparing clone...";
}

async function handleClone(
  url: string,
  path: string,
  shallow: boolean,
  folderName?: string | null,
  done?: (ok: boolean, error?: string) => void,
) {
  const progressToastId = toast.progress("Cloning repository...", 0, "Preparing clone...");
  const normalizedUrl = url.trim().toLowerCase();
  let unlistenCloneProgress: UnlistenFn | null = null;

  try {
    unlistenCloneProgress = await listen<CloneProgressEventPayload>("clone-progress", (event) => {
      const payload = event.payload;
      if (!payload || payload.url.trim().toLowerCase() !== normalizedUrl) {
        return;
      }

      toast.update(progressToastId, {
        message: "Cloning repository...",
        detail: formatCloneProgressDetail(payload),
        progress: payload.percent,
      });
    });

    const clonedPath = await git.cloneRepo(url, path, shallow, undefined, folderName);
    if (!clonedPath) {
      toast.remove(progressToastId);
      done?.(false, git.error.value || "Clone failed.");
      toast.error("Clone failed: " + (git.error.value || "Unknown error"));
      return;
    }
    toast.update(progressToastId, {
      message: "Cloning repository...",
      detail: "Clone completed.",
      progress: 100,
    });
    setTimeout(() => {
      toast.remove(progressToastId);
    }, 1100);
    showCloneDialog.value = false;
    await openRepo(clonedPath);
    done?.(true);
    toast.success("Repository cloned successfully");
  } catch (e) {
    toast.remove(progressToastId);
    done?.(false, String(e));
    toast.error("Clone failed: " + String(e));
  } finally {
    if (unlistenCloneProgress) {
      unlistenCloneProgress();
    }
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
  appendLog("user", "Push triggered.");
  // First, check if origin exists
  const hasOrigin = await git.checkOriginExists();
  if (hasOrigin) {
    // Origin exists, push normally
    activeRemoteAction.value = "push";
    try {
      await git.push();
      if (git.error.value) {
        appendLog("error", `Push failed: ${git.error.value}`);
      } else {
        appendLog("app", "Push completed.");
      }
      maybeShowAuthDialogFromGitError();
    } finally {
      activeRemoteAction.value = null;
    }
  } else {
    appendLog("app", "Push requires remote setup first.");
    // No origin, show dialog to select platform
    const repoName = git.repoInfo.value?.name || "repository";
    multiPlatformPushRepoName.value = repoName;
    showMultiPlatformPushDialog.value = true;
  }
}

function handleMultiPlatformPush(platform: string) {
  // For hosted platforms, ask for owner/workspace first. Azure additionally needs host domain.
  if (platform === 'github' || platform === 'gitlab' || platform === 'github-enterprise' || platform === 'gitlab-self-hosted' || platform === 'gitlab-self' || platform === 'bitbucket' || platform === 'azure') {
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
  const needsDomain = platform === "gitlab-self-hosted" || platform === "gitlab-self" || platform === "github-enterprise" || platform === "azure";
  
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
    } else if (authProvider.value === "bitbucket") {
      await git.saveProviderToken("bitbucket", token);
    } else if (authProvider.value === "azure") {
      const domainRaw = authDomainInput.value;
      const domainClean = normalizeAzureDomainInput(domainRaw);
      if (!domainClean) {
        toast.error("Host domain is required for Azure DevOps");
        return;
      }
      authDomainInput.value = domainClean;
      await git.saveProviderToken("azure", token);
      await git.saveProviderToken("azure-domain", domainClean);
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

function normalizeCommitSelectionPayload(payload: CommitSelectionPayload): { commit: CommitInfo | null; additive: boolean } {
  if (payload && typeof payload === "object" && "commit" in payload) {
    return {
      commit: payload.commit,
      additive: !!payload.additive,
    };
  }

  return {
    commit: (payload as CommitInfo | null) || null,
    additive: false,
  };
}

function mergeCommitFileStatus(currentStatus: string, nextStatus: string): string {
  const priority: Record<string, number> = {
    conflicted: 6,
    deleted: 5,
    renamed: 4,
    modified: 3,
    added: 2,
    new: 2,
  };

  const current = priority[currentStatus] || 1;
  const next = priority[nextStatus] || 1;
  return next >= current ? nextStatus : currentStatus;
}

async function getCommitFiles(repoPath: string, sha: string): Promise<CommitFileInfo[]> {
  const cached = getCachedCommitFiles(repoPath, sha);
  if (cached) return cached;

  try {
    const files = await invoke<CommitFileInfo[]>("get_commit_files", { path: repoPath, sha });
    cacheCommitFiles(repoPath, sha, files);
    return files;
  } catch {
    return [];
  }
}

async function loadAggregatedSelectedCommitFiles(commits: CommitInfo[]) {
  const repoPath = git.repoPath.value;
  if (!repoPath || commits.length <= 1) {
    return;
  }

  const runToken = ++multiCommitFilesRunToken;
  const sorted = [...commits].sort((a, b) => b.timestamp - a.timestamp);

  const responses = await Promise.all(
    sorted.map(async (commit) => {
      const files = await getCommitFiles(repoPath, commit.sha);
      return { sha: commit.sha, files };
    }),
  );

  if (runToken !== multiCommitFilesRunToken || git.repoPath.value !== repoPath) {
    return;
  }

  const byPath = new Map<string, CommitFileInfo>();
  for (const response of responses) {
    for (const file of response.files) {
      const existing = byPath.get(file.path);
      if (!existing) {
        byPath.set(file.path, {
          ...file,
          commit_shas: [response.sha],
        });
        continue;
      }

      existing.additions += file.additions;
      existing.deletions += file.deletions;
      existing.status = mergeCommitFileStatus(existing.status, file.status);

      const hashes = existing.commit_shas || [];
      if (!hashes.includes(response.sha)) {
        hashes.push(response.sha);
      }
      existing.commit_shas = hashes;
    }
  }

  git.selectedCommitFiles.value = Array.from(byPath.values()).sort((a, b) => a.path.localeCompare(b.path));
}

async function loadSingleSelectedCommitFiles(commit: CommitInfo) {
  const repoPath = git.repoPath.value;
  if (!repoPath) {
    git.selectedCommitFiles.value = [];
    return;
  }

  const cacheKey = getCommitFilesCacheKey(repoPath, commit.sha);
  const cached = getCachedCommitFiles(repoPath, commit.sha);
  if (cached) {
    const stillSelected = git.selectedCommits.value.length === 1 && git.selectedCommits.value[0]?.sha === commit.sha;
    if (stillSelected) {
      git.selectedCommitFiles.value = cached;
    }
    return;
  }

  if (singleCommitLoadKey === cacheKey) {
    return;
  }

  singleCommitLoadKey = cacheKey;
  const runToken = ++multiCommitFilesRunToken;

  try {
    const files = await getCommitFiles(repoPath, commit.sha);

    if (runToken !== multiCommitFilesRunToken || git.repoPath.value !== repoPath) {
      return;
    }

    const stillSelected = git.selectedCommits.value.length === 1 && git.selectedCommits.value[0]?.sha === commit.sha;
    if (!stillSelected) {
      return;
    }

    git.selectedCommitFiles.value = files;
  } finally {
    if (singleCommitLoadKey === cacheKey) {
      singleCommitLoadKey = null;
    }
  }
}

async function onSelectCommit(payload: CommitSelectionPayload) {
  const { commit, additive } = normalizeCommitSelectionPayload(payload);
  viewingWorkingChanges.value = false;
  viewingStash.value = false;

  let nextSelection: CommitInfo[] = [];
  if (additive && commit) {
    const alreadySelected = git.selectedCommits.value.some((item) => item.sha === commit.sha);
    if (alreadySelected) {
      nextSelection = git.selectedCommits.value.filter((item) => item.sha !== commit.sha);
    } else {
      nextSelection = [...git.selectedCommits.value, commit];
    }
  } else if (commit) {
    nextSelection = [commit];
  }

  git.selectedCommits.value = nextSelection;
  const watcherSkipSha = nextSelection.length === 1 ? nextSelection[0].sha : null;
  selectedCommitWatcherSkipSha = watcherSkipSha;
  git.selectedCommit.value = nextSelection.length === 1 ? nextSelection[0] : null;
  if (watcherSkipSha) {
    queueMicrotask(() => {
      if (selectedCommitWatcherSkipSha === watcherSkipSha) {
        selectedCommitWatcherSkipSha = null;
      }
    });
  }
  git.clearStashSelection();
  detailsPanelCollapsed.value = false;

  if (nextSelection.length === 0) {
    multiCommitFilesRunToken += 1;
    git.selectedCommitFiles.value = [];
  } else if (nextSelection.length === 1) {
    multiCommitFilesRunToken += 1;
    const repoPath = git.repoPath.value;
    const cached = repoPath ? getCachedCommitFiles(repoPath, nextSelection[0].sha) : null;
    git.selectedCommitFiles.value = cached || [];
    if (!cached) {
      void loadSingleSelectedCommitFiles(nextSelection[0]);
    }
  } else {
    multiCommitFilesRunToken += 1;
    git.selectedCommitFiles.value = [];
    void loadAggregatedSelectedCommitFiles(nextSelection);
  }
}

function onSelectWorkingChanges() {
  selectWorkingChangesState();
}

function onSelectConflicts() {
  selectWorkingChangesState();
}

function selectWorkingChangesState() {
  multiCommitFilesRunToken += 1;
  viewingWorkingChanges.value = true;
  viewingStash.value = false;
  git.selectedCommit.value = null;
  git.selectedCommits.value = [];
  git.selectedCommitFiles.value = [];
  git.clearStashSelection();
  detailsPanelCollapsed.value = false;
}

function onSelectStash(stash: StashInfo) {
  multiCommitFilesRunToken += 1;
  viewingWorkingChanges.value = false;
  viewingStash.value = true;
  git.selectedCommit.value = null;
  git.selectedCommits.value = [];
  git.selectedCommitFiles.value = [];
  git.selectStash(stash);
  detailsPanelCollapsed.value = false;
}

interface MergeReleaseNotesRequest {
  source: string;
  sourceRemote: boolean;
  target: string;
  beforeTargetSha: string;
  afterTargetSha: string;
}

function mergeSourceRef(payload: { source: string; sourceRemote: boolean }): string {
  return payload.sourceRemote ? `origin/${payload.source}` : payload.source;
}

async function getGitRefSha(ref: string): Promise<string | null> {
  if (!git.repoPath.value || !ref.trim()) return null;

  try {
    const result = await invoke<string>("run_git_command", {
      path: git.repoPath.value,
      args: ["rev-parse", "--verify", ref],
    });
    return result.trim().split(/\s+/)[0] || null;
  } catch {
    return null;
  }
}

async function buildMergeReleaseNotes(request: MergeReleaseNotesRequest): Promise<string> {
  if (!git.repoPath.value) {
    throw new Error("No active repository.");
  }

  const range = `${request.beforeTargetSha}..${request.afterTargetSha}`;
  const logOutput = await invoke<string>("run_git_command", {
    path: git.repoPath.value,
    args: [
      "log",
      "--no-merges",
      "--date=short",
      `--format=${RELEASE_NOTES_LOG_FORMAT}`,
      range,
    ],
  });

  const commits = parseReleaseNotesLog(logOutput);
  return buildReleaseNotesMarkdown(commits, {
    sourceRef: mergeSourceRef(request),
    targetRef: request.target,
    fromSha: request.beforeTargetSha,
    toSha: request.afterTargetSha,
    generatedAt: new Date(),
  });
}

async function downloadMergeReleaseNotes(request: MergeReleaseNotesRequest): Promise<void> {
  try {
    const content = await buildMergeReleaseNotes(request);
    const defaultPath = defaultReleaseNotesFileName(mergeSourceRef(request), request.target);
    const selectedPath = await saveDialog({
      defaultPath,
      filters: [
        {
          name: "Markdown",
          extensions: ["md"],
        },
      ],
    });

    if (!selectedPath) {
      return;
    }

    await invoke("write_text_file", {
      path: selectedPath,
      content,
    });
    toast.success("Release notes saved.");
  } catch (error) {
    toast.error(`Could not save release notes: ${String(error)}`);
  }
}

function offerMergeReleaseNotes(request: MergeReleaseNotesRequest): void {
  toast.action(
    "success",
    "Merge completed. Download release notes?",
    [
      {
        label: "Download",
        style: "primary",
        onClick: () => downloadMergeReleaseNotes(request),
      },
      {
        label: "Skip",
        style: "neutral",
        onClick: () => {},
      },
    ],
    22000,
    "GitSwamp will generate a local Markdown summary from the commits merged into the target branch.",
  );
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
        onClick: async () => {
          const beforeTargetSha = await getGitRefSha(payload.target);
          const merged = await git.mergeBranchIntoCurrent(payload.source, payload.sourceRemote, payload.target);
          const afterTargetSha = merged ? await getGitRefSha("HEAD") : null;
          if (merged && beforeTargetSha && afterTargetSha && beforeTargetSha !== afterTargetSha) {
            offerMergeReleaseNotes({ ...payload, beforeTargetSha, afterTargetSha });
          }
        },
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

function handleRequestRebase(payload: { source: string; sourceRemote: boolean; target: string }) {
  if (!payload.source || !payload.target || payload.source === payload.target) return;
  toast.action(
    "warning",
    `Rebase ${payload.sourceRemote ? "origin/" + payload.source : payload.source} onto ${payload.target}?`,
    [
      {
        label: "Rebase",
        style: "primary",
        onClick: () => {
          void startRequestedRebase(payload);
        },
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

function openRebaseConflictDialog(source: string, target: string) {
  rebaseConflictSource.value = source;
  rebaseConflictTarget.value = target;
  showRebaseConflictDialog.value = true;
}

function closeRebaseConflictDialog(force = false) {
  if (!force && rebaseConflictBusy.value) {
    return;
  }

  showRebaseConflictDialog.value = false;
  rebaseConflictSource.value = "";
  rebaseConflictTarget.value = "";
}

async function startRequestedRebase(payload: { source: string; sourceRemote: boolean; target: string }) {
  const sourceLabel = payload.sourceRemote ? `origin/${payload.source}` : payload.source;
  const status = await git.rebaseBranchOnto(payload.source, payload.sourceRemote, payload.target);
  if (status === "conflict") {
    openRebaseConflictDialog(sourceLabel, payload.target);
    return;
  }

  if (status === "ok") {
    closeRebaseConflictDialog(true);
  }
}

async function handleRebaseConflictContinue() {
  if (rebaseConflictBusy.value) {
    return;
  }

  rebaseConflictBusy.value = true;
  try {
    const status = await git.rebaseContinue();
    if (status === "ok") {
      closeRebaseConflictDialog(true);
    }
  } finally {
    rebaseConflictBusy.value = false;
  }
}

async function handleRebaseConflictSkip() {
  if (rebaseConflictBusy.value) {
    return;
  }

  rebaseConflictBusy.value = true;
  try {
    const status = await git.rebaseSkip();
    if (status === "ok") {
      closeRebaseConflictDialog(true);
    }
  } finally {
    rebaseConflictBusy.value = false;
  }
}

async function handleRebaseConflictAbort() {
  if (rebaseConflictBusy.value) {
    return;
  }

  rebaseConflictBusy.value = true;
  try {
    const status = await git.rebaseAbort();
    if (status === "ok") {
      closeRebaseConflictDialog(true);
    }
  } finally {
    rebaseConflictBusy.value = false;
  }
}

async function handleCheckoutRemoteBranch(name: string) {
  const localBranch = git.localBranches.value.find(b => b.name === name);
  if (localBranch) {
    scheduleDestructiveAction({
      message: `Reset "${name}" to remote in 5 seconds.`,
      detail: "Click Undo to keep the current local branch state.",
      run: () => git.resetBranchToRemote(name),
    });
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
  scheduleDestructiveAction({
    message: "Discard active Ghost Branch experiment in 5 seconds.",
    detail: "Click Undo to keep the current Ghost Branch state.",
    run: () => git.discardGhostBranch(),
  });
}

function handleTimeMachineBlame(sha: string) {
  timeMachineFocusSha.value = sha;
  setHistoryViewMode("time-machine");
}

async function submitCreateBranch(name: string) {
  await submitCreateBranchFromDialog(name);
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
  showOptions.value ||
  showEditMessageDialog.value ||
  showRenameDialog.value ||
  showAnnotatedTagDialog.value ||
  showTagDialog.value ||
  showRebaseConflictDialog.value ||
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
  safeStorageSet("gitswamp-terminal-allow-all", String(value));
});

function handleCreateBranchAtCommit(sha: string) {
  branchAtSha.value = sha;
  newBranchName.value = "";
  showBranchDialog.value = true;
}

async function submitCreateBranchFromDialog(name: string) {
  const startPoint = branchAtSha.value || undefined;
  const created = await git.createBranch(name, startPoint);
  if (!created) {
    return;
  }

  branchAtSha.value = "";
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

</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-[var(--background)] overflow-hidden overscroll-none">
    <TitleBar />
    <RepositoryTabs
      :tabs="tabs"
      :active-tab-id="activeTabId"
      :can-reopen-closed-tab="canReopenClosedTab"
      @select-tab="selectTab"
      @close-tab="closeTab"
      @new-tab="newTab"
      @reopen-closed-tab="reopenClosedTab"
      @open-repository="browseAndOpen"
      @toggle-terminal="toggleTerminalPanel"
      @set-history-view="setHistoryViewMode($event)"
      @open-settings="openOptions('preferences')"
      @open-integrations="openOptions('integrations')"
      @open-git-integration="openOptions('git')"
      @open-advanced="openOptions('advanced')"
      @open-organisations="openOptions('organisations')"
      @refresh-repository="refreshCurrentRepo()"
      @open-in-vs-code="openRepoInVsCode()"
      @open-in-explorer="openRepoInExplorer()"
      @create-gist="createGistFromRepo()"
      @open-logs="toggleLogsPanel()"
    />

    <CommandPalette
      :visible="showCommandPalette"
      :actions="commandPaletteActions"
      @close="closeCommandPalette"
    />

    <PickaxeSearchPanel
      v-if="showPickaxeSearch && git.repoPath.value"
      :visible="showPickaxeSearch"
      :repo-path="git.repoPath.value"
      @close="closePickaxeSearch"
      @open-result="openPickaxeResult"
    />

    <template v-if="isLanding">
      <div class="flex-1 min-h-0 flex overflow-hidden">
        <LandingPage
          :open-repos="openReposList"
          :recent-repos="recentRepos"
          :github-token="git.providerTokens.value.github || git.githubToken.value || ''"
          @open="openRepo"
          @browse="browseAndOpen"
          @clone="showCloneDialog = true"
          @init="showInitDialog = true"
          @settings="openOptions('preferences')"
          @logs="toggleLogsPanel()"
          @remove-recent="removeRecent"
          @clear-recent="clearRecent"
        />

        <div
          v-if="showLogsPanel"
          class="h-full w-[360px] max-w-[42vw] flex-shrink-0"
        >
          <LogsPanel
            :app-logs="appLogs"
            :user-logs="userLogs"
            :error-logs="errorLogs"
            @close="showLogsPanel = false"
          />
        </div>
      </div>
    </template>

    <template v-else-if="git.repoInfo.value">
      <AppHeader
        :loading="git.loading.value"
        :active-action="activeRemoteAction"
        :ghost-active="git.ghostBranchState.value.active"
        :origin-conflict-risk="originConflictRisk"
        :rpg-profile="gitRpgProfile"
        :rpg-loading="gitRpgLoading"
        :identity-guard="identityGuardHeaderStatus"
        @pull="handlePull"
        @push="handlePush"
        @fetch="handleFetch"
        @branch="handleCreateBranch"
        @ghost-branch="handleStartGhostBranch"
        @materialize-ghost-branch="handleOpenGhostMaterializeDialog"
        @discard-ghost-branch="handleDiscardGhostBranch"
        @explain-git-state="explainGitState"
        @identity-guard="explainIdentityGuardState"
        @stash="handleStash"
        @terminal="toggleTerminalPanel"
        @settings="openOptions('preferences')"
      />
      <RepositoryWorkspace
        :git="git"
        :show-terminal="showTerminal"
        :terminal-allow-all="terminalAllowAll"
        :open-pull-request-branches="openPullRequestBranches"
        :issues="githubIssues"
        :pull-requests="githubPullRequests"
        :gists="githubGists"
        :lost-commits="lostCommits"
        :lost-commits-loading="lostCommitsLoading"
        :rescuing-lost-commit-sha="rescuingLostCommitSha"
        :issues-has-more="githubIssuesHasMore"
        :pull-requests-has-more="githubPullRequestsHasMore"
        :issues-loading-all="githubIssuesLoadingAll"
        :pull-requests-loading-all="githubPullRequestsLoadingAll"
        :selected-issue="selectedIssue"
        :selected-pull-request="selectedPullRequest"
        :remote-insight-detail-loading="remoteInsightDetailLoading"
        :remote-create-options="remoteCreateOptions"
        :remote-create-options-loading="remoteCreateOptionsLoading"
        :remote-insights-mode="remoteInsightsMode"
        :show-diff-viewer="showDiffViewer"
        :diff-file-path="diffFilePath"
        :diff-commit-sha="diffCommitSha"
        :diff-staged="diffStaged"
        :diff-fallback-status="diffFallbackStatus"
        :diff-fallback-old-path="diffFallbackOldPath"
        :conflict-resolver-path="conflictResolverPath"
        :details-panel-collapsed="detailsPanelCollapsed"
        :history-view-mode="historyViewMode"
        :time-machine-focus-sha="timeMachineFocusSha"
        :viewing-working-changes="viewingWorkingChanges"
        :viewing-stash="viewingStash"
        :show-logs-panel="showLogsPanel"
        :app-logs="appLogs"
        :user-logs="userLogs"
        :error-logs="errorLogs"
        :smart-gitignore-wizard-enabled="smartGitignoreWizardEnabled"
        :bug-autopsy-enabled="bugAutopsyEnabled"
        @set-history-view="setHistoryViewMode($event)"
        @update:show-terminal="showTerminal = $event"
        @update:terminal-allow-all="terminalAllowAll = $event"
        @update:details-panel-collapsed="detailsPanelCollapsed = $event"
        @update:show-logs-panel="showLogsPanel = $event"
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
        @load-all-issues="loadAllGithubIssues"
        @load-all-pull-requests="loadAllGithubPullRequests"
        @open-create-issue="openCreateIssuePanel"
        @open-create-pull-request="openCreatePullRequestPanel"
        @open-gist="openGistInBrowser($event)"
        @create-issue="createRemoteIssue($event)"
        @create-pull-request="createRemotePullRequest($event)"
        @refresh-lost-found="refreshLostFound(true)"
        @rescue-lost-commit="rescueLostCommit($event)"
        @request-merge="handleRequestMerge($event)"
        @request-rebase="handleRequestRebase($event)"
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
      :existing-branch-names="git.localBranches.value.map((branch) => branch.name)"
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
      :show-rebase-conflict-dialog="showRebaseConflictDialog"
      :rebase-conflict-source="rebaseConflictSource"
      :rebase-conflict-target="rebaseConflictTarget"
      :rebase-conflict-busy="rebaseConflictBusy"
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
      @close:rebase-conflict="closeRebaseConflictDialog()"
      @submit:rebase-continue="handleRebaseConflictContinue()"
      @submit:rebase-skip="handleRebaseConflictSkip()"
      @submit:rebase-abort="handleRebaseConflictAbort()"
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
    <OptionsDialog
      v-if="optionsMounted"
      v-show="showOptions"
      :git-path="git.gitPath.value"
      :initial-section="optionsInitialSection"
      @close="showOptions = false"
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

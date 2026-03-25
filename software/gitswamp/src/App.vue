<script setup lang="ts">
import TitleBar from "@/components/layout/TitleBar.vue";
import AppHeader from "@/components/layout/AppHeader.vue";
import RepositoryTabs from "@/components/repository/RepositoryTabs.vue";
import Sidebar from "@/components/repository/Sidebar.vue";
import CommitGraph from "@/components/commits/CommitGraph.vue";
import CommitDetails from "@/components/commits/CommitDetails.vue";
import FileDiffViewer from "@/components/ui/FileDiffViewer.vue";
import ConflictResolver from "@/components/ui/ConflictResolver.vue";
import LandingPage from "@/components/repository/LandingPage.vue";
import CloneDialog from "@/components/repository/CloneDialog.vue";
import InitDialog from "@/components/repository/InitDialog.vue";
import MultiPlatformPushDialog from "@/components/repository/MultiPlatformPushDialog.vue";
import TerminalPanel from "@/components/layout/TerminalPanel.vue";
import SettingsDialog from "@/components/layout/SettingsDialog.vue";
import ToastContainer from "@/components/ui/ToastContainer.vue";
import { useGit } from "@/composables/useGit";
import { useToast } from "@/composables/useToast";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

import { ref, watch, onMounted, computed } from "vue";
import type { RepoInfo, CommitInfo, StashInfo } from "@/types";

const git = useGit();
const toast = useToast();

const savedTheme = localStorage.getItem("gitswamp-theme");
if (savedTheme === "light") {
  document.documentElement.classList.add("light");
}

const savedFontSize = localStorage.getItem("gitswamp-font-size");
if (savedFontSize) {
  const fontSizes: Record<string, string> = { small: "14px", medium: "16px", large: "18px" };
  document.documentElement.style.setProperty("--font-size", fontSizes[savedFontSize] || "16px");
}
const savedCompact = localStorage.getItem("gitswamp-compact-mode");
if (savedCompact === "true") {
  document.documentElement.classList.add("compact");
}
const savedAvatars = localStorage.getItem("gitswamp-show-avatars");
if (savedAvatars === "false") {
  document.documentElement.classList.add("hide-avatars");
}

interface Tab {
  id: string;
  repo: RepoInfo | null;
  label: string;
  path: string;
}

const tabs = ref<Tab[]>([
  { id: "landing", repo: null, label: "Start", path: "" },
]);
const activeTabId = ref("landing");
const showCloneDialog = ref(false);
const showInitDialog = ref(false);
const showTerminal = ref(false);
const showBranchDialog = ref(false);
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
    // Map resolution to git strategy
    const strategy = resolution === 'keep-modified' 
      ? 'ours' 
      : resolution === 'keep-base' 
        ? 'theirs' 
        : 'delete';

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

const showDetailsPanel = computed(() =>
  viewingWorkingChanges.value || viewingStash.value || git.selectedCommit.value !== null
);

function detectAuthProviderFromOrigin(): "github" | "gitlab" | "gitlab-self" {
  const origin = git.repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url?.toLowerCase() || "";
  if (origin.includes("gitlab.com")) return "gitlab";
  if (origin.includes("gitlab.")) return "gitlab-self";
  return "github";
}

function parseAuthDomainFromOrigin(): string {
  const origin = git.repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url || "";
  const noProto = origin.replace(/^https?:\/\//i, "");
  const noCreds = noProto.includes("@") ? noProto.split("@")[1] : noProto;
  return noCreds.split("/")[0] || "";
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
  await git.pull();
  maybeShowAuthDialogFromGitError();
}

async function handleFetch() {
  await git.fetchAll();
  maybeShowAuthDialogFromGitError();
}

const recentRepos = ref<{ name: string; path: string; branch: string; owner?: string }[]>([]);

const hasWorkingChanges = computed(() => git.stagedFiles.value.length > 0 || git.unstagedFiles.value.length > 0);
const hasConflicts = computed(() => git.hasConflicts.value);

onMounted(() => {
  try {
    const saved = localStorage.getItem("gitswamp-tabs");
    if (saved) {
      const parsed = JSON.parse(saved);
      if (parsed.tabs?.length) {
        tabs.value = parsed.tabs;
        activeTabId.value = parsed.activeTabId || tabs.value[0].id;
      }
    }
    const savedRecent = localStorage.getItem("gitswamp-recent");
    if (savedRecent) {
      recentRepos.value = JSON.parse(savedRecent);
    }
  } catch {}
  const active = tabs.value.find((t) => t.id === activeTabId.value);
  if (active?.path) {
    git.openRepository(active.path);
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
    await git.push();
    maybeShowAuthDialogFromGitError();
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
      if (!authDomainInput.value.trim()) {
        toast.error("Domain is required for self-hosted GitLab");
        return;
      }
      await git.saveProviderToken("gitlab-self", `${authDomainInput.value.trim()}|${token}`);
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
    await invoke("add_gitlab_ssh_key", {
      domain: authDomainInput.value.trim(),
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
  viewingWorkingChanges.value = true;
  viewingStash.value = false;
  git.selectedCommit.value = null;
  git.clearStashSelection();
  detailsPanelCollapsed.value = false;
}

function onSelectConflicts() {
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
        @pull="handlePull"
        @push="handlePush"
        @fetch="handleFetch"
        @branch="handleCreateBranch"
        @stash="handleStash"
        @terminal="showTerminal = !showTerminal"
        @settings="showSettings = true"
      />
      <div class="flex-1 flex overflow-hidden">
        <Sidebar
          :branches="git.localBranches.value"
          :remote-branches="git.remoteBranches.value"
          :current-branch="git.currentBranch.value"
          :stashes="git.stashes.value"
          :tags="git.tags.value"
          :remote-provider="git.repoInfo.value?.remotes?.[0]?.provider || 'unknown'"
          @checkout="git.checkoutBranch($event)"
          @create-branch="submitCreateBranch"
          @delete-branch="git.deleteBranch($event)"
          @stash-pop="git.stashPop($event)"
          @stash-apply="git.stashApply($event)"
          @stash-drop="git.stashDrop($event)"
        />
        <div class="flex-1 flex flex-col overflow-hidden">
          <div class="flex-1 flex overflow-hidden" :style="showTerminal ? 'height: 75%' : ''">
            <FileDiffViewer
              v-if="showDiffViewer"
              class="flex-1"
              :repo-path="git.repoPath.value"
              :file-path="diffFilePath"
              :commit-sha="diffCommitSha"
              :staged="diffStaged"
              @close="closeDiffViewer"
              @refresh="git.refreshStatus()"
            />
            <CommitGraph
              v-else
              :class="showDetailsPanel ? '' : 'flex-1'"
              :commits="git.displayedCommits.value"
              :selected="git.selectedCommit.value"
              :search-query="git.searchQuery.value"
              :has-working-changes="hasWorkingChanges"
              :has-conflicts="hasConflicts"
              :current-branch="git.currentBranch.value"
              :has-more="git.searchQuery.value ? git.hasMoreSearchResults.value : git.hasMoreCommits.value"
              :stashes="git.stashes.value"
              :tags="git.tags.value"
              :remote-provider="git.repoInfo.value?.remotes?.[0]?.provider || 'unknown'"
              @select="onSelectCommit"
              @search="git.searchCommits($event)"
              @clear-search="git.clearSearch()"
              @select-working-changes="onSelectWorkingChanges"
              @select-conflicts="onSelectConflicts"
              @load-more="git.loadMoreCommits()"
              @checkout="git.checkoutCommit($event)"
              @create-branch-at="handleCreateBranchAtCommit($event)"
              @cherry-pick="git.cherryPick($event)"
              @revert="git.revertCommit($event)"
              @reset-soft="git.resetToCommit($event, 'soft')"
              @reset-mixed="git.resetToCommit($event, 'mixed')"
              @reset-hard="git.resetToCommit($event, 'hard')"
              @copy-sha="() => {}"
              @create-tag-at="handleCreateTagAtCommit($event)"
              @create-annotated-tag-at="handleCreateAnnotatedTag($event)"
              @checkout-branch="git.checkoutBranch($event)"
              @checkout-remote-branch="handleCheckoutRemoteBranch($event)"
              @pull="handlePull"
              @push="handlePush"
              @set-upstream="(branch: string, remoteBranch: string) => git.setUpstream(branch, remoteBranch)"
              @edit-commit-message="handleEditCommitMessage($event)"
              @rename-branch="handleRenameBranch($event)"
              @delete-branch="git.deleteBranch($event)"
              @delete-remote-branch="git.deleteRemoteBranch($event)"
              @delete-branch-and-remote="handleDeleteBranchAndRemote($event)"
              @copy-branch-name="() => {}"
              @reset-branch-to-remote="git.resetBranchToRemote($event)"
              @delete-tag="git.deleteTag($event)"
              @stash-pop="git.stashPop($event)"
              @stash-apply="git.stashApply($event)"
              @stash-drop="git.stashDrop($event)"
              @select-stash="onSelectStash($event)"
              @request-merge="handleRequestMerge($event)"
            />
            <div v-if="showDetailsPanel" class="relative flex h-full">
              <button
                @click="detailsPanelCollapsed = !detailsPanelCollapsed"
                :class="[
                  'absolute z-[400] w-5 h-7 rounded-l-lg flex items-center justify-center hover:bg-[var(--primary)]/15 transition-colors text-[var(--primary)] bg-[var(--card)] border border-r-0 border-[var(--primary)]/30 hover:border-[var(--primary)]/60',
                  detailsPanelCollapsed ? '-left-5' : '-left-5',
                ]"
                :title="detailsPanelCollapsed ? 'Expand panel' : 'Collapse panel'"
              >
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  class="w-3.5 h-3.5 transition-transform duration-200"
                  :class="detailsPanelCollapsed ? 'rotate-180' : ''"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
                </svg>
              </button>
              <CommitDetails
                v-show="!detailsPanelCollapsed"
                :commit="git.selectedCommit.value"
                :staged-files="git.stagedFiles.value"
                :unstaged-files="git.unstagedFiles.value"
                :conflict-files="git.conflictFiles.value"
                :has-conflicts="git.hasConflicts.value"
                :commit-files="git.selectedCommitFiles.value"
                :is-working-changes="viewingWorkingChanges"
                :is-stash="viewingStash"
                :selected-stash="git.selectedStash.value"
                :stash-files="git.selectedStashFiles.value"
                :repo-path="git.repoPath.value"
                @stage="git.stageFile($event)"
                @unstage="git.unstageFile($event)"
                @stage-all="git.stageAll()"
                @unstage-all="git.unstageAll()"
                @commit="git.commitChanges($event)"
                @discard="git.discardFile($event)"
                @discard-all="git.discardAll()"
                @resolve-all-conflicts="git.resolveAllConflicts()"
                @resolve-conflict="git.promptResolveConflict($event)"
                @manual-resolve="openConflictResolver($event)"
                @stash-pop="git.stashPop($event)"
                @stash-apply="git.stashApply($event)"
                @stash-drop="git.stashDrop($event)"
                @view-diff="openDiffViewer($event.path, $event.sha, $event.staged)"
              />
            </div>
          </div>
          <TerminalPanel
            v-if="showTerminal"
            :output="git.terminalOutput.value"
            :repo-path="git.repoPath.value"
            style="height: 25%"
            @run="git.runTerminalCommand($event)"
            @close="showTerminal = false"
          />
        </div>
      </div>
    </template>

    <div v-if="showBranchDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="showBranchDialog = false">
      <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
        <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Create New Branch</h3>
        <input
          v-model="newBranchName"
          placeholder="Branch name..."
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
          @keyup.enter="submitCreateBranch(newBranchName)"
          autofocus
        />
        <div class="flex justify-end gap-2">
          <button @click="showBranchDialog = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
          <button @click="submitCreateBranch(newBranchName)" :disabled="!newBranchName.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Create</button>
        </div>
      </div>
    </div>

    <div v-if="showStashDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="showStashDialog = false">
      <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
        <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Stash Changes</h3>
        <input
          v-model="stashMessage"
          placeholder="Stash message (optional)..."
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
          @keyup.enter="submitStash"
          autofocus
        />
        <div class="flex justify-end gap-2">
          <button @click="showStashDialog = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
          <button @click="submitStash" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded transition-colors">Stash</button>
        </div>
      </div>
    </div>

    <div v-if="showTagDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="showTagDialog = false">
      <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
        <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Create Tag</h3>
        <input
          v-model="tagName"
          placeholder="Tag name..."
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
          @keyup.enter="submitCreateTag"
          autofocus
        />
        <div class="flex justify-end gap-2">
          <button @click="showTagDialog = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
          <button @click="submitCreateTag" :disabled="!tagName.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Create</button>
        </div>
      </div>
    </div>

    <div v-if="showAnnotatedTagDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="showAnnotatedTagDialog = false">
      <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
        <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Create Annotated Tag</h3>
        <input
          v-model="annotatedTagName"
          placeholder="Tag name..."
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-3"
          autofocus
        />
        <textarea
          v-model="annotatedTagMessage"
          placeholder="Tag message..."
          rows="3"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-none mb-4"
        />
        <div class="flex justify-end gap-2">
          <button @click="showAnnotatedTagDialog = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
          <button @click="submitAnnotatedTag" :disabled="!annotatedTagName.trim() || !annotatedTagMessage.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Create</button>
        </div>
      </div>
    </div>

    <div v-if="showEditMessageDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="showEditMessageDialog = false">
      <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-[480px] shadow-2xl">
        <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Edit Commit Message</h3>
        <textarea
          v-model="editMessageText"
          rows="5"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-none mb-4"
          autofocus
        />
        <div class="flex justify-end gap-2">
          <button @click="showEditMessageDialog = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
          <button @click="submitEditMessage" :disabled="!editMessageText.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Save</button>
        </div>
      </div>
    </div>

    <div v-if="showRenameDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="showRenameDialog = false">
      <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
        <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Rename Branch</h3>
        <div class="text-[10px] text-[var(--muted-foreground)] mb-2">Rename "{{ renameBranchOld }}" to:</div>
        <input
          v-model="renameBranchNew"
          placeholder="New branch name..."
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
          @keyup.enter="submitRenameBranch"
          autofocus
        />
        <div class="flex justify-end gap-2">
          <button @click="showRenameDialog = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
          <button @click="submitRenameBranch" :disabled="!renameBranchNew.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Rename</button>
        </div>
      </div>
    </div>

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
    
    <div v-if="showPushUsernameDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="showPushUsernameDialog = false">
      <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
        <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Git {{ pushPlatform === 'gitlab-self-hosted' ? 'GitLab' : pushPlatform === 'github-enterprise' ? 'GitHub Enterprise' : pushPlatform }} Credentials</h3>
        
        <!-- Domain input for self-hosted instances -->
        <div v-if="pushPlatform === 'gitlab-self-hosted' || pushPlatform === 'github-enterprise'" class="mb-4">
          <label for="push-domain" class="text-xs text-[var(--muted-foreground)] block mb-2">Domain (e.g., gitlab.company.com)</label>
          <input
            id="push-domain"
            v-model="pushDomain"
            placeholder="gitlab.company.com"
            class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
          />
        </div>
        
        <!-- Username input -->
        <div class="mb-4">
          <label for="push-username" class="text-xs text-[var(--muted-foreground)] block mb-2">Username</label>
          <input
            id="push-username"
            v-model="pushUsername"
            :placeholder="`Your ${pushPlatform} username...`"
            class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            @keyup.enter="performPush(pushPlatform, pushUsername)"
            :autofocus="!(pushPlatform === 'gitlab-self-hosted' || pushPlatform === 'github-enterprise')"
          />
        </div>
        
        <div class="flex justify-end gap-2">
          <button @click="showPushUsernameDialog = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
          <button @click="performPush(pushPlatform, pushUsername)" :disabled="!pushUsername.trim() || (pushPlatform === 'gitlab-self-hosted' || pushPlatform === 'github-enterprise') && !pushDomain.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Push</button>
        </div>
      </div>
    </div>

    <div v-if="showAuthRequiredDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="showAuthRequiredDialog = false">
      <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-[460px] shadow-2xl">
        <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Authentication Required</h3>

        <div class="mb-3">
          <label for="auth-provider" class="text-xs text-[var(--muted-foreground)] block mb-2">Provider</label>
          <select
            id="auth-provider"
            v-model="authProvider"
            class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          >
            <option value="github">GitHub</option>
            <option value="gitlab">GitLab.com</option>
            <option value="gitlab-self">GitLab self-hosted</option>
          </select>
        </div>

        <div v-if="authProvider === 'gitlab-self'" class="mb-3">
          <label for="auth-domain" class="text-xs text-[var(--muted-foreground)] block mb-2">GitLab domain</label>
          <input
            id="auth-domain"
            v-model="authDomainInput"
            placeholder="gitlab.company.com"
            class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          />
        </div>

        <div class="mb-3">
          <label for="auth-token" class="text-xs text-[var(--muted-foreground)] block mb-2">Personal access token</label>
          <input
            id="auth-token"
            v-model="authTokenInput"
            type="password"
            placeholder="Paste token"
            class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          />
        </div>

        <div v-if="authProvider === 'gitlab-self'" class="space-y-3 mb-4 border border-[var(--border)] rounded p-3 bg-[var(--card)]/30">
          <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wide">Generate and push SSH key (optional)</div>
          <input
            v-model="authEmailInput"
            placeholder="Email for SSH key"
            class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          />
          <input
            v-model="authKeyNameInput"
            placeholder="Key name (default: gitswamp)"
            class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          />
          <button
            class="w-full px-3 py-2 text-xs text-white bg-[#f59e0b] hover:opacity-90 rounded disabled:opacity-50 transition-colors"
            :disabled="authSubmitting || !authDomainInput.trim() || !authTokenInput.trim() || !authEmailInput.trim()"
            @click="generateAndPushGitlabKey"
          >
            Generate & Push SSH Key
          </button>
        </div>

        <div class="flex justify-end gap-2">
          <button @click="showAuthRequiredDialog = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
          <button
            @click="saveAuthToken"
            :disabled="authSubmitting || !authTokenInput.trim() || (authProvider === 'gitlab-self' && !authDomainInput.trim())"
            class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors"
          >
            Save Token
          </button>
        </div>
      </div>
    </div>

    <ToastContainer />
  </div>
</template>

<script setup lang="ts">
import TitleBar from "@/components/layout/TitleBar.vue";
import AppHeader from "@/components/layout/AppHeader.vue";
import RepositoryTabs from "@/components/repository/RepositoryTabs.vue";
import Sidebar from "@/components/repository/Sidebar.vue";
import CommitGraph from "@/components/commits/CommitGraph.vue";
import CommitDetails from "@/components/commits/CommitDetails.vue";
import LandingPage from "@/components/repository/LandingPage.vue";
import CloneDialog from "@/components/repository/CloneDialog.vue";
import InitDialog from "@/components/repository/InitDialog.vue";
import TerminalPanel from "@/components/layout/TerminalPanel.vue";
import SettingsDialog from "@/components/layout/SettingsDialog.vue";
import { useGit } from "@/composables/useGit";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ref, watch, onMounted, computed } from "vue";
import type { RepoInfo, CommitInfo, StashInfo } from "@/types";

const git = useGit();

// Initialize theme from localStorage
const savedTheme = localStorage.getItem("gitswamp-theme");
if (savedTheme === "light") {
  document.documentElement.classList.add("light");
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

// Track whether user clicked "working changes" vs a commit vs a stash
const viewingWorkingChanges = ref(false);
const viewingStash = ref(false);

// Whether to show the details panel (only when something is selected)
const showDetailsPanel = computed(() =>
  viewingWorkingChanges.value || viewingStash.value || git.selectedCommit.value !== null
);

const recentRepos = ref<{ name: string; path: string; branch: string; owner?: string }[]>([]);

const hasWorkingChanges = computed(() =>
  git.stagedFiles.value.length > 0 || git.unstagedFiles.value.length > 0
);

// Load saved state
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

// Save tabs on change
watch([tabs, activeTabId], () => {
  try {
    localStorage.setItem(
      "gitswamp-tabs",
      JSON.stringify({ tabs: tabs.value, activeTabId: activeTabId.value })
    );
  } catch {}
}, { deep: true });

// Save recent repos
watch(recentRepos, () => {
  try {
    localStorage.setItem("gitswamp-recent", JSON.stringify(recentRepos.value));
  } catch {}
}, { deep: true });

// Fetch commit files when a commit is selected
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
      // Auto-select right panel: working changes if any, otherwise last commit
      if (git.stagedFiles.value.length > 0 || git.unstagedFiles.value.length > 0) {
        viewingWorkingChanges.value = true;
        git.selectedCommit.value = null;
      } else if (git.displayedCommits.value.length > 0) {
        viewingWorkingChanges.value = false;
        git.selectedCommit.value = git.displayedCommits.value[0];
      }
    }
  } catch {}
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

async function handleClone(url: string, path: string, shallow: boolean) {
  const clonedPath = await git.cloneRepo(url, path, shallow);
  if (clonedPath) {
    showCloneDialog.value = false;
    await openRepo(clonedPath);
  }
}

async function handleInit(path: string, branchName: string) {
  const ok = await git.initRepo(path, branchName);
  if (ok) {
    showInitDialog.value = false;
    await openRepo(path);
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
}

function onSelectWorkingChanges() {
  viewingWorkingChanges.value = true;
  viewingStash.value = false;
  git.selectedCommit.value = null;
  git.clearStashSelection();
}

function onSelectStash(stash: StashInfo) {
  viewingWorkingChanges.value = false;
  viewingStash.value = true;
  git.selectedCommit.value = null;
  git.selectStash(stash);
}

async function handleCheckoutRemoteBranch(name: string) {
  // Check if local branch exists and is behind remote
  const localBranch = git.localBranches.value.find(b => b.name === name);
  if (localBranch && localBranch.behind > 0) {
    // Local exists and is behind remote - reset to remote
    await git.resetBranchToRemote(name);
  } else if (localBranch) {
    // Local exists, just checkout
    await git.checkoutBranch(name);
  } else {
    // No local branch, create tracking branch from remote
    try {
      await git.createBranch(name, "origin/" + name);
      await git.checkoutBranch(name);
    } catch {
      // Fallback: try just checkout (git may auto-create tracking branch)
      await git.checkoutBranch(name);
    }
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

    <!-- Landing page -->
    <template v-if="isLanding">
      <LandingPage
        :open-repos="openReposList"
        :recent-repos="recentRepos"
        @open="openRepo"
        @browse="browseAndOpen"
        @clone="showCloneDialog = true"
        @init="showInitDialog = true"
        @remove-recent="removeRecent"
        @clear-recent="clearRecent"
      />
    </template>

    <!-- Repository view -->
    <template v-else-if="git.repoInfo.value">
      <AppHeader
        :loading="git.loading.value"
        @pull="git.pull()"
        @push="git.push()"
        @fetch="git.fetchAll()"
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
          @checkout="git.checkoutBranch($event)"
          @create-branch="submitCreateBranch"
          @delete-branch="git.deleteBranch($event)"
          @stash-pop="git.stashPop($event)"
          @stash-apply="git.stashApply($event)"
          @stash-drop="git.stashDrop($event)"
        />
        <div class="flex-1 flex flex-col overflow-hidden">
          <div class="flex-1 flex overflow-hidden" :style="showTerminal ? 'height: 75%' : ''">
            <CommitGraph
              :class="showDetailsPanel ? '' : 'flex-1'"
              :commits="git.displayedCommits.value"
              :selected="git.selectedCommit.value"
              :search-query="git.searchQuery.value"
              :has-working-changes="hasWorkingChanges"
              :current-branch="git.currentBranch.value"
              :has-more="git.hasMoreCommits.value"
              :stashes="git.stashes.value"
              :tags="git.tags.value"
              @select="onSelectCommit"
              @search="git.searchCommits($event)"
              @clear-search="git.clearSearch()"
              @select-working-changes="onSelectWorkingChanges"
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
              @pull="git.pull()"
              @push="git.push()"
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
            />
            <CommitDetails
              v-if="showDetailsPanel"
              :commit="git.selectedCommit.value"
              :staged-files="git.stagedFiles.value"
              :unstaged-files="git.unstagedFiles.value"
              :commit-files="git.selectedCommitFiles.value"
              :is-working-changes="viewingWorkingChanges"
              :is-stash="viewingStash"
              :selected-stash="git.selectedStash.value"
              :stash-files="git.selectedStashFiles.value"
              @stage="git.stageFile($event)"
              @unstage="git.unstageFile($event)"
              @stage-all="git.stageAll()"
              @unstage-all="git.unstageAll()"
              @commit="git.commitChanges($event)"
              @discard="git.discardFile($event)"
              @discard-all="git.discardAll()"
              @stash-pop="git.stashPop($event)"
              @stash-apply="git.stashApply($event)"
              @stash-drop="git.stashDrop($event)"
            />
          </div>
          <!-- Terminal panel (25% height) -->
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

    <!-- Branch creation dialog -->
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

    <!-- Stash dialog -->
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

    <!-- Tag creation dialog -->
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

    <!-- Annotated tag creation dialog -->
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

    <!-- Edit commit message dialog -->
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

    <!-- Rename branch dialog -->
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

    <!-- Dialogs -->
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
    <SettingsDialog
      v-if="showSettings"
      :token="git.githubToken.value"
      :git-path="git.gitPath.value"
      @save="git.saveToken($event); showSettings = false"
      @delete="git.deleteToken()"
      @close="showSettings = false"
    />
  </div>
</template>
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
import { useGit } from "@/composables/useGit";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ref, watch, onMounted, computed } from "vue";
import type { RepoInfo, CommitInfo } from "@/types";

const git = useGit();

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
const newBranchName = ref("");
const stashMessage = ref("");

// Track whether user clicked "working changes" vs a commit
const viewingWorkingChanges = ref(false);

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
  const ok = await git.cloneRepo(url, path, shallow);
  if (ok) {
    showCloneDialog.value = false;
    const repoName = url.split("/").pop()?.replace(/\.git$/, "") || "repo";
    const fullPath = path.replace(/[/\\]$/, "") + "\\" + repoName;
    await openRepo(fullPath);
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
  git.selectedCommit.value = commit;
}

function onSelectWorkingChanges() {
  viewingWorkingChanges.value = true;
  git.selectedCommit.value = null;
}

function handleCreateBranch() {
  showBranchDialog.value = true;
}

function submitCreateBranch(name: string) {
  git.createBranch(name);
  showBranchDialog.value = false;
  newBranchName.value = "";
}

function handleStash() {
  showStashDialog.value = true;
}

function submitStash() {
  git.stashPush(stashMessage.value || undefined);
  showStashDialog.value = false;
  stashMessage.value = "";
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
  <div class="h-screen w-screen flex flex-col bg-[#0d1017] overflow-hidden">
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
              :commits="git.displayedCommits.value"
              :selected="git.selectedCommit.value"
              :search-query="git.searchQuery.value"
              :has-working-changes="hasWorkingChanges"
              :current-branch="git.currentBranch.value"
              @select="onSelectCommit"
              @search="git.searchCommits($event)"
              @clear-search="git.clearSearch()"
              @select-working-changes="onSelectWorkingChanges"
            />
            <CommitDetails
              :commit="git.selectedCommit.value"
              :staged-files="git.stagedFiles.value"
              :unstaged-files="git.unstagedFiles.value"
              :commit-files="git.selectedCommitFiles.value"
              :is-working-changes="viewingWorkingChanges"
              @stage="git.stageFile($event)"
              @unstage="git.unstageFile($event)"
              @stage-all="git.stageAll()"
              @unstage-all="git.unstageAll()"
              @commit="git.commitChanges($event)"
              @discard="git.discardFile($event)"
              @discard-all="git.discardAll()"
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
      <div class="bg-[#0f1620] border border-[#8b5cf6]/20 rounded-lg p-6 w-96 shadow-2xl">
        <h3 class="text-sm font-medium text-[#e2e8f0] mb-4">Create New Branch</h3>
        <input
          v-model="newBranchName"
          placeholder="Branch name..."
          class="w-full px-3 py-2 bg-[#151d28] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] placeholder:text-[#334155] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40 mb-4"
          @keyup.enter="submitCreateBranch(newBranchName)"
          autofocus
        />
        <div class="flex justify-end gap-2">
          <button @click="showBranchDialog = false" class="px-3 py-1.5 text-xs text-[#64748b] hover:text-[#e2e8f0] rounded hover:bg-[#1e293b] transition-colors">Cancel</button>
          <button @click="submitCreateBranch(newBranchName)" :disabled="!newBranchName.trim()" class="px-3 py-1.5 text-xs text-white bg-[#8b5cf6] hover:bg-[#7c3aed] rounded disabled:opacity-50 transition-colors">Create</button>
        </div>
      </div>
    </div>

    <!-- Stash dialog -->
    <div v-if="showStashDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="showStashDialog = false">
      <div class="bg-[#0f1620] border border-[#8b5cf6]/20 rounded-lg p-6 w-96 shadow-2xl">
        <h3 class="text-sm font-medium text-[#e2e8f0] mb-4">Stash Changes</h3>
        <input
          v-model="stashMessage"
          placeholder="Stash message (optional)..."
          class="w-full px-3 py-2 bg-[#151d28] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] placeholder:text-[#334155] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40 mb-4"
          @keyup.enter="submitStash"
          autofocus
        />
        <div class="flex justify-end gap-2">
          <button @click="showStashDialog = false" class="px-3 py-1.5 text-xs text-[#64748b] hover:text-[#e2e8f0] rounded hover:bg-[#1e293b] transition-colors">Cancel</button>
          <button @click="submitStash" class="px-3 py-1.5 text-xs text-white bg-[#8b5cf6] hover:bg-[#7c3aed] rounded transition-colors">Stash</button>
        </div>
      </div>
    </div>

    <!-- Dialogs -->
    <CloneDialog
      :visible="showCloneDialog"
      @close="showCloneDialog = false"
      @clone="handleClone"
    />
    <InitDialog
      :visible="showInitDialog"
      @close="showInitDialog = false"
      @init="handleInit"
    />
  </div>
</template>
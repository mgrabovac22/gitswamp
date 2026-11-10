<script setup lang="ts">
import { reactive, ref, computed, nextTick, watch } from "vue";
import {
  Search,
  GitBranch,
  Archive,
  Tag,
  Globe,
  Plus,
  Trash2,
  Github,
  HardDrive,
  Cloud,
  GitPullRequest,
  CircleDot,
  FileCode2,
  LifeBuoy,
  Clipboard,
  Columns3,
  GitBranchPlus,
  GitMerge,
  GitPullRequestArrow,
  Link2,
  PencilLine,
  RotateCcw,
} from "lucide-vue-next";
import RepositorySidebarSection from "./RepositorySidebarSection.vue";
import AppUpdatePanel from "@/features/updater/AppUpdatePanel.vue";
import type { BranchInfo, StashInfo, TagInfo, IssueInfo, PullRequestInfo, GistInfo, LostCommitInfo } from "@/types";

const props = defineProps<{
  repoPath?: string;
  branches: BranchInfo[];
  remoteBranches: BranchInfo[];
  currentBranch: string;
  stashes: StashInfo[];
  tags: TagInfo[];
  issues?: IssueInfo[];
  pullRequests?: PullRequestInfo[];
  gists?: GistInfo[];
  lostCommits?: LostCommitInfo[];
  issuesHasMore?: boolean;
  pullRequestsHasMore?: boolean;
  issuesLoadingAll?: boolean;
  pullRequestsLoadingAll?: boolean;
  selectedIssueNumber?: number | null;
  selectedPullRequestNumber?: number | null;
  openPullRequestBranches?: string[];
  remoteProvider?: 'github' | 'gitlab' | 'bitbucket' | 'azure' | 'unknown';
}>();

const emit = defineEmits<{
  checkout: [branchName: string];
  checkoutRemote: [branchName: string];
  checkoutTag: [sha: string];
  createBranch: [name: string];
  createBranchAt: [ref: string];
  openBranchWorkspace: [branchRef: string];
  renameBranch: [name: string];
  requestMerge: [payload: { source: string; sourceRemote: boolean; target: string }];
  requestRebase: [payload: { source: string; sourceRemote: boolean; target: string }];
  setUpstream: [branch: string, upstream: string];
  resetBranchToRemote: [name: string];
  deleteBranch: [name: string];
  deleteRemoteBranch: [name: string];
  deleteBranchAndRemote: [name: string];
  stashPop: [index: number];
  stashApply: [index: number];
  stashDrop: [index: number];
  selectIssue: [issueNumber: number];
  selectPullRequest: [pullRequestNumber: number];
  loadAllIssues: [];
  loadAllPullRequests: [];
  openGist: [url: string];
  openCreateIssue: [];
  openCreatePullRequest: [];
  createGist: [];
  openLostFound: [];
}>();

interface RepositorySidebarSections {
  local: boolean;
  remote: boolean;
  stashes: boolean;
  lostFound: boolean;
  tags: boolean;
  issues: boolean;
  pullRequests: boolean;
  gists: boolean;
}

const expandedSections = reactive<RepositorySidebarSections>({
  local: true,
  remote: false,
  stashes: false,
  lostFound: false,
  tags: false,
  issues: false,
  pullRequests: false,
  gists: false,
});

const branchFilter = ref("");
const issueSearch = ref("");
const pullRequestSearch = ref("");
const showNewBranch = ref(false);
const newBranchName = ref("");
const branchMenu = ref<{ branch: BranchInfo; x: number; y: number; trigger: HTMLElement } | null>(null);
const branchMenuElement = ref<HTMLElement | null>(null);

function openBranchMenu(event: MouseEvent | KeyboardEvent, branch: BranchInfo) {
  if (event instanceof KeyboardEvent && event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10")) return;
  event.preventDefault();
  event.stopPropagation();
  const trigger = event.currentTarget as HTMLElement;
  const bounds = trigger.getBoundingClientRect();
  const x = event instanceof MouseEvent ? event.clientX : bounds.left;
  const y = event instanceof MouseEvent ? event.clientY : bounds.bottom;
  const menu = { branch, x, y, trigger };
  branchMenu.value = menu;
  void nextTick(() => {
    const panel = branchMenuElement.value;
    if (!panel || branchMenu.value?.trigger !== trigger) return;
    branchMenu.value.x = Math.max(8, Math.min(x, window.innerWidth - panel.offsetWidth - 8));
    branchMenu.value.y = Math.max(8, Math.min(y, window.innerHeight - panel.offsetHeight - 8));
    panel.querySelector<HTMLElement>('[role="menuitem"]')?.focus({ preventScroll: true });
  });
}

watch(() => props.repoPath, () => { branchMenu.value = null; });
watch(() => Boolean(branchMenu.value), (open, _, onCleanup) => {
  if (!open) return;
  const dismiss = () => { branchMenu.value = null; };
  const outside = (event: PointerEvent) => {
    if (!branchMenuElement.value?.contains(event.target as Node)) dismiss();
  };
  window.addEventListener("pointerdown", outside);
  window.addEventListener("resize", dismiss);
  onCleanup(() => {
    window.removeEventListener("pointerdown", outside);
    window.removeEventListener("resize", dismiss);
  });
});

function onBranchMenuKey(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    branchMenu.value?.trigger.focus({ preventScroll: true });
    branchMenu.value = null;
    return;
  }
  if (event.key === "Tab") { branchMenu.value = null; return; }
  if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
  const items = Array.from(branchMenuElement.value?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
  if (!items.length) return;
  event.preventDefault();
  const index = items.indexOf(document.activeElement as HTMLButtonElement);
  const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1
    : (index + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
  items[next].focus();
}

const canIntegrateMenuBranch = computed(() => {
  const branch = branchMenu.value?.branch;
  return Boolean(branch && props.currentBranch && props.currentBranch !== "HEAD"
    && (branch.is_remote ? branch.name.startsWith("origin/") && branch.name !== `origin/${props.currentBranch}` : branch.name !== props.currentBranch));
});

const menuBranchHasOrigin = computed(() => {
  const branch = branchMenu.value?.branch;
  return !!branch && !branch.is_remote && props.remoteBranches.some(remote => remote.name === `origin/${branch.name}`);
});

function branchMenuAction(action: "checkout" | "create" | "workspace" | "rename" | "merge" | "rebase" | "upstream" | "reset" | "copy" | "delete" | "delete-both") {
  const branch = branchMenu.value?.branch;
  if (!branch) return;
  const hasOrigin = menuBranchHasOrigin.value;
  branchMenu.value = null;
  const ref = `${branch.is_remote ? "refs/remotes/" : "refs/heads/"}${branch.name}`;
  if (action === "checkout") emit("checkout", branch.is_remote ? ref : branch.name);
  if (action === "create") emit("createBranchAt", ref);
  if (action === "workspace") emit("openBranchWorkspace", ref);
  if (action === "rename" && !branch.is_remote) emit("renameBranch", branch.name);
  const source = branch.is_remote ? branch.name.slice("origin/".length) : branch.name;
  if (action === "merge") emit("requestMerge", { source, sourceRemote: branch.is_remote, target: props.currentBranch });
  if (action === "rebase") emit("requestRebase", { source: props.currentBranch, sourceRemote: false, target: source });
  if (action === "upstream" && hasOrigin) emit("setUpstream", branch.name, `origin/${branch.name}`);
  if (action === "reset" && hasOrigin) emit("resetBranchToRemote", branch.name);
  if (action === "copy") void navigator.clipboard.writeText(branch.name).catch(() => {});
  if (action === "delete" && !branch.is_remote) emit("deleteBranch", branch.name);
  if (action === "delete" && branch.is_remote && branch.name.startsWith("origin/")) emit("deleteRemoteBranch", source);
  if (action === "delete-both" && hasOrigin) emit("deleteBranchAndRemote", branch.name);
}

function toggleSection(section: keyof RepositorySidebarSections) {
  expandedSections[section] = !expandedSections[section];
}

function submitNewBranch() {
  if (!newBranchName.value.trim()) return;
  emit("createBranch", newBranchName.value.trim());
  newBranchName.value = "";
  showNewBranch.value = false;
}

function filteredBranches(list: BranchInfo[]): BranchInfo[] {
  if (!branchFilter.value.trim()) return list;
  const q = branchFilter.value.toLowerCase();
  return list.filter((b) => b.name.toLowerCase().includes(q));
}

const remoteLabel = computed(() => {
  switch (props.remoteProvider) {
    case 'github': return 'GITHUB';
    case 'gitlab': return 'GITLAB';
    case 'bitbucket': return 'BITBUCKET';
    case 'azure': return 'AZURE';
    default: return 'REMOTE';
  }
});

const remoteIcon = computed(() => {
  return props.remoteProvider === 'github' ? Github : Globe;
});

const showGithubInsights = computed(() => props.remoteProvider === "github");

const sortedIssues = computed(() => {
  const values = [...(props.issues || [])];
  values.sort((a, b) => b.number - a.number);
  return values;
});

const filteredIssues = computed(() => {
  const query = issueSearch.value.trim().toLowerCase();
  if (!query) {
    return sortedIssues.value;
  }

  return sortedIssues.value.filter((issue) => {
    return issue.title.toLowerCase().includes(query)
      || issue.author.toLowerCase().includes(query)
      || String(issue.number).includes(query);
  });
});

const visibleIssues = computed(() => {
  if (issueSearch.value.trim()) {
    return filteredIssues.value;
  }

  return props.issuesHasMore ? filteredIssues.value.slice(0, 50) : filteredIssues.value;
});

const showIssueSearch = computed(() => {
  return sortedIssues.value.length > 50 || props.issuesHasMore || issueSearch.value.trim().length > 0;
});

const issuesCountLabel = computed(() => {
  if (props.issuesHasMore && !issueSearch.value.trim()) {
    return `${Math.min(50, sortedIssues.value.length)}+`;
  }

  return String(visibleIssues.value.length);
});

const sortedPullRequests = computed(() => {
  const values = [...(props.pullRequests || [])];
  values.sort((a, b) => b.number - a.number);
  return values;
});

const filteredPullRequests = computed(() => {
  const query = pullRequestSearch.value.trim().toLowerCase();
  if (!query) {
    return sortedPullRequests.value;
  }

  return sortedPullRequests.value.filter((pullRequest) => {
    return pullRequest.title.toLowerCase().includes(query)
      || pullRequest.author.toLowerCase().includes(query)
      || pullRequest.sourceBranch.toLowerCase().includes(query)
      || pullRequest.targetBranch.toLowerCase().includes(query)
      || String(pullRequest.number).includes(query);
  });
});

const visiblePullRequests = computed(() => {
  if (pullRequestSearch.value.trim()) {
    return filteredPullRequests.value;
  }

  return props.pullRequestsHasMore ? filteredPullRequests.value.slice(0, 50) : filteredPullRequests.value;
});

const showPullRequestSearch = computed(() => {
  return sortedPullRequests.value.length > 50 || props.pullRequestsHasMore || pullRequestSearch.value.trim().length > 0;
});

const pullRequestsCountLabel = computed(() => {
  if (props.pullRequestsHasMore && !pullRequestSearch.value.trim()) {
    return `${Math.min(50, sortedPullRequests.value.length)}+`;
  }

  return String(visiblePullRequests.value.length);
});

const sortedGists = computed(() => {
  const values = [...(props.gists || [])];
  values.sort((a, b) => {
    const bTime = Date.parse(b.updatedAt || "");
    const aTime = Date.parse(a.updatedAt || "");
    return (Number.isFinite(bTime) ? bTime : 0) - (Number.isFinite(aTime) ? aTime : 0);
  });
  return values;
});

function gistLabel(gist: GistInfo): string {
  return gist.description || gist.filename || "Untitled gist";
}

function gistMeta(gist: GistInfo): string {
  const visibility = gist.public ? "public" : "secret";
  const fileLabel = `${gist.fileCount || 1} file${gist.fileCount === 1 ? "" : "s"}`;
  return `${visibility} · ${fileLabel}`;
}

const openPullRequestBranchSet = computed(() => {
  const branches = props.openPullRequestBranches || [];
  return new Set(
    branches
      .map((name) => normalizeBranchName(name).toLowerCase())
      .filter((name) => name.length > 0),
  );
});

function normalizeBranchName(name: string): string {
  const withoutRemote = name.replace(/^origin\//i, "");
  return withoutRemote.replace(/^remotes\/[a-z0-9_-]+\//i, "");
}

function showPullRequestBadge(branch: BranchInfo): boolean {
  if (props.remoteProvider !== "github" && props.remoteProvider !== "gitlab") {
    return false;
  }

  const normalized = normalizeBranchName(branch.name).toLowerCase();
  if (!normalized) {
    return false;
  }

  return openPullRequestBranchSet.value.has(normalized);
}

function pullRequestStateLabel(pullRequest: PullRequestInfo): string {
  if (pullRequest.merged) return "merged";
  if (pullRequest.draft) return "draft";
  return pullRequest.state || "unknown";
}

function pullRequestStateClass(pullRequest: PullRequestInfo): string {
  if (pullRequest.merged) return "border-[#8b5cf6]/35 bg-[#8b5cf6]/12 text-[#a78bfa]";
  if (pullRequest.draft) return "border-[#f59e0b]/35 bg-[#f59e0b]/12 text-[#f59e0b]";
  return pullRequest.state.toLowerCase() === "open"
    ? "border-[#10b981]/35 bg-[#10b981]/12 text-[#34d399]"
    : "border-[var(--sidebar-border)] bg-[var(--sidebar-accent)] text-[var(--muted-foreground)]";
}

function issueStateClass(issue: IssueInfo): string {
  return issue.state.toLowerCase() === "open"
    ? "border-[#10b981]/35 bg-[#10b981]/12 text-[#34d399]"
    : "border-[var(--sidebar-border)] bg-[var(--sidebar-accent)] text-[var(--muted-foreground)]";
}
</script>

<template>
  <div class="w-full bg-[var(--sidebar)] border-r border-[var(--sidebar-border)] flex flex-col h-full flex-shrink-0">
    <div class="p-3 border-b border-[var(--sidebar-border)]">
      <div class="flex items-center justify-start text-left gap-2.5 mb-2.5 px-2 py-2 rounded-lg bg-[var(--sidebar-primary)]/20 border-2 border-[var(--sidebar-primary)]/50 shadow-md shadow-[var(--sidebar-primary)]/10">
        <GitBranch class="w-5 h-5 text-[var(--sidebar-primary)] flex-shrink-0 drop-shadow-[0_0_6px_var(--sidebar-primary)]" />
        <div class="flex-1 min-w-0">
          <div class="text-[9px] text-[var(--sidebar-primary)]/70 uppercase tracking-wider font-semibold">Current Branch</div>
          <div class="text-sm text-[var(--sidebar-primary)] font-extrabold truncate drop-shadow-[0_0_8px_var(--sidebar-primary)]" :title="currentBranch">{{ currentBranch }}</div>
        </div>
      </div>
      <div class="relative">
        <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3 h-3 text-[var(--muted-foreground)]" />
        <input
          v-model="branchFilter"
          placeholder="Filter branches..."
          class="w-full h-7 pl-7 pr-2 bg-[var(--sidebar-accent)] border border-[var(--sidebar-border)] rounded text-[var(--sidebar-foreground)] placeholder:text-[var(--muted-foreground)]/50 text-[11px] focus:outline-none focus:ring-1 focus:ring-[var(--sidebar-ring)]/40"
        />
      </div>
    </div>

    <div class="flex-1 overflow-y-auto">
      <RepositorySidebarSection
        label="LOCAL"
        :count="branches.length"
        :icon="GitBranch"
        :expanded="expandedSections.local"
        @toggle="toggleSection('local')"
      >
        <div class="px-4 pb-1">
          <button
            v-if="!showNewBranch"
            @click="showNewBranch = true"
            class="w-full flex items-center justify-start gap-1.5 px-2 py-1 text-left text-[10px] text-[var(--muted-foreground)] hover:text-[var(--sidebar-primary)] hover:bg-[var(--sidebar-accent)] rounded transition-all"
          >
            <Plus class="w-3 h-3" />
            New branch
          </button>
          <div v-else class="flex items-center gap-1">
            <input
              v-model="newBranchName"
              @keyup.enter="submitNewBranch"
              @keyup.escape="showNewBranch = false"
              placeholder="branch-name"
              class="flex-1 h-6 px-2 bg-[var(--sidebar-accent)] border border-[var(--sidebar-border)] rounded text-[10px] text-[var(--sidebar-foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--sidebar-ring)]/40"
              autofocus
            />
            <button @click="submitNewBranch" class="p-1 rounded hover:bg-[var(--sidebar-primary)]/20 text-[var(--sidebar-primary)]">
              <Plus class="w-3 h-3" />
            </button>
          </div>
        </div>

        <div
          v-for="branch in filteredBranches(branches)"
          :key="branch.name"
          :class="[
            'w-full flex items-center pr-4 text-left text-[11px] transition-all group',
            branch.is_head
              ? 'text-[var(--sidebar-primary)] bg-[var(--sidebar-primary)]/10 font-medium'
              : 'text-[var(--muted-foreground)] hover:text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)]',
          ]"
        >
          <button type="button" class="flex flex-1 min-w-0 items-center gap-2 py-1 pl-8 text-left text-[11px]"
            aria-haspopup="menu" @click="emit('checkout', branch.name)"
            @contextmenu="openBranchMenu($event, branch)" @keydown="openBranchMenu($event, branch)">
          <span class="truncate flex-1 text-left">{{ branch.name }}</span>

          <span class="flex items-center gap-1 flex-shrink-0">
            <HardDrive class="w-3 h-3 text-[var(--muted-foreground)]" title="Local branch" />
            <Cloud
              v-if="branch.upstream"
              class="w-3 h-3 text-[var(--muted-foreground)]"
              :title="'Tracks ' + branch.upstream"
            />
            <GitPullRequest
              v-if="showPullRequestBadge(branch)"
              class="w-3 h-3 text-[var(--sidebar-primary)]"
              title="Pull request branch"
            />
          </span>

          <span v-if="branch.upstream" class="flex items-center gap-0.5 flex-shrink-0">
            <span v-if="branch.ahead > 0" class="text-[9px] text-[#10b981]">↑{{ branch.ahead }}</span>
            <span v-if="branch.behind > 0" class="text-[9px] text-[#f59e0b]">↓{{ branch.behind }}</span>
            <span v-if="branch.ahead === 0 && branch.behind === 0" class="text-[9px] text-[var(--muted-foreground)]">✓</span>
          </span>
          <span v-else class="text-[9px] text-[var(--muted-foreground)] italic flex-shrink-0">local</span>
          </button>
          <button
            v-if="!branch.is_head"
            @click.stop="emit('deleteBranch', branch.name)"
            class="ml-2 p-0.5 rounded opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-[#ef4444]/20 transition-all flex-shrink-0"
            title="Delete branch"
          >
            <Trash2 class="w-2.5 h-2.5 text-[var(--muted-foreground)] hover:text-[#ef4444]" />
          </button>
        </div>
      </RepositorySidebarSection>

      <RepositorySidebarSection
        :label="remoteLabel"
        :count="remoteBranches.length"
        :icon="remoteIcon"
        :expanded="expandedSections.remote"
        @toggle="toggleSection('remote')"
      >
        <button
          v-for="branch in filteredBranches(remoteBranches)"
          :key="branch.name"
          aria-haspopup="menu"
          @click="emit('checkoutRemote', normalizeBranchName(branch.name))"
          @contextmenu="openBranchMenu($event, branch)" @keydown="openBranchMenu($event, branch)"
          class="w-full flex items-center justify-start gap-2 px-4 py-1 pl-10 text-left text-[11px] text-[var(--muted-foreground)] hover:text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)] transition-all"
        >
          <span class="truncate flex-1 text-left">{{ branch.name }}</span>
          <span class="flex items-center gap-1 flex-shrink-0">
            <Cloud class="w-3 h-3 text-[var(--muted-foreground)]" title="Remote branch" />
            <GitPullRequest
              v-if="showPullRequestBadge(branch)"
              class="w-3 h-3 text-[var(--sidebar-primary)]"
              title="Pull request branch"
            />
          </span>
        </button>
      </RepositorySidebarSection>

      <RepositorySidebarSection
        label="STASHES"
        :count="stashes.length"
        :icon="Archive"
        :expanded="expandedSections.stashes"
        @toggle="toggleSection('stashes')"
      >
        <div v-if="stashes.length === 0" class="px-4 py-2 text-[10px] text-[var(--muted-foreground)] italic">
          No stashes
        </div>
        <div
          v-for="stash in stashes"
          :key="stash.index"
          class="px-4 py-1.5 pl-8 hover:bg-[var(--sidebar-accent)] transition-all group"
        >
          <div class="text-[11px] text-[var(--sidebar-foreground)] truncate">{{ stash.message || ('stash@{' + stash.index + '}') }}</div>
          <div class="text-[9px] text-[var(--muted-foreground)] truncate" :title="'on ' + stash.branch">on {{ stash.branch }}</div>
          <div class="flex items-center gap-1 mt-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              @click="emit('stashPop', stash.index)"
              class="text-[9px] text-[var(--sidebar-primary)] hover:opacity-80 px-1 py-0.5 rounded hover:bg-[var(--sidebar-primary)]/10"
            >Pop</button>
            <button
              @click="emit('stashApply', stash.index)"
              class="text-[9px] text-[#06b6d4] hover:text-[#22d3ee] px-1 py-0.5 rounded hover:bg-[#06b6d4]/10"
            >Apply</button>
            <button
              @click="emit('stashDrop', stash.index)"
              class="text-[9px] text-[#ef4444] hover:text-[#f87171] px-1 py-0.5 rounded hover:bg-[#ef4444]/10"
            >Drop</button>
          </div>
        </div>
      </RepositorySidebarSection>

      <RepositorySidebarSection
        v-if="(props.lostCommits || []).length > 0"
        label="LOST & FOUND"
        short-label="LOST"
        :count="(props.lostCommits || []).length"
        :icon="LifeBuoy"
        :expanded="expandedSections.lostFound"
        @toggle="toggleSection('lostFound')"
      >
        <div class="px-4 pb-1">
          <button
            @click="emit('openLostFound')"
            class="w-full flex items-center justify-start gap-1.5 px-2 py-1 text-left text-[10px] text-[var(--sidebar-primary)] hover:bg-[var(--sidebar-accent)] rounded transition-all"
          >
            <LifeBuoy class="w-3 h-3" />
            Open recovery view
          </button>
        </div>
        <button
          v-for="commit in (props.lostCommits || []).slice(0, 5)"
          :key="`lost-${commit.sha}`"
          class="w-full text-left px-4 py-1.5 pl-8 text-[var(--muted-foreground)] transition-all hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-foreground)]"
          :title="commit.message || commit.sha"
          @click="emit('openLostFound')"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="min-w-0 truncate text-[10px]">{{ commit.message || "No commit message" }}</span>
            <span class="flex-shrink-0 rounded border border-[#10b981]/25 bg-[#10b981]/10 px-1 py-0.5 font-mono text-[8px] text-[#34d399]">
              {{ commit.short_sha }}
            </span>
          </div>
          <div class="mt-0.5 text-[9px] text-[var(--muted-foreground)] truncate">{{ commit.author_name }} · {{ commit.time_ago }}</div>
        </button>
        <div v-if="(props.lostCommits || []).length > 5" class="px-4 pt-1 text-[9px] text-[var(--muted-foreground)]">
          +{{ (props.lostCommits || []).length - 5 }} more in recovery view
        </div>
      </RepositorySidebarSection>

      <RepositorySidebarSection
        label="TAGS"
        :count="tags.length"
        :icon="Tag"
        :expanded="expandedSections.tags"
        @toggle="toggleSection('tags')"
      >
        <div v-if="tags.length === 0" class="px-4 py-2 text-[10px] text-[var(--muted-foreground)] italic">
          No tags
        </div>
        <button
          v-for="tag in tags"
          :key="tag.name"
          class="w-full px-4 py-1 pl-8 flex items-center text-left text-[11px] text-[var(--muted-foreground)] hover:text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)] transition-all"
          :title="`Checkout ${tag.name} (${tag.sha.slice(0, 7)})`"
          @click="emit('checkoutTag', tag.sha)"
        >
          <span class="min-w-0 flex-1 truncate">{{ tag.name }}</span>
          <span class="ml-2 flex-shrink-0 font-mono text-[8px] opacity-60">{{ tag.sha.slice(0, 7) }}</span>
        </button>
      </RepositorySidebarSection>

      <RepositorySidebarSection
        v-if="showGithubInsights"
        label="ISSUES"
        :count="issuesCountLabel"
        :icon="CircleDot"
        :expanded="expandedSections.issues"
        @toggle="toggleSection('issues')"
      >
        <div class="px-4 pb-1">
          <button
            @click="emit('openCreateIssue')"
            class="w-full flex items-center gap-1.5 px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:text-[var(--sidebar-primary)] hover:bg-[var(--sidebar-accent)] rounded transition-all"
          >
            <Plus class="w-3 h-3" />
            Create issue
          </button>
        </div>
        <div v-if="showIssueSearch" class="px-4 pb-1">
          <input
            v-model="issueSearch"
            type="text"
            placeholder="Search issues..."
            class="w-full h-6 px-2 bg-[var(--sidebar-accent)] border border-[var(--sidebar-border)] rounded text-[10px] text-[var(--sidebar-foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--sidebar-ring)]/40"
          />
        </div>
        <div v-if="visibleIssues.length === 0" class="px-4 py-2 text-[10px] text-[var(--muted-foreground)] italic">
          {{ issueSearch.trim() ? "No matching issues" : "No open issues" }}
        </div>
        <button
          v-for="issue in visibleIssues"
          :key="`issue-${issue.id}`"
          class="w-full text-left px-4 py-1.5 pl-8 transition-all"
          :class="props.selectedIssueNumber === issue.number
            ? 'bg-[var(--sidebar-primary)]/12 text-[var(--sidebar-foreground)]'
            : 'text-[var(--muted-foreground)] hover:text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)]'"
          @click="emit('selectIssue', issue.number)"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="text-[10px] font-semibold">#{{ issue.number }}</span>
            <span class="rounded-full border px-1.5 py-0.5 text-[8px] uppercase tracking-wide" :class="issueStateClass(issue)">{{ issue.state }}</span>
          </div>
          <div class="text-[10px] truncate">{{ issue.title }}</div>
          <div v-if="issue.labels && issue.labels.length > 0" class="mt-1 flex flex-wrap gap-1">
            <span
              v-for="label in issue.labels.slice(0, 2)"
              :key="`issue-label-${issue.number}-${label.name}`"
              class="max-w-full truncate rounded border border-[var(--sidebar-border)] bg-[var(--sidebar-accent)] px-1.5 py-0.5 text-[8px] text-[var(--muted-foreground)]"
            >
              {{ label.name }}
            </span>
          </div>
        </button>
        <div v-if="props.issuesHasMore" class="px-4 pt-1">
          <button
            class="w-full flex items-center justify-center gap-1.5 rounded border border-[var(--sidebar-border)] bg-[var(--sidebar-accent)] px-2 py-1 text-[10px] font-medium text-[var(--sidebar-primary)] transition-colors hover:bg-[var(--sidebar-primary)]/10 disabled:cursor-wait disabled:opacity-60"
            :disabled="props.issuesLoadingAll"
            @click="emit('loadAllIssues')"
          >
            <span v-if="props.issuesLoadingAll">Loading...</span>
            <span v-else>Load all</span>
          </button>
        </div>
      </RepositorySidebarSection>

      <RepositorySidebarSection
        v-if="showGithubInsights"
        label="PULL REQUESTS"
        short-label="PRS"
        :count="pullRequestsCountLabel"
        :icon="GitPullRequest"
        :expanded="expandedSections.pullRequests"
        @toggle="toggleSection('pullRequests')"
      >
        <div class="px-4 pb-1">
          <button
            @click="emit('openCreatePullRequest')"
            class="w-full flex items-center gap-1.5 px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:text-[var(--sidebar-primary)] hover:bg-[var(--sidebar-accent)] rounded transition-all"
          >
            <Plus class="w-3 h-3" />
            Create pull request
          </button>
        </div>
        <div v-if="showPullRequestSearch" class="px-4 pb-1">
          <input
            v-model="pullRequestSearch"
            type="text"
            placeholder="Search pull requests..."
            class="w-full h-6 px-2 bg-[var(--sidebar-accent)] border border-[var(--sidebar-border)] rounded text-[10px] text-[var(--sidebar-foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--sidebar-ring)]/40"
          />
        </div>
        <div v-if="visiblePullRequests.length === 0" class="px-4 py-2 text-[10px] text-[var(--muted-foreground)] italic">
          {{ pullRequestSearch.trim() ? "No matching pull requests" : "No open pull requests" }}
        </div>
        <button
          v-for="pullRequest in visiblePullRequests"
          :key="`pr-${pullRequest.id}`"
          class="w-full text-left px-4 py-1.5 pl-8 transition-all"
          :class="props.selectedPullRequestNumber === pullRequest.number
            ? 'bg-[var(--sidebar-primary)]/12 text-[var(--sidebar-foreground)]'
            : 'text-[var(--muted-foreground)] hover:text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)]'"
          @click="emit('selectPullRequest', pullRequest.number)"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="text-[10px] font-semibold">#{{ pullRequest.number }}</span>
            <span class="rounded-full border px-1.5 py-0.5 text-[8px] uppercase tracking-wide" :class="pullRequestStateClass(pullRequest)">
              {{ pullRequestStateLabel(pullRequest) }}
            </span>
          </div>
          <div class="mt-0.5 text-[10px] font-medium truncate">{{ pullRequest.title }}</div>
          <div class="mt-1 flex items-center gap-1 text-[9px] text-[var(--muted-foreground)]">
            <span class="min-w-0 truncate font-mono text-[var(--sidebar-primary)]">{{ pullRequest.sourceBranch }}</span>
            <span class="flex-shrink-0">-></span>
            <span class="min-w-0 truncate font-mono">{{ pullRequest.targetBranch }}</span>
          </div>
          <div v-if="pullRequest.labels && pullRequest.labels.length > 0" class="mt-1 flex flex-wrap gap-1">
            <span
              v-for="label in pullRequest.labels.slice(0, 2)"
              :key="`pr-label-${pullRequest.number}-${label.name}`"
              class="max-w-full truncate rounded border border-[var(--sidebar-border)] bg-[var(--sidebar-accent)] px-1.5 py-0.5 text-[8px] text-[var(--muted-foreground)]"
            >
              {{ label.name }}
            </span>
          </div>
        </button>
        <div v-if="props.pullRequestsHasMore" class="px-4 pt-1">
          <button
            class="w-full flex items-center justify-center gap-1.5 rounded border border-[var(--sidebar-border)] bg-[var(--sidebar-accent)] px-2 py-1 text-[10px] font-medium text-[var(--sidebar-primary)] transition-colors hover:bg-[var(--sidebar-primary)]/10 disabled:cursor-wait disabled:opacity-60"
            :disabled="props.pullRequestsLoadingAll"
            @click="emit('loadAllPullRequests')"
          >
            <span v-if="props.pullRequestsLoadingAll">Loading...</span>
            <span v-else>Load all</span>
          </button>
        </div>
      </RepositorySidebarSection>

      <RepositorySidebarSection
        v-if="showGithubInsights"
        label="GISTS"
        :count="sortedGists.length"
        :icon="FileCode2"
        :expanded="expandedSections.gists"
        @toggle="toggleSection('gists')"
      >
        <div v-if="sortedGists.length === 0" class="px-4 py-2 text-[10px] text-[var(--muted-foreground)] italic">
          No gists loaded
        </div>
        <button
          v-for="gist in sortedGists"
          :key="`gist-${gist.id}`"
          class="w-full text-left px-4 py-1.5 pl-8 text-[var(--muted-foreground)] transition-all hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-foreground)]"
          :title="gistLabel(gist)"
          @click="emit('openGist', gist.url)"
        >
          <div class="text-[10px] truncate">{{ gistLabel(gist) }}</div>
          <div class="text-[9px] text-[var(--muted-foreground)] truncate">{{ gistMeta(gist) }}</div>
        </button>
      </RepositorySidebarSection>

      <div class="px-4 py-2 border-b border-[var(--sidebar-border)]">
        <button
          @click="emit('createGist')"
          class="w-full flex items-center gap-1.5 px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:text-[var(--sidebar-primary)] hover:bg-[var(--sidebar-accent)] rounded transition-all"
        >
          <FileCode2 class="w-3 h-3" />
          Create a gist
        </button>
      </div>
    </div>

    <AppUpdatePanel />
  </div>
  <Teleport to="body">
    <div v-if="branchMenu" ref="branchMenuElement" role="menu" :aria-label="`Branch ${branchMenu.branch.name}`"
      class="branch-menu" :style="{ left: `${branchMenu.x}px`, top: `${branchMenu.y}px` }"
      @keydown="onBranchMenuKey" @contextmenu.prevent>
      <div class="branch-menu-name" :title="branchMenu.branch.name">{{ branchMenu.branch.name }}</div>
      <button role="menuitem" @click="branchMenuAction('checkout')"><GitBranch />Checkout branch</button>
      <button role="menuitem" @click="branchMenuAction('create')"><GitBranchPlus />Create branch here</button>
      <button v-if="!branchMenu.branch.is_remote || !branchMenu.branch.name.endsWith('/HEAD')" role="menuitem" @click="branchMenuAction('workspace')"><Columns3 />Open Branch Workspace</button>
      <button v-if="!branchMenu.branch.is_remote" role="menuitem" @click="branchMenuAction('rename')"><PencilLine />Rename branch</button>
      <button v-if="canIntegrateMenuBranch" role="menuitem" @click="branchMenuAction('merge')"><GitMerge />Merge into current branch</button>
      <button v-if="canIntegrateMenuBranch && !branchMenu.branch.is_remote" role="menuitem" @click="branchMenuAction('rebase')"><GitPullRequestArrow />Rebase current onto this branch</button>
      <button v-if="menuBranchHasOrigin" role="menuitem" @click="branchMenuAction('upstream')"><Link2 />Set upstream</button>
      <button v-if="menuBranchHasOrigin" role="menuitem" @click="branchMenuAction('reset')"><RotateCcw />Reset branch to remote</button>
      <button role="menuitem" @click="branchMenuAction('copy')"><Clipboard />Copy branch name</button>
      <button v-if="!branchMenu.branch.is_remote || branchMenu.branch.name.startsWith('origin/')" role="menuitem" class="branch-menu-delete" @click="branchMenuAction('delete')"><Trash2 />{{ branchMenu.branch.is_remote ? 'Delete remote branch' : 'Delete local branch' }}</button>
      <button v-if="menuBranchHasOrigin" role="menuitem" class="branch-menu-delete" @click="branchMenuAction('delete-both')"><Trash2 />Delete local and remote</button>
    </div>
  </Teleport>
</template>

<style scoped>
.branch-menu { position: fixed; z-index: 120; width: min(236px, calc(100vw - 16px)); max-height: calc(100dvh - 16px); overflow: auto; padding: 4px; border: 1px solid var(--border); border-radius: 6px; background: var(--popover); color: var(--foreground); box-shadow: 0 8px 24px #0004; }
.branch-menu-name { margin-bottom: 3px; padding: 4px 7px 6px; border-bottom: 1px solid var(--border); font-size: 11px; font-weight: 650; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.branch-menu button { display: flex; width: 100%; align-items: center; gap: 8px; padding: 5px 7px; border-radius: 4px; text-align: left; font-size: 11px; }
.branch-menu button:hover, .branch-menu button:focus-visible { background: var(--accent); outline: none; }
.branch-menu button svg { width: 14px; height: 14px; flex: none; }
.branch-menu-delete { color: var(--destructive); }
</style>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, GitPullRequest, X, CircleDot, ArrowLeft } from "lucide-vue-next";
import type { BranchInfo, IssueInfo, PullRequestInfo } from "@/types";

type RemoteInsightsViewMode = "pull-request-detail" | "pull-request-create" | "issue-detail" | "issue-create";

const props = defineProps<{
  mode: RemoteInsightsViewMode;
  pullRequest: PullRequestInfo | null;
  issue: IssueInfo | null;
  remoteBranches: BranchInfo[];
  currentBranch: string;
}>();

const emit = defineEmits<{
  close: [];
  createPullRequest: [payload: { title: string; description: string; sourceBranch: string; targetBranch: string }];
  createIssue: [payload: { title: string; description: string }];
}>();

const pullRequestTitle = ref("");
const pullRequestDescription = ref("");
const pullRequestSourceBranch = ref("");
const pullRequestTargetBranch = ref("");
const sourceBranchSearch = ref("");
const targetBranchSearch = ref("");
const issueTitle = ref("");
const issueDescription = ref("");

function normalizeBranchName(name: string): string {
  return name
    .replace(/^origin\//i, "")
    .replace(/^remotes\/[a-z0-9_-]+\//i, "")
    .trim();
}

const remoteBranchOptions = computed(() => {
  const values = new Set<string>();
  for (const branch of props.remoteBranches) {
    const normalized = normalizeBranchName(branch.name);
    if (normalized) {
      values.add(normalized);
    }
  }
  return Array.from(values).sort((a, b) => a.localeCompare(b));
});

const filteredSourceBranches = computed(() => {
  const query = sourceBranchSearch.value.trim().toLowerCase();
  if (!query) return remoteBranchOptions.value;
  return remoteBranchOptions.value.filter((branch) => branch.toLowerCase().includes(query));
});

const filteredTargetBranches = computed(() => {
  const query = targetBranchSearch.value.trim().toLowerCase();
  if (!query) return remoteBranchOptions.value;
  return remoteBranchOptions.value.filter((branch) => branch.toLowerCase().includes(query));
});

const canSubmitPullRequest = computed(() => {
  return (
    pullRequestTitle.value.trim().length > 0
    && pullRequestSourceBranch.value.trim().length > 0
    && pullRequestTargetBranch.value.trim().length > 0
    && pullRequestSourceBranch.value !== pullRequestTargetBranch.value
  );
});

const canSubmitIssue = computed(() => issueTitle.value.trim().length > 0);

function formatDate(value: string): string {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function suggestDefaultTargetBranch(): string {
  const preferred = ["main", "master", "develop"];
  for (const branch of preferred) {
    if (remoteBranchOptions.value.includes(branch)) {
      return branch;
    }
  }
  return remoteBranchOptions.value[0] || "";
}

function initPullRequestDraft() {
  const branches = remoteBranchOptions.value;
  const current = normalizeBranchName(props.currentBranch);

  const source = branches.includes(current) ? current : (branches[0] || "");
  let target = suggestDefaultTargetBranch();
  if (source && source === target) {
    target = branches.find((branch) => branch !== source) || target;
  }

  pullRequestSourceBranch.value = source;
  pullRequestTargetBranch.value = target;
  sourceBranchSearch.value = source;
  targetBranchSearch.value = target;
  pullRequestTitle.value = source && target ? `Merge ${source} into ${target}` : "";
  pullRequestDescription.value = "";
}

function initIssueDraft() {
  issueTitle.value = "";
  issueDescription.value = "";
}

watch(
  () => props.mode,
  (mode) => {
    if (mode === "pull-request-create") {
      initPullRequestDraft();
    }
    if (mode === "issue-create") {
      initIssueDraft();
    }
  },
  { immediate: true },
);

function selectSourceBranch(branch: string) {
  pullRequestSourceBranch.value = branch;
  sourceBranchSearch.value = branch;
  pullRequestTitle.value = `Merge ${branch} into ${pullRequestTargetBranch.value || "target"}`;
}

function selectTargetBranch(branch: string) {
  pullRequestTargetBranch.value = branch;
  targetBranchSearch.value = branch;
  pullRequestTitle.value = `Merge ${pullRequestSourceBranch.value || "source"} into ${branch}`;
}

function submitPullRequest() {
  if (!canSubmitPullRequest.value) return;
  emit("createPullRequest", {
    title: pullRequestTitle.value.trim(),
    description: pullRequestDescription.value.trim(),
    sourceBranch: pullRequestSourceBranch.value,
    targetBranch: pullRequestTargetBranch.value,
  });
}

function submitIssue() {
  if (!canSubmitIssue.value) return;
  emit("createIssue", {
    title: issueTitle.value.trim(),
    description: issueDescription.value.trim(),
  });
}

function openInBrowser(url: string) {
  if (!url) return;
  openUrl(url).catch(() => {});
}
</script>

<template>
  <div class="flex-1 h-full bg-[var(--background)] flex flex-col min-w-0">
    <div class="h-11 px-3 border-b border-[var(--border)] bg-[var(--background)]/95 backdrop-blur-sm flex items-center justify-between">
      <div class="flex items-center gap-2 min-w-0">
        <button
          class="h-7 px-2 rounded border border-[var(--border)] bg-[var(--secondary)] text-[var(--foreground)] hover:opacity-90 transition-colors text-[10px] inline-flex items-center gap-1"
          @click="emit('close')"
        >
          <ArrowLeft class="w-3 h-3" />
          Back to graph
        </button>

        <div class="min-w-0">
          <div v-if="mode === 'pull-request-detail'" class="text-xs text-[var(--foreground)] font-semibold truncate">
            Pull Request #{{ pullRequest?.number ?? '-' }}
          </div>
          <div v-else-if="mode === 'pull-request-create'" class="text-xs text-[var(--foreground)] font-semibold truncate">
            Create Pull Request
          </div>
          <div v-else-if="mode === 'issue-detail'" class="text-xs text-[var(--foreground)] font-semibold truncate">
            Issue #{{ issue?.number ?? '-' }}
          </div>
          <div v-else class="text-xs text-[var(--foreground)] font-semibold truncate">
            Create Issue
          </div>
        </div>
      </div>

      <button
        class="h-7 w-7 rounded border border-[var(--border)] bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:opacity-90 transition-colors inline-flex items-center justify-center"
        @click="emit('close')"
      >
        <X class="w-3.5 h-3.5" />
      </button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto p-4">
      <div v-if="mode === 'pull-request-detail' && pullRequest" class="space-y-4">
        <div class="rounded border border-[var(--border)] bg-[var(--card)] p-4">
          <div class="text-lg text-[var(--foreground)] font-semibold leading-tight">{{ pullRequest.title }}</div>
          <div class="mt-1 text-[11px] text-[var(--muted-foreground)]">
            {{ pullRequest.state }}{{ pullRequest.draft ? ' • draft' : '' }}
          </div>
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Source branch</div>
            <div class="text-xs text-[var(--foreground)]">{{ pullRequest.sourceBranch }}</div>
          </div>
          <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Target branch</div>
            <div class="text-xs text-[var(--foreground)]">{{ pullRequest.targetBranch }}</div>
          </div>
          <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Author</div>
            <div class="text-xs text-[var(--foreground)]">{{ pullRequest.author }}</div>
          </div>
          <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Updated</div>
            <div class="text-xs text-[var(--foreground)]">{{ formatDate(pullRequest.updatedAt) }}</div>
          </div>
        </div>

        <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Description</div>
          <div class="text-xs text-[var(--foreground)] whitespace-pre-wrap leading-relaxed">{{ pullRequest.description || 'No description provided.' }}</div>
        </div>

        <div class="flex items-center justify-between">
          <div class="text-[10px] text-[var(--muted-foreground)]">Created {{ formatDate(pullRequest.createdAt) }}</div>
          <button
            class="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded text-[11px] font-medium bg-[var(--primary)] text-white hover:opacity-90 transition-colors"
            @click="openInBrowser(pullRequest.url)"
          >
            <ExternalLink class="w-3.5 h-3.5" />
            Open on GitHub
          </button>
        </div>
      </div>

      <div v-else-if="mode === 'issue-detail' && issue" class="space-y-4">
        <div class="rounded border border-[var(--border)] bg-[var(--card)] p-4">
          <div class="text-lg text-[var(--foreground)] font-semibold leading-tight">{{ issue.title }}</div>
          <div class="mt-1 text-[11px] text-[var(--muted-foreground)]">{{ issue.state }}</div>
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Author</div>
            <div class="text-xs text-[var(--foreground)]">{{ issue.author }}</div>
          </div>
          <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Updated</div>
            <div class="text-xs text-[var(--foreground)]">{{ formatDate(issue.updatedAt) }}</div>
          </div>
        </div>

        <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Description</div>
          <div class="text-xs text-[var(--foreground)] whitespace-pre-wrap leading-relaxed">{{ issue.description || 'No description provided.' }}</div>
        </div>

        <div class="flex items-center justify-between">
          <div class="text-[10px] text-[var(--muted-foreground)]">Created {{ formatDate(issue.createdAt) }}</div>
          <button
            class="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded text-[11px] font-medium bg-[var(--primary)] text-white hover:opacity-90 transition-colors"
            @click="openInBrowser(issue.url)"
          >
            <ExternalLink class="w-3.5 h-3.5" />
            Open on GitHub
          </button>
        </div>
      </div>

      <div v-else-if="mode === 'pull-request-create'" class="space-y-4">
        <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3 space-y-3">
          <div>
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Title</div>
            <input
              v-model="pullRequestTitle"
              placeholder="Briefly describe this pull request"
              class="w-full px-3 py-2 text-xs rounded border border-[var(--border)] bg-[var(--input-background)] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            />
          </div>

          <div>
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Description</div>
            <textarea
              v-model="pullRequestDescription"
              rows="4"
              placeholder="What changed and why?"
              class="w-full px-3 py-2 text-xs rounded border border-[var(--border)] bg-[var(--input-background)] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-y"
            />
          </div>

          <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
            <div class="space-y-1.5">
              <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Source branch (remote)</div>
              <input
                v-model="sourceBranchSearch"
                placeholder="Search remote branches..."
                class="w-full px-2.5 py-1.5 text-[11px] rounded border border-[var(--border)] bg-[var(--input-background)] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              />
              <div class="max-h-36 overflow-y-auto rounded border border-[var(--border)] bg-[var(--input-background)]">
                <button
                  v-for="branch in filteredSourceBranches.slice(0, 40)"
                  :key="'src-' + branch"
                  class="w-full text-left px-2.5 py-1.5 text-[11px] transition-colors"
                  :class="pullRequestSourceBranch === branch
                    ? 'bg-[var(--primary)]/18 text-[var(--foreground)]'
                    : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
                  @click="selectSourceBranch(branch)"
                >
                  {{ branch }}
                </button>
              </div>
            </div>

            <div class="space-y-1.5">
              <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Target branch (remote)</div>
              <input
                v-model="targetBranchSearch"
                placeholder="Search remote branches..."
                class="w-full px-2.5 py-1.5 text-[11px] rounded border border-[var(--border)] bg-[var(--input-background)] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              />
              <div class="max-h-36 overflow-y-auto rounded border border-[var(--border)] bg-[var(--input-background)]">
                <button
                  v-for="branch in filteredTargetBranches.slice(0, 40)"
                  :key="'dst-' + branch"
                  class="w-full text-left px-2.5 py-1.5 text-[11px] transition-colors"
                  :class="pullRequestTargetBranch === branch
                    ? 'bg-[var(--primary)]/18 text-[var(--foreground)]'
                    : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
                  @click="selectTargetBranch(branch)"
                >
                  {{ branch }}
                </button>
              </div>
            </div>
          </div>

          <div class="text-[10px] text-[var(--muted-foreground)]">
            Source: <span class="text-[var(--foreground)]">{{ pullRequestSourceBranch || '-' }}</span>
            <span class="mx-1.5">-></span>
            Target: <span class="text-[var(--foreground)]">{{ pullRequestTargetBranch || '-' }}</span>
          </div>

          <div class="pt-1 flex items-center justify-end gap-2">
            <button
              class="px-3 py-1.5 text-[11px] rounded border border-[var(--border)] bg-[var(--secondary)] text-[var(--foreground)] hover:opacity-90 transition-colors"
              @click="emit('close')"
            >
              Cancel
            </button>
            <button
              class="px-3 py-1.5 text-[11px] rounded bg-[var(--primary)] text-white hover:opacity-90 disabled:opacity-45 disabled:cursor-not-allowed transition-colors inline-flex items-center gap-1.5"
              :disabled="!canSubmitPullRequest"
              @click="submitPullRequest"
            >
              <GitPullRequest class="w-3.5 h-3.5" />
              Create Pull Request
            </button>
          </div>
        </div>
      </div>

      <div v-else class="space-y-4">
        <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3 space-y-3">
          <div>
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Title</div>
            <input
              v-model="issueTitle"
              placeholder="What needs to be fixed or improved?"
              class="w-full px-3 py-2 text-xs rounded border border-[var(--border)] bg-[var(--input-background)] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            />
          </div>

          <div>
            <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Description</div>
            <textarea
              v-model="issueDescription"
              rows="6"
              placeholder="Add details, reproduction steps, expected behavior..."
              class="w-full px-3 py-2 text-xs rounded border border-[var(--border)] bg-[var(--input-background)] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-y"
            />
          </div>

          <div class="pt-1 flex items-center justify-end gap-2">
            <button
              class="px-3 py-1.5 text-[11px] rounded border border-[var(--border)] bg-[var(--secondary)] text-[var(--foreground)] hover:opacity-90 transition-colors"
              @click="emit('close')"
            >
              Cancel
            </button>
            <button
              class="px-3 py-1.5 text-[11px] rounded bg-[var(--primary)] text-white hover:opacity-90 disabled:opacity-45 disabled:cursor-not-allowed transition-colors inline-flex items-center gap-1.5"
              :disabled="!canSubmitIssue"
              @click="submitIssue"
            >
              <CircleDot class="w-3.5 h-3.5" />
              Create Issue
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

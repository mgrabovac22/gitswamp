<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ExternalLink,
  GitPullRequest,
  CircleDot,
  ArrowLeft,
  Loader2,
  GitBranch,
  MessageSquare,
  Files,
  GitCommitHorizontal,
} from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import type {
  BranchInfo,
  IssueInfo,
  PullRequestInfo,
  RemoteIssueCreatePayload,
  RemotePullRequestCreatePayload,
  RemoteLabelInfo,
  RemoteMilestoneInfo,
  RemoteReferenceInfo,
  RemoteUserInfo,
} from "@/types";

type RemoteInsightsViewMode = "pull-request-detail" | "pull-request-create" | "issue-detail" | "issue-create";

interface RemoteCreateOptions {
  labels: RemoteLabelInfo[];
  milestones: RemoteMilestoneInfo[];
  assignees: RemoteUserInfo[];
  reviewers: RemoteUserInfo[];
}

const props = defineProps<{
  mode: RemoteInsightsViewMode;
  pullRequest: PullRequestInfo | null;
  issue: IssueInfo | null;
  detailLoading?: boolean;
  createOptions?: RemoteCreateOptions;
  createOptionsLoading?: boolean;
  remoteBranches: BranchInfo[];
  currentBranch: string;
}>();

const emit = defineEmits<{
  close: [];
  createPullRequest: [payload: RemotePullRequestCreatePayload];
  createIssue: [payload: RemoteIssueCreatePayload];
}>();

const pullRequestTitle = ref("");
const pullRequestDescription = ref("");
const pullRequestSourceBranch = ref("");
const pullRequestTargetBranch = ref("");
const sourceBranchSearch = ref("");
const targetBranchSearch = ref("");
const issueTitle = ref("");
const issueDescription = ref("");
const issueLabelNames = ref<string[]>([]);
const issueAssignees = ref<string[]>([]);
const issueMilestone = ref<number | null>(null);
const pullRequestLabelNames = ref<string[]>([]);
const pullRequestAssignees = ref<string[]>([]);
const pullRequestReviewers = ref<string[]>([]);
const pullRequestMilestone = ref<number | null>(null);

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

const createLabels = computed(() => props.createOptions?.labels || []);
const createMilestones = computed(() => props.createOptions?.milestones || []);
const createAssignees = computed(() => props.createOptions?.assignees || []);
const createReviewers = computed(() => props.createOptions?.reviewers || []);

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
  pullRequestLabelNames.value = [];
  pullRequestAssignees.value = [];
  pullRequestReviewers.value = [];
  pullRequestMilestone.value = null;
}

function initIssueDraft() {
  issueTitle.value = "";
  issueDescription.value = "";
  issueLabelNames.value = [];
  issueAssignees.value = [];
  issueMilestone.value = null;
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
    labels: pullRequestLabelNames.value,
    assignees: pullRequestAssignees.value,
    reviewers: pullRequestReviewers.value,
    milestone: selectedMilestoneNumber(pullRequestMilestone.value),
  });
}

function submitIssue() {
  if (!canSubmitIssue.value) return;
  emit("createIssue", {
    title: issueTitle.value.trim(),
    description: issueDescription.value.trim(),
    labels: issueLabelNames.value,
    assignees: issueAssignees.value,
    milestone: selectedMilestoneNumber(issueMilestone.value),
  });
}

function toggleValue(values: string[], value: string): string[] {
  return values.includes(value)
    ? values.filter((item) => item !== value)
    : [...values, value];
}

function toggleIssueLabel(name: string) {
  issueLabelNames.value = toggleValue(issueLabelNames.value, name);
}

function togglePullRequestLabel(name: string) {
  pullRequestLabelNames.value = toggleValue(pullRequestLabelNames.value, name);
}

function toggleIssueAssignee(login: string) {
  issueAssignees.value = toggleValue(issueAssignees.value, login);
}

function togglePullRequestAssignee(login: string) {
  pullRequestAssignees.value = toggleValue(pullRequestAssignees.value, login);
}

function togglePullRequestReviewer(login: string) {
  pullRequestReviewers.value = toggleValue(pullRequestReviewers.value, login);
}

function selectedMilestoneNumber(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

function openInBrowser(url: string) {
  if (!url) return;
  openUrl(url).catch(() => {});
}

function listLabel(values?: string[]): string {
  return values && values.length > 0 ? values.join(", ") : "None";
}

function hasValues(values?: string[]): boolean {
  return Array.isArray(values) && values.length > 0;
}

function pullRequestStateLabel(pullRequest: PullRequestInfo): string {
  if (pullRequest.merged) return "Merged";
  if (pullRequest.draft) return "Draft";
  return pullRequest.state || "unknown";
}

function issueStateClass(state: string): string {
  return state.toLowerCase() === "open"
    ? "border-[#10b981]/35 bg-[#10b981]/12 text-[#34d399]"
    : "border-[#8b5cf6]/35 bg-[#8b5cf6]/12 text-[#a78bfa]";
}

function pullRequestStateClass(pullRequest: PullRequestInfo): string {
  if (pullRequest.merged) return "border-[#8b5cf6]/35 bg-[#8b5cf6]/12 text-[#a78bfa]";
  if (pullRequest.draft) return "border-[#f59e0b]/35 bg-[#f59e0b]/12 text-[#f59e0b]";
  return pullRequest.state.toLowerCase() === "open"
    ? "border-[#10b981]/35 bg-[#10b981]/12 text-[#34d399]"
    : "border-[var(--border)] bg-[var(--secondary)] text-[var(--muted-foreground)]";
}

function mergeStateLabel(pullRequest: PullRequestInfo): string {
  if (pullRequest.merged) {
    const actor = pullRequest.mergedBy ? ` by ${pullRequest.mergedBy}` : "";
    const when = pullRequest.mergedAt ? ` on ${formatDate(pullRequest.mergedAt)}` : "";
    return `Merged${actor}${when}`;
  }

  if (pullRequest.mergeableState) return pullRequest.mergeableState;
  if (pullRequest.mergeable === null) return "Merge status unknown";
  return pullRequest.mergeable ? "Ready to merge" : "Blocked";
}

function fileStatusClass(status: string): string {
  const normalized = status.toLowerCase();
  if (normalized === "added") return "text-[#10b981] bg-[#10b981]/10 border-[#10b981]/25";
  if (normalized === "removed" || normalized === "deleted") return "text-[#ef4444] bg-[#ef4444]/10 border-[#ef4444]/25";
  if (normalized === "renamed") return "text-[#f59e0b] bg-[#f59e0b]/10 border-[#f59e0b]/25";
  return "text-[var(--muted-foreground)] bg-[var(--secondary)] border-[var(--border)]";
}

function shortSha(value?: string): string {
  return value ? value.slice(0, 7) : "";
}

function milestoneProgress(milestone?: RemoteMilestoneInfo | null): string {
  if (!milestone) return "No milestone";
  const total = milestone.openIssues + milestone.closedIssues;
  if (total <= 0) return `${milestone.title} · no issues`;
  const percent = Math.round((milestone.closedIssues / total) * 100);
  return `${milestone.title} · ${percent}% complete`;
}

function labelStyle(label: RemoteLabelInfo): Record<string, string> {
  const color = label.color || "64748b";
  return {
    borderColor: `#${color}66`,
    backgroundColor: `#${color}22`,
    color: `#${color}`,
  };
}

function referenceTitle(reference: RemoteReferenceInfo): string {
  if (reference.kind === "commit") {
    return `${shortSha(reference.sha)} · ${reference.title}`;
  }

  return `#${reference.number} · ${reference.title}`;
}

function referenceKindLabel(reference: RemoteReferenceInfo): string {
  if (reference.kind === "pull_request") return "PR";
  if (reference.kind === "issue") return "Issue";
  return "Commit";
}

function openReference(reference: RemoteReferenceInfo) {
  openInBrowser(reference.url);
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

      <CloseIconButton size="sm" title="Back to Git Graph" @click="emit('close')" />
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto p-4">
      <div v-if="mode === 'pull-request-detail' && pullRequest" class="space-y-4">
        <div class="rounded border border-[var(--border)] bg-[var(--card)] p-4 space-y-3">
          <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
            <div class="min-w-0 flex-1">
              <div class="text-lg text-[var(--foreground)] font-semibold leading-tight">{{ pullRequest.title }}</div>
              <div class="mt-2 flex flex-wrap items-center gap-1.5 text-[11px] text-[var(--muted-foreground)]">
                <span class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 uppercase tracking-wide" :class="pullRequestStateClass(pullRequest)">
                  <GitPullRequest class="h-3 w-3" />
                  {{ pullRequestStateLabel(pullRequest) }}
                </span>
                <span class="text-[var(--muted-foreground)]">#{{ pullRequest.number }}</span>
              </div>
            </div>
            <div class="inline-flex max-w-full items-center gap-1.5 rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[10px]">
              <GitBranch class="h-3 w-3 flex-shrink-0 text-[var(--primary)]" />
              <span class="min-w-0 truncate font-mono text-[var(--primary)]">{{ pullRequest.sourceBranch }}</span>
              <span class="text-[var(--muted-foreground)]">-></span>
              <span class="min-w-0 truncate font-mono text-[var(--foreground)]">{{ pullRequest.targetBranch }}</span>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-2 text-[11px] text-[var(--muted-foreground)]">
            <span v-if="props.detailLoading" class="inline-flex items-center gap-1 text-[var(--muted-foreground)]">
              <Loader2 class="h-3 w-3 animate-spin" />
              Loading details
            </span>
          </div>
        </div>

        <div class="rounded border border-[var(--border)] bg-[var(--card)] divide-y divide-[var(--border)]">
          <div class="flex flex-col gap-1 px-3 py-2.5 sm:flex-row sm:items-center">
            <div class="w-28 flex-shrink-0 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Branches</div>
            <div class="min-w-0 flex-1 text-xs text-[var(--foreground)]">
              <span class="font-mono text-[var(--primary)]">{{ pullRequest.sourceBranch }}</span>
              <span class="mx-2 text-[var(--muted-foreground)]">-></span>
              <span class="font-mono">{{ pullRequest.targetBranch }}</span>
            </div>
          </div>
          <div class="flex flex-col gap-1 px-3 py-2.5 sm:flex-row sm:items-center">
            <div class="w-28 flex-shrink-0 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">People</div>
            <div class="min-w-0 flex-1 flex flex-wrap gap-x-3 gap-y-1 text-xs text-[var(--foreground)]">
              <span>{{ pullRequest.author }} opened</span>
              <span v-if="hasValues(pullRequest.assignees)" class="text-[var(--muted-foreground)]">Assignees: <span class="text-[var(--foreground)]">{{ listLabel(pullRequest.assignees) }}</span></span>
              <span v-if="hasValues(pullRequest.requestedReviewers)" class="text-[var(--muted-foreground)]">Reviewers: <span class="text-[var(--foreground)]">{{ listLabel(pullRequest.requestedReviewers) }}</span></span>
              <span v-if="!hasValues(pullRequest.assignees) && !hasValues(pullRequest.requestedReviewers)" class="text-[var(--muted-foreground)]">No assignees or reviewers yet</span>
            </div>
          </div>
          <div class="flex flex-col gap-1 px-3 py-2.5 sm:flex-row sm:items-center">
            <div class="w-28 flex-shrink-0 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Status</div>
            <div class="min-w-0 flex-1 text-xs text-[var(--foreground)]">
              {{ mergeStateLabel(pullRequest) }}
              <span class="text-[var(--muted-foreground)]"> · updated {{ formatDate(pullRequest.updatedAt) }}</span>
            </div>
          </div>
          <div class="flex flex-col gap-1 px-3 py-2.5 sm:flex-row">
            <div class="w-28 flex-shrink-0 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Milestone</div>
            <div class="min-w-0 flex-1 text-xs text-[var(--foreground)]">{{ milestoneProgress(pullRequest.milestone) }}</div>
          </div>
          <div class="flex flex-col gap-1 px-3 py-2.5 sm:flex-row">
            <div class="w-28 flex-shrink-0 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Labels</div>
            <div v-if="pullRequest.labels && pullRequest.labels.length > 0" class="min-w-0 flex flex-1 flex-wrap gap-1.5">
              <span
                v-for="label in pullRequest.labels"
                :key="label.name"
                class="rounded-full border px-2 py-0.5 text-[10px] font-medium"
                :style="labelStyle(label)"
                :title="label.description"
              >
                {{ label.name }}
              </span>
            </div>
            <div v-else class="text-xs text-[var(--muted-foreground)]">None</div>
          </div>
        </div>

        <div class="rounded border border-[var(--border)] bg-[var(--card)] px-3 py-2.5">
          <div class="flex flex-wrap items-center gap-2 text-[11px] text-[var(--muted-foreground)]">
            <span class="inline-flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1"><GitCommitHorizontal class="h-3 w-3" /><strong class="text-[var(--foreground)]">{{ pullRequest.commitsCount ?? '-' }}</strong> commits</span>
            <span class="inline-flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1"><Files class="h-3 w-3" /><strong class="text-[var(--foreground)]">{{ pullRequest.changedFiles ?? '-' }}</strong> files</span>
            <span class="rounded border border-[#10b981]/20 bg-[#10b981]/10 px-2 py-1"><strong class="text-[#10b981]">+{{ pullRequest.additions ?? 0 }}</strong> added</span>
            <span class="rounded border border-[#ef4444]/20 bg-[#ef4444]/10 px-2 py-1"><strong class="text-[#ef4444]">-{{ pullRequest.deletions ?? 0 }}</strong> deleted</span>
            <span class="inline-flex items-center gap-1 rounded border border-[var(--border)] bg-[var(--secondary)] px-2 py-1"><MessageSquare class="h-3 w-3" /><strong class="text-[var(--foreground)]">{{ (pullRequest.comments ?? 0) + (pullRequest.reviewComments ?? 0) }}</strong> comments</span>
          </div>
        </div>

        <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Description</div>
          <div class="text-xs text-[var(--foreground)] whitespace-pre-wrap leading-relaxed">{{ pullRequest.description || 'No description provided.' }}</div>
        </div>

        <div v-if="pullRequest.linkedIssues && pullRequest.linkedIssues.length > 0" class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-2">Linked issues</div>
          <button
            v-for="reference in pullRequest.linkedIssues"
            :key="`pr-issue-${reference.number}`"
            class="w-full rounded px-2 py-1.5 text-left text-[11px] text-[var(--foreground)] transition-colors hover:bg-[var(--secondary)]"
            @click="openReference(reference)"
          >
            <span class="font-semibold">{{ referenceKindLabel(reference) }}</span>
            <span class="ml-2">{{ referenceTitle(reference) }}</span>
          </button>
        </div>

        <div v-if="pullRequest.linkedCommits && pullRequest.linkedCommits.length > 0" class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-2">Commits</div>
          <button
            v-for="reference in pullRequest.linkedCommits"
            :key="`pr-commit-${reference.sha}`"
            class="w-full rounded px-2 py-1.5 text-left text-[11px] text-[var(--foreground)] transition-colors hover:bg-[var(--secondary)]"
            @click="openReference(reference)"
          >
            <span class="font-mono text-[var(--primary)]">{{ shortSha(reference.sha) }}</span>
            <span class="ml-2">{{ reference.title }}</span>
          </button>
        </div>

        <div v-if="pullRequest.files && pullRequest.files.length > 0" class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-2">Changed files preview</div>
          <button
            v-for="file in pullRequest.files"
            :key="file.filename"
            class="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-[11px] transition-colors hover:bg-[var(--secondary)]"
            @click="openInBrowser(file.url)"
          >
            <span class="w-16 rounded border px-1.5 py-0.5 text-center text-[9px] uppercase tracking-wide" :class="fileStatusClass(file.status)">{{ file.status }}</span>
            <span class="min-w-0 flex-1 truncate text-[var(--foreground)]">{{ file.filename }}</span>
            <span class="font-mono text-[#10b981]">+{{ file.additions }}</span>
            <span class="font-mono text-[#ef4444]">-{{ file.deletions }}</span>
          </button>
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
          <div class="mt-2 flex flex-wrap items-center gap-2 text-[11px] text-[var(--muted-foreground)]">
            <span class="rounded-full border px-2 py-0.5 uppercase tracking-wide" :class="issueStateClass(issue.state)">{{ issue.state }}</span>
            <span v-if="issue.stateReason" class="rounded-full border border-[var(--border)] px-2 py-0.5">{{ issue.stateReason }}</span>
            <span v-if="props.detailLoading" class="inline-flex items-center gap-1 text-[var(--muted-foreground)]">
              <Loader2 class="h-3 w-3 animate-spin" />
              Loading details
            </span>
          </div>
        </div>

        <div class="rounded border border-[var(--border)] bg-[var(--card)] divide-y divide-[var(--border)]">
          <div class="flex flex-col gap-1 px-3 py-2.5 sm:flex-row sm:items-center">
            <div class="w-28 flex-shrink-0 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">People</div>
            <div class="min-w-0 flex-1 text-xs text-[var(--foreground)]">
              {{ issue.author }} opened, assigned to {{ listLabel(issue.assignees) }}
            </div>
          </div>
          <div class="flex flex-col gap-1 px-3 py-2.5 sm:flex-row sm:items-center">
            <div class="w-28 flex-shrink-0 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Activity</div>
            <div class="min-w-0 flex-1 text-xs text-[var(--foreground)]">
              Updated {{ formatDate(issue.updatedAt) }}
              <span v-if="issue.closedAt" class="text-[var(--muted-foreground)]"> · closed {{ formatDate(issue.closedAt) }}</span>
            </div>
          </div>
          <div class="flex flex-col gap-1 px-3 py-2.5 sm:flex-row sm:items-center">
            <div class="w-28 flex-shrink-0 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Milestone</div>
            <div class="min-w-0 flex-1 text-xs text-[var(--foreground)]">{{ milestoneProgress(issue.milestone) }}</div>
          </div>
          <div class="flex flex-col gap-1 px-3 py-2.5 sm:flex-row">
            <div class="w-28 flex-shrink-0 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Discussion</div>
            <div class="min-w-0 flex-1 flex flex-col gap-2 md:flex-row md:items-start">
              <div class="w-24 flex-shrink-0 text-xs text-[var(--foreground)]">
                {{ issue.comments ?? 0 }} comments
              </div>
              <div v-if="issue.labels && issue.labels.length > 0" class="min-w-0 flex flex-1 flex-wrap gap-1.5">
                <span
                  v-for="label in issue.labels"
                  :key="label.name"
                  class="rounded-full border px-2 py-0.5 text-[10px] font-medium"
                  :style="labelStyle(label)"
                  :title="label.description"
                >
                  {{ label.name }}
                </span>
              </div>
              <div v-else class="text-xs text-[var(--muted-foreground)]">No labels</div>
            </div>
          </div>
        </div>

        <div class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-1">Description</div>
          <div class="text-xs text-[var(--foreground)] whitespace-pre-wrap leading-relaxed">{{ issue.description || 'No description provided.' }}</div>
        </div>

        <div v-if="issue.linkedPullRequests && issue.linkedPullRequests.length > 0" class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-2">Linked pull requests</div>
          <button
            v-for="reference in issue.linkedPullRequests"
            :key="`issue-pr-${reference.number}`"
            class="w-full rounded px-2 py-1.5 text-left text-[11px] text-[var(--foreground)] transition-colors hover:bg-[var(--secondary)]"
            @click="openReference(reference)"
          >
            <span class="font-semibold">{{ referenceKindLabel(reference) }}</span>
            <span class="ml-2">{{ referenceTitle(reference) }}</span>
            <span v-if="reference.state" class="ml-2 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">{{ reference.state }}</span>
          </button>
        </div>

        <div v-if="issue.linkedCommits && issue.linkedCommits.length > 0" class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-2">Tagged commits</div>
          <button
            v-for="reference in issue.linkedCommits"
            :key="`issue-commit-${reference.sha}`"
            class="w-full rounded px-2 py-1.5 text-left text-[11px] text-[var(--foreground)] transition-colors hover:bg-[var(--secondary)]"
            @click="openReference(reference)"
          >
            <span class="font-mono text-[var(--primary)]">{{ shortSha(reference.sha) }}</span>
            <span v-if="reference.author" class="ml-2 text-[var(--muted-foreground)]">{{ reference.author }}</span>
          </button>
        </div>

        <div v-if="issue.linkedIssues && issue.linkedIssues.length > 0" class="rounded border border-[var(--border)] bg-[var(--card)] p-3">
          <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] mb-2">Related issues</div>
          <button
            v-for="reference in issue.linkedIssues"
            :key="`issue-related-${reference.number}`"
            class="w-full rounded px-2 py-1.5 text-left text-[11px] text-[var(--foreground)] transition-colors hover:bg-[var(--secondary)]"
            @click="openReference(reference)"
          >
            <span class="font-semibold">{{ referenceKindLabel(reference) }}</span>
            <span class="ml-2">{{ referenceTitle(reference) }}</span>
          </button>
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

          <div class="flex flex-col gap-3 lg:flex-row">
            <div class="space-y-1.5 lg:flex-1">
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

            <div class="space-y-1.5 lg:flex-1">
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

          <div class="border-t border-[var(--border)] pt-3 space-y-3">
            <div class="flex items-center justify-between gap-2">
              <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Metadata</div>
              <span v-if="props.createOptionsLoading" class="inline-flex items-center gap-1 text-[10px] text-[var(--muted-foreground)]">
                <Loader2 class="h-3 w-3 animate-spin" />
                Loading GitHub options
              </span>
            </div>

            <div v-if="createLabels.length > 0" class="space-y-1.5">
              <div class="text-[10px] text-[var(--muted-foreground)]">Labels</div>
              <div class="flex flex-wrap gap-1.5">
                <button
                  v-for="label in createLabels"
                  :key="'pr-label-' + label.name"
                  class="rounded-full border px-2 py-0.5 text-[10px] font-medium transition-opacity"
                  :class="pullRequestLabelNames.includes(label.name) ? 'opacity-100 ring-1 ring-[var(--ring)]' : 'opacity-55 hover:opacity-90'"
                  :style="labelStyle(label)"
                  @click="togglePullRequestLabel(label.name)"
                >
                  {{ label.name }}
                </button>
              </div>
            </div>

            <div class="flex flex-col gap-3 lg:flex-row">
              <div v-if="createAssignees.length > 0" class="space-y-1.5 lg:flex-1">
                <div class="text-[10px] text-[var(--muted-foreground)]">Assignees</div>
                <div class="flex max-h-24 flex-wrap gap-1.5 overflow-y-auto">
                  <button
                    v-for="user in createAssignees"
                    :key="'pr-assignee-' + user.login"
                    class="rounded border px-2 py-0.5 text-[10px] transition-colors"
                    :class="pullRequestAssignees.includes(user.login) ? 'border-[var(--primary)] bg-[var(--primary)]/12 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                    @click="togglePullRequestAssignee(user.login)"
                  >
                    {{ user.login }}
                  </button>
                </div>
              </div>

              <div v-if="createReviewers.length > 0" class="space-y-1.5 lg:flex-1">
                <div class="text-[10px] text-[var(--muted-foreground)]">Reviewers</div>
                <div class="flex max-h-24 flex-wrap gap-1.5 overflow-y-auto">
                  <button
                    v-for="user in createReviewers"
                    :key="'pr-reviewer-' + user.login"
                    class="rounded border px-2 py-0.5 text-[10px] transition-colors"
                    :class="pullRequestReviewers.includes(user.login) ? 'border-[var(--primary)] bg-[var(--primary)]/12 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                    @click="togglePullRequestReviewer(user.login)"
                  >
                    {{ user.login }}
                  </button>
                </div>
              </div>

              <div v-if="createMilestones.length > 0" class="space-y-1.5 lg:flex-1">
                <div class="text-[10px] text-[var(--muted-foreground)]">Milestone</div>
                <select
                  v-model.number="pullRequestMilestone"
                  class="h-8 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 text-xs text-[var(--foreground)] outline-none"
                >
                  <option :value="null">No milestone</option>
                  <option v-for="milestone in createMilestones" :key="'pr-ms-' + milestone.number" :value="milestone.number">
                    {{ milestone.title }}
                  </option>
                </select>
              </div>
            </div>
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

          <div class="border-t border-[var(--border)] pt-3 space-y-3">
            <div class="flex items-center justify-between gap-2">
              <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Metadata</div>
              <span v-if="props.createOptionsLoading" class="inline-flex items-center gap-1 text-[10px] text-[var(--muted-foreground)]">
                <Loader2 class="h-3 w-3 animate-spin" />
                Loading GitHub options
              </span>
            </div>

            <div v-if="createLabels.length > 0" class="space-y-1.5">
              <div class="text-[10px] text-[var(--muted-foreground)]">Labels</div>
              <div class="flex flex-wrap gap-1.5">
                <button
                  v-for="label in createLabels"
                  :key="'issue-label-' + label.name"
                  class="rounded-full border px-2 py-0.5 text-[10px] font-medium transition-opacity"
                  :class="issueLabelNames.includes(label.name) ? 'opacity-100 ring-1 ring-[var(--ring)]' : 'opacity-55 hover:opacity-90'"
                  :style="labelStyle(label)"
                  @click="toggleIssueLabel(label.name)"
                >
                  {{ label.name }}
                </button>
              </div>
            </div>

            <div class="flex flex-col gap-3 lg:flex-row">
              <div v-if="createAssignees.length > 0" class="space-y-1.5 lg:flex-1">
                <div class="text-[10px] text-[var(--muted-foreground)]">Assignees</div>
                <div class="flex max-h-24 flex-wrap gap-1.5 overflow-y-auto">
                  <button
                    v-for="user in createAssignees"
                    :key="'issue-assignee-' + user.login"
                    class="rounded border px-2 py-0.5 text-[10px] transition-colors"
                    :class="issueAssignees.includes(user.login) ? 'border-[var(--primary)] bg-[var(--primary)]/12 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                    @click="toggleIssueAssignee(user.login)"
                  >
                    {{ user.login }}
                  </button>
                </div>
              </div>

              <div v-if="createMilestones.length > 0" class="space-y-1.5 lg:flex-1">
                <div class="text-[10px] text-[var(--muted-foreground)]">Milestone</div>
                <select
                  v-model.number="issueMilestone"
                  class="h-8 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 text-xs text-[var(--foreground)] outline-none"
                >
                  <option :value="null">No milestone</option>
                  <option v-for="milestone in createMilestones" :key="'issue-ms-' + milestone.number" :value="milestone.number">
                    {{ milestone.title }}
                  </option>
                </select>
              </div>
            </div>
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

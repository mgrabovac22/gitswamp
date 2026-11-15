<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ArrowLeft, ArrowRight, Check, CircleDot, ExternalLink, Files, GitCommitHorizontal, GitMerge, GitPullRequest, Loader2, MessageSquare } from "lucide-vue-next";
import BranchCombobox from "@/features/repository/remote/BranchCombobox.vue";
import RemoteConversation from "@/features/repository/remote/RemoteConversation.vue";
import { branchOnRemote, fetchedBranches, recommendedBase } from "@/features/repository/remote/branchOptions";
import type { BranchComparison, RemoteBranchCatalog, RemoteDiscussionClient } from "@/features/repository/remote/githubRemote";
import type { WorkflowConfig } from "@/features/repository/workflow/branchWorkflow";
import type { BranchInfo, IssueInfo, PullRequestInfo, RemoteIssueCreatePayload, RemotePullRequestCreatePayload, RemoteLabelInfo, RemoteMilestoneInfo, RemoteUserInfo } from "@/types";

interface CreateOptions { labels: RemoteLabelInfo[]; milestones: RemoteMilestoneInfo[]; assignees: RemoteUserInfo[]; reviewers: RemoteUserInfo[] }
type Tab = "conversation" | "commits" | "files" | "review";

const props = defineProps<{
  mode: "pull-request-detail" | "pull-request-create" | "issue-detail" | "issue-create";
  pullRequest: PullRequestInfo | null;
  issue: IssueInfo | null;
  detailLoading?: boolean;
  createOptions?: CreateOptions;
  createOptionsLoading?: boolean;
  creating?: boolean;
  workflowDraft?: { source: string; target: string; description?: string } | null;
  workflow?: WorkflowConfig | null;
  remoteBranches: BranchInfo[];
  remoteName: string;
  currentBranch: string;
  currentUpstream?: string;
  branchCatalog?: RemoteBranchCatalog | null;
  branchCatalogLoading?: boolean;
  branchCatalogError?: boolean;
  compareBranches?: (base: string, head: string, signal: AbortSignal) => Promise<BranchComparison>;
  discussion?: RemoteDiscussionClient;
}>();

const emit = defineEmits<{
  close: [];
  createPullRequest: [payload: RemotePullRequestCreatePayload];
  createIssue: [payload: RemoteIssueCreatePayload];
}>();

const isPull = computed(() => props.mode.startsWith("pull-request"));
const isCreate = computed(() => props.mode.endsWith("-create"));
const item = computed(() => isPull.value ? props.pullRequest : props.issue);
const scrollArea = ref<HTMLElement>();
const tab = ref<Tab>("conversation");
const consulted = ref(false);
const title = ref("");
const titleEdited = ref(false);
const body = ref("");
const source = ref("");
const target = ref("");
const chosenSource = ref(false);
const chosenTarget = ref(false);
const labels = ref<string[]>([]);
const assignees = ref<string[]>([]);
const reviewers = ref<string[]>([]);
const milestone = ref<number | null>(null);
const comparison = ref<BranchComparison>();
const comparing = ref(false);
const comparisonUnavailable = ref(false);

const branches = computed(() => props.branchCatalog?.names ?? fetchedBranches(props.remoteBranches, props.remoteName));
const defaultBranch = computed(() => props.branchCatalog?.defaultBranch || "");
const currentRemoteBranch = computed(() => {
  const upstream = branchOnRemote(props.currentUpstream ?? "", props.remoteName);
  return [upstream, props.currentBranch].find(name => branches.value.includes(name)) ?? "";
});
const recommendation = computed(() => recommendedBase(source.value, branches.value, defaultBranch.value, props.workflow));
const displayedTitle = computed(() => titleEdited.value || !isPull.value ? title.value :
  source.value && target.value ? `Merge ${source.value} into ${target.value}` : "");
const unavailablePair = computed(() => !!source.value && !!target.value && props.branchCatalog != null &&
  !(branches.value.includes(source.value) && branches.value.includes(target.value)));
const canSubmit = computed(() => !props.creating && !!displayedTitle.value.trim() && (!isPull.value ||
  (!!source.value && !!target.value && source.value !== target.value && !unavailablePair.value && !comparing.value && comparison.value?.ahead !== 0)));

watch([() => props.mode, () => props.workflowDraft, () => props.discussion?.key], () => {
  if (!isCreate.value) return;
  title.value = ""; titleEdited.value = false;
  body.value = isPull.value ? props.workflowDraft?.description || "" : "";
  source.value = isPull.value ? props.workflowDraft?.source || currentRemoteBranch.value : "";
  target.value = isPull.value ? props.workflowDraft?.target || recommendation.value?.branch || "" : "";
  chosenSource.value = isPull.value && !!props.workflowDraft?.source;
  chosenTarget.value = isPull.value && !!props.workflowDraft?.target;
  labels.value = []; assignees.value = []; reviewers.value = []; milestone.value = null;
}, { immediate: true });

watch([currentRemoteBranch, recommendation], () => {
  if (props.mode !== "pull-request-create") return;
  if (!chosenSource.value) source.value = currentRemoteBranch.value === target.value && chosenTarget.value ? "" : currentRemoteBranch.value;
  if (!chosenTarget.value) target.value = recommendation.value?.branch || "";
});

watch([() => props.mode, () => item.value?.number, () => props.discussion?.key], () => {
  tab.value = "conversation";
  consulted.value = false;
  if (scrollArea.value) scrollArea.value.scrollTop = 0;
});

watch([() => props.mode, source, target, unavailablePair, () => props.compareBranches], ([mode, head, base, unavailable, compare], _, cleanup) => {
  comparison.value = undefined;
  comparing.value = comparisonUnavailable.value = false;
  if (mode !== "pull-request-create" || !compare || !head || !base || head === base || unavailable) return;
  const controller = new AbortController();
  comparing.value = true;
  const timer = setTimeout(async () => {
    try {
      const result = await compare(base, head, controller.signal);
      if (!controller.signal.aborted) comparison.value = result;
    } catch {
      if (!controller.signal.aborted) comparisonUnavailable.value = true;
    } finally {
      if (!controller.signal.aborted) comparing.value = false;
    }
  }, 250);
  cleanup(() => { clearTimeout(timer); controller.abort(); });
}, { immediate: true });

function chooseSource(branch: string) {
  chosenSource.value = true;
  source.value = branch;
  if (!chosenTarget.value) target.value = recommendation.value?.branch || "";
}
function chooseTarget(branch: string) { chosenTarget.value = true; target.value = branch; }
function chooseTab(value: Tab) { tab.value = value; if (value === "review") consulted.value = true; }

function submit() {
  if (!canSubmit.value) return;
  const payload = { title: displayedTitle.value.trim(), description: body.value.trim(), labels: labels.value, assignees: assignees.value, milestone: milestone.value };
  if (isPull.value) emit("createPullRequest", { ...payload, sourceBranch: source.value, targetBranch: target.value, reviewers: reviewers.value });
  else emit("createIssue", payload);
}

function toggle(values: string[], name: string) { return values.includes(name) ? values.filter(value => value !== name) : [...values, name]; }
function openInBrowser(url: string) { if (/^https:\/\//i.test(url)) void openUrl(url).catch(() => {}); }
const state = computed(() => props.mode === "pull-request-detail" && props.pullRequest?.merged ? "merged"
  : props.mode === "pull-request-detail" && props.pullRequest?.draft && item.value?.state === "open" ? "draft" : item.value?.state ?? "");
const mergeStatus = computed(() => {
  const pr = props.pullRequest;
  if (!pr || pr.merged || pr.state !== "open") return "";
  if (pr.mergeable === false || pr.mergeableState === "dirty") return "Conflicts need attention";
  if (pr.mergeableState === "behind") return "Behind base branch";
  if (pr.mergeableState === "blocked") return "Blocked by repository requirements";
  if (pr.mergeableState === "unstable") return "Checks need attention";
  if (pr.mergeableState === "clean" && pr.mergeable === true) return "No conflicts reported by GitHub";
  return "Merge status not confirmed";
});
const related = computed(() => isPull.value ? props.pullRequest?.linkedIssues ?? [] : [
  ...(props.issue?.linkedPullRequests ?? []), ...(props.issue?.linkedIssues ?? []), ...(props.issue?.linkedCommits ?? []),
]);
const labelColor = (label: RemoteLabelInfo) => /^[a-f\d]{6}$/i.test(label.color) ? `#${label.color}` : "var(--muted-foreground)";
</script>

<template>
  <div class="remote-panel">
    <header class="toolbar">
      <button class="secondary" type="button" @click="emit('close')"><ArrowLeft :size="15" /> Back to graph</button>
      <span>{{ isPull ? 'Pull requests' : 'Issues' }}<template v-if="!isCreate && item"> / #{{ item.number }}</template></span>
      <button v-if="!isCreate && item?.url" type="button" class="secondary github" @click="openInBrowser(item.url)"><ExternalLink :size="14" /> Open on GitHub</button>
    </header>

    <div ref="scrollArea" class="remote-scroll">
      <form v-if="isCreate" class="remote-content" @submit.prevent="submit">
        <header class="heading">
          <h1>{{ isPull ? 'New pull request' : 'New issue' }}</h1>
          <div v-if="isPull" class="compare-branches">
            <BranchCombobox label="Base" :model-value="target" :branches="branches" :excluded="source" :preferred="[recommendation?.branch || '', defaultBranch]" :recommendation="recommendation" @update:model-value="chooseTarget" />
            <ArrowLeft class="compare-arrow" :size="15" aria-hidden="true" />
            <BranchCombobox label="Compare" :model-value="source" :branches="branches" :excluded="target" :preferred="[currentRemoteBranch, defaultBranch]"
              :recommendation="currentRemoteBranch ? { branch: currentRemoteBranch, reason: 'Current branch' } : null" @update:model-value="chooseSource" />
          </div>
          <div v-if="isPull" class="branch-status" role="status">
            <span v-if="branchCatalogLoading"><Loader2 :size="14" class="animate-spin" /> Loading GitHub branches</span>
            <span v-else-if="branchCatalogError">GitHub branches unavailable. Showing fetched branches for {{ remoteName }}.</span>
            <span v-else-if="!source && currentBranch && !currentRemoteBranch">Publish {{ currentBranch }} or select a published compare branch.</span>
            <span v-if="unavailablePair" class="error">One of the selected branches is no longer on GitHub. Select another branch.</span>
            <span v-else-if="comparing"><Loader2 :size="14" class="animate-spin" /> Comparing branches</span>
            <span v-else-if="comparison?.ahead === 0" class="error">The compare branch has no commits ahead of the base.</span>
            <span v-else-if="comparison"><Check :size="14" /> {{ comparison.ahead }} ahead · {{ comparison.behind }} behind</span>
            <span v-else-if="comparisonUnavailable">GitHub comparison unavailable.</span>
          </div>
        </header>

        <div class="columns">
          <div class="fields">
            <label>Title<input :value="displayedTitle" :disabled="creating" maxlength="256" required :placeholder="isPull ? 'Briefly describe this pull request' : 'What needs to be fixed or improved?'" @input="title = ($event.target as HTMLInputElement).value; titleEdited = true" /></label>
            <label>Description<textarea v-model="body" :disabled="creating" rows="9" maxlength="65536" :placeholder="isPull ? 'What changed and why?' : 'Add details, reproduction steps, or expected behavior'" /></label>
            <footer><button type="button" class="secondary" @click="emit('close')">Cancel</button><button class="primary" type="submit" :disabled="!canSubmit">
              <Loader2 v-if="creating" :size="15" class="animate-spin" /><component :is="isPull ? GitPullRequest : CircleDot" v-else :size="15" />{{ creating ? 'Creating…' : isPull ? 'Create pull request' : 'Create issue' }}
            </button></footer>
          </div>
          <aside class="metadata" aria-label="Metadata">
            <p v-if="createOptionsLoading" class="muted inline"><Loader2 :size="14" class="animate-spin" /> Loading GitHub options</p>
            <template v-else>
              <details v-if="isPull && createOptions?.reviewers.length"><summary>Reviewers <small>{{ reviewers.length || '' }}</small></summary><div class="choices"><label v-for="user in createOptions.reviewers" :key="user.login"><input type="checkbox" :checked="reviewers.includes(user.login)" @change="reviewers = toggle(reviewers, user.login)" />{{ user.login }}</label></div></details>
              <details v-if="createOptions?.assignees.length"><summary>Assignees <small>{{ assignees.length || '' }}</small></summary><div class="choices"><label v-for="user in createOptions.assignees" :key="user.login"><input type="checkbox" :checked="assignees.includes(user.login)" @change="assignees = toggle(assignees, user.login)" />{{ user.login }}</label></div></details>
              <details v-if="createOptions?.labels.length"><summary>Labels <small>{{ labels.length || '' }}</small></summary><div class="choices"><label v-for="label in createOptions.labels" :key="label.name"><input type="checkbox" :checked="labels.includes(label.name)" @change="labels = toggle(labels, label.name)" /><span class="label-dot" :style="{ background: labelColor(label) }" />{{ label.name }}</label></div></details>
              <label v-if="createOptions?.milestones.length" class="milestone">Milestone<select v-model="milestone"><option :value="null">No milestone</option><option v-for="value in createOptions.milestones" :key="value.number" :value="value.number">{{ value.title }}</option></select></label>
            </template>
          </aside>
        </div>
      </form>

      <div v-else-if="item" class="remote-content">
        <header class="heading">
          <h1>{{ item.title }} <span>#{{ item.number }}</span></h1>
          <div class="summary">
            <span class="state" :data-state="state"><component :is="state === 'merged' ? GitMerge : isPull ? GitPullRequest : CircleDot" :size="15" />{{ state }}</span>
            <span v-if="isPull && pullRequest" class="route"><code>{{ pullRequest.sourceBranch }}</code><ArrowRight :size="14" /><code>{{ pullRequest.targetBranch }}</code></span>
            <span v-if="!isPull && issue?.stateReason" class="muted">{{ issue.stateReason.replace(/_/g, ' ') }}</span>
            <span v-if="mergeStatus" class="muted">{{ mergeStatus }}</span>
            <span v-if="detailLoading" class="muted inline"><Loader2 :size="14" class="animate-spin" /> Loading details</span>
          </div>
        </header>
        <nav v-if="isPull" aria-label="Pull request sections" class="tabs">
          <button type="button" :aria-pressed="tab === 'conversation'" @click="chooseTab('conversation')"><MessageSquare :size="15" /> Conversation</button>
          <button type="button" :aria-pressed="tab === 'commits'" @click="chooseTab('commits')"><GitCommitHorizontal :size="15" /> Commits <small v-if="pullRequest?.commitsCount != null">{{ pullRequest.commitsCount }}</small></button>
          <button type="button" :aria-pressed="tab === 'files'" @click="chooseTab('files')"><Files :size="15" /> Files changed <small v-if="pullRequest?.changedFiles != null">{{ pullRequest.changedFiles }}</small></button>
          <button type="button" :aria-pressed="tab === 'review'" @click="chooseTab('review')">Review comments <small v-if="pullRequest?.reviewComments">{{ pullRequest.reviewComments }}</small></button>
        </nav>
        <div class="columns">
          <div class="detail-content">
            <RemoteConversation v-show="tab === 'conversation'" :client="discussion" :number="item.number" kind="conversation" :author="item.author" :created-at="item.createdAt" :body="item.description" />
            <RemoteConversation v-if="consulted && isPull" v-show="tab === 'review'" :client="discussion" :number="item.number" kind="review" />
            <div v-if="tab === 'commits'" class="changes" aria-label="Commits">
              <button v-for="commit in pullRequest?.linkedCommits" :key="commit.sha" type="button" @click="openInBrowser(commit.url)"><code>{{ commit.sha?.slice(0, 7) }}</code><span>{{ commit.title }}</span><ExternalLink :size="13" /></button>
              <p v-if="!pullRequest?.linkedCommits?.length" class="muted">{{ detailLoading ? 'Loading commits…' : 'No commit preview available.' }}</p>
              <button v-if="pullRequest && (pullRequest.commitsCount ?? 0) > (pullRequest.linkedCommits?.length ?? 0)" type="button" @click="openInBrowser(pullRequest.url + '/commits')">View all commits on GitHub <ExternalLink :size="13" /></button>
            </div>
            <div v-if="tab === 'files'" class="changes" aria-label="Changed files">
              <p v-if="pullRequest" class="change-totals"><span class="positive">+{{ pullRequest.additions ?? 0 }}</span> <span class="negative">−{{ pullRequest.deletions ?? 0 }}</span></p>
              <button v-for="file in pullRequest?.files" :key="file.filename" type="button" @click="openInBrowser(file.url)"><small>{{ file.status }}</small><span>{{ file.filename }}</span><code class="positive">+{{ file.additions }}</code><code class="negative">−{{ file.deletions }}</code></button>
              <p v-if="!pullRequest?.files?.length" class="muted">{{ detailLoading ? 'Loading changed files…' : 'No file preview available.' }}</p>
              <button v-if="pullRequest && (pullRequest.changedFiles ?? 0) > (pullRequest.files?.length ?? 0)" type="button" @click="openInBrowser(pullRequest.url + '/files')">View all files on GitHub <ExternalLink :size="13" /></button>
            </div>
          </div>
          <aside class="metadata" aria-label="Metadata">
            <section v-if="isPull"><h2>Reviewers</h2><p :class="{ muted: !pullRequest?.requestedReviewers?.length }">{{ pullRequest?.requestedReviewers?.join(', ') || 'None requested' }}</p></section>
            <section><h2>Assignees</h2><p :class="{ muted: !item.assignees?.length }">{{ item.assignees?.join(', ') || 'No assignees' }}</p></section>
            <section><h2>Labels</h2><div v-if="item.labels?.length" class="labels"><span v-for="label in item.labels" :key="label.name" :title="label.description"><i class="label-dot" :style="{ background: labelColor(label) }" />{{ label.name }}</span></div><p v-else class="muted">No labels</p></section>
            <section><h2>Milestone</h2><p :class="{ muted: !item.milestone }">{{ item.milestone?.title || 'No milestone' }}</p><progress v-if="item.milestone && item.milestone.openIssues + item.milestone.closedIssues > 0" aria-label="Milestone completion" :max="item.milestone.openIssues + item.milestone.closedIssues" :value="item.milestone.closedIssues" /></section>
            <section v-if="related.length"><h2>Related work</h2><button v-for="link in related" :key="link.url" type="button" class="related" @click="openInBrowser(link.url)">{{ link.number ? `#${link.number}` : link.sha?.slice(0, 7) }} · {{ link.title }}</button></section>
          </aside>
        </div>
      </div>
      <p v-else class="remote-content muted" role="status">{{ detailLoading ? 'Loading details…' : 'Select a pull request or issue.' }}</p>
    </div>
  </div>
</template>

<style scoped>
.remote-panel { display: flex; flex-direction: column; min-width: 0; min-height: 0; height: 100%; background: var(--background); color: var(--foreground); font-size: 13px; container-type: inline-size; }
.toolbar { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 12px; min-height: 45px; padding: 6px 12px; border-bottom: 1px solid var(--border); color: var(--muted-foreground); }
.toolbar .github { margin-inline-start: auto; }
.remote-scroll { flex: 1; min-height: 0; overflow-y: auto; }
.remote-content { max-width: 1100px; margin: 0 auto; padding: 18px; }
.heading { border-bottom: 1px solid var(--border); padding-bottom: 12px; }
.heading h1 { font-size: 23px; font-weight: 650; line-height: 1.25; overflow-wrap: anywhere; }
.heading h1 span, .muted { color: var(--muted-foreground); }
.summary, .route, .inline, .compare-branches, .branch-status, .branch-status span, .fields footer { display: flex; align-items: center; gap: 7px; flex-wrap: wrap; }
.summary, .compare-branches { margin-top: 10px; }
.route code { padding: 2px 5px; background: var(--secondary); border-radius: 4px; overflow-wrap: anywhere; }
.state { display: inline-flex; align-items: center; gap: 5px; padding: 4px 7px; border-radius: 5px; background: var(--secondary); font-weight: 600; text-transform: capitalize; }
.state[data-state="open"] { background: color-mix(in srgb, #27955e 18%, var(--background)); border-inline-start: 3px solid #27955e; }
.state[data-state="merged"] { background: color-mix(in srgb, #8957d9 18%, var(--background)); border-inline-start: 3px solid #8957d9; }
.state[data-state="closed"] { border-inline-start: 3px solid var(--destructive); }
.compare-branches { display: grid; grid-template-columns: minmax(0, 1fr) 15px minmax(0, 1fr); max-width: 720px; }
.branch-status { margin-top: 7px; color: var(--muted-foreground); overflow-wrap: anywhere; }
.branch-status .error { color: var(--destructive); }
.columns { display: grid; grid-template-columns: minmax(0, 1fr) minmax(155px, 215px); gap: 22px; align-items: start; padding-top: 14px; }
.tabs { display: flex; flex-wrap: wrap; gap: 3px 8px; border-bottom: 1px solid var(--border); }
.tabs button { display: inline-flex; align-items: center; gap: 6px; min-height: 40px; padding: 4px 8px; border-bottom: 2px solid transparent; }
.tabs button[aria-pressed="true"] { border-bottom-color: var(--primary); font-weight: 650; }
.tabs small, .metadata small { color: var(--muted-foreground); }
.tabs button:hover, .secondary:hover, .changes button:hover { background: var(--secondary); }
.secondary, .primary { display: inline-flex; align-items: center; gap: 6px; min-height: 33px; padding: 4px 9px; border: 1px solid var(--border); border-radius: 6px; color: var(--foreground); }
.primary { background: var(--primary); color: var(--primary-foreground); }
.primary:hover:not(:disabled) { filter: brightness(.94); }
.primary:disabled { opacity: .5; }
.fields, .fields label, .milestone { display: grid; gap: 7px; }
.fields { gap: 13px; }
.fields label, .milestone { font-weight: 600; }
.fields label input, .fields label textarea, .metadata select { width: 100%; min-width: 0; border: 1px solid var(--border); border-radius: 6px; padding: 7px 9px; background: var(--input-background); font-weight: normal; }
.fields textarea { resize: vertical; }
.fields footer { justify-content: end; }
.metadata { min-width: 0; overflow-wrap: anywhere; }
.metadata section, .metadata details, .milestone { border-bottom: 1px solid var(--border); padding: 8px 0; }
.metadata h2 { font-weight: 650; margin-bottom: 4px; }
.metadata summary { cursor: pointer; font-weight: 600; }
.metadata summary small { margin-left: 5px; }
.choices { max-height: 165px; overflow: auto; padding: 4px 1px; }
.choices label, .labels span { display: flex; align-items: center; gap: 5px; padding: 3px 0; }
.labels { display: flex; flex-wrap: wrap; gap: 5px; }
.labels span { border: 1px solid var(--border); border-radius: 5px; padding: 2px 5px; }
.label-dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.related { display: block; text-align: start; padding: 4px 0; }
.related:hover { text-decoration: underline; }
.changes p, .changes button { padding: 8px; }
.changes button { display: flex; align-items: center; flex-wrap: wrap; gap: 6px 9px; width: 100%; text-align: start; border-bottom: 1px solid var(--border); overflow-wrap: anywhere; }
.changes button span { flex: 1; min-width: 120px; }
.changes button svg { flex-shrink: 0; }
.positive { color: color-mix(in srgb, #24985d 65%, var(--foreground)); }
.negative { color: var(--destructive); }
@container (max-width: 640px) { .columns { grid-template-columns: minmax(0, 1fr); gap: 12px; } .remote-content { padding: 12px; } }
@container (max-width: 430px) { .compare-branches { grid-template-columns: minmax(0, 1fr); } .compare-arrow { display: none; } .heading h1 { font-size: 19px; } }
</style>

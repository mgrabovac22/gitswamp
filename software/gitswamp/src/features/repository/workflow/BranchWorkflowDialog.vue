<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, useId } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, GitBranch, GitMerge, GitPullRequest, GitBranchPlus, Loader2, Settings2, Tag, Trash2 } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import HandoffReview from "./HandoffReview.vue";
import WorkflowSetup from "./WorkflowSetup.vue";
import {
  rememberWorkflowPath, saveWorkflow, workflowName, workflowPreference, workflowSources,
  type HandoffDraft, type WorkflowAction, type WorkflowConfig, type WorkflowInspection,
  type WorkflowOutcome, type WorkflowPlan, type WorkflowRequest,
} from "./branchWorkflow";

const props = defineProps<{ repoPath: string; initial?: WorkflowInspection; canOpenPullRequest: boolean; remoteName?: string; fetchRefs?: () => Promise<void> }>();
const emit = defineEmits<{
  close: []; saved: []; outcome: [result: WorkflowOutcome]; pullRequest: [draft: HandoffDraft];
}>();
const dialog = ref<HTMLDialogElement>();
const info = shallowRef<WorkflowInspection>();
const config = shallowRef<WorkflowConfig | null>(null);
const request = ref<WorkflowRequest>();
const plan = shallowRef<WorkflowPlan>();
const outcome = shallowRef<WorkflowOutcome>();
const setup = ref(false);
const busy = ref(false);
const error = ref("");
const note = ref("");
const sources = computed(() => config.value && info.value ? workflowSources(config.value, info.value.local) : []);
const branchNames = computed(() => info.value ? [...new Set([...info.value.local, ...info.value.remote])].sort() : []);
const formatAction: Record<WorkflowAction, string> = {
  feature: "Start feature", fix: "Start fix", release: "Start release", hotfix: "Start hotfix",
  support: "Start maintenance branch", "prepare-primary": "Create local primary branch",
  "prepare-development": "Create development branch", finish: "Finish branch", cleanup: "Delete merged branch",
  tag: "Create release tag", handoff: "Review pull request handoff",
};
type HintAction = WorkflowAction | "manage";
const hintId = useId();
const hint = shallowRef<{ action: HintAction; x: number; y: number; above: boolean }>();
const hints = computed<Record<HintAction, string>>(() => {
  const c = config.value;
  const primary = c?.primary ? `'${c.primary}'` : "the primary branch";
  const development = c?.development ? `'${c.development}'` : "the development branch";
  const base = c?.strategy === "gitflow" ? development : primary;
  return {
    feature: `Create and switch to a feature branch based on ${base}.`,
    fix: `Create and switch to a bugfix branch based on ${primary}.`,
    release: `Create and switch to a release branch from ${base} to prepare a new version.`,
    hotfix: `Create and switch to a hotfix branch from ${primary} for an urgent production fix.`,
    support: `Create and switch to a maintenance branch from a release tag, or from ${primary} if no tag is entered.`,
    "prepare-primary": `Create a local ${primary} from its remote branch, without switching branches.`,
    "prepare-development": `Create ${development} from its remote branch if available, or from ${primary}. Stay on the current branch.`,
    finish: c?.strategy === "gitflow"
      ? `Merge a feature into ${development}. Releases and hotfixes merge into ${primary} and ${development}, with an optional tag.`
      : `Preview a fast-forward merge into ${primary}. You can also remove the local source after integration.`,
    cleanup: "Delete the local branch only if it is fully merged and has no remote counterpart.",
    tag: `Create a local version tag on the latest commit of ${primary}. It is not pushed automatically.`,
    handoff: `Review changes and possible merge conflicts before opening a pull request into ${primary}.`,
    manage: "Change the active strategy, its base branches and its naming prefixes.",
  };
});
const actions = computed(() => {
  const c = config.value;
  const repo = info.value;
  if (!c || c.strategy === "manual" || !repo || repo.blocked) return [] as WorkflowAction[];
  if (!repo.local.includes(c.primary)) return repo.remote.includes(c.primary) ? ["prepare-primary"] as WorkflowAction[] : [];
  if (c.strategy === "gitflow" && !repo.local.includes(c.development)) return ["prepare-development"] as WorkflowAction[];
  const items: WorkflowAction[] = [];
  const pr = c.strategy === "github" || c.strategy === "swamp";
  if (repo.current && sources.value.includes(repo.current)) items.push(pr ? "handoff" : "finish");
  if (c.strategy !== "trunk" || c.shortBranches) items.push("feature");
  if (pr || c.strategy === "trunk" && c.shortBranches) items.push("fix");
  if (c.strategy === "gitflow" || c.strategy === "trunk" && c.releaseBranches) items.push("release");
  if (c.strategy === "gitflow") items.push("hotfix", "support");
  if (c.strategy === "trunk" || c.strategy === "gitflow") items.push("tag");
  if (sources.value.length) {
    if (!items.includes(pr ? "handoff" : "finish")) items.push(pr ? "handoff" : "finish");
    items.push("cleanup");
  }
  return items;
});
const title = computed(() => config.value && config.value.strategy !== "manual" ? workflowName(config.value.strategy) : "Branching workflow");
const prefix = computed(() => {
  const c = config.value;
  const action = request.value?.action;
  if (!c || !action) return "";
  if (action === "tag") return c.tag;
  if (action === "feature" || action === "fix" || action === "release" || action === "hotfix" || action === "support") return c[action];
  return "";
});
const requestsName = computed(() => request.value && ["feature", "fix", "release", "hotfix", "support", "tag"].includes(request.value.action));
const requestsSource = computed(() => request.value && ["finish", "cleanup", "handoff"].includes(request.value.action));
const versionOnFinish = computed(() => {
  const c = config.value, r = request.value;
  return c?.strategy === "gitflow" && r?.action === "finish"
    && (r.source.startsWith(c.release) || r.source.startsWith(c.hotfix));
});
const canOpenHandoff = computed(() => !!plan.value?.handoff?.published && props.canOpenPullRequest
  && (config.value?.strategy !== "swamp" || plan.value.comparisons.every(review => review.ahead > 0 && review.conflicts === false)));

function icon(action: WorkflowAction) {
  if (action === "finish") return GitMerge;
  if (action === "cleanup") return Trash2;
  if (action === "handoff") return GitPullRequest;
  if (action === "tag") return Tag;
  return GitBranchPlus;
}

async function load(initial?: WorkflowInspection) {
  info.value = initial ?? await invoke<WorkflowInspection>("inspect_branch_workflow", { path: props.repoPath });
  rememberWorkflowPath(props.repoPath, info.value.key);
  config.value = workflowPreference(info.value.key);
  emit("saved");
}

onMounted(async () => {
  dialog.value?.showModal();
  busy.value = true;
  try {
    await load(props.initial);
    setup.value = !config.value;
  } catch (failure) { error.value = String(failure); }
  finally { busy.value = false; }
});

function save(value: WorkflowConfig) {
  if (!info.value) return;
  const persisted = saveWorkflow(props.repoPath, info.value.key, value);
  config.value = value;
  setup.value = false;
  note.value = persisted ? "" : "Settings are available for this session only.";
  emit("saved");
}

function begin(action: WorkflowAction) {
  hint.value = undefined;
  const current = info.value?.current;
  const source = current && sources.value.includes(current) ? current : sources.value[0] ?? "";
  request.value = { action, name: "", source: action === "support" ? "" : source, cleanup: false, remote: props.remoteName };
  suggestTag();
}

function showHint(action: HintAction, event: PointerEvent | FocusEvent) {
  const anchor = event.currentTarget as HTMLButtonElement;
  if (event.type === "focus" ? !anchor.matches(":focus-visible") : (event as PointerEvent).pointerType === "touch") return;
  const { left, right, top, bottom } = anchor.getBoundingClientRect();
  const width = Math.min(260, window.innerWidth - 16);
  const above = window.innerHeight - bottom < 130 && top > window.innerHeight - bottom;
  const x = Math.max(8, Math.min((left + right - width) / 2, window.innerWidth - width - 8));
  hint.value = { action, x, y: above ? top - 6 : bottom + 6, above };
}

function hideHint(action: HintAction, event: PointerEvent | FocusEvent) {
  if (hint.value?.action !== action) return;
  const anchor = event.currentTarget as HTMLButtonElement;
  if (event.type === "pointerleave" ? anchor.matches(":focus-visible") : anchor.matches(":hover")) return;
  hint.value = undefined;
}

function suggestTag() {
  if (!request.value || !config.value || !versionOnFinish.value) return;
  const source = request.value.source;
  const prefix = source.startsWith(config.value.release) ? config.value.release : config.value.hotfix;
  request.value.name = source.slice(prefix.length);
}

function back() {
  plan.value = undefined;
  request.value = undefined;
  outcome.value = undefined;
  error.value = "";
}

function preview() { return refreshPreview(false); }

async function refreshPreview(fetchFirst: boolean) {
  if (!config.value || !request.value || busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    if (fetchFirst) await props.fetchRefs?.();
    request.value.name = request.value.name.trim();
    if (request.value.action === "handoff") request.value.remote = props.remoteName;
    plan.value = await invoke<WorkflowPlan>("preview_branch_workflow", {
      path: props.repoPath, config: config.value, request: request.value,
    });
  } catch (failure) { error.value = String(failure); }
  finally { busy.value = false; }
}

async function openHandoff(description?: string) {
  const reviewed = plan.value;
  const handoff = reviewed?.handoff;
  if (!reviewed || !handoff || !config.value || !request.value || !canOpenHandoff.value || busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    if (request.value.remote !== props.remoteName) throw new Error("The remote changed. Preview the handoff again.");
    await invoke("verify_branch_handoff", {
      path: props.repoPath, config: config.value, request: request.value, token: reviewed.token,
    });
    emit("pullRequest", { source: handoff.source, target: handoff.target, published: handoff.published, description });
  } catch (failure) { error.value = String(failure); }
  finally { busy.value = false; }
}

async function execute() {
  if (!config.value || !request.value || !plan.value || busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    const result = await invoke<WorkflowOutcome>("execute_branch_workflow", {
      path: props.repoPath, config: config.value, request: request.value, token: plan.value.token,
    });
    outcome.value = result;
    plan.value = undefined;
    emit("outcome", result);
    if (result.status === "stopped") await load();
  } catch (failure) {
    error.value = String(failure);
    plan.value = undefined;
  } finally { busy.value = false; }
}

function close() { if (!busy.value) emit("close"); }
function cancel() {
  if (hint.value) hint.value = undefined;
  else close();
}
</script>

<template>
  <dialog ref="dialog" class="workflow-dialog" aria-labelledby="workflow-title" @cancel.prevent="cancel" @scroll.self="hint = undefined">
    <header class="workflow-header">
      <GitBranch :size="19" class="branch-icon" />
      <div><h2 id="workflow-title">{{ setup ? 'Manage branching strategy' : title }}</h2>
        <small v-if="info?.current">{{ info.current }}</small></div>
      <CloseIconButton title="Close branching workflow" :disabled="busy" @click="close" />
    </header>
    <div class="workflow-content">
      <p v-if="busy && !info" role="status" class="inline"><Loader2 :size="16" class="animate-spin" /> Inspecting branch refs</p>
      <template v-else-if="info">
        <WorkflowSetup v-if="setup" :initial="config ?? undefined" :suggestion="config ? null : info.suggestion"
          :primary="info.primary" :development="info.development" :branches="branchNames" label="Save strategy"
          @save="save" @cancel="config ? setup = false : close()" />
        <template v-else-if="outcome">
          <strong>{{ outcome.status === "applied" ? (outcome.completed.length ? 'Workflow complete' : 'Already up to date') : 'Workflow stopped' }}</strong>
          <div v-if="outcome.completed.length"><span>Completed</span><ul><li v-for="step in outcome.completed" :key="step">{{ step }}</li></ul></div>
          <div v-if="outcome.remaining.length"><span>Not completed</span><ul><li v-for="step in outcome.remaining" :key="step">{{ step }}</li></ul></div>
          <p v-if="outcome.error" role="alert" class="error">{{ outcome.error }}</p>
          <footer>
            <button v-if="outcome.status !== 'applied'" class="plain" type="button" :disabled="busy" @click="back">Back to actions</button>
            <button class="primary" type="button" :disabled="busy" @click="close">Done</button>
          </footer>
        </template>
        <template v-else-if="request">
          <div class="inline">
            <button class="plain icon" type="button" aria-label="Back to workflow actions" :disabled="busy" @click="back"><ArrowLeft :size="16" /></button>
            <strong>{{ formatAction[request.action] }}</strong>
          </div>
          <form v-if="!plan" class="workflow-form" @submit.prevent="preview">
            <label v-if="requestsSource">Source branch
              <select v-model="request.source" :disabled="busy" @change="request.name = ''; suggestTag()"><option v-for="source in sources" :key="source" :value="source">{{ source }}</option></select>
            </label>
            <label v-if="requestsName || versionOnFinish">{{ request.action === 'tag' || versionOnFinish ? 'Version' : 'Branch name' }}
              <input v-model="request.name" :required="!!requestsName" maxlength="160" :disabled="busy" />
            </label>
            <label v-if="request.action === 'support'">Base release tag (optional)
              <input v-model.trim="request.source" :placeholder="config?.primary" maxlength="160" :disabled="busy" />
            </label>
            <code v-if="request.name && (prefix || versionOnFinish)">{{ (versionOnFinish ? config?.tag : prefix) }}{{ request.name.trim() }}</code>
            <label v-if="request.action === 'finish'" class="inline"><input v-model="request.cleanup" type="checkbox" :disabled="busy" /> Delete local source after integration</label>
            <footer><button type="submit" class="primary" :disabled="busy"><Loader2 v-if="busy" :size="14" class="animate-spin" /> Preview</button></footer>
          </form>
          <HandoffReview v-else-if="config?.strategy === 'swamp' && plan.handoff && plan.comparisons[0]"
            :handoff="plan.handoff" :comparison="plan.comparisons[0]" :busy="busy"
            :supports-pull-request="canOpenPullRequest" :can-fetch="!!(remoteName && fetchRefs)"
            @refresh="preview" @fetch="refreshPreview(true)" @open="openHandoff" @back="plan = undefined; error = ''" />
          <template v-else>
            <ol v-if="plan.steps.length" class="steps"><li v-for="step in plan.steps" :key="step">{{ step }}</li></ol>
            <div v-for="review in plan.comparisons" :key="review.target" class="comparison">
              <strong>{{ review.target }}</strong><span>{{ review.ahead }} ahead · {{ review.behind }} behind</span>
              <p v-if="review.conflicts === true" class="error">Conflicts detected in the merge preview.</p>
              <p v-else-if="review.conflicts === null" class="muted">Conflicts could not be checked.</p>
              <p v-else class="muted">No conflicts detected in the local preview.</p>
            </div>
            <template v-if="plan.handoff">
              <p class="muted">{{ plan.handoff.source }} → {{ plan.handoff.target }}</p>
              <p v-if="plan.handoff.checkpoint" class="muted">
                {{ plan.handoff.checkpoint.changedPaths }} changed paths · {{ plan.handoff.checkpoint.testPaths }} test paths touched. Tests were not run.
              </p>
              <p v-if="!canOpenPullRequest" class="muted">Use the remote provider to open a pull request.</p>
              <p v-else-if="!plan.handoff.published" class="muted">Push the source branch and fetch the target before opening a pull request.</p>
              <p v-else-if="!canOpenHandoff" class="error">Resolve the preflight findings before handoff.</p>
            </template>
            <p v-else-if="!plan.steps.length" class="muted">Nothing to change. The branch is already integrated.</p>
            <footer>
              <button type="button" class="plain" :disabled="busy" @click="plan = undefined">Back</button>
              <button v-if="plan.handoff" type="button" class="primary" :disabled="busy || !canOpenHandoff" @click="openHandoff()">Open pull request form</button>
              <button v-else type="button" class="primary" :disabled="busy || !plan.steps.length" @click="execute"><Loader2 v-if="busy" :size="14" class="animate-spin" /> Confirm</button>
            </footer>
          </template>
        </template>
        <template v-else>
          <div v-if="actions.length" class="workflow-actions">
            <button v-for="action in actions" :key="action" class="plain action" type="button" :aria-describedby="hint?.action === action ? hintId : undefined"
              @pointerenter="showHint(action, $event)" @pointerleave="hideHint(action, $event)"
              @focus="showHint(action, $event)" @blur="hideHint(action, $event)" @click="begin(action)">
              <component :is="icon(action)" :size="16" />{{ formatAction[action] }}
            </button>
          </div>
          <p v-else-if="config?.strategy !== 'manual'" class="muted">{{ info.blocked || 'Choose an existing primary branch in the strategy settings.' }}</p>
          <p v-else class="muted">No active branching strategy.</p>
          <footer><button type="button" class="plain" :aria-describedby="hint?.action === 'manage' ? hintId : undefined"
            @pointerenter="showHint('manage', $event)" @pointerleave="hideHint('manage', $event)"
            @focus="showHint('manage', $event)" @blur="hideHint('manage', $event)" @click="hint = undefined; setup = true"><Settings2 :size="15" /> Manage strategy</button></footer>
        </template>
      </template>
      <p v-if="note" role="status" class="muted">{{ note }}</p>
      <p v-if="error" role="alert" class="error">{{ error }}</p>
    </div>
    <div v-if="hint" :id="hintId" role="tooltip" class="workflow-hint" :style="{ left: `${hint.x}px`, top: `${hint.y}px` }" :class="{ above: hint.above }">{{ hints[hint.action] }}</div>
  </dialog>
</template>

<style scoped>
.workflow-dialog { width: min(570px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); margin: auto; padding: 0; border: 1px solid var(--border); border-radius: 8px; background: var(--popover); color: var(--foreground); box-shadow: 0 16px 50px #0005; overflow: auto; }
.workflow-dialog::backdrop { background: #0008; }
.workflow-header { display: grid; grid-template-columns: 20px minmax(0, 1fr) auto; align-items: center; gap: 10px; border-bottom: 1px solid var(--border); padding: 10px 14px; overflow-wrap: anywhere; }
.workflow-header h2 { font-size: 15px; font-weight: 700; }
.branch-icon { color: var(--primary); }
.workflow-header small, .muted { color: var(--muted-foreground); }
.workflow-content, .workflow-form { display: grid; gap: 11px; font-size: 12px; }
.workflow-content { padding: 14px; overflow-wrap: anywhere; }
.workflow-actions { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 7px; }
.workflow-form label:not(.inline) { display: grid; gap: 4px; }
.workflow-form :is(select, input:not([type="checkbox"])) { width: 100%; padding: 5px 7px; border: 1px solid var(--border); border-radius: 4px; background: var(--input-background); }
.inline, .plain, .primary { display: inline-flex; align-items: center; gap: 6px; }
.plain, .primary { min-height: 30px; padding: 5px 8px; border: 1px solid var(--border); border-radius: 4px; }
.plain:hover { background: var(--secondary); }
.action { justify-content: flex-start; text-align: left; }
.action svg { color: var(--primary); flex-shrink: 0; }
.primary { background: var(--primary); color: var(--primary-foreground); }
button:disabled { opacity: .5; }
footer { display: flex; justify-content: flex-end; gap: 7px; flex-wrap: wrap; }
.steps, ul { list-style: decimal inside; }
.comparison { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 3px 8px; border-block-end: 1px solid var(--border); padding: 4px 0; }
.comparison p { grid-column: 1 / -1; }
.error { color: var(--destructive); white-space: pre-wrap; }
.workflow-hint { position: fixed; z-index: 1; pointer-events: none; width: min(260px, calc(100vw - 16px)); padding: 7px 9px; border: 1px solid var(--border); border-radius: 5px; background: var(--popover); color: var(--foreground); box-shadow: 0 4px 15px #0004; font-size: 11px; overflow-wrap: anywhere; }
.workflow-hint.above { transform: translateY(-100%); }
@media (max-width: 390px) { .workflow-actions { grid-template-columns: 1fr; } }
</style>

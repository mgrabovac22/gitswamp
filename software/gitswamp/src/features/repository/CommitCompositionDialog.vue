<script setup lang="ts">
import { computed, nextTick, onMounted, ref, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ArrowDown, ArrowUp, ChevronRight, CircleHelp, GitCommitHorizontal, Hammer, Hash, Layers, Loader2, Plus, RefreshCw, Undo2 } from "lucide-vue-next";
import { createCommitLintEngine, inferScopeFromPaths } from "@/domain/analyzer/commitAnalyzer";
import { COMMIT_TYPES, closingIssueTag, formatCommitSubject, normalizeCommitTag, splitCommitSubject, withClosingIssue, withCommitTag } from "@/domain/analyzer/commitSubject";
import { getStoredCommitAnalyzerSettings } from "@/shared/config/commitAnalyzerPreferences";
import { secretDetectorEnabled } from "@/shared/config/secretDetectorPreferences";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";

type Change = { path: string; oldPath?: string; status: string; staged: boolean; partial: boolean; added: number; removed: number; kind: string;
  chunk?: { oldStart: number; oldLines: number; newStart: number; newLines: number; scope: string } };
type Unit = { id: number; changes: number[] };
type Draft = { subject: string; description: string; units: number[] };
type Plan = {
  branch: string; snapshot: string; changes: Change[]; units: Unit[]; groups: Draft[]; limited: boolean; dependencyNotice?: string;
  review: { findings: { path: string; line: number; kind: string }[]; skipped: string[]; large_files: { path: string; size: number }[] };
};
type Row = Draft & { id: number; tag: string; closeIssue: string; moved?: boolean };
type ChangePreview = {
  lines: { line_type: string; old_line_no?: number; new_line_no?: number; content: string }[];
  binary: boolean;
  limited: boolean;
};

const props = defineProps<{ repoPath: string }>();
const emit = defineEmits<{ close: []; committed: [count: number] }>();
const dialog = ref<HTMLDialogElement>();
const plan = shallowRef<Plan>();
const rows = ref<Row[]>([]);
const selected = ref(0);
const selectedChange = ref<number>();
const diffPanel = ref<HTMLElement>();
const preview = shallowRef<ChangePreview>();
const previewError = ref("");
const previewLoading = ref(false);
const loading = ref(true);
const pending = ref<"regroup" | "create" | "">("");
const saving = computed(() => !!pending.value);
const error = ref("");
const buildStatus = ref("");
const previous = shallowRef<{ plan: Plan; rows: Row[]; selected: number; appliedTag: string; sharedTag: string }>();
const acceptedWarnings = ref(false);
const verifyBuild = ref(false);
const buildCommand = ref("");
const useBuilder = ref(false);
const sharedTag = ref("");
const appliedTag = ref("");
const analyzer = createCommitLintEngine();
const settings = getStoredCommitAnalyzerSettings();
let nextId = 0;
let secretCheck = false;
let previewQueue = Promise.resolve();

const row = computed(() => rows.value.find(group => group.id === selected.value));
const builder = computed(() => splitCommitSubject(row.value?.subject ?? ""));
const issue = computed(() => closingIssueTag(row.value?.tag || builder.value.tag));
const normalizedSharedTag = computed(() => normalizeCommitTag(sharedTag.value));
const activeIndex = computed(() => rows.value.findIndex(group => group.id === selected.value));
const warnings = computed(() => !!(plan.value?.review.findings.length || plan.value?.review.skipped.length));
const filesFor = (ids: number[]) => ids.flatMap(id => plan.value?.units[id]?.changes ?? [])
  .map(id => plan.value!.changes[id]);
const summaries = computed(() => rows.value.map(group => {
  const files = filesFor(group.units);
  const paths = files.map(file => file.path);
  const uniqueFiles = new Map(files.map(file => [file.path, file]));
  const tag = normalizeCommitTag(group.tag);
  const subject = withCommitTag(group.subject, tag ?? "");
  const issue = closingIssueTag(tag || splitCommitSubject(group.subject).tag);
  const description = withClosingIssue(group.description, issue === group.closeIssue ? issue : "");
  const findings = analyzer.analyze({
    message: subject,
    description,
    stagedFiles: [...uniqueFiles.values()].map(file => ({ path: file.path, status: file.status, staged: true })),
    diffSummary: {
      filesChanged: uniqueFiles.size,
      inferredScope: inferScopeFromPaths(paths),
      fileTypes: [...new Set(paths.map(path => path.split(".").pop() || ""))],
      hasTestChanges: files.some(file => file.kind === "test"),
      hasMigrationChanges: files.some(file => file.kind === "migration"),
      totalLinesAdded: files.reduce((total, file) => total + file.added, 0),
      totalLinesRemoved: files.reduce((total, file) => total + file.removed, 0),
    },
    settings,
  });
  return { id: group.id, files: uniqueFiles.size, findings, subject, description,
    invalid: tag === null || !splitCommitSubject(group.subject).summary || subject.length > 72 ||
      /[\u0000-\u001f\u007f]/.test(group.subject) || !group.units.length || !!findings?.errors.length };
}));
const activeSummary = computed(() => summaries.value[activeIndex.value]);
// Keep the submitted series oldest first; the graph-style list presents it in reverse.
const history = computed(() => rows.value.map((group, index) => ({ group, index, summary: summaries.value[index] })).reverse());
const activeChange = computed(() => selectedChange.value === undefined ? undefined : plan.value?.changes[selectedChange.value]);
const destinations = computed(() => history.value.filter(entry => entry.group.id !== selected.value));
const statuses: Record<string, [string, string]> = {
  added: ["+", "Added"], modified: ["M", "Modified"], deleted: ["−", "Deleted"],
  renamed: ["R", "Renamed"], typechange: ["T", "Type changed"],
};
const status = (value: string) => statuses[value] ?? ["?", value];
const canReview = computed(() => plan.value && !loading.value && !saving.value && rows.value.length &&
  !summaries.value.some(group => group.invalid) && !plan.value.review.large_files.length);
const canConfirm = computed(() => canReview.value && (!warnings.value || acceptedWarnings.value) &&
  (!verifyBuild.value || !!buildCommand.value.trim()));
watch([buildCommand, verifyBuild, () => rows.value.map(group => group.units.join(",")).join(";")],
  () => { buildStatus.value = ""; }, { flush: "sync" });
watch(selected, () => { selectedChange.value = undefined; });
watch([selectedChange, () => plan.value?.snapshot], ([change, snapshot], _, onCleanup) => {
  preview.value = undefined;
  previewError.value = "";
  previewLoading.value = change !== undefined && !!snapshot;
  if (change === undefined || !snapshot) return;
  let current = true;
  const timer = window.setTimeout(() => {
    previewQueue = previewQueue.then(async () => {
      if (!current) return;
      try {
        const result = await invoke<ChangePreview>("preview_commit_composition_change", {
          path: props.repoPath, snapshot, secretCheck, change,
        });
        if (current) preview.value = result;
      } catch (failure) {
        if (current) previewError.value = String(failure);
      } finally {
        if (current) {
          previewLoading.value = false;
          await nextTick();
          if (current) diffPanel.value?.scrollIntoView({ block: "nearest", inline: "nearest" });
        }
      }
    });
  }, 120);
  onCleanup(() => { current = false; window.clearTimeout(timer); });
});

function newRow(draft: Draft): Row {
  const parts = splitCommitSubject(draft.subject);
  return { ...draft, subject: parts.subject, tag: parts.tag, closeIssue: "", id: ++nextId };
}

async function generate() {
  loading.value = true;
  error.value = "";
  buildStatus.value = "";
  previous.value = undefined;
  plan.value = undefined;
  rows.value = [];
  acceptedWarnings.value = false;
  sharedTag.value = "";
  appliedTag.value = "";
  secretCheck = secretDetectorEnabled();
  try {
    const result = await invoke<Plan>("plan_commit_composition", { path: props.repoPath, secretCheck });
    plan.value = result;
    rows.value = result.groups.map(newRow);
    selected.value = rows.value[rows.value.length - 1]?.id ?? 0;
  } catch (failure) {
    error.value = String(failure);
  } finally {
    loading.value = false;
  }
}

function editBuilder(change: Partial<Pick<ReturnType<typeof splitCommitSubject>, "type" | "scope" | "breaking" | "summary">>) {
  if (!row.value) return;
  if (!row.value.tag && builder.value.tag) row.value.tag = builder.value.tag;
  const { type, scope, breaking, summary } = { ...builder.value, ...change };
  row.value.subject = formatCommitSubject(type, scope, breaking, summary);
}

function applySharedTag() {
  if (normalizedSharedTag.value === null) return;
  appliedTag.value = normalizedSharedTag.value;
  for (const group of rows.value) {
    group.subject = splitCommitSubject(group.subject).subject;
    if (group.closeIssue !== closingIssueTag(normalizedSharedTag.value)) group.closeIssue = "";
    group.tag = normalizedSharedTag.value;
  }
}

function editTag(group: Row, value: string) {
  group.tag = value;
  if (group.closeIssue !== closingIssueTag(value)) group.closeIssue = "";
}

function editSubject(group: Row, value: string) {
  group.subject = value;
  if (group.closeIssue !== closingIssueTag(group.tag || splitCommitSubject(value).tag)) group.closeIssue = "";
}

function reorder(offset: number) {
  const index = activeIndex.value;
  const next = index + offset;
  if (index < 0 || next < 0 || next >= rows.value.length) return;
  const [group] = rows.value.splice(index, 1);
  rows.value.splice(next, 0, group);
}

function move(unit: number, destination: string) {
  if (!row.value || !destination) return;
  const source = row.value;
  const target = destination === "new"
    ? { id: ++nextId, units: [], subject: "", description: "", tag: appliedTag.value, closeIssue: "" } as Row
    : rows.value.find(group => group.id === Number(destination));
  if (!target || target.id === source.id) return;
  if (destination === "new") {
    rows.value.splice(activeIndex.value + 1, 0, target);
  }
  if (selectedChange.value !== undefined && plan.value?.units[unit].changes.includes(selectedChange.value)) {
    selectedChange.value = undefined;
  }
  target.units.push(unit);
  target.moved = source.moved = true;
  source.units = source.units.filter(id => id !== unit);
  if (!source.units.length) {
    rows.value = rows.value.filter(group => group.id !== source.id);
    selected.value = target.id;
  }
}

function addGroup() {
  const group = { id: ++nextId, units: [], subject: "", description: "", tag: appliedTag.value, closeIssue: "" };
  rows.value.push(group);
  selected.value = group.id;
}

function removeEmpty() {
  if (!row.value || row.value.units.length) return;
  rows.value.splice(activeIndex.value, 1);
  selected.value = rows.value[rows.value.length - 1]?.id ?? 0;
}

function draftsForReview(): Draft[] {
  return rows.value.map(({ units }, index) => {
    const { subject, description } = summaries.value[index];
    return { subject, description, units };
  });
}

async function findBuildableGroups() {
  const current = plan.value;
  if (!current || !canReview.value || !verifyBuild.value || !buildCommand.value.trim()) return;
  const original = rows.value;
  pending.value = "regroup";
  error.value = "";
  buildStatus.value = "";
  try {
    const result = await invoke<Pick<Plan, "groups" | "review">>("find_buildable_commit_groups", {
      path: props.repoPath, snapshot: current.snapshot, secretCheck,
      buildCommand: buildCommand.value.trim(), drafts: draftsForReview(),
    });
    const removed = original.length - result.groups.length;
    plan.value = { ...current, review: result.review };
    acceptedWarnings.value = false;
    if (!removed) {
      buildStatus.value = "Every commit passed the build. No groups were changed.";
      return;
    }
    if (!previous.value) {
      previous.value = {
        plan: current, selected: selected.value, appliedTag: appliedTag.value, sharedTag: sharedTag.value,
        rows: original.map(group => ({ ...group, units: [...group.units] })),
      };
    }
    const byFirstUnit = new Map(original.map(group => [group.units[0], group]));
    const next = result.groups.map(group => {
      const old = byFirstUnit.get(group.units[0]);
      if (old?.units.length === group.units.length) return old;
      return { ...newRow(group), tag: appliedTag.value };
    });
    rows.value = next;
    selected.value = next.find(group => !original.some(old => old.id === group.id))?.id ?? next[next.length - 1]?.id ?? 0;
    buildStatus.value = `Build passed. Review the new messages and issue tags (${original.length} to ${next.length} commits).`;
  } catch (failure) {
    error.value = String(failure);
  } finally {
    pending.value = "";
  }
}

function undoRegroup() {
  if (!previous.value || saving.value) return;
  const restored = previous.value;
  plan.value = restored.plan;
  rows.value = restored.rows;
  selected.value = restored.selected;
  appliedTag.value = restored.appliedTag;
  sharedTag.value = restored.sharedTag;
  previous.value = undefined;
  acceptedWarnings.value = false;
  error.value = "";
  buildStatus.value = "";
}

async function confirm() {
  if (!canConfirm.value || !plan.value) return;
  pending.value = "create";
  error.value = "";
  try {
    const commits = await invoke<string[]>("apply_commit_composition", {
      path: props.repoPath, snapshot: plan.value.snapshot, secretCheck, secretApproval: acceptedWarnings.value,
      buildCommand: verifyBuild.value ? buildCommand.value.trim() : null,
      drafts: draftsForReview(),
    });
    emit("committed", commits.length);
  } catch (failure) {
    error.value = String(failure);
  } finally {
    pending.value = "";
  }
}

function close() {
  if (!saving.value) emit("close");
}

onMounted(() => { dialog.value?.showModal(); void generate(); });
</script>

<template>
  <dialog ref="dialog" data-commit-composer class="composer" :class="{ 'composer-ready': plan }" aria-labelledby="compose-title" @cancel.prevent="close">
    <header class="composer-header">
      <Layers :size="20" class="composer-icon" />
      <div class="composer-heading">
        <h2 id="compose-title">Compose commits</h2>
        <span v-if="plan">{{ plan.branch }} · {{ plan.changes.length }} changes</span>
      </div>
      <span class="experiment">Experimental feature</span>
      <CloseIconButton title="Close commit composer" :disabled="saving" @click="close" />
    </header>

    <div v-if="loading" class="composer-state" role="status"><Loader2 :size="17" class="animate-spin" /> Analyzing working changes</div>
    <div v-else-if="!plan" class="composer-state" role="alert">{{ error }}</div>
    <template v-else>
      <div class="composer-bulk">
        <label><Hash :size="14" /><span>Tag all commits</span>
          <input v-model="sharedTag" placeholder="#123" maxlength="33" :disabled="saving" :aria-invalid="normalizedSharedTag === null" />
        </label>
        <button type="button" :disabled="saving || normalizedSharedTag === null" @click="applySharedTag">Apply to all</button>
      </div>
      <div class="composer-grid">
        <section class="composer-history" aria-label="Proposed commits">
          <header>
            <span>Commits · {{ rows.length }}</span>
            <span class="composer-order">
              Newest first
              <button type="button" aria-label="Commit order" aria-describedby="composer-order-help"><CircleHelp :size="14" /></button>
              <span id="composer-order-help" class="composer-tooltip" role="tooltip">Like the Git graph, the newest commit is at the top. Commits are created from bottom to top.</span>
            </span>
          </header>
          <ol class="composer-list">
            <li v-for="{ group, index, summary } in history" :key="group.id">
              <button type="button" class="composer-row" :title="summary.subject || 'Untitled commit'"
                :aria-current="selected === group.id ? 'step' : undefined" :disabled="saving" @click="selected = group.id">
                <span class="composer-node"><GitCommitHorizontal :size="15" /></span>
                <span class="composer-row-text">
                  <strong>{{ summary.subject || 'Untitled commit' }}</strong>
                  <small>Commit {{ index + 1 }} · {{ summary.files }} {{ summary.files === 1 ? 'file' : 'files' }}</small>
                </span>
                <ChevronRight :size="14" class="composer-chevron" />
              </button>
            </li>
          </ol>
        </section>
        <section v-if="row" :key="row.id" class="composer-editor" aria-label="Edit selected commit">
          <div class="composer-editor-header">
            <strong>Commit {{ activeIndex + 1 }}</strong>
            <div class="composer-actions">
              <button type="button" title="Visual Commit Builder" aria-label="Visual Commit Builder" :aria-pressed="useBuilder" :disabled="saving" @click="useBuilder = !useBuilder"><Hammer :size="15" /></button>
              <button type="button" title="Move commit up" aria-label="Move commit up" :disabled="saving || activeIndex === rows.length - 1" @click="reorder(1)"><ArrowUp :size="15" /></button>
              <button type="button" title="Move commit down" aria-label="Move commit down" :disabled="saving || activeIndex === 0" @click="reorder(-1)"><ArrowDown :size="15" /></button>
            </div>
          </div>
          <fieldset v-if="useBuilder" class="composer-builder" aria-label="Visual Commit Builder">
            <div class="composer-builder-fields">
              <label class="composer-field">Type
                <select :value="builder.type" :disabled="saving" @change="editBuilder({ type: ($event.target as HTMLSelectElement).value as typeof builder.type })">
                  <option v-for="type in COMMIT_TYPES" :key="type" :value="type">{{ type }}</option>
                </select>
              </label>
              <label class="composer-field">Scope
                <input :value="builder.scope" maxlength="32" :disabled="saving" @change="editBuilder({ scope: ($event.target as HTMLInputElement).value.trim().toLowerCase().replace(/[^a-z0-9_./-]/g, '-') })" />
              </label>
              <label class="composer-breaking">
                <input type="checkbox" :checked="builder.breaking" :disabled="saving" @change="editBuilder({ breaking: ($event.target as HTMLInputElement).checked })" />
                Breaking change
              </label>
            </div>
            <label class="composer-field">Summary
              <input :value="builder.summary" maxlength="72" :disabled="saving" @change="editBuilder({ summary: ($event.target as HTMLInputElement).value })" />
            </label>
          </fieldset>
          <label v-else class="composer-field">Subject <input :value="row.subject" maxlength="72" :disabled="saving" @input="editSubject(row, ($event.target as HTMLInputElement).value)" /></label>
          <label class="composer-field composer-tag"><span><Hash :size="13" /> Commit tag</span>
            <input :value="row.tag" placeholder="#123" maxlength="33" :disabled="saving" :aria-invalid="normalizeCommitTag(row.tag) === null" @input="editTag(row, ($event.target as HTMLInputElement).value)" />
          </label>
          <div v-if="useBuilder || row.tag" class="composer-preview">
            <code>{{ activeSummary?.subject }}</code>
            <span :class="{ 'composer-error': (activeSummary?.subject.length ?? 0) > 72 }">{{ activeSummary?.subject.length ?? 0 }}/72</span>
          </div>
          <p v-if="normalizeCommitTag(row.tag) === null" class="composer-error composer-lint">Use letters, numbers, dots, underscores, slashes or hyphens in the tag.</p>
          <label class="composer-field">Description <textarea v-model="row.description" maxlength="8000" rows="3" :disabled="saving" /></label>
          <label v-if="issue" class="composer-breaking">
            <input type="checkbox" :checked="row.closeIssue === issue" :disabled="saving"
              @change="row.closeIssue = ($event.target as HTMLInputElement).checked ? issue : ''" />
            Close issue {{ issue }}
          </label>
          <code v-if="issue && row.closeIssue === issue" class="composer-closing">Closes {{ issue }}</code>
          <ul v-if="activeSummary?.findings?.findings.length" class="composer-lint" aria-label="Commit message checks">
            <li v-for="finding in activeSummary.findings.findings" :key="finding.id" :class="{ 'composer-error': finding.severity === 'error' }">{{ finding.message }}</li>
          </ul>
          <p v-if="row.moved" class="composer-lint" role="status">Files moved. Review the subject and description.</p>
          <div v-if="row.units.length" class="composer-changes">
            <header class="composer-changes-header"><strong>Changed files</strong><span>{{ activeSummary?.files }}</span></header>
            <ul class="composer-files" aria-label="Changed files">
              <template v-for="unit in row.units" :key="unit">
                <li v-for="(change, index) in plan.units[unit].changes" :key="change">
                  <button type="button" class="composer-file" :aria-pressed="selectedChange === change"
                    :disabled="saving" :title="plan.changes[change].path"
                    @click="selectedChange = selectedChange === change ? undefined : change">
                    <span class="composer-file-status" :data-status="plan.changes[change].status" :title="status(plan.changes[change].status)[1]">
                      <span aria-hidden="true">{{ status(plan.changes[change].status)[0] }}</span>
                      <span class="sr-only">{{ status(plan.changes[change].status)[1] }}</span>
                    </span>
                    <span class="composer-paths">
                      <code>{{ plan.changes[change].oldPath ? plan.changes[change].oldPath + ' → ' : '' }}{{ plan.changes[change].path }}</code>
                      <small v-if="plan.changes[change].partial">Staged only</small>
                      <small v-if="plan.changes[change].chunk" class="composer-hunk">
                        {{ plan.changes[change].chunk!.scope }} · L{{ plan.changes[change].chunk!.newStart }}–{{ plan.changes[change].chunk!.newStart + Math.max(plan.changes[change].chunk!.newLines - 1, 0) }}
                      </small>
                    </span>
                    <ChevronRight :size="13" class="composer-chevron" />
                  </button>
                  <select v-if="index === 0" :aria-label="'Move ' + plan.changes[change].path"
                    :title="plan.units[unit].changes.length > 1 ? 'These changes must move together to keep the Git tree valid.' : 'Move this change to another commit'"
                    :disabled="saving" value="" @change="move(unit, ($event.target as HTMLSelectElement).value)">
                    <option value="" disabled>{{ plan.units[unit].changes.length > 1 ? 'Move linked files' : 'Move to…' }}</option>
                    <option v-for="entry in destinations" :key="entry.group.id" :value="entry.group.id">
                      {{ entry.index + 1 }}. {{ entry.summary.subject || 'Untitled commit' }}
                    </option>
                    <option v-if="rows.length < 20" value="new">New commit</option>
                  </select>
                  <span v-else class="composer-linked">Moves together</span>
                </li>
              </template>
            </ul>
            <section v-if="activeChange" ref="diffPanel" class="composer-diff" aria-label="File diff">
              <header><strong>Diff</strong><code :title="activeChange.path">{{ activeChange.path }}</code></header>
              <p v-if="previewLoading" class="composer-diff-note" role="status"><Loader2 :size="13" class="animate-spin" /> Loading diff</p>
              <p v-else-if="previewError" class="composer-diff-note composer-error" role="alert">{{ previewError }}</p>
              <template v-else-if="preview">
                <p v-if="preview.binary" class="composer-diff-note">Binary file.</p>
                <div v-else-if="preview.lines.length" class="composer-diff-lines" tabindex="0" aria-label="Diff lines">
                  <div v-for="(line, index) in preview.lines" :key="index" :data-line="line.line_type">
                    <span aria-hidden="true">{{ line.old_line_no }}</span><span aria-hidden="true">{{ line.new_line_no }}</span>
                    <span aria-hidden="true">{{ line.line_type === 'addition' ? '+' : line.line_type === 'deletion' ? '−' : ' ' }}</span><code>{{ line.content }}</code>
                  </div>
                </div>
                <p v-else-if="!preview.limited" class="composer-diff-note">No text changes.</p>
                <p v-if="preview.limited" class="composer-diff-note">{{ preview.lines.length ? 'Diff preview shortened.' : 'File is too large for an inline diff.' }}</p>
              </template>
            </section>
          </div>
          <button v-else class="composer-empty" type="button" :disabled="saving" @click="removeEmpty">Remove empty commit</button>
        </section>
      </div>
      <div v-if="plan.limited || plan.dependencyNotice || warnings || plan.review.large_files.length" class="composer-notices">
        <p v-if="plan.limited">Some diffs were classified by path only.</p>
        <p v-if="plan.dependencyNotice">{{ plan.dependencyNotice }}</p>
        <p v-for="file in plan.review.large_files" :key="file.path" class="composer-error">{{ file.path }} exceeds 100 MiB. Stage it with Git LFS Manager first.</p>
        <details v-if="warnings">
          <summary>Secret review: {{ plan.review.findings.length }} findings, {{ plan.review.skipped.length }} scan notices</summary>
          <ul>
            <li v-for="(finding, index) in plan.review.findings" :key="index">{{ finding.path }}:{{ finding.line }} · {{ finding.kind }}</li>
            <li v-for="(notice, index) in plan.review.skipped" :key="'skip' + index">{{ notice }}</li>
          </ul>
        </details>
        <label v-if="warnings"><input v-model="acceptedWarnings" type="checkbox" :disabled="saving" /> Commit after reviewing these warnings</label>
      </div>
    </template>
    <p v-if="plan && error" class="composer-apply-error composer-error" role="alert">{{ error }}</p>
    <div v-if="plan" class="composer-verify">
      <label class="composer-verify-toggle">
        <input v-model="verifyBuild" type="checkbox" :disabled="saving" /> Verify every commit
      </label>
      <label v-if="verifyBuild" class="composer-field composer-build-command">
        Build command
        <input v-model="buildCommand" type="text" maxlength="2048" spellcheck="false" placeholder="cargo check --locked" :disabled="saving" />
      </label>
      <p v-if="verifyBuild" class="composer-build-note">Runs trusted project code in a temporary source copy. A failed build blocks the entire series.</p>
      <div v-if="verifyBuild || previous" class="composer-actions composer-build-actions">
        <button v-if="verifyBuild" type="button" :disabled="!canReview || !buildCommand.trim()" @click="findBuildableGroups">
          <Loader2 v-if="pending === 'regroup'" :size="15" class="animate-spin" /><Layers v-else :size="15" />
          {{ pending === 'regroup' ? 'Checking groups' : 'Find buildable groups' }}
        </button>
        <button v-if="previous" type="button" :disabled="saving" @click="undoRegroup"><Undo2 :size="15" /> Undo regroup</button>
      </div>
      <p v-if="buildStatus" class="composer-build-note" role="status">{{ buildStatus }}</p>
    </div>
    <footer class="composer-footer">
      <div class="composer-actions">
        <button type="button" title="Regenerate plan" aria-label="Regenerate plan" :disabled="loading || saving" @click="generate"><RefreshCw :size="15" /></button>
        <button v-if="plan" type="button" title="Add commit" aria-label="Add commit" :disabled="saving || rows.length >= 20" @click="addGroup"><Plus :size="16" /></button>
      </div>
      <div class="composer-actions">
        <button type="button" :disabled="saving" @click="close">Cancel</button>
        <button type="button" class="composer-confirm" :disabled="!canConfirm" @click="confirm">
          <Loader2 v-if="pending === 'create'" :size="15" class="animate-spin" /><GitCommitHorizontal v-else :size="15" />
          {{ pending === 'create' ? (verifyBuild ? 'Verifying and creating' : 'Creating commits') : 'Create ' + rows.length + (rows.length === 1 ? ' commit' : ' commits') }}
        </button>
      </div>
    </footer>
  </dialog>
</template>

<style scoped>
.composer { margin: auto; width: min(1040px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); padding: 0; border: 1px solid var(--border); border-radius: 8px; background: var(--popover); color: var(--foreground); box-shadow: 0 18px 70px #0005; overflow: auto; }
.composer[open].composer-ready { display: flex; flex-direction: column; height: min(900px, calc(100dvh - 24px)); overflow: hidden; }
.composer-ready > :not(.composer-grid) { flex-shrink: 0; }
.composer::backdrop { background: #0007; }
.composer-header, .composer-footer { display: flex; align-items: center; gap: 10px; padding: 12px 14px; }
.composer-header { border-bottom: 1px solid var(--border); }
.composer-icon { color: var(--primary); flex-shrink: 0; }
.composer-heading { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.composer-heading h2 { font-size: 16px; font-weight: 700; margin: 0; }
.composer-heading span { font-size: 11px; color: var(--muted-foreground); }
.experiment { font-size: 11px; font-weight: 650; color: #086243; background: #dcfce7; border: 1px solid #76cf9e; padding: 3px 6px; border-radius: 4px; flex-shrink: 0; }
.composer-grid { display: grid; flex: 1; grid-template-columns: minmax(220px, 34%) minmax(0, 1fr); grid-template-rows: minmax(0, 1fr); min-height: 0; overflow: hidden; }
.composer-history { display: flex; flex-direction: column; min-width: 0; min-height: 0; border-right: 1px solid var(--border); }
.composer-history > header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 6px 12px; border-bottom: 1px solid var(--border); background: var(--secondary); color: var(--muted-foreground); font-size: 11px; font-weight: 650; }
.composer-order { position: relative; display: flex; align-items: center; gap: 4px; }
.composer-order button { display: grid; place-items: center; width: 24px; height: 24px; border-radius: 4px; color: var(--muted-foreground); }
.composer-order button:hover, .composer-order button:focus-visible { color: var(--primary); }
.composer-tooltip { display: none; position: absolute; inset: calc(100% + 4px) 0 auto auto; z-index: 2; width: 200px; padding: 7px 9px; border: 1px solid var(--border); border-radius: 5px; background: var(--popover); color: var(--foreground); font-weight: 400; box-shadow: 0 4px 14px #0002; pointer-events: none; }
.composer-order:hover .composer-tooltip, .composer-order:focus-within .composer-tooltip { display: block; }
.composer-list, .composer-editor { min-width: 0; min-height: 0; overflow: auto; }
.composer-list { margin: 0; padding: 0; list-style: none; }
.composer-row { display: grid; grid-template-columns: 20px minmax(0, 1fr) 14px; align-items: center; gap: 8px; width: 100%; min-height: 56px; padding: 6px 9px; border: 0; border-left: 3px solid transparent; border-bottom: 1px solid var(--border); font-size: 12px; text-align: left; transition: background-color .12s, border-color .12s; }
.composer-row-text { display: grid; min-width: 0; gap: 2px; }
.composer-row-text strong { display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; overflow: hidden; overflow-wrap: anywhere; line-height: 14px; font-weight: 650; }
.composer-row small, .composer-chevron { color: var(--muted-foreground); }
.composer-node { display: flex; color: var(--primary); }
.composer-row:not(:disabled):hover { background: var(--secondary); }
.composer-row[aria-current] { border-left-color: var(--primary); background: var(--secondary); box-shadow: inset 0 0 0 1px var(--border); }
.composer-row[aria-current] :is(.composer-chevron, .composer-node) { color: var(--primary); }
.composer-editor { padding: 12px 14px; overscroll-behavior: contain; }
.composer-editor-header, .composer-actions { display: flex; align-items: center; gap: 6px; }
.composer-editor-header { justify-content: space-between; font-size: 12px; margin-bottom: 8px; }
.composer-field { display: grid; gap: 3px; margin-bottom: 8px; font-size: 12px; }
.composer-field input, .composer-field textarea, .composer-field select, .composer-files select, .composer-bulk input { padding: 5px 7px; min-width: 0; border: 1px solid var(--border); border-radius: 5px; background: var(--input-background); color: var(--foreground); }
.composer :is(button, select, input, textarea):focus-visible { outline: 2px solid var(--ring); outline-offset: -2px; }
.composer :is(button, select):not(:disabled) { cursor: pointer; }
.composer-builder { min-width: 0; border: 0; padding: 0; margin: 0; }
.composer-builder-fields { display: grid; grid-template-columns: minmax(70px, 1fr) minmax(75px, 1fr) auto; align-items: end; gap: 8px; }
.composer-breaking { display: flex; align-items: center; gap: 4px; min-height: 30px; margin-bottom: 8px; font-size: 11px; }
.composer-tag span, .composer-bulk label { display: flex; gap: 5px; align-items: center; }
.composer-tag { max-width: 185px; }
.composer-bulk { display: flex; align-items: center; gap: 8px; padding: 7px 12px; border-bottom: 1px solid var(--border); font-size: 12px; }
.composer-bulk label { min-width: 0; }
.composer-bulk input { width: 120px; }
.composer-preview { display: flex; justify-content: space-between; gap: 8px; padding: 5px 7px; margin: 0 0 8px; border-radius: 4px; background: var(--secondary); font-size: 11px; }
.composer-preview code { min-width: 0; overflow-wrap: anywhere; }
.composer-preview span { flex-shrink: 0; }
.composer-field textarea { resize: vertical; }
.composer-closing { display: block; margin-bottom: 8px; color: var(--muted-foreground); font-size: 11px; }
.composer-lint { margin: 6px 0 8px; font-size: 11px; color: var(--muted-foreground); }
.composer-error { color: var(--destructive); overflow-wrap: anywhere; }
.composer-changes { border-top: 1px solid var(--border); }
.composer-changes-header, .composer-diff > header { display: flex; align-items: center; gap: 8px; padding: 8px 0; font-size: 12px; }
.composer-changes-header span, .composer-linked { color: var(--muted-foreground); font-size: 11px; }
.composer-files { max-height: min(225px, 27vh); overflow: auto; }
.composer-files li { display: grid; grid-template-columns: minmax(0, 1fr) minmax(82px, 120px); align-items: center; gap: 8px; border-top: 1px solid var(--border); padding: 3px 3px 3px 0; font-size: 11px; }
.composer-files select { width: 100%; min-height: 28px; text-overflow: ellipsis; }
.composer-file { display: grid; grid-template-columns: 19px minmax(0, 1fr) 13px; gap: 5px; align-items: center; text-align: left; width: 100%; min-height: 35px; padding: 4px; border: 1px solid transparent; border-radius: 4px; }
.composer-file:hover, .composer-file[aria-pressed="true"] { background: var(--secondary); }
.composer-file[aria-pressed="true"] { border-color: var(--primary); }
.composer-file-status { font: 700 12px monospace; text-align: center; }
.composer-file-status[data-status="added"] { color: #10b981; }
.composer-file-status[data-status="modified"], .composer-file-status[data-status="typechange"] { color: #f59e0b; }
.composer-file-status[data-status="deleted"] { color: #ef4444; }
.composer-file-status[data-status="renamed"] { color: #06b6d4; }
.composer-paths { display: flex; flex-wrap: wrap; min-width: 0; gap: 2px 8px; overflow-wrap: anywhere; }
.composer-paths small { color: var(--muted-foreground); }
.composer-paths .composer-hunk { color: var(--primary); font-family: monospace; }
.composer-diff { margin-top: 4px; border-top: 1px solid var(--border); }
.composer-diff > header code { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted-foreground); font-size: 11px; }
.composer-diff-note { display: flex; gap: 6px; align-items: center; font-size: 11px; margin: 4px 0; overflow-wrap: anywhere; }
.composer-diff-lines { max-height: 190px; overflow: auto; border: 1px solid var(--border); border-radius: 4px; background: var(--input-background); font: 11px/1.55 monospace; }
.composer-diff-lines > div { display: grid; grid-template-columns: 4ch 4ch 2ch max-content; width: max-content; min-width: 100%; min-height: 17px; padding-inline: 4px; }
.composer-diff-lines span { text-align: center; color: var(--muted-foreground); }
.composer-diff-lines code { white-space: pre; }
.composer-diff-lines [data-line="addition"] { background: color-mix(in srgb, #10b981 14%, transparent); }
.composer-diff-lines [data-line="deletion"] { background: color-mix(in srgb, #ef4444 14%, transparent); }
.composer-diff-lines [data-line="hunk"] { background: var(--secondary); color: var(--muted-foreground); }
.composer-notices, .composer-apply-error { border-top: 1px solid var(--border); padding: 8px 14px; max-height: 130px; overflow: auto; font-size: 12px; overflow-wrap: anywhere; }
.composer-notices label { display: flex; align-items: center; gap: 6px; }
.composer-state { display: flex; align-items: center; gap: 8px; padding: 24px 14px; font-size: 13px; overflow-wrap: anywhere; }
.composer-footer { border-top: 1px solid var(--border); justify-content: space-between; }
.composer-verify { display: grid; gap: 6px; border-top: 1px solid var(--border); padding: 8px 14px; font-size: 12px; }
.composer-verify-toggle { display: flex; align-items: center; gap: 6px; }
.composer-build-command { margin-bottom: 0; }
.composer-build-note { color: var(--muted-foreground); font-size: 11px; }
.composer-build-actions { flex-wrap: wrap; }
.composer-apply-error { white-space: pre-wrap; }
.composer-actions button, .composer-empty, .composer-bulk button { display: inline-flex; min-height: 29px; align-items: center; gap: 5px; padding: 4px 8px; border: 1px solid var(--border); border-radius: 5px; background: var(--secondary); font-size: 12px; }
.composer-bulk button { flex-shrink: 0; }
.composer-actions button:not(:disabled):hover, .composer-empty:hover, .composer-bulk button:not(:disabled):hover { border-color: var(--primary); color: var(--primary); }
.composer-actions button[aria-pressed="true"] { border-color: var(--primary); color: var(--primary); }
.composer-actions .composer-confirm { background: var(--primary); color: var(--primary-foreground); }
.composer-actions .composer-confirm:not(:disabled):hover { background: color-mix(in srgb, var(--primary) 84%, var(--foreground)); color: var(--primary-foreground); }
.composer button:disabled { opacity: .5; }
@media (max-width: 570px) {
  .composer-grid { grid-template-columns: minmax(0, 1fr); grid-template-rows: minmax(66px, 28%) minmax(0, 1fr); }
  .composer-history { border-right: 0; border-bottom: 1px solid var(--border); max-height: 140px; }
  .composer-header { gap: 6px; padding: 8px; }
  .experiment { max-width: 86px; white-space: normal; }
  .composer-bulk input { width: min(84px, 25vw); }
}
@media (max-height: 540px) {
  .composer[open].composer-ready { overflow-y: auto; }
  .composer-grid { flex-basis: 280px; flex-shrink: 0; }
}
</style>

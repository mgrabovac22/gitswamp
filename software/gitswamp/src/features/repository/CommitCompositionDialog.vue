<script setup lang="ts">
import { computed, onMounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ArrowDown, ArrowUp, ChevronRight, GitCommitHorizontal, Hammer, Hash, Layers, Loader2, Plus, RefreshCw } from "lucide-vue-next";
import { createCommitLintEngine, inferScopeFromPaths } from "@/domain/analyzer/commitAnalyzer";
import { COMMIT_TYPES, formatCommitSubject, normalizeCommitTag, splitCommitSubject, withCommitTag } from "@/domain/analyzer/commitSubject";
import { getStoredCommitAnalyzerSettings } from "@/shared/config/commitAnalyzerPreferences";
import { secretDetectorEnabled } from "@/shared/config/secretDetectorPreferences";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";

type Change = { path: string; oldPath?: string; status: string; staged: boolean; partial: boolean; added: number; removed: number; kind: string };
type Unit = { id: number; changes: number[] };
type Draft = { subject: string; description: string; units: number[] };
type Plan = {
  branch: string; snapshot: string; changes: Change[]; units: Unit[]; groups: Draft[]; limited: boolean;
  review: { findings: { path: string; line: number; kind: string }[]; skipped: string[]; large_files: { path: string; size: number }[] };
};
type Row = Draft & { id: number; tag: string };

const props = defineProps<{ repoPath: string }>();
const emit = defineEmits<{ close: []; committed: [count: number] }>();
const dialog = ref<HTMLDialogElement>();
const plan = shallowRef<Plan>();
const rows = ref<Row[]>([]);
const selected = ref(0);
const loading = ref(true);
const saving = ref(false);
const error = ref("");
const acceptedWarnings = ref(false);
const useBuilder = ref(false);
const sharedTag = ref("");
const appliedTag = ref("");
const analyzer = createCommitLintEngine();
const settings = getStoredCommitAnalyzerSettings();
let nextId = 0;
let secretCheck = false;

const row = computed(() => rows.value.find(group => group.id === selected.value));
const builder = computed(() => splitCommitSubject(row.value?.subject ?? ""));
const normalizedSharedTag = computed(() => normalizeCommitTag(sharedTag.value));
const activeIndex = computed(() => rows.value.findIndex(group => group.id === selected.value));
const warnings = computed(() => !!(plan.value?.review.findings.length || plan.value?.review.skipped.length));
const filesFor = (ids: number[]) => ids.flatMap(id => plan.value?.units[id]?.changes ?? [])
  .map(id => plan.value!.changes[id]);
const summaries = computed(() => rows.value.map(group => {
  const files = filesFor(group.units);
  const paths = files.map(file => file.path);
  const tag = normalizeCommitTag(group.tag);
  const subject = withCommitTag(group.subject, tag ?? "");
  const findings = analyzer.analyze({
    message: subject,
    description: group.description,
    stagedFiles: files.map(file => ({ path: file.path, status: file.status, staged: true })),
    diffSummary: {
      filesChanged: files.length,
      inferredScope: inferScopeFromPaths(paths),
      fileTypes: [...new Set(paths.map(path => path.split(".").pop() || ""))],
      hasTestChanges: files.some(file => file.kind === "test"),
      hasMigrationChanges: files.some(file => file.kind === "migration"),
      totalLinesAdded: files.reduce((total, file) => total + file.added, 0),
      totalLinesRemoved: files.reduce((total, file) => total + file.removed, 0),
    },
    settings,
  });
  return { id: group.id, files: files.length, findings, subject,
    invalid: tag === null || !splitCommitSubject(group.subject).summary || subject.length > 72 ||
      /[\u0000-\u001f\u007f]/.test(group.subject) || !group.units.length || !!findings?.errors.length };
}));
const activeSummary = computed(() => summaries.value[activeIndex.value]);
const canConfirm = computed(() => plan.value && !loading.value && !saving.value && rows.value.length &&
  !summaries.value.some(group => group.invalid) && !plan.value.review.large_files.length && (!warnings.value || acceptedWarnings.value));

async function generate() {
  loading.value = true;
  error.value = "";
  plan.value = undefined;
  rows.value = [];
  acceptedWarnings.value = false;
  sharedTag.value = "";
  appliedTag.value = "";
  secretCheck = secretDetectorEnabled();
  try {
    const result = await invoke<Plan>("plan_commit_composition", { path: props.repoPath, secretCheck });
    plan.value = result;
    rows.value = result.groups.map(draft => {
      const parts = splitCommitSubject(draft.subject);
      return { ...draft, subject: parts.subject, tag: parts.tag, id: ++nextId };
    });
    selected.value = rows.value[0]?.id ?? 0;
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
    group.tag = normalizedSharedTag.value;
  }
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
    ? { id: ++nextId, units: [], subject: "", description: "", tag: appliedTag.value } as Row
    : rows.value.find(group => group.id === Number(destination));
  if (!target || target.id === source.id) return;
  if (destination === "new") {
    rows.value.splice(activeIndex.value + 1, 0, target);
  }
  target.units.push(unit);
  source.units = source.units.filter(id => id !== unit);
  if (!source.units.length) {
    rows.value = rows.value.filter(group => group.id !== source.id);
    selected.value = target.id;
  }
}

function addGroup() {
  const group = { id: ++nextId, units: [], subject: "", description: "", tag: appliedTag.value };
  rows.value.push(group);
  selected.value = group.id;
}

function removeEmpty() {
  if (!row.value || row.value.units.length) return;
  rows.value.splice(activeIndex.value, 1);
  selected.value = rows.value[0]?.id ?? 0;
}

async function confirm() {
  if (!canConfirm.value || !plan.value) return;
  saving.value = true;
  error.value = "";
  try {
    const commits = await invoke<string[]>("apply_commit_composition", {
      path: props.repoPath, snapshot: plan.value.snapshot, secretCheck, secretApproval: acceptedWarnings.value,
      drafts: rows.value.map(({ description, units }, index) => ({ subject: summaries.value[index].subject, description, units })),
    });
    emit("committed", commits.length);
  } catch (failure) {
    error.value = String(failure);
  } finally {
    saving.value = false;
  }
}

function close() {
  if (!saving.value) emit("close");
}

onMounted(() => { dialog.value?.showModal(); void generate(); });
</script>

<template>
  <dialog ref="dialog" data-commit-composer class="composer" aria-labelledby="compose-title" @cancel.prevent="close">
    <header class="composer-header">
      <Layers :size="20" class="composer-icon" />
      <div class="composer-heading">
        <h2 id="compose-title">Compose commits</h2>
        <span v-if="plan">{{ plan.branch }} · {{ plan.changes.length }} changed paths</span>
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
          <header><span>Commits</span><span>{{ rows.length }}</span></header>
          <ol class="composer-list">
            <li v-for="(group, index) in rows" :key="group.id">
              <button type="button" class="composer-row" :title="summaries[index].subject || 'Untitled commit'"
                :aria-current="selected === group.id ? 'step' : undefined" :disabled="saving" @click="selected = group.id">
                <span class="composer-node"><GitCommitHorizontal :size="15" /></span>
                <span class="composer-row-text">
                  <strong>{{ summaries[index].subject || 'Untitled commit' }}</strong>
                  <small>Commit {{ index + 1 }} · {{ summaries[index].files }} {{ summaries[index].files === 1 ? 'file' : 'files' }}</small>
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
              <button type="button" title="Move commit up" aria-label="Move commit up" :disabled="saving || activeIndex === 0" @click="reorder(-1)"><ArrowUp :size="15" /></button>
              <button type="button" title="Move commit down" aria-label="Move commit down" :disabled="saving || activeIndex === rows.length - 1" @click="reorder(1)"><ArrowDown :size="15" /></button>
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
          <label v-else class="composer-field">Subject <input v-model="row.subject" maxlength="72" :disabled="saving" /></label>
          <label class="composer-field composer-tag"><span><Hash :size="13" /> Commit tag</span>
            <input v-model="row.tag" placeholder="#123" maxlength="33" :disabled="saving" :aria-invalid="normalizeCommitTag(row.tag) === null" />
          </label>
          <div v-if="useBuilder || row.tag" class="composer-preview">
            <code>{{ activeSummary?.subject }}</code>
            <span :class="{ 'composer-error': (activeSummary?.subject.length ?? 0) > 72 }">{{ activeSummary?.subject.length ?? 0 }}/72</span>
          </div>
          <p v-if="normalizeCommitTag(row.tag) === null" class="composer-error composer-lint">Use letters, numbers, dots, underscores, slashes or hyphens in the tag.</p>
          <label class="composer-field">Description <textarea v-model="row.description" maxlength="8000" rows="3" :disabled="saving" /></label>
          <ul v-if="activeSummary?.findings?.findings.length" class="composer-lint" aria-label="Commit message checks">
            <li v-for="finding in activeSummary.findings.findings" :key="finding.id" :class="{ 'composer-error': finding.severity === 'error' }">{{ finding.message }}</li>
          </ul>
          <ul v-if="row.units.length" class="composer-files" aria-label="Change sets">
            <li v-for="unit in row.units" :key="unit">
              <div class="composer-paths">
                <div v-for="file in filesFor([unit])" :key="file.path">
                  <code :title="file.path">{{ file.oldPath ? file.oldPath + ' → ' : '' }}{{ file.path }}</code>
                  <span :title="file.partial ? 'Unstaged edits to this file will stay local' : undefined">{{ file.partial ? 'Staged only' : file.status }}</span>
                </div>
              </div>
              <select aria-label="Move change set" title="Move change set" :disabled="saving"
                :value="''" @change="move(unit, ($event.target as HTMLSelectElement).value)">
                <option value="" disabled>Move to…</option>
                <option v-for="(group, index) in rows.filter(candidate => candidate.id !== row?.id)" :key="group.id" :value="group.id">
                  {{ rows.findIndex(candidate => candidate.id === group.id) + 1 }}. {{ group.subject || 'Untitled commit ' + (index + 1) }}
                </option>
                <option v-if="rows.length < 20" value="new">New commit</option>
              </select>
            </li>
          </ul>
          <button v-else class="composer-empty" type="button" :disabled="saving" @click="removeEmpty">Remove empty commit</button>
        </section>
      </div>
      <div v-if="plan.limited || warnings || plan.review.large_files.length" class="composer-notices">
        <p v-if="plan.limited">Some diffs were classified by path only.</p>
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
    <footer class="composer-footer">
      <div class="composer-actions">
        <button type="button" title="Regenerate plan" aria-label="Regenerate plan" :disabled="loading || saving" @click="generate"><RefreshCw :size="15" /></button>
        <button v-if="plan" type="button" title="Add commit" aria-label="Add commit" :disabled="saving || rows.length >= 20" @click="addGroup"><Plus :size="16" /></button>
      </div>
      <div class="composer-actions">
        <button type="button" :disabled="saving" @click="close">Cancel</button>
        <button type="button" class="composer-confirm" :disabled="!canConfirm" @click="confirm">
          <Loader2 v-if="saving" :size="15" class="animate-spin" /><GitCommitHorizontal v-else :size="15" />
          {{ saving ? 'Creating commits' : 'Create ' + rows.length + (rows.length === 1 ? ' commit' : ' commits') }}
        </button>
      </div>
    </footer>
  </dialog>
</template>

<style scoped>
.composer { margin: auto; width: min(840px, calc(100vw - 24px)); max-height: min(760px, calc(100vh - 24px)); padding: 0; border: 1px solid var(--border); border-radius: 8px; background: var(--popover); color: var(--foreground); box-shadow: 0 18px 70px #0005; overflow: auto; }
.composer::backdrop { background: #0007; }
.composer-header, .composer-footer { display: flex; align-items: center; gap: 10px; padding: 12px 14px; }
.composer-header { border-bottom: 1px solid var(--border); }
.composer-icon { color: var(--primary); flex-shrink: 0; }
.composer-heading { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.composer-heading h2 { font-size: 16px; font-weight: 700; margin: 0; }
.composer-heading span { font-size: 11px; color: var(--muted-foreground); }
.experiment { font-size: 11px; font-weight: 650; color: #086243; background: #dcfce7; border: 1px solid #76cf9e; padding: 3px 6px; border-radius: 4px; flex-shrink: 0; }
.composer-grid { display: grid; grid-template-columns: minmax(195px, 34%) minmax(0, 1fr); min-height: 280px; max-height: min(530px, 58vh); }
.composer-history { display: flex; flex-direction: column; min-width: 0; min-height: 0; border-right: 1px solid var(--border); }
.composer-history > header { display: flex; justify-content: space-between; padding: 7px 12px; border-bottom: 1px solid var(--border); background: var(--secondary); color: var(--muted-foreground); font-size: 11px; font-weight: 650; }
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
.composer-editor { padding: 12px; }
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
.composer-lint { margin: 6px 0 8px; font-size: 11px; color: var(--muted-foreground); }
.composer-error { color: var(--destructive); overflow-wrap: anywhere; }
.composer-files li { display: flex; align-items: center; justify-content: space-between; gap: 6px; border-top: 1px solid var(--border); padding: 7px 0; font-size: 11px; }
.composer-files select { width: 114px; flex-shrink: 0; }
.composer-paths { min-width: 0; }
.composer-paths div { display: flex; flex-wrap: wrap; gap: 2px 6px; overflow-wrap: anywhere; }
.composer-paths span { color: var(--muted-foreground); }
.composer-notices, .composer-apply-error { border-top: 1px solid var(--border); padding: 8px 14px; max-height: 130px; overflow: auto; font-size: 12px; overflow-wrap: anywhere; }
.composer-notices label { display: flex; align-items: center; gap: 6px; }
.composer-state { display: flex; align-items: center; gap: 8px; padding: 24px 14px; font-size: 13px; overflow-wrap: anywhere; }
.composer-footer { border-top: 1px solid var(--border); justify-content: space-between; }
.composer-actions button, .composer-empty, .composer-bulk button { display: inline-flex; min-height: 29px; align-items: center; gap: 5px; padding: 4px 8px; border: 1px solid var(--border); border-radius: 5px; background: var(--secondary); font-size: 12px; }
.composer-bulk button { flex-shrink: 0; }
.composer-actions button:not(:disabled):hover, .composer-empty:hover, .composer-bulk button:not(:disabled):hover { border-color: var(--primary); color: var(--primary); }
.composer-actions button[aria-pressed="true"] { border-color: var(--primary); color: var(--primary); }
.composer-actions .composer-confirm { background: var(--primary); color: var(--primary-foreground); }
.composer-actions .composer-confirm:not(:disabled):hover { background: color-mix(in srgb, var(--primary) 84%, var(--foreground)); color: var(--primary-foreground); }
.composer button:disabled { opacity: .5; }
@media (max-width: 570px) {
  .composer-grid { grid-template-columns: minmax(0, 1fr); grid-template-rows: auto minmax(0, 1fr); max-height: 57vh; }
  .composer-history { border-right: 0; border-bottom: 1px solid var(--border); max-height: 140px; }
  .composer-header { gap: 6px; padding: 8px; }
  .experiment { max-width: 86px; white-space: normal; }
  .composer-bulk input { width: min(84px, 25vw); }
}
</style>

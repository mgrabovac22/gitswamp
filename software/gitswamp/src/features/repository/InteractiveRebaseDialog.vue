<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { GitBranch, GripVertical, Loader2, ListRestart } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";

const props = defineProps<{ repoPath: string; base: string; busy: boolean; executionError: string }>();
const emit = defineEmits<{ close: []; start: [payload: { head: string; branch: string; steps: Step[] }] }>();
type Action = "pick" | "reword" | "edit" | "squash" | "fixup" | "drop";
type Commit = { sha: string; message: string };
type Step = { sha: string; action: Action; message?: string; combinedMessage?: string };
type Plan = { head: string; branch: string; commits: Commit[] };

const actions: { id: Action; label: string; title: string }[] = [
  { id: "pick", label: "Pick", title: "Keep the commit" },
  { id: "reword", label: "Reword", title: "Change the commit message" },
  { id: "edit", label: "Edit", title: "Pause to amend the commit" },
  { id: "squash", label: "Squash", title: "Combine and edit the resulting message" },
  { id: "fixup", label: "Fixup", title: "Combine without keeping this message" },
  { id: "drop", label: "Drop", title: "Remove the commit" },
];
const dialog = ref<HTMLDialogElement>();
const plan = ref<Plan>();
const rows = ref<(Commit & { action: Action; reword: string })[]>([]);
const combinedDrafts = ref<Record<string, string>>({});
const loading = ref(true);
const error = ref("");
const dragged = ref<string | null>(null);
const dropTarget = ref<{ sha: string; after: boolean } | null>(null);
let mounted = true;

function subject(commit: Commit) { return commit.message.split("\n", 1)[0] || "(no commit message)"; }
function combines(action: Action) { return action === "squash" || action === "fixup"; }
const groups = computed(() => {
  const rootBySha = new Map<string, string>();
  const childrenByRoot = new Map<string, typeof rows.value>();
  let root = "";
  let invalid = false;
  for (const row of rows.value) {
    if (row.action === "drop") continue;
    if (!combines(row.action)) { root = row.sha; continue; }
    if (!root) { invalid = true; continue; }
    rootBySha.set(row.sha, root);
    const children = childrenByRoot.get(root) ?? [];
    children.push(row);
    childrenByRoot.set(root, children);
  }
  return { rootBySha, childrenByRoot, invalid };
});
const combinedFields = computed(() => {
  const fields = new Map<string, { root: string; message: string }>();
  for (const row of rows.value) {
    const children = groups.value.childrenByRoot.get(row.sha);
    const squash = children?.filter(child => child.action === "squash");
    if (!children || !squash?.length) continue;
    fields.set(children[children.length - 1].sha, {
      root: row.sha,
      message: [row.action === "reword" ? row.reword : row.message, ...squash.map(child => child.message)].filter(Boolean).join("\n\n"),
    });
  }
  return fields;
});
const validation = computed(() => {
  if (groups.value.invalid) return "Squash and Fixup need a preceding commit that is not dropped.";
  if (rows.value.some(row => row.action === "reword" && !row.reword.trim())) return "Enter a message for each Reword commit.";
  if ([...combinedFields.value.values()].some(field => !(combinedDrafts.value[field.root] ?? field.message).trim())) return "Enter a message for each Squash group.";
  return "";
});
const changed = computed(() => rows.value.some((row, index) => row.action !== "pick" || row.sha !== plan.value?.commits[index].sha));

onMounted(async () => {
  dialog.value?.showModal();
  try {
    const loaded = await invoke<Plan>("get_interactive_rebase_plan", { path: props.repoPath, base: props.base });
    if (!mounted) return;
    plan.value = loaded;
    rows.value = loaded.commits.map(commit => ({ ...commit, action: "pick", reword: commit.message }));
  } catch (cause) {
    if (mounted) error.value = String(cause);
  } finally {
    if (mounted) loading.value = false;
  }
});
onBeforeUnmount(() => { mounted = false; dialog.value?.close(); });

function close() { if (!props.busy) emit("close"); }

function move(sha: string, index: number) {
  const from = rows.value.findIndex(row => row.sha === sha);
  if (from < 0 || index < 0 || index >= rows.value.length || index === from || props.busy) return;
  const next = [...rows.value];
  const [row] = next.splice(from, 1);
  next.splice(index, 0, row);
  rows.value = next;
}

function startDrag(event: DragEvent, sha: string) {
  if (!event.dataTransfer || props.busy) return;
  dragged.value = sha;
  event.dataTransfer.effectAllowed = "move";
  event.dataTransfer.setData("text/plain", sha);
  const row = (event.currentTarget as HTMLElement).closest("li");
  if (row) event.dataTransfer.setDragImage(row, 16, 16);
}

function dragOver(event: DragEvent, sha: string) {
  if (!dragged.value || dragged.value === sha) { dropTarget.value = null; return; }
  event.preventDefault();
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  dropTarget.value = { sha, after: event.clientY >= rect.top + rect.height / 2 };
}

function endDrag() { dragged.value = null; dropTarget.value = null; }

function drop() {
  if (dragged.value && dropTarget.value) {
    const from = rows.value.findIndex(row => row.sha === dragged.value);
    const target = rows.value.findIndex(row => row.sha === dropTarget.value?.sha) + Number(dropTarget.value.after);
    move(dragged.value, target - Number(from < target));
  }
  endDrag();
}

function actionChanged(event: Event, sha: string) {
  if ((event.target as HTMLSelectElement).value !== "reword") return;
  void nextTick(() => document.getElementById(`reword-${sha}`)?.focus());
}

function submit() {
  if (!plan.value || props.busy || !changed.value || validation.value) return;
  const combined = new Map([...combinedFields.value.values()].map(field => [field.root, combinedDrafts.value[field.root] ?? field.message]));
  emit("start", { head: plan.value.head, branch: plan.value.branch, steps: rows.value.map(row => ({
    sha: row.sha, action: row.action, message: row.action === "reword" ? row.reword : undefined,
    combinedMessage: combined.get(row.sha),
  })) });
}
</script>

<template>
  <dialog ref="dialog" class="rebase-dialog" aria-labelledby="rebase-title" @cancel.prevent="close">
    <header class="rebase-heading">
      <ListRestart :size="20" class="rebase-icon" />
      <div class="min-w-0">
        <h2 id="rebase-title">Interactive Rebase</h2>
        <p><GitBranch :size="13" /> <strong>{{ plan?.branch || 'Current branch' }}</strong><span>From</span><code>{{ base.slice(0, 7) }}</code></p>
      </div>
      <CloseIconButton class="ml-auto" title="Close interactive rebase" :disabled="busy" @click="close" />
    </header>

    <div v-if="loading" class="rebase-placeholder" role="status"><Loader2 :size="16" class="animate-spin" /> Loading commits</div>
    <div v-else-if="error" class="rebase-placeholder" role="alert">{{ error }}</div>
    <template v-else>
      <div class="rebase-list-label"><span>{{ rows.length }} {{ rows.length === 1 ? 'commit' : 'commits' }}</span><span>Oldest first</span></div>
      <ol class="rebase-list" aria-label="Commits to rebase">
        <li v-for="(row, index) in rows" :key="row.sha" class="rebase-row"
          :data-dropped="row.action === 'drop'" :data-grouped="groups.rootBySha.has(row.sha) || groups.childrenByRoot.has(row.sha)"
          :data-joined="groups.rootBySha.has(row.sha)"
          :data-drop="dropTarget?.sha === row.sha ? (dropTarget.after ? 'after' : 'before') : undefined"
          @dragover="dragOver($event, row.sha)" @drop.prevent="drop">
          <div class="rebase-row-main">
            <button type="button" class="rebase-grip" :draggable="!busy" :disabled="busy" :aria-label="`Reorder ${subject(row)}`"
              title="Drag to reorder, or press Up or Down" @dragstart="startDrag($event, row.sha)" @dragend="endDrag"
              @keydown.up.prevent="move(row.sha, index - 1)" @keydown.down.prevent="move(row.sha, index + 1)"><GripVertical :size="16" /></button>
            <code>{{ row.sha.slice(0, 7) }}</code>
            <span class="rebase-subject" :title="subject(row)">{{ subject(row) }}</span>
            <select v-model="row.action" :disabled="busy" :aria-label="`Action for ${subject(row)}`"
              :title="actions.find(action => action.id === row.action)?.title" @change="actionChanged($event, row.sha)">
              <option v-for="action in actions" :key="action.id" :value="action.id">{{ action.label }}</option>
            </select>
          </div>
          <label v-if="row.action === 'reword'" class="rebase-message">
            <span>Commit message</span>
            <textarea :id="`reword-${row.sha}`" v-model="row.reword" rows="3" :disabled="busy" />
          </label>
          <label v-if="combinedFields.get(row.sha)" class="rebase-message">
            <span>Combined commit message</span>
            <textarea :value="combinedDrafts[combinedFields.get(row.sha)!.root] ?? combinedFields.get(row.sha)!.message"
              rows="3" :disabled="busy" @input="combinedDrafts[combinedFields.get(row.sha)!.root] = ($event.target as HTMLTextAreaElement).value" />
          </label>
        </li>
      </ol>
    </template>

    <footer class="rebase-footer">
      <p v-if="validation || executionError" class="rebase-error" role="alert">{{ validation || executionError }}</p>
      <p v-else-if="plan">Rewrites local commit history. No changes are pushed.</p>
      <div class="rebase-buttons">
        <button type="button" :disabled="busy" @click="close">Cancel</button>
        <button type="button" class="rebase-start" :disabled="busy || loading || !changed || !!validation" @click="submit">
          <Loader2 v-if="busy" :size="14" class="animate-spin" />{{ busy ? 'Rebasing…' : 'Start rebase' }}
        </button>
      </div>
    </footer>
  </dialog>
</template>

<style scoped>
.rebase-dialog { position: fixed; inset: 0; margin: auto; padding: 0; width: min(650px, calc(100vw - 24px)); max-height: min(680px, calc(100dvh - 24px)); border: 1px solid var(--border); border-radius: 8px; background: var(--popover); color: var(--foreground); box-shadow: 0 18px 54px #0005; }
.rebase-dialog[open] { display: flex; flex-direction: column; }
.rebase-dialog::backdrop { background: #0007; }
.rebase-heading { display: flex; align-items: center; gap: 10px; padding: 13px 16px; border-bottom: 1px solid var(--border); }
.rebase-heading h2 { font-size: 15px; font-weight: 650; }
.rebase-heading p { display: flex; align-items: center; gap: 6px; min-width: 0; font-size: 11px; color: var(--muted-foreground); }
.rebase-heading strong { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: var(--foreground); }
.rebase-icon { color: var(--primary); flex: none; }
.rebase-list-label { display: flex; justify-content: space-between; padding: 10px 17px 4px; color: var(--muted-foreground); font-size: 11px; }
.rebase-list { min-height: 0; overflow-y: auto; padding: 0 14px 10px; list-style: none; }
.rebase-row { position: relative; border: 1px solid var(--border); border-radius: 5px; margin-top: 4px; }
.rebase-row[data-grouped='true'] { border-inline-start: 2px solid var(--primary); background: var(--secondary); }
.rebase-row[data-joined='true'] .rebase-subject { padding-inline-start: 10px; }
.rebase-row[data-dropped='true'] :is(code, .rebase-subject) { opacity: .55; text-decoration: line-through; }
.rebase-row[data-drop]::after { content: ''; position: absolute; inset-inline: 0; height: 2px; background: var(--primary); }
.rebase-row[data-drop='before']::after { top: -3px; }
.rebase-row[data-drop='after']::after { bottom: -3px; }
.rebase-row-main { display: grid; grid-template-columns: 23px 55px minmax(0, 1fr) 88px; align-items: center; gap: 5px; padding: 4px 7px; min-height: 34px; font-size: 12px; }
.rebase-grip { height: 24px; display: grid; place-items: center; color: var(--muted-foreground); cursor: grab; }
.rebase-grip:active { cursor: grabbing; }
.rebase-row code { font-size: 11px; color: var(--muted-foreground); }
.rebase-subject { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.rebase-row select, .rebase-message textarea { background: var(--input-background); color: var(--foreground); border: 1px solid var(--border); border-radius: 4px; padding: 3px 5px; }
.rebase-message { display: grid; gap: 4px; padding: 1px 8px 8px 36px; font-size: 11px; color: var(--muted-foreground); }
.rebase-message textarea { width: 100%; resize: vertical; font: 12px/1.45 monospace; }
.rebase-placeholder { display: flex; align-items: center; gap: 8px; padding: 22px 16px; font-size: 12px; overflow: auto; }
.rebase-footer { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 7px; border-top: 1px solid var(--border); padding: 10px 14px; font-size: 11px; color: var(--muted-foreground); }
.rebase-footer p { flex: 1 1 220px; overflow-wrap: anywhere; max-height: 100px; overflow: auto; }
.rebase-footer .rebase-error { color: var(--destructive); }
.rebase-buttons { display: flex; margin-left: auto; gap: 6px; }
.rebase-buttons button { display: flex; align-items: center; gap: 6px; padding: 5px 9px; border: 1px solid var(--border); border-radius: 5px; color: var(--foreground); }
.rebase-buttons .rebase-start { background: var(--primary); color: var(--primary-foreground, white); border-color: var(--primary); }
.rebase-buttons button:disabled { opacity: .5; }
@media (max-width: 420px) {
  .rebase-row-main { grid-template-columns: 19px 48px minmax(0, 1fr) 72px; gap: 3px; padding-inline: 5px; }
  .rebase-list { padding-inline: 8px; }
}
</style>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  Terminal,
  Plus,
  Play,
  Trash2,
  Save,
  Search,
  Globe,
  Folder,
  Zap,
  FileCode,
  X,
  Layers,
  ArrowUp,
  ArrowDown,
  Pencil,
  BarChart3,
  Clock,
} from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import { useToast } from "@/shared/notifications/useToast";
import {
  FAST_SCRIPTS_EVENT,
  FAST_SCRIPT_TYPE_OPTIONS,
  addFastScript,
  addFastScriptSequence,
  deleteFastScript,
  deleteFastScriptSequence,
  getFastScriptRunStats,
  getStoredFastScripts,
  getStoredSequences,
  recordFastScriptRun,
  scriptsForRepo,
  sequencesForRepo,
  updateFastScript,
  updateFastScriptSequence,
  type FastScript,
  type FastScriptDraft,
  type FastScriptRunStat,
  type FastScriptSequence,
  type FastScriptSequenceDraft,
  type FastScriptType,
} from "@/shared/config/fastScriptsPreferences";

const props = defineProps<{
  repoPath: string;
}>();

const emit = defineEmits<{
  close: [];
  run: [payload: { type: FastScriptType; body: string; name: string }];
  runSequence: [payload: { name: string; steps: { type: FastScriptType; body: string; name: string }[] }];
}>();

const toast = useToast();

const scripts = ref<FastScript[]>([]);
const sequences = ref<FastScriptSequence[]>([]);
const runStats = ref<Record<string, FastScriptRunStat>>({});
const search = ref("");
const selectedId = ref<string | null>(null);
const mode = ref<"idle" | "create" | "edit">("idle");
const confirmDeleteId = ref<string | null>(null);
const draft = ref<FastScriptDraft>({
  name: "",
  type: "shell",
  scope: "repo",
  body: "",
});

const selectedSequenceId = ref<string | null>(null);
const sequenceMode = ref<"idle" | "create" | "edit">("idle");
const confirmDeleteSequenceId = ref<string | null>(null);
const sequenceDraft = ref<FastScriptSequenceDraft>({
  name: "",
  scope: "repo",
  scriptIds: [],
});

const editorRef = ref<HTMLTextAreaElement | null>(null);

const typeOptions = FAST_SCRIPT_TYPE_OPTIONS;

const repoScripts = computed(() => scripts.value.filter((script) => script.scope === "repo"));
const globalScripts = computed(() => scripts.value.filter((script) => script.scope === "global"));

const filteredRepoScripts = computed(() => filterScripts(repoScripts.value));
const filteredGlobalScripts = computed(() => filterScripts(globalScripts.value));
const hasVisibleScripts = computed(
  () => filteredRepoScripts.value.length > 0 || filteredGlobalScripts.value.length > 0,
);
const selectedScript = computed(() => scripts.value.find((script) => script.id === selectedId.value) ?? null);
const bodyPlaceholder = computed(() => (draft.value.type === "python" ? 'print("Hello from GitSwamp")' : "npm run build && npm test"));

const visibleSequences = computed(() => sequencesForRepo(sequences.value, props.repoPath));
const selectedSequence = computed(
  () => sequences.value.find((sequence) => sequence.id === selectedSequenceId.value) ?? null,
);
const sequenceStepScripts = computed(() =>
  sequenceDraft.value.scriptIds
    .map((id) => scripts.value.find((script) => script.id === id))
    .filter((script): script is FastScript => script !== undefined),
);
const availableScriptsForSequence = computed(() =>
  filterScripts(scripts.value).filter((script) => !sequenceDraft.value.scriptIds.includes(script.id)),
);

const recentScripts = computed(() => {
  return [...scripts.value]
    .sort((a, b) => {
      const statA = runStats.value[a.id];
      const statB = runStats.value[b.id];
      const lastA = statA?.lastRunAt || 0;
      const lastB = statB?.lastRunAt || 0;
      if (lastA !== lastB) {
        return lastB - lastA;
      }
      const countA = statA?.count || 0;
      const countB = statB?.count || 0;
      if (countA !== countB) {
        return countB - countA;
      }
      return b.updatedAt - a.updatedAt;
    })
    .slice(0, 4);
});

const totalRuns = computed(() =>
  scripts.value.reduce((sum, script) => sum + (runStats.value[script.id]?.count || 0), 0),
);

function filterScripts(list: FastScript[]): FastScript[] {
  const term = search.value.trim().toLowerCase();
  if (!term) {
    return list;
  }

  return list.filter((script) =>
    `${script.name} ${script.type} ${script.body}`.toLowerCase().includes(term),
  );
}

function loadScripts() {
  scripts.value = scriptsForRepo(getStoredFastScripts(), props.repoPath);
  if (selectedId.value && !scripts.value.some((script) => script.id === selectedId.value)) {
    selectedId.value = null;
    if (mode.value === "edit") {
      mode.value = "idle";
    }
  }
}

function loadSequences() {
  sequences.value = getStoredSequences();
  runStats.value = getFastScriptRunStats();
  if (
    selectedSequenceId.value &&
    !visibleSequences.value.some((sequence) => sequence.id === selectedSequenceId.value)
  ) {
    selectedSequenceId.value = null;
    if (sequenceMode.value === "edit") {
      sequenceMode.value = "idle";
    }
  }
}

function typeLabel(type: FastScriptType): string {
  return typeOptions.find((option) => option.value === type)?.label ?? type;
}

function typeHint(type: FastScriptType): string {
  return typeOptions.find((option) => option.value === type)?.hint ?? "";
}

function formatWhen(timestamp: number): string {
  return new Date(timestamp).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function startCreate() {
  selectedId.value = null;
  confirmDeleteId.value = null;
  closeSequenceEditor();
  mode.value = "create";
  draft.value = { name: "", type: "shell", scope: props.repoPath ? "repo" : "global", body: "" };
  focusEditor();
}

function startEdit(script: FastScript) {
  selectedId.value = script.id;
  confirmDeleteId.value = null;
  closeSequenceEditor();
  mode.value = "edit";
  draft.value = {
    name: script.name,
    type: script.type,
    scope: script.scope,
    body: script.body,
  };
  focusEditor();
}

function focusEditor() {
  requestAnimationFrame(() => editorRef.value?.focus());
}

function closeEditor() {
  mode.value = "idle";
  selectedId.value = null;
  confirmDeleteId.value = null;
}

function startCreateSequence() {
  closeEditor();
  selectedSequenceId.value = null;
  confirmDeleteSequenceId.value = null;
  sequenceMode.value = "create";
  sequenceDraft.value = { name: "", scope: props.repoPath ? "repo" : "global", scriptIds: [] };
}

function startEditSequence(sequence: FastScriptSequence) {
  closeEditor();
  selectedSequenceId.value = sequence.id;
  confirmDeleteSequenceId.value = null;
  sequenceMode.value = "edit";
  sequenceDraft.value = {
    name: sequence.name,
    scope: sequence.scope,
    scriptIds: [...sequence.scriptIds].filter((id) => scripts.value.some((script) => script.id === id)),
  };
}

function closeSequenceEditor() {
  sequenceMode.value = "idle";
  selectedSequenceId.value = null;
  confirmDeleteSequenceId.value = null;
}

function addScriptToSequence(script: FastScript) {
  if (!sequenceDraft.value.scriptIds.includes(script.id)) {
    sequenceDraft.value.scriptIds = [...sequenceDraft.value.scriptIds, script.id];
  }
}

function removeScriptFromSequence(index: number) {
  sequenceDraft.value.scriptIds = sequenceDraft.value.scriptIds.filter((_, i) => i !== index);
}

function moveSequenceStep(index: number, direction: -1 | 1) {
  const ids = sequenceDraft.value.scriptIds;
  const target = index + direction;
  if (target < 0 || target >= ids.length) {
    return;
  }

  const next = [...ids];
  [next[index], next[target]] = [next[target], next[index]];
  sequenceDraft.value.scriptIds = next;
}

function saveSequence(showToast = true): FastScriptSequence | null {
  if (sequenceDraft.value.scriptIds.length === 0) {
    toast.warning("Add at least one script to the sequence.");
    return null;
  }

  if (sequenceMode.value === "edit" && selectedSequenceId.value) {
    updateFastScriptSequence(selectedSequenceId.value, sequenceDraft.value, props.repoPath);
  } else {
    const created = addFastScriptSequence(sequenceDraft.value, props.repoPath);
    selectedSequenceId.value = created.id;
    sequenceMode.value = "edit";
  }

  loadSequences();
  if (showToast) {
    toast.success("Sequence saved.");
  }
  return selectedSequence.value;
}

function runSequence(sequence: FastScriptSequence) {
  const steps = sequence.scriptIds
    .map((id) => scripts.value.find((script) => script.id === id))
    .filter((script): script is FastScript => script !== undefined)
    .map((script) => {
      recordFastScriptRun(script.id);
      return runPayload(script);
    });

  if (steps.length === 0) {
    toast.warning("This sequence has no runnable scripts.");
    return;
  }

  runStats.value = getFastScriptRunStats();
  emit("runSequence", { name: sequence.name, steps });
}

function runSequenceDraft() {
  if (sequenceDraft.value.scriptIds.length === 0) {
    toast.warning("Add at least one script to the sequence.");
    return;
  }

  const saved = saveSequence(false);
  const sequence = saved ?? selectedSequence.value;
  const steps = sequenceStepScripts.value.map((script) => {
    recordFastScriptRun(script.id);
    return runPayload(script);
  });

  if (steps.length === 0) {
    toast.warning("This sequence has no runnable scripts.");
    return;
  }

  runStats.value = getFastScriptRunStats();
  emit("runSequence", { name: sequence?.name || "Sequence", steps });
}

function requestDeleteSequence(id: string) {
  if (confirmDeleteSequenceId.value === id) {
    deleteFastScriptSequence(id);
    confirmDeleteSequenceId.value = null;
    if (selectedSequenceId.value === id) {
      closeSequenceEditor();
    }
    loadSequences();
    toast.success("Sequence deleted.");
    return;
  }

  confirmDeleteSequenceId.value = id;
}

function saveDraft(showToast = true): FastScript | null {
  if (!draft.value.body.trim()) {
    toast.warning("Add a script body before saving.");
    return null;
  }

  if (mode.value === "edit" && selectedId.value) {
    updateFastScript(selectedId.value, draft.value, props.repoPath);
  } else {
    const created = addFastScript(draft.value, props.repoPath);
    selectedId.value = created.id;
    mode.value = "edit";
  }

  loadScripts();
  if (showToast) {
    toast.success("Script saved.");
  }
  return selectedScript.value;
}

function runPayload(script: FastScript) {
  return { type: script.type, body: script.body, name: script.name };
}

function runScript(script: FastScript) {
  recordFastScriptRun(script.id);
  runStats.value = getFastScriptRunStats();
  emit("run", runPayload(script));
}

function runDraft() {
  if (!draft.value.body.trim()) {
    toast.warning("Nothing to run yet.");
    return;
  }

  const saved = saveDraft(false);
  const payload = saved
    ? runPayload(saved)
    : { type: draft.value.type, body: draft.value.body, name: draft.value.name || typeLabel(draft.value.type) };
  emit("run", payload);
}

function requestDelete(id: string) {
  if (confirmDeleteId.value === id) {
    deleteFastScript(id);
    confirmDeleteId.value = null;
    if (selectedId.value === id) {
      selectedId.value = null;
      mode.value = "idle";
    }
    loadScripts();
    toast.success("Script deleted.");
    return;
  }

  confirmDeleteId.value = id;
}

function onKeydown(event: KeyboardEvent) {
  const modifier = event.ctrlKey || event.metaKey;
  if (!modifier) {
    return;
  }

  if (sequenceMode.value !== "idle") {
    if (event.key.toLowerCase() === "s") {
      event.preventDefault();
      saveSequence();
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      runSequenceDraft();
      return;
    }

    return;
  }

  if (mode.value === "idle") {
    return;
  }

  if (event.key.toLowerCase() === "s") {
    event.preventDefault();
    saveDraft();
    return;
  }

  if (event.key === "Enter") {
    event.preventDefault();
    runDraft();
  }
}

function handleScriptsChanged() {
  loadScripts();
  loadSequences();
}

watch(() => props.repoPath, () => {
  mode.value = "idle";
  selectedId.value = null;
  sequenceMode.value = "idle";
  selectedSequenceId.value = null;
  loadScripts();
  loadSequences();
});

onMounted(() => {
  loadScripts();
  loadSequences();
  globalThis.addEventListener(FAST_SCRIPTS_EVENT, handleScriptsChanged);
  globalThis.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  globalThis.removeEventListener(FAST_SCRIPTS_EVENT, handleScriptsChanged);
  globalThis.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden bg-[var(--background)] min-h-0">
    <div class="flex items-center justify-between gap-3 border-b border-[var(--border)] px-4 py-3 flex-shrink-0">
      <div class="flex items-center gap-2 min-w-0">
        <Terminal class="h-4 w-4 text-[var(--primary)]" />
        <h2 class="text-sm font-semibold text-[var(--foreground)]">Fast Scripts</h2>
        <span class="rounded-full bg-[var(--primary)]/15 px-2 py-0.5 text-[10px] font-medium text-[var(--primary)]">
          {{ scripts.length }}
        </span>
        <p class="ml-1 hidden truncate text-[10px] text-[var(--muted-foreground)] lg:block">
          Save terminal, Python and custom commands, then run them with one click.
        </p>
      </div>
      <div class="flex items-center gap-2 flex-shrink-0">
        <button
          type="button"
          class="inline-flex items-center gap-1.5 rounded-md bg-[var(--primary)] px-3 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90"
          @click="startCreate"
        >
          <Plus class="h-3.5 w-3.5" />
          New Script
        </button>
        <CloseIconButton title="Close Fast Scripts" @click="emit('close')" />
      </div>
    </div>

    <div class="flex flex-1 overflow-hidden min-h-0">
      <aside class="flex w-[300px] flex-shrink-0 flex-col overflow-hidden border-r border-[var(--border)]">
        <div class="border-b border-[var(--border)] p-3">
          <div class="relative">
            <Search class="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--muted-foreground)]" />
            <input
              v-model="search"
              type="text"
              placeholder="Search scripts..."
              class="w-full rounded border border-[var(--border)] bg-[var(--input-background)] py-1.5 pl-8 pr-2 text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            />
          </div>
        </div>

        <div class="flex-1 overflow-y-auto p-2">
          <div v-if="!hasVisibleScripts" class="px-2 py-6 text-center text-[11px] text-[var(--muted-foreground)]">
            <p v-if="search.trim()">No scripts match "{{ search }}".</p>
            <p v-else>No scripts yet. Create your first one.</p>
          </div>

          <template v-else>
            <div v-if="filteredRepoScripts.length" class="mb-3">
              <div class="mb-1 flex items-center gap-1.5 px-2 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
                <Folder class="h-3 w-3" />
                This repository
              </div>
              <button
                v-for="script in filteredRepoScripts"
                :key="script.id"
                type="button"
                class="group mb-1 flex w-full items-center gap-2 rounded-md border px-2 py-2 text-left transition-colors"
                :class="selectedId === script.id ? 'border-[var(--primary)]/50 bg-[var(--primary)]/10' : 'border-transparent hover:bg-[var(--primary)]/6'"
                @click="startEdit(script)"
              >
                <span class="flex h-6 w-6 flex-shrink-0 items-center justify-center rounded bg-[var(--primary)]/15 text-[var(--primary)]">
                  <Zap class="h-3.5 w-3.5" />
                </span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-xs font-medium text-[var(--foreground)]">{{ script.name }}</span>
                  <span class="block truncate text-[10px] text-[var(--muted-foreground)]">{{ typeLabel(script.type) }} · {{ formatWhen(script.updatedAt) }}</span>
                </span>
                <span
                  class="flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full bg-[var(--primary)]/15 text-[var(--primary)] opacity-0 transition-opacity group-hover:opacity-100"
                  title="Run script"
                  @click.stop="runScript(script)"
                >
                  <Play class="h-3 w-3" />
                </span>
              </button>
            </div>

            <div v-if="filteredGlobalScripts.length">
              <div class="mb-1 flex items-center gap-1.5 px-2 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
                <Globe class="h-3 w-3" />
                All repositories
              </div>
              <button
                v-for="script in filteredGlobalScripts"
                :key="script.id"
                type="button"
                class="group mb-1 flex w-full items-center gap-2 rounded-md border px-2 py-2 text-left transition-colors"
                :class="selectedId === script.id ? 'border-[var(--primary)]/50 bg-[var(--primary)]/10' : 'border-transparent hover:bg-[var(--primary)]/6'"
                @click="startEdit(script)"
              >
                <span class="flex h-6 w-6 flex-shrink-0 items-center justify-center rounded bg-[var(--secondary)] text-[var(--muted-foreground)]">
                  <Globe class="h-3.5 w-3.5" />
                </span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-xs font-medium text-[var(--foreground)]">{{ script.name }}</span>
                  <span class="block truncate text-[10px] text-[var(--muted-foreground)]">{{ typeLabel(script.type) }} · {{ formatWhen(script.updatedAt) }}</span>
                </span>
                <span
                  class="flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full bg-[var(--primary)]/15 text-[var(--primary)] opacity-0 transition-opacity group-hover:opacity-100"
                  title="Run script"
                  @click.stop="runScript(script)"
                >
                  <Play class="h-3 w-3" />
                </span>
              </button>
            </div>
          </template>
        </div>
      </aside>

      <section class="flex-1 overflow-y-auto min-w-0">
        <div
          v-if="mode === 'idle' && sequenceMode !== 'idle'"
          class="mx-auto flex max-w-3xl flex-col gap-4 p-5"
        >
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Layers class="h-4 w-4 text-[var(--primary)]" />
              <h3 class="text-sm font-semibold text-[var(--foreground)]">
                {{ sequenceMode === "edit" ? "Edit sequence" : "New sequence" }}
              </h3>
            </div>
            <button
              type="button"
              class="inline-flex items-center gap-1 text-[11px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
              @click="closeSequenceEditor"
            >
              <X class="h-3.5 w-3.5" />
              Close
            </button>
          </div>

          <p class="text-[11px] leading-relaxed text-[var(--muted-foreground)]">
            Chain saved scripts and run them one after another in a single click.
          </p>

          <div class="grid gap-3 sm:grid-cols-2">
            <label class="flex flex-col gap-1">
              <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--muted-foreground)]">Name</span>
              <input
                v-model="sequenceDraft.name"
                type="text"
                placeholder="e.g. Prepare release"
                class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2.5 py-2 text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              />
            </label>

            <label class="flex flex-col gap-1">
              <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--muted-foreground)]">Where to keep it</span>
              <div class="flex gap-2">
                <button
                  type="button"
                  class="flex flex-1 items-center justify-center gap-1.5 rounded border px-2.5 py-2 text-xs transition-colors"
                  :class="sequenceDraft.scope === 'repo' ? 'border-[var(--primary)]/60 bg-[var(--primary)]/12 text-[var(--foreground)]' : 'border-[var(--border)] bg-[var(--input-background)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                  :disabled="!repoPath"
                  @click="sequenceDraft.scope = 'repo'"
                >
                  <Folder class="h-3.5 w-3.5" />
                  This repository
                </button>
                <button
                  type="button"
                  class="flex flex-1 items-center justify-center gap-1.5 rounded border px-2.5 py-2 text-xs transition-colors"
                  :class="sequenceDraft.scope === 'global' ? 'border-[var(--primary)]/60 bg-[var(--primary)]/12 text-[var(--foreground)]' : 'border-[var(--border)] bg-[var(--input-background)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                  @click="sequenceDraft.scope = 'global'"
                >
                  <Globe class="h-3.5 w-3.5" />
                  All repositories
                </button>
              </div>
            </label>
          </div>

          <div class="grid gap-4 md:grid-cols-2">
            <div class="flex flex-col gap-2">
              <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--muted-foreground)]">Available scripts</span>
              <div class="flex max-h-72 flex-col gap-1 overflow-y-auto rounded border border-[var(--border)] bg-[var(--input-background)] p-2">
                <p v-if="!availableScriptsForSequence.length" class="px-2 py-4 text-center text-[11px] text-[var(--muted-foreground)]">
                  No scripts left to add.
                </p>
                <button
                  v-for="script in availableScriptsForSequence"
                  :key="script.id"
                  type="button"
                  class="flex items-center gap-2 rounded px-2 py-1.5 text-left transition-colors hover:bg-[var(--primary)]/8"
                  @click="addScriptToSequence(script)"
                >
                  <Plus class="h-3.5 w-3.5 flex-shrink-0 text-[var(--primary)]" />
                  <span class="min-w-0 flex-1">
                    <span class="block truncate text-xs text-[var(--foreground)]">{{ script.name }}</span>
                    <span class="block truncate text-[10px] text-[var(--muted-foreground)]">{{ typeLabel(script.type) }}</span>
                  </span>
                </button>
              </div>
            </div>

            <div class="flex flex-col gap-2">
              <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--muted-foreground)]">
                Sequence steps ({{ sequenceDraft.scriptIds.length }})
              </span>
              <div class="flex max-h-72 flex-col gap-1 overflow-y-auto rounded border border-[var(--border)] bg-[var(--input-background)] p-2">
                <p v-if="!sequenceStepScripts.length" class="px-2 py-4 text-center text-[11px] text-[var(--muted-foreground)]">
                  Add scripts from the left to build the sequence.
                </p>
                <div
                  v-for="(script, index) in sequenceStepScripts"
                  :key="`${script.id}-${index}`"
                  class="flex items-center gap-2 rounded bg-[var(--secondary)]/50 px-2 py-1.5"
                >
                  <span class="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-[var(--primary)]/15 text-[10px] font-semibold text-[var(--primary)]">
                    {{ index + 1 }}
                  </span>
                  <span class="min-w-0 flex-1 truncate text-xs text-[var(--foreground)]">{{ script.name }}</span>
                  <button
                    type="button"
                    class="flex h-6 w-6 items-center justify-center rounded text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)] disabled:opacity-30"
                    title="Move up"
                    :disabled="index === 0"
                    @click="moveSequenceStep(index, -1)"
                  >
                    <ArrowUp class="h-3.5 w-3.5" />
                  </button>
                  <button
                    type="button"
                    class="flex h-6 w-6 items-center justify-center rounded text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)] disabled:opacity-30"
                    title="Move down"
                    :disabled="index === sequenceStepScripts.length - 1"
                    @click="moveSequenceStep(index, 1)"
                  >
                    <ArrowDown class="h-3.5 w-3.5" />
                  </button>
                  <button
                    type="button"
                    class="flex h-6 w-6 items-center justify-center rounded text-[#ef4444] transition-colors hover:bg-[#ef4444]/15"
                    title="Remove"
                    @click="removeScriptFromSequence(index)"
                  >
                    <X class="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div class="flex flex-wrap items-center gap-2">
            <button
              type="button"
              class="inline-flex items-center gap-1.5 rounded-md bg-[var(--primary)] px-3.5 py-2 text-xs font-medium text-white transition-opacity hover:opacity-90"
              @click="runSequenceDraft"
            >
              <Play class="h-3.5 w-3.5" />
              Save &amp; Run
            </button>
            <button
              type="button"
              class="inline-flex items-center gap-1.5 rounded-md border border-[var(--border)] bg-[var(--secondary)] px-3.5 py-2 text-xs font-medium text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/40"
              @click="saveSequence()"
            >
              <Save class="h-3.5 w-3.5" />
              Save
            </button>
            <button
              v-if="sequenceMode === 'edit' && selectedSequence"
              type="button"
              class="inline-flex items-center gap-1.5 rounded-md px-3.5 py-2 text-xs font-medium transition-colors"
              :class="confirmDeleteSequenceId === selectedSequence.id ? 'bg-[#ef4444] text-white' : 'bg-[#ef4444]/15 text-[#ef4444] hover:bg-[#ef4444]/25'"
              @click="requestDeleteSequence(selectedSequence.id)"
            >
              <Trash2 class="h-3.5 w-3.5" />
              {{ confirmDeleteSequenceId === selectedSequence.id ? 'Confirm delete' : 'Delete' }}
            </button>
            <span class="ml-auto text-[10px] text-[var(--muted-foreground)]">
              Ctrl/Cmd+S to save · Ctrl/Cmd+Enter to run
            </span>
          </div>
        </div>

        <div v-else-if="mode === 'idle'" class="mx-auto flex max-w-4xl flex-col gap-5 p-5">
          <div v-if="!scripts.length" class="flex flex-col items-center justify-center gap-3 px-8 py-16 text-center">
            <span class="flex h-14 w-14 items-center justify-center rounded-2xl bg-[var(--primary)]/12 text-[var(--primary)]">
              <Terminal class="h-7 w-7" />
            </span>
            <h3 class="text-base font-semibold text-[var(--foreground)]">Automate your terminal</h3>
            <p class="max-w-md text-xs leading-relaxed text-[var(--muted-foreground)]">
              Store the shell, Python and custom commands you run often. Pick a repository or make a script global,
              then run it straight in the integrated terminal with a single click.
            </p>
            <button
              type="button"
              class="mt-1 inline-flex items-center gap-1.5 rounded-md bg-[var(--primary)] px-4 py-2 text-xs font-medium text-white transition-opacity hover:opacity-90"
              @click="startCreate"
            >
              <Plus class="h-3.5 w-3.5" />
              Create a script
            </button>
          </div>

          <template v-else>
            <div class="flex flex-col gap-2">
              <div class="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
                <BarChart3 class="h-3 w-3" />
                Overview
              </div>
              <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
                <div class="rounded-lg border border-[var(--border)] bg-[var(--input-background)] p-3">
                  <p class="text-lg font-semibold text-[var(--foreground)]">{{ scripts.length }}</p>
                  <p class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Total scripts</p>
                </div>
                <div class="rounded-lg border border-[var(--border)] bg-[var(--input-background)] p-3">
                  <p class="text-lg font-semibold text-[var(--foreground)]">{{ repoScripts.length }}</p>
                  <p class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">This repo</p>
                </div>
                <div class="rounded-lg border border-[var(--border)] bg-[var(--input-background)] p-3">
                  <p class="text-lg font-semibold text-[var(--foreground)]">{{ globalScripts.length }}</p>
                  <p class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Global</p>
                </div>
                <div class="rounded-lg border border-[var(--border)] bg-[var(--input-background)] p-3">
                  <p class="text-lg font-semibold text-[var(--foreground)]">{{ totalRuns }}</p>
                  <p class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Executions</p>
                </div>
              </div>
            </div>

            <div v-if="recentScripts.length" class="flex flex-col gap-2">
              <div class="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
                <Clock class="h-3 w-3" />
                Recent scripts
              </div>
              <div class="grid gap-2 sm:grid-cols-2">
                <button
                  v-for="script in recentScripts"
                  :key="script.id"
                  type="button"
                  class="group flex items-center gap-2 rounded-lg border border-[var(--border)] bg-[var(--input-background)] px-3 py-2.5 text-left transition-colors hover:border-[var(--primary)]/40"
                  @click="startEdit(script)"
                >
                  <span class="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded bg-[var(--primary)]/15 text-[var(--primary)]">
                    <Zap class="h-4 w-4" />
                  </span>
                  <span class="min-w-0 flex-1">
                    <span class="block truncate text-xs font-medium text-[var(--foreground)]">{{ script.name }}</span>
                    <span class="block truncate text-[10px] text-[var(--muted-foreground)]">
                      {{ typeLabel(script.type) }}
                      <template v-if="runStats[script.id]?.count">
                        · {{ runStats[script.id].count }} runs
                      </template>
                      <template v-else>· never run</template>
                    </span>
                  </span>
                  <span
                    class="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-full bg-[var(--primary)]/15 text-[var(--primary)] opacity-0 transition-opacity group-hover:opacity-100"
                    title="Run script"
                    @click.stop="runScript(script)"
                  >
                    <Play class="h-3.5 w-3.5" />
                  </span>
                </button>
              </div>
            </div>

            <div class="flex flex-col gap-2">
              <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
                  <Layers class="h-3 w-3" />
                  Sequences
                </div>
                <button
                  type="button"
                  class="inline-flex items-center gap-1 rounded-md border border-[var(--border)] bg-[var(--secondary)] px-2.5 py-1 text-[11px] font-medium text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/40"
                  @click="startCreateSequence"
                >
                  <Plus class="h-3 w-3" />
                  New sequence
                </button>
              </div>

              <p v-if="!visibleSequences.length" class="rounded-lg border border-dashed border-[var(--border)] px-3 py-6 text-center text-[11px] text-[var(--muted-foreground)]">
                No sequences yet. Chain scripts together to run them in order.
              </p>

              <div v-else class="flex flex-col gap-2">
                <div
                  v-for="sequence in visibleSequences"
                  :key="sequence.id"
                  class="flex items-center gap-3 rounded-lg border border-[var(--border)] bg-[var(--input-background)] px-3 py-2.5"
                >
                  <span class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded bg-[var(--primary)]/15 text-[var(--primary)]">
                    <Layers class="h-4 w-4" />
                  </span>
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                      <span class="truncate text-xs font-medium text-[var(--foreground)]">{{ sequence.name }}</span>
                      <span class="flex-shrink-0 rounded-full bg-[var(--secondary)] px-1.5 py-0.5 text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">
                        {{ sequence.scope === 'global' ? 'Global' : 'Repo' }}
                      </span>
                    </div>
                    <span class="block truncate text-[10px] text-[var(--muted-foreground)]">
                      {{ sequence.scriptIds.length }} steps · {{ sequence.scriptIds.map((id) => scripts.find((s) => s.id === id)?.name || "missing").join(" → ") }}
                    </span>
                  </div>
                  <div class="flex flex-shrink-0 items-center gap-1">
                    <button
                      type="button"
                      class="inline-flex items-center gap-1 rounded-md bg-[var(--primary)] px-2.5 py-1.5 text-[11px] font-medium text-white transition-opacity hover:opacity-90"
                      @click="runSequence(sequence)"
                    >
                      <Play class="h-3 w-3" />
                      Run
                    </button>
                    <button
                      type="button"
                      class="flex h-7 w-7 items-center justify-center rounded text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
                      title="Edit sequence"
                      @click="startEditSequence(sequence)"
                    >
                      <Pencil class="h-3.5 w-3.5" />
                    </button>
                    <button
                      type="button"
                      class="flex h-7 items-center justify-center rounded px-1.5 text-[11px] font-medium transition-colors"
                      :class="confirmDeleteSequenceId === sequence.id ? 'bg-[#ef4444] text-white' : 'text-[#ef4444] hover:bg-[#ef4444]/15'"
                      :title="confirmDeleteSequenceId === sequence.id ? 'Confirm delete' : 'Delete sequence'"
                      @click="requestDeleteSequence(sequence.id)"
                    >
                      <Trash2 class="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </template>
        </div>

        <div v-if="mode !== 'idle'" class="mx-auto flex max-w-3xl flex-col gap-4 p-5">
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <FileCode class="h-4 w-4 text-[var(--primary)]" />
              <h3 class="text-sm font-semibold text-[var(--foreground)]">
                {{ mode === "edit" ? "Edit script" : "New script" }}
              </h3>
            </div>
            <button
              type="button"
              class="inline-flex items-center gap-1 text-[11px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
              @click="closeEditor"
            >
              <X class="h-3.5 w-3.5" />
              Close
            </button>
          </div>

          <div class="grid gap-3 sm:grid-cols-2">
            <label class="flex flex-col gap-1">
              <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--muted-foreground)]">Name <span class="normal-case">(optional)</span></span>
              <input
                v-model="draft.name"
                type="text"
                placeholder="e.g. Rebuild and run tests"
                class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2.5 py-2 text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              />
            </label>

            <label class="flex flex-col gap-1">
              <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--muted-foreground)]">Script type</span>
              <select
                v-model="draft.type"
                class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2.5 py-2 text-xs text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              >
                <option v-for="option in typeOptions" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </label>
          </div>

          <div class="grid gap-3 sm:grid-cols-2">
            <label class="flex flex-col gap-1">
              <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--muted-foreground)]">Where to keep it</span>
              <div class="flex gap-2">
                <button
                  type="button"
                  class="flex flex-1 items-center justify-center gap-1.5 rounded border px-2.5 py-2 text-xs transition-colors"
                  :class="draft.scope === 'repo' ? 'border-[var(--primary)]/60 bg-[var(--primary)]/12 text-[var(--foreground)]' : 'border-[var(--border)] bg-[var(--input-background)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                  :disabled="!repoPath"
                  @click="draft.scope = 'repo'"
                >
                  <Folder class="h-3.5 w-3.5" />
                  This repository
                </button>
                <button
                  type="button"
                  class="flex flex-1 items-center justify-center gap-1.5 rounded border px-2.5 py-2 text-xs transition-colors"
                  :class="draft.scope === 'global' ? 'border-[var(--primary)]/60 bg-[var(--primary)]/12 text-[var(--foreground)]' : 'border-[var(--border)] bg-[var(--input-background)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                  @click="draft.scope = 'global'"
                >
                  <Globe class="h-3.5 w-3.5" />
                  All repositories
                </button>
              </div>
            </label>

            <div class="flex flex-col justify-end gap-1">
              <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--muted-foreground)]">Runs with</span>
              <p class="rounded bg-[var(--secondary)]/50 px-2.5 py-2 text-[11px] leading-snug text-[var(--muted-foreground)]">
                {{ typeHint(draft.type) }}
              </p>
            </div>
          </div>

          <label class="flex flex-col gap-1">
            <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--muted-foreground)]">Script body</span>
            <textarea
              ref="editorRef"
              v-model="draft.body"
              rows="13"
              spellcheck="false"
              :placeholder="bodyPlaceholder"
              class="w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] p-3 font-mono text-xs leading-relaxed text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            ></textarea>
          </label>

          <div class="flex flex-wrap items-center gap-2">
            <button
              type="button"
              class="inline-flex items-center gap-1.5 rounded-md bg-[var(--primary)] px-3.5 py-2 text-xs font-medium text-white transition-opacity hover:opacity-90"
              @click="runDraft"
            >
              <Play class="h-3.5 w-3.5" />
              Save &amp; Run
            </button>
            <button
              type="button"
              class="inline-flex items-center gap-1.5 rounded-md border border-[var(--border)] bg-[var(--secondary)] px-3.5 py-2 text-xs font-medium text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/40"
              @click="saveDraft()"
            >
              <Save class="h-3.5 w-3.5" />
              Save
            </button>
            <button
              v-if="mode === 'edit' && selectedScript"
              type="button"
              class="inline-flex items-center gap-1.5 rounded-md px-3.5 py-2 text-xs font-medium transition-colors"
              :class="confirmDeleteId === selectedScript.id ? 'bg-[#ef4444] text-white' : 'bg-[#ef4444]/15 text-[#ef4444] hover:bg-[#ef4444]/25'"
              @click="requestDelete(selectedScript.id)"
            >
              <Trash2 class="h-3.5 w-3.5" />
              {{ confirmDeleteId === selectedScript.id ? 'Confirm delete' : 'Delete' }}
            </button>
            <span class="ml-auto text-[10px] text-[var(--muted-foreground)]">
              Ctrl/Cmd+S to save · Ctrl/Cmd+Enter to run
            </span>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

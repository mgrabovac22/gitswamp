<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onBeforeUnmount, onMounted, reactive, ref, shallowRef, watch } from "vue";
import { ArrowLeft, ArrowUpRight, Columns3, FolderOpen, GitBranch, LockKeyhole, Plus, RefreshCw } from "lucide-vue-next";
import { invoke } from "@tauri-apps/api/core";
import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";
import { useToast } from "@/shared/notifications/useToast";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import type { BranchInfo } from "@/types";
import BranchWorkspaceColumn from "./BranchWorkspaceColumn.vue";
import { isOriginalWorktree, sameWorkspacePath, type WorkspaceLocation, type WorkspaceRemovalRequest } from "./branchWorkspaceTypes";

const DiffViewer = defineAsyncComponent(() => import("@/shared/ui/FileDiffViewer.vue"));
const ConflictResolver = defineAsyncComponent(() => import("@/shared/ui/ConflictResolver.vue"));
const props = defineProps<{
  repoPath: string; currentBranch: string; localBranches: BranchInfo[]; remoteBranches: BranchInfo[];
  request?: { branchRef: string } | null;
  removedWorktree?: { path: string } | null;
}>();
const emit = defineEmits<{
  close: []; requestConsumed: []; openRepository: [location: WorkspaceLocation];
  removeWorktree: [request: WorkspaceRemovalRequest]; busyChange: [busy: boolean];
}>();
const MAX_COLUMNS = 4;
const toast = useToast();
const storageKey = `gitswamp-branch-workspaces:${props.repoPath}`;
const options = computed(() => {
  const local = props.localBranches.map(branch => ({ ref: `refs/heads/${branch.name}`, name: branch.name, remote: false }));
  const names = new Set(props.localBranches.map(branch => branch.name));
  const remote = props.remoteBranches.flatMap(branch => {
    const name = branch.name.slice(branch.name.indexOf("/") + 1);
    return name !== "HEAD" && !names.has(name) ? [{ ref: `refs/remotes/${branch.name}`, name: branch.name, remote: true }] : [];
  });
  return [...local, ...remote];
});

function normalizedRef(branchRef: string) {
  if (!branchRef.startsWith("refs/remotes/")) return branchRef;
  const name = branchRef.slice("refs/remotes/".length);
  const local = props.localBranches.find(branch => branch.upstream === name && name.slice(name.indexOf("/") + 1) === branch.name);
  return local ? `refs/heads/${local.name}` : branchRef;
}

function initialRefs(): string[] {
  let saved: unknown = null;
  try { saved = JSON.parse(safeStorageGet(storageKey) || "null"); } catch { /* Invalid preferences are ignored. */ }
  const choices = new Set(options.value.map(option => option.ref));
  const stored = Array.isArray(saved) ? saved.filter((item): item is string => typeof item === "string") : [];
  const restored = [...new Set(stored.map(normalizedRef).filter(item => choices.has(item) || (!choices.size && /^refs\/(heads|remotes)\//.test(item))))];
  if (props.request) return [...new Set([normalizedRef(props.request.branchRef), ...restored])].slice(0, MAX_COLUMNS);
  if (restored.length) return restored.slice(0, MAX_COLUMNS);
  const current = `refs/heads/${props.currentBranch}`;
  return choices.has(current) ? [current] : [];
}

const columns = ref(initialRefs());
const active = ref(columns.value[0] ?? "");
const nextBranch = ref("");
const grid = ref<HTMLElement | null>(null);
const instances = new Map<string, InstanceType<typeof BranchWorkspaceColumn>>();
const locations = reactive(new Map<string, WorkspaceLocation>());
const inspection = shallowRef<{ branchRef: string; branch: string; repoPath: string; path: string; sha?: string | null; staged?: boolean; conflict?: boolean } | null>(null);
const available = computed(() => options.value.filter(option => !columns.value.includes(option.ref)));
const inspectorKey = computed(() => inspection.value ? JSON.stringify(inspection.value) : "");
const diffViewer = ref<{ hasUnsavedChanges: boolean; saving: boolean } | null>(null);
const openTabs = computed(() => columns.value.map(branchRef => {
  const location = locations.get(branchRef);
  return { branchRef, name: location?.branch ?? branchRef.replace(/^refs\/(heads|remotes)\//, ""), original: !!location && isOriginalWorktree(props.repoPath, location) };
}));
const activeTab = computed(() => openTabs.value.find(tab => tab.branchRef === active.value));
const activeLocation = computed(() => locations.get(active.value));
const canAdd = computed(() => columns.value.length < MAX_COLUMNS && available.value.length > 0);
const busyColumns = ref(new Set<string>());
const busy = computed(() => busyColumns.value.size > 0 || Boolean(diffViewer.value?.saving));
const refreshingAll = ref(false);
let cursor = 0;
let polling = false;
let timer: ReturnType<typeof setInterval> | undefined;

function focus(branchRef: string) {
  active.value = branchRef;
  void nextTick(() => {
    const index = columns.value.indexOf(branchRef);
    if (index >= 0) grid.value?.children.item(index)?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
}

function add(branchRef: string) {
  if (!branchRef) return;
  const ref = normalizedRef(branchRef);
  if (columns.value.includes(ref)) { focus(ref); return; }
  if (columns.value.length >= MAX_COLUMNS) { toast.info("Close a workspace before opening a fifth branch."); return; }
  columns.value = [...columns.value, ref];
  focus(ref);
}

function addSelected() { add(nextBranch.value); nextBranch.value = ""; }

function closeColumn(branchRef: string) {
  const index = columns.value.indexOf(branchRef);
  if (index < 0 || busyColumns.value.has(branchRef)) return;
  if (inspection.value?.branchRef === branchRef) inspection.value = null;
  columns.value = columns.value.filter(item => item !== branchRef);
  locations.delete(branchRef);
  if (active.value === branchRef) active.value = columns.value[Math.min(index, columns.value.length - 1)] ?? "";
}

function requestCloseColumn(branchRef: string) {
  if (busyColumns.value.has(branchRef)) return;
  const location = locations.get(branchRef);
  if (!location) { closeColumn(branchRef); return; }
  if (isOriginalWorktree(props.repoPath, location)) return;
  emit("removeWorktree", { sourcePath: props.repoPath, location, finish: () => closeColumn(branchRef) });
}

function workspaceReady(branchRef: string, location: WorkspaceLocation) {
  const same = [...locations].find(([other, item]) => other !== branchRef && sameWorkspacePath(item.path, location.path));
  if (same) { closeColumn(branchRef); focus(same[0]); return; }
  locations.set(branchRef, location);
}

function register(branchRef: string, instance: unknown) {
  if (instance) instances.set(branchRef, instance as InstanceType<typeof BranchWorkspaceColumn>);
  else instances.delete(branchRef);
}

function setBusy(branchRef: string, busy: boolean) {
  const next = new Set(busyColumns.value);
  if (busy) next.add(branchRef); else next.delete(branchRef);
  busyColumns.value = next;
}

watch(columns, items => safeStorageSet(storageKey, JSON.stringify(items)), { immediate: true });
watch(busy, value => emit("busyChange", value), { flush: "sync" });
watch(() => props.removedWorktree, removed => {
  if (!removed) return;
  for (const [branchRef, location] of locations) {
    if (sameWorkspacePath(location.path, removed.path)) closeColumn(branchRef);
  }
});
watch(() => props.request, request => {
  if (!request) return;
  add(request.branchRef);
  emit("requestConsumed");
}, { immediate: true });

function refreshActive() {
  if (!document.hidden) void instances.get(active.value)?.refresh();
}

async function refreshAll() {
  if (refreshingAll.value) return;
  refreshingAll.value = true;
  try {
    for (const branchRef of columns.value) await instances.get(branchRef)?.refresh();
  } finally { refreshingAll.value = false; }
}

async function revealFolder() {
  const location = activeLocation.value;
  if (!location) return;
  try { await invoke("open_path_with_tool", { path: location.path, tool: "explorer" }); }
  catch (error) { toast.error(`Could not open ${location.branch}: ${String(error)}`); }
}

onMounted(() => {
  timer = setInterval(async () => {
    if (document.hidden || polling || refreshingAll.value || !columns.value.length) return;
    polling = true;
    try { await instances.get(columns.value[cursor++ % columns.value.length])?.refresh(); }
    finally { polling = false; }
  }, 2500);
  window.addEventListener("focus", refreshActive);
});
onBeforeUnmount(() => {
  clearInterval(timer);
  window.removeEventListener("focus", refreshActive);
  emit("busyChange", false);
});

function refreshInspection() {
  if (inspection.value) void instances.get(inspection.value.branchRef)?.refresh(true);
}

function closeInspection() {
  if (diffViewer.value?.saving) return;
  if (diffViewer.value?.hasUnsavedChanges && !window.confirm("Close the diff without saving the editor draft?")) return;
  inspection.value = null;
}
</script>

<template>
  <section class="branch-workspaces" aria-label="Branch Workspace">
    <header class="workspace-header">
      <Columns3 :size="19" class="workspace-icon" />
      <h2>Branch Workspace</h2>
      <div v-if="activeTab" class="active-worktree" :title="activeLocation?.path ?? activeTab.name" aria-label="Active worktree">
        <GitBranch :size="13" /><span>Active</span><strong>{{ activeTab.name }}</strong>
      </div>
      <span class="workspace-count" aria-label="Open workspaces">{{ columns.length }} / 4</span>
      <CloseIconButton title="Close Branch Workspace" size="sm" :disabled="busy" @click="emit('close')" />
    </header>

    <div class="workspace-body">
      <aside class="workspace-tools" aria-label="Workspace tools">
        <button type="button" class="original-repository" title="Return to original graph" :disabled="busy" @click="emit('close')">
          <ArrowLeft :size="15" /><span><small>Original graph</small><strong :title="currentBranch">{{ currentBranch || 'HEAD' }}</strong></span>
        </button>
        <form class="workspace-add" @submit.prevent="addSelected">
          <select v-model="nextBranch" aria-label="Choose a branch" :disabled="!canAdd">
            <option value="" disabled>{{ columns.length >= MAX_COLUMNS ? 'Limit reached' : 'Choose a branch' }}</option>
            <option v-for="option in available" :key="option.ref" :value="option.ref">{{ option.name }}</option>
          </select>
          <button type="submit" class="add-workspace" title="Open branch workspace" aria-label="Open branch workspace" :disabled="!nextBranch || !canAdd"><Plus :size="16" /></button>
        </form>
        <div class="workspace-actions">
          <button type="button" title="Refresh workspaces" :disabled="refreshingAll || !columns.length" @click="refreshAll"><RefreshCw :size="15" :class="{ 'animate-spin': refreshingAll }" /><span>Refresh</span></button>
          <button type="button" title="Open active worktree in repository tab" :disabled="!activeLocation || busy" @click="activeLocation && emit('openRepository', activeLocation)"><ArrowUpRight :size="15" /><span>Open in repo tab</span></button>
          <button type="button" title="Show active worktree folder" :disabled="!activeLocation" @click="revealFolder"><FolderOpen :size="15" /><span>Show folder</span></button>
        </div>
      </aside>

      <div class="workspace-content">
        <nav v-if="openTabs.length" class="workspace-tabs" aria-label="Open branch workspaces">
          <div v-for="tab in openTabs" :key="tab.branchRef" class="workspace-tab" :data-active="active === tab.branchRef">
            <button type="button" :aria-current="active === tab.branchRef ? 'true' : undefined" :title="tab.name" @click="focus(tab.branchRef)">
              <GitBranch :size="13" /><span>{{ tab.name }}</span>
            </button>
            <LockKeyhole v-if="tab.original" :size="13" class="protected-tab" aria-label="Original worktree is protected" />
            <CloseIconButton v-else :title="`Close ${tab.name} workspace`" size="sm" subtle :disabled="busyColumns.has(tab.branchRef)" @click="requestCloseColumn(tab.branchRef)" />
          </div>
        </nav>
        <div v-if="columns.length" ref="grid" class="workspace-grid" :style="{ '--workspace-columns': columns.length }">
          <BranchWorkspaceColumn v-for="branchRef in columns" :key="branchRef" :ref="instance => register(branchRef, instance)"
            :repo-path="repoPath" :branch-ref="branchRef" :active="active === branchRef"
            @activate="active = branchRef" @ready="workspaceReady(branchRef, $event)"
            @busy-change="setBusy(branchRef, $event)" @open-repository="emit('openRepository', $event)"
            @inspect="inspection = { branchRef, ...$event }" />
        </div>
        <div v-else class="workspace-empty"><GitBranch :size="28" /><strong>No workspaces open</strong><span v-if="!options.length">No branches with commits yet</span></div>
      </div>
    </div>

    <div v-if="inspection" class="workspace-inspection">
      <div class="inspection-title"><GitBranch :size="15" /><strong :title="inspection.repoPath">{{ inspection.branch }}</strong></div>
      <ConflictResolver v-if="inspection.conflict" :key="inspectorKey" class="inspection-content" :repo-path="inspection.repoPath" :file-path="inspection.path" :expected-branch="inspection.branch" embedded
        @close="inspection = null" @resolved="refreshInspection(); inspection = null" />
      <DiffViewer v-else ref="diffViewer" :key="inspectorKey" class="inspection-content" :repo-path="inspection.repoPath" :file-path="inspection.path" :commit-sha="inspection.sha" :staged="inspection.staged" :expected-branch="inspection.branch"
        @close="closeInspection" @refresh="refreshInspection" />
    </div>
  </section>
</template>

<style scoped>
.branch-workspaces { position: relative; display: flex; flex-direction: column; flex: 1; min-height: 0; min-width: 0; overflow: hidden; background: var(--background); }
.workspace-header { display: flex; align-items: center; gap: 9px; padding: 7px 10px; border-bottom: 1px solid var(--border); background: var(--card); }
.workspace-icon { color: var(--primary); flex: none; }
.workspace-header h2 { flex: 1; min-width: 0; font-size: 14px; font-weight: 700; }
.active-worktree { display: flex; align-items: center; gap: 6px; min-width: 0; max-width: 45%; padding: 4px 6px; color: var(--primary); background: var(--secondary); border-radius: 5px; font-size: 11px; }
.active-worktree svg { flex: none; }
.active-worktree strong { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.active-worktree span { color: var(--muted-foreground); }
.workspace-count { font: 11px monospace; white-space: nowrap; color: var(--muted-foreground); }
.workspace-body { display: flex; min-height: 0; flex: 1; }
.workspace-tools { flex: 0 0 158px; min-width: 0; display: flex; flex-direction: column; gap: 10px; padding: 8px; border-right: 1px solid var(--sidebar-border); background: var(--sidebar); color: var(--sidebar-foreground); }
.original-repository, .workspace-actions button { width: 100%; display: flex; align-items: center; gap: 7px; min-height: 29px; padding: 4px 6px; border-radius: 5px; text-align: left; font-size: 11px; }
.original-repository { background: var(--sidebar-accent); color: var(--sidebar-foreground); }
.original-repository svg, .workspace-actions svg { color: var(--sidebar-primary); flex: none; }
.original-repository span { min-width: 0; }
.original-repository strong, .original-repository small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.original-repository small { font-size: 10px; color: var(--muted-foreground); }
.workspace-actions button:not(:disabled):hover, .original-repository:not(:disabled):hover { background: var(--accent); }
.workspace-actions button:disabled { opacity: .5; }
.workspace-add { display: flex; align-items: center; gap: 5px; min-width: 0; }
.workspace-add select, .add-workspace { height: 28px; color: var(--foreground); background: var(--secondary); border: 1px solid var(--border); border-radius: 5px; font-size: 12px; }
.workspace-add select { flex: 1; min-width: 0; padding: 0 5px; }
.add-workspace { width: 28px; display: grid; place-items: center; flex: 0 0 28px; }
.add-workspace:not(:disabled):hover { background: var(--accent); }
.workspace-add :disabled { opacity: .55; }
.workspace-content { display: flex; flex-direction: column; min-width: 0; flex: 1; }
.workspace-tabs { display: flex; gap: 5px; padding: 5px 6px 0; overflow-x: auto; flex: none; }
.workspace-tab { display: flex; align-items: center; flex: 1 1 0; min-width: 104px; border: 1px solid var(--border); border-radius: 6px 6px 3px 3px; color: var(--muted-foreground); }
.workspace-tab > button:first-child { display: flex; align-items: center; gap: 5px; padding: 5px 6px; min-width: 0; flex: 1; text-align: left; font-size: 11px; }
.workspace-tab[data-active="true"] { border-color: color-mix(in srgb, var(--primary) 55%, var(--border)); background: var(--secondary); color: var(--foreground); }
.protected-tab { margin-inline: 7px; color: var(--muted-foreground); }
.workspace-tabs button svg { flex: none; }
.workspace-tabs button span, .inspection-title strong { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.workspace-grid { min-height: 0; flex: 1; display: grid; grid-template-columns: repeat(var(--workspace-columns), minmax(225px, 1fr)); grid-template-rows: minmax(0, 1fr); gap: 6px; overflow: auto; padding: 6px; }
.workspace-empty { display: grid; flex: 1; align-content: center; justify-items: center; gap: 8px; color: var(--muted-foreground); font-size: 13px; }
.workspace-inspection { position: absolute; inset: 0; z-index: 10; display: flex; flex-direction: column; background: var(--card); }
.inspection-title { display: flex; align-items: center; gap: 6px; padding: 6px 12px; color: var(--primary); background: var(--secondary); border-bottom: 1px solid var(--border); font-size: 12px; }
.inspection-content { flex: 1; min-height: 0; }
@media (max-width: 470px) {
  .workspace-header { gap: 5px; padding-inline: 7px; }
  .workspace-header h2 { font-size: 12px; }
  .workspace-header > .workspace-icon { display: none; }
}
@media (max-width: 600px) {
  .workspace-body { flex-direction: column; }
  .workspace-tools { flex: none; flex-direction: row; align-items: center; gap: 6px; padding: 4px 6px; border-right: 0; border-bottom: 1px solid var(--sidebar-border); }
  .original-repository { width: auto; }
  .original-repository span, .workspace-actions button span, .active-worktree span { display: none; }
  .workspace-add { flex: 1; }
  .workspace-actions { display: flex; }
  .workspace-actions button { width: 28px; }
  .workspace-content { min-height: 0; }
  .workspace-grid { grid-template-columns: repeat(var(--workspace-columns), minmax(100%, 1fr)); scroll-snap-type: x proximity; scroll-padding-inline: 6px; }
  .workspace-grid > * { scroll-snap-align: start; }
}
</style>

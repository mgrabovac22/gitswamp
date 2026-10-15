<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  X,
  Plus,
  FolderPlus,
  History,
  Globe,
  Server,
  Cookie as CookieIcon,
  Trash2,
  Copy,
  ClipboardPaste,
  ChevronRight,
  Pencil,
  Layers,
  Clock,
  Search,
  Play,
  FileUp,
  FileDown,
  ArrowUp,
  ArrowDown,
  Variable,
  FlaskConical,
  Activity,
} from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import { useToast } from "@/shared/notifications/useToast";
import { useHttpClient } from "../useHttpClient";
import {
  HTTP_CLIENT_COMMAND_EVENT,
  consumePendingHttpClientCommand,
  type HttpClientCommand,
  type HttpClientCommandEvent,
} from "../httpClientCommands";
import type { HttpCollection, HttpHistoryEntry, HttpRequestDef, HttpRunStats, HttpRunStep, HttpScope } from "../httpClient.types";
import { HTTP_METHODS } from "../httpClient.types";
import { formatBytes, formatDuration, hostnameFromUrl, parseExportBundle, serializeExportBundle, statusTone } from "../httpClient.utils";
import { openJsonFile, saveJsonFile } from "../httpClientFiles";
import HttpKeyValueTable from "./HttpKeyValueTable.vue";
import HttpRequestEditor from "./HttpRequestEditor.vue";
import HttpResponseViewer from "./HttpResponseViewer.vue";
import HttpCookieManager from "./HttpCookieManager.vue";
import HttpHeaderSetsDialog from "./HttpHeaderSetsDialog.vue";
import HttpRunDetailsView from "./HttpRunDetailsView.vue";
import HttpSimulationView from "./HttpSimulationView.vue";

const props = defineProps<{
  repoPath: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

const controller = useHttpClient(() => props.repoPath);
const toast = useToast();

const sideTab = ref<"collections" | "history" | "environments">("collections");
const showCookieManager = ref(false);
const showHeaderSets = ref(false);
const showNewCollection = ref(false);
const newCollectionName = ref("");
const newCollectionScope = ref<HttpScope>("repo");
const showImportCurl = ref(false);
const importCurlText = ref("");
const historyFilter = ref("");
const editingCollectionId = ref<string | null>(null);
const editingRequestId = ref<string | null>(null);
const renameValue = ref("");
const collapsedCollections = ref<string[]>([]);
const showDeleteCollection = ref(false);
const deleteTargetCollection = ref<HttpCollection | null>(null);

type MainView = "requests" | "run" | "simulated";

const centerView = ref<MainView>("requests");
const runDetailsCollectionId = ref<string | null>(null);

function openRunDetails(collectionId: string | null): void {
  runDetailsCollectionId.value = collectionId;
  centerView.value = "run";
}


const methodTone: Record<string, string> = {
  GET: "text-[#10b981]",
  POST: "text-[#f59e0b]",
  PUT: "text-[#3b82f6]",
  PATCH: "text-[#8b5cf6]",
  DELETE: "text-[#ef4444]",
  HEAD: "text-[#06b6d4]",
  OPTIONS: "text-[#a855f7]",
};

const statusToneClass: Record<string, string> = {
  success: "text-[#10b981]",
  redirect: "text-[#3b82f6]",
  warning: "text-[#f59e0b]",
  danger: "text-[#ef4444]",
  muted: "text-[var(--muted-foreground)]",
};

interface FlatCollection {
  collection: HttpCollection;
  depth: number;
  stats: HttpRunStats | null;
}

const flatCollections = computed<FlatCollection[]>(() => {
  const all = controller.visibleCollections.value;
  const stats = controller.collectionStats.value;
  const byParent = new Map<string | null, HttpCollection[]>();
  for (const collection of all) {
    const key = collection.parentId;
    byParent.set(key, [...(byParent.get(key) ?? []), collection]);
  }
  for (const list of byParent.values()) list.sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
  const out: FlatCollection[] = [];
  const walk = (parentId: string | null, depth: number) => {
    if (depth > 8) return;
    for (const collection of byParent.get(parentId) ?? []) {
      out.push({ collection, depth, stats: stats[collection.id] ?? null });
      walk(collection.id, depth + 1);
    }
  };
  walk(null, 0);
  return out;
});

function requestsInCollection(collectionId: string): HttpRequestDef[] {
  return controller.visibleRequests.value
    .filter((request) => request.collectionId === collectionId)
    .sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
}

const unfiledRequests = computed(() => {
  const collectionIds = new Set(controller.visibleCollections.value.map((collection) => collection.id));
  return controller.visibleRequests.value
    .filter((request) => !request.collectionId || !collectionIds.has(request.collectionId))
    .sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
});

const filteredHistory = computed<HttpHistoryEntry[]>(() => {
  const term = historyFilter.value.trim().toLowerCase();
  if (!term) return controller.history.value;
  return controller.history.value.filter(
    (entry) => entry.url.toLowerCase().includes(term) || entry.method.toLowerCase().includes(term),
  );
});

const runProgressLabel = computed(() => {
  const steps = controller.runSteps.value;
  if (!steps.length) return "";
  const done = steps.filter((step) => step.status !== "pending").length;
  return `${done}/${steps.length}`;
});

const activeCollectionName = computed(() => {
  const request = controller.activeRequest.value;
  if (!request?.collectionId) return "Requests";
  return controller.collections.value.find((collection) => collection.id === request.collectionId)?.name ?? "Requests";
});

const lastStatusTone = computed(() => statusTone(controller.response.value?.status ?? 0));

function methodBadgeClass(method: string): string {
  return methodTone[method] ?? "text-[var(--foreground)]";
}

function lastStepFor(requestId: string): HttpRunStep | undefined {
  return [...controller.runSteps.value].reverse().find((step) => step.requestId === requestId);
}

function stepToneClass(requestId: string): string {
  const step = lastStepFor(requestId);
  if (!step) return "bg-[var(--muted-foreground)]";
  if (step.status === "running") return "bg-[var(--primary)] animate-pulse";
  if (step.status === "passed") return "bg-[#10b981]";
  if (step.status === "failed") return "bg-[#ef4444]";
  return "bg-[var(--muted-foreground)]";
}

function stepTitle(requestId: string): string {
  const step = lastStepFor(requestId);
  if (!step) return "";
  const parts: string[] = [step.status];
  if (step.httpStatus) parts.push(String(step.httpStatus));
  if (step.durationMs) parts.push(formatDuration(step.durationMs));
  if (step.iterations > 1) parts.push(`x${step.iterations}`);
  return `Last run: ${parts.join(" · ")}`;
}

function statsTitle(stats: HttpRunStats): string {
  return `${stats.passed} passed · ${stats.failed} failed · avg ${formatDuration(stats.avgMs)} · p95 ${formatDuration(stats.p95Ms)}`;
}

function toggleCollapse(id: string): void {
  collapsedCollections.value = collapsedCollections.value.includes(id)
    ? collapsedCollections.value.filter((value) => value !== id)
    : [...collapsedCollections.value, id];
}

function startRename(collection: HttpCollection): void {
  editingCollectionId.value = collection.id;
  renameValue.value = collection.name;
}

function commitRename(): void {
  if (editingCollectionId.value) {
    controller.renameCollection(editingCollectionId.value, renameValue.value);
  }
  editingCollectionId.value = null;
}

function startRequestRename(request: HttpRequestDef): void {
  editingRequestId.value = request.id;
  renameValue.value = request.name;
}

function commitRequestRename(): void {
  if (editingRequestId.value) {
    controller.renameRequest(editingRequestId.value, renameValue.value);
  }
  editingRequestId.value = null;
}

const dragRequestId = ref<string | null>(null);
const dropCollectionId = ref<string | null>(null);

function onRowDragStart(event: DragEvent, requestId: string): void {
  dragRequestId.value = requestId;
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", requestId);
  }
}

function onRowDragEnd(): void {
  dragRequestId.value = null;
  dropCollectionId.value = null;
}

function onCollectionDragOver(event: DragEvent, collectionId: string): void {
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
  dropCollectionId.value = collectionId;
}

function onCollectionDragLeave(collectionId: string): void {
  if (dropCollectionId.value === collectionId) dropCollectionId.value = null;
}

function onCollectionDrop(event: DragEvent, collectionId: string | null): void {
  event.preventDefault();
  const requestId = dragRequestId.value || event.dataTransfer?.getData("text/plain");
  if (requestId) controller.moveRequestToCollection(requestId, collectionId);
  onRowDragEnd();
}

function createCollection(scope: HttpScope, name = ""): void {
  const collection = controller.createCollection();
  collection.scope = scope;
  collection.repoPath = scope === "global" ? "" : props.repoPath;
  const trimmed = name.trim();
  if (trimmed) collection.name = trimmed;
  sideTab.value = "collections";
  if (trimmed) {
    editingCollectionId.value = null;
  } else {
    startRename(collection);
  }
}

function openNewCollection(): void {
  newCollectionName.value = "";
  newCollectionScope.value = props.repoPath ? "repo" : "global";
  showNewCollection.value = true;
}

function confirmNewCollection(): void {
  createCollection(newCollectionScope.value, newCollectionName.value);
  showNewCollection.value = false;
}

function newRequestInCollection(collection: HttpCollection): void {
  controller.newRequest(collection.id);
}

function loadHistory(entry: HttpHistoryEntry): void {
  if (entry.requestId && controller.requests.value.some((request) => request.id === entry.requestId)) {
    controller.openRequest(entry.requestId);
    return;
  }
  const created = controller.newRequest(null, entry.method);
  created.name = entry.request.name;
  created.url = entry.request.url;
  created.params = entry.request.params.map((item) => ({ ...item }));
  created.headers = entry.request.headers.map((item) => ({ ...item }));
  created.cookies = entry.request.cookies.map((item) => ({ ...item }));
  created.auth = { ...entry.request.auth };
  created.body = { ...entry.request.body, form: [], urlencoded: [] };
  created.updatedAt = Date.now();
}

function applyImportCurl(): void {
  const text = importCurlText.value.trim();
  if (!text) return;
  controller.importCurl(text);
  importCurlText.value = "";
  showImportCurl.value = false;
}

function openCookieManager(): void {
  showCookieManager.value = true;
}

function slug(value: string): string {
  return value.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "gitswamp-http";
}

async function exportBundle(fileName: string, bundle: ReturnType<typeof controller.bundleForWorkspace> | null): Promise<void> {
  if (!bundle) return;
  try {
    const saved = await saveJsonFile(`${slug(fileName)}.json`, serializeExportBundle(bundle));
    if (saved) toast.success("Export saved.");
  } catch (error) {
    toast.error(`Export failed: ${String(error)}`);
  }
}

function exportWorkspace(): void {
  void exportBundle(`gitswamp-http-workspace-${new Date().toISOString().slice(0, 10)}`, controller.bundleForWorkspace());
}

function exportCollection(collection: HttpCollection): void {
  void exportBundle(`gitswamp-http-${collection.name}`, controller.bundleForCollection(collection.id));
}

function exportRequest(request: HttpRequestDef): void {
  void exportBundle(`gitswamp-http-${request.name}`, controller.bundleForRequest(request.id));
}

function exportHeaderDocuments(): void {
  void exportBundle(`gitswamp-http-headers-${new Date().toISOString().slice(0, 10)}`, controller.bundleForHeaderDocuments());
}

async function importFromFile(): Promise<void> {
  try {
    const content = await openJsonFile();
    if (!content) return;
    const bundle = parseExportBundle(content);
    if (!bundle) {
      toast.error("Not a GitSwamp HTTP export file.");
      return;
    }
    const scope: HttpScope | undefined = props.repoPath ? undefined : "global";
    const result = controller.importBundle(bundle, { scope, repoPath: props.repoPath || undefined });
    sideTab.value = "collections";
    toast.success(`Imported ${result.requests} request(s), ${result.collections} collection(s), ${result.headerDocuments} header set(s).`);
  } catch (error) {
    toast.error(`Import failed: ${String(error)}`);
  }
}

function startRun(mode: "request" | "collection" | "all", collectionId?: string): void {
  if (mode === "request") controller.runActiveRequest();
  else if (mode === "collection" && collectionId) controller.runCollection(collectionId);
  else controller.runAll();
  runDetailsCollectionId.value = mode === "collection" && collectionId ? collectionId : null;
  centerView.value = "run";
}

function executeCommand(command: HttpClientCommand): void {
  switch (command) {
    case "open-cookie-manager":
      openCookieManager();
      break;
    case "add-header": {
      const request = controller.activeRequest.value;
      if (request) {
        request.headers = [...request.headers, { id: `kv_${Math.random().toString(36).slice(2, 9)}`, key: "", value: "", enabled: true }];
      }
      break;
    }
    case "add-cookie":
      openCookieManager();
      break;
    case "copy-curl":
      controller.copyAsCurl();
      break;
    case "clear-domain-cookies": {
      const request = controller.activeRequest.value;
      const host = hostnameFromUrl(request?.url ?? "");
      if (host) controller.clearJarDomain(host);
      break;
    }
    case "run-active":
      startRun("request");
      break;
    case "run-all":
      startRun("all");
      break;
  }
}

function confirmDeleteCollection(collection: HttpCollection): void {
  deleteTargetCollection.value = collection;
  showDeleteCollection.value = true;
}

function executeDeleteCollection(): void {
  if (deleteTargetCollection.value) {
    controller.deleteCollection(deleteTargetCollection.value.id);
  }
  showDeleteCollection.value = false;
  deleteTargetCollection.value = null;
}

function cancelDeleteCollection(): void {
  showDeleteCollection.value = false;
  deleteTargetCollection.value = null;
}

function openSimulated(): void {
  centerView.value = "simulated";
}

function onCommand(event: Event): void {
  const detail = (event as CustomEvent<HttpClientCommandEvent>).detail;
  if (!detail) return;
  if (detail.repoPath && detail.repoPath !== props.repoPath) return;
  executeCommand(detail.command);
}

onMounted(() => {
  window.addEventListener(HTTP_CLIENT_COMMAND_EVENT, onCommand);
  const pending = consumePendingHttpClientCommand(props.repoPath);
  if (pending) openNextTick(pending);
});

onUnmounted(() => window.removeEventListener(HTTP_CLIENT_COMMAND_EVENT, onCommand));

function openNextTick(command: HttpClientCommand): void {
  window.setTimeout(() => executeCommand(command), 0);
}
</script>

<template>
  <div class="flex min-h-0 flex-1 overflow-hidden">
    <div class="flex min-w-0 flex-1 flex-col overflow-hidden">
      <div class="flex items-center gap-1 border-b border-[var(--border)] bg-[var(--secondary)]/30 pl-2 pr-1">
        <div class="http-tabstrip flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
          <button
            v-for="request in controller.openRequests.value"
            :key="request.id"
            type="button"
            class="group flex max-w-[190px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-t border-b-2 px-2.5 py-2 text-[11px] transition-colors"
            :class="controller.activeId.value === request.id ? 'border-[var(--primary)] text-[var(--foreground)]' : 'border-transparent text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
            :title="`${request.method} ${request.url || request.name}`"
            @click="controller.openRequest(request.id)"
            @mousedown.middle="controller.closeTab(request.id)"
          >
            <span class="font-mono text-[9px] font-semibold" :class="methodBadgeClass(request.method)">{{ request.method }}</span>
            <span class="truncate">{{ request.name }}</span>
            <span
              class="flex h-4 w-4 items-center justify-center rounded text-[var(--muted-foreground)] transition-opacity hover:bg-[var(--secondary)] hover:text-[var(--foreground)]"
              :class="controller.activeId.value === request.id ? 'opacity-70 hover:opacity-100' : 'opacity-0 group-hover:opacity-100'"
              title="Close tab"
              @click.stop="controller.closeTab(request.id)"
            >
              <X class="h-3 w-3" />
            </span>
          </button>
          <button
            type="button"
            class="flex h-6 w-6 shrink-0 items-center justify-center rounded text-[var(--muted-foreground)] transition-colors hover:bg-[var(--primary)]/12 hover:text-[var(--primary)]"
            title="New request"
            @click="controller.newRequest()"
          >
            <Plus class="h-3.5 w-3.5" />
          </button>
        </div>

        <CloseIconButton size="sm" title="Close APIPulse" @click="emit('close')" />
      </div>

      <HttpRunDetailsView
        v-if="centerView === 'run'"
        :controller="controller"
        :collection-id="runDetailsCollectionId"
        @back="centerView = 'requests'; runDetailsCollectionId = null"
      />
      <HttpSimulationView
        v-else-if="centerView === 'simulated'"
        :controller="controller"
        @back="centerView = 'requests'"
      />
      <template v-else>
        <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
          <HttpRequestEditor
            :controller="controller"
            @open-cookie-manager="openCookieManager"
            @open-header-sets="showHeaderSets = true"
          />
          <HttpResponseViewer :controller="controller" />
        </div>
      </template>

      <div class="flex items-center gap-3 border-t border-[var(--border)] bg-[var(--secondary)]/30 px-3 py-1 text-[10px] text-[var(--muted-foreground)]">
        <span class="inline-flex items-center gap-1"><Layers class="h-3 w-3" /> {{ activeCollectionName }}</span>
        <span>{{ controller.visibleRequests.value.length }} requests</span>
        <span class="inline-flex items-center gap-1" title="Cookies in the jar for this host"><CookieIcon class="h-3 w-3" /> {{ controller.activeCookieCount.value }}</span>
        <span class="ml-auto" :class="statusToneClass[lastStatusTone]">
          <template v-if="controller.runActive.value">
            Running… {{ runProgressLabel }}
          </template>
          <template v-else-if="centerView === 'simulated' && controller.simulationActive.value">
            Simulating…
          </template>
          <template v-else-if="controller.response.value && centerView === 'requests'">
            {{ controller.response.value.status }} · {{ formatDuration(controller.response.value.durationMs) }} · {{ formatBytes(controller.response.value.sizeBytes) }}
          </template>
          <template v-else-if="controller.loading.value">Sending…</template>
          <template v-else>Ready</template>
        </span>
      </div>
    </div>

    <div class="flex w-[300px] shrink-0 flex-col overflow-hidden border-l border-[var(--border)]">
      <div class="flex w-full items-stretch gap-0.5 border-b border-[var(--border)] px-1.5">
        <button
          v-for="tab in [['collections', Layers], ['history', History], ['environments', Globe]] as const"
          :key="tab[0]"
          type="button"
          class="relative flex min-w-0 flex-1 items-center justify-center gap-1 px-2 py-2 text-[11px] font-medium capitalize transition-colors"
          :class="sideTab === tab[0] ? 'text-[var(--primary)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
          @click="sideTab = tab[0]"
        >
          <component :is="tab[1]" class="h-3.5 w-3.5" />
          {{ tab[0] }}
          <span v-if="sideTab === tab[0]" class="absolute inset-x-2 bottom-0 h-0.5 rounded-full bg-[var(--primary)]"></span>
        </button>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto">
        <template v-if="sideTab === 'collections'">
          <div class="flex items-center gap-1 border-b border-[var(--border)]/60 px-2 py-1.5">
            <button type="button" class="inline-flex items-center gap-1 rounded px-1.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]" title="Create a collection" @click="openNewCollection">
              <FolderPlus class="h-3 w-3" /> New collection
            </button>
            <button type="button" class="inline-flex items-center gap-1 rounded px-1.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[#10b981]" title="Run all requests sequentially" @click="startRun('all')">
              <Play class="h-3 w-3" /> Run all
            </button>
            <button type="button" class="ml-auto inline-flex items-center gap-1 rounded px-1.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]" title="Import cURL" @click="showImportCurl = !showImportCurl">
              <ClipboardPaste class="h-3 w-3" /> cURL
            </button>
            <button type="button" class="inline-flex items-center gap-1 rounded px-1.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]" title="Import JSON bundle" @click="importFromFile">
              <FileUp class="h-3 w-3" />
            </button>
            <button type="button" class="inline-flex items-center gap-1 rounded px-1.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]" title="Export everything as JSON" @click="exportWorkspace">
              <FileDown class="h-3 w-3" />
            </button>
          </div>

          <div v-if="showImportCurl" class="border-b border-[var(--border)]/60 p-2">
            <textarea
              v-model="importCurlText"
              rows="3"
              spellcheck="false"
              placeholder="curl -X POST https://api.example.com -H 'Content-Type: application/json' -d '{}'"
              class="w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] p-1.5 font-mono text-[10px] text-[var(--foreground)] focus:outline-none"
            ></textarea>
            <button type="button" class="mt-1 rounded bg-[var(--primary)] px-2 py-1 text-[10px] font-medium text-white" @click="applyImportCurl">Import</button>
          </div>

          <div
            class="py-1"
            :class="dragRequestId && dropCollectionId === '' ? 'mx-1 rounded border border-[var(--primary)]/60 bg-[var(--primary)]/10' : ''"
            @dragover="onCollectionDragOver($event, '')"
            @dragleave="onCollectionDragLeave('')"
            @drop="onCollectionDrop($event, null)"
          >
            <div class="px-2 py-1 text-[9px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">Requests</div>
            <div v-if="unfiledRequests.length === 0" class="px-2 pb-1 text-[10px] text-[var(--muted-foreground)]">
              Drop a request here to unfile it.
            </div>
            <div
              v-for="request in unfiledRequests"
              :key="request.id"
              role="button"
              tabindex="0"
              draggable="true"
              class="group flex w-full cursor-grab items-center gap-1.5 px-2 py-1 text-left text-[11px] transition-colors hover:bg-[var(--primary)]/10 active:cursor-grabbing"
              :class="controller.activeId.value === request.id ? 'bg-[var(--primary)]/10 text-[var(--foreground)]' : 'text-[var(--muted-foreground)]'"
              @click="controller.openRequest(request.id)"
              @keydown.enter="controller.openRequest(request.id)"
              @dragstart="onRowDragStart($event, request.id)"
              @dragend="onRowDragEnd"
            >
              <span class="shrink-0 font-mono text-[9px] font-semibold" :class="methodBadgeClass(request.method)">{{ request.method }}</span>
              <input
                v-if="editingRequestId === request.id"
                :value="renameValue"
                class="min-w-0 flex-1 rounded border border-[var(--border)] bg-[var(--input-background)] px-1 text-[11px] text-[var(--foreground)] focus:outline-none"
                @click.stop
                @input="renameValue = ($event.target as HTMLInputElement).value"
                @keydown.enter.stop="commitRequestRename"
                @keydown.esc="editingRequestId = null"
                @blur="commitRequestRename"
              >
              <span v-else class="truncate">{{ request.name }}</span>
              <span class="ml-auto flex items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                <span class="cursor-pointer rounded p-0.5 hover:text-[var(--primary)]" title="Rename" @click.stop="startRequestRename(request)"><Pencil class="h-3 w-3" /></span>
                <span class="cursor-pointer rounded p-0.5 hover:text-[#ef4444]" title="Delete" @click.stop="controller.deleteRequest(request.id)"><Trash2 class="h-3 w-3" /></span>
              </span>
            </div>
          </div>

          <div
            v-for="node in flatCollections"
            :key="node.collection.id"
            class="py-0.5"
            @dragover="onCollectionDragOver($event, node.collection.id)"
            @dragleave="onCollectionDragLeave(node.collection.id)"
            @drop="onCollectionDrop($event, node.collection.id)"
          >
            <div
              class="group flex items-center gap-1 px-1.5 py-1 transition-colors"
              :class="[dropCollectionId === node.collection.id && dragRequestId ? 'border border-dashed border-[var(--primary)]/70 bg-[var(--primary)]/12' : 'hover:bg-[var(--primary)]/8']"
              :style="{ paddingLeft: `${6 + node.depth * 12}px` }"
              :title="node.collection.scope === 'global' ? 'Global collection' : 'Repository collection'"
            >
              <button type="button" class="flex h-4 w-4 items-center justify-center text-[var(--muted-foreground)]" @click="toggleCollapse(node.collection.id)">
                <ChevronRight class="h-3 w-3 transition-transform" :class="collapsedCollections.includes(node.collection.id) ? '' : 'rotate-90'" />
              </button>
              <Globe v-if="node.collection.scope === 'global'" class="h-3 w-3 text-[#06b6d4]" />
              <Layers v-else class="h-3 w-3 text-[var(--muted-foreground)]" />
              <input
                v-if="editingCollectionId === node.collection.id"
                :value="renameValue"
                class="min-w-0 flex-1 rounded border border-[var(--border)] bg-[var(--input-background)] px-1 text-[11px] text-[var(--foreground)] focus:outline-none"
                @input="renameValue = ($event.target as HTMLInputElement).value"
                @keydown.enter="commitRename"
                @keydown.esc="editingCollectionId = null"
                @blur="commitRename"
              >
              <span v-else class="truncate text-[11px] font-medium text-[var(--foreground)]">{{ node.collection.name }}</span>
              <span
                v-if="node.stats"
                class="rounded px-1 py-0.5 text-[8px] font-medium"
                :class="node.stats.failed ? 'bg-[#ef4444]/15 text-[#ef4444]' : 'bg-[#10b981]/15 text-[#10b981]'"
                :title="statsTitle(node.stats)"
              >
                {{ node.stats.passed }}/{{ node.stats.total }} · {{ formatDuration(node.stats.avgMs) }}
              </span>
              <div class="ml-auto flex items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                <button type="button" class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[#10b981]" title="Run collection sequentially" @click="startRun('collection', node.collection.id)">
                  <Play class="h-3 w-3" />
                </button>
                <button type="button" class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--primary)]" title="New request" @click="newRequestInCollection(node.collection)">
                  <Plus class="h-3 w-3" />
                </button>
                <button type="button" class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--foreground)]" title="Rename" @click="startRename(node.collection)">
                  <Pencil class="h-3 w-3" />
                </button>
                <button type="button" class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--foreground)]" title="Export collection" @click="exportCollection(node.collection)">
                  <FileDown class="h-3 w-3" />
                </button>
                <button type="button" class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[#ef4444]" title="Delete" @click="confirmDeleteCollection(node.collection)">
                  <Trash2 class="h-3 w-3" />
                </button>
              </div>
            </div>

            <template v-if="!collapsedCollections.includes(node.collection.id)">
              <div
                v-for="request in requestsInCollection(node.collection.id)"
                :key="request.id"
                role="button"
                tabindex="0"
                draggable="true"
                class="group flex w-full cursor-grab items-center gap-1.5 py-1 pr-2 text-left text-[11px] transition-colors hover:bg-[var(--primary)]/10 active:cursor-grabbing"
                :style="{ paddingLeft: `${28 + node.depth * 12}px` }"
                :class="controller.activeId.value === request.id ? 'bg-[var(--primary)]/10 text-[var(--foreground)]' : 'text-[var(--muted-foreground)]'"
                @click="controller.openRequest(request.id)"
                @keydown.enter="controller.openRequest(request.id)"
                @dragstart="onRowDragStart($event, request.id)"
                @dragend="onRowDragEnd"
              >
                <span class="shrink-0 font-mono text-[9px] font-semibold" :class="methodBadgeClass(request.method)">{{ request.method }}</span>
                <input
                  v-if="editingRequestId === request.id"
                  :value="renameValue"
                  class="min-w-0 flex-1 rounded border border-[var(--border)] bg-[var(--input-background)] px-1 text-[11px] text-[var(--foreground)] focus:outline-none"
                  @click.stop
                  @input="renameValue = ($event.target as HTMLInputElement).value"
                  @keydown.enter.stop="commitRequestRename"
                  @keydown.esc="editingRequestId = null"
                  @blur="commitRequestRename"
                >
                <span v-else class="truncate">{{ request.name }}</span>
                <span v-if="request.extractors.length" class="text-[9px] text-[var(--primary)]" title="Has extractors"><Variable class="h-2.5 w-2.5" /></span>
                <span
                  v-if="controller.runSteps.value.some((step) => step.requestId === request.id)"
                  class="h-1.5 w-1.5 shrink-0 rounded-full"
                  :class="stepToneClass(request.id)"
                  :title="stepTitle(request.id)"
                ></span>
                <span class="ml-auto flex items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                  <span class="cursor-pointer rounded p-0.5 hover:text-[var(--primary)]" title="Rename" @click.stop="startRequestRename(request)"><Pencil class="h-3 w-3" /></span>
                  <span class="cursor-pointer rounded p-0.5 hover:text-[var(--foreground)]" title="Move up" @click.stop="controller.reorderRequestInCollection(request.id, -1)"><ArrowUp class="h-3 w-3" /></span>
                  <span class="cursor-pointer rounded p-0.5 hover:text-[var(--foreground)]" title="Move down" @click.stop="controller.reorderRequestInCollection(request.id, 1)"><ArrowDown class="h-3 w-3" /></span>
                  <span class="cursor-pointer rounded p-0.5 hover:text-[var(--foreground)]" title="Duplicate" @click.stop="controller.duplicateRequest(request.id)"><Copy class="h-3 w-3" /></span>
                  <span class="cursor-pointer rounded p-0.5 hover:text-[var(--foreground)]" title="Export request" @click.stop="exportRequest(request)"><FileDown class="h-3 w-3" /></span>
                  <span class="cursor-pointer rounded p-0.5 hover:text-[#ef4444]" title="Delete" @click.stop="controller.deleteRequest(request.id)"><Trash2 class="h-3 w-3" /></span>
                </span>
              </div>

              <div
                v-if="node.stats"
                class="mb-1 mr-1.5 mt-0.5 flex items-center gap-1.5 rounded border border-[var(--border)]/70 bg-[var(--card)] px-1.5 py-0.5 text-[9px] text-[var(--muted-foreground)]"
                :style="{ marginLeft: `${28 + node.depth * 12}px` }"
              >
                <Activity class="h-2.5 w-2.5 text-[var(--muted-foreground)]" />
                <span :class="node.stats.failed ? 'text-[#ef4444]' : 'text-[#10b981]'">{{ node.stats.passed }}/{{ node.stats.total }}</span>
                <span>· {{ formatDuration(node.stats.avgMs) }}</span>
                <button type="button" class="ml-auto inline-flex items-center text-[var(--primary)] hover:opacity-80" title="Open run details" @click="openRunDetails(node.collection.id)">
                  <ChevronRight class="h-3 w-3" />
                </button>
              </div>
            </template>
          </div>

          <div v-if="flatCollections.length === 0 && unfiledRequests.length === 0" class="px-3 py-8 text-center text-[11px] text-[var(--muted-foreground)]">
            No saved requests yet.
          </div>

          <div v-if="controller.runStats.value" class="m-2 rounded-md border border-[var(--border)] bg-[var(--card)] p-2">
            <div class="mb-1.5 flex items-center gap-1.5">
              <Activity class="h-3 w-3 text-[var(--muted-foreground)]" />
              <span class="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">Run summary</span>
              <button
                type="button"
                class="ml-auto inline-flex items-center gap-1 rounded border border-[var(--border)] px-1.5 py-0.5 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--primary)]"
                @click="openRunDetails(null)"
              >
                <ChevronRight class="h-3 w-3" /> Details
              </button>
            </div>
            <div class="grid grid-cols-3 gap-1.5 text-center">
              <div class="rounded bg-[var(--secondary)]/60 px-1 py-1">
                <div class="text-[13px] font-semibold text-[var(--foreground)]">{{ controller.runStats.value.total }}</div>
                <div class="text-[9px] text-[var(--muted-foreground)]">requests</div>
              </div>
              <div class="rounded bg-[#10b981]/10 px-1 py-1">
                <div class="text-[13px] font-semibold text-[#10b981]">{{ controller.runStats.value.passed }}</div>
                <div class="text-[9px] text-[var(--muted-foreground)]">passed</div>
              </div>
              <div class="rounded px-1 py-1" :class="controller.runStats.value.failed ? 'bg-[#ef4444]/10' : 'bg-[var(--secondary)]/60'">
                <div class="text-[13px] font-semibold" :class="controller.runStats.value.failed ? 'text-[#ef4444]' : 'text-[var(--foreground)]'">{{ controller.runStats.value.failed }}</div>
                <div class="text-[9px] text-[var(--muted-foreground)]">failed</div>
              </div>
            </div>
            <div class="mt-1.5 flex items-center justify-between text-[9px] text-[var(--muted-foreground)]">
              <span>avg {{ formatDuration(controller.runStats.value.avgMs) }}</span>
              <span>p95 {{ formatDuration(controller.runStats.value.p95Ms) }}</span>
              <span>total {{ formatDuration(controller.runStats.value.totalMs) }}</span>
            </div>
          </div>
        </template>

        <template v-else-if="sideTab === 'history'">
          <div class="flex items-center gap-1 border-b border-[var(--border)]/60 px-2 py-1.5">
            <div class="relative flex-1">
              <Search class="pointer-events-none absolute left-1.5 top-1/2 h-3 w-3 -translate-y-1/2 text-[var(--muted-foreground)]" />
              <input v-model="historyFilter" type="text" placeholder="Filter history" class="w-full rounded border border-[var(--border)] bg-[var(--input-background)] py-1 pl-6 pr-2 text-[10px] text-[var(--foreground)] focus:outline-none" >
            </div>
            <button type="button" class="rounded p-1 text-[var(--muted-foreground)] hover:text-[#ef4444]" title="Clear history" @click="controller.clearHistory()">
              <Trash2 class="h-3 w-3" />
            </button>
          </div>
          <button
            v-for="entry in filteredHistory"
            :key="entry.id"
            type="button"
            class="flex w-full flex-col gap-0.5 border-b border-[var(--border)]/40 px-2 py-1.5 text-left transition-colors hover:bg-[var(--primary)]/8"
            @click="loadHistory(entry)"
          >
            <div class="flex items-center gap-1.5">
              <span class="font-mono text-[9px] font-semibold" :class="methodBadgeClass(entry.method)">{{ entry.method }}</span>
              <span class="text-[11px]" :class="statusToneClass[statusTone(entry.status)]">{{ entry.status || "ERR" }}</span>
              <span class="ml-auto inline-flex items-center gap-0.5 text-[9px] text-[var(--muted-foreground)]"><Clock class="h-2.5 w-2.5" /> {{ formatDuration(entry.durationMs) }}</span>
            </div>
            <span class="truncate font-mono text-[10px] text-[var(--muted-foreground)]" :title="entry.url">{{ entry.url }}</span>
          </button>
          <div v-if="filteredHistory.length === 0" class="px-3 py-8 text-center text-[11px] text-[var(--muted-foreground)]">No history.</div>
        </template>

        <template v-else>
          <div class="flex items-center gap-1 border-b border-[var(--border)]/60 px-2 py-1.5">
            <select
              :value="controller.activeEnvironmentId.value"
              class="flex-1 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-[11px] text-[var(--foreground)] focus:outline-none"
              @change="controller.selectEnvironment(($event.target as HTMLSelectElement).value)"
            >
              <option v-for="env in controller.environments.value" :key="env.id" :value="env.id">{{ env.name }}</option>
            </select>
            <button type="button" class="rounded p-1 text-[var(--muted-foreground)] hover:text-[var(--primary)]" title="New environment" @click="controller.addEnvironment()">
              <Plus class="h-3 w-3" />
            </button>
            <button
              type="button"
              class="rounded p-1 transition-colors"
              :class="(controller.activeEnvironment.value?.name ?? '').trim().toLowerCase() === 'local' ? 'cursor-not-allowed text-[var(--muted-foreground)]/40' : 'text-[var(--muted-foreground)] hover:text-[#ef4444]'"
              :title="(controller.activeEnvironment.value?.name ?? '').trim().toLowerCase() === 'local' ? 'The Local environment cannot be deleted' : 'Delete environment'"
              @click="controller.activeEnvironment.value && controller.deleteEnvironment(controller.activeEnvironment.value.id)"
            >
              <Trash2 class="h-3 w-3" />
            </button>
          </div>

          <div v-if="controller.activeEnvironment.value" class="p-2">
            <div class="mb-1 text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Variables</div>
            <HttpKeyValueTable
              :model-value="controller.activeEnvironment.value.variables"
              key-placeholder="name"
              value-placeholder="value"
              @update:model-value="(value) => { if (controller.activeEnvironment.value) controller.activeEnvironment.value.variables = value; }"
            />
          </div>

          <div class="border-t border-[var(--border)]/60 p-2">
            <div class="mb-1 flex items-center justify-between">
              <span class="inline-flex items-center gap-1 text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]"><Variable class="h-3 w-3" /> Session variables</span>
              <button
                v-if="Object.keys(controller.sessionVariables.value).length"
                type="button"
                class="text-[10px] text-[var(--primary)] hover:underline"
                @click="controller.clearSessionVariables()"
              >
                Clear
              </button>
            </div>
            <div v-if="Object.keys(controller.sessionVariables.value).length === 0" class="text-[10px] text-[var(--muted-foreground)]">
              Values captured by extractors during runs appear here.
            </div>
            <div v-for="(value, name) in controller.sessionVariables.value" :key="name" class="flex items-baseline gap-1 py-0.5 text-[10px]">
              <span class="font-mono text-[var(--primary)]">{{ name }}</span>
              <span class="text-[var(--muted-foreground)]">=</span>
              <span class="min-w-0 flex-1 truncate font-mono text-[var(--foreground)]" :title="String(value)">{{ value }}</span>
            </div>
          </div>

          <div class="border-t border-[var(--border)]/60 p-2">
            <div class="mb-1 flex items-center justify-between">
              <span class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Cookie jar</span>
              <button type="button" class="text-[10px] text-[var(--primary)] hover:underline" @click="openCookieManager">Manage</button>
            </div>
            <div class="flex items-center gap-2 text-[11px] text-[var(--muted-foreground)]">
              <CookieIcon class="h-3 w-3" />
              {{ controller.jarDomains.value.length }} domains · {{ controller.activeCookieCount.value }} cookies
            </div>
            <label class="mt-1.5 flex items-center gap-1.5 text-[11px] text-[var(--muted-foreground)]">
              <input type="checkbox" class="h-3.5 w-3.5 accent-[var(--primary)]" :checked="controller.settings.value.autoManageCookies" @change="controller.settings.value.autoManageCookies = ($event.target as HTMLInputElement).checked" >
              Auto-manage cookie jar
            </label>
          </div>

          <div class="border-t border-[var(--border)]/60 p-2">
            <div class="mb-1 flex items-center gap-1 text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]"><Server class="h-3 w-3" /> Request defaults</div>
            <label class="flex items-center justify-between text-[11px] text-[var(--muted-foreground)]">
              Timeout (ms)
              <input
                type="number"
                class="w-20 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-0.5 text-[10px] text-[var(--foreground)] focus:outline-none"
                :value="controller.settings.value.timeoutMs"
                min="1000"
                step="1000"
                @input="controller.settings.value.timeoutMs = Number(($event.target as HTMLInputElement).value) || 30000"
              >
            </label>
            <label class="mt-1.5 flex items-center gap-1.5 text-[11px] text-[var(--muted-foreground)]">
              <input type="checkbox" class="h-3.5 w-3.5 accent-[var(--primary)]" :checked="controller.settings.value.followRedirects" @change="controller.settings.value.followRedirects = ($event.target as HTMLInputElement).checked" >
              Follow redirects
            </label>
          </div>

          <div class="border-t border-[var(--border)]/60 p-2">
            <div class="mb-1 flex items-center gap-1 text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]"><Globe class="h-3 w-3" /> Environments explained</div>
            <p class="text-[10px] leading-snug text-[var(--muted-foreground)]">
              An environment is a named set of variables such as <span class="font-mono text-[var(--primary)]">baseUrl</span> or a token.
              Use them anywhere in a request as
              <code class="ml-0.5 rounded bg-[var(--secondary)] px-1 font-mono text-[var(--primary)]" v-text="'{{baseUrl}}/users'"></code>
              and the active environment supplies the value. Switching environments re-points every request.
              Values captured by extractors go to session (below); environment-scoped extractors are written back here.
            </p>
          </div>

          <div class="border-t border-[var(--border)]/60 p-2 text-[10px] text-[var(--muted-foreground)]">
            <div class="mb-1 font-mono">{{ props.repoPath || "No repository selected" }}</div>
            <div>{{ HTTP_METHODS.length }} methods supported</div>
          </div>
        </template>
      </div>

      <div class="flex items-center gap-1 border-t border-[var(--border)] px-1.5 py-1">
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded px-1.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--primary)]"
          :class="centerView === 'simulated' ? 'bg-[var(--primary)]/10 text-[var(--primary)]' : ''"
          title="Simulated tests (smoke, load, spike and more)"
          @click="openSimulated()"
        >
          <FlaskConical class="h-3.5 w-3.5" /> Simulated tests
        </button>
        <span v-if="controller.simulationResults.value.length" class="rounded-full bg-[var(--primary)]/15 px-1.5 text-[9px] text-[var(--primary)]">
          {{ controller.simulationResults.value.length }}
        </span>
      </div>
    </div>

    <HttpCookieManager :open="showCookieManager" :controller="controller" @close="showCookieManager = false" />
    <HttpHeaderSetsDialog
      :open="showHeaderSets"
      :controller="controller"
      @close="showHeaderSets = false"
      @import="importFromFile"
      @export="exportHeaderDocuments"
    />

    <div v-if="showNewCollection" class="fixed inset-0 z-[7300] flex items-center justify-center bg-black/40 p-6" @click.self="showNewCollection = false">
      <div class="w-full max-w-sm overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--card)] shadow-2xl">
        <div class="flex items-center gap-2 border-b border-[var(--border)] px-3 py-2">
          <FolderPlus class="h-4 w-4 text-[var(--primary)]" />
          <span class="text-sm font-semibold text-[var(--foreground)]">New collection</span>
        </div>
        <div class="flex flex-col gap-2 p-3">
          <label class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Name</label>
          <input
            v-model="newCollectionName"
            type="text"
            placeholder="e.g. Payments API"
            class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-[12px] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            @keydown.enter="confirmNewCollection"
          >
          <label class="mt-1 text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Where should it live?</label>
          <button
            type="button"
            class="flex items-start gap-2 rounded border p-2 text-left transition-colors"
            :class="newCollectionScope === 'repo' ? 'border-[var(--primary)] bg-[var(--primary)]/10' : 'border-[var(--border)] hover:border-[var(--primary)]/40'"
            @click="newCollectionScope = 'repo'"
          >
            <Layers class="mt-0.5 h-4 w-4 text-[var(--muted-foreground)]" />
            <span>
              <span class="block text-[11px] font-medium text-[var(--foreground)]">This repository</span>
              <span class="block text-[10px] text-[var(--muted-foreground)]">Only visible while this repository is open.</span>
            </span>
          </button>
          <button
            type="button"
            class="flex items-start gap-2 rounded border p-2 text-left transition-colors"
            :class="newCollectionScope === 'global' ? 'border-[#06b6d4] bg-[#06b6d4]/10' : 'border-[var(--border)] hover:border-[#06b6d4]/40'"
            @click="newCollectionScope = 'global'"
          >
            <Globe class="mt-0.5 h-4 w-4 text-[#06b6d4]" />
            <span>
              <span class="block text-[11px] font-medium text-[var(--foreground)]">Global</span>
              <span class="block text-[10px] text-[var(--muted-foreground)]">Available in every repository and the global view.</span>
            </span>
          </button>
        </div>
        <div class="flex items-center justify-end gap-1.5 border-t border-[var(--border)] px-3 py-2">
          <button type="button" class="rounded border border-[var(--border)] px-2.5 py-1.5 text-[11px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]" @click="showNewCollection = false">
            Cancel
          </button>
          <button type="button" class="rounded bg-[var(--primary)] px-2.5 py-1.5 text-[11px] font-medium text-white transition-opacity hover:opacity-90" @click="confirmNewCollection">
            Create
          </button>
        </div>
      </div>
    </div>
    <div v-if="showDeleteCollection" class="fixed inset-0 z-[7300] flex items-center justify-center bg-black/40 p-6" @click.self="cancelDeleteCollection">
      <div class="w-full max-w-sm overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--card)] shadow-2xl">
        <div class="flex items-center gap-2 border-b border-[var(--border)] px-3 py-2">
          <Trash2 class="h-4 w-4 text-[#ef4444]" />
          <span class="text-sm font-semibold text-[var(--foreground)]">Delete collection</span>
        </div>
        <div class="p-3 text-[12px] leading-relaxed text-[var(--muted-foreground)]">
          Delete
          <span class="font-medium text-[var(--foreground)]">"{{ deleteTargetCollection?.name }}"</span>
          together with its requests and folders? This cannot be undone.
        </div>
        <div class="flex items-center justify-end gap-1.5 border-t border-[var(--border)] px-3 py-2">
          <button type="button" class="rounded border border-[var(--border)] px-2.5 py-1.5 text-[11px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]" @click="cancelDeleteCollection">
            Cancel
          </button>
          <button type="button" class="rounded bg-[#ef4444] px-2.5 py-1.5 text-[11px] font-medium text-white transition-opacity hover:opacity-90" @click="executeDeleteCollection">
            Delete
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.http-tabstrip {
  scrollbar-width: thin;
  scrollbar-color: transparent transparent;
}

.http-tabstrip:hover {
  scrollbar-color: #8a8f98 transparent;
}

.http-tabstrip::-webkit-scrollbar {
  height: 2px;
}

.http-tabstrip::-webkit-scrollbar-track {
  background: transparent;
}

.http-tabstrip::-webkit-scrollbar-thumb {
  background: transparent;
}

.http-tabstrip:hover::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.25);
}
</style>

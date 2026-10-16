<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Plus, Trash2, Link2, Globe, Layers, FileUp, FileDown, X, Check } from "lucide-vue-next";
import type { HttpClientController } from "../useHttpClient";
import type { HttpHeaderDocument, HttpScope } from "../httpClient.types";
import HttpKeyValueTable from "./HttpKeyValueTable.vue";

const props = defineProps<{
  open: boolean;
  controller: HttpClientController;
}>();

const emit = defineEmits<{
  close: [];
  import: [];
  export: [];
}>();

const selectedId = ref<string | null>(null);
const documents = computed(() => props.controller.visibleHeaderDocuments.value);
const selected = computed<HttpHeaderDocument | null>(
  () => documents.value.find((document) => document.id === selectedId.value) ?? documents.value[0] ?? null,
);

watch(
  () => props.open,
  (open) => {
    if (open) selectedId.value = documents.value[0]?.id ?? null;
  },
  { immediate: true },
);

function createSet(): void {
  const document = props.controller.createHeaderDocument();
  selectedId.value = document.id;
}

function removeSet(document: HttpHeaderDocument): void {
  if (!window.confirm(`Delete header set "${document.name}"?`)) return;
  props.controller.deleteHeaderDocument(document.id);
  if (selectedId.value === document.id) selectedId.value = documents.value[0]?.id ?? null;
}

function setScope(document: HttpHeaderDocument, scope: HttpScope): void {
  document.scope = scope;
  document.repoPath = scope === "global" ? "" : document.repoPath;
  document.updatedAt = Date.now();
}

function attached(id: string): boolean {
  return props.controller.activeRequest.value?.headerDocumentIds.includes(id) ?? false;
}

function toggleAttach(id: string): void {
  const request = props.controller.activeRequest.value;
  if (!request) return;
  request.headerDocumentIds = request.headerDocumentIds.includes(id)
    ? request.headerDocumentIds.filter((item) => item !== id)
    : [...request.headerDocumentIds, id];
  request.updatedAt = Date.now();
}
</script>

<template>
  <div v-if="open" class="fixed inset-0 z-[7300] flex items-center justify-center bg-black/40 p-6" @click.self="emit('close')">
    <div class="flex h-[70vh] w-full max-w-2xl flex-col overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--card)] shadow-2xl">
      <div class="flex items-center gap-2 border-b border-[var(--border)] px-3 py-2">
        <Link2 class="h-4 w-4 text-[var(--primary)]" />
        <span class="text-sm font-semibold text-[var(--foreground)]">Shared header sets</span>
        <span class="text-[10px] text-[var(--muted-foreground)]">Define once, attach to many requests</span>
        <div class="ml-auto flex items-center gap-1">
          <button type="button" class="inline-flex items-center gap-1 rounded border border-[var(--border)] px-1.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]" title="Import header sets" @click="emit('import')">
            <FileUp class="h-3 w-3" /> Import
          </button>
          <button type="button" class="inline-flex items-center gap-1 rounded border border-[var(--border)] px-1.5 py-1 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]" title="Export header sets" @click="emit('export')">
            <FileDown class="h-3 w-3" /> Export
          </button>
          <button type="button" class="rounded p-1 text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]" @click="emit('close')">
            <X class="h-4 w-4" />
          </button>
        </div>
      </div>

      <div class="flex min-h-0 flex-1 overflow-hidden">
        <div class="flex w-56 shrink-0 flex-col overflow-hidden border-r border-[var(--border)]">
          <button
            type="button"
            class="m-1.5 inline-flex items-center justify-center gap-1 rounded border border-dashed border-[var(--border)] py-1.5 text-[10px] text-[var(--muted-foreground)] transition-colors hover:border-[var(--primary)]/40 hover:text-[var(--foreground)]"
            @click="createSet"
          >
            <Plus class="h-3 w-3" /> New header set
          </button>
          <div class="min-h-0 flex-1 overflow-y-auto">
            <button
              v-for="document in documents"
              :key="document.id"
              type="button"
              class="group flex w-full items-center gap-1.5 px-2 py-1.5 text-left text-[11px] transition-colors"
              :class="selected?.id === document.id ? 'bg-[var(--primary)]/10 text-[var(--foreground)]' : 'text-[var(--muted-foreground)] hover:bg-[var(--primary)]/8'"
              @click="selectedId = document.id"
            >
              <component :is="document.scope === 'global' ? Globe : Layers" class="h-3 w-3 shrink-0" :class="document.scope === 'global' ? 'text-[#06b6d4]' : ''" />
              <span class="truncate">{{ document.name }}</span>
              <span
                v-if="attached(document.id)"
                class="ml-auto inline-flex items-center gap-0.5 rounded-full bg-[#10b981]/15 px-1 text-[8px] uppercase tracking-wider text-[#10b981]"
              >
                <Check class="h-2.5 w-2.5" /> on
              </span>
            </button>
            <div v-if="documents.length === 0" class="px-2 py-6 text-center text-[10px] text-[var(--muted-foreground)]">
              No header sets yet.
            </div>
          </div>
        </div>

        <div v-if="selected" class="flex min-w-0 flex-1 flex-col overflow-hidden">
          <div class="flex items-center gap-1.5 border-b border-[var(--border)] px-3 py-2">
            <input
              :value="selected.name"
              class="min-w-0 flex-1 rounded border border-transparent bg-transparent px-1 text-[12px] font-medium text-[var(--foreground)] focus:border-[var(--border)] focus:bg-[var(--input-background)] focus:outline-none"
              @input="controller.renameHeaderDocument(selected.id, ($event.target as HTMLInputElement).value)"
            >
            <select
              :value="selected.scope"
              class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1 py-0.5 text-[9px] text-[var(--muted-foreground)] focus:outline-none"
              @change="setScope(selected, ($event.target as HTMLSelectElement).value as HttpScope)"
            >
              <option value="repo">Repo</option>
              <option value="global">Global</option>
            </select>
            <button
              type="button"
              class="inline-flex items-center gap-1 rounded border px-1.5 py-1 text-[10px] transition-colors"
              :class="attached(selected.id) ? 'border-[#10b981]/40 text-[#10b981]' : 'border-[var(--primary)]/40 text-[var(--primary)]'"
              :disabled="!controller.activeRequest.value"
              @click="toggleAttach(selected.id)"
            >
              <Check class="h-3 w-3" /> {{ attached(selected.id) ? "Attached" : "Attach to request" }}
            </button>
            <button type="button" class="rounded p-1 text-[var(--muted-foreground)] transition-colors hover:text-[#ef4444]" title="Delete set" @click="removeSet(selected)">
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </div>

          <div class="min-h-0 flex-1 overflow-y-auto p-3">
            <input
              :value="selected.description"
              placeholder="Description (optional)"
              class="mb-2 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[11px] text-[var(--muted-foreground)] focus:outline-none"
              @input="selected.description = ($event.target as HTMLInputElement).value; selected.updatedAt = Date.now()"
            >
            <HttpKeyValueTable
              :model-value="selected.headers"
              key-placeholder="Header"
              value-placeholder="Value"
              empty-message="No headers in this set. Add the first one."
              @update:model-value="(value) => { selected!.headers = value; selected!.updatedAt = Date.now(); }"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

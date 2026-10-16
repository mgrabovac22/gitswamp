<script setup lang="ts">
import { computed, ref } from "vue";
import { Trash2, AlertTriangle, Rows3, ListTree } from "lucide-vue-next";
import type { HttpKeyValue } from "../httpClient.types";
import { emptyKeyValue } from "../httpClient.utils";
import HttpCheckbox from "./HttpCheckbox.vue";

const props = withDefaults(
  defineProps<{
    modelValue: HttpKeyValue[];
    keyPlaceholder?: string;
    valuePlaceholder?: string;
    descriptionPlaceholder?: string;
    showDescription?: boolean;
    keySuggestions?: string[];
    valueSuggestions?: string[];
    emptyMessage?: string;
    allowBulk?: boolean;
  }>(),
  {
    keyPlaceholder: "Key",
    valuePlaceholder: "Value",
    descriptionPlaceholder: "Description",
    showDescription: false,
    keySuggestions: () => [],
    valueSuggestions: () => [],
    emptyMessage: "",
    allowBulk: false,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: HttpKeyValue[]];
}>();

const draftKey = ref("");
const draftValue = ref("");
const draftDescription = ref("");
const bulkMode = ref(false);
const bulkText = ref("");
const draftRowEl = ref<HTMLElement | null>(null);

const keyListId = `http-kv-keys-${Math.random().toString(36).slice(2, 8)}`;
const valueListId = `http-kv-values-${Math.random().toString(36).slice(2, 8)}`;

const duplicateKeys = computed(() => {
  const counts = new Map<string, number>();
  for (const row of props.modelValue) {
    const key = row.key.trim().toLowerCase();
    if (!key) continue;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return new Set(Array.from(counts.entries()).filter(([, count]) => count > 1).map(([key]) => key));
});

function commitDraft(): void {
  const key = draftKey.value;
  const value = draftValue.value;
  const description = draftDescription.value;
  if (!key.trim() && !value.trim()) return;

  const next = [...props.modelValue, { ...emptyKeyValue(), key, value, description }];
  emit("update:modelValue", next);
  draftKey.value = "";
  draftValue.value = "";
  draftDescription.value = "";
}

function updateRow(index: number, patch: Partial<HttpKeyValue>): void {
  const next = props.modelValue.map((row, i) => (i === index ? { ...row, ...patch } : row));
  emit("update:modelValue", next);
}

function removeRow(index: number): void {
  emit("update:modelValue", props.modelValue.filter((_, i) => i !== index));
}

function onDraftKeydown(event: KeyboardEvent): void {
  if (event.key === "Enter") {
    event.preventDefault();
    commitDraft();
  }
}

function onDraftBlur(): void {
  window.setTimeout(() => {
    if (draftRowEl.value && draftRowEl.value.contains(document.activeElement)) return;
    commitDraft();
  }, 0);
}

function enterBulkMode(): void {
  bulkText.value = props.modelValue
    .map((row) => `${row.key}: ${row.value}`)
    .join("\n");
  bulkMode.value = true;
}

function applyBulk(): void {
  const rows: HttpKeyValue[] = [];
  for (const line of bulkText.value.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const separator = trimmed.indexOf(":");
    if (separator < 0) {
      rows.push({ ...emptyKeyValue(), key: trimmed, value: "" });
    } else {
      rows.push({
        ...emptyKeyValue(),
        key: trimmed.slice(0, separator).trim(),
        value: trimmed.slice(separator + 1).trim(),
      });
    }
  }
  emit("update:modelValue", rows);
  bulkMode.value = false;
}
</script>

<template>
  <div class="flex flex-col">
    <div class="mb-1.5 flex items-center justify-between gap-2">
      <span class="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
        {{ allowBulk && bulkMode ? "Bulk edit (Key: Value per line)" : "Key / Value" }}
      </span>
      <button
        v-if="allowBulk"
        type="button"
        class="inline-flex items-center gap-1 rounded border border-[var(--border)] px-2 py-0.5 text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
        @click="bulkMode ? applyBulk() : enterBulkMode()"
      >
        <ListTree v-if="!bulkMode" class="h-3 w-3" />
        <Rows3 v-else class="h-3 w-3" />
        {{ bulkMode ? "Apply table" : "Bulk edit" }}
      </button>
    </div>

    <textarea
      v-if="allowBulk && bulkMode"
      v-model="bulkText"
      rows="7"
      spellcheck="false"
      class="w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] p-2 font-mono text-[11px] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
      placeholder="Content-Type: application/json&#10;Authorization: Bearer {{token}}"
    ></textarea>

    <div v-else class="overflow-hidden rounded border border-[var(--border)]">
      <div class="grid items-center gap-1 border-b border-[var(--border)] bg-[var(--secondary)]/50 px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]"
        :style="{ gridTemplateColumns: showDescription ? '24px 1fr 1fr 1fr 28px' : '24px 1fr 1fr 28px' }">
        <span></span>
        <span>Key</span>
        <span>Value</span>
        <span v-if="showDescription">Description</span>
        <span></span>
      </div>

      <div
        v-for="(row, index) in modelValue"
        :key="row.id"
        class="grid items-center gap-1 border-b border-[var(--border)]/60 px-2 py-1"
        :class="!row.enabled ? 'opacity-45' : ''"
        :style="{ gridTemplateColumns: showDescription ? '24px 1fr 1fr 1fr 28px' : '24px 1fr 1fr 28px' }"
      >
        <span class="flex items-center justify-center" title="Include this row">
          <HttpCheckbox
            :model-value="row.enabled"
            @update:model-value="updateRow(index, { enabled: $event })"
          />
        </span>
        <input
          :value="row.key"
          :list="keyListId"
          type="text"
          :placeholder="keyPlaceholder"
          class="rounded bg-transparent px-1.5 py-1 text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:bg-[var(--input-background)] focus:outline-none"
          @input="updateRow(index, { key: ($event.target as HTMLInputElement).value })"
        >
        <input
          :value="row.value"
          :list="valueListId"
          type="text"
          :placeholder="valuePlaceholder"
          class="rounded bg-transparent px-1.5 py-1 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:bg-[var(--input-background)] focus:outline-none"
          @input="updateRow(index, { value: ($event.target as HTMLInputElement).value })"
        >
        <input
          v-if="showDescription"
          :value="row.description || ''"
          type="text"
          :placeholder="descriptionPlaceholder"
          class="rounded bg-transparent px-1.5 py-1 text-[11px] text-[var(--muted-foreground)] focus:bg-[var(--input-background)] focus:outline-none"
          @input="updateRow(index, { description: ($event.target as HTMLInputElement).value })"
        >
        <button
          type="button"
          class="flex h-6 w-6 items-center justify-center rounded text-[var(--muted-foreground)] transition-colors hover:bg-[#ef4444]/15 hover:text-[#ef4444]"
          title="Delete row"
          @click="removeRow(index)"
        >
          <Trash2 class="h-3 w-3" />
        </button>
      </div>

      <div
        ref="draftRowEl"
        class="grid items-center gap-1 px-2 py-1"
        :style="{ gridTemplateColumns: showDescription ? '24px 1fr 1fr 1fr 28px' : '24px 1fr 1fr 28px' }"
      >
        <span class="flex h-6 w-6 items-center justify-center text-[var(--primary)]">
          <span class="text-xs font-semibold leading-none">+</span>
        </span>
        <input
          v-model="draftKey"
          :list="keyListId"
          type="text"
          :placeholder="keyPlaceholder"
          class="rounded bg-transparent px-1.5 py-1 text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:bg-[var(--input-background)] focus:outline-none"
          @keydown="onDraftKeydown"
          @blur="onDraftBlur"
        >
        <input
          v-model="draftValue"
          :list="valueListId"
          type="text"
          :placeholder="valuePlaceholder"
          class="rounded bg-transparent px-1.5 py-1 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:bg-[var(--input-background)] focus:outline-none"
          @keydown="onDraftKeydown"
          @blur="onDraftBlur"
        >
        <input
          v-if="showDescription"
          v-model="draftDescription"
          type="text"
          :placeholder="descriptionPlaceholder"
          class="rounded bg-transparent px-1.5 py-1 text-[11px] text-[var(--muted-foreground)] focus:bg-[var(--input-background)] focus:outline-none"
          @keydown="onDraftKeydown"
          @blur="onDraftBlur"
        >
        <span></span>
      </div>
    </div>

    <datalist :id="keyListId">
      <option v-for="suggestion in keySuggestions" :key="suggestion" :value="suggestion" />
    </datalist>
    <datalist :id="valueListId">
      <option v-for="suggestion in valueSuggestions" :key="suggestion" :value="suggestion" />
    </datalist>

    <div v-if="duplicateKeys.size > 0" class="mt-1 flex items-center gap-1.5 text-[10px] text-[#f59e0b]">
      <AlertTriangle class="h-3 w-3" />
      Duplicate header names detected. The last value wins for most servers.
    </div>

    <div v-if="modelValue.length === 0 && !draftKey && emptyMessage" class="mt-1 text-[10px] text-[var(--muted-foreground)]">
      {{ emptyMessage }}
    </div>
  </div>
</template>

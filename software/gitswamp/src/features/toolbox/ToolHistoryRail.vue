<script setup lang="ts">
import { History, Trash2 } from "lucide-vue-next";

withDefaults(
  defineProps<{
    title?: string;
    items: { id: string; primary: string; secondary?: string; tertiary?: string; mono?: boolean }[];
    emptyText?: string;
    clearable?: boolean;
  }>(),
  {
    title: "History",
    emptyText: "Nothing saved yet.",
    clearable: true,
  },
);

const emit = defineEmits<{
  select: [id: string];
  remove: [id: string];
  clear: [];
}>();
</script>

<template>
  <div class="w-[220px] flex-shrink-0 flex flex-col rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25">
    <div class="flex items-center justify-between border-b border-[var(--border)] px-2.5 py-2">
      <div class="flex items-center gap-1.5">
        <History class="h-3.5 w-3.5 text-[var(--primary)]" />
        <span class="text-[11px] font-semibold text-[var(--foreground)]">{{ title }}</span>
      </div>
      <button v-if="clearable && items.length" class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--destructive)]" @click="emit('clear')">Clear</button>
    </div>
    <div class="flex-1 overflow-y-auto p-1.5">
      <p v-if="!items.length" class="px-1.5 py-2 text-[10px] text-[var(--muted-foreground)]">{{ emptyText }}</p>
      <button
        v-for="item in items"
        :key="item.id"
        class="group mb-1 flex w-full items-start justify-between gap-1.5 rounded border border-transparent px-1.5 py-1.5 text-left hover:border-[var(--border)] hover:bg-[var(--card)]"
        @click="emit('select', item.id)"
      >
        <span class="min-w-0 flex-1">
          <span class="block truncate text-[10px] text-[var(--foreground)]" :class="item.mono ? 'font-mono' : 'font-medium'">{{ item.primary }}</span>
          <span v-if="item.secondary" class="block truncate font-mono text-[9px] text-[var(--muted-foreground)]">{{ item.secondary }}</span>
          <span v-if="item.tertiary" class="block text-[9px] text-[var(--muted-foreground)]/70">{{ item.tertiary }}</span>
        </span>
        <span class="rounded p-0.5 text-[var(--muted-foreground)] opacity-0 group-hover:opacity-100 hover:text-[var(--destructive)]" @click.stop="emit('remove', item.id)">
          <Trash2 class="h-3 w-3" />
        </span>
      </button>
    </div>
    <slot name="footer" />
  </div>
</template>

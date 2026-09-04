<script setup lang="ts">
import { computed, ref } from "vue";
import { ListTree, User, AlertTriangle } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";

type LogTab = "app" | "user" | "error";

const props = defineProps<{
  appLogs: string[];
  userLogs: string[];
  errorLogs: string[];
}>();

const emit = defineEmits<{
  close: [];
}>();

const activeTab = ref<LogTab>("app");

const activeRows = computed(() => {
  if (activeTab.value === "user") {
    return props.userLogs;
  }
  if (activeTab.value === "error") {
    return props.errorLogs;
  }
  return props.appLogs;
});

function tabClass(tab: LogTab): string {
  if (activeTab.value === tab) {
    return "bg-[var(--primary)]/15 text-[var(--foreground)] border-[var(--primary)]/40";
  }
  return "bg-[var(--secondary)] text-[var(--muted-foreground)] border-[var(--border)] hover:text-[var(--foreground)]";
}
</script>

<template>
  <div class="h-full flex flex-col bg-[var(--card)] border-l border-[var(--border)]">
    <div class="flex items-center justify-between px-3 py-2 border-b border-[var(--border)]">
      <div class="text-xs font-semibold text-[var(--foreground)] uppercase tracking-[0.08em]">Logs</div>
      <CloseIconButton size="sm" title="Close logs panel" @click="emit('close')" />
    </div>

    <div class="px-2 py-2 flex items-center gap-1 border-b border-[var(--border)]">
      <button class="px-2 py-1 text-[11px] rounded border transition-colors flex items-center gap-1" :class="tabClass('app')" @click="activeTab = 'app'">
        <ListTree class="w-3.5 h-3.5" />
        App
        <span class="opacity-75">({{ appLogs.length }})</span>
      </button>
      <button class="px-2 py-1 text-[11px] rounded border transition-colors flex items-center gap-1" :class="tabClass('user')" @click="activeTab = 'user'">
        <User class="w-3.5 h-3.5" />
        User
        <span class="opacity-75">({{ userLogs.length }})</span>
      </button>
      <button class="px-2 py-1 text-[11px] rounded border transition-colors flex items-center gap-1" :class="tabClass('error')" @click="activeTab = 'error'">
        <AlertTriangle class="w-3.5 h-3.5" />
        Error
        <span class="opacity-75">({{ errorLogs.length }})</span>
      </button>
    </div>

    <div class="flex-1 overflow-auto px-2 py-2">
      <div v-if="activeRows.length === 0" class="h-full flex items-center justify-center text-[11px] text-[var(--muted-foreground)]">
        No logs in this category.
      </div>

      <div v-else class="space-y-1.5">
        <div
          v-for="(row, idx) in activeRows"
          :key="`${activeTab}:${idx}:${row}`"
          class="px-2 py-1.5 rounded text-[11px] font-mono whitespace-pre-wrap break-all border"
          :class="activeTab === 'error'
            ? 'bg-[var(--destructive)]/10 text-[var(--destructive)] border-[var(--destructive)]/30'
            : 'bg-[var(--secondary)]/55 text-[var(--foreground)] border-[var(--border)]'"
        >
          {{ row }}
        </div>
      </div>
    </div>
  </div>
</template>

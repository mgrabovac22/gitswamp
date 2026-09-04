<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { Command, Search } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";

interface CommandPaletteAction {
  id: string;
  label: string;
  description?: string;
  shortcut?: string;
  keywords?: string[];
  disabled?: boolean;
  tone?: "default" | "success" | "warning" | "danger";
  run: () => void | Promise<void>;
}

const props = defineProps<{
  visible: boolean;
  actions: CommandPaletteAction[];
}>();

const emit = defineEmits<{
  close: [];
}>();

const query = ref("");
const selectedIndex = ref(0);
const inputRef = ref<HTMLInputElement | null>(null);

const filteredActions = computed(() => {
  const needle = query.value.trim().toLowerCase();
  const actions = props.actions.filter((action) => !action.disabled);
  if (!needle) return actions;

  return actions.filter((action) => {
    const haystack = [
      action.label,
      action.description || "",
      action.shortcut || "",
      ...(action.keywords || []),
    ].join(" ").toLowerCase();
    return needle.split(/\s+/).every((part) => haystack.includes(part));
  });
});

watch(
  () => props.visible,
  async (visible) => {
    if (!visible) return;
    query.value = "";
    selectedIndex.value = 0;
    await nextTick();
    inputRef.value?.focus();
  },
);

watch(filteredActions, () => {
  selectedIndex.value = Math.min(selectedIndex.value, Math.max(0, filteredActions.value.length - 1));
});

function close() {
  emit("close");
}

async function runAction(action: CommandPaletteAction) {
  if (action.disabled) return;
  await action.run();
  close();
}

function onKeyDown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
    return;
  }

  if (event.key === "ArrowDown") {
    event.preventDefault();
    selectedIndex.value = Math.min(selectedIndex.value + 1, Math.max(0, filteredActions.value.length - 1));
    return;
  }

  if (event.key === "ArrowUp") {
    event.preventDefault();
    selectedIndex.value = Math.max(0, selectedIndex.value - 1);
    return;
  }

  if (event.key === "Enter") {
    event.preventDefault();
    const action = filteredActions.value[selectedIndex.value];
    if (action) {
      void runAction(action);
    }
  }
}

function toneClass(action: CommandPaletteAction): string {
  if (action.tone === "success") return "text-[#10b981]";
  if (action.tone === "warning") return "text-[#f59e0b]";
  if (action.tone === "danger") return "text-[#ef4444]";
  return "text-[var(--primary)]";
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 z-[7300] flex items-start justify-center bg-black/40 pt-[12vh] backdrop-blur-sm"
      @click.self="close"
    >
      <div class="w-[620px] max-w-[92vw] overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--popover)] shadow-2xl">
        <div class="flex items-center justify-between border-b border-[var(--border)] px-3 py-2">
          <div class="flex items-center gap-2">
            <Command class="h-4 w-4 text-[var(--primary)]" />
            <div>
              <div class="text-sm font-semibold text-[var(--foreground)]">Command Palette</div>
              <div class="text-[10px] text-[var(--muted-foreground)]">Run common GitSwamp actions</div>
            </div>
          </div>
          <CloseIconButton size="sm" title="Close command palette" @click="close" />
        </div>

        <div class="flex items-center gap-2 border-b border-[var(--border)] px-3 py-2">
          <Search class="h-4 w-4 text-[var(--muted-foreground)]" />
          <input
            ref="inputRef"
            v-model="query"
            class="h-8 flex-1 bg-transparent text-sm text-[var(--foreground)] outline-none placeholder:text-[var(--muted-foreground)]"
            placeholder="stage all, checkout main, open settings..."
            @keydown="onKeyDown"
          >
          <span class="rounded border border-[var(--border)] bg-[var(--secondary)] px-1.5 py-0.5 text-[10px] text-[var(--muted-foreground)]">Ctrl K</span>
        </div>

        <div class="max-h-[390px] overflow-y-auto p-2">
          <button
            v-for="(action, index) in filteredActions"
            :key="action.id"
            type="button"
            class="w-full rounded-lg px-2.5 py-2 text-left transition-colors"
            :class="index === selectedIndex ? 'bg-[var(--primary)]/12' : 'hover:bg-[var(--secondary)]/70'"
            @mouseenter="selectedIndex = index"
            @click="runAction(action)"
          >
            <div class="flex items-center justify-between gap-3">
              <span class="text-xs font-semibold" :class="toneClass(action)">{{ action.label }}</span>
              <span v-if="action.shortcut" class="text-[10px] text-[var(--muted-foreground)]">{{ action.shortcut }}</span>
            </div>
            <div v-if="action.description" class="mt-0.5 text-[11px] text-[var(--muted-foreground)]">
              {{ action.description }}
            </div>
          </button>

          <div v-if="filteredActions.length === 0" class="px-4 py-10 text-center text-xs text-[var(--muted-foreground)]">
            No matching actions.
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { GitBranch, ChevronDown, Ghost, WandSparkles, Trash2 } from "lucide-vue-next";
import AppButton from "@/shared/ui/AppButton.vue";

const props = defineProps<{
  loading: boolean;
  ghostActive: boolean;
}>();

const emit = defineEmits<{
  branch: [];
  ghost: [];
  materialize: [];
  discard: [];
}>();

const root = ref<HTMLElement | null>(null);
const menuOpen = ref(false);

function closeMenu() {
  menuOpen.value = false;
}

function toggleMenu() {
  menuOpen.value = !menuOpen.value;
}

function runAndClose(action: "branch" | "ghost" | "materialize" | "discard") {
  if (action === "branch") emit("branch");
  if (action === "ghost") emit("ghost");
  if (action === "materialize") emit("materialize");
  if (action === "discard") emit("discard");
  closeMenu();
}

function onDocumentPointerDown(event: MouseEvent) {
  const target = event.target as Node | null;
  if (!target || !root.value) return;
  if (!root.value.contains(target)) {
    closeMenu();
  }
}

function onDocumentKeyDown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    closeMenu();
  }
}

onMounted(() => {
  document.addEventListener("mousedown", onDocumentPointerDown);
  document.addEventListener("keydown", onDocumentKeyDown);
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocumentPointerDown);
  document.removeEventListener("keydown", onDocumentKeyDown);
});
</script>

<template>
  <div ref="root" class="relative inline-flex items-center">
    <AppButton
      variant="ghost"
      size="sm"
      class="h-8 rounded-r-none border border-r-0 border-transparent text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
      :disabled="props.loading"
      @click="emit('branch')"
    >
      <GitBranch class="w-3.5 h-3.5" />
      Branch
      <span
        v-if="props.ghostActive"
        class="ml-1 rounded-full border border-[var(--destructive)]/55 bg-[var(--destructive)]/15 px-1.5 py-0.5 text-[9px] font-semibold text-[var(--destructive)]"
      >
        Ghost
      </span>
    </AppButton>

    <button
      class="h-8 w-7 rounded-r-md border border-transparent text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] transition-colors"
      :disabled="props.loading"
      title="Branch actions"
      @click="toggleMenu"
    >
      <ChevronDown class="w-3.5 h-3.5 mx-auto" />
    </button>

    <div
      v-if="menuOpen"
      class="absolute top-full right-0 mt-1 z-[2300] min-w-[220px] rounded-lg border border-[var(--border)] bg-[var(--popover)] shadow-2xl p-1"
    >
      <button
        class="w-full text-left px-2.5 py-2 rounded text-[11px] text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors inline-flex items-center gap-2"
        @click="runAndClose('branch')"
      >
        <GitBranch class="w-3.5 h-3.5 text-[var(--primary)]" />
        Create a branch
      </button>

      <button
        v-if="!props.ghostActive"
        class="w-full text-left px-2.5 py-2 rounded text-[11px] text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors inline-flex items-center gap-2"
        @click="runAndClose('ghost')"
      >
        <Ghost class="w-3.5 h-3.5 text-[var(--destructive)]" />
        Create a ghost branch
      </button>

      <template v-else>
        <button
          class="w-full text-left px-2.5 py-2 rounded text-[11px] text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors inline-flex items-center gap-2"
          @click="runAndClose('materialize')"
        >
          <WandSparkles class="w-3.5 h-3.5 text-[var(--primary)]" />
          Materialize ghost branch
        </button>
        <button
          class="w-full text-left px-2.5 py-2 rounded text-[11px] text-[var(--destructive)] hover:bg-[var(--destructive)]/10 transition-colors inline-flex items-center gap-2"
          @click="runAndClose('discard')"
        >
          <Trash2 class="w-3.5 h-3.5" />
          Discard ghost branch
        </button>
      </template>
    </div>
  </div>
</template>

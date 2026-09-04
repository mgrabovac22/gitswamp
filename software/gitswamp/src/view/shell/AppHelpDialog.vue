<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { BookOpen, HelpCircle, Keyboard, Search, Shield, X } from "lucide-vue-next";
import GitRpgShield from "@/features/repository/rpg/GitRpgShield.vue";
import { GIT_RPG_ROLES } from "@/features/repository/rpg/gitRpgProfiler";
import {
  KEYBOARD_SHORTCUT_GROUPS,
  type AppHelpSection,
} from "@/features/shell/keyboardShortcuts";

const props = defineProps<{
  open: boolean;
  initialSection: AppHelpSection;
}>();

const emit = defineEmits<{
  close: [];
}>();

const activeSection = ref<AppHelpSection>(props.initialSection);
const shortcutQuery = ref("");

const sections: readonly { id: AppHelpSection; label: string }[] = [
  { id: "overview", label: "Overview" },
  { id: "shortcuts", label: "Keyboard Shortcuts" },
  { id: "roles", label: "Git RPG Roles" },
];

const dialogTitle = computed(() => {
  if (activeSection.value === "shortcuts") return "Keyboard Shortcuts";
  if (activeSection.value === "roles") return "Git RPG Roles";
  return "GitSwamp Help";
});

const visibleShortcutGroups = computed(() => {
  const query = shortcutQuery.value.trim().toLowerCase();
  if (!query) return KEYBOARD_SHORTCUT_GROUPS;

  return KEYBOARD_SHORTCUT_GROUPS
    .map((group) => ({
      ...group,
      shortcuts: group.shortcuts.filter((shortcut) =>
        `${shortcut.label} ${shortcut.keys}`.toLowerCase().includes(query),
      ),
    }))
    .filter((group) => group.shortcuts.length > 0);
});

watch(
  () => [props.open, props.initialSection] as const,
  ([open, initialSection]) => {
    if (!open) return;
    activeSection.value = initialSection;
    shortcutQuery.value = "";
  },
);
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-[7100] flex items-center justify-center bg-black/55 backdrop-blur-sm"
      @click.self="emit('close')"
    >
      <div class="flex h-[min(680px,88vh)] w-[780px] max-w-[95vw] overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--card)] shadow-2xl">
        <nav class="w-[164px] flex-shrink-0 border-r border-[var(--border)] bg-[var(--secondary)]/35 p-2">
          <div class="mb-2 flex h-9 items-center gap-2 px-2 text-[var(--foreground)]">
            <HelpCircle class="h-4 w-4 text-[var(--primary)]" />
            <span class="text-xs font-semibold">Help</span>
          </div>

          <button
            v-for="section in sections"
            :key="section.id"
            type="button"
            class="mb-1 flex w-full items-center gap-2 rounded px-2 py-2 text-left text-[11px] transition-colors"
            :class="activeSection === section.id
              ? 'bg-[var(--primary)]/15 text-[var(--foreground)]'
              : 'text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]'"
            @click="activeSection = section.id"
          >
            <BookOpen v-if="section.id === 'overview'" class="h-3.5 w-3.5" />
            <Keyboard v-else-if="section.id === 'shortcuts'" class="h-3.5 w-3.5" />
            <Shield v-else class="h-3.5 w-3.5" />
            <span>{{ section.label }}</span>
          </button>
        </nav>

        <div class="flex min-w-0 flex-1 flex-col">
          <header class="flex h-12 flex-shrink-0 items-center justify-between border-b border-[var(--border)] px-4">
            <h3 class="text-sm font-semibold text-[var(--foreground)]">{{ dialogTitle }}</h3>
            <button
              type="button"
              class="flex h-7 w-7 items-center justify-center rounded text-[var(--muted-foreground)] transition-colors hover:bg-[var(--secondary)] hover:text-[var(--foreground)]"
              title="Close help"
              @click="emit('close')"
            >
              <X class="h-3.5 w-3.5" />
            </button>
          </header>

          <div class="help-content flex-1 overflow-y-auto px-5 py-4">
            <div v-if="activeSection === 'overview'" class="space-y-5">
              <section>
                <h4 class="mb-2 text-xs font-semibold uppercase text-[var(--foreground)]">Core Workflow</h4>
                <ul class="space-y-1.5 text-[11px] leading-5 text-[var(--muted-foreground)]">
                  <li>Use the graph to search commits, follow branch history and open commit actions.</li>
                  <li>Use the sidebar for local and remote branches, stashes, tags, issues and pull requests.</li>
                  <li>Use Working Changes to stage, unstage, inspect and commit repository changes.</li>
                  <li>Use the command palette for quick access to repository and application actions.</li>
                </ul>
              </section>

              <section>
                <h4 class="mb-2 text-xs font-semibold uppercase text-[var(--foreground)]">Visual History</h4>
                <ul class="space-y-1.5 text-[11px] leading-5 text-[var(--muted-foreground)]">
                  <li>Galaxy View maps loaded branches, commits and ancestry links on an interactive canvas.</li>
                  <li>Repository City maps folders and files to districts, buildings and activity hotspots.</li>
                  <li>Time Machine, conflict analytics and Burnout Analytics provide focused history views.</li>
                </ul>
              </section>

              <section>
                <h4 class="mb-2 text-xs font-semibold uppercase text-[var(--foreground)]">Fast Access</h4>
                <p class="text-[11px] leading-5 text-[var(--muted-foreground)]">
                  Press <span class="font-mono text-[var(--foreground)]">Ctrl+K</span> for commands,
                  <span class="font-mono text-[var(--foreground)]">F1</span> for shortcuts, or use the dedicated
                  Keyboard Shortcuts section in this Help menu.
                </p>
              </section>
            </div>

            <div v-else-if="activeSection === 'shortcuts'" class="space-y-4">
              <label class="flex h-8 items-center gap-2 border-b border-[var(--border)] px-1 text-[var(--muted-foreground)] focus-within:border-[var(--primary)]">
                <Search class="h-3.5 w-3.5 flex-shrink-0" />
                <input
                  v-model="shortcutQuery"
                  type="search"
                  class="min-w-0 flex-1 bg-transparent text-[11px] text-[var(--foreground)] outline-none placeholder:text-[var(--muted-foreground)]"
                  placeholder="Filter actions or keys"
                />
              </label>

              <section v-for="group in visibleShortcutGroups" :key="group.id">
                <h4 class="mb-1.5 text-[10px] font-semibold uppercase text-[var(--muted-foreground)]">{{ group.label }}</h4>
                <div class="divide-y divide-[var(--border)]/70 border-y border-[var(--border)]/70">
                  <div
                    v-for="shortcut in group.shortcuts"
                    :key="shortcut.id"
                    class="flex min-h-8 items-center justify-between gap-4 px-1 text-[11px]"
                  >
                    <span class="text-[var(--foreground)]">{{ shortcut.label }}</span>
                    <kbd class="flex-shrink-0 font-mono text-[10px] text-[var(--muted-foreground)]">{{ shortcut.keys }}</kbd>
                  </div>
                </div>
              </section>

              <p v-if="visibleShortcutGroups.length === 0" class="py-12 text-center text-[11px] text-[var(--muted-foreground)]">
                No shortcuts match this filter.
              </p>
            </div>

            <div v-else class="space-y-3">
              <p class="text-[11px] leading-5 text-[var(--muted-foreground)]">
                The shield near Branch summarizes the current repository's commit style. Hover it for the role name or click it for the current profile explanation.
              </p>
              <div class="divide-y divide-[var(--border)]/70 border-y border-[var(--border)]/70">
                <div v-for="role in GIT_RPG_ROLES" :key="role.id" class="flex items-center gap-3 px-1 py-2.5">
                  <GitRpgShield :role="role" size="help" class="flex-shrink-0" />
                  <div class="min-w-0">
                    <div class="text-[11px] font-semibold text-[var(--foreground)]">{{ role.title }}</div>
                    <div class="text-[10px] leading-4 text-[var(--muted-foreground)]">{{ role.signal }}</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.help-content {
  scrollbar-width: none;
}

.help-content::-webkit-scrollbar {
  width: 0;
  height: 0;
}
</style>

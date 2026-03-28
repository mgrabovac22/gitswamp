<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Folder, Plus, X, Home, Menu, HelpCircle } from "lucide-vue-next";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { RepoInfo } from "@/types";

type MenuSection = "file" | "edit" | "view" | "help";

interface MenuAction {
  id: string;
  label: string;
  shortcut?: string;
  disabled?: boolean;
  run: () => void;
}

const props = defineProps<{
  tabs: { id: string; repo: RepoInfo | null; label: string }[];
  activeTabId: string;
}>();

const emit = defineEmits<{
  selectTab: [id: string];
  closeTab: [id: string];
  newTab: [];
  openRepository: [];
  toggleTerminal: [];
  openSettings: [];
  refreshRepository: [];
  openInVsCode: [];
  openInExplorer: [];
  createGist: [];
}>();

const menuRoot = ref<HTMLElement | null>(null);
const menuOpen = ref(false);
const showHelpPanel = ref(false);
const activeSection = ref<MenuSection>("file");

const sectionLabels: { id: MenuSection; label: string }[] = [
  { id: "file", label: "File" },
  { id: "edit", label: "Edit" },
  { id: "view", label: "View" },
  { id: "help", label: "Help" },
];

const activeTab = computed(() => props.tabs.find((tab) => tab.id === props.activeTabId) ?? null);
const hasActiveRepo = computed(() => !!activeTab.value?.repo);
const activeRepoPath = computed(() => activeTab.value?.repo?.path ?? "");
const canCloseActiveTab = computed(() => props.tabs.length > 1);

function toggleMenu() {
  menuOpen.value = !menuOpen.value;
  if (menuOpen.value) {
    activeSection.value = "file";
  }
}

function closeMenu() {
  menuOpen.value = false;
}

function openHelpPanel() {
  showHelpPanel.value = true;
  closeMenu();
}

function openProjectGuide() {
  openUrl("https://github.com/mgrabovac22/gitswamp/wiki").catch(() => {});
}

function openIssueTracker() {
  openUrl("https://github.com/mgrabovac22/gitswamp/issues").catch(() => {});
}

function copyActiveRepoPath() {
  if (!activeRepoPath.value) return;
  navigator.clipboard.writeText(activeRepoPath.value).catch(() => {});
}

const menuActions = computed<Record<MenuSection, MenuAction[]>>(() => ({
  file: [
    {
      id: "new-tab",
      label: "New Tab",
      shortcut: "Ctrl+T",
      run: () => emit("newTab"),
    },
    {
      id: "open-repo",
      label: "Open Repository",
      shortcut: "Ctrl+O",
      run: () => emit("openRepository"),
    },
    {
      id: "close-tab",
      label: "Close Current Tab",
      shortcut: "Ctrl+W",
      disabled: !canCloseActiveTab.value,
      run: () => emit("closeTab", props.activeTabId),
    },
    {
      id: "create-gist",
      label: "Create a Gist",
      shortcut: "Ctrl+Shift+G",
      disabled: !hasActiveRepo.value,
      run: () => emit("createGist"),
    },
  ],
  edit: [
    {
      id: "copy-path",
      label: "Copy Repository Path",
      shortcut: "Ctrl+Shift+C",
      disabled: !hasActiveRepo.value,
      run: copyActiveRepoPath,
    },
    {
      id: "refresh-repo",
      label: "Refresh Repository",
      shortcut: "Ctrl+Shift+R",
      disabled: !hasActiveRepo.value,
      run: () => emit("refreshRepository"),
    },
    {
      id: "open-vscode",
      label: "Open in VS Code",
      shortcut: "Ctrl+Shift+O",
      disabled: !hasActiveRepo.value,
      run: () => emit("openInVsCode"),
    },
  ],
  view: [
    {
      id: "toggle-terminal",
      label: "Toggle Terminal",
      shortcut: "Ctrl+`",
      run: () => emit("toggleTerminal"),
    },
    {
      id: "open-explorer",
      label: "Open in Folder Explorer",
      shortcut: "Alt+O",
      disabled: !hasActiveRepo.value,
      run: () => emit("openInExplorer"),
    },
    {
      id: "open-settings",
      label: "Open Settings",
      shortcut: "Ctrl+,",
      run: () => emit("openSettings"),
    },
  ],
  help: [
    {
      id: "help-overview",
      label: "Help and Shortcuts",
      shortcut: "F1",
      run: openHelpPanel,
    },
    {
      id: "help-guide",
      label: "Open Online Guide",
      run: openProjectGuide,
    },
    {
      id: "help-issues",
      label: "Report Issue",
      run: openIssueTracker,
    },
  ],
}));

function executeAction(action: MenuAction) {
  if (action.disabled) return;
  action.run();
  if (action.id !== "help-overview") {
    closeMenu();
  }
}

function onDocumentPointerDown(event: MouseEvent) {
  const target = event.target as Node | null;
  if (!menuRoot.value || !target) return;
  if (!menuRoot.value.contains(target)) {
    closeMenu();
  }
}

function isEditableTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  if (!element) return false;
  const tag = element.tagName.toLowerCase();
  return element.isContentEditable || tag === "input" || tag === "textarea" || tag === "select";
}

function onGlobalKeyDown(event: KeyboardEvent) {
  if (event.key === "F1" && !isEditableTarget(event.target)) {
    event.preventDefault();
    openHelpPanel();
    return;
  }

  if (event.key === "Escape") {
    closeMenu();
    showHelpPanel.value = false;
  }
}

onMounted(() => {
  document.addEventListener("mousedown", onDocumentPointerDown);
  document.addEventListener("keydown", onGlobalKeyDown);
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocumentPointerDown);
  document.removeEventListener("keydown", onGlobalKeyDown);
});
</script>

<template>
  <div class="tabs-strip h-10 bg-[var(--background)] flex items-end px-1 border-b border-[var(--border)] gap-px flex-shrink-0 [app-region:no-drag]">
    <div ref="menuRoot" class="relative flex-shrink-0">
      <button
        class="h-9 w-9 flex items-center justify-center rounded-t-md text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors [app-region:no-drag]"
        title="Menu"
        @click.stop="toggleMenu"
      >
        <Menu class="w-3.5 h-3.5" />
      </button>

      <div
        v-if="menuOpen"
        class="absolute left-0 top-full mt-1 z-[220] w-[450px] bg-[var(--popover)] border border-[var(--border)] rounded-xl shadow-2xl overflow-hidden [app-region:no-drag]"
        @click.stop
      >
        <div class="flex min-h-[240px]">
          <div class="w-[110px] border-r border-[var(--border)] bg-[var(--secondary)]/45 p-1.5">
            <button
              v-for="section in sectionLabels"
              :key="section.id"
              class="w-full text-left px-2 py-1.5 rounded text-[11px] transition-colors"
              :class="activeSection === section.id
                ? 'bg-[var(--primary)]/15 text-[var(--foreground)]'
                : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
              @mouseenter="activeSection = section.id"
              @focus="activeSection = section.id"
              @click="activeSection = section.id"
            >
              {{ section.label }}
            </button>
          </div>

          <div class="flex-1 p-2">
            <button
              v-for="action in menuActions[activeSection]"
              :key="action.id"
              class="w-full text-left px-2.5 py-2 rounded transition-colors mb-1 last:mb-0"
              :class="action.disabled
                ? 'opacity-45 cursor-not-allowed text-[var(--muted-foreground)]'
                : 'hover:bg-[var(--primary)]/12 text-[var(--foreground)]'"
              :disabled="action.disabled"
              @click="executeAction(action)"
            >
              <div class="flex items-center justify-between gap-2">
                <span class="text-[11px] font-medium">{{ action.label }}</span>
                <span v-if="action.shortcut" class="text-[9px] text-[var(--muted-foreground)]">{{ action.shortcut }}</span>
              </div>
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="tabs-scroll flex items-end gap-px overflow-x-auto overflow-y-hidden flex-1 min-w-0 [app-region:no-drag]">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        @click="emit('selectTab', tab.id)"
        :class="[
          'h-9 px-3 rounded-t-md flex items-center gap-2 text-xs font-medium transition-colors relative group min-w-0 max-w-48 flex-shrink-0',
          activeTabId === tab.id
            ? 'bg-[var(--card)] text-[var(--foreground)] border-t border-x border-[var(--border)]'
            : 'bg-[var(--background)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]',
        ]"
      >
        <Home v-if="!tab.repo" class="w-3 h-3 text-[var(--primary)] flex-shrink-0" />
        <Folder v-else class="w-3 h-3 text-[var(--primary)] flex-shrink-0" />
        <span class="truncate">{{ tab.label }}</span>
        <button
          v-if="tabs.length > 1"
          @click.stop="emit('closeTab', tab.id)"
          class="ml-1 p-0.5 rounded hover:bg-[#ef4444]/20 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0"
        >
          <X class="w-3 h-3" />
        </button>
      </button>

      <button
        @click="emit('newTab')"
        class="h-9 w-8 flex items-center justify-center rounded-t-md text-[var(--muted-foreground)] hover:text-[var(--primary)] hover:bg-[var(--secondary)] transition-colors flex-shrink-0"
        title="New Tab"
      >
        <Plus class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>

  <Teleport to="body">
    <div
      v-if="showHelpPanel"
      class="fixed inset-0 z-[260] flex items-center justify-center bg-black/55 backdrop-blur-sm"
      @click.self="showHelpPanel = false"
    >
      <div class="w-[700px] max-w-[95vw] max-h-[88vh] overflow-y-auto rounded-2xl border border-[var(--border)] bg-[var(--card)] shadow-2xl">
        <div class="px-4 py-3 border-b border-[var(--border)] flex items-center justify-between">
          <div class="flex items-center gap-2">
            <HelpCircle class="w-4 h-4 text-[var(--primary)]" />
            <h3 class="text-sm font-semibold text-[var(--foreground)]">Help and Keyboard Shortcuts</h3>
          </div>
          <button
            class="p-1 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
            @click="showHelpPanel = false"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>

        <div class="px-4 py-4 space-y-4">
          <section>
            <h4 class="text-xs font-semibold text-[var(--foreground)] uppercase tracking-wide mb-2">Core Features</h4>
            <ul class="space-y-1 text-[11px] text-[var(--muted-foreground)]">
              <li>Graph panel: search, navigate matches, isolate branch history, drag and drop merge requests.</li>
              <li>Repository sidebar: local and remote branches, stashes, tags, plus Create a Gist action.</li>
              <li>Right-click menus on commits and branches expose checkout, merge, reset, and branch operations.</li>
              <li>Terminal panel supports git aliases, quick actions, history, reverse search and open-tool commands.</li>
            </ul>
          </section>

          <section>
            <h4 class="text-xs font-semibold text-[var(--foreground)] uppercase tracking-wide mb-2">Shortcuts</h4>
            <div class="space-y-1.5 text-[11px]">
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open help and shortcuts</span><span class="text-[var(--muted-foreground)] font-mono">F1</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">New tab</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+T</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Close active tab</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+W</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Toggle terminal panel</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+`</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Refresh repository data</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+R</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open repository in VS Code</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+O</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open repository in folder explorer</span><span class="text-[var(--muted-foreground)] font-mono">Alt+O</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Focus commit search</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+R</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open Gist creator</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+G</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open settings</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+,</span></div>
            </div>
          </section>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.tabs-scroll {
  scrollbar-width: thin;
  scrollbar-color: rgba(139, 92, 246, 0.3) transparent;
}

.tabs-scroll::-webkit-scrollbar {
  height: 5px;
}

.tabs-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.tabs-scroll::-webkit-scrollbar-thumb {
  background: rgba(139, 92, 246, 0.28);
  border-radius: 999px;
}

.tabs-scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(139, 92, 246, 0.45);
}
</style>

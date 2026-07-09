<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Folder, Plus, X, Home, Menu, HelpCircle, Info } from "lucide-vue-next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isEditableTarget } from "@/shared/dom/keyboardTargets";
import GitRpgShield from "@/features/repository/rpg/GitRpgShield.vue";
import { GIT_RPG_ROLES } from "@/features/repository/rpg/gitRpgProfiler";
import type { RepoInfo } from "@/types";

type MenuSection = "file" | "edit" | "view" | "options" | "help";
type HistoryViewMode = "graph" | "galaxy" | "city" | "productivity" | "time-machine" | "conflict-heatmap" | "burnout";

interface MenuAction {
  id: string;
  label: string;
  description?: string;
  shortcut?: string;
  disabled?: boolean;
  run: () => void;
}

const props = defineProps<{
  tabs: { id: string; repo: RepoInfo | null; label: string }[];
  activeTabId: string;
  canReopenClosedTab?: boolean;
}>();

const emit = defineEmits<{
  selectTab: [id: string];
  closeTab: [id: string];
  newTab: [];
  reopenClosedTab: [];
  openRepository: [];
  toggleTerminal: [];
  openSettings: [];
  openIntegrations: [];
  openGitIntegration: [];
  openAdvanced: [];
  openOrganisations: [];
  refreshRepository: [];
  openInVsCode: [];
  openInExplorer: [];
  createGist: [];
  setHistoryView: [mode: HistoryViewMode];
  openLogs: [];
}>();

const menuRoot = ref<HTMLElement | null>(null);
const menuButton = ref<HTMLElement | null>(null);
const menuPanel = ref<HTMLElement | null>(null);
const menuOpen = ref(false);
const showHelpPanel = ref(false);
const showAboutPanel = ref(false);
const activeSection = ref<MenuSection>("file");
const menuPanelStyle = ref<Record<string, string>>({});
const APP_VERSION = "0.1.0";

const sectionLabels: { id: MenuSection; label: string }[] = [
  { id: "file", label: "File" },
  { id: "edit", label: "Edit" },
  { id: "view", label: "View" },
  { id: "options", label: "Options" },
  { id: "help", label: "Help" },
];

const activeTab = computed(() => props.tabs.find((tab) => tab.id === props.activeTabId) ?? null);
const hasActiveRepo = computed(() => !!activeTab.value?.repo);
const activeRepoPath = computed(() => activeTab.value?.repo?.path ?? "");
const canCloseActiveTab = computed(() => props.tabs.length > 1 || !!activeTab.value?.repo);

function canCloseTab(tab: { id: string; repo: RepoInfo | null; label: string }): boolean {
  return props.tabs.length > 1 || !!tab.repo;
}

function closeTabWithMiddleClick(tab: { id: string; repo: RepoInfo | null; label: string }) {
  if (!canCloseTab(tab)) {
    return;
  }
  emit("closeTab", tab.id);
}

function toggleMenu() {
  menuOpen.value = !menuOpen.value;
  if (menuOpen.value) {
    activeSection.value = "file";
    updateMenuPosition();
  }
}

function closeMenu() {
  menuOpen.value = false;
}

function updateMenuPosition() {
  if (!menuButton.value) return;
  const rect = menuButton.value.getBoundingClientRect();
  menuPanelStyle.value = {
    left: `${Math.max(8, rect.left)}px`,
    top: `${Math.max(8, rect.bottom + 6)}px`,
  };
}

function openHelpPanel() {
  showHelpPanel.value = true;
  closeMenu();
}

function openAboutPanel() {
  showAboutPanel.value = true;
  closeMenu();
}

function openProjectGuide() {
  openUrl("https://github.com/mgrabovac22/gitswamp/blob/main/documentation/DOCUMENTATION_31_USER_GUIDE.md").catch(() => {});
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
      description: "Open another workspace tab in GitSwamp.",
      shortcut: "Ctrl+T",
      run: () => emit("newTab"),
    },
    {
      id: "open-repo",
      label: "Open Repository",
      description: "Select a local repository folder and load it.",
      shortcut: "Ctrl+O",
      run: () => emit("openRepository"),
    },
    {
      id: "close-tab",
      label: "Close Current Tab",
      description: "Close the active tab and keep other repositories open.",
      shortcut: "Ctrl+W",
      disabled: !canCloseActiveTab.value,
      run: () => emit("closeTab", props.activeTabId),
    },
    {
      id: "reopen-closed-tab",
      label: "Reopen Closed Tab",
      description: "Restore the most recently closed workspace tab.",
      shortcut: "Ctrl+Shift+T",
      disabled: !props.canReopenClosedTab,
      run: () => emit("reopenClosedTab"),
    },
    {
      id: "create-gist",
      label: "Create a Gist",
      description: "Create and upload a shareable gist from current context.",
      shortcut: "Ctrl+Shift+G",
      disabled: !hasActiveRepo.value,
      run: () => emit("createGist"),
    },
  ],
  edit: [
    {
      id: "copy-path",
      label: "Copy Repository Path",
      description: "Copy the full active repository path to clipboard.",
      shortcut: "Ctrl+Shift+C",
      disabled: !hasActiveRepo.value,
      run: copyActiveRepoPath,
    },
    {
      id: "refresh-repo",
      label: "Refresh Repository",
      description: "Reload status, branches and commits from git.",
      shortcut: "Ctrl+Shift+R",
      disabled: !hasActiveRepo.value,
      run: () => emit("refreshRepository"),
    },
    {
      id: "open-vscode",
      label: "Open in VS Code",
      description: "Open the current repository in Visual Studio Code.",
      shortcut: "Ctrl+Shift+O",
      disabled: !hasActiveRepo.value,
      run: () => emit("openInVsCode"),
    },
  ],
  view: [
    {
      id: "toggle-terminal",
      label: "Toggle Terminal",
      description: "Show or hide the integrated terminal panel.",
      shortcut: "Ctrl+`",
      run: () => emit("toggleTerminal"),
    },
    {
      id: "view-graph",
      label: "Visualise Commit History",
      description: "Open the commit graph timeline view.",
      shortcut: "Alt+1",
      disabled: !hasActiveRepo.value,
      run: () => emit("setHistoryView", "graph"),
    },
    {
      id: "view-galaxy",
      label: "Galaxy View",
      description: "Explore commits and branches as an interactive canvas galaxy.",
      shortcut: "Alt+2",
      disabled: !hasActiveRepo.value,
      run: () => emit("setHistoryView", "galaxy"),
    },
    {
      id: "view-city",
      label: "Repository City",
      description: "Navigate folders, file hotspots and branch activity as a city.",
      shortcut: "Alt+7",
      disabled: !hasActiveRepo.value,
      run: () => emit("setHistoryView", "city"),
    },
    {
      id: "view-productivity",
      label: "Productivity Arena",
      description: "Show commit activity and productivity analytics.",
      shortcut: "Alt+3",
      disabled: !hasActiveRepo.value,
      run: () => emit("setHistoryView", "productivity"),
    },
    {
      id: "view-time-machine",
      label: "Time Machine",
      description: "Navigate history frames and inspect repository state.",
      shortcut: "Alt+4",
      disabled: !hasActiveRepo.value,
      run: () => emit("setHistoryView", "time-machine"),
    },
    {
      id: "view-conflict-heatmap",
      label: "Usual Conflict Suspects",
      description: "Highlight merge hotspots and risky conflict areas.",
      shortcut: "Alt+5",
      disabled: !hasActiveRepo.value,
      run: () => emit("setHistoryView", "conflict-heatmap"),
    },
    {
      id: "view-burnout",
      label: "Burnout Analytics",
      description: "Show contributor focus, after-hours rhythm and hot-file ownership pressure.",
      shortcut: "Alt+6",
      disabled: !hasActiveRepo.value,
      run: () => emit("setHistoryView", "burnout"),
    },
    {
      id: "open-explorer",
      label: "Open in Folder Explorer",
      description: "Open the current repository path in system explorer.",
      shortcut: "Alt+O",
      disabled: !hasActiveRepo.value,
      run: () => emit("openInExplorer"),
    },
  ],
  options: [
    {
      id: "open-integrations",
      label: "Integrations",
      description: "Manage GitHub, GitLab, Bitbucket and Azure connections.",
      shortcut: "Ctrl+Shift+I",
      run: () => emit("openIntegrations"),
    },
    {
      id: "open-git-integration",
      label: "Git Integration",
      description: "Check Git installation and configure background auto-fetch.",
      shortcut: "Ctrl+Shift+K",
      run: () => emit("openGitIntegration"),
    },
    {
      id: "open-preferences",
      label: "Options",
      description: "Configure appearance, behavior and application options.",
      shortcut: "Ctrl+,",
      run: () => emit("openSettings"),
    },
    {
      id: "open-advanced",
      label: "Advanced",
      description: "Configure graph behavior and advanced toggles.",
      run: () => emit("openAdvanced"),
    },
    {
      id: "open-organisations",
      label: "Organisations",
      description: "Manage organisation repositories and batch clone selections.",
      shortcut: "Ctrl+Shift+Y",
      run: () => emit("openOrganisations"),
    },
  ],
  help: [
    {
      id: "help-overview",
      label: "Help and Shortcuts",
      description: "Open help panel with shortcuts and key features.",
      shortcut: "F1",
      run: openHelpPanel,
    },
    {
      id: "help-about",
      label: "About GitSwamp",
      description: "Project summary, creator and technology stack.",
      run: openAboutPanel,
    },
    {
      id: "help-logs",
      label: "Logs",
      description: "Show or hide the app, user and error log panel.",
      shortcut: "Ctrl+Shift+L",
      run: () => emit("openLogs"),
    },
    {
      id: "help-guide",
      label: "Open Online Guide",
      description: "Open the project wiki in your browser.",
      run: openProjectGuide,
    },
    {
      id: "help-issues",
      label: "Report Issue",
      description: "Open issue tracker to report bugs or requests.",
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
  const clickedMenuRoot = menuRoot.value.contains(target);
  const clickedMenuPanel = !!menuPanel.value?.contains(target);
  if (!clickedMenuRoot && !clickedMenuPanel) {
    closeMenu();
  }
}

function onWindowReposition() {
  if (!menuOpen.value) return;
  updateMenuPosition();
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
    showAboutPanel.value = false;
  }
}

onMounted(() => {
  document.addEventListener("mousedown", onDocumentPointerDown);
  document.addEventListener("keydown", onGlobalKeyDown);
  window.addEventListener("resize", onWindowReposition);
  window.addEventListener("scroll", onWindowReposition, true);
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocumentPointerDown);
  document.removeEventListener("keydown", onGlobalKeyDown);
  window.removeEventListener("resize", onWindowReposition);
  window.removeEventListener("scroll", onWindowReposition, true);
});
</script>

<template>
  <div class="tabs-strip relative z-[12] h-10 bg-[var(--background)] flex items-end px-1 border-b border-[var(--border)] gap-px flex-shrink-0 [app-region:no-drag]">
    <div ref="menuRoot" class="relative flex-shrink-0">
      <button
        ref="menuButton"
        type="button"
        class="h-9 w-9 flex items-center justify-center rounded-t-md text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)] transition-all [app-region:no-drag] pointer-events-auto"
        title="Menu"
        @click.stop="toggleMenu"
      >
        <Menu :class="['w-3.5 h-3.5 transition-transform duration-300 ease-out', menuOpen && 'rotate-90']" />
      </button>
    </div>

    <div class="tabs-scroll flex items-end gap-px overflow-x-auto overflow-y-hidden flex-1 min-w-0 [app-region:no-drag]">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        @click="emit('selectTab', tab.id)"
        @mousedown.middle.prevent.stop="closeTabWithMiddleClick(tab)"
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
          v-if="tab.repo || tabs.length > 1"
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
      v-if="menuOpen"
      ref="menuPanel"
      class="fixed z-[7200] w-[450px] bg-[var(--popover)] border border-[var(--border)] rounded-xl shadow-2xl overflow-hidden [app-region:no-drag]"
      :style="menuPanelStyle"
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
            class="menu-action-btn w-full text-left px-2.5 py-2 rounded transition-colors mb-1 last:mb-0"
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
            <div v-if="action.description" class="menu-action-desc text-[10px] text-[var(--muted-foreground)] mt-0.5 pr-8">
              {{ action.description }}
            </div>
          </button>
        </div>
      </div>
    </div>
  </Teleport>

  <Teleport to="body">
    <div
      v-if="showHelpPanel"
      class="fixed inset-0 z-[7100] flex items-center justify-center bg-black/55 backdrop-blur-sm"
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
              <li>Galaxy view: zoom and pan a canvas map of loaded commits, branches and ancestry links.</li>
              <li>Repository City: inspect branch files as districts and buildings with hotspot and activity layers.</li>
              <li>Repository sidebar: local and remote branches, stashes, tags, plus Create a Gist action.</li>
              <li>Right-click menus on commits and branches expose checkout, merge, reset, and branch operations.</li>
              <li>Terminal panel supports git aliases, quick actions, history, reverse search and open-tool commands.</li>
            </ul>
          </section>

          <section>
            <h4 class="text-xs font-semibold text-[var(--foreground)] uppercase tracking-wide mb-2">Git RPG Badges</h4>
            <p class="mb-2 text-[11px] leading-5 text-[var(--muted-foreground)]">
              The small shield near Branch is a lightweight style profile for the current repository. Hover it for the role name, click it to see the role explanation before the Git state summary.
            </p>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
              <div
                v-for="role in GIT_RPG_ROLES"
                :key="role.id"
                class="flex gap-2 rounded border border-[var(--border)] bg-[var(--secondary)]/35 px-2 py-2"
              >
                <GitRpgShield :role="role" size="help" class="flex-shrink-0" />
                <div class="min-w-0">
                  <div class="text-[11px] font-semibold text-[var(--foreground)]">{{ role.title }}</div>
                  <div class="text-[10px] leading-4 text-[var(--muted-foreground)]">{{ role.signal }}</div>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h4 class="text-xs font-semibold text-[var(--foreground)] uppercase tracking-wide mb-2">Shortcuts</h4>
            <div class="space-y-1.5 text-[11px]">
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open help and shortcuts</span><span class="text-[var(--muted-foreground)] font-mono">F1</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open command palette</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+K</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open Pickaxe Explorer</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+F</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">New tab</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+T</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Next tab</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Tab</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Previous tab</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+Tab</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open repository</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+O</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Close active tab</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+W</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Reopen closed tab</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+T</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Toggle terminal panel</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+`</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Refresh repository data</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+R</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open repository in VS Code</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+O</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open repository in folder explorer</span><span class="text-[var(--muted-foreground)] font-mono">Alt+O</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open Galaxy View</span><span class="text-[var(--muted-foreground)] font-mono">Alt+2</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open Usual Conflict Suspects</span><span class="text-[var(--muted-foreground)] font-mono">Alt+5</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open Burnout Analytics</span><span class="text-[var(--muted-foreground)] font-mono">Alt+6</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open Repository City</span><span class="text-[var(--muted-foreground)] font-mono">Alt+7</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Focus commit search</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+R</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open Gist creator</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+G</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open integrations</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+I</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open Git integration</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+K</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open advanced options</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+A</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open organisations</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+Y</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Open options</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+,</span></div>
              <div class="flex items-center justify-between gap-3"><span class="text-[var(--foreground)]">Toggle logs panel</span><span class="text-[var(--muted-foreground)] font-mono">Ctrl+Shift+L</span></div>
            </div>
          </section>
        </div>
      </div>
    </div>
  </Teleport>

  <Teleport to="body">
    <div
      v-if="showAboutPanel"
      class="fixed inset-0 z-[7120] flex items-center justify-center bg-black/55 backdrop-blur-sm"
      @click.self="showAboutPanel = false"
    >
      <div class="w-[560px] max-w-[95vw] max-h-[88vh] overflow-y-auto rounded-2xl border border-[var(--border)] bg-[var(--card)] shadow-2xl">
        <div class="px-4 py-3 border-b border-[var(--border)] flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Info class="w-4 h-4 text-[var(--primary)]" />
            <h3 class="text-sm font-semibold text-[var(--foreground)]">About GitSwamp</h3>
          </div>
          <button
            class="p-1 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
            @click="showAboutPanel = false"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>

        <div class="px-4 py-4 space-y-4">
          <section class="space-y-1">
            <h4 class="text-xs font-semibold text-[var(--foreground)] uppercase tracking-wide">Summary</h4>
            <p class="text-[11px] text-[var(--muted-foreground)] leading-5">
              GitSwamp is a desktop Git client focused on visual history, branch operations, conflict workflows,
              and practical everyday repository actions.
            </p>
          </section>

          <section class="space-y-2">
            <h4 class="text-xs font-semibold text-[var(--foreground)] uppercase tracking-wide">Project Info</h4>
            <div class="grid grid-cols-[140px_1fr] gap-y-1 gap-x-2 text-[11px]">
              <span class="text-[var(--muted-foreground)]">Version</span>
              <span class="text-[var(--foreground)]">{{ APP_VERSION }}</span>
              <span class="text-[var(--muted-foreground)]">Creator</span>
              <span class="text-[var(--foreground)]">Marin Grabovac</span>
              <span class="text-[var(--muted-foreground)]">Interface</span>
              <span class="text-[var(--foreground)]">Vue 3 + TypeScript + Tailwind</span>
              <span class="text-[var(--muted-foreground)]">Desktop Runtime</span>
              <span class="text-[var(--foreground)]">Tauri 2</span>
              <span class="text-[var(--muted-foreground)]">Backend Engine</span>
              <span class="text-[var(--foreground)]">Rust (Git services + Tauri commands)</span>
            </div>
          </section>

          <section class="space-y-2">
            <h4 class="text-xs font-semibold text-[var(--foreground)] uppercase tracking-wide">Highlights</h4>
            <ul class="space-y-1 text-[11px] text-[var(--muted-foreground)]">
              <li>Commit graph and search navigation</li>
              <li>Diff viewer with inline change emphasis</li>
              <li>Branch, stash, tag and remote workflows</li>
              <li>Integrated terminal and context actions</li>
            </ul>
          </section>

          <div class="flex items-center gap-2 pt-1">
            <button
              class="px-2.5 py-1 text-[11px] rounded border border-[var(--diff-border)] bg-[var(--secondary)] hover:opacity-85 text-[var(--foreground)] transition-colors"
              @click="openProjectGuide"
            >
              Open Guide
            </button>
            <button
              class="px-2.5 py-1 text-[11px] rounded border border-[var(--diff-border)] bg-[var(--secondary)] hover:opacity-85 text-[var(--foreground)] transition-colors"
              @click="openIssueTracker"
            >
              Issue Tracker
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.tabs-scroll {
  scrollbar-width: none;
}

.tabs-scroll::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.menu-action-desc {
  display: none;
  line-height: 1.25;
}

:global(html.dummy-mode .menu-action-desc) {
  display: block;
}

</style>

import { computed, ref } from "vue";
import type { RepoInfo } from "@/types";
import type { RepositoryTab } from "./repositoryTabs.types";

const CLOSED_TABS_LIMIT = 10;

interface RepositoryTabsOptions {
  openRepository: (path: string) => void | Promise<void>;
}

function createTabId(): string {
  return `tab-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
}

function createStartTab(id = createTabId()): RepositoryTab {
  return { id, repo: null, label: "Start", path: "" };
}

function cloneTab(tab: RepositoryTab): RepositoryTab {
  return {
    id: tab.id,
    repo: tab.repo ? { ...tab.repo } : null,
    label: tab.label,
    path: tab.path,
  };
}

export function useRepositoryTabs(options: RepositoryTabsOptions) {
  const tabs = ref<RepositoryTab[]>([createStartTab("landing")]);
  const activeTabId = ref("landing");
  const closedTabs = ref<RepositoryTab[]>([]);

  const activeTab = computed(() => tabs.value.find((tab) => tab.id === activeTabId.value));
  const isLanding = computed(() => !activeTab.value?.repo);
  const canReopenClosedTab = computed(() => closedTabs.value.length > 0);
  const canCloseActiveTab = computed(() => tabs.value.length > 1 || !!activeTab.value?.repo);
  const openReposList = computed(() =>
    tabs.value
      .filter((tab) => tab.repo)
      .map((tab) => ({
        name: tab.repo!.name,
        path: tab.repo!.path,
        branch: tab.repo!.current_branch,
      })),
  );

  function restoreTabs(nextTabs: RepositoryTab[], nextActiveTabId?: string) {
    if (!nextTabs.length) return;
    tabs.value = nextTabs;
    activeTabId.value = nextActiveTabId || tabs.value[0].id;
  }

  function selectTab(id: string) {
    if (id === activeTabId.value) return;
    activeTabId.value = id;
    const tab = tabs.value.find((item) => item.id === id);
    if (tab?.path) {
      void options.openRepository(tab.path);
    }
  }

  function selectAdjacentTab(direction: 1 | -1) {
    if (tabs.value.length <= 1) return;

    const currentIndex = tabs.value.findIndex((tab) => tab.id === activeTabId.value);
    const startIndex = currentIndex >= 0 ? currentIndex : 0;
    const nextIndex = (startIndex + direction + tabs.value.length) % tabs.value.length;
    selectTab(tabs.value[nextIndex].id);
  }

  function rememberClosedTab(tab: RepositoryTab) {
    closedTabs.value = [cloneTab(tab), ...closedTabs.value].slice(0, CLOSED_TABS_LIMIT);
  }

  function closeTab(id: string) {
    const idx = tabs.value.findIndex((tab) => tab.id === id);
    if (idx < 0) return;
    if (!tabs.value[idx]?.repo && tabs.value.length <= 1) return;

    const closedTab = tabs.value[idx];
    rememberClosedTab(closedTab);
    tabs.value.splice(idx, 1);

    if (tabs.value.length === 0) {
      const startTab = createStartTab("landing");
      tabs.value.push(startTab);
      activeTabId.value = startTab.id;
      return;
    }

    if (activeTabId.value === id) {
      const nextTab = tabs.value[idx] || tabs.value[idx - 1] || tabs.value[0];
      if (!nextTab) return;
      activeTabId.value = nextTab.id;
      const active = tabs.value.find((tab) => tab.id === activeTabId.value);
      if (active?.path) {
        void options.openRepository(active.path);
      }
    }
  }

  function newTab() {
    const id = createTabId();
    tabs.value.push(createStartTab(id));
    activeTabId.value = id;
  }

  function reopenClosedTab() {
    const [closedTab, ...rest] = closedTabs.value;
    if (!closedTab) return;

    closedTabs.value = rest;
    const id = tabs.value.some((tab) => tab.id === closedTab.id) ? createTabId() : closedTab.id;
    const restoredTab = { ...cloneTab(closedTab), id };
    tabs.value.push(restoredTab);
    activeTabId.value = id;

    if (restoredTab.path) {
      void options.openRepository(restoredTab.path);
    }
  }

  function closeActiveTab() {
    closeTab(activeTabId.value);
  }

  function setActiveTabRepository(repo: RepoInfo) {
    const tab = tabs.value.find((item) => item.id === activeTabId.value);
    if (!tab) return;
    tab.repo = repo;
    tab.label = repo.name;
    tab.path = repo.path;
  }

  return {
    tabs,
    activeTabId,
    activeTab,
    isLanding,
    canCloseActiveTab,
    canReopenClosedTab,
    openReposList,
    restoreTabs,
    selectTab,
    selectAdjacentTab,
    closeTab,
    newTab,
    reopenClosedTab,
    closeActiveTab,
    setActiveTabRepository,
  };
}

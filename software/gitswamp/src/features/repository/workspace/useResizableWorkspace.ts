import { computed, onMounted, onUnmounted, ref } from "vue";
import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

type ResizeTarget = "sidebar" | "details" | "logs" | "terminal";

const SIDEBAR_WIDTH_KEY = "gitswamp-sidebar-width";
const DETAILS_WIDTH_KEY = "gitswamp-details-width";
const LOGS_WIDTH_KEY = "gitswamp-logs-width";
const TERMINAL_HEIGHT_KEY = "gitswamp-terminal-height";

function readNumberPreference(key: string, fallback: number): number {
  const value = Number(safeStorageGet(key));
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

export function useResizableWorkspace(showTerminal: () => boolean) {
  const sidebarWidth = ref(readNumberPreference(SIDEBAR_WIDTH_KEY, 224));
  const detailsWidth = ref(readNumberPreference(DETAILS_WIDTH_KEY, 320));
  const logsWidth = ref(readNumberPreference(LOGS_WIDTH_KEY, 360));
  const terminalHeight = ref(readNumberPreference(TERMINAL_HEIGHT_KEY, 240));
  const resizeTarget = ref<ResizeTarget | null>(null);
  const resizeStartX = ref(0);
  const resizeStartWidth = ref(0);
  const resizeStartY = ref(0);
  const resizeStartHeight = ref(0);
  const workspaceColumnRef = ref<HTMLElement | null>(null);

  const contentAreaStyle = computed(() => {
    if (!showTerminal()) return undefined;
    return { height: `calc(100% - ${terminalHeight.value}px)` };
  });

  const terminalPanelStyle = computed(() => ({
    height: `${terminalHeight.value}px`,
  }));

  function clampWidth(target: Exclude<ResizeTarget, "terminal">, width: number): number {
    if (target === "sidebar") {
      return Math.min(Math.max(width, 180), 420);
    }

    if (target === "logs") {
      return Math.min(Math.max(width, 280), 720);
    }

    return Math.min(Math.max(width, 260), 700);
  }

  function clampTerminalHeight(height: number): number {
    const containerHeight = workspaceColumnRef.value?.clientHeight ?? globalThis.innerHeight;
    const maxHeight = Math.max(180, Math.floor(containerHeight * 0.7));
    return Math.min(Math.max(height, 140), maxHeight);
  }

  function onWindowResize() {
    terminalHeight.value = clampTerminalHeight(terminalHeight.value);
  }

  function beginResize(target: ResizeTarget, event: MouseEvent) {
    resizeTarget.value = target;

    if (target === "terminal") {
      resizeStartY.value = event.clientY;
      resizeStartHeight.value = terminalHeight.value;
      document.body.style.cursor = "row-resize";
    } else {
      resizeStartX.value = event.clientX;
      if (target === "sidebar") {
        resizeStartWidth.value = sidebarWidth.value;
      } else if (target === "logs") {
        resizeStartWidth.value = logsWidth.value;
      } else {
        resizeStartWidth.value = detailsWidth.value;
      }
      document.body.style.cursor = "col-resize";
    }

    document.body.style.userSelect = "none";
  }

  function onPointerMove(event: MouseEvent) {
    if (!resizeTarget.value) return;

    if (resizeTarget.value === "terminal") {
      const deltaY = event.clientY - resizeStartY.value;
      terminalHeight.value = clampTerminalHeight(resizeStartHeight.value - deltaY);
      return;
    }

    const deltaX = event.clientX - resizeStartX.value;
    if (resizeTarget.value === "sidebar") {
      sidebarWidth.value = clampWidth("sidebar", resizeStartWidth.value + deltaX);
    } else if (resizeTarget.value === "logs") {
      logsWidth.value = clampWidth("logs", resizeStartWidth.value - deltaX);
    } else {
      detailsWidth.value = clampWidth("details", resizeStartWidth.value - deltaX);
    }
  }

  function endResize() {
    if (!resizeTarget.value) return;

    if (resizeTarget.value === "terminal") {
      safeStorageSet(TERMINAL_HEIGHT_KEY, String(terminalHeight.value));
    } else if (resizeTarget.value === "logs") {
      safeStorageSet(LOGS_WIDTH_KEY, String(logsWidth.value));
    } else {
      safeStorageSet(SIDEBAR_WIDTH_KEY, String(sidebarWidth.value));
      safeStorageSet(DETAILS_WIDTH_KEY, String(detailsWidth.value));
    }

    resizeTarget.value = null;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }

  onMounted(() => {
    terminalHeight.value = clampTerminalHeight(terminalHeight.value);
    globalThis.addEventListener("mousemove", onPointerMove);
    globalThis.addEventListener("mouseup", endResize);
    globalThis.addEventListener("resize", onWindowResize);
  });

  onUnmounted(() => {
    globalThis.removeEventListener("mousemove", onPointerMove);
    globalThis.removeEventListener("mouseup", endResize);
    globalThis.removeEventListener("resize", onWindowResize);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  });

  return {
    sidebarWidth,
    detailsWidth,
    logsWidth,
    workspaceColumnRef,
    contentAreaStyle,
    terminalPanelStyle,
    beginResize,
  };
}

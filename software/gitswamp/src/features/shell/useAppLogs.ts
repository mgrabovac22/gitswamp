import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

type LogChannel = "app" | "user" | "error";

const MAX_LOG_ROWS = 400;

function normalizeLogMessage(value: string): string {
  const flattened = value.replace(/\s+/g, " ").trim();
  if (flattened.length <= 240) {
    return flattened;
  }
  return `${flattened.slice(0, 240)}...`;
}

export function useAppLogs() {
  const showLogsPanel = ref(safeStorageGet("gitswamp-show-logs-panel") === "true");
  const appLogs = ref<string[]>([]);
  const userLogs = ref<string[]>([]);
  const errorLogs = ref<string[]>([]);

  function appendLog(target: LogChannel, message: string) {
    const row = `[${new Date().toLocaleTimeString()}] ${normalizeLogMessage(message)}`;
    let bucket = appLogs;
    if (target === "user") {
      bucket = userLogs;
    } else if (target === "error") {
      bucket = errorLogs;
    }

    bucket.value.push(row);
    if (bucket.value.length > MAX_LOG_ROWS) {
      bucket.value.splice(0, bucket.value.length - MAX_LOG_ROWS);
    }

    invoke("append_app_log", {
      channel: target,
      message: row,
    }).catch(() => {});
  }

  function openLogsPanel() {
    showLogsPanel.value = true;
    appendLog("user", "Opened logs panel.");
  }

  watch(showLogsPanel, (value) => {
    safeStorageSet("gitswamp-show-logs-panel", String(value));
  });

  return {
    showLogsPanel,
    appLogs,
    userLogs,
    errorLogs,
    appendLog,
    openLogsPanel,
  };
}

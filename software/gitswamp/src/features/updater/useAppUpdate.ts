import { computed, ref } from "vue";
import { Channel, invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { openUrl } from "@tauri-apps/plugin-opener";
import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

export interface AppUpdateInfo {
  tag: string;
  version: string;
  currentVersion: string;
  name: string;
  notes: string;
  publishedAt: string;
  prerelease: boolean;
  htmlUrl: string;
  manifestUrl: string;
}

export type AppUpdateStatus =
  | "idle"
  | "checking"
  | "none"
  | "available"
  | "installing"
  | "installed"
  | "error";

type DownloadEvent =
  | { event: "Started"; data: { contentLength: number | null } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

const DISMISS_STORAGE_KEY = "gitswamp-dismissed-update";

const status = ref<AppUpdateStatus>("idle");
const info = ref<AppUpdateInfo | null>(null);
const errorMessage = ref("");
const downloaded = ref(0);
const total = ref<number | null>(null);

const percent = computed(() => {
  const size = total.value;
  if (!size || size <= 0) return null;
  return Math.min(100, Math.round((downloaded.value / size) * 100));
});

const isVisible = computed(
  () => info.value !== null && (status.value === "available" || status.value === "installing"),
);

function setBytes(value: number) {
  downloaded.value = value;
}

async function check(force = false): Promise<void> {
  if (status.value === "checking" || status.value === "installing") return;
  status.value = "checking";
  errorMessage.value = "";
  try {
    const result = await invoke<AppUpdateInfo | null>("check_for_update");
    if (!result) {
      status.value = "none";
      return;
    }
    if (!force && safeStorageGet(DISMISS_STORAGE_KEY) === result.tag) {
      status.value = "none";
      return;
    }
    info.value = result;
    status.value = "available";
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
    status.value = "error";
  }
}

async function install(): Promise<void> {
  if (!info.value || status.value === "installing") return;
  status.value = "installing";
  errorMessage.value = "";
  setBytes(0);
  total.value = null;

  try {
    const channel = new Channel<DownloadEvent>();
    channel.onmessage = (message) => {
      if (message.event === "Started") {
        total.value = message.data.contentLength ?? null;
      } else if (message.event === "Progress") {
        setBytes(downloaded.value + message.data.chunkLength);
      }
    };

    await invoke("install_update", { onEvent: channel });
    status.value = "installed";

    try {
      await relaunch();
    } catch {
      // On Windows the installer exits the app before relaunch resolves.
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    errorMessage.value = message.includes("InvalidUpdaterFormat")
      ? "Linux automatic updates work only when GitSwamp is running from an AppImage. Install the latest AppImage once, then future updates will install automatically."
      : message;
    status.value = "available";
  }
}

function dismiss(): void {
  if (info.value) {
    safeStorageSet(DISMISS_STORAGE_KEY, info.value.tag);
  }
  status.value = "none";
}

async function openReleasePage(): Promise<void> {
  if (!info.value?.htmlUrl) return;
  await openUrl(info.value.htmlUrl);
}

export function useAppUpdate() {
  return {
    status,
    info,
    errorMessage,
    downloaded,
    total,
    percent,
    isVisible,
    check,
    install,
    dismiss,
    openReleasePage,
  };
}

export function checkForAppUpdate(): Promise<void> {
  return check();
}

export function checkForAppUpdateManually(): Promise<void> {
  return check(true);
}

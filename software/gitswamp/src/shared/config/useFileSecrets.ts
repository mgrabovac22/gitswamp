import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { FileStatusInfo } from "@/types";
import type { SecretScan } from "@/shared/notifications/secretConfirmation";
import { SECRET_DETECTOR_EVENT, secretDetectorEnabled } from "./secretDetectorPreferences";

export function useFileSecrets(repo: () => string, files: () => FileStatusInfo[], active: () => boolean, staged = false) {
  const enabled = ref(secretDetectorEnabled());
  const report = shallowRef<SecretScan | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let generation = 0, busy = false, disposed = false, lastStart = 0;
  const findingsByPath = computed(() => {
    const result = new Map<string, string[]>();
    for (const finding of report.value?.findings ?? []) {
      const entries = result.get(finding.path) ?? [];
      entries.push(`${finding.kind} · line ${finding.line}`);
      result.set(finding.path, entries);
    }
    return result;
  });
  function schedule() {
    generation++;
    clearTimeout(timer);
    if (!enabled.value || !active() || !repo() || !files().length) { report.value = null; return; }
    timer = setTimeout(run, Math.max(800, 5000 - (Date.now() - lastStart)));
  }
  async function run() {
    if (disposed) return;
    if (busy) { timer = setTimeout(run, 800); return; }
    const token = generation, path = repo();
    busy = true; lastStart = Date.now();
    try {
      const result = await invoke<SecretScan>("scan_secrets", { path, staged, filePaths: files().slice(0, 2048).map(file => file.path) });
      if (!disposed && token === generation && path === repo()) report.value = result;
    } catch { if (!disposed && token === generation) report.value = null; }
    finally { busy = false; }
  }
  function settingsChanged() { enabled.value = secretDetectorEnabled(); schedule(); }
  watch(() => [repo(), active()], () => { report.value = null; schedule(); });
  watch(files, schedule, { immediate: true });
  onMounted(() => globalThis.addEventListener(SECRET_DETECTOR_EVENT, settingsChanged));
  onBeforeUnmount(() => { disposed = true; generation++; clearTimeout(timer); globalThis.removeEventListener(SECRET_DETECTOR_EVENT, settingsChanged); });
  return { findingsByPath };
}

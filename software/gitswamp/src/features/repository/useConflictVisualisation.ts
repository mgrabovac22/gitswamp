import { onScopeDispose, ref, shallowRef, watch } from "vue";
import { VISUALISE_OPERATIONS_EVENT, visualiseOperationsEnabled } from "@/shared/config/visualiseOperationsPreferences";
import type { FileStatusInfo } from "@/types";
import { DESTRUCTIVE_ACTION_UNDONE_EVENT } from "@/shared/notifications/destructiveScheduler";

export const CONFLICT_PREVIEW_MS = 3000;

export interface ConflictVisualisation {
  repoPath: string;
  paths: string[];
}

export function useConflictVisualisation(repoPath: () => string, ready: () => boolean, conflicts: () => FileStatusInfo[]) {
  const enabled = ref(visualiseOperationsEnabled());
  const preview = shallowRef<ConflictVisualisation | null>(null);
  let baseline: { repoPath: string; paths: Set<string> } | null = null;

  function close() {
    preview.value = null;
  }

  function settingsChanged() {
    enabled.value = visualiseOperationsEnabled();
  }

  watch([enabled, repoPath, ready, conflicts], ([on, path, loaded, files]) => {
    if (!on || !path || !loaded) {
      baseline = null;
      close();
      return;
    }
    const previous = baseline;
    const paths = new Set(files.map(file => file.path));
    baseline = { repoPath: path, paths };
    if (!previous || previous.repoPath !== path || paths.size === 0) {
      close();
      return;
    }
    if (preview.value) return;
    const added = [...paths].filter(file => !previous.paths.has(file));
    if (added.length) preview.value = { repoPath: path, paths: added };
  }, { immediate: true });

  watch(enabled, (on, _, onCleanup) => {
    if (!on) return;
    globalThis.addEventListener(DESTRUCTIVE_ACTION_UNDONE_EVENT, close);
    onCleanup(() => globalThis.removeEventListener(DESTRUCTIVE_ACTION_UNDONE_EVENT, close));
  }, { immediate: true, flush: "sync" });

  globalThis.addEventListener(VISUALISE_OPERATIONS_EVENT, settingsChanged);
  onScopeDispose(() => {
    globalThis.removeEventListener(VISUALISE_OPERATIONS_EVENT, settingsChanged);
    close();
  });
  return { preview, close };
}

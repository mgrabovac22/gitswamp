import { onScopeDispose, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { VISUALISE_OPERATIONS_EVENT, visualiseOperationsEnabled } from "@/shared/config/visualiseOperationsPreferences";
import { GIT_OPERATION_PREVIEW_EVENT, GIT_PENDING_PREVIEW_SETTLED_EVENT, type CommitPreviewRequest, type OperationPreviewRequest } from "@/shared/notifications/gitOperationPreview";
import { DESTRUCTIVE_ACTION_UNDONE_EVENT } from "@/shared/notifications/destructiveScheduler";

export interface OperationCommit {
  sha: string;
  subject: string;
}

interface OperationSummary {
  before: OperationCommit;
  result: OperationCommit;
  incoming: OperationCommit[];
  hasMore: boolean;
}

export type OperationVisualisation = (CommitPreviewRequest & OperationSummary) | Exclude<OperationPreviewRequest, CommitPreviewRequest>;

export function useOperationVisualisation(repoPath: () => string, blocked: () => boolean) {
  const preview = shallowRef<OperationVisualisation | null>(null);
  let generation = 0;
  let loading = false;
  let listening = false;

  function close() {
    generation++;
    preview.value = null;
  }

  function settle(event: Event) {
    if (preview.value && "pendingUndo" in preview.value && preview.value.pendingUndo === (event as CustomEvent).detail) close();
  }

  async function receive(event: Event) {
    const request = (event as CustomEvent<OperationPreviewRequest>).detail;
    if (!request || request.repoPath !== repoPath() || blocked()) return;
    const pending = "pendingUndo" in request && request.pendingUndo;
    if (!pending && (loading || (preview.value && "pendingUndo" in preview.value && preview.value.pendingUndo))) return;
    close();
    if (!("source" in request)) {
      preview.value = request;
      return;
    }
    const token = generation;
    loading = true;
    try {
      const summary = await invoke<OperationSummary | null>("get_operation_preview", {
        path: request.repoPath,
        kind: request.kind,
        resultSha: request.resultSha,
        beforeSha: request.beforeSha,
        sourceSha: request.sourceSha,
      });
      if (summary && token === generation && request.repoPath === repoPath() && !blocked() && visualiseOperationsEnabled()) {
        preview.value = { ...request, ...summary };
      }
    } catch {
      // The operation already succeeded; optional visuals must never report it as failed.
    } finally {
      loading = false;
    }
  }

  function settingsChanged() {
    const enabled = visualiseOperationsEnabled();
    if (enabled === listening) return;
    listening = enabled;
    if (enabled) {
      globalThis.addEventListener(GIT_OPERATION_PREVIEW_EVENT, receive);
      globalThis.addEventListener(DESTRUCTIVE_ACTION_UNDONE_EVENT, close);
      globalThis.addEventListener(GIT_PENDING_PREVIEW_SETTLED_EVENT, settle);
    }
    else {
      globalThis.removeEventListener(GIT_OPERATION_PREVIEW_EVENT, receive);
      globalThis.removeEventListener(DESTRUCTIVE_ACTION_UNDONE_EVENT, close);
      globalThis.removeEventListener(GIT_PENDING_PREVIEW_SETTLED_EVENT, settle);
      close();
    }
  }

  watch([repoPath, blocked], close, { flush: "sync" });
  globalThis.addEventListener(VISUALISE_OPERATIONS_EVENT, settingsChanged);
  settingsChanged();
  onScopeDispose(() => {
    globalThis.removeEventListener(VISUALISE_OPERATIONS_EVENT, settingsChanged);
    globalThis.removeEventListener(GIT_OPERATION_PREVIEW_EVENT, receive);
    globalThis.removeEventListener(DESTRUCTIVE_ACTION_UNDONE_EVENT, close);
    globalThis.removeEventListener(GIT_PENDING_PREVIEW_SETTLED_EVENT, settle);
    close();
  });

  return { preview, close };
}

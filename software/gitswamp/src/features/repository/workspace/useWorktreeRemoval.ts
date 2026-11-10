import { ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { isOriginalWorktree, type WorkspaceRemovalRequest } from "./branchWorkspaceTypes";

export function useWorktreeRemoval(onRemoved: (request: WorkspaceRemovalRequest) => void) {
  const pending = shallowRef<WorkspaceRemovalRequest | null>(null);
  const busy = ref(false);
  const error = ref("");

  function requestRemoval(request: WorkspaceRemovalRequest) {
    if (pending.value || isOriginalWorktree(request.sourcePath, request.location)) return;
    pending.value = request;
    error.value = "";
  }

  function cancel() {
    if (!busy.value) pending.value = null;
  }

  function closeOnly() {
    if (busy.value || !pending.value) return;
    const request = pending.value;
    pending.value = null;
    request.finish();
  }

  async function remove() {
    const request = pending.value;
    if (!request || busy.value) return;
    busy.value = true;
    error.value = "";
    try {
      await invoke("remove_branch_workspace", {
        path: request.sourcePath, worktreePath: request.location.path, branch: request.location.branch,
      });
    } catch (cause) {
      error.value = String(cause);
      return;
    } finally { busy.value = false; }
    pending.value = null;
    onRemoved(request);
    request.finish();
  }

  return { pending, busy, error, requestRemoval, cancel, closeOnly, remove };
}

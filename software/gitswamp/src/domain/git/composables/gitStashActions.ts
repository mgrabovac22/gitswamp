import { useToast } from "@/shared/notifications/useToast";
import { publishOperationPreview } from "@/shared/notifications/gitOperationPreview";

import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";
import type { StashInfo } from "@/types";

type RefreshDeps = {
  refreshStatus: () => Promise<void>;
  refreshStashes: () => Promise<void>;
  selectStash: (stash: StashInfo | null) => Promise<void>;
};

export function createStashActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  let stashOperationInProgress = false;

  function previewStash(repoPath: string, kind: "stash-save" | "stash-restore", label: string) {
    if (state.repoPath.value === repoPath) {
      publishOperationPreview({ kind, repoPath, target: "Working Changes", endpoint: { label } });
    }
  }

  async function syncSelectedStash(repoPath: string) {
    if (repoPath !== state.repoPath.value || !state.selectedStash.value) return;
    const current = state.stashes.value.find(stash => stash.stash_sha === state.selectedStash.value?.stash_sha);
    if (current) state.selectedStash.value = current;
    else await refresh.selectStash(null);
  }

  async function stashPush(message?: string, includeUntracked = true) {
    if (!state.repoPath.value) return;
    const repoPath = state.repoPath.value;
    if (state.hasConflicts.value) {
      toast.error("Cannot stash while conflicts exist. Resolve conflicts first.");
      return;
    }

    if (stashOperationInProgress) {
      toast.info("A stash operation is already running. Please wait.");
      return;
    }

    stashOperationInProgress = true;
    state.error.value = null;
    const loadingToast = toast.loading("Creating stash...");

    try {
      const result = await callTauri<string>("stash_push", {
        path: repoPath,
        message: message || null,
        includeUntracked,
      });
      state.terminalOutput.value.push(
        "$ git stash push"
          + (includeUntracked ? " --include-untracked" : "")
          + (message ? ' -m "' + message + '"' : "")
          + "\n"
          + result,
      );
      if (repoPath !== state.repoPath.value) return;
      state.fileStatuses.value = includeUntracked ? [] : state.fileStatuses.value.filter(file => !file.staged && (file.status === "new" || file.status === "untracked"));
      const validated = Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
      toast.remove(loadingToast);
      toast.success(includeUntracked ? "Stash created with untracked files." : "Tracked changes stashed.", 2500);
      previewStash(repoPath, "stash-save", "Stash");
      await validated;
      await syncSelectedStash(repoPath);
    } catch (e) {
      if (repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash push\nError: " + e);
      toast.error("Failed to create stash: " + String(e));
      if (repoPath === state.repoPath.value) await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
    } finally {
      toast.remove(loadingToast);
      stashOperationInProgress = false;
    }
  }

  async function stashPop(index = 0) {
    if (!state.repoPath.value) return;
    const repoPath = state.repoPath.value;
    if (state.hasConflicts.value) {
      toast.error("Cannot pop stash while conflicts exist. Resolve conflicts first.");
      return;
    }

    if (stashOperationInProgress) {
      toast.info("A stash operation is already running. Please wait.");
      return;
    }

    stashOperationInProgress = true;
    state.error.value = null;
    const loadingToast = toast.loading(`Popping stash@{${index}}...`);

    try {
      const result = await callTauri<string>("stash_pop", { path: repoPath, index });
      if (repoPath !== state.repoPath.value) return;
      state.terminalOutput.value.push("$ git stash pop stash@{" + index + "}\n" + result);
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
      if (repoPath !== state.repoPath.value) return;
      await syncSelectedStash(repoPath);
      toast.success(`stash@{${index}} popped.`, 2500);
      previewStash(repoPath, "stash-restore", `stash@{${index}}`);
    } catch (e) {
      if (repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash pop\nError: " + e);
      toast.error(`Failed to pop stash@{${index}}: ${String(e)}`);
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
    } finally {
      toast.remove(loadingToast);
      stashOperationInProgress = false;
    }
  }

  async function stashApply(index = 0) {
    if (!state.repoPath.value) return;
    const repoPath = state.repoPath.value;
    if (state.hasConflicts.value) {
      toast.error("Cannot apply stash while conflicts exist. Resolve conflicts first.");
      return;
    }

    if (stashOperationInProgress) {
      toast.info("A stash operation is already running. Please wait.");
      return;
    }

    stashOperationInProgress = true;
    state.error.value = null;
    const loadingToast = toast.loading(`Applying stash@{${index}}...`);

    try {
      const result = await callTauri<string>("stash_apply", { path: repoPath, index });
      if (repoPath !== state.repoPath.value) return;
      state.terminalOutput.value.push("$ git stash apply stash@{" + index + "}\n" + result);
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
      if (repoPath !== state.repoPath.value) return;
      toast.success(`stash@{${index}} applied.`, 2500);
      previewStash(repoPath, "stash-restore", `stash@{${index}}`);
    } catch (e) {
      if (repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash apply\nError: " + e);
      toast.error(`Failed to apply stash@{${index}}: ${String(e)}`);
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
    } finally {
      toast.remove(loadingToast);
      stashOperationInProgress = false;
    }
  }

  async function stashDrop(index = 0) {
    if (!state.repoPath.value) return;
    const repoPath = state.repoPath.value;

    if (stashOperationInProgress) {
      toast.info("A stash operation is already running. Please wait.");
      return;
    }

    stashOperationInProgress = true;
    state.error.value = null;
    const loadingToast = toast.loading(`Dropping stash@{${index}}...`);

    try {
      const result = await callTauri<string>("stash_drop", { path: repoPath, index });
      if (repoPath !== state.repoPath.value) return;
      state.terminalOutput.value.push("$ git stash drop stash@{" + index + "}\n" + result);
      await refresh.refreshStashes();
      if (repoPath !== state.repoPath.value) return;
      await syncSelectedStash(repoPath);
      toast.success(`stash@{${index}} dropped.`, 2500);
    } catch (e) {
      if (repoPath !== state.repoPath.value) return;
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash drop\nError: " + e);
      toast.error(`Failed to drop stash@{${index}}.`);
    } finally {
      toast.remove(loadingToast);
      stashOperationInProgress = false;
    }
  }

  async function restoreSelectedStash(stash: StashInfo, paths: string[], pop: boolean) {
    const repoPath = state.repoPath.value;
    if (!repoPath || !paths.length || stashOperationInProgress) return;
    if (state.hasConflicts.value) { toast.warning("Resolve existing conflicts before restoring stash files."); return; }
    stashOperationInProgress = true;
    state.error.value = null;
    const pending = toast.loading("Restoring selected stash files…");
    try {
      const result = await callTauri<{ message: string; remainder?: string }>("restore_selected_stash", {
        path: repoPath, index: stash.index, sha: stash.stash_sha, paths, pop,
      });
      if (repoPath !== state.repoPath.value) return;
      await Promise.all([refresh.refreshStatus(), ...(pop ? [refresh.refreshStashes()] : [])]);
      if (repoPath !== state.repoPath.value) return;
      if (pop && state.selectedStash.value?.stash_sha === stash.stash_sha) {
        await refresh.selectStash(state.stashes.value.find(item => item.stash_sha === result.remainder) ?? null);
      }
      toast.success(result.message);
      previewStash(repoPath, "stash-restore", `${paths.length} selected files`);
    } catch (error) {
      if (repoPath !== state.repoPath.value) return;
      state.error.value = String(error);
      toast.error(String(error));
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
    } finally {
      toast.remove(pending);
      stashOperationInProgress = false;
    }
  }

  return {
    stashPush,
    stashPop,
    stashApply,
    stashDrop,
    restoreSelectedStash,
  };
}

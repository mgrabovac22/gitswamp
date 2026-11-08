import { useToast } from "@/shared/notifications/useToast";
import { publishOperationPreview } from "@/shared/notifications/gitOperationPreview";

import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshStatus: () => Promise<void>;
  refreshStashes: () => Promise<void>;
};

export function createStashActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  let stashOperationInProgress = false;

  function previewStash(repoPath: string, kind: "stash-save" | "stash-restore", label: string) {
    if (state.repoPath.value === repoPath) {
      publishOperationPreview({ kind, repoPath, target: "Working Changes", endpoint: { label } });
    }
  }

  async function stashPush(message?: string, includeUntracked = false) {
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
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
      toast.success(includeUntracked ? "Stash created with untracked files." : "Tracked changes stashed.", 2500);
      previewStash(repoPath, "stash-save", "Stash");
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash push\nError: " + e);
      toast.error("Failed to create stash: " + String(e));
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
      state.terminalOutput.value.push("$ git stash pop stash@{" + index + "}\n" + result);
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
      toast.success(`stash@{${index}} popped.`, 2500);
      previewStash(repoPath, "stash-restore", `stash@{${index}}`);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash pop\nError: " + e);
      toast.error(`Failed to pop stash@{${index}}.`);
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
      state.terminalOutput.value.push("$ git stash apply stash@{" + index + "}\n" + result);
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
      toast.success(`stash@{${index}} applied.`, 2500);
      previewStash(repoPath, "stash-restore", `stash@{${index}}`);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash apply\nError: " + e);
      toast.error(`Failed to apply stash@{${index}}.`);
    } finally {
      toast.remove(loadingToast);
      stashOperationInProgress = false;
    }
  }

  async function stashDrop(index = 0) {
    if (!state.repoPath.value) return;

    if (stashOperationInProgress) {
      toast.info("A stash operation is already running. Please wait.");
      return;
    }

    stashOperationInProgress = true;
    state.error.value = null;
    const loadingToast = toast.loading(`Dropping stash@{${index}}...`);

    try {
      const result = await callTauri<string>("stash_drop", { path: state.repoPath.value, index });
      state.terminalOutput.value.push("$ git stash drop stash@{" + index + "}\n" + result);
      await refresh.refreshStashes();
      toast.success(`stash@{${index}} dropped.`, 2500);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash drop\nError: " + e);
      toast.error(`Failed to drop stash@{${index}}.`);
    } finally {
      toast.remove(loadingToast);
      stashOperationInProgress = false;
    }
  }

  return {
    stashPush,
    stashPop,
    stashApply,
    stashDrop,
  };
}

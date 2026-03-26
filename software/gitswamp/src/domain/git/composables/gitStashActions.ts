import { useToast } from "@/shared/notifications/useToast";

import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshStatus: () => Promise<void>;
  refreshStashes: () => Promise<void>;
  refreshCommits: () => Promise<void>;
};

export function createStashActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  async function stashPush(message?: string) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot stash while conflicts exist. Resolve conflicts first.");
      return;
    }
    try {
      const result = await callTauri<string>("stash_push", {
        path: state.repoPath.value,
        message: message || null,
      });
      state.terminalOutput.value.push("$ git stash push" + (message ? ' -m "' + message + '"' : "") + "\n" + result);
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes(), refresh.refreshCommits()]);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash push\nError: " + e);
    }
  }

  async function stashPop(index = 0) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot pop stash while conflicts exist. Resolve conflicts first.");
      return;
    }
    try {
      const result = await callTauri<string>("stash_pop", { path: state.repoPath.value, index });
      state.terminalOutput.value.push("$ git stash pop stash@{" + index + "}\n" + result);
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes(), refresh.refreshCommits()]);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git stash pop\nError: " + e);
    }
  }

  async function stashApply(index = 0) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot apply stash while conflicts exist. Resolve conflicts first.");
      return;
    }
    try {
      const result = await callTauri<string>("stash_apply", { path: state.repoPath.value, index });
      state.terminalOutput.value.push("$ git stash apply stash@{" + index + "}\n" + result);
      await Promise.all([refresh.refreshStatus(), refresh.refreshStashes()]);
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function stashDrop(index = 0) {
    if (!state.repoPath.value) return;
    try {
      const result = await callTauri<string>("stash_drop", { path: state.repoPath.value, index });
      state.terminalOutput.value.push("$ git stash drop stash@{" + index + "}\n" + result);
      await refresh.refreshStashes();
    } catch (e) {
      state.error.value = String(e);
    }
  }

  return {
    stashPush,
    stashPop,
    stashApply,
    stashDrop,
  };
}

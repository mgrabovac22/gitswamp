import type { RepoInfo } from "@/types";
import { useToast } from "@/shared/notifications/useToast";

import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshCommits: () => Promise<void>;
  refreshBranches: () => Promise<void>;
  refreshStatus: () => Promise<void>;
};

export function createBranchActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  async function checkoutBranch(branchName: string) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot checkout branch while conflicts exist. Resolve conflicts first.");
      return;
    }
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading(`Loading: checking out branch ${branchName}...`);
      state.loading.value = true;
      await callTauri("checkout_branch", { path: state.repoPath.value, branchName });
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      toast.success(`Checked out branch "${branchName}"`);
    } catch (e) {
      state.error.value = String(e);
      toast.error("Checkout branch failed: " + String(e));
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function createBranch(name: string, startPoint?: string) {
    if (!state.repoPath.value) return;
    try {
      state.loading.value = true;
      await callTauri("create_branch", { path: state.repoPath.value, name, startPoint: startPoint || null });
      await Promise.all([refresh.refreshBranches(), refresh.refreshCommits()]);
    } catch (e) {
      state.error.value = String(e);
    } finally {
      state.loading.value = false;
    }
  }

  async function deleteBranch(name: string) {
    if (!state.repoPath.value) return;
    try {
      state.loading.value = true;
      await callTauri("delete_branch", { path: state.repoPath.value, name });
      await Promise.all([refresh.refreshBranches(), refresh.refreshCommits()]);
    } catch (e) {
      state.error.value = String(e);
    } finally {
      state.loading.value = false;
    }
  }

  async function renameBranch(oldName: string, newName: string) {
    if (!state.repoPath.value) return;
    try {
      state.loading.value = true;
      const result = await callTauri<string>("rename_branch", { path: state.repoPath.value, oldName, newName });
      state.terminalOutput.value.push("$ git branch -m " + oldName + " " + newName + "\n" + (result || "(done)"));
      await Promise.all([refresh.refreshBranches(), refresh.refreshCommits()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git branch -m\nError: " + e);
    } finally {
      state.loading.value = false;
    }
  }

  async function mergeBranchIntoCurrent(sourceBranch: string, sourceRemote = false, targetBranch?: string) {
    if (!state.repoPath.value) return;
    const current = state.repoInfo.value?.current_branch || targetBranch || "";
    if (!current) {
      toast.error("No active branch to merge into.");
      return;
    }
    const sourceRef = sourceRemote ? `origin/${sourceBranch}` : sourceBranch;
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading(`Loading: merging ${sourceRef}...`);
      state.loading.value = true;
      const result = await callTauri<string>("run_git_command", {
        path: state.repoPath.value,
        args: ["merge", sourceRef],
      });
      state.terminalOutput.value.push(`$ git merge ${sourceRef}\n` + (result || "(done)"));
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
      toast.success(`Merged ${sourceRef} into ${current}`);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push(`$ git merge ${sourceRef}\nError: ${e}`);
      toast.error("Merge failed: " + String(e));
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  return {
    checkoutBranch,
    createBranch,
    deleteBranch,
    renameBranch,
    mergeBranchIntoCurrent,
  };
}

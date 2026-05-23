import type { RepoInfo, MergeRiskPreflight } from "@/types";
import { useToast } from "@/shared/notifications/useToast";

import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshCommits: () => Promise<void>;
  refreshBranches: () => Promise<void>;
  refreshStatus: () => Promise<void>;
};

type RebaseExecutionState = "ok" | "conflict" | "error";

const REBASE_CONFLICT_MARKER = "REBASE_CONFLICT:";

function isRebaseConflictMessage(value: unknown): boolean {
  let text = "";
  if (typeof value === "string") {
    text = value;
  } else if (typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
    text = String(value);
  } else if (value && typeof value === "object" && "message" in value) {
    const maybeMessage = (value as { message?: unknown }).message;
    if (typeof maybeMessage === "string") {
      text = maybeMessage;
    }
  }

  const normalized = text.toLowerCase();
  return normalized.includes(REBASE_CONFLICT_MARKER.toLowerCase())
    || (normalized.includes("rebase") && normalized.includes("conflict"));
}

function buildPreflightMessage(report: MergeRiskPreflight): string {
  const samples = report.suspect_files
    .slice(0, 5)
    .map((item) => `- ${item.path}`)
    .join("\n");

  return [
    `Merge pre-check flagged ${report.risk_level.toUpperCase()} risk for ${report.source_ref} -> ${report.target_ref}.`,
    `Shared changed files: ${report.shared_change_count}`,
    `Suspect files: ${report.suspect_count}`,
    "",
    "Top suspects:",
    samples || "- no suspect files",
    "",
    "Continue merge?",
  ].join("\n");
}

export function createBranchActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  async function fetchMergePreflight(
    sourceBranch: string,
    sourceRemote: boolean,
    targetBranch: string,
  ): Promise<MergeRiskPreflight | null> {
    if (!state.repoPath.value) {
      return null;
    }

    try {
      return await callTauri<MergeRiskPreflight>("get_merge_preflight_risk", {
        path: state.repoPath.value,
        sourceBranch,
        sourceRemote,
        targetBranch,
        maxCount: 500,
        lookbackMonths: 6,
      });
    } catch {
      return null;
    }
  }

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

  async function createBranch(name: string, startPoint?: string): Promise<boolean> {
    if (!state.repoPath.value) return false;
    try {
      state.loading.value = true;
      await callTauri("create_branch", { path: state.repoPath.value, name, startPoint: startPoint || null });
      await Promise.all([refresh.refreshBranches(), refresh.refreshCommits()]);
      toast.success(startPoint ? `Branch "${name}" created at selected commit` : `Branch "${name}" created`);
      return true;
    } catch (e) {
      state.error.value = String(e);
      toast.error("Create branch failed: " + String(e));
      return false;
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
    const current = state.repoInfo.value?.current_branch || "";
    const target = (targetBranch || current).trim();
    if (!target) {
      toast.error("No active branch to merge into.");
      return;
    }
    const sourceRef = sourceRemote ? `origin/${sourceBranch}` : sourceBranch;

    const preflight = await fetchMergePreflight(sourceBranch, sourceRemote, target);
    if (preflight && preflight.risk_level !== "low" && preflight.suspect_count > 0) {
      const confirmed = typeof globalThis.confirm === "function"
        ? globalThis.confirm(buildPreflightMessage(preflight))
        : true;

      if (!confirmed) {
        toast.info("Merge cancelled after risk pre-check.");
        return;
      }

      toast.warning(
        `Proceeding with ${preflight.risk_level} merge risk (${preflight.suspect_count} suspect files).`,
      );
    } else if (preflight && preflight.suspect_count > 0) {
      toast.info(`Pre-check: ${preflight.suspect_count} suspect files detected for this merge path.`);
    }

    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading(`Loading: merging ${sourceRef} into ${target}...`);
      state.loading.value = true;

      if (current && current !== target) {
        await callTauri("checkout_branch", { path: state.repoPath.value, branchName: target });
      }

      const result = await callTauri<string>("run_git_command", {
        path: state.repoPath.value,
        args: ["merge", sourceRef],
      });
      state.terminalOutput.value.push(`$ git checkout ${target}\n$ git merge ${sourceRef}\n` + (result || "(done)"));
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
      toast.success(`Merged ${sourceRef} into ${target}`);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push(`$ git checkout ${target}\n$ git merge ${sourceRef}\nError: ${e}`);
      toast.error("Merge failed: " + String(e));
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function rebaseBranchOnto(
    sourceBranch: string,
    sourceRemote = false,
    targetBranch?: string,
  ): Promise<RebaseExecutionState> {
    if (!state.repoPath.value) return "error";
    const source = sourceBranch.trim();
    const current = state.repoInfo.value?.current_branch || "";
    const target = (targetBranch || current).trim();
    if (!source) {
      toast.error("No source branch to rebase.");
      return "error";
    }
    if (!target) {
      toast.error("No target branch to rebase onto.");
      return "error";
    }
    if (source === target) {
      toast.info("Source and target branches are the same.");
      return "error";
    }

    const sourceRef = sourceRemote ? `origin/${source}` : source;
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading(`Loading: rebasing ${sourceRef} onto ${target}...`);
      state.loading.value = true;

      const result = await callTauri<string>("rebase_branch_onto", {
        path: state.repoPath.value,
        sourceBranch: source,
        sourceRemote,
        targetBranch: target,
      });

      state.terminalOutput.value.push(`$ git checkout ${source}\n$ git rebase ${target}\n` + (result || "(done)"));
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
      toast.success(`Rebased ${source} onto ${target}`);
      return "ok";
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push(`$ git checkout ${source}\n$ git rebase ${target}\nError: ${e}`);
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });

      if (isRebaseConflictMessage(e)) {
        toast.warning("Rebase paused due to conflicts. Resolve files, then Continue/Skip/Abort.");
        return "conflict";
      }

      toast.error("Rebase failed: " + String(e));
      return "error";
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function runRebaseFollowUpAction(
    command: "rebase_continue" | "rebase_skip" | "rebase_abort",
    title: string,
    terminalCommand: string,
  ): Promise<RebaseExecutionState> {
    if (!state.repoPath.value) {
      return "error";
    }

    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading(`Loading: ${title.toLowerCase()}...`);
      state.loading.value = true;

      const result = await callTauri<string>(command, { path: state.repoPath.value });
      state.terminalOutput.value.push(`$ ${terminalCommand}\n` + (result || "(done)"));

      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
      toast.success(result || title);
      return "ok";
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push(`$ ${terminalCommand}\nError: ${e}`);

      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });

      if (isRebaseConflictMessage(e)) {
        toast.warning("Rebase still has conflicts. Resolve files, then Continue/Skip/Abort.");
        return "conflict";
      }

      toast.error(`${title} failed: ` + String(e));
      return "error";
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function rebaseContinue() {
    return runRebaseFollowUpAction("rebase_continue", "Rebase continue", "git rebase --continue");
  }

  async function rebaseSkip() {
    return runRebaseFollowUpAction("rebase_skip", "Rebase skip", "git rebase --skip");
  }

  async function rebaseAbort() {
    return runRebaseFollowUpAction("rebase_abort", "Rebase abort", "git rebase --abort");
  }

  return {
    checkoutBranch,
    createBranch,
    deleteBranch,
    renameBranch,
    mergeBranchIntoCurrent,
    rebaseBranchOnto,
    rebaseContinue,
    rebaseSkip,
    rebaseAbort,
  };
}

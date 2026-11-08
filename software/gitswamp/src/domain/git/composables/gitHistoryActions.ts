import type { CommitFileInfo, CommitInfo, RepoInfo } from "@/types";
import { useToast } from "@/shared/notifications/useToast";
import { visualiseOperationsEnabled } from "@/shared/config/visualiseOperationsPreferences";
import { publishOperationPreview, type VisualisedOperation } from "@/shared/notifications/gitOperationPreview";

import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshCommits: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  refreshBranches: () => Promise<void>;
  refreshTags: () => Promise<void>;
};

type CommitOperationResult = { message: string; sha: string };

export function createHistoryActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  function previewCommitOperation(kind: VisualisedOperation, repoPath: string, sourceSha: string, resultSha: string, target: string) {
    if (state.repoPath.value !== repoPath) return;
    publishOperationPreview({ kind, repoPath, source: sourceSha.slice(0, 7), sourceSha, target, resultSha });
  }

  async function cherryPick(sha: string) {
    if (!state.repoPath.value) return;
    if (state.repositoryOperation.value) {
      toast.warning(`Complete or abort the active ${state.repositoryOperation.value.kind} first.`);
      return;
    }
    if (state.hasConflicts.value) {
      toast.error("Cannot cherry-pick while conflicts exist. Resolve conflicts first.");
      return;
    }
    const repoPath = state.repoPath.value;
    let completedSha: string | undefined;
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading("Loading: cherry-pick in progress...");
      state.loading.value = true;
      const result = await callTauri<CommitOperationResult>("cherry_pick", { path: repoPath, sha });
      completedSha = result.sha;
      state.terminalOutput.value.push("$ git cherry-pick " + sha.substring(0, 7) + "\n" + result.message);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git cherry-pick\nError: " + e);
      toast.warning("Cherry-pick stopped: " + String(e));
    } finally {
      try {
        await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
        const info = await callTauri<RepoInfo>("get_repo_info", { path: repoPath });
        if (state.repoPath.value === repoPath) state.repoInfo.value = info;
        if (completedSha === info.head_sha && completedSha) {
          previewCommitOperation("cherry-pick", repoPath, sha, completedSha, info.current_branch);
        }
      } catch (error) {
        toast.error("Could not refresh cherry-pick state: " + String(error));
      } finally {
        state.loading.value = false;
        if (loadingToastId !== null) toast.remove(loadingToastId);
      }
    }
  }

  async function revertCommit(sha: string) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot revert while conflicts exist. Resolve conflicts first.");
      return;
    }
    const repoPath = state.repoPath.value;
    const target = state.repoInfo.value?.current_branch || "HEAD";
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading("Loading: reverting commit...");
      state.loading.value = true;
      const result = await callTauri<CommitOperationResult>("revert_commit", { path: repoPath, sha });
      state.terminalOutput.value.push("$ git revert " + sha.substring(0, 7) + "\n" + result.message);
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      previewCommitOperation("revert", repoPath, sha, result.sha, target);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git revert\nError: " + e);
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function resetToCommit(sha: string, mode: "soft" | "mixed" | "hard") {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot reset while conflicts exist. Resolve conflicts first.");
      return;
    }
    const repoPath = state.repoPath.value;
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading(`Loading: reset (${mode}) in progress...`);
      state.loading.value = true;
      const result = await callTauri<{ message: string; previousSha: string | null }>("reset_to_commit", {
        path: repoPath, sha, mode, visualise: visualiseOperationsEnabled(),
      });
      state.terminalOutput.value.push("$ git reset --" + mode + " " + sha.substring(0, 7) + "\n" + result.message);
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      const info = await callTauri<RepoInfo>("get_repo_info", { path: repoPath });
      if (state.repoPath.value === repoPath) {
        state.repoInfo.value = info;
        if (result.previousSha && info.head_sha === sha && result.previousSha !== sha) {
          publishOperationPreview({
            kind: "reset", repoPath, target: info.current_branch, mode, previousSha: result.previousSha, resultSha: sha,
          });
        }
      }
      toast.success(`Reset to ${sha.substring(0, 7)} (${mode})`);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git reset\nError: " + e);
      toast.error("Reset failed: " + String(e));
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function checkoutCommit(sha: string): Promise<boolean> {
    if (!state.repoPath.value) return false;
    if (state.hasConflicts.value) {
      toast.error("Cannot checkout commit while conflicts exist. Resolve conflicts first.");
      return false;
    }
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading("Loading: checking out commit...");
      state.loading.value = true;
      const result = await callTauri<string>("checkout_commit", { path: state.repoPath.value, sha });
      state.terminalOutput.value.push("$ git checkout " + sha.substring(0, 7) + "\n" + (result || "(done)"));
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      toast.success(`Checked out ${sha.substring(0, 7)}`);
      return true;
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git checkout\nError: " + e);
      toast.error("Checkout failed: " + String(e));
      return false;
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function createTagAt(name: string, sha: string) {
    if (!state.repoPath.value) return;
    try {
      await callTauri<string>("create_tag_at", { path: state.repoPath.value, name, sha });
      state.terminalOutput.value.push("$ git tag " + name + " " + sha.substring(0, 7) + "\n(done)");
      await Promise.all([refresh.refreshTags(), refresh.refreshCommits()]);
      toast.success(`Tag "${name}" created`);
    } catch (e) {
      state.error.value = String(e);
      toast.error("Create tag failed: " + String(e));
    }
  }

  async function deleteTag(name: string) {
    if (!state.repoPath.value) return;
    try {
      const result = await callTauri<string>("delete_tag", { path: state.repoPath.value, name });
      state.terminalOutput.value.push("$ git tag -d " + name + "\n" + (result || "(done)"));
      await Promise.all([refresh.refreshTags(), refresh.refreshCommits()]);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git tag -d " + name + "\nError: " + e);
    }
  }

  async function editCommitMessage(sha: string, newMessage: string) {
    if (!state.repoPath.value) return;
    try {
      state.error.value = null;
      state.loading.value = true;
      const result = await callTauri<string>("edit_commit_message", { path: state.repoPath.value, sha, newMessage });
      state.terminalOutput.value.push("$ git commit --amend\n" + (result || "(done)"));

      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });

      const branch = state.repoInfo.value?.current_branch || "";
      const remoteBranch = branch ? `origin/${branch}` : "";
      const headRef = branch ? `HEAD -> ${branch}` : "";

      const selectedHead = state.commits.value.find((commit: CommitInfo) =>
        branch && commit.refs.some((ref) => ref === branch || ref === remoteBranch || ref.includes(headRef)),
      ) ?? state.commits.value[0] ?? null;

      state.selectedCommit.value = selectedHead;
      state.selectedCommits.value = selectedHead ? [selectedHead] : [];
      if (selectedHead) {
        state.selectedCommitFiles.value = await callTauri<CommitFileInfo[]>("get_commit_files", {
          path: state.repoPath.value,
          sha: selectedHead.sha,
        });
      } else {
        state.selectedCommitFiles.value = [];
      }
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git commit --amend\nError: " + e);
    } finally {
      state.loading.value = false;
    }
  }

  async function createAnnotatedTag(name: string, sha: string, message: string) {
    if (!state.repoPath.value) return;
    try {
      await callTauri<string>("create_annotated_tag", { path: state.repoPath.value, name, sha, message });
      state.terminalOutput.value.push("$ git tag -a " + name + " " + sha.substring(0, 7) + "\n(done)");
      await Promise.all([refresh.refreshTags(), refresh.refreshCommits()]);
    } catch (e) {
      state.error.value = String(e);
    }
  }

  return {
    cherryPick,
    revertCommit,
    resetToCommit,
    checkoutCommit,
    createTagAt,
    deleteTag,
    editCommitMessage,
    createAnnotatedTag,
  };
}

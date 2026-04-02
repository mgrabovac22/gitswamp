import type { CommitFileInfo, CommitInfo, RepoInfo } from "@/types";
import { useToast } from "@/shared/notifications/useToast";

import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshCommits: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  refreshBranches: () => Promise<void>;
  refreshTags: () => Promise<void>;
};

export function createHistoryActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  async function cherryPick(sha: string) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot cherry-pick while conflicts exist. Resolve conflicts first.");
      return;
    }
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading("Loading: cherry-pick in progress...");
      state.loading.value = true;
      const result = await callTauri<string>("cherry_pick", { path: state.repoPath.value, sha });
      state.terminalOutput.value.push("$ git cherry-pick " + sha.substring(0, 7) + "\n" + (result || "(done)"));
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git cherry-pick\nError: " + e);
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function revertCommit(sha: string) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot revert while conflicts exist. Resolve conflicts first.");
      return;
    }
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading("Loading: reverting commit...");
      state.loading.value = true;
      const result = await callTauri<string>("revert_commit", { path: state.repoPath.value, sha });
      state.terminalOutput.value.push("$ git revert " + sha.substring(0, 7) + "\n" + (result || "(done)"));
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
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

  async function resetToCommit(sha: string, mode: string) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot reset while conflicts exist. Resolve conflicts first.");
      return;
    }
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading(`Loading: reset (${mode}) in progress...`);
      state.loading.value = true;
      const result = await callTauri<string>("reset_to_commit", { path: state.repoPath.value, sha, mode });
      state.terminalOutput.value.push("$ git reset --" + mode + " " + sha.substring(0, 7) + "\n" + (result || "(done)"));
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
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

  async function checkoutCommit(sha: string) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot checkout commit while conflicts exist. Resolve conflicts first.");
      return;
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
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git checkout\nError: " + e);
      toast.error("Checkout failed: " + String(e));
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

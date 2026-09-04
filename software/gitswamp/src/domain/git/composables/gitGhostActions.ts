import type { GhostBranchState, RepoInfo } from "@/types";
import { useToast } from "@/shared/notifications/useToast";

import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshCommits: () => Promise<void>;
  refreshBranches: () => Promise<void>;
  refreshStatus: () => Promise<void>;
};

const EMPTY_GHOST_STATE: GhostBranchState = {
  active: false,
  base_branch: "",
  ghost_branch: "",
};

export function createGhostActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  async function refreshGhostBranchState() {
    const repoPath = state.repoPath.value;
    if (!repoPath) {
      state.ghostBranchState.value = { ...EMPTY_GHOST_STATE };
      return;
    }

    try {
      const result = await callTauri<GhostBranchState>("get_ghost_branch_state", { path: repoPath });
      if (repoPath !== state.repoPath.value) return;
      state.ghostBranchState.value = result;
    } catch {
      if (repoPath !== state.repoPath.value) return;
      state.ghostBranchState.value = { ...EMPTY_GHOST_STATE };
    }
  }

  async function refreshStateAfterGhostMutation() {
    if (!state.repoPath.value) return;

    state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", {
      path: state.repoPath.value,
    });

    await Promise.all([
      refresh.refreshCommits(),
      refresh.refreshBranches(),
      refresh.refreshStatus(),
      refreshGhostBranchState(),
    ]);
  }

  async function startGhostBranch() {
    if (!state.repoPath.value) return;

    try {
      state.loading.value = true;
      const result = await callTauri<GhostBranchState>("start_ghost_branch", {
        path: state.repoPath.value,
      });
      state.ghostBranchState.value = result;
      await refreshStateAfterGhostMutation();
      toast.success(`Ghost mode enabled on ${result.ghost_branch}`);
    } catch (e) {
      state.error.value = String(e);
      toast.error("Ghost mode failed: " + String(e));
    } finally {
      state.loading.value = false;
    }
  }

  async function materializeGhostBranch(name: string) {
    if (!state.repoPath.value) return;

    try {
      state.loading.value = true;
      const message = await callTauri<string>("materialize_ghost_branch", {
        path: state.repoPath.value,
        name,
      });
      await refreshStateAfterGhostMutation();
      toast.success(message || "Ghost branch materialized.");
    } catch (e) {
      state.error.value = String(e);
      toast.error("Materialize failed: " + String(e));
    } finally {
      state.loading.value = false;
    }
  }

  async function discardGhostBranch() {
    if (!state.repoPath.value) return;

    try {
      state.loading.value = true;
      const message = await callTauri<string>("discard_ghost_branch", {
        path: state.repoPath.value,
      });
      await refreshStateAfterGhostMutation();
      toast.success(message || "Ghost branch discarded.");
    } catch (e) {
      state.error.value = String(e);
      toast.error("Discard ghost failed: " + String(e));
    } finally {
      state.loading.value = false;
    }
  }

  return {
    refreshGhostBranchState,
    startGhostBranch,
    materializeGhostBranch,
    discardGhostBranch,
  };
}

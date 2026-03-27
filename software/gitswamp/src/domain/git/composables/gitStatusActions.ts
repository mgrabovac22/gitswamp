import { useToast } from "@/shared/notifications/useToast";

import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshStatus: () => Promise<void>;
  refreshCommits: () => Promise<void>;
  refreshBranches: () => Promise<void>;
};

export function createStatusActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  function refreshStatusInBackground() {
    void refresh.refreshStatus();
  }

  async function stageFile(filePath: string) {
    if (!state.repoPath.value) return;
    try {
      await callTauri("stage_file", { path: state.repoPath.value, filePath });
      refreshStatusInBackground();
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function unstageFile(filePath: string) {
    if (!state.repoPath.value) return;
    try {
      await callTauri("unstage_file", { path: state.repoPath.value, filePath });
      refreshStatusInBackground();
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function stageAll() {
    if (!state.repoPath.value) return;
    try {
      const filePaths = state.unstagedFiles.value.map((f) => f.path);
      if (!filePaths.length) return;
      await callTauri("stage_files", { path: state.repoPath.value, filePaths });
      refreshStatusInBackground();
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function unstageAll() {
    if (!state.repoPath.value) return;
    try {
      const filePaths = state.stagedFiles.value.map((f) => f.path);
      if (!filePaths.length) return;
      await callTauri("unstage_files", { path: state.repoPath.value, filePaths });
      refreshStatusInBackground();
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function commitChanges(message: string) {
    if (!state.repoPath.value) return;
    try {
      await callTauri("create_commit", { path: state.repoPath.value, message });
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function discardFile(filePath: string) {
    if (!state.repoPath.value) return;
    try {
      await callTauri("discard_file", { path: state.repoPath.value, filePath });
      refreshStatusInBackground();
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function discardAll() {
    if (!state.repoPath.value) return;
    try {
      const filePaths = state.unstagedFiles.value.map((f) => f.path);
      if (!filePaths.length) return;
      await callTauri("discard_files", { path: state.repoPath.value, filePaths });
      refreshStatusInBackground();
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function resolveAllConflicts() {
    if (!state.repoPath.value || !state.hasConflicts.value) return;
    try {
      state.loading.value = true;
      await callTauri("resolve_all_conflicts", { path: state.repoPath.value, strategy: "ours" });
      await refresh.refreshStatus();
      toast.success("All conflicts resolved");
    } catch (e) {
      state.error.value = String(e);
      toast.error("Resolve all conflicts failed: " + String(e));
    } finally {
      state.loading.value = false;
    }
  }

  async function resolveConflictFile(filePath: string, strategy: "ours" | "theirs" | "delete") {
    if (!state.repoPath.value) return;
    try {
      await callTauri("resolve_conflict_file", { path: state.repoPath.value, filePath, strategy });
      await refresh.refreshStatus();
      const labels: Record<string, string> = {
        ours: "Kept modified version",
        theirs: "Kept base version",
        delete: "Deleted file",
      };
      toast.success(`${labels[strategy]}: ${filePath}`);
    } catch (e) {
      state.error.value = String(e);
      toast.error("Resolve conflict failed: " + String(e));
    }
  }

  function promptResolveConflict(filePath: string) {
    toast.action("warning", `Resolve conflict: ${filePath}`, [
      { label: "Keep modified", style: "primary", onClick: () => void resolveConflictFile(filePath, "ours") },
      { label: "Keep base", style: "neutral", onClick: () => void resolveConflictFile(filePath, "theirs") },
      { label: "Delete file", style: "danger", onClick: () => void resolveConflictFile(filePath, "delete") },
      { label: "Cancel", style: "neutral", onClick: () => {} },
    ], 20000);
  }

  return {
    stageFile,
    unstageFile,
    stageAll,
    unstageAll,
    commitChanges,
    discardFile,
    discardAll,
    resolveAllConflicts,
    promptResolveConflict,
  };
}

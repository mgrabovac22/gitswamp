import type { CommitInfo, FileStatusInfo } from "@/types";

import { callTauri } from "./gitCall";
import { statusHash } from "./gitHelpers";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshCommits: () => Promise<void>;
  refreshBranches: () => Promise<void>;
};

export function createWatcherActions(state: GitState, deps: RefreshDeps) {
  let watchInterval: ReturnType<typeof setInterval> | null = null;

  function startFileWatcher() {
    stopFileWatcher();
    watchInterval = setInterval(() => {
      void pollStatus();
    }, 2000);
  }

  function stopFileWatcher() {
    if (!watchInterval) return;
    clearInterval(watchInterval);
    watchInterval = null;
  }

  async function pollStatus() {
    if (!state.repoPath.value) return;
    try {
      const newStatuses = await callTauri<FileStatusInfo[]>("get_status", { path: state.repoPath.value });
      const newHash = statusHash(newStatuses);
      if (newHash === state.lastStatusHash.value) return;

      state.lastStatusHash.value = newHash;
      state.fileStatuses.value = newStatuses;

      const topCheck = await callTauri<CommitInfo[]>("get_commits", {
        path: state.repoPath.value,
        maxCount: 1,
      });

      if (topCheck.length > 0 && topCheck[0].sha !== state.commits.value[0]?.sha) {
        await deps.refreshCommits();
        await deps.refreshBranches();
      }
    } catch {
      // Ignore periodic watcher errors.
    }
  }

  return {
    startFileWatcher,
    stopFileWatcher,
  };
}

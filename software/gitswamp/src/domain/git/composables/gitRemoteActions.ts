import type { RepoInfo } from "@/types";
import { useToast } from "@/shared/notifications/useToast";

import { callTauri } from "./gitCall";
import { getOriginUrl, getTokenForUrl, isAuthenticationError, isRemoteBehindPushError } from "./gitHelpers";
import type { GitState } from "./gitState";

type RefreshDeps = {
  refreshCommits: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  refreshBranches: () => Promise<void>;
  refreshTags: () => Promise<void>;
  refreshStashes: () => Promise<void>;
};

export type PullOutcome = "success" | "worktree-dirty" | "operation-stash-retained" | "stash-restore-failed" | "error";

export function createRemoteActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  async function refreshRemoteRefs(repoPath: string, includeStatus = false, includeRepoInfo = false) {
    if (state.repoPath.value !== repoPath) return;

    try {
      const tasks: Promise<void>[] = [
        refresh.refreshBranches(),
        refresh.refreshCommits(),
        refresh.refreshTags(),
      ];

      if (includeStatus) {
        tasks.push(refresh.refreshStatus());
      }

      await Promise.all(tasks);

      if (includeRepoInfo && state.repoPath.value === repoPath) {
        state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: repoPath });
      }
    } catch {
      // Best-effort local refresh after remote operations; keep the original remote error visible.
    }
  }

  async function autoFetchAfterPush() {
    if (!state.repoPath.value) return;
    const repoPath = state.repoPath.value;
    const fetchResult = await callTauri<string>("fetch_all", {
      path: repoPath,
      token: getTokenForUrl(state, getOriginUrl(state)),
    });
    state.terminalOutput.value.push("$ git fetch --all --prune\n" + fetchResult);
    await refreshRemoteRefs(repoPath, true, true);
  }

  async function forcePushCurrentBranch() {
    if (!state.repoPath.value) return;
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading("Loading: force pushing branch...");
      state.loading.value = true;
      const forceResult = await callTauri<string>("push_force", {
        path: state.repoPath.value,
        token: getTokenForUrl(state, getOriginUrl(state)),
      });
      state.terminalOutput.value.push("$ git push --force-with-lease\n" + forceResult);
      await autoFetchAfterPush();
      toast.success("Force push completed successfully");
      state.error.value = null;
    } catch (e) {
      const errMsg = String(e);
      state.error.value = isAuthenticationError(errMsg) ? `AUTH_REQUIRED:${errMsg}` : errMsg;
      state.terminalOutput.value.push("$ git push --force-with-lease\nError: " + errMsg);
      toast.error("Force push failed: " + errMsg);
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function pull(autoStash = false): Promise<PullOutcome> {
    if (!state.repoPath.value) return "error";
    const repoPath = state.repoPath.value;
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading("Loading: pulling remote changes...");
      state.loading.value = true;
      const result = await callTauri<string>("pull", {
        path: repoPath,
        token: getTokenForUrl(state, getOriginUrl(state)),
        autoStash,
      });
      state.terminalOutput.value.push(`$ git pull${autoStash ? " --autostash" : ""}\n` + result);
      await refreshRemoteRefs(repoPath, true, true);
      toast.success(autoStash ? "Pull completed and local changes are safe" : "Pull completed successfully");
      state.error.value = null;
      return "success";
    } catch (e) {
      const errorMsg = String(e);
      state.error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
      state.terminalOutput.value.push("$ git pull\nError: " + e);
      await refreshRemoteRefs(repoPath, true, true);
      if (errorMsg.startsWith("PULL_SUCCEEDED_STASH_RESTORE_FAILED:")
        || errorMsg.startsWith("PULL_FAILED_STASH_RETAINED:")) {
        await refresh.refreshStashes().catch(() => {});
      }
      if (errorMsg.startsWith("WORKTREE_DIRTY:")) {
        toast.warning("Pull stopped because local working changes were detected.");
        return "worktree-dirty";
      } else if (errorMsg.startsWith("PULL_SUCCEEDED_STASH_RESTORE_FAILED:")) {
        return "stash-restore-failed";
      } else if (errorMsg.startsWith("PULL_FAILED_STASH_RETAINED:")) {
        return "operation-stash-retained";
      } else {
        toast.error("Pull failed: " + String(e));
        return "error";
      }
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function push() {
    if (!state.repoPath.value) return;
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading("Loading: pushing changes...");
      state.loading.value = true;
      const result = await callTauri<string>("push", {
        path: state.repoPath.value,
        token: getTokenForUrl(state, getOriginUrl(state)),
      });
      state.terminalOutput.value.push("$ git push\n" + result);
      await autoFetchAfterPush();
      toast.success("Push completed successfully");
      state.error.value = null;
    } catch (e) {
      const errorMsg = String(e);

      if (isRemoteBehindPushError(errorMsg)) {
        state.terminalOutput.value.push("$ git push\nPush rejected (non-fast-forward).");
        state.error.value = errorMsg;
        toast.action(
          "warning",
          "Push rejected (non-fast-forward). Force push this branch?",
          [
            { label: "Force Push", style: "danger", onClick: () => void forcePushCurrentBranch() },
            { label: "Cancel", style: "neutral", onClick: () => {} },
          ],
          18000,
        );
        return;
      }

      if (errorMsg.includes("No remote 'origin'") || errorMsg.includes("not found")) {
        state.error.value = "NO_ORIGIN";
        state.terminalOutput.value.push("$ git push\nError: No origin remote configured");
        toast.error("No origin remote. Select a platform to push to.");
      } else {
        state.error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
        state.terminalOutput.value.push("$ git push\nError: " + e);
        toast.error("Push failed: " + String(e));
      }
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function fetchAll() {
    if (!state.repoPath.value) return;
    const repoPath = state.repoPath.value;
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading("Loading: fetching all remotes...");
      state.loading.value = true;
      const result = await callTauri<string>("fetch_all", {
        path: repoPath,
        token: getTokenForUrl(state, getOriginUrl(state)),
      });
      state.terminalOutput.value.push("$ git fetch --all --prune\n" + result);
      await refreshRemoteRefs(repoPath);
      toast.success("Fetch completed successfully");
      state.error.value = null;
    } catch (e) {
      const errorMsg = String(e);
      state.error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
      state.terminalOutput.value.push("$ git fetch --all --prune\nError: " + e);
      await refreshRemoteRefs(repoPath);
      toast.error("Fetch failed: " + String(e));
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function backgroundFetchAll() {
    if (!state.repoPath.value) return;
    const repoPath = state.repoPath.value;

    try {
      await callTauri<string>("fetch_all", {
        path: repoPath,
        token: getTokenForUrl(state, getOriginUrl(state)),
      });

      if (state.repoPath.value !== repoPath) {
        return;
      }

      await refreshRemoteRefs(repoPath);
      state.error.value = null;
    } catch (e) {
      const errorMsg = String(e);
      state.error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
    }
  }

  async function deleteRemoteBranch(branch: string) {
    if (!state.repoPath.value) return;
    try {
      state.loading.value = true;
      const result = await callTauri<string>("delete_remote_branch", {
        path: state.repoPath.value,
        remote: "origin",
        branch,
        token: getTokenForUrl(state, getOriginUrl(state)),
      });
      state.terminalOutput.value.push("$ git push origin --delete " + branch + "\n" + (result || "(done)"));
      await Promise.all([refresh.refreshBranches(), refresh.refreshCommits()]);
    } catch (e) {
      const errorMsg = String(e);
      state.error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
      state.terminalOutput.value.push("$ git push origin --delete\nError: " + e);
    } finally {
      state.loading.value = false;
    }
  }

  async function setUpstream(branch: string, remoteBranch: string) {
    if (!state.repoPath.value) return;
    try {
      const result = await callTauri<string>("set_upstream", {
        path: state.repoPath.value,
        branch,
        remoteBranch,
      });
      state.terminalOutput.value.push("$ git branch --set-upstream-to=" + remoteBranch + " " + branch + "\n" + (result || "(done)"));
      await refresh.refreshBranches();
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function resetBranchToRemote(branch: string) {
    if (!state.repoPath.value) return;
    if (state.hasConflicts.value) {
      toast.error("Cannot reset branch while conflicts exist. Resolve conflicts first.");
      return;
    }
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading(`Loading: resetting ${branch} to origin...`);
      state.loading.value = true;
      const result = await callTauri<string>("reset_branch_to_remote", { path: state.repoPath.value, branch });
      state.terminalOutput.value.push("$ git reset --hard origin/" + branch + "\n" + (result || "(done)"));
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
    } catch (e) {
      state.error.value = String(e);
      state.terminalOutput.value.push("$ git reset --hard origin/" + branch + "\nError: " + e);
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  async function checkOriginExists() {
    if (!state.repoPath.value) return false;
    try {
      return await callTauri<boolean>("check_origin", { path: state.repoPath.value });
    } catch {
      return false;
    }
  }

  async function pushToMultiplePlatforms(platform: string, repoName: string) {
    if (!state.repoPath.value) return;
    let loadingToastId: number | null = null;
    try {
      loadingToastId = toast.loading(`Loading: pushing to ${platform}...`);
      state.loading.value = true;
      const tokenKey = platform === "gitlab-self-hosted" ? "gitlab-self" : platform;
      const backendPlatform = platform === "gitlab-self" ? "gitlab-self-hosted" : platform;
      const token = state.providerTokens.value[tokenKey];
      if (!token) {
        toast.error(`No token configured for ${platform}`);
        return;
      }

      const result = await callTauri<string>("push_to_platform", {
        path: state.repoPath.value,
        platform: backendPlatform,
        token,
        repoName,
      });

      state.terminalOutput.value.push(`$ git push ${platform}\n` + result);
      await autoFetchAfterPush();
      toast.success(`Push to ${platform} completed successfully`);
      state.error.value = null;
    } catch (e) {
      const errorMsg = String(e);
      state.error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
      state.terminalOutput.value.push(`$ git push ${platform}\nError: ` + e);
      toast.error(`Push to ${platform} failed: ` + String(e));
    } finally {
      state.loading.value = false;
      if (loadingToastId !== null) {
        toast.remove(loadingToastId);
      }
    }
  }

  return {
    pull,
    push,
    fetchAll,
    backgroundFetchAll,
    deleteRemoteBranch,
    setUpstream,
    resetBranchToRemote,
    checkOriginExists,
    pushToMultiplePlatforms,
  };
}

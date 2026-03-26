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
};

export function createRemoteActions(state: GitState, refresh: RefreshDeps, toast: ReturnType<typeof useToast>) {
  async function autoFetchAfterPush() {
    if (!state.repoPath.value) return;
    const fetchResult = await callTauri<string>("fetch_all", {
      path: state.repoPath.value,
      token: getTokenForUrl(state, getOriginUrl(state)),
    });
    state.terminalOutput.value.push("$ git fetch --all\n" + fetchResult);
    await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches(), refresh.refreshTags()]);
    state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
  }

  async function forcePushCurrentBranch() {
    if (!state.repoPath.value) return;
    try {
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
    }
  }

  async function pull() {
    if (!state.repoPath.value) return;
    try {
      state.loading.value = true;
      const result = await callTauri<string>("pull", {
        path: state.repoPath.value,
        token: getTokenForUrl(state, getOriginUrl(state)),
      });
      state.terminalOutput.value.push("$ git pull\n" + result);
      await Promise.all([refresh.refreshCommits(), refresh.refreshStatus(), refresh.refreshBranches()]);
      state.repoInfo.value = await callTauri<RepoInfo>("get_repo_info", { path: state.repoPath.value });
      toast.success("Pull completed successfully");
      state.error.value = null;
    } catch (e) {
      const errorMsg = String(e);
      state.error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
      state.terminalOutput.value.push("$ git pull\nError: " + e);
      toast.error("Pull failed: " + String(e));
    } finally {
      state.loading.value = false;
    }
  }

  async function push() {
    if (!state.repoPath.value) return;
    try {
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
    }
  }

  async function fetchAll() {
    if (!state.repoPath.value) return;
    try {
      state.loading.value = true;
      const result = await callTauri<string>("fetch_all", {
        path: state.repoPath.value,
        token: getTokenForUrl(state, getOriginUrl(state)),
      });
      state.terminalOutput.value.push("$ git fetch --all\n" + result);
      await Promise.all([refresh.refreshBranches(), refresh.refreshCommits(), refresh.refreshTags()]);
      toast.success("Fetch completed successfully");
      state.error.value = null;
    } catch (e) {
      const errorMsg = String(e);
      state.error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
      state.terminalOutput.value.push("$ git fetch --all\nError: " + e);
      toast.error("Fetch failed: " + String(e));
    } finally {
      state.loading.value = false;
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
    try {
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
    try {
      state.loading.value = true;
      const token = state.providerTokens.value[platform];
      if (!token) {
        toast.error(`No token configured for ${platform}`);
        return;
      }

      const result = await callTauri<string>("push_to_platform", {
        path: state.repoPath.value,
        platform,
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
    }
  }

  return {
    pull,
    push,
    fetchAll,
    deleteRemoteBranch,
    setUpstream,
    resetBranchToRemote,
    checkOriginExists,
    pushToMultiplePlatforms,
  };
}

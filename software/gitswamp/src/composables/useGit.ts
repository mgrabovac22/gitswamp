import { ref, computed, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "@/composables/useToast";
import type {
  CommitInfo,
  BranchInfo,
  FileStatusInfo,
  RepoInfo,
  CommitFileInfo,
  StashInfo,
  TagInfo,
  GithubRepo,
} from "@/types";

const toast = useToast();

const repoPath = ref("");
const repoInfo = ref<RepoInfo | null>(null);
const commits = shallowRef<CommitInfo[]>([]);
const branches = ref<BranchInfo[]>([]);
const fileStatuses = ref<FileStatusInfo[]>([]);
const selectedCommit = ref<CommitInfo | null>(null);
const selectedCommitFiles = ref<CommitFileInfo[]>([]);
const selectedStash = ref<StashInfo | null>(null);
const selectedStashFiles = ref<CommitFileInfo[]>([]);
const stashes = ref<StashInfo[]>([]);
const tags = ref<TagInfo[]>([]);
const loading = ref(false);
const loadingMore = ref(false);
const error = ref<string | null>(null);
const searchQuery = ref("");
const searchResults = shallowRef<CommitInfo[] | null>(null);
const terminalOutput = ref<string[]>([]);
const githubToken = ref<string | null>(null);
const providerTokens = ref<Record<string, string | null>>({});
const hasMoreCommits = ref(true);
const hasMoreSearchResults = ref(false);
const gitPath = ref("");

const PAGE_SIZE = 200;

let watchInterval: ReturnType<typeof setInterval> | null = null;
let lastStatusHash = "";
let loadMoreDebounce: ReturnType<typeof setTimeout> | null = null;

const localBranches = computed(() =>
  branches.value.filter((b) => !b.is_remote)
);
const remoteBranches = computed(() =>
  branches.value.filter((b) => b.is_remote)
);
const stagedFiles = computed(() =>
  fileStatuses.value.filter((f) => f.staged && !f.conflicted)
);
const unstagedFiles = computed(() =>
  fileStatuses.value.filter((f) => !f.staged && !f.conflicted)
);
const conflictFiles = computed(() => {
  const byPath = new Map<string, FileStatusInfo>();
  for (const f of fileStatuses.value) {
    if (!f.conflicted) continue;
    if (!byPath.has(f.path)) byPath.set(f.path, f);
  }
  return Array.from(byPath.values());
});
const hasConflicts = computed(() => conflictFiles.value.length > 0);
const currentBranch = computed(
  () => repoInfo.value?.current_branch ?? ""
);

const displayedCommits = computed(() =>
  searchResults.value !== null ? searchResults.value : commits.value
);

function statusHash(files: FileStatusInfo[]): string {
  return files.map((f) => f.path + ":" + f.status + ":" + f.staged + ":" + (!!f.conflicted)).join("|");
}

function startFileWatcher() {
  stopFileWatcher();
  watchInterval = setInterval(async () => {
    if (!repoPath.value) return;
    try {
      const newStatuses = await invoke<FileStatusInfo[]>("get_status", {
        path: repoPath.value,
      });
      const newHash = statusHash(newStatuses);
      if (newHash !== lastStatusHash) {
        lastStatusHash = newHash;
        fileStatuses.value = newStatuses;
        const topCheck = await invoke<CommitInfo[]>("get_commits", {
          path: repoPath.value,
          maxCount: 1,
        });
        if (topCheck.length > 0 && topCheck[0].sha !== commits.value[0]?.sha) {
          await refreshCommits();
          await refreshBranches();
        }
      }
    } catch {}
  }, 2000);
}

function stopFileWatcher() {
  if (watchInterval) {
    clearInterval(watchInterval);
    watchInterval = null;
  }
}

async function loadSavedToken() {
  try {
    const token = await invoke<string | null>("load_token");
    githubToken.value = token || null;
    // Also set as provider token for GitHub
    if (token) {
      providerTokens.value["github"] = token;
    }
  } catch {
    githubToken.value = null;
  }
}
loadSavedToken();

async function loadProviderTokens() {
  const providers = ["github", "gitlab", "bitbucket", "azure", "github-enterprise", "gitlab-self", "bitbucket-dc"];
  for (const p of providers) {
    try {
      const token = await invoke<string | null>("load_provider_token", { provider: p });
      providerTokens.value[p] = token || null;
    } catch {
      providerTokens.value[p] = null;
    }
  }
}
loadProviderTokens();

async function loadGitPath() {
  try {
    gitPath.value = await invoke<string>("get_git_path");
  } catch {
    gitPath.value = "not found";
  }
}
loadGitPath();

async function saveToken(token: string) {
  try {
    await invoke("save_token", { token });
    githubToken.value = token;
    // Also set as provider token for GitHub
    providerTokens.value["github"] = token;
  } catch (e) {
    error.value = String(e);
  }
}

async function deleteToken() {
  try {
    await invoke("delete_token");
    githubToken.value = null;
  } catch (e) {
    error.value = String(e);
  }
}

async function saveProviderToken(provider: string, token: string) {
  try {
    await invoke("save_provider_token", { provider, token });
    providerTokens.value[provider] = token;
  } catch (e) {
    error.value = String(e);
  }
}

async function deleteProviderToken(provider: string) {
  try {
    await invoke("delete_provider_token", { provider });
    providerTokens.value[provider] = null;
  } catch (e) {
    error.value = String(e);
  }
}

function getTokenParam(): string | null {
  return githubToken.value || null;
}

function getTokenForUrl(url?: string): string | null {
  if (!url) return getTokenParam();
  if (url.includes("gitlab.") || url.includes("/gitlab")) {
    const stored = providerTokens.value["gitlab-self"];
    if (stored && stored.includes("|")) {
      const parts = stored.split("|");
      return parts[1] || null;
    }
    return providerTokens.value["gitlab"] || null;
  }
  if (url.includes("gitlab.com")) return providerTokens.value["gitlab"] || null;
  if (url.includes("bitbucket.org")) return providerTokens.value["bitbucket"] || null;
  if (url.includes("dev.azure.com") || url.includes("visualstudio.com")) return providerTokens.value["azure"] || null;
  return getTokenParam();
}

function getOriginUrl(): string | undefined {
  return repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url;
}

function isAuthenticationError(message: string): boolean {
  const m = message.toLowerCase();
  return (
    m.includes("authentication") ||
    m.includes("auth") ||
    m.includes("permission denied") ||
    m.includes("access denied") ||
    m.includes("http 401") ||
    m.includes("http 403") ||
    m.includes("could not read username") ||
    m.includes("requires authentication") ||
    m.includes("invalid credentials")
  );
}

function isRemoteBehindPushError(message: string): boolean {
  const m = message.toLowerCase();
  return (
    m.includes("non-fast-forward") ||
    m.includes("non fast-forward") ||
    m.includes("non-fastforward") ||
    m.includes("non fastforward") ||
    m.includes("non-fastforwardable") ||
    m.includes("non fastforwardable") ||
    m.includes("cannot push non-fastforwardable reference") ||
    m.includes("cannot push non-fast-forwardable reference") ||
    m.includes("failed to push some refs") ||
    m.includes("fetch first") ||
    m.includes("tip of your current branch is behind") ||
    (m.includes("rejected") && m.includes("push"))
  );
}

async function openRepository(path: string) {
  try {
    loading.value = true;
    error.value = null;
    repoPath.value = path;
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path });
    hasMoreCommits.value = true;
    await Promise.all([refreshCommits(), refreshBranches(), refreshStatus(), refreshStashes(), refreshTags()]);
    startFileWatcher();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function refreshCommits() {
  if (!repoPath.value) return;
  try {
    const result = await invoke<CommitInfo[]>("get_commits", {
      path: repoPath.value,
      maxCount: PAGE_SIZE,
    });
    commits.value = result;
    hasMoreCommits.value = result.length >= PAGE_SIZE;
  } catch (e) {
    error.value = String(e);
  }
}

function loadMoreCommits() {
  if (loadMoreDebounce) {
    clearTimeout(loadMoreDebounce);
  }
  loadMoreDebounce = setTimeout(() => {
    _doLoadMoreCommits();
  }, 50);
}

async function _doLoadMoreCommits() {
  if (searchQuery.value && searchResults.value !== null) {
    return;
  }
  
  if (!repoPath.value || !hasMoreCommits.value || loadingMore.value) return;
  
  loadingMore.value = true;
  try {
    const currentCount = commits.value.length;
    const nextCount = currentCount + PAGE_SIZE;
    const result = await invoke<CommitInfo[]>("get_commits", {
      path: repoPath.value,
      maxCount: nextCount,
    });
    if (result.length <= currentCount) {
      hasMoreCommits.value = false;
    } else {
      commits.value = result;
      hasMoreCommits.value = result.length >= nextCount;
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loadingMore.value = false;
  }
}

async function refreshBranches() {
  if (!repoPath.value) return;
  try {
    branches.value = await invoke<BranchInfo[]>("get_branches", {
      path: repoPath.value,
    });
  } catch (e) {
    error.value = String(e);
  }
}

async function refreshStatus() {
  if (!repoPath.value) return;
  try {
    fileStatuses.value = await invoke<FileStatusInfo[]>("get_status", {
      path: repoPath.value,
    });
    lastStatusHash = statusHash(fileStatuses.value);
  } catch (e) {
    error.value = String(e);
  }
}

async function refreshStashes() {
  if (!repoPath.value) return;
  try {
    stashes.value = await invoke<StashInfo[]>("stash_list", {
      path: repoPath.value,
    });
  } catch (e) {
  }
}

async function refreshTags() {
  if (!repoPath.value) return;
  try {
    tags.value = await invoke<TagInfo[]>("get_tags", {
      path: repoPath.value,
    });
  } catch (e) {
  }
}

async function getCommitFiles(sha: string) {
  if (!repoPath.value) return;
  try {
    selectedCommitFiles.value = await invoke<CommitFileInfo[]>("get_commit_files", {
      path: repoPath.value,
      sha,
    });
  } catch (e) {
    error.value = String(e);
    selectedCommitFiles.value = [];
  }
}

async function selectStash(stash: StashInfo | null) {
  selectedStash.value = stash;
  selectedCommit.value = null;
  selectedCommitFiles.value = [];
  if (stash && repoPath.value) {
    try {
      selectedStashFiles.value = await invoke<CommitFileInfo[]>("stash_files", {
        path: repoPath.value,
        index: stash.index,
      });
    } catch (e) {
      error.value = String(e);
      selectedStashFiles.value = [];
    }
  } else {
    selectedStashFiles.value = [];
  }
}

function clearStashSelection() {
  selectedStash.value = null;
  selectedStashFiles.value = [];
}

async function stageFile(filePath: string) {
  if (!repoPath.value) return;
  try {
    await invoke("stage_file", { path: repoPath.value, filePath });
    await refreshStatus();
  } catch (e) {
    error.value = String(e);
  }
}

async function unstageFile(filePath: string) {
  if (!repoPath.value) return;
  try {
    await invoke("unstage_file", { path: repoPath.value, filePath });
    await refreshStatus();
  } catch (e) {
    error.value = String(e);
  }
}

async function stageAll() {
  if (!repoPath.value) return;
  try {
    for (const f of unstagedFiles.value) {
      await invoke("stage_file", { path: repoPath.value, filePath: f.path });
    }
    await refreshStatus();
  } catch (e) {
    error.value = String(e);
  }
}

async function unstageAll() {
  if (!repoPath.value) return;
  try {
    for (const f of stagedFiles.value) {
      await invoke("unstage_file", { path: repoPath.value, filePath: f.path });
    }
    await refreshStatus();
  } catch (e) {
    error.value = String(e);
  }
}

async function commitChanges(message: string) {
  if (!repoPath.value) return;
  try {
    await invoke("create_commit", { path: repoPath.value, message });
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
  } catch (e) {
    error.value = String(e);
  }
}

async function checkoutBranch(branchName: string) {
  if (!repoPath.value) return;
  if (hasConflicts.value) {
    toast.error("Cannot checkout branch while conflicts exist. Resolve conflicts first.");
    return;
  }
  try {
    loading.value = true;
    await invoke("checkout_branch", { path: repoPath.value, branchName });
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", {
      path: repoPath.value,
    });
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
    toast.success(`Checked out branch "${branchName}"`);
  } catch (e) {
    error.value = String(e);
    toast.error("Checkout branch failed: " + String(e));
  } finally {
    loading.value = false;
  }
}

async function createBranch(name: string, startPoint?: string) {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    await invoke("create_branch", {
      path: repoPath.value,
      name,
      startPoint: startPoint || null,
    });
    await Promise.all([refreshBranches(), refreshCommits()]);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function deleteBranch(name: string) {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    await invoke("delete_branch", { path: repoPath.value, name });
    await Promise.all([refreshBranches(), refreshCommits()]);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function renameBranch(oldName: string, newName: string) {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const result = await invoke<string>("rename_branch", { path: repoPath.value, oldName, newName });
    terminalOutput.value.push("$ git branch -m " + oldName + " " + newName + "\n" + (result || "(done)"));
    await Promise.all([refreshBranches(), refreshCommits()]);
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git branch -m\nError: " + e);
  } finally {
    loading.value = false;
  }
}

async function deleteRemoteBranch(branch: string) {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const result = await invoke<string>("delete_remote_branch", { path: repoPath.value, remote: "origin", branch });
    terminalOutput.value.push("$ git push origin --delete " + branch + "\n" + (result || "(done)"));
    await Promise.all([refreshBranches(), refreshCommits()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git push origin --delete\nError: " + e);
  } finally {
    loading.value = false;
  }
}

async function setUpstream(branch: string, remoteBranch: string) {
  if (!repoPath.value) return;
  try {
    const result = await invoke<string>("set_upstream", { path: repoPath.value, branch, remoteBranch });
    terminalOutput.value.push("$ git branch --set-upstream-to=" + remoteBranch + " " + branch + "\n" + (result || "(done)"));
    await refreshBranches();
  } catch (e) {
    error.value = String(e);
  }
}

async function editCommitMessage(sha: string, newMessage: string) {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const result = await invoke<string>("edit_commit_message", { path: repoPath.value, sha, newMessage });
    terminalOutput.value.push("$ git commit --amend\n" + (result || "(done)"));
    await refreshCommits();
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git commit --amend\nError: " + e);
  } finally {
    loading.value = false;
  }
}

async function createAnnotatedTag(name: string, sha: string, message: string) {
  if (!repoPath.value) return;
  try {
    await invoke<string>("create_annotated_tag", { path: repoPath.value, name, sha, message });
    terminalOutput.value.push("$ git tag -a " + name + " " + sha.substring(0, 7) + "\n(done)");
    await Promise.all([refreshTags(), refreshCommits()]);
  } catch (e) {
    error.value = String(e);
  }
}

async function resetBranchToRemote(branch: string) {
  if (!repoPath.value) return;
  if (hasConflicts.value) {
    toast.error("Cannot reset branch while conflicts exist. Resolve conflicts first.");
    return;
  }
  try {
    loading.value = true;
    const result = await invoke<string>("reset_branch_to_remote", { path: repoPath.value, branch });
    terminalOutput.value.push("$ git reset --hard origin/" + branch + "\n" + (result || "(done)"));
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git reset --hard origin/" + branch + "\nError: " + e);
  } finally {
    loading.value = false;
  }
}

async function pull() {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const result = await invoke<string>("pull", {
      path: repoPath.value,
      token: getTokenForUrl(getOriginUrl()),
    });
    terminalOutput.value.push("$ git pull\n" + result);
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
    toast.success("Pull completed successfully");
    error.value = null;
  } catch (e) {
    const errorMsg = String(e);
    error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
    terminalOutput.value.push("$ git pull\nError: " + e);
    toast.error("Pull failed: " + String(e));
  } finally {
    loading.value = false;
  }
}

async function autoFetchAfterPush() {
  if (!repoPath.value) return;
  const fetchResult = await invoke<string>("fetch_all", {
    path: repoPath.value,
    token: getTokenForUrl(getOriginUrl()),
  });
  terminalOutput.value.push("$ git fetch --all\n" + fetchResult);
  await Promise.all([refreshCommits(), refreshStatus(), refreshBranches(), refreshTags()]);
  repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
}

async function forcePushCurrentBranch() {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const forceResult = await invoke<string>("push_force", {
      path: repoPath.value,
      token: getTokenForUrl(getOriginUrl()),
    });
    terminalOutput.value.push("$ git push --force-with-lease\n" + forceResult);
    await autoFetchAfterPush();
    toast.success("Force push completed successfully");
    error.value = null;
  } catch (e) {
    const errMsg = String(e);
    error.value = isAuthenticationError(errMsg) ? `AUTH_REQUIRED:${errMsg}` : errMsg;
    terminalOutput.value.push("$ git push --force-with-lease\nError: " + errMsg);
    toast.error("Force push failed: " + errMsg);
  } finally {
    loading.value = false;
  }
}

async function push() {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const token = getTokenForUrl(getOriginUrl());
    const result = await invoke<string>("push", {
      path: repoPath.value,
      token,
    });
    terminalOutput.value.push("$ git push\n" + result);
    await autoFetchAfterPush();
    toast.success("Push completed successfully");
    error.value = null;
  } catch (e) {
    const errorMsg = String(e);

    if (isRemoteBehindPushError(errorMsg)) {
      terminalOutput.value.push("$ git push\nPush rejected (non-fast-forward).");
      error.value = errorMsg;
      toast.action(
        "warning",
        "Push rejected (non-fast-forward). Force push this branch?",
        [
          {
            label: "Force Push",
            style: "danger",
            onClick: async () => {
              await forcePushCurrentBranch();
            },
          },
          {
            label: "Cancel",
            style: "neutral",
            onClick: () => {},
          },
        ],
        18000
      );
      return;
    }

    // Check if error is because no origin exists
    if (errorMsg.includes("No remote 'origin'") || errorMsg.includes("not found")) {
      // Emit event to show platform selection dialog
      // This will be handled by the parent component
      error.value = "NO_ORIGIN";
      terminalOutput.value.push("$ git push\nError: No origin remote configured");
      toast.error("No origin remote. Select a platform to push to.");
    } else {
      error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
      terminalOutput.value.push("$ git push\nError: " + e);
      toast.error("Push failed: " + String(e));
    }
  } finally {
    loading.value = false;
  }
}

async function checkOriginExists() {
  if (!repoPath.value) return false;
  try {
    return await invoke<boolean>("check_origin", {
      path: repoPath.value,
    });
  } catch {
    return false;
  }
}

async function pushToMultiplePlatforms(platform: string, repoName: string) {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const providerToken = providerTokens.value[platform];
    if (!providerToken) {
      toast.error(`No token configured for ${platform}`);
      return;
    }
    const result = await invoke<string>("push_to_platform", {
      path: repoPath.value,
      platform: platform,
      providerToken: providerToken,
      repoName: repoName,
    });
    terminalOutput.value.push(`$ git push ${platform}\n` + result);
    await autoFetchAfterPush();
    toast.success(`Push to ${platform} completed successfully`);
    error.value = null;
  } catch (e) {
    const errorMsg = String(e);
    error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
    terminalOutput.value.push(`$ git push ${platform}\nError: ` + e);
    toast.error(`Push to ${platform} failed: ` + String(e));
  } finally {
    loading.value = false;
  }
}

async function fetchAll() {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const result = await invoke<string>("fetch_all", {
      path: repoPath.value,
      token: getTokenForUrl(getOriginUrl()),
    });
    terminalOutput.value.push("$ git fetch --all\n" + result);
    await Promise.all([refreshBranches(), refreshCommits(), refreshTags()]);
    toast.success("Fetch completed successfully");
    error.value = null;
  } catch (e) {
    const errorMsg = String(e);
    error.value = isAuthenticationError(errorMsg) ? `AUTH_REQUIRED:${errorMsg}` : errorMsg;
    terminalOutput.value.push("$ git fetch --all\nError: " + e);
    toast.error("Fetch failed: " + String(e));
  } finally {
    loading.value = false;
  }
}

async function refreshAll() {
  if (!repoPath.value) return;
  loading.value = true;
  try {
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", {
      path: repoPath.value,
    });
    await Promise.all([refreshCommits(), refreshBranches(), refreshStatus(), refreshStashes(), refreshTags()]);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function stashPush(message?: string) {
  if (!repoPath.value) return;
  if (hasConflicts.value) {
    toast.error("Cannot stash while conflicts exist. Resolve conflicts first.");
    return;
  }
  try {
    const result = await invoke<string>("stash_push", {
      path: repoPath.value,
      message: message || null,
    });
    terminalOutput.value.push("$ git stash push" + (message ? ' -m "' + message + '"' : "") + "\n" + result);
    await Promise.all([refreshStatus(), refreshStashes(), refreshCommits()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git stash push\nError: " + e);
  }
}

async function stashPop(index: number = 0) {
  if (!repoPath.value) return;
  if (hasConflicts.value) {
    toast.error("Cannot pop stash while conflicts exist. Resolve conflicts first.");
    return;
  }
  try {
    const result = await invoke<string>("stash_pop", {
      path: repoPath.value,
      index,
    });
    terminalOutput.value.push("$ git stash pop stash@{" + index + "}\n" + result);
    await Promise.all([refreshStatus(), refreshStashes(), refreshCommits()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git stash pop\nError: " + e);
  }
}

async function stashApply(index: number = 0) {
  if (!repoPath.value) return;
  if (hasConflicts.value) {
    toast.error("Cannot apply stash while conflicts exist. Resolve conflicts first.");
    return;
  }
  try {
    const result = await invoke<string>("stash_apply", {
      path: repoPath.value,
      index,
    });
    terminalOutput.value.push("$ git stash apply stash@{" + index + "}\n" + result);
    await Promise.all([refreshStatus(), refreshStashes()]);
  } catch (e) {
    error.value = String(e);
  }
}

async function stashDrop(index: number = 0) {
  if (!repoPath.value) return;
  try {
    const result = await invoke<string>("stash_drop", {
      path: repoPath.value,
      index,
    });
    terminalOutput.value.push("$ git stash drop stash@{" + index + "}\n" + result);
    await refreshStashes();
  } catch (e) {
    error.value = String(e);
  }
}

async function cloneRepo(url: string, path: string, shallow: boolean = false, token?: string | null): Promise<string | null> {
  try {
    loading.value = true;
    error.value = null;
    const t = token || getTokenForUrl(url);
    const clonedPath = await invoke<string>("clone_repo", { url, path, shallow, token: t });
    return clonedPath;
  } catch (e) {
    error.value = String(e);
    return null;
  } finally {
    loading.value = false;
  }
}

async function initRepo(path: string, branchName?: string) {
  try {
    loading.value = true;
    error.value = null;
    await invoke<string>("init_repo", { path, branchName });
    return true;
  } catch (e) {
    error.value = String(e);
    return false;
  } finally {
    loading.value = false;
  }
}

async function searchCommits(query: string) {
  if (!repoPath.value) return;
  if (!query.trim()) {
    searchQuery.value = "";
    searchResults.value = null;
    hasMoreSearchResults.value = false;
    return;
  }
  try {
    searchQuery.value = query;
    const result = await invoke<CommitInfo[]>("search_commits", {
      path: repoPath.value,
      query,
      maxCount: 2000,
    });
    searchResults.value = result;
    hasMoreSearchResults.value = false;
  } catch (e) {
    error.value = String(e);
  }
}

function clearSearch() {
  searchQuery.value = "";
  searchResults.value = null;
  hasMoreSearchResults.value = false;
}

async function runTerminalCommand(command: string) {
  if (!repoPath.value) return;
  const args = command.trim().split(/\s+/);
  if (args[0] === "git") args.shift();
  try {
    const result = await invoke<string>("run_git_command", {
      path: repoPath.value,
      args,
    });
    terminalOutput.value.push("$ git " + args.join(" ") + "\n" + (result || "(done)"));
  } catch (e) {
    terminalOutput.value.push("$ git " + args.join(" ") + "\nError: " + e);
  }
}

async function discardFile(filePath: string) {
  if (!repoPath.value) return;
  try {
    await invoke("discard_file", { path: repoPath.value, filePath });
    await refreshStatus();
  } catch (e) {
    error.value = String(e);
  }
}

async function discardAll() {
  if (!repoPath.value) return;
  try {
    for (const f of unstagedFiles.value) {
      await invoke("discard_file", { path: repoPath.value, filePath: f.path });
    }
    await refreshStatus();
  } catch (e) {
    error.value = String(e);
  }
}

async function resolveAllConflicts() {
  if (!repoPath.value) return;
  if (!hasConflicts.value) return;
  try {
    loading.value = true;
    await invoke("resolve_all_conflicts", { path: repoPath.value, strategy: "ours" });
    await refreshStatus();
    toast.success("All conflicts resolved");
  } catch (e) {
    error.value = String(e);
    toast.error("Resolve all conflicts failed: " + String(e));
  } finally {
    loading.value = false;
  }
}

async function resolveConflictFile(filePath: string, strategy: "ours" | "theirs" | "delete") {
  if (!repoPath.value) return;
  try {
    await invoke("resolve_conflict_file", { path: repoPath.value, filePath, strategy });
    await refreshStatus();
    const labels: Record<string, string> = {
      ours: "Kept modified version",
      theirs: "Kept base version",
      delete: "Deleted file",
    };
    toast.success(`${labels[strategy]}: ${filePath}`);
  } catch (e) {
    error.value = String(e);
    toast.error("Resolve conflict failed: " + String(e));
  }
}

function promptResolveConflict(filePath: string) {
  toast.action("warning", `Resolve conflict: ${filePath}`, [
    {
      label: "Keep modified",
      style: "primary",
      onClick: () => resolveConflictFile(filePath, "ours"),
    },
    {
      label: "Keep base",
      style: "neutral",
      onClick: () => resolveConflictFile(filePath, "theirs"),
    },
    {
      label: "Delete file",
      style: "danger",
      onClick: () => resolveConflictFile(filePath, "delete"),
    },
    {
      label: "Cancel",
      style: "neutral",
      onClick: () => {},
    },
  ], 20000);
}

async function cherryPick(sha: string) {
  if (!repoPath.value) return;
  if (hasConflicts.value) {
    toast.error("Cannot cherry-pick while conflicts exist. Resolve conflicts first.");
    return;
  }
  try {
    loading.value = true;
    const result = await invoke<string>("cherry_pick", { path: repoPath.value, sha });
    terminalOutput.value.push("$ git cherry-pick " + sha.substring(0, 7) + "\n" + (result || "(done)"));
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git cherry-pick\nError: " + e);
  } finally {
    loading.value = false;
  }
}

async function revertCommit(sha: string) {
  if (!repoPath.value) return;
  if (hasConflicts.value) {
    toast.error("Cannot revert while conflicts exist. Resolve conflicts first.");
    return;
  }
  try {
    loading.value = true;
    const result = await invoke<string>("revert_commit", { path: repoPath.value, sha });
    terminalOutput.value.push("$ git revert " + sha.substring(0, 7) + "\n" + (result || "(done)"));
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git revert\nError: " + e);
  } finally {
    loading.value = false;
  }
}

async function resetToCommit(sha: string, mode: string) {
  if (!repoPath.value) return;
  if (hasConflicts.value) {
    toast.error("Cannot reset while conflicts exist. Resolve conflicts first.");
    return;
  }
  try {
    loading.value = true;
    const result = await invoke<string>("reset_to_commit", { path: repoPath.value, sha, mode });
    terminalOutput.value.push("$ git reset --" + mode + " " + sha.substring(0, 7) + "\n" + (result || "(done)"));
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
    toast.success(`Reset to ${sha.substring(0, 7)} (${mode})`);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git reset\nError: " + e);
    toast.error("Reset failed: " + String(e));
  } finally {
    loading.value = false;
  }
}

async function checkoutCommit(sha: string) {
  if (!repoPath.value) return;
  if (hasConflicts.value) {
    toast.error("Cannot checkout commit while conflicts exist. Resolve conflicts first.");
    return;
  }
  try {
    loading.value = true;
    const result = await invoke<string>("checkout_commit", { path: repoPath.value, sha });
    terminalOutput.value.push("$ git checkout " + sha.substring(0, 7) + "\n" + (result || "(done)"));
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
    toast.success(`Checked out ${sha.substring(0, 7)}`);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git checkout\nError: " + e);
    toast.error("Checkout failed: " + String(e));
  } finally {
    loading.value = false;
  }
}

async function createTagAt(name: string, sha: string) {
  if (!repoPath.value) return;
  try {
    await invoke<string>("create_tag_at", { path: repoPath.value, name, sha });
    terminalOutput.value.push("$ git tag " + name + " " + sha.substring(0, 7) + "\n(done)");
    await Promise.all([refreshTags(), refreshCommits()]);
    toast.success(`Tag "${name}" created`);
  } catch (e) {
    error.value = String(e);
    toast.error("Create tag failed: " + String(e));
  }
}

async function deleteTag(name: string) {
  if (!repoPath.value) return;
  try {
    const result = await invoke<string>("delete_tag", { path: repoPath.value, name });
    terminalOutput.value.push("$ git tag -d " + name + "\n" + (result || "(done)"));
    await Promise.all([refreshTags(), refreshCommits()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git tag -d " + name + "\nError: " + e);
  }
}

async function mergeBranchIntoCurrent(sourceBranch: string, sourceRemote = false, targetBranch?: string) {
  if (!repoPath.value) return;
  const current = repoInfo.value?.current_branch || targetBranch || "";
  if (!current) {
    toast.error("No active branch to merge into.");
    return;
  }
  const sourceRef = sourceRemote ? `origin/${sourceBranch}` : sourceBranch;
  try {
    loading.value = true;
    const result = await invoke<string>("run_git_command", {
      path: repoPath.value,
      args: ["merge", sourceRef],
    });
    terminalOutput.value.push(`$ git merge ${sourceRef}\n` + (result || "(done)"));
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
    toast.success(`Merged ${sourceRef} into ${current}`);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push(`$ git merge ${sourceRef}\nError: ${e}`);
    toast.error("Merge failed: " + String(e));
  } finally {
    loading.value = false;
  }
}

async function searchGithubRepos(query: string): Promise<GithubRepo[]> {
  if (!githubToken.value) {
    error.value = "No GitHub token configured. Go to Settings to add one.";
    return [];
  }
  try {
    return await invoke<GithubRepo[]>("search_github_repos", {
      token: githubToken.value,
      query,
    });
  } catch (e) {
    error.value = String(e);
    return [];
  }
}

export function useGit() {
  return {
    repoPath,
    repoInfo,
    commits,
    branches,
    localBranches,
    remoteBranches,
    fileStatuses,
    stagedFiles,
    unstagedFiles,
    conflictFiles,
    hasConflicts,
    selectedCommit,
    selectedCommitFiles,
    selectedStash,
    selectedStashFiles,
    stashes,
    tags,
    currentBranch,
    loading,
    loadingMore,
    error,
    searchQuery,
    searchResults,
    displayedCommits,
    terminalOutput,
    githubToken,
    providerTokens,
    hasMoreCommits,
    hasMoreSearchResults,
    gitPath,
    openRepository,
    refreshCommits,
    refreshBranches,
    refreshStatus,
    refreshStashes,
    refreshTags,
    refreshAll,
    loadMoreCommits,
    getCommitFiles,
    stageFile,
    unstageFile,
    stageAll,
    unstageAll,
    commitChanges,
    checkoutBranch,
    createBranch,
    deleteBranch,
    renameBranch,
    deleteRemoteBranch,
    setUpstream,
    editCommitMessage,
    createAnnotatedTag,
    resetBranchToRemote,
    pull,
    push,
    pushToMultiplePlatforms,
    checkOriginExists,
    fetchAll,
    stashPush,
    stashPop,
    stashApply,
    stashDrop,
    selectStash,
    clearStashSelection,
    cloneRepo,
    initRepo,
    searchCommits,
    clearSearch,
    runTerminalCommand,
    discardFile,
    discardAll,
    resolveAllConflicts,
    promptResolveConflict,
    saveToken,
    deleteToken,
    saveProviderToken,
    deleteProviderToken,
    startFileWatcher,
    stopFileWatcher,
    cherryPick,
    revertCommit,
    resetToCommit,
    checkoutCommit,
    createTagAt,
    deleteTag,
    mergeBranchIntoCurrent,
    searchGithubRepos,
  };
}


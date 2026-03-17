import { ref, computed, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
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

const repoPath = ref("");
const repoInfo = ref<RepoInfo | null>(null);
const commits = shallowRef<CommitInfo[]>([]);
const branches = ref<BranchInfo[]>([]);
const fileStatuses = ref<FileStatusInfo[]>([]);
const selectedCommit = ref<CommitInfo | null>(null);
const selectedCommitFiles = ref<CommitFileInfo[]>([]);
const stashes = ref<StashInfo[]>([]);
const tags = ref<TagInfo[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const searchQuery = ref("");
const searchResults = shallowRef<CommitInfo[] | null>(null);
const terminalOutput = ref<string[]>([]);
const githubToken = ref<string | null>(null);
const providerTokens = ref<Record<string, string | null>>({});
const hasMoreCommits = ref(true);
const gitPath = ref("");

const PAGE_SIZE = 200;

let watchInterval: ReturnType<typeof setInterval> | null = null;
let lastStatusHash = "";

const localBranches = computed(() =>
  branches.value.filter((b) => !b.is_remote)
);
const remoteBranches = computed(() =>
  branches.value.filter((b) => b.is_remote)
);
const stagedFiles = computed(() =>
  fileStatuses.value.filter((f) => f.staged)
);
const unstagedFiles = computed(() =>
  fileStatuses.value.filter((f) => !f.staged)
);
const currentBranch = computed(
  () => repoInfo.value?.current_branch ?? ""
);

const displayedCommits = computed(() =>
  searchResults.value !== null ? searchResults.value : commits.value
);

function statusHash(files: FileStatusInfo[]): string {
  return files.map((f) => f.path + ":" + f.status + ":" + f.staged).join("|");
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
        // Check if top commit changed (new commits from external tools)
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

// Load saved token on init
async function loadSavedToken() {
  try {
    const token = await invoke<string | null>("load_token");
    githubToken.value = token || null;
  } catch {
    githubToken.value = null;
  }
}
loadSavedToken();

// Load provider tokens on init
async function loadProviderTokens() {
  const providers = ["gitlab", "bitbucket", "azure", "github-enterprise", "gitlab-self", "bitbucket-dc"];
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

// Load git path on init
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
  if (url.includes("gitlab.com")) return providerTokens.value["gitlab"] || null;
  if (url.includes("bitbucket.org")) return providerTokens.value["bitbucket"] || null;
  if (url.includes("dev.azure.com") || url.includes("visualstudio.com")) return providerTokens.value["azure"] || null;
  return getTokenParam();
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

async function loadMoreCommits() {
  if (!repoPath.value || !hasMoreCommits.value || loading.value) return;
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
    // Stash might not be available
  }
}

async function refreshTags() {
  if (!repoPath.value) return;
  try {
    tags.value = await invoke<TagInfo[]>("get_tags", {
      path: repoPath.value,
    });
  } catch (e) {
    // Tags might fail on empty repo
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
  try {
    loading.value = true;
    await invoke("checkout_branch", { path: repoPath.value, branchName });
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", {
      path: repoPath.value,
    });
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
  } catch (e) {
    error.value = String(e);
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
      token: getTokenParam(),
    });
    terminalOutput.value.push("$ git pull\n" + result);
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git pull\nError: " + e);
  } finally {
    loading.value = false;
  }
}

async function push() {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const result = await invoke<string>("push", {
      path: repoPath.value,
      token: getTokenParam(),
    });
    terminalOutput.value.push("$ git push\n" + result);
    await Promise.all([refreshCommits(), refreshBranches()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git push\nError: " + e);
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
      token: getTokenParam(),
    });
    terminalOutput.value.push("$ git fetch --all\n" + result);
    await Promise.all([refreshBranches(), refreshCommits(), refreshTags()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git fetch --all\nError: " + e);
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
    return;
  }
  try {
    searchQuery.value = query;
    searchResults.value = await invoke<CommitInfo[]>("search_commits", {
      path: repoPath.value,
      query,
      maxCount: 2000,
    });
  } catch (e) {
    error.value = String(e);
  }
}

function clearSearch() {
  searchQuery.value = "";
  searchResults.value = null;
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

async function cherryPick(sha: string) {
  if (!repoPath.value) return;
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
  try {
    loading.value = true;
    const result = await invoke<string>("reset_to_commit", { path: repoPath.value, sha, mode });
    terminalOutput.value.push("$ git reset --" + mode + " " + sha.substring(0, 7) + "\n" + (result || "(done)"));
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git reset\nError: " + e);
  } finally {
    loading.value = false;
  }
}

async function checkoutCommit(sha: string) {
  if (!repoPath.value) return;
  try {
    loading.value = true;
    const result = await invoke<string>("checkout_commit", { path: repoPath.value, sha });
    terminalOutput.value.push("$ git checkout " + sha.substring(0, 7) + "\n" + (result || "(done)"));
    repoInfo.value = await invoke<RepoInfo>("get_repo_info", { path: repoPath.value });
    await Promise.all([refreshCommits(), refreshStatus(), refreshBranches()]);
  } catch (e) {
    error.value = String(e);
    terminalOutput.value.push("$ git checkout\nError: " + e);
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
  } catch (e) {
    error.value = String(e);
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
    selectedCommit,
    selectedCommitFiles,
    stashes,
    tags,
    currentBranch,
    loading,
    error,
    searchQuery,
    searchResults,
    displayedCommits,
    terminalOutput,
    githubToken,
    providerTokens,
    hasMoreCommits,
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
    fetchAll,
    stashPush,
    stashPop,
    stashApply,
    stashDrop,
    cloneRepo,
    initRepo,
    searchCommits,
    clearSearch,
    runTerminalCommand,
    discardFile,
    discardAll,
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
    searchGithubRepos,
  };
}

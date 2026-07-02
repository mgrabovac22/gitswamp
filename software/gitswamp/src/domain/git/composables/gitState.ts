import { computed, ref, shallowRef } from "vue";

import type {
  BranchInfo,
  CommitFileInfo,
  CommitInfo,
  FileStatusInfo,
  GhostBranchState,
  RepoInfo,
  StashInfo,
  TagInfo,
} from "@/types";

export const PAGE_SIZE = 200;

export function createGitState() {
  const repoPath = ref("");
  const repoInfo = ref<RepoInfo | null>(null);
  const commits = shallowRef<CommitInfo[]>([]);
  const branches = shallowRef<BranchInfo[]>([]);
  const fileStatuses = shallowRef<FileStatusInfo[]>([]);
  const selectedCommit = ref<CommitInfo | null>(null);
  const selectedCommits = ref<CommitInfo[]>([]);
  const selectedCommitFiles = shallowRef<CommitFileInfo[]>([]);
  const selectedStash = ref<StashInfo | null>(null);
  const selectedStashFiles = shallowRef<CommitFileInfo[]>([]);
  const stashes = shallowRef<StashInfo[]>([]);
  const tags = shallowRef<TagInfo[]>([]);
  const loading = ref(false);
  const loadingMore = ref(false);
  const commitWaveLoading = ref(false);
  const error = ref<string | null>(null);
  const searchQuery = ref("");
  const searchResults = shallowRef<CommitInfo[] | null>(null);
  const terminalOutput = ref<string[]>([]);
  const githubToken = ref<string | null>(null);
  const providerTokens = ref<Record<string, string | null>>({});
  const hasMoreCommits = ref(true);
  const hasMoreSearchResults = ref(false);
  const gitPath = ref("");
  const ghostBranchState = ref<GhostBranchState>({
    active: false,
    base_branch: "",
    ghost_branch: "",
  });
  const lastStatusHash = ref("");

  const localBranches = computed(() => branches.value.filter((b) => !b.is_remote));
  const remoteBranches = computed(() => branches.value.filter((b) => b.is_remote));
  const stagedFiles = computed(() => fileStatuses.value.filter((f) => f.staged && !f.conflicted));
  const unstagedFiles = computed(() => fileStatuses.value.filter((f) => !f.staged && !f.conflicted));
  const conflictFiles = computed(() => {
    const byPath = new Map<string, FileStatusInfo>();
    for (const f of fileStatuses.value) {
      if (!f.conflicted) continue;
      if (!byPath.has(f.path)) byPath.set(f.path, f);
    }
    return Array.from(byPath.values());
  });
  const hasConflicts = computed(() => conflictFiles.value.length > 0);
  const currentBranch = computed(() => repoInfo.value?.current_branch ?? "");
  const displayedCommits = computed(() => commits.value);

  return {
    repoPath,
    repoInfo,
    commits,
    branches,
    fileStatuses,
    selectedCommit,
    selectedCommits,
    selectedCommitFiles,
    selectedStash,
    selectedStashFiles,
    stashes,
    tags,
    loading,
    loadingMore,
    commitWaveLoading,
    error,
    searchQuery,
    searchResults,
    terminalOutput,
    githubToken,
    providerTokens,
    hasMoreCommits,
    hasMoreSearchResults,
    gitPath,
    ghostBranchState,
    lastStatusHash,
    localBranches,
    remoteBranches,
    stagedFiles,
    unstagedFiles,
    conflictFiles,
    hasConflicts,
    currentBranch,
    displayedCommits,
  };
}

export type GitState = ReturnType<typeof createGitState>;

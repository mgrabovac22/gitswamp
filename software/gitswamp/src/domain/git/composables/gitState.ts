import { computed, ref, shallowRef } from "vue";

import type {
  BranchInfo,
  CommitFileInfo,
  CommitInfo,
  FileStatusInfo,
  RepoInfo,
  StashInfo,
  TagInfo,
} from "@/types";

export const PAGE_SIZE = 200;

export function createGitState() {
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
    selectedCommitFiles,
    selectedStash,
    selectedStashFiles,
    stashes,
    tags,
    loading,
    loadingMore,
    error,
    searchQuery,
    searchResults,
    terminalOutput,
    githubToken,
    providerTokens,
    hasMoreCommits,
    hasMoreSearchResults,
    gitPath,
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

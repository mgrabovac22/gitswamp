import { computed, ref, type ComputedRef, type Ref } from "vue";
import { useToast } from "@/shared/notifications/useToast";
import type { CommitInfo } from "@/types";

export type ManualBisectPhase = "select-good" | "testing" | "complete";
export type ManualBisectResult = "good" | "bad";

export interface ManualBisectCommit {
  sha: string;
  shortSha: string;
  message: string;
  timeAgo: string;
}

export interface ManualBisectSession {
  phase: ManualBisectPhase;
  badSha: string;
  originalBranch: string;
  range: ManualBisectCommit[];
  lowIndex: number;
  highIndex: number;
  currentIndex: number | null;
  currentSha: string | null;
  culpritSha: string | null;
  step: number;
  tested: { sha: string; result: ManualBisectResult }[];
}

export interface ManualBisectDetailsState {
  phase: ManualBisectPhase;
  badSha: string;
  goodSha: string | null;
  currentSha: string | null;
  culpritSha: string | null;
}

interface UseManualBisectOptions {
  commits: Ref<CommitInfo[]>;
  displayedCommits: Ref<CommitInfo[]>;
  currentBranch: Ref<string>;
  hasWorkingChanges: ComputedRef<boolean>;
  hasConflicts: ComputedRef<boolean>;
  gitError: Ref<string | null>;
  checkoutCommit: (sha: string) => Promise<void>;
  checkoutBranch: (branch: string) => Promise<void>;
  ensureCommitLoaded?: (sha: string) => Promise<boolean>;
  selectCommit: (commit: CommitInfo) => void;
}

function commitSubjectLine(message: string): string {
  return message.split(/\r?\n/).find((line) => line.trim().length > 0)?.trim() || "No commit message";
}

function shortSha(sha: string): string {
  return sha.slice(0, 7);
}

function toManualCommit(commit: CommitInfo): ManualBisectCommit {
  return {
    sha: commit.sha,
    shortSha: commit.short_sha || shortSha(commit.sha),
    message: commitSubjectLine(commit.message),
    timeAgo: commit.time_ago || "",
  };
}

export function useManualBisect(options: UseManualBisectOptions) {
  const toast = useToast();
  const session = ref<ManualBisectSession | null>(null);
  const busy = ref(false);

  function findLoadedCommit(sha: string): CommitInfo | null {
    return options.commits.value.find((commit) => commit.sha === sha)
      || options.displayedCommits.value.find((commit) => commit.sha === sha)
      || null;
  }

  function findLoadedCommitIndex(sha: string): number {
    return options.commits.value.findIndex((commit) => commit.sha === sha);
  }

  function commitSummaryFromLoadedSha(sha: string): ManualBisectCommit | null {
    const commit = findLoadedCommit(sha);
    return commit ? toManualCommit(commit) : null;
  }

  function selectCommitIfLoaded(sha: string) {
    const commit = findLoadedCommit(sha);
    if (commit) {
      options.selectCommit(commit);
    }
  }

  async function focusCommitAfterCheckout(sha: string) {
    await options.ensureCommitLoaded?.(sha);
    selectCommitIfLoaded(sha);
  }

  const detailsState = computed<ManualBisectDetailsState | null>(() => {
    const current = session.value;
    if (!current) return null;
    return {
      phase: current.phase,
      badSha: current.badSha,
      goodSha: current.range[current.highIndex]?.sha || null,
      currentSha: current.currentSha,
      culpritSha: current.culpritSha,
    };
  });

  const remaining = computed(() => {
    const current = session.value;
    if (!current || current.phase === "select-good") return 0;
    return Math.max(0, current.highIndex - current.lowIndex + 1);
  });

  const currentCommit = computed(() => {
    const current = session.value;
    if (!current || current.currentIndex === null) return null;
    return current.range[current.currentIndex] || null;
  });

  const badBound = computed(() => {
    const current = session.value;
    if (!current) return null;
    return current.range[current.lowIndex] || commitSummaryFromLoadedSha(current.badSha);
  });

  const goodBound = computed(() => {
    const current = session.value;
    if (!current || current.phase === "select-good") return null;
    return current.range[current.highIndex] || null;
  });

  const culprit = computed(() => {
    const current = session.value;
    if (!current || !current.culpritSha) return null;
    return current.range.find((commit) => commit.sha === current.culpritSha)
      || commitSummaryFromLoadedSha(current.culpritSha);
  });

  function start(badSha: string) {
    if (options.hasConflicts.value) {
      toast.error("Resolve conflicts before starting Bug Autopsy.");
      return;
    }
    if (options.hasWorkingChanges.value) {
      toast.error("Commit, stash, or discard working changes before Bug Autopsy checkout steps.");
      return;
    }

    const badIndex = findLoadedCommitIndex(badSha);
    if (badIndex < 0) {
      toast.error("Selected commit is not loaded in the current graph.");
      return;
    }

    session.value = {
      phase: "select-good",
      badSha,
      originalBranch: options.currentBranch.value || "",
      range: [toManualCommit(options.commits.value[badIndex])],
      lowIndex: 0,
      highIndex: 0,
      currentIndex: null,
      currentSha: null,
      culpritSha: null,
      step: 0,
      tested: [],
    };
    toast.info("Bug Autopsy started. Select an older commit that still worked, then confirm it in the Info panel.");
  }

  async function checkoutIndex(index: number) {
    const current = session.value;
    if (!current) return;
    const commit = current.range[index];
    if (!commit) return;

    busy.value = true;
    const previousStep = current.step;
    current.currentIndex = index;
    current.currentSha = commit.sha;
    current.step += 1;

    try {
      options.gitError.value = null;
      await options.checkoutCommit(commit.sha);
      if (options.gitError.value) {
        current.currentIndex = index;
        current.currentSha = null;
        current.step = previousStep;
        toast.error("Bug Autopsy could not checkout the next commit. Clean the working tree, then try again.");
        return;
      }
      await focusCommitAfterCheckout(commit.sha);
    } finally {
      busy.value = false;
    }
  }

  async function advance() {
    const current = session.value;
    if (!current) return;

    if (current.highIndex - current.lowIndex <= 1) {
      const found = current.range[current.lowIndex];
      current.phase = "complete";
      current.culpritSha = found?.sha || current.badSha;
      current.currentIndex = null;
      current.currentSha = null;
      toast.success(`Bug Autopsy found ${shortSha(current.culpritSha)} as the likely breaking commit.`);
      selectCommitIfLoaded(current.culpritSha);
      return;
    }

    await checkoutIndex(Math.floor((current.lowIndex + current.highIndex) / 2));
  }

  async function selectGood(goodSha: string) {
    const current = session.value;
    if (!current || current.phase !== "select-good") return;

    const badIndex = findLoadedCommitIndex(current.badSha);
    const goodIndex = findLoadedCommitIndex(goodSha);
    if (badIndex < 0 || goodIndex < 0) {
      toast.error("Both commits must be loaded in the current graph.");
      return;
    }
    if (goodIndex <= badIndex) {
      toast.error("Pick an older commit below the broken commit, one that still worked.");
      return;
    }

    const range = options.commits.value.slice(badIndex, goodIndex + 1).map(toManualCommit);
    session.value = {
      phase: "testing",
      badSha: current.badSha,
      originalBranch: current.originalBranch,
      range,
      lowIndex: 0,
      highIndex: range.length - 1,
      currentIndex: null,
      currentSha: null,
      culpritSha: null,
      step: 0,
      tested: [],
    };

    await advance();
  }

  async function mark(result: ManualBisectResult) {
    const current = session.value;
    if (!current || current.phase !== "testing" || current.currentIndex === null || !current.currentSha) return;

    current.tested.push({ sha: current.currentSha, result });
    if (result === "good") {
      current.highIndex = current.currentIndex;
    } else {
      current.lowIndex = current.currentIndex;
    }

    current.currentIndex = null;
    current.currentSha = null;
    await advance();
  }

  async function retryCheckout() {
    const current = session.value;
    if (!current || current.phase !== "testing" || current.currentIndex === null || current.currentSha) return;
    await checkoutIndex(current.currentIndex);
  }

  function cancel() {
    session.value = null;
    busy.value = false;
    toast.info("Bug Autopsy cancelled.");
  }

  function close() {
    session.value = null;
  }

  async function checkoutCulprit() {
    const culpritSha = session.value?.culpritSha;
    if (!culpritSha) return;
    busy.value = true;
    try {
      await options.checkoutCommit(culpritSha);
      await focusCommitAfterCheckout(culpritSha);
    } finally {
      busy.value = false;
    }
  }

  async function returnToOriginalBranch() {
    const branch = session.value?.originalBranch;
    if (!branch) {
      toast.info("No original branch was recorded for this session.");
      return;
    }
    busy.value = true;
    try {
      await options.checkoutBranch(branch);
    } finally {
      busy.value = false;
    }
  }

  return {
    session,
    busy,
    detailsState,
    remaining,
    currentCommit,
    badBound,
    goodBound,
    culprit,
    start,
    selectGood,
    mark,
    retryCheckout,
    cancel,
    close,
    checkoutCulprit,
    returnToOriginalBranch,
  };
}

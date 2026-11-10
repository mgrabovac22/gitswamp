import { computed, onScopeDispose, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { checkCommitSecrets } from "@/domain/git/composables/secretCommitGuard";
import { statusHash } from "@/domain/git/composables/gitHelpers";
import { finishSecretConfirmation, secretConfirmation } from "@/shared/notifications/secretConfirmation";
import { useUndoableDestructiveAction } from "@/shared/notifications/useUndoableDestructiveAction";
import { useToast } from "@/shared/notifications/useToast";
import type { CommitFileInfo, CommitInfo, FileStatusInfo, RepositoryOperationInfo } from "@/types";
import type { WorkspaceLocation } from "./branchWorkspaceTypes";

const PAGE_SIZE = 60;

interface WorkspaceStatus { headSha: string; files: FileStatusInfo[]; operation: RepositoryOperationInfo | null }
type WorkspaceAction = { kind: "stage" | "unstage" | "discard"; paths: string[] }
  | { kind: "abort"; operation: "merge" | "cherry-pick" }
  | { kind: "resolve"; filePath?: string; strategy: "ours" | "theirs" | "delete" }
  | { kind: "commit"; message: string; allowEmpty: boolean; secretCheck: boolean; secretApproval?: string };

export function useBranchWorkspaceColumn(repoPath: string, branchRef: string) {
  const toast = useToast();
  const location = shallowRef<WorkspaceLocation | null>(null);
  const snapshot = shallowRef<WorkspaceStatus | null>(null);
  const commits = shallowRef<CommitInfo[]>([]);
  const selectedCommit = shallowRef<CommitInfo | null>(null);
  const commitFiles = shallowRef<CommitFileInfo[]>([]);
  const commitFilesLoading = ref(false);
  const commitFilesError = ref<string | null>(null);
  const opening = ref(false);
  const refreshing = ref(false);
  const historyLoading = ref(false);
  const hasMore = ref(false);
  const busy = ref(false);
  const error = ref<string | null>(null);
  const historyError = ref<string | null>(null);
  const staged = computed(() => snapshot.value?.files.filter(file => file.staged && !file.conflicted) ?? []);
  const unstaged = computed(() => snapshot.value?.files.filter(file => !file.staged && !file.conflicted) ?? []);
  const conflicts = computed(() => [...new Map(snapshot.value?.files.filter(file => file.conflicted).map(file => [file.path, file])).values()]);
  const context = computed(() => JSON.stringify([
    location.value?.path, snapshot.value?.headSha, snapshot.value?.operation?.kind, snapshot.value ? statusHash(snapshot.value.files) : null,
  ]));
  const { scheduleDestructiveAction } = useUndoableDestructiveAction(() => context.value, () => !busy.value && !error.value);
  let disposed = false;
  let statusGeneration = 0;
  let historyGeneration = 0;
  let fileGeneration = 0;

  onScopeDispose(() => {
    disposed = true;
    if (busy.value && location.value && secretConfirmation.value?.repoPath === location.value.path) finishSecretConfirmation();
  });

  async function loadHistory(reset = false) {
    const path = location.value?.path;
    const tip = snapshot.value?.headSha;
    if (!path || !tip || disposed || (!reset && (historyLoading.value || !hasMore.value))) return;
    const id = ++historyGeneration;
    if (reset) { commits.value = []; hasMore.value = false; }
    historyLoading.value = true;
    historyError.value = null;
    try {
      const page = await invoke<CommitInfo[]>("get_branch_workspace_history", { path, tip, offset: commits.value.length });
      if (disposed || id !== historyGeneration) return;
      hasMore.value = page.length > PAGE_SIZE;
      commits.value = [...commits.value, ...page.slice(0, PAGE_SIZE)];
    } catch (cause) {
      if (!disposed && id === historyGeneration) historyError.value = String(cause);
    } finally {
      if (id === historyGeneration) historyLoading.value = false;
    }
  }

  async function refresh(force = false) {
    const workspace = location.value;
    if (!workspace || disposed || (!force && (refreshing.value || busy.value))) return;
    const id = ++statusGeneration;
    refreshing.value = true;
    try {
      const next = await invoke<WorkspaceStatus>("get_branch_workspace_status", { path: workspace.path, branch: workspace.branch });
      if (disposed || id !== statusGeneration) return;
      const previous = snapshot.value;
      const headChanged = previous?.headSha !== next.headSha;
      const filesChanged = !previous || previous.files.length !== next.files.length || next.files.some((file, index) => {
        const prior = previous.files[index];
        return file.path !== prior.path || file.status !== prior.status || file.staged !== prior.staged
          || file.conflicted !== prior.conflicted || file.file_size_bytes !== prior.file_size_bytes;
      });
      if (headChanged || filesChanged || previous?.operation?.kind !== next.operation?.kind || previous?.operation?.message !== next.operation?.message) {
        snapshot.value = next;
      }
      error.value = null;
      if (headChanged) void loadHistory(true);
    } catch (cause) {
      if (!disposed && id === statusGeneration) error.value = String(cause);
    } finally { if (id === statusGeneration) refreshing.value = false; }
  }

  async function open() {
    if (disposed || opening.value) return;
    if (location.value) { await refresh(); return; }
    opening.value = true;
    error.value = null;
    try {
      const result = await invoke<WorkspaceLocation>("open_branch_workspace", { path: repoPath, branchRef });
      if (!disposed) { location.value = result; await refresh(); }
    } catch (cause) {
      if (!disposed) error.value = String(cause);
    } finally { opening.value = false; }
  }

  async function selectCommit(commit: CommitInfo | null) {
    const id = ++fileGeneration;
    selectedCommit.value = commit;
    commitFiles.value = [];
    commitFilesError.value = null;
    commitFilesLoading.value = !!commit;
    if (!commit || !location.value) return;
    try {
      const files = await invoke<CommitFileInfo[]>("get_commit_files", { path: location.value.path, sha: commit.sha });
      if (!disposed && id === fileGeneration) commitFiles.value = files;
    } catch (cause) {
      if (!disposed && id === fileGeneration) commitFilesError.value = String(cause);
    } finally {
      if (id === fileGeneration) commitFilesLoading.value = false;
    }
  }

  async function run(action: () => WorkspaceAction | null | Promise<WorkspaceAction | null>): Promise<boolean> {
    const workspace = location.value;
    if (disposed || !workspace || error.value || !snapshot.value || busy.value) return false;
    busy.value = true;
    try {
      const payload = await action();
      if (!payload || disposed) return false;
      await invoke("mutate_branch_workspace", { path: workspace.path, branch: workspace.branch, action: payload });
      await refresh(true);
      return true;
    } catch (cause) {
      if (!disposed) {
        toast.error(`${workspace.branch}: ${String(cause)}`);
        await refresh(true);
      }
      return false;
    } finally { busy.value = false; }
  }

  function changeStage(paths: string[], kind: "stage" | "unstage") {
    if (paths.length) void run(() => ({ kind, paths }));
  }

  async function commit(payload: { message: string; allowEmpty: boolean; done: (success: boolean) => void }) {
    const path = location.value?.path;
    if (!path) { payload.done(false); return; }
    const saved = await run(async () => {
      const approval = await checkCommitSecrets(path, () => !disposed && location.value?.path === path, () => refresh(true));
      return approval && { kind: "commit", message: payload.message, allowEmpty: payload.allowEmpty, ...approval };
    });
    payload.done(saved);
    if (saved && !disposed) toast.success(`Committed on ${location.value?.branch}`);
  }

  function discard(paths: string[]) {
    if (!paths.length || !snapshot.value || busy.value) return;
    const branch = location.value?.branch;
    if (snapshot.value.operation) { toast.warning("Complete or abort the active Git operation first."); return; }
    scheduleDestructiveAction({
      message: `Discard ${paths.length} unstaged ${paths.length === 1 ? "file" : "files"} on ${branch}?`,
      detail: "Staged changes are kept. Click Undo to cancel.",
      run: () => run(() => ({ kind: "discard", paths })),
    });
  }

  function abort(operation: "merge" | "cherry-pick") {
    scheduleDestructiveAction({
      message: `Abort ${operation} on ${location.value?.branch}?`,
      detail: "Click Undo to leave the operation in progress.",
      run: () => run(() => ({ kind: "abort", operation })),
    });
  }

  function resolve(filePath?: string) {
    const branch = location.value?.branch;
    const expected = context.value;
    const choose = (strategy: "ours" | "theirs") => {
      if (!disposed && context.value === expected) return run(() => ({ kind: "resolve", filePath, strategy })).then(() => {});
    };
    toast.action("warning", `Resolve ${filePath ?? "all conflicts"} on ${branch}?`, [
      { label: "Keep modified", style: "primary", onClick: () => choose("ours") },
      { label: "Keep base", style: "neutral", onClick: () => choose("theirs") },
      { label: "Cancel", style: "neutral", onClick: () => {} },
    ], 20000);
  }

  return { location, snapshot, commits, selectedCommit, commitFiles, commitFilesLoading, commitFilesError, opening,
    refreshing, historyLoading, hasMore, busy, error, historyError, staged, unstaged, conflicts, context, open, refresh,
    loadHistory, selectCommit, changeStage, commit, discard, abort, resolve };
}

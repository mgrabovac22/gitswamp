import { visualiseOperationsEnabled } from "@/shared/config/visualiseOperationsPreferences";
import type { RemoteInfo } from "@/types";

export const GIT_OPERATION_PREVIEW_EVENT = "gitswamp-git-operation-preview";
export const GIT_PENDING_PREVIEW_SETTLED_EVENT = "gitswamp-git-pending-preview-settled";
export type VisualisedOperation = "merge" | "cherry-pick" | "revert";

export interface CommitPreviewRequest {
  kind: VisualisedOperation;
  repoPath: string;
  source: string;
  target: string;
  resultSha: string;
  beforeSha?: string;
  sourceSha?: string;
}

export type RefOperationPreview = { repoPath: string; target: string; pendingUndo?: () => void } & (
  | { kind: "branch-deleted" }
  | { kind: "branch-created"; sourceSha: string }
  | { kind: "branch-renamed"; previousName: string }
  | { kind: "reset"; previousSha: string; resultSha: string; mode: "soft" | "mixed" | "hard" }
);

export interface TransferOperationPreview {
  kind: "push" | "pull" | "fetch" | "stash-save" | "stash-restore";
  repoPath: string;
  target: string;
  endpoint: { label: string; provider?: RemoteInfo["provider"] };
  sha?: string;
}

export type OperationPreviewRequest = CommitPreviewRequest | RefOperationPreview | TransferOperationPreview;

export function publishOperationPreview(request: OperationPreviewRequest) {
  if (!visualiseOperationsEnabled()) return;
  globalThis.dispatchEvent(new CustomEvent(GIT_OPERATION_PREVIEW_EVENT, { detail: request }));
}

export function publishPendingOperationPreview(
  request: Extract<RefOperationPreview, { kind: "branch-deleted" | "reset" }>,
  undo: () => void,
) {
  if (!visualiseOperationsEnabled()) return;
  publishOperationPreview({ ...request, pendingUndo: undo });
  return () => globalThis.dispatchEvent(new CustomEvent(GIT_PENDING_PREVIEW_SETTLED_EVENT, { detail: undo }));
}

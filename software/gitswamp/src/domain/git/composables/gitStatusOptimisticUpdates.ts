import type { FileStatusInfo } from "@/types";

import { statusHash } from "./gitHelpers";
import type { GitState } from "./gitState";

function commitStatusState(state: GitState, files: FileStatusInfo[]): void {
  state.fileStatuses.value = files;
  state.lastStatusHash.value = statusHash(files);
}

function replacePathEntries(
  files: FileStatusInfo[],
  filePath: string,
  nextEntries: FileStatusInfo[],
): FileStatusInfo[] {
  const firstIndex = files.findIndex((file) => file.path === filePath);
  const next = files.filter((file) => file.path !== filePath);

  if (nextEntries.length === 0) {
    return next;
  }

  const insertAt = firstIndex >= 0 ? firstIndex : next.length;
  next.splice(insertAt, 0, ...nextEntries);
  return next;
}

function moveFileToStage(files: FileStatusInfo[], filePath: string, staged: boolean): FileStatusInfo[] {
  const entries = files.filter((file) => file.path === filePath && !file.conflicted);
  const preferred = entries.find((file) => file.staged !== staged) ?? entries.find((file) => file.staged === staged);

  if (!preferred) {
    return files;
  }

  const existingTarget = entries.find((file) => file.staged === staged);
  const nextEntry: FileStatusInfo = {
    ...(existingTarget ?? preferred),
    status: preferred.status,
    staged,
  };

  return replacePathEntries(files, filePath, [nextEntry]);
}

export function markFilesStaged(state: GitState, filePaths: string[]): void {
  let next = state.fileStatuses.value;
  for (const filePath of filePaths) {
    next = moveFileToStage(next, filePath, true);
  }
  commitStatusState(state, next);
}

export function markFilesUnstaged(state: GitState, filePaths: string[]): void {
  let next = state.fileStatuses.value;
  for (const filePath of filePaths) {
    const hasUnstagedEntry = next.some((file) => file.path === filePath && !file.staged && !file.conflicted);
    if (hasUnstagedEntry) {
      next = next.filter((file) => !(file.path === filePath && file.staged && !file.conflicted));
      continue;
    }

    next = moveFileToStage(next, filePath, false);
  }
  commitStatusState(state, next);
}

export function removeUnstagedFiles(state: GitState, filePaths: string[]): void {
  const removePaths = new Set(filePaths);
  const next = state.fileStatuses.value.filter(
    (file) => file.conflicted || file.staged || !removePaths.has(file.path),
  );
  commitStatusState(state, next);
}

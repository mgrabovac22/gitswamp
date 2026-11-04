import { shallowRef } from "vue";

export interface SecretFinding { path: string; line: number; kind: string; tracked: boolean }
export interface SecretScan { findings: SecretFinding[]; skipped: string[]; snapshot: string; large_files?: { path: string; size: number }[] }
export type SecretDecision = "cancel" | "commit" | "ignore";
export const secretConfirmation = shallowRef<{ repoPath: string; report: SecretScan } | null>(null);
let complete: ((choice: SecretDecision) => void) | null = null;

export function finishSecretConfirmation(choice: SecretDecision = "cancel") {
  const resolve = complete;
  complete = null;
  secretConfirmation.value = null;
  resolve?.(choice);
}

export function requestSecretConfirmation(repoPath: string, report: SecretScan): Promise<SecretDecision> {
  finishSecretConfirmation();
  secretConfirmation.value = { repoPath, report };
  return new Promise(resolve => { complete = resolve; });
}

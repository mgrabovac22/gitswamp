export const ALLOW_EMPTY_COMMIT_STORAGE_KEY = "gitswamp-allow-empty-commit";
export const ALLOW_EMPTY_COMMIT_EVENT = "gitswamp-allow-empty-commit-changed";

export function getStoredAllowEmptyCommit(): boolean {
  try {
    const stored = localStorage.getItem(ALLOW_EMPTY_COMMIT_STORAGE_KEY);
    return stored === null ? false : stored === "true";
  } catch {
    return false;
  }
}

export function storeAllowEmptyCommit(enabled: boolean): void {
  try {
    localStorage.setItem(ALLOW_EMPTY_COMMIT_STORAGE_KEY, String(enabled));
  } catch {
  }

  globalThis.dispatchEvent(new CustomEvent(ALLOW_EMPTY_COMMIT_EVENT, { detail: enabled }));
}

export const DESTRUCTIVE_UNDO_STORAGE_KEY = "gitswamp-destructive-undo-enabled";
export const DESTRUCTIVE_UNDO_EVENT = "gitswamp-destructive-undo-changed";

export function getStoredDestructiveUndoEnabled(): boolean {
  try {
    const stored = localStorage.getItem(DESTRUCTIVE_UNDO_STORAGE_KEY);
    return stored === null ? true : stored !== "false";
  } catch {
    return true;
  }
}

export function storeDestructiveUndoEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(DESTRUCTIVE_UNDO_STORAGE_KEY, String(enabled));
  } catch {
    // Open views still observe the event when storage is unavailable.
  }

  globalThis.dispatchEvent(new CustomEvent(DESTRUCTIVE_UNDO_EVENT, { detail: enabled }));
}

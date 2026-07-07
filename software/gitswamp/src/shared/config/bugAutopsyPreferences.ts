export const BUG_AUTOPSY_STORAGE_KEY = "gitswamp-bug-autopsy-enabled";
export const BUG_AUTOPSY_EVENT = "gitswamp-bug-autopsy-changed";

export function getStoredBugAutopsyEnabled(): boolean {
  try {
    const stored = localStorage.getItem(BUG_AUTOPSY_STORAGE_KEY);
    return stored === null ? false : stored === "true";
  } catch {
    return false;
  }
}

export function storeBugAutopsyEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(BUG_AUTOPSY_STORAGE_KEY, String(enabled));
  } catch {
    // Storage can fail in restricted contexts; the event still updates open views.
  }

  globalThis.dispatchEvent(new CustomEvent(BUG_AUTOPSY_EVENT, { detail: enabled }));
}

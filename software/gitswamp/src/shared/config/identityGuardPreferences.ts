export const IDENTITY_GUARD_STORAGE_KEY = "gitswamp-identity-guard-enabled";
export const IDENTITY_GUARD_EVENT = "gitswamp-identity-guard-changed";

export function getStoredIdentityGuardEnabled(): boolean {
  try {
    const stored = localStorage.getItem(IDENTITY_GUARD_STORAGE_KEY);
    return stored === null ? false : stored === "true";
  } catch {
    return false;
  }
}

export function storeIdentityGuardEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(IDENTITY_GUARD_STORAGE_KEY, String(enabled));
  } catch {
    // Storage can fail in restricted contexts; the event still updates open views.
  }

  globalThis.dispatchEvent(new CustomEvent(IDENTITY_GUARD_EVENT, { detail: enabled }));
}

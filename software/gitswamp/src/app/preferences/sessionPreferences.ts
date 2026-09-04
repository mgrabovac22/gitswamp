import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

const RESTORE_SESSION_KEY = "gitswamp-restore-session";

export function shouldRestoreSession(): boolean {
  const saved = safeStorageGet(RESTORE_SESSION_KEY);
  if (saved === null) {
    safeStorageSet(RESTORE_SESSION_KEY, "true");
    return true;
  }

  return saved === "true";
}

import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

const key = "gitswamp-visualise-operations";
export const VISUALISE_OPERATIONS_EVENT = "gitswamp-visualise-operations-changed";

export function visualiseOperationsEnabled() {
  return safeStorageGet(key) === "true";
}

export function storeVisualiseOperations(enabled: boolean) {
  safeStorageSet(key, String(enabled));
  globalThis.dispatchEvent(new Event(VISUALISE_OPERATIONS_EVENT));
}

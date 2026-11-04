export const SECRET_DETECTOR_EVENT = "gitswamp-secret-detector-changed";
const KEY = "gitswamp-secret-detector-enabled";

export function secretDetectorEnabled(): boolean {
  try { return localStorage.getItem(KEY) !== "false"; } catch { return true; }
}

export function storeSecretDetectorEnabled(enabled: boolean): void {
  try { localStorage.setItem(KEY, String(enabled)); } catch { /* Keep the safe default. */ }
  globalThis.dispatchEvent(new CustomEvent(SECRET_DETECTOR_EVENT, { detail: enabled }));
}

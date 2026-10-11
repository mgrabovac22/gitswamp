export const SMART_GITIGNORE_WIZARD_STORAGE_KEY = "gitswamp-smart-gitignore-wizard-enabled";
export const SMART_GITIGNORE_WIZARD_EVENT = "gitswamp-smart-gitignore-wizard-changed";

export function getStoredSmartGitignoreWizardEnabled(): boolean {
  try {
    const stored = localStorage.getItem(SMART_GITIGNORE_WIZARD_STORAGE_KEY);
    return stored === null ? true : stored !== "false";
  } catch {
    return true;
  }
}

export function storeSmartGitignoreWizardEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(SMART_GITIGNORE_WIZARD_STORAGE_KEY, String(enabled));
  } catch {
  }

  globalThis.dispatchEvent(new CustomEvent(SMART_GITIGNORE_WIZARD_EVENT, { detail: enabled }));
}

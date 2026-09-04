export const BACKGROUND_MAINTENANCE_STORAGE_KEY = "gitswamp-background-maintenance-settings";
export const BACKGROUND_MAINTENANCE_EVENT = "gitswamp-background-maintenance-settings-changed";

export interface BackgroundMaintenanceSettings {
  healthRefreshEnabled: boolean;
  remoteHygieneEnabled: boolean;
  focusSyncEnabled: boolean;
  idleOnlyEnabled: boolean;
  staleWorkReminderEnabled: boolean;
  behindBranchReminderEnabled: boolean;
  largeChangeReminderEnabled: boolean;
  largeChangeThreshold: number;
  conflictReminderEnabled: boolean;
  commitDetailsPreloadEnabled: boolean;
  intervalMinutes: number;
}

export const DEFAULT_BACKGROUND_MAINTENANCE_SETTINGS: BackgroundMaintenanceSettings = {
  healthRefreshEnabled: false,
  remoteHygieneEnabled: false,
  focusSyncEnabled: false,
  idleOnlyEnabled: false,
  staleWorkReminderEnabled: false,
  behindBranchReminderEnabled: false,
  largeChangeReminderEnabled: false,
  largeChangeThreshold: 40,
  conflictReminderEnabled: false,
  commitDetailsPreloadEnabled: false,
  intervalMinutes: 10,
};

export function hasBackgroundMaintenanceEnabled(settings: BackgroundMaintenanceSettings): boolean {
  return settings.healthRefreshEnabled
    || settings.remoteHygieneEnabled
    || settings.focusSyncEnabled
    || settings.staleWorkReminderEnabled
    || settings.behindBranchReminderEnabled
    || settings.largeChangeReminderEnabled
    || settings.conflictReminderEnabled
    || settings.commitDetailsPreloadEnabled;
}

export function getStoredBackgroundMaintenanceSettings(): BackgroundMaintenanceSettings {
  try {
    const raw = localStorage.getItem(BACKGROUND_MAINTENANCE_STORAGE_KEY);
    if (!raw) {
      return { ...DEFAULT_BACKGROUND_MAINTENANCE_SETTINGS };
    }

    return sanitizeBackgroundMaintenanceSettings(JSON.parse(raw) as Partial<BackgroundMaintenanceSettings>);
  } catch {
    return { ...DEFAULT_BACKGROUND_MAINTENANCE_SETTINGS };
  }
}

export function storeBackgroundMaintenanceSettings(settings: BackgroundMaintenanceSettings): BackgroundMaintenanceSettings {
  const safe = sanitizeBackgroundMaintenanceSettings(settings);
  try {
    localStorage.setItem(BACKGROUND_MAINTENANCE_STORAGE_KEY, JSON.stringify(safe));
  } catch {
    // Storage can fail in restricted contexts. The event still keeps the current session in sync.
  }

  globalThis.dispatchEvent(new CustomEvent(BACKGROUND_MAINTENANCE_EVENT, { detail: safe }));
  return safe;
}

export function updateBackgroundMaintenanceSettings(partial: Partial<BackgroundMaintenanceSettings>): BackgroundMaintenanceSettings {
  return storeBackgroundMaintenanceSettings({
    ...getStoredBackgroundMaintenanceSettings(),
    ...partial,
  });
}

function sanitizeBackgroundMaintenanceSettings(value: Partial<BackgroundMaintenanceSettings>): BackgroundMaintenanceSettings {
  return {
    healthRefreshEnabled: value.healthRefreshEnabled === true,
    remoteHygieneEnabled: value.remoteHygieneEnabled === true,
    focusSyncEnabled: value.focusSyncEnabled === true,
    idleOnlyEnabled: value.idleOnlyEnabled === true,
    staleWorkReminderEnabled: value.staleWorkReminderEnabled === true,
    behindBranchReminderEnabled: value.behindBranchReminderEnabled === true,
    largeChangeReminderEnabled: value.largeChangeReminderEnabled === true,
    largeChangeThreshold: sanitizeLargeChangeThreshold(value.largeChangeThreshold),
    conflictReminderEnabled: value.conflictReminderEnabled === true,
    commitDetailsPreloadEnabled: value.commitDetailsPreloadEnabled === true,
    intervalMinutes: sanitizeIntervalMinutes(value.intervalMinutes),
  };
}

function sanitizeIntervalMinutes(value: number | undefined): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return DEFAULT_BACKGROUND_MAINTENANCE_SETTINGS.intervalMinutes;
  }

  return Math.max(2, Math.min(60, Math.round(value)));
}

function sanitizeLargeChangeThreshold(value: number | undefined): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return DEFAULT_BACKGROUND_MAINTENANCE_SETTINGS.largeChangeThreshold;
  }

  return Math.max(8, Math.min(500, Math.round(value)));
}

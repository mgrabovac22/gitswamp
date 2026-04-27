import { APP_THEME_OPTIONS, type AppPalettePreference, type ThemeModePreference } from "./themeCatalog";

export const THEME_MODE_STORAGE_KEY = "gitswamp-theme-mode";
export const APP_PALETTE_STORAGE_KEY = "gitswamp-theme-palette";

const MODE_VALUES = new Set<ThemeModePreference>(["dark", "light"]);
const APP_PALETTE_VALUES = new Set<AppPalettePreference>(APP_THEME_OPTIONS.map((theme) => theme.id));

function safeStorageGet(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function safeStorageSet(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {}
}

export function getStoredThemeModePreference(): ThemeModePreference {
  const saved = safeStorageGet(THEME_MODE_STORAGE_KEY);
  if (saved && MODE_VALUES.has(saved as ThemeModePreference)) {
    return saved as ThemeModePreference;
  }

  return "dark";
}

export function getStoredAppPalettePreference(): AppPalettePreference {
  const saved = safeStorageGet(APP_PALETTE_STORAGE_KEY);
  if (saved && APP_PALETTE_VALUES.has(saved as AppPalettePreference)) {
    return saved as AppPalettePreference;
  }

  return "swamp";
}

export function applyThemeModePreference(preference: ThemeModePreference): void {
  document.documentElement.classList.toggle("light", preference === "light");
}

export function storeThemeModePreference(preference: ThemeModePreference): void {
  safeStorageSet(THEME_MODE_STORAGE_KEY, preference);
}

export function applyAppPalettePreference(preference: AppPalettePreference): void {
  document.documentElement.dataset.appTheme = preference;
}

export function storeAppPalettePreference(preference: AppPalettePreference): void {
  safeStorageSet(APP_PALETTE_STORAGE_KEY, preference);
}

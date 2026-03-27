import { APP_THEME_OPTIONS, type AppPalettePreference, type ThemeModePreference } from "./themeCatalog";

export const THEME_MODE_STORAGE_KEY = "gitswamp-theme-mode";
export const APP_PALETTE_STORAGE_KEY = "gitswamp-theme-palette";

const MODE_VALUES = new Set<ThemeModePreference>(["dark", "light"]);
const APP_PALETTE_VALUES = new Set<AppPalettePreference>(APP_THEME_OPTIONS.map((theme) => theme.id));

export function getStoredThemeModePreference(): ThemeModePreference {
  const saved = localStorage.getItem(THEME_MODE_STORAGE_KEY);
  if (saved && MODE_VALUES.has(saved as ThemeModePreference)) {
    return saved as ThemeModePreference;
  }

  return "dark";
}

export function getStoredAppPalettePreference(): AppPalettePreference {
  const saved = localStorage.getItem(APP_PALETTE_STORAGE_KEY);
  if (saved && APP_PALETTE_VALUES.has(saved as AppPalettePreference)) {
    return saved as AppPalettePreference;
  }

  return "swamp";
}

export function applyThemeModePreference(preference: ThemeModePreference): void {
  document.documentElement.classList.toggle("light", preference === "light");
}

export function storeThemeModePreference(preference: ThemeModePreference): void {
  localStorage.setItem(THEME_MODE_STORAGE_KEY, preference);
}

export function applyAppPalettePreference(preference: AppPalettePreference): void {
  document.documentElement.dataset.appTheme = preference;
}

export function storeAppPalettePreference(preference: AppPalettePreference): void {
  localStorage.setItem(APP_PALETTE_STORAGE_KEY, preference);
}

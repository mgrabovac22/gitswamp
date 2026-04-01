export type ThemeModePreference = "dark" | "light";

export type AppPalettePreference =
  | "swamp"
  | "default"
  | "github-dark"
  | "graphite-gray"
  | "dark-red"
  | "emerald-night"
  | "midnight-blue"
  | "github-light"
  | "mint-light"
  | "sand-light"
  | "rose-light";

export interface AppThemeOption {
  id: AppPalettePreference;
  label: string;
  mode: ThemeModePreference;
  group: "dark" | "light";
}

export const APP_THEME_OPTIONS: AppThemeOption[] = [
  { id: "swamp", label: "Swamp", mode: "dark", group: "dark" },
  { id: "default", label: "Default Dark", mode: "dark", group: "dark" },
  { id: "github-dark", label: "GitHub Dark", mode: "dark", group: "dark" },
  { id: "graphite-gray", label: "Graphite Gray", mode: "dark", group: "dark" },
  { id: "dark-red", label: "Dark Red", mode: "dark", group: "dark" },
  { id: "emerald-night", label: "Emerald Night", mode: "dark", group: "dark" },
  { id: "midnight-blue", label: "Midnight Blue", mode: "dark", group: "dark" },
  { id: "github-light", label: "GitHub Light", mode: "light", group: "light" },
  { id: "mint-light", label: "Mint Light", mode: "light", group: "light" },
  { id: "sand-light", label: "Sand Light", mode: "light", group: "light" },
  { id: "rose-light", label: "Rose Light", mode: "light", group: "light" },
];

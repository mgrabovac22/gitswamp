export type ThemeModePreference = "dark" | "light";

export type AppPalettePreference =
  | "swamp"
  | "default"
  | "github-dark"
  | "graphite-gray"
  | "dark-red"
  | "emerald-night"
  | "midnight-blue"
  | "obsidian-teal"
  | "copper-night"
  | "nord-fjord"
  | "espresso"
  | "github-light"
  | "mint-light"
  | "sand-light"
  | "rose-light"
  | "paper-blue"
  | "lavender-light"
  | "olive-light"
  | "slate-light"
  | "storm-forge"
  | "forest-midnight"
  | "crimson-oxide"
  | "deep-ocean"
  | "charcoal-gold"
  | "dusk-slate-light"
  | "smoke-blue-light"
  | "terracotta-light"
  | "pine-mist-light"
  | "graphite-paper-light";

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
  { id: "obsidian-teal", label: "Obsidian Teal", mode: "dark", group: "dark" },
  { id: "copper-night", label: "Copper Night", mode: "dark", group: "dark" },
  { id: "nord-fjord", label: "Nord Fjord", mode: "dark", group: "dark" },
  { id: "espresso", label: "Espresso", mode: "dark", group: "dark" },
  { id: "github-light", label: "GitHub Light", mode: "light", group: "light" },
  { id: "mint-light", label: "Mint Light", mode: "light", group: "light" },
  { id: "sand-light", label: "Sand Light", mode: "light", group: "light" },
  { id: "rose-light", label: "Rose Light", mode: "light", group: "light" },
  { id: "paper-blue", label: "Paper Blue", mode: "light", group: "light" },
  { id: "lavender-light", label: "Lavender Light", mode: "light", group: "light" },
  { id: "olive-light", label: "Olive Light", mode: "light", group: "light" },
  { id: "slate-light", label: "Slate Light", mode: "light", group: "light" },
  { id: "storm-forge", label: "Storm Forge", mode: "dark", group: "dark" },
  { id: "forest-midnight", label: "Forest Midnight", mode: "dark", group: "dark" },
  { id: "crimson-oxide", label: "Crimson Oxide", mode: "dark", group: "dark" },
  { id: "deep-ocean", label: "Deep Ocean", mode: "dark", group: "dark" },
  { id: "charcoal-gold", label: "Charcoal Gold", mode: "dark", group: "dark" },
  { id: "dusk-slate-light", label: "Dusk Slate Light", mode: "light", group: "light" },
  { id: "smoke-blue-light", label: "Smoke Blue Light", mode: "light", group: "light" },
  { id: "terracotta-light", label: "Terracotta Light", mode: "light", group: "light" },
  { id: "pine-mist-light", label: "Pine Mist Light", mode: "light", group: "light" },
  { id: "graphite-paper-light", label: "Graphite Paper Light", mode: "light", group: "light" },
];

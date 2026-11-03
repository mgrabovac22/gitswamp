export type ThemeModePreference = "dark" | "light";

export type AppPalettePreference =
  | "swamp"
  | "default"
  | "default-light"
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
  | "graphite-paper-light"
  | "volcanic-night"
  | "arctic-paper-light"
  | "aurora-night"
  | "sakura-night"
  | "desert-dusk"
  | "abyss-purple"
  | "rainforest-night"
  | "seafoam-light"
  | "peach-light"
  | "sky-light"
  | "lemon-grove-light"
  | "blush-gray-light"
  | "neon-grid"
  | "solar-flare"
  | "vaporwave"
  | "holographic-light"
  | "blueprint-light"
  | "kitty"
  | "kawaii-pastel"
  | "anime-sakura"
  | "pixel-arcade"
  | "starry-night"
  | "snowy"
  | "lux-special"
  | "all-star-ogre"
  | "toon-town"
  | "comic-boom"
  | "rubber-hose"
  | "cartoon-carnival";

export interface AppThemeOption {
  id: AppPalettePreference;
  label: string;
  mode: ThemeModePreference;
  group: "dark" | "light" | "special";
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
  { id: "default-light", label: "Default Light", mode: "light", group: "light" },
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
  { id: "volcanic-night", label: "Volcanic Night", mode: "dark", group: "dark" },
  { id: "arctic-paper-light", label: "Arctic Paper Light", mode: "light", group: "light" },
  { id: "aurora-night", label: "Aurora Night", mode: "dark", group: "dark" },
  { id: "sakura-night", label: "Sakura Night", mode: "dark", group: "dark" },
  { id: "desert-dusk", label: "Desert Dusk", mode: "dark", group: "dark" },
  { id: "abyss-purple", label: "Abyss Purple", mode: "dark", group: "dark" },
  { id: "rainforest-night", label: "Rainforest Night", mode: "dark", group: "dark" },
  { id: "seafoam-light", label: "Seafoam Light", mode: "light", group: "light" },
  { id: "peach-light", label: "Peach Light", mode: "light", group: "light" },
  { id: "sky-light", label: "Sky Light", mode: "light", group: "light" },
  { id: "lemon-grove-light", label: "Lemon Grove Light", mode: "light", group: "light" },
  { id: "blush-gray-light", label: "Blush Gray Light", mode: "light", group: "light" },
  { id: "neon-grid", label: "Neon Grid", mode: "dark", group: "special" },
  { id: "solar-flare", label: "Solar Flare", mode: "dark", group: "special" },
  { id: "vaporwave", label: "Vaporwave", mode: "dark", group: "special" },
  { id: "holographic-light", label: "Holographic Light", mode: "light", group: "special" },
  { id: "blueprint-light", label: "Blueprint Light", mode: "light", group: "special" },
  { id: "kitty", label: "Kitty", mode: "light", group: "special" },
  { id: "kawaii-pastel", label: "Kawaii Pastel", mode: "light", group: "special" },
  { id: "anime-sakura", label: "Anime Sakura", mode: "dark", group: "special" },
  { id: "pixel-arcade", label: "Pixel Arcade", mode: "dark", group: "special" },
  { id: "starry-night", label: "Starry Night", mode: "dark", group: "special" },
  { id: "snowy", label: "Snowy", mode: "light", group: "special" },
  { id: "lux-special", label: "Lux Special", mode: "light", group: "special" },
  { id: "all-star-ogre", label: "All-Star Ogre", mode: "dark", group: "special" },
  { id: "toon-town", label: "Toon Town", mode: "light", group: "special" },
  { id: "comic-boom", label: "Comic Boom", mode: "light", group: "special" },
  { id: "rubber-hose", label: "Rubber Hose", mode: "light", group: "special" },
  { id: "cartoon-carnival", label: "Cartoon Carnival", mode: "dark", group: "special" },
];

import { readonly, shallowRef } from "vue";
import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

export const PET_HATS = [
  { id: "none", label: "No hat" }, { id: "top", label: "Top hat" }, { id: "crown", label: "Crown" },
  { id: "beanie", label: "Beanie" }, { id: "party", label: "Party hat" }, { id: "explorer", label: "Explorer hat" },
] as const;
export const PET_ACCESSORIES = [
  { id: "none", label: "No accessory" }, { id: "bandana", label: "Bandana" }, { id: "bow", label: "Bow tie" },
] as const;
export const PET_COLORS = [
  { id: "plum", label: "Plum", value: "#9c62bd" }, { id: "river", label: "River", value: "#389b9b" },
  { id: "copper", label: "Copper", value: "#bd632e" }, { id: "rose", label: "Rose", value: "#d14f77" },
  { id: "ink", label: "Ink", value: "#41465c" },
] as const;

export type PetAppearance = {
  name: string;
  hat: typeof PET_HATS[number]["id"];
  accessory: typeof PET_ACCESSORIES[number]["id"];
  color: typeof PET_COLORS[number]["id"];
  tricks: boolean;
};

export const DEFAULT_PET: PetAppearance = { name: "Swampy", hat: "none", accessory: "none", color: "plum", tricks: true };
const ENABLED_KEY = "gitswamp-desktop-pet";
const APPEARANCE_KEY = "gitswamp-desktop-pet-appearance";

function sanitize(value?: Partial<PetAppearance> | null): PetAppearance {
  return {
    name: (typeof value?.name === "string" ? value.name.trim().slice(0, 24) : "") || DEFAULT_PET.name,
    hat: PET_HATS.find(item => item.id === value?.hat)?.id ?? DEFAULT_PET.hat,
    accessory: PET_ACCESSORIES.find(item => item.id === value?.accessory)?.id ?? DEFAULT_PET.accessory,
    color: PET_COLORS.find(item => item.id === value?.color)?.id ?? DEFAULT_PET.color,
    tricks: typeof value?.tricks === "boolean" ? value.tricks : DEFAULT_PET.tricks,
  };
}

function readAppearance() {
  try { return sanitize(JSON.parse(safeStorageGet(APPEARANCE_KEY) ?? "null")); }
  catch { return { ...DEFAULT_PET }; }
}

const enabled = shallowRef(safeStorageGet(ENABLED_KEY) === "true");
const appearance = shallowRef(readAppearance());

export const petEnabled = readonly(enabled);
export const petAppearance = readonly(appearance);

export function setPetEnabled(value: boolean) {
  enabled.value = value;
  safeStorageSet(ENABLED_KEY, String(value));
}

export function savePetAppearance(value: PetAppearance) {
  appearance.value = sanitize(value);
  safeStorageSet(APPEARANCE_KEY, JSON.stringify(appearance.value));
}

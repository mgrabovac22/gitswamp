import { ref } from "vue";
import {
  applyAppPalettePreference,
  applyThemeModePreference,
  getStoredAppPalettePreference,
  getStoredThemeModePreference,
} from "@/shared/themePreferences";
import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

const MIN_FONT_SIZE = 10;
const MAX_FONT_SIZE = 40;
const COMMIT_SCALE_NAMES = [
  "tiny",
  "small",
  "medium",
  "largest",
  "huge",
  "xlarge",
  "xxlarge",
  "xxxlarge",
  "xxxxlarge",
] as const;

const legacyGeneralFontSizeByScale: Record<string, string> = {
  tiny: "14px",
  small: "16px",
  medium: "18px",
  largest: "22px",
  huge: "24px",
  large: "18px",
};

function applyStoredDocumentClass(key: string, className: string, enabledValue = "true") {
  if (safeStorageGet(key) === enabledValue) {
    document.documentElement.classList.add(className);
  }
}

export function useAppAppearance() {
  const generalFontSize = ref<number>(16);
  const commitNumeric = ref<number>(4);

  function applyGeneralFontSize(size: number) {
    const clamped = Math.max(MIN_FONT_SIZE, Math.min(MAX_FONT_SIZE, Math.round(size)));
    generalFontSize.value = clamped;
    document.documentElement.style.setProperty("--font-size", `${clamped}px`);
    safeStorageSet("gitswamp-general-font-size", String(clamped));
  }

  function applyCommitNumeric(n: number) {
    const clamped = Math.max(0, Math.min(COMMIT_SCALE_NAMES.length - 1, Math.round(n)));
    commitNumeric.value = clamped;
    const name = COMMIT_SCALE_NAMES[clamped] as string;
    document.documentElement.classList.remove(
      "font-scale-tiny",
      "font-scale-small",
      "font-scale-medium",
      "font-scale-largest",
      "font-scale-huge",
      "font-scale-xlarge",
      "font-scale-xxlarge",
      "font-scale-xxxlarge",
      "font-scale-xxxxlarge",
    );
    document.documentElement.classList.add(`font-scale-${name}`);
    safeStorageSet("gitswamp-font-size", name);
  }

  function initialize() {
    applyThemeModePreference(getStoredThemeModePreference());
    applyAppPalettePreference(getStoredAppPalettePreference());

    const savedGeneralFontSize = safeStorageGet("gitswamp-general-font-size");
    if (savedGeneralFontSize) {
      const parsedGeneral = Number.parseInt(savedGeneralFontSize, 10);
      const generalSize = Number.isFinite(parsedGeneral)
        ? Math.max(MIN_FONT_SIZE, Math.min(MAX_FONT_SIZE, parsedGeneral))
        : 16;
      applyGeneralFontSize(generalSize);
    } else {
      const savedLegacyScale = safeStorageGet("gitswamp-font-size");
      const legacySize = savedLegacyScale ? legacyGeneralFontSizeByScale[savedLegacyScale] : undefined;
      applyGeneralFontSize(Number.parseInt(legacySize ?? "16", 10) || 16);
    }

    const savedCommitScale = safeStorageGet("gitswamp-font-size");
    if (savedCommitScale && (COMMIT_SCALE_NAMES as readonly string[]).includes(savedCommitScale)) {
      applyCommitNumeric(COMMIT_SCALE_NAMES.indexOf(savedCommitScale as typeof COMMIT_SCALE_NAMES[number]));
    } else {
      applyCommitNumeric(4);
    }

    applyStoredDocumentClass("gitswamp-compact-mode", "compact");
    applyStoredDocumentClass("gitswamp-dummy-mode", "dummy-mode");
    applyStoredDocumentClass("gitswamp-show-avatars", "hide-avatars", "false");
    applyStoredDocumentClass("gitswamp-reduced-motion", "reduced-motion");
    applyStoredDocumentClass("gitswamp-wrap-diff-lines", "diff-wrap-lines");
    applyStoredDocumentClass("gitswamp-show-diff-line-numbers", "hide-diff-line-numbers", "false");
  }

  initialize();

  return {
    generalFontSize,
    commitNumeric,
    applyGeneralFontSize,
    applyCommitNumeric,
  };
}

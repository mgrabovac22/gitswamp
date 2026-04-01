import {
  DEFAULT_COMMIT_ANALYZER_SETTINGS,
  type CommitAnalyzerSettings,
  type CommitRuleSeverity,
} from "@/domain/analyzer/commitAnalyzer";

export const COMMIT_ANALYZER_SETTINGS_STORAGE_KEY = "gitswamp-commit-analyzer-settings";
export const COMMIT_ANALYZER_SETTINGS_EVENT = "gitswamp-commit-analyzer-settings-changed";

const MAX_CUSTOM_WORDS = 60;

export function getStoredCommitAnalyzerSettings(): CommitAnalyzerSettings {
  const raw = localStorage.getItem(COMMIT_ANALYZER_SETTINGS_STORAGE_KEY);
  if (!raw) {
    return { ...DEFAULT_COMMIT_ANALYZER_SETTINGS };
  }

  try {
    const parsed = JSON.parse(raw) as Partial<CommitAnalyzerSettings>;
    return sanitizeCommitAnalyzerSettings(parsed);
  } catch {
    return { ...DEFAULT_COMMIT_ANALYZER_SETTINGS };
  }
}

export function storeCommitAnalyzerSettings(settings: CommitAnalyzerSettings): void {
  const safe = sanitizeCommitAnalyzerSettings(settings);
  localStorage.setItem(COMMIT_ANALYZER_SETTINGS_STORAGE_KEY, JSON.stringify(safe));
  globalThis.dispatchEvent(new CustomEvent(COMMIT_ANALYZER_SETTINGS_EVENT, { detail: safe }));
}

export function updateCommitAnalyzerSettings(partial: Partial<CommitAnalyzerSettings>): CommitAnalyzerSettings {
  const next = sanitizeCommitAnalyzerSettings({
    ...getStoredCommitAnalyzerSettings(),
    ...partial,
  });
  storeCommitAnalyzerSettings(next);
  return next;
}

function sanitizeCommitAnalyzerSettings(value: Partial<CommitAnalyzerSettings>): CommitAnalyzerSettings {
  return {
    enabled: value.enabled ?? DEFAULT_COMMIT_ANALYZER_SETTINGS.enabled,
    showScore: value.showScore ?? DEFAULT_COMMIT_ANALYZER_SETTINGS.showScore,
    severityThreshold: sanitizeSeverity(value.severityThreshold),
    disabledRules: sanitizeStringList(value.disabledRules || []),
    conventionalCommitsEnforced: value.conventionalCommitsEnforced ?? DEFAULT_COMMIT_ANALYZER_SETTINGS.conventionalCommitsEnforced,
    customVagueWords: sanitizeStringList(value.customVagueWords || [], MAX_CUSTOM_WORDS),
    maxDiffLinesForDescWarning: sanitizeNumber(value.maxDiffLinesForDescWarning, DEFAULT_COMMIT_ANALYZER_SETTINGS.maxDiffLinesForDescWarning),
  };
}

function sanitizeSeverity(value?: string): CommitRuleSeverity {
  if (value === "error" || value === "warning" || value === "info") {
    return value;
  }
  return DEFAULT_COMMIT_ANALYZER_SETTINGS.severityThreshold;
}

function sanitizeStringList(values: string[], maxItems = 200): string[] {
  return Array.from(
    new Set(
      values
        .map((value) => value.trim())
        .filter((value) => value.length > 0)
        .slice(0, maxItems),
    ),
  );
}

function sanitizeNumber(value: number | undefined, fallback: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return fallback;
  }
  return Math.max(1, Math.round(value));
}

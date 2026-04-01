export type CommitRuleSeverity = "error" | "warning" | "info";

export interface StagedFileSnapshot {
  path: string;
  status: string;
  staged: boolean;
}

export interface DiffSummary {
  totalLinesAdded: number;
  totalLinesRemoved: number;
  filesChanged: number;
  fileTypes: string[];
  hasTestChanges: boolean;
  hasMigrationChanges: boolean;
  inferredScope: string;
}

export interface CommitAnalyzerSettings {
  enabled: boolean;
  showScore: boolean;
  severityThreshold: CommitRuleSeverity;
  disabledRules: string[];
  conventionalCommitsEnforced: boolean;
  customVagueWords: string[];
  maxDiffLinesForDescWarning: number;
}

export const DEFAULT_COMMIT_ANALYZER_SETTINGS: CommitAnalyzerSettings = {
  enabled: true,
  showScore: true,
  severityThreshold: "warning",
  disabledRules: [],
  conventionalCommitsEnforced: false,
  customVagueWords: [],
  maxDiffLinesForDescWarning: 200,
};

export interface CommitContext {
  message: string;
  description: string;
  stagedFiles: StagedFileSnapshot[];
  diffSummary: DiffSummary;
  detectedLanguage: string;
  settings: CommitAnalyzerSettings;
}

export interface RuleResult {
  passed: boolean;
  feedback?: string;
}

export interface CommitRule {
  id: string;
  name: string;
  severity: CommitRuleSeverity;
  weight: number;
  languages: string[];
  check: (ctx: CommitContext) => RuleResult;
}

export interface CommitLintFinding {
  id: string;
  severity: CommitRuleSeverity;
  weight: number;
  message: string;
}

export interface CommitLintResult {
  enabled: true;
  detectedLanguage: string;
  score: number;
  findings: CommitLintFinding[];
  errors: CommitLintFinding[];
  warnings: CommitLintFinding[];
  info: CommitLintFinding[];
}

export interface CommitAnalyzerInput {
  message: string;
  description: string;
  stagedFiles: StagedFileSnapshot[];
  diffSummary?: Partial<DiffSummary>;
  settings: CommitAnalyzerSettings;
}

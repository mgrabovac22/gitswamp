import { detectLanguage } from "./commitAnalyzerLanguage";
import { normalizeDiffSummary } from "../analyzer/commitAnalyzerDiff";
import { getCommitRules, severityRank } from "../analyzer/commitAnalyzerRules";
import { calculateCommitScore } from "../analyzer/commitAnalyzerScorer";
import type {
  CommitAnalyzerInput,
  CommitAnalyzerSettings,
  CommitLintFinding,
  CommitLintResult,
  CommitRuleSeverity,
  CommitContext,
} from "./commitAnalyzerTypes";

export class CommitLintEngine {
  private readonly rules = getCommitRules();

  analyze(input: CommitAnalyzerInput): CommitLintResult | null {
    const settings = normalizeSettings(input.settings);
    if (!settings.enabled) {
      return null;
    }

    const detectedLanguage = detectLanguage(`${input.message}\n${input.description}`);
    const context: CommitContext = {
      message: input.message,
      description: input.description,
      stagedFiles: input.stagedFiles,
      diffSummary: normalizeDiffSummary(input.diffSummary),
      detectedLanguage,
      settings,
    };

    const allFindings: CommitLintFinding[] = [];

    for (const rule of this.rules) {
      if (settings.disabledRules.includes(rule.id)) {
        continue;
      }

      if (!rule.languages.includes("all") && !rule.languages.includes(detectedLanguage)) {
        continue;
      }

      const outcome = rule.check(context);
      if (outcome.passed) {
        continue;
      }

      const severity = resolveSeverity(rule.id, rule.severity, settings);
      allFindings.push({
        id: rule.id,
        severity,
        weight: rule.weight,
        message: outcome.feedback || rule.name,
      });
    }

    const score = calculateCommitScore(allFindings);
    const threshold = severityRank(settings.severityThreshold);
    const findings = allFindings
      .filter((finding) => severityRank(finding.severity) <= threshold)
      .sort((left, right) => {
        const severityDelta = severityRank(left.severity) - severityRank(right.severity);
        if (severityDelta !== 0) {
          return severityDelta;
        }

        if (left.weight !== right.weight) {
          return right.weight - left.weight;
        }

        return left.id.localeCompare(right.id);
      });

    return {
      enabled: true,
      detectedLanguage,
      score,
      findings,
      errors: findings.filter((finding) => finding.severity === "error"),
      warnings: findings.filter((finding) => finding.severity === "warning"),
      info: findings.filter((finding) => finding.severity === "info"),
    };
  }
}

export function createCommitLintEngine(): CommitLintEngine {
  return new CommitLintEngine();
}

function resolveSeverity(
  id: string,
  severity: CommitRuleSeverity,
  settings: CommitAnalyzerSettings,
): CommitRuleSeverity {
  if (id === "msg-conventional-format" && settings.conventionalCommitsEnforced) {
    return "error";
  }
  return severity;
}

function normalizeSettings(settings: CommitAnalyzerSettings): CommitAnalyzerSettings {
  return {
    ...settings,
    disabledRules: Array.from(new Set(settings.disabledRules.map((value) => value.trim()).filter((value) => value.length > 0))),
    customVagueWords: Array.from(new Set(settings.customVagueWords.map((value) => value.trim()).filter((value) => value.length > 0))),
    maxDiffLinesForDescWarning: Math.max(1, settings.maxDiffLinesForDescWarning || 200),
  };
}

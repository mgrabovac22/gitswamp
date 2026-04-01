import type { CommitLintFinding } from "./commitAnalyzerTypes";

const SEVERITY_MULTIPLIER: Record<CommitLintFinding["severity"], number> = {
  error: 3,
  warning: 1.5,
  info: 0.5,
};

export function calculateCommitScore(findings: CommitLintFinding[]): number {
  let deduction = 0;

  for (const finding of findings) {
    deduction += finding.weight * SEVERITY_MULTIPLIER[finding.severity];
  }

  return Math.max(0, Math.min(100, Math.round(100 - deduction)));
}

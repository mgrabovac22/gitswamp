export interface ReleaseNotesCommit {
  sha: string;
  shortSha: string;
  author: string;
  date: string;
  subject: string;
}

interface ReleaseNotesContext {
  sourceRef: string;
  targetRef: string;
  fromSha: string;
  toSha: string;
  generatedAt: Date;
}

type ReleaseSectionKey = "breaking" | "features" | "fixes" | "performance" | "docs" | "other";

interface ReleaseSection {
  key: ReleaseSectionKey;
  title: string;
  commits: ReleaseNotesCommit[];
}

const FIELD_SEPARATOR = "\x1f";
const RECORD_SEPARATOR = "\x1e";

export const RELEASE_NOTES_LOG_FORMAT = "%H%x1f%h%x1f%an%x1f%ad%x1f%s%x1e";

function cleanField(value: string | undefined): string {
  return (value || "").replace(/\s+/g, " ").trim();
}

function markdownEscape(value: string): string {
  return value.replace(/([\\`*_{}\[\]()#+\-.!|>])/g, "\\$1");
}

function sentenceCase(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return trimmed;
  return trimmed.charAt(0).toUpperCase() + trimmed.slice(1);
}

function stripConventionalPrefix(subject: string): { type: string; scope: string; summary: string; breaking: boolean } {
  const match = subject.match(/^([a-z]+)(?:\(([^)]+)\))?(!)?:\s*(.+)$/i);
  if (!match) {
    return { type: "", scope: "", summary: subject.trim(), breaking: /breaking change/i.test(subject) };
  }

  return {
    type: match[1]?.toLowerCase() || "",
    scope: match[2] || "",
    summary: match[4] || subject,
    breaking: Boolean(match[3]) || /breaking change/i.test(subject),
  };
}

function formatScope(scope: string): string {
  if (!scope) return "";
  return `**${markdownEscape(scope)}:** `;
}

function plural(value: number, singular: string, pluralValue = `${singular}s`): string {
  return `${value} ${value === 1 ? singular : pluralValue}`;
}

function sanitizeFilePart(value: string): string {
  const cleaned = value
    .replace(/^origin\//, "")
    .replace(/[^a-z0-9._-]+/gi, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
  return cleaned || "release";
}

export function defaultReleaseNotesFileName(sourceRef: string, targetRef: string, date = new Date()): string {
  const stamp = date.toISOString().slice(0, 10);
  return `release-notes-${sanitizeFilePart(targetRef)}-${sanitizeFilePart(sourceRef)}-${stamp}.md`;
}

export function parseReleaseNotesLog(output: string): ReleaseNotesCommit[] {
  return output
    .split(RECORD_SEPARATOR)
    .map((record) => record.trim())
    .filter(Boolean)
    .map((record) => {
      const [sha, shortSha, author, date, subject] = record.split(FIELD_SEPARATOR);
      return {
        sha: cleanField(sha),
        shortSha: cleanField(shortSha),
        author: cleanField(author) || "Unknown",
        date: cleanField(date),
        subject: cleanField(subject) || "(no subject)",
      };
    })
    .filter((commit) => commit.sha.length > 0);
}

function categoryForCommit(commit: ReleaseNotesCommit): ReleaseSectionKey {
  const subject = commit.subject.trim();
  const lower = subject.toLowerCase();
  const conventional = stripConventionalPrefix(subject);

  if (conventional.breaking || lower.includes("breaking change")) {
    return "breaking";
  }
  if (["feat", "feature"].includes(conventional.type) || /\b(add|added|introduce|introduced|implement|implemented|new)\b/.test(lower)) {
    return "features";
  }
  if (["fix", "bugfix", "hotfix"].includes(conventional.type) || /\b(fix|fixed|bug|resolve|resolved|patch)\b/.test(lower)) {
    return "fixes";
  }
  if (["perf", "performance"].includes(conventional.type) || /\b(performance|optimi[sz]e|faster|speed|cache|memory|ram)\b/.test(lower)) {
    return "performance";
  }
  if (["docs", "doc"].includes(conventional.type) || /\b(documentation|readme|docs)\b/.test(lower)) {
    return "docs";
  }
  return "other";
}

function groupCommits(commits: ReleaseNotesCommit[]): ReleaseSection[] {
  const sectionOrder: Array<{ key: ReleaseSectionKey; title: string }> = [
    { key: "breaking", title: "Breaking Changes" },
    { key: "features", title: "Features" },
    { key: "fixes", title: "Bug Fixes" },
    { key: "performance", title: "Performance" },
    { key: "docs", title: "Documentation" },
    { key: "other", title: "Other Changes" },
  ];

  const grouped = new Map<ReleaseSectionKey, ReleaseNotesCommit[]>();
  for (const commit of commits) {
    const key = categoryForCommit(commit);
    grouped.set(key, [...(grouped.get(key) || []), commit]);
  }

  return sectionOrder.map((section) => ({
    ...section,
    commits: grouped.get(section.key) || [],
  }));
}

function formatCommitLine(commit: ReleaseNotesCommit): string {
  const conventional = stripConventionalPrefix(commit.subject);
  const subject = `${formatScope(conventional.scope)}${markdownEscape(sentenceCase(conventional.summary))}`;
  const author = markdownEscape(commit.author);
  const shortSha = markdownEscape(commit.shortSha || commit.sha.slice(0, 8));
  const date = commit.date ? `, ${markdownEscape(commit.date)}` : "";
  return `- ${subject} (${shortSha}, ${author}${date})`;
}

function commitSummary(commit: ReleaseNotesCommit): string {
  const conventional = stripConventionalPrefix(commit.subject);
  return `${formatScope(conventional.scope)}${markdownEscape(sentenceCase(conventional.summary))}`;
}

function contributorSummary(commits: ReleaseNotesCommit[]): string[] {
  const byAuthor = new Map<string, number>();
  for (const commit of commits) {
    byAuthor.set(commit.author, (byAuthor.get(commit.author) || 0) + 1);
  }

  return Array.from(byAuthor.entries())
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([author, count]) => `- ${markdownEscape(author)}: ${count} commit${count === 1 ? "" : "s"}`);
}

function topValues(values: string[], limit: number): string[] {
  const counts = new Map<string, number>();
  for (const value of values) {
    const cleaned = value.trim();
    if (!cleaned) continue;
    counts.set(cleaned, (counts.get(cleaned) || 0) + 1);
  }

  return Array.from(counts.entries())
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .slice(0, limit)
    .map(([value, count]) => `${markdownEscape(value)} (${count})`);
}

function inlineList(values: string[]): string {
  if (values.length === 0) return "";
  if (values.length === 1) return values[0];
  return `${values.slice(0, -1).join(", ")} and ${values[values.length - 1]}`;
}

function sectionScopes(commits: ReleaseNotesCommit[]): string[] {
  return topValues(
    commits
      .map((commit) => stripConventionalPrefix(commit.subject).scope)
      .filter(Boolean),
    4,
  );
}

function sectionAuthors(commits: ReleaseNotesCommit[]): string[] {
  return topValues(commits.map((commit) => commit.author), 3);
}

function firstDate(commits: ReleaseNotesCommit[]): string {
  return commits
    .map((commit) => commit.date)
    .filter(Boolean)
    .sort()[0] || "";
}

function lastDate(commits: ReleaseNotesCommit[]): string {
  const dates = commits
    .map((commit) => commit.date)
    .filter(Boolean)
    .sort();
  return dates[dates.length - 1] || "";
}

function sectionNarrative(section: ReleaseSection): string[] {
  const scopeText = inlineList(sectionScopes(section.commits));
  const authorText = inlineList(sectionAuthors(section.commits));
  const scopeLine = scopeText ? `Primary areas touched: ${scopeText}.` : "No explicit conventional-commit scope was detected.";
  const authorLine = authorText ? `Main contributors in this area: ${authorText}.` : "";

  switch (section.key) {
    case "breaking":
      return [
        `${plural(section.commits.length, "change")} may require migration, configuration updates, or coordinated rollout.`,
        scopeLine,
        authorLine,
      ].filter(Boolean);
    case "features":
      return [
        `${plural(section.commits.length, "feature")} landed in this merge, focused on new workflow capability or product behavior.`,
        scopeLine,
        authorLine,
      ].filter(Boolean);
    case "fixes":
      return [
        `${plural(section.commits.length, "fix", "fixes")} reduce defects, edge-case failures, or operational noise.`,
        scopeLine,
        authorLine,
      ].filter(Boolean);
    case "performance":
      return [
        `${plural(section.commits.length, "performance change")} target runtime, memory, loading, caching, or responsiveness.`,
        scopeLine,
        authorLine,
      ].filter(Boolean);
    case "docs":
      return [
        `${plural(section.commits.length, "documentation update")} improve project communication or implementation context.`,
        scopeLine,
        authorLine,
      ].filter(Boolean);
    default:
      return [
        `${plural(section.commits.length, "supporting change")} did not fit the primary release categories but should still be reviewed.`,
        scopeLine,
        authorLine,
      ].filter(Boolean);
  }
}

function riskNotes(sections: ReleaseSection[]): string[] {
  const notes: string[] = [];
  const breaking = sections.find((section) => section.key === "breaking")?.commits.length || 0;
  const performance = sections.find((section) => section.key === "performance")?.commits.length || 0;
  const fixes = sections.find((section) => section.key === "fixes")?.commits.length || 0;

  if (breaking > 0) {
    notes.push("- Review breaking changes before rollout and confirm migration steps with consumers.");
  }
  if (performance > 0) {
    notes.push("- Smoke-test the hottest user paths to confirm the performance behavior in production-like data.");
  }
  if (fixes > 0) {
    notes.push("- Prioritize regression checks around the fixed areas listed below.");
  }

  if (notes.length === 0) {
    notes.push("- No explicit high-risk category was detected from commit metadata.");
  }

  return notes;
}

function verificationNotes(sections: ReleaseSection[]): string[] {
  const notes: string[] = [];
  const hasFeatures = (sections.find((section) => section.key === "features")?.commits.length || 0) > 0;
  const hasFixes = (sections.find((section) => section.key === "fixes")?.commits.length || 0) > 0;
  const hasPerformance = (sections.find((section) => section.key === "performance")?.commits.length || 0) > 0;
  const hasBreaking = (sections.find((section) => section.key === "breaking")?.commits.length || 0) > 0;

  if (hasFeatures) {
    notes.push("- Walk through the primary user workflows touched by the feature commits.");
  }
  if (hasFixes) {
    notes.push("- Re-run regression checks around the bug-fix areas and nearby flows.");
  }
  if (hasPerformance) {
    notes.push("- Compare loading, memory, and responsiveness on representative repository data.");
  }
  if (hasBreaking) {
    notes.push("- Confirm migration notes, rollout order, and downstream compatibility before release.");
  }
  if (notes.length === 0) {
    notes.push("- Smoke-test the changed paths listed in the commit appendix.");
  }

  return notes;
}

function highlightCommits(sections: ReleaseSection[]): ReleaseNotesCommit[] {
  const priority: ReleaseSectionKey[] = ["breaking", "features", "fixes", "performance", "docs", "other"];
  return priority
    .flatMap((key) => sections.find((section) => section.key === key)?.commits || [])
    .slice(0, 6);
}

export function buildReleaseNotesMarkdown(commits: ReleaseNotesCommit[], context: ReleaseNotesContext): string {
  const sections = groupCommits(commits);
  const generated = context.generatedAt.toLocaleString();
  const nonEmptySections = sections.filter((section) => section.commits.length > 0);
  const startDate = firstDate(commits);
  const endDate = lastDate(commits);
  const contributors = contributorSummary(commits);
  const commitWindowLine = startDate && endDate
    ? `**Commit window:** ${markdownEscape(startDate)} to ${markdownEscape(endDate)}`
    : "";

  const lines = [
    `# Release Notes`,
    "",
    `**Merge:** ${markdownEscape(context.sourceRef)} -> ${markdownEscape(context.targetRef)}`,
    `**Generated:** ${markdownEscape(generated)}`,
    `**Range:** ${markdownEscape(context.fromSha.slice(0, 12))}..${markdownEscape(context.toSha.slice(0, 12))}`,
    ...(commitWindowLine ? [commitWindowLine] : []),
    "",
    "## Executive Summary",
    "",
    `This release merges ${commits.length} non-merge commit${commits.length === 1 ? "" : "s"} from \`${markdownEscape(context.sourceRef)}\` into \`${markdownEscape(context.targetRef)}\`.`,
    "The changes are grouped below by engineering impact so reviewers can scan product changes, fixes, and operational risk quickly.",
    "",
    "| Area | Count |",
    "| --- | ---: |",
    ...sections.map((section) => `| ${section.title} | ${section.commits.length} |`),
    "",
    "## Release Readiness Notes",
    "",
    ...riskNotes(sections),
    "",
    "## Suggested Verification",
    "",
    ...verificationNotes(sections),
    "",
    "## Contributors",
    "",
    ...(contributors.length > 0 ? contributors : ["- No commit authors detected."]),
    "",
  ];

  if (commits.length === 0) {
    lines.push("## Changes", "", "No non-merge commits were found in this merge range.", "");
    return lines.join("\n");
  }

  const highlights = highlightCommits(sections);
  if (highlights.length > 0) {
    lines.push("## Highlights", "");
    for (const commit of highlights) {
      lines.push(`- ${commitSummary(commit)}`);
    }
    lines.push("");
  }

  for (const section of nonEmptySections) {
    lines.push(`## ${section.title}`, "");
    lines.push(...sectionNarrative(section), "");
    for (const commit of section.commits) {
      lines.push(formatCommitLine(commit));
    }
    lines.push("");
  }

  lines.push("## Commit Appendix", "");
  for (const commit of commits) {
    lines.push(formatCommitLine(commit));
  }
  lines.push("");

  return lines.join("\n");
}

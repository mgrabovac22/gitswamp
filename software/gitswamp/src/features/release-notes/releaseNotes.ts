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

  if (/^[a-z]+(?:\([^)]+\))?!:/.test(lower) || lower.includes("breaking change")) {
    return "breaking";
  }
  if (/^(feat|feature)(?:\([^)]+\))?:/.test(lower) || /\b(add|added|introduce|introduced|implement|implemented|new)\b/.test(lower)) {
    return "features";
  }
  if (/^(fix|bugfix|hotfix)(?:\([^)]+\))?:/.test(lower) || /\b(fix|fixed|bug|resolve|resolved|patch)\b/.test(lower)) {
    return "fixes";
  }
  if (/^(perf|performance)(?:\([^)]+\))?:/.test(lower) || /\b(performance|optimi[sz]e|faster|speed|cache|memory|ram)\b/.test(lower)) {
    return "performance";
  }
  if (/^(docs|doc)(?:\([^)]+\))?:/.test(lower) || /\b(documentation|readme|docs)\b/.test(lower)) {
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
  const subject = markdownEscape(commit.subject);
  const author = markdownEscape(commit.author);
  const shortSha = markdownEscape(commit.shortSha || commit.sha.slice(0, 8));
  const date = commit.date ? `, ${markdownEscape(commit.date)}` : "";
  return `- ${subject} (${shortSha}, ${author}${date})`;
}

export function buildReleaseNotesMarkdown(commits: ReleaseNotesCommit[], context: ReleaseNotesContext): string {
  const sections = groupCommits(commits);
  const generated = context.generatedAt.toLocaleString();
  const nonEmptySections = sections.filter((section) => section.commits.length > 0);

  const lines = [
    `# Release Notes: ${markdownEscape(context.sourceRef)} into ${markdownEscape(context.targetRef)}`,
    "",
    `Generated: ${markdownEscape(generated)}`,
    `Range: ${markdownEscape(context.fromSha.slice(0, 12))}..${markdownEscape(context.toSha.slice(0, 12))}`,
    "",
    "## Summary",
    "",
    `- Commits included: ${commits.length}`,
    ...sections.map((section) => `- ${section.title}: ${section.commits.length}`),
    "",
  ];

  if (commits.length === 0) {
    lines.push("## Changes", "", "No non-merge commits were found in this merge range.", "");
    return lines.join("\n");
  }

  for (const section of nonEmptySections) {
    lines.push(`## ${section.title}`, "");
    for (const commit of section.commits) {
      lines.push(formatCommitLine(commit));
    }
    lines.push("");
  }

  return lines.join("\n");
}

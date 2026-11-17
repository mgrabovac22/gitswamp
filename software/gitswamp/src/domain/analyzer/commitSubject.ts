export const COMMIT_TYPES = ["Fix", "Feat", "Refactor", "Docs", "Style", "Test", "Chore", "Perf", "Build", "Ci", "Revert"] as const;

const HEADER = /^([a-z]+)(?:\(([a-z0-9_./-]+)\))?(!)?:\s*(.*)$/i;
const INLINE_TAG = /^#[a-z0-9][a-z0-9._/-]{0,31}$/i;
const START_TAG = /^(#[a-z0-9][a-z0-9._/-]{0,31})(?:\s+|$)/i;

export function normalizeCommitTag(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return "";
  const tag = trimmed.startsWith("#") ? trimmed : "#" + trimmed;
  return INLINE_TAG.test(tag) ? tag : null;
}

export function closingIssueTag(input: string): string {
  const tag = normalizeCommitTag(input);
  return tag && /^#[1-9][0-9]*$/.test(tag) ? tag : "";
}

export function withClosingIssue(description: string, issue: string): string {
  const body = description.trimEnd();
  if (!closingIssueTag(issue)) return body;
  const existing = new RegExp(`^\\s*(?:close[sd]?|fix(?:es|ed)?|resolve[sd]?):?\\s+${issue}\\s*$`, "i");
  const clean = body.split("\n").filter(line => !existing.test(line)).join("\n").trimEnd();
  return clean ? `${clean}\n\nCloses ${issue}` : `Closes ${issue}`;
}

export function splitClosingIssue(description: string, issue: string) {
  const body = description.trimEnd();
  const suffix = closingIssueTag(issue) && `Closes ${issue}`;
  if (suffix && (body === suffix || body.endsWith(`\n\n${suffix}`))) {
    return { body: body.slice(0, -suffix.length).trimEnd(), issue };
  }
  return { body, issue: "" };
}

export function splitCommitSubject(subject: string) {
  const text = subject.trim();
  const header = HEADER.exec(text);
  const body = header?.[4] ?? text;
  const tagged = START_TAG.exec(body);
  const summary = tagged ? body.slice(tagged[0].length) : body;
  return {
    type: COMMIT_TYPES.find(type => type.toLowerCase() === header?.[1].toLowerCase()) ?? "Chore",
    scope: header?.[2] ?? "",
    breaking: !!header?.[3],
    tag: tagged?.[1] ?? "",
    summary,
    subject: header ? text.slice(0, text.length - body.length) + summary : summary,
  };
}

export function formatCommitSubject(type: string, scope: string, breaking: boolean, summary: string, tag = "") {
  const cleanScope = scope.trim();
  return `${type}${cleanScope ? `(${cleanScope})` : ""}${breaking ? "!" : ""}: ${[tag, summary.trim()].filter(Boolean).join(" ")}`;
}

export function withCommitTag(subject: string, tag: string) {
  const clean = subject.trim();
  if (!tag) return clean;
  const parts = splitCommitSubject(clean);
  const header = HEADER.exec(clean);
  return header
    ? `${clean.slice(0, clean.indexOf(":") + 1)} ${tag} ${parts.summary}`.trimEnd()
    : `${tag} ${parts.summary}`.trimEnd();
}

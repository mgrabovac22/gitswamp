import type { DiffSummary } from "./commitAnalyzerTypes";

const DIFF_FILE_REGEX = /^diff --git a\/(.+?) b\/(.+)$/;
const LINE_ADDED_REGEX = /^\+(?!\+\+)/;
const LINE_REMOVED_REGEX = /^-(?!---)/;

const EMPTY_DIFF_SUMMARY: DiffSummary = {
  totalLinesAdded: 0,
  totalLinesRemoved: 0,
  filesChanged: 0,
  fileTypes: [],
  hasTestChanges: false,
  hasMigrationChanges: false,
  inferredScope: "general",
};

export function parseDiff(rawDiff: string): DiffSummary {
  if (!rawDiff.trim()) {
    return { ...EMPTY_DIFF_SUMMARY };
  }

  let totalLinesAdded = 0;
  let totalLinesRemoved = 0;
  const paths: string[] = [];

  for (const line of rawDiff.split("\n")) {
    const fileMatch = DIFF_FILE_REGEX.exec(line);
    if (fileMatch) {
      paths.push(fileMatch[2]);
      continue;
    }
    if (LINE_ADDED_REGEX.test(line)) {
      totalLinesAdded += 1;
      continue;
    }
    if (LINE_REMOVED_REGEX.test(line)) {
      totalLinesRemoved += 1;
    }
  }

  return buildDiffSummaryFromPaths(paths, {
    totalLinesAdded,
    totalLinesRemoved,
    filesChanged: paths.length,
  });
}

export function buildDiffSummaryFromPaths(paths: string[], seed?: Partial<DiffSummary>): DiffSummary {
  const normalizedPaths = Array.from(
    new Set(paths.map((value) => value.split("\\").join("/").trim()).filter((value) => value.length > 0)),
  );

  const fileTypes = new Set<string>();
  let hasTestChanges = false;
  let hasMigrationChanges = false;

  for (const path of normalizedPaths) {
    const lower = path.toLowerCase();

    const dotIndex = path.lastIndexOf(".");
    if (dotIndex > 0 && dotIndex < path.length - 1) {
      fileTypes.add(path.slice(dotIndex).toLowerCase());
    }

    if (
      lower.includes("/test/")
      || lower.includes("/tests/")
      || lower.includes(".test.")
      || lower.includes(".spec.")
    ) {
      hasTestChanges = true;
    }

    if (
      lower.includes("/migration/")
      || lower.includes("/migrations/")
      || lower.includes("migrate")
    ) {
      hasMigrationChanges = true;
    }
  }

  return {
    totalLinesAdded: seed?.totalLinesAdded ?? 0,
    totalLinesRemoved: seed?.totalLinesRemoved ?? 0,
    filesChanged: seed?.filesChanged ?? normalizedPaths.length,
    fileTypes: Array.from(fileTypes).sort((left, right) => left.localeCompare(right)),
    hasTestChanges,
    hasMigrationChanges,
    inferredScope: inferScopeFromPaths(normalizedPaths),
  };
}

export function normalizeDiffSummary(summary?: Partial<DiffSummary>): DiffSummary {
  if (!summary) {
    return { ...EMPTY_DIFF_SUMMARY };
  }

  return {
    totalLinesAdded: summary.totalLinesAdded ?? 0,
    totalLinesRemoved: summary.totalLinesRemoved ?? 0,
    filesChanged: summary.filesChanged ?? 0,
    fileTypes: summary.fileTypes
      ? Array.from(new Set(summary.fileTypes)).sort((left, right) => left.localeCompare(right))
      : [],
    hasTestChanges: summary.hasTestChanges ?? false,
    hasMigrationChanges: summary.hasMigrationChanges ?? false,
    inferredScope: summary.inferredScope?.trim() || "general",
  };
}

export function inferScopeFromPaths(paths: string[]): string {
  const scores = new Map<string, number>();

  for (const rawPath of paths) {
    const path = rawPath.toLowerCase();
    const candidates = resolveScopeCandidates(path);
    for (const candidate of candidates) {
      scores.set(candidate, (scores.get(candidate) || 0) + 1);
    }
  }

  if (scores.size === 0) {
    return "general";
  }

  let bestScope = "general";
  let bestScore = -1;
  for (const [scope, score] of scores.entries()) {
    if (score > bestScore) {
      bestScope = scope;
      bestScore = score;
    }
  }

  return bestScope;
}

function resolveScopeCandidates(path: string): string[] {
  const scopes: string[] = [];

  if (path.includes("/auth/")) scopes.push("auth");
  if (path.includes("/api/") || path.includes("/routes/") || path.includes("/controllers/")) scopes.push("api");
  if (path.includes("/ui/") || path.includes("/view/") || path.includes("/components/")) scopes.push("ui");
  if (path.includes("/styles/") || path.endsWith(".css") || path.endsWith(".scss")) scopes.push("style");
  if (path.includes("/docs/") || path.endsWith(".md")) scopes.push("docs");
  if (path.includes("/db/") || path.includes("migration")) scopes.push("data");
  if (path.includes("/test/") || path.includes("/tests/") || path.includes(".test.") || path.includes(".spec.")) scopes.push("tests");

  if (scopes.length === 0) {
    scopes.push("general");
  }

  return scopes;
}

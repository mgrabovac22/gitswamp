export type PickaxeFileChangeKind = "added" | "deleted" | "modified" | "renamed" | "copied" | "typechange" | "unknown";
export type PickaxeSnippetKind = "added" | "deleted" | "context";

export interface PickaxeFileChange {
  path: string;
  oldPath?: string;
  status: string;
  kind: PickaxeFileChangeKind;
}

export interface PickaxeCommitHit {
  sha: string;
  shortSha: string;
  author: string;
  date: string;
  timestamp: number;
  subject: string;
  files: PickaxeFileChange[];
}

export interface PickaxeSnippetLine {
  id: string;
  file: string;
  oldLine?: number;
  newLine?: number;
  kind: PickaxeSnippetKind;
  text: string;
}

function classifyStatus(status: string): PickaxeFileChangeKind {
  const code = status.charAt(0).toUpperCase();
  if (code === "A") return "added";
  if (code === "D") return "deleted";
  if (code === "M") return "modified";
  if (code === "R") return "renamed";
  if (code === "C") return "copied";
  if (code === "T") return "typechange";
  return "unknown";
}

function decodeGitPath(path: string): string {
  const trimmed = path.trim();
  if (trimmed.length < 2 || !trimmed.startsWith("\"") || !trimmed.endsWith("\"")) {
    return trimmed;
  }

  try {
    return JSON.parse(trimmed) as string;
  } catch {
    return trimmed
      .slice(1, -1)
      .replace(/\\"/g, "\"")
      .replace(/\\\\/g, "\\");
  }
}

export function parsePickaxeLog(output: string): PickaxeCommitHit[] {
  return output
    .split("\x1e")
    .map((record) => record.trim())
    .filter(Boolean)
    .map((record) => {
      const lines = record.split(/\r?\n/);
      const header = (lines.shift() || "").split("\x1f");
      const files: PickaxeFileChange[] = [];

      for (const line of lines) {
        const parts = line.split("\t").filter(Boolean);
        if (parts.length < 2) continue;

        const status = parts[0];
        const kind = classifyStatus(status);
        if (kind === "renamed" || kind === "copied") {
          files.push({
            path: decodeGitPath(parts[2] || parts[1]),
            oldPath: decodeGitPath(parts[1]),
            status,
            kind,
          });
          continue;
        }

        files.push({
          path: decodeGitPath(parts[1]),
          status,
          kind,
        });
      }

      return {
        sha: header[0] || "",
        shortSha: header[1] || (header[0] || "").slice(0, 8),
        author: header[2] || "Unknown",
        timestamp: Number(header[3] || "0") || 0,
        date: header[4] || "",
        subject: header.slice(5).join("\x1f") || "(no subject)",
        files,
      };
    })
    .filter((hit) => hit.sha);
}

function parseHunkHeader(line: string): { oldLine: number; newLine: number } | null {
  const match = line.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
  if (!match) return null;
  return {
    oldLine: Number(match[1]),
    newLine: Number(match[2]),
  };
}

function normalizeNeedle(value: string, caseSensitive: boolean): string {
  return caseSensitive ? value : value.toLowerCase();
}

export function parsePickaxePatchSnippets(
  patch: string,
  query: string,
  caseSensitive = true,
  maxLines = 180,
): PickaxeSnippetLine[] {
  const needle = normalizeNeedle(query, caseSensitive);
  if (!needle) return [];

  const snippets: PickaxeSnippetLine[] = [];
  let file = "";
  let oldLine = 0;
  let newLine = 0;
  let inHunk = false;

  for (const rawLine of patch.split(/\r?\n/)) {
    if (snippets.length >= maxLines) break;

    if (rawLine.startsWith("diff --git ")) {
      const match = rawLine.match(/^diff --git a\/(.+?) b\/(.+)$/);
      file = match?.[2] || file;
      inHunk = false;
      continue;
    }

    if (rawLine.startsWith("+++ b/")) {
      file = rawLine.slice(6);
      continue;
    }

    const hunk = parseHunkHeader(rawLine);
    if (hunk) {
      oldLine = hunk.oldLine;
      newLine = hunk.newLine;
      inHunk = true;
      continue;
    }

    if (!inHunk || !file) {
      continue;
    }

    const marker = rawLine.charAt(0);
    if (marker !== "+" && marker !== "-" && marker !== " ") {
      continue;
    }

    const text = rawLine.slice(1);
    const haystack = normalizeNeedle(text, caseSensitive);
    const matches = haystack.includes(needle);

    if (marker === "+") {
      if (matches && !rawLine.startsWith("+++")) {
        snippets.push({
          id: `${file}:${newLine}:add:${snippets.length}`,
          file,
          newLine,
          kind: "added",
          text,
        });
      }
      newLine += 1;
      continue;
    }

    if (marker === "-") {
      if (matches && !rawLine.startsWith("---")) {
        snippets.push({
          id: `${file}:${oldLine}:del:${snippets.length}`,
          file,
          oldLine,
          kind: "deleted",
          text,
        });
      }
      oldLine += 1;
      continue;
    }

    if (matches) {
      snippets.push({
        id: `${file}:${oldLine}:${newLine}:ctx:${snippets.length}`,
        file,
        oldLine,
        newLine,
        kind: "context",
        text,
      });
    }
    oldLine += 1;
    newLine += 1;
  }

  return snippets;
}

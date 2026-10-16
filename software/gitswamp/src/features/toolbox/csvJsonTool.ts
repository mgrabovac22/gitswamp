export function csvToJson(input: string): { json: string; error: string | null } {
  if (!input.trim()) return { json: "[]", error: null };
  try {
    const lines = input.replace(/\r\n/g, "\n").split("\n").filter((l) => l.trim().length > 0);
    if (!lines.length) return { json: "[]", error: null };
    const headers = splitCsvLine(lines[0]);
    const rows: Record<string, string>[] = [];
    for (let i = 1; i < lines.length; i += 1) {
      const cols = splitCsvLine(lines[i]);
      const row: Record<string, string> = {};
      headers.forEach((h, idx) => {
        row[h || `col${idx + 1}`] = cols[idx] ?? "";
      });
      rows.push(row);
    }
    return { json: JSON.stringify(rows, null, 2), error: null };
  } catch (err) {
    return { json: "", error: err instanceof Error ? err.message : String(err) };
  }
}

function splitCsvLine(line: string): string[] {
  const result: string[] = [];
  let current = "";
  let inQuotes = false;
  for (let i = 0; i < line.length; i += 1) {
    const c = line[i];
    if (c === '"') {
      if (inQuotes && line[i + 1] === '"') {
        current += '"';
        i += 1;
      } else {
        inQuotes = !inQuotes;
      }
    } else if (c === "," && !inQuotes) {
      result.push(current);
      current = "";
    } else {
      current += c;
    }
  }
  result.push(current);
  return result.map((v) => v.trim());
}

export function jsonToCsv(input: string): { csv: string; error: string | null } {
  if (!input.trim()) return { csv: "", error: null };
  try {
    const data = JSON.parse(input);
    if (!Array.isArray(data) || !data.length) return { csv: "", error: "Need a non-empty array of objects." };
    const headers = Array.from(new Set(data.flatMap((row) => (row && typeof row === "object" ? Object.keys(row) : []))));
    const escape = (v: unknown) => {
      if (v === null || v === undefined) return "";
      const s = String(v);
      if (s.includes(",") || s.includes('"') || s.includes("\n") || s.includes("\r")) {
        return `"${s.replace(/"/g, '""')}"`;
      }
      return s;
    };
    const lines = [headers.map(escape).join(",")];
    for (const row of data) {
      if (!row || typeof row !== "object") continue;
      lines.push(headers.map((h) => escape((row as Record<string, unknown>)[h])).join(","));
    }
    return { csv: lines.join("\n"), error: null };
  } catch (err) {
    return { csv: "", error: err instanceof Error ? err.message : String(err) };
  }
}

export type DiffKind = "equal" | "add" | "remove";

export interface DiffLine {
  kind: DiffKind;
  text: string;
  leftNo: number | null;
  rightNo: number | null;
}

export interface DiffOptions {
  ignoreCase?: boolean;
  ignoreWhitespace?: boolean;
}

export interface DiffStats {
  added: number;
  removed: number;
  unchanged: number;
}

const MAX_CELLS = 4_000_000;

function normalize(line: string, options: DiffOptions): string {
  let value = line;
  if (options.ignoreWhitespace) value = value.replace(/\s+/g, " ").trim();
  if (options.ignoreCase) value = value.toLowerCase();
  return value;
}

export function computeLineDiff(left: string, right: string, options: DiffOptions = {}): DiffLine[] {
  const leftLines = left.length ? left.replace(/\r\n/g, "\n").split("\n") : [];
  const rightLines = right.length ? right.replace(/\r\n/g, "\n").split("\n") : [];
  const n = leftLines.length;
  const m = rightLines.length;

  if (n === 0 && m === 0) return [];

  const leftNorm = leftLines.map((line) => normalize(line, options));
  const rightNorm = rightLines.map((line) => normalize(line, options));

  if (n * m > MAX_CELLS) {
    const out: DiffLine[] = [];
    leftLines.forEach((text, index) => out.push({ kind: "remove", text, leftNo: index + 1, rightNo: null }));
    rightLines.forEach((text, index) => out.push({ kind: "add", text, leftNo: null, rightNo: index + 1 }));
    return out;
  }

  const width = m + 1;
  const table = new Uint32Array((n + 1) * width);
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      if (leftNorm[i] === rightNorm[j]) {
        table[i * width + j] = table[(i + 1) * width + (j + 1)] + 1;
      } else {
        const down = table[(i + 1) * width + j];
        const rightCell = table[i * width + (j + 1)];
        table[i * width + j] = down >= rightCell ? down : rightCell;
      }
    }
  }

  const result: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (leftNorm[i] === rightNorm[j]) {
      result.push({ kind: "equal", text: leftLines[i], leftNo: i + 1, rightNo: j + 1 });
      i += 1;
      j += 1;
    } else if (table[(i + 1) * width + j] >= table[i * width + (j + 1)]) {
      result.push({ kind: "remove", text: leftLines[i], leftNo: i + 1, rightNo: null });
      i += 1;
    } else {
      result.push({ kind: "add", text: rightLines[j], leftNo: null, rightNo: j + 1 });
      j += 1;
    }
  }
  while (i < n) {
    result.push({ kind: "remove", text: leftLines[i], leftNo: i + 1, rightNo: null });
    i += 1;
  }
  while (j < m) {
    result.push({ kind: "add", text: rightLines[j], leftNo: null, rightNo: j + 1 });
    j += 1;
  }
  return result;
}

export function diffStats(lines: DiffLine[]): DiffStats {
  return {
    added: lines.filter((line) => line.kind === "add").length,
    removed: lines.filter((line) => line.kind === "remove").length,
    unchanged: lines.filter((line) => line.kind === "equal").length,
  };
}

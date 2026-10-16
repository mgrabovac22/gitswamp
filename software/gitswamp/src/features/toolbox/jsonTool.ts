export interface JsonParseResult {
  value: unknown;
  error: string | null;
}

export function parseJson(text: string): JsonParseResult {
  if (!text.trim()) return { value: undefined, error: null };
  try {
    return { value: JSON.parse(text), error: null };
  } catch (err) {
    return { value: undefined, error: err instanceof Error ? err.message : String(err) };
  }
}

export function formatJson(value: unknown, indent = 2): string {
  return JSON.stringify(value, null, indent);
}

export function minifyJson(value: unknown): string {
  return JSON.stringify(value);
}

function sortValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortValue);
  if (value && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>).sort(([a], [b]) => a.localeCompare(b));
    const out: Record<string, unknown> = {};
    for (const [key, child] of entries) out[key] = sortValue(child);
    return out;
  }
  return value;
}

export function sortJsonKeys(value: unknown): string {
  return JSON.stringify(sortValue(value), null, 2);
}

export interface JsonStats {
  type: string;
  size: number;
  keys: number;
  depth: number;
}

function measureDepth(value: unknown): number {
  if (Array.isArray(value)) {
    let max = 0;
    for (const item of value) max = Math.max(max, measureDepth(item));
    return max + 1;
  }
  if (value && typeof value === "object") {
    let max = 0;
    for (const item of Object.values(value as Record<string, unknown>)) max = Math.max(max, measureDepth(item));
    return max + 1;
  }
  return 0;
}

export function jsonStats(text: string, value: unknown): JsonStats {
  let type = "unknown";
  let keys = 0;
  if (Array.isArray(value)) {
    type = "array";
  } else if (value !== null && typeof value === "object") {
    type = "object";
    keys = Object.keys(value as Record<string, unknown>).length;
  } else if (value === null) {
    type = "null";
  } else {
    type = typeof value;
  }
  return { type, size: text.length, keys, depth: measureDepth(value) };
}

export interface JsonQueryResult {
  found: boolean;
  value: unknown;
  error: string | null;
  path: string;
}

export function queryJsonPath(root: unknown, path: string): JsonQueryResult {
  const clean = path.trim();
  if (!clean) return { found: true, value: root, error: null, path: clean };

  const tokens: (string | number)[] = [];
  const pattern = /([^.[\]]+)|\[(\d+)\]/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(clean)) !== null) {
    if (match[1] !== undefined) tokens.push(match[1]);
    else if (match[2] !== undefined) tokens.push(Number(match[2]));
  }

  let current: unknown = root;
  for (const token of tokens) {
    if (current === null || current === undefined) {
      return { found: false, value: undefined, error: `Cannot read “${String(token)}” of ${String(current)}`, path: clean };
    }
    if (typeof token === "number") {
      if (!Array.isArray(current)) {
        return { found: false, value: undefined, error: `“${clean}” expects an array but found ${typeof current}`, path: clean };
      }
      current = current[token];
    } else {
      if (typeof current !== "object" || Array.isArray(current)) {
        return { found: false, value: undefined, error: `“${clean}” expects an object but found ${Array.isArray(current) ? "array" : typeof current}`, path: clean };
      }
      current = (current as Record<string, unknown>)[token];
    }
    if (current === undefined) {
      return { found: false, value: undefined, error: `Path not found: ${clean}`, path: clean };
    }
  }
  return { found: true, value: current, error: null, path: clean };
}

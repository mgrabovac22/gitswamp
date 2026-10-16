import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

export type ToolId =
  | "regex"
  | "json"
  | "url"
  | "ids"
  | "hash"
  | "text"
  | "diff"
  | "cron"
  | "jwt"
  | "schema"
  | "color"
  | "base"
  | "csv"
  | "slug"
  | "html"
  | "similar";

export type SnippetToolId = Exclude<ToolId, "regex">;

export interface RegexHistoryEntry {
  id: string;
  name: string;
  pattern: string;
  flags: string;
  at: number;
}

export interface SnippetHistoryEntry {
  id: string;
  label: string;
  text: string;
  at: number;
}

const HISTORY_LIMIT = 30;

const KEYS: Record<ToolId, string> = {
  regex: "gitswamp-toolbox-regex-history",
  json: "gitswamp-toolbox-json-history",
  url: "gitswamp-toolbox-url-history",
  ids: "gitswamp-toolbox-ids-history",
  hash: "gitswamp-toolbox-hash-history",
  text: "gitswamp-toolbox-text-history",
  diff: "gitswamp-toolbox-diff-history",
  cron: "gitswamp-toolbox-cron-history",
  jwt: "gitswamp-toolbox-jwt-history",
  schema: "gitswamp-toolbox-schema-history",
  color: "gitswamp-toolbox-color-history",
  base: "gitswamp-toolbox-base-history",
  csv: "gitswamp-toolbox-csv-history",
  slug: "gitswamp-toolbox-slug-history",
  html: "gitswamp-toolbox-html-history",
  similar: "gitswamp-toolbox-similar-history",
};

export function generateToolboxId(prefix = "tb"): string {
  try {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
      return `${prefix}-${crypto.randomUUID()}`;
    }
  } catch {
    // fall through
  }
  return `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function readArray<T>(key: string): T[] {
  const raw = safeStorageGet(key);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as T[]) : [];
  } catch {
    return [];
  }
}

function writeArray<T>(key: string, entries: T[]): void {
  try {
    safeStorageSet(key, JSON.stringify(entries));
  } catch {
    // ignore
  }
}

function sanitizeRegexEntry(entry: unknown): RegexHistoryEntry | null {
  if (!entry || typeof entry !== "object") return null;
  const value = entry as Partial<RegexHistoryEntry>;
  if (typeof value.pattern !== "string") return null;
  return {
    id: typeof value.id === "string" && value.id ? value.id : generateToolboxId("regex"),
    name: typeof value.name === "string" && value.name ? value.name : value.pattern,
    pattern: value.pattern,
    flags: typeof value.flags === "string" ? value.flags : "",
    at: typeof value.at === "number" ? value.at : Date.now(),
  };
}

function sanitizeSnippetEntry(entry: unknown): SnippetHistoryEntry | null {
  if (!entry || typeof entry !== "object") return null;
  const value = entry as Partial<SnippetHistoryEntry>;
  if (typeof value.text !== "string") return null;
  return {
    id: typeof value.id === "string" && value.id ? value.id : generateToolboxId("snip"),
    label: typeof value.label === "string" && value.label ? value.label : value.text.slice(0, 60),
    text: value.text,
    at: typeof value.at === "number" ? value.at : Date.now(),
  };
}

export function getRegexHistory(): RegexHistoryEntry[] {
  return readArray<unknown>(KEYS.regex)
    .map(sanitizeRegexEntry)
    .filter((entry): entry is RegexHistoryEntry => entry !== null)
    .sort((a, b) => b.at - a.at)
    .slice(0, HISTORY_LIMIT);
}

export function addRegexHistory(entry: RegexHistoryEntry): RegexHistoryEntry[] {
  const rest = getRegexHistory().filter((item) => !(item.pattern === entry.pattern && item.flags === entry.flags));
  const next = [entry, ...rest].slice(0, HISTORY_LIMIT);
  writeArray(KEYS.regex, next);
  return next;
}

export function deleteRegexHistory(id: string): RegexHistoryEntry[] {
  const next = getRegexHistory().filter((item) => item.id !== id);
  writeArray(KEYS.regex, next);
  return next;
}

export function clearRegexHistory(): RegexHistoryEntry[] {
  writeArray(KEYS.regex, []);
  return [];
}

export function getSnippetHistory(tool: SnippetToolId): SnippetHistoryEntry[] {
  return readArray<unknown>(KEYS[tool])
    .map(sanitizeSnippetEntry)
    .filter((entry): entry is SnippetHistoryEntry => entry !== null)
    .sort((a, b) => b.at - a.at)
    .slice(0, HISTORY_LIMIT);
}

export function addSnippetHistory(tool: SnippetToolId, entry: SnippetHistoryEntry): SnippetHistoryEntry[] {
  const rest = getSnippetHistory(tool).filter((item) => item.text !== entry.text);
  const next = [entry, ...rest].slice(0, HISTORY_LIMIT);
  writeArray(KEYS[tool], next);
  return next;
}

export function deleteSnippetHistory(tool: SnippetToolId, id: string): SnippetHistoryEntry[] {
  const next = getSnippetHistory(tool).filter((item) => item.id !== id);
  writeArray(KEYS[tool], next);
  return next;
}

export function clearSnippetHistory(tool: SnippetToolId): SnippetHistoryEntry[] {
  writeArray(KEYS[tool], []);
  return [];
}

export function formatHistoryTime(at: number): string {
  try {
    return new Date(at).toLocaleString();
  } catch {
    return "";
  }
}

const REGEX_DRAFT_KEY = "gitswamp-toolbox-regex-draft";

export function getRegexDraft<T>(): T | null {
  const raw = safeStorageGet(REGEX_DRAFT_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

export function setRegexDraft(value: unknown): void {
  try {
    safeStorageSet(REGEX_DRAFT_KEY, JSON.stringify(value));
  } catch {
    // ignore
  }
}

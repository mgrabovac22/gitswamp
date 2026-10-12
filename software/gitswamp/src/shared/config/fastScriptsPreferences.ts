import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

export const FAST_SCRIPTS_STORAGE_KEY = "gitswamp-fast-scripts";
export const FAST_SCRIPT_SEQUENCES_STORAGE_KEY = "gitswamp-fast-script-sequences";
export const FAST_SCRIPT_RUNS_STORAGE_KEY = "gitswamp-fast-script-runs";
export const FAST_SCRIPTS_EVENT = "gitswamp-fast-scripts-changed";

export type FastScriptType = "shell" | "python" | "node" | "powershell" | "cmd" | "custom";
export type FastScriptScope = "repo" | "global";

export interface FastScript {
  id: string;
  name: string;
  type: FastScriptType;
  scope: FastScriptScope;
  repoPath: string;
  body: string;
  createdAt: number;
  updatedAt: number;
}

export type FastScriptDraft = Pick<FastScript, "name" | "type" | "scope" | "body">;

export interface FastScriptSequence {
  id: string;
  name: string;
  scope: FastScriptScope;
  repoPath: string;
  scriptIds: string[];
  createdAt: number;
  updatedAt: number;
}

export type FastScriptSequenceDraft = Pick<FastScriptSequence, "name" | "scope" | "scriptIds">;

export interface FastScriptRunStat {
  count: number;
  lastRunAt: number;
}

export const FAST_SCRIPT_TYPE_OPTIONS: {
  value: FastScriptType;
  label: string;
  hint: string;
}[] = [
  { value: "shell", label: "Shell", hint: "Terminal commands (bash / sh / cmd) run in the repository" },
  { value: "python", label: "Python", hint: "Saved to a temp file and run with python3 / python" },
  { value: "node", label: "Node.js", hint: "Saved to a temp file and run with node" },
  { value: "powershell", label: "PowerShell", hint: "Saved to a .ps1 file and run with pwsh / powershell" },
  { value: "cmd", label: "Batch (CMD)", hint: "Saved to a .cmd file and run with cmd on Windows" },
  { value: "custom", label: "Custom command", hint: "Executed verbatim in the platform shell" },
];

const VALID_TYPES: readonly FastScriptType[] = ["shell", "python", "node", "powershell", "cmd", "custom"];
const VALID_SCOPES: readonly FastScriptScope[] = ["repo", "global"];

export function generateFastScriptId(): string {
  const stamp = Date.now().toString(36);
  const random = Math.random().toString(36).slice(2, 8);
  return `fs_${stamp}_${random}`;
}

export function deriveFastScriptName(draft: FastScriptDraft): string {
  const trimmed = draft.name.trim();
  if (trimmed) {
    return trimmed;
  }

  const firstLine = draft.body
    .split("\n")
    .map((line) => line.trim())
    .find((line) => line.length > 0 && !line.startsWith("#") && !line.startsWith("//"));

  if (firstLine) {
    return firstLine.length > 42 ? `${firstLine.slice(0, 39)}...` : firstLine;
  }

  const typeLabel = FAST_SCRIPT_TYPE_OPTIONS.find((option) => option.value === draft.type)?.label ?? "Script";
  return `Untitled ${typeLabel}`;
}

function sanitizeFastScript(raw: unknown): FastScript | null {
  if (!raw || typeof raw !== "object") {
    return null;
  }

  const value = raw as Record<string, unknown>;
  const body = typeof value.body === "string" ? value.body : "";
  const type = VALID_TYPES.includes(value.type as FastScriptType) ? (value.type as FastScriptType) : "shell";
  const scope = VALID_SCOPES.includes(value.scope as FastScriptScope) ? (value.scope as FastScriptScope) : "repo";
  const repoPath = typeof value.repoPath === "string" ? value.repoPath : "";
  const name = typeof value.name === "string" ? value.name.trim() : "";
  const id = typeof value.id === "string" && value.id.trim() ? value.id : generateFastScriptId();
  const createdAt = typeof value.createdAt === "number" && Number.isFinite(value.createdAt) ? value.createdAt : Date.now();
  const updatedAt = typeof value.updatedAt === "number" && Number.isFinite(value.updatedAt) ? value.updatedAt : createdAt;

  if (!body.trim()) {
    return null;
  }

  return {
    id,
    name: name || deriveFastScriptName({ name, type, scope, body }),
    type,
    scope,
    repoPath,
    body,
    createdAt,
    updatedAt,
  };
}

export function sanitizeFastScripts(raw: unknown): FastScript[] {
  if (!Array.isArray(raw)) {
    return [];
  }

  return raw
    .map((entry) => sanitizeFastScript(entry))
    .filter((entry): entry is FastScript => entry !== null);
}

export function getStoredFastScripts(): FastScript[] {
  const stored = safeStorageGet(FAST_SCRIPTS_STORAGE_KEY);
  if (!stored) {
    return [];
  }

  try {
    return sanitizeFastScripts(JSON.parse(stored));
  } catch {
    return [];
  }
}

function persist(scripts: FastScript[]): void {
  safeStorageSet(FAST_SCRIPTS_STORAGE_KEY, JSON.stringify(scripts));
  globalThis.dispatchEvent(new CustomEvent(FAST_SCRIPTS_EVENT, { detail: scripts }));
}

export function storeFastScripts(scripts: FastScript[]): void {
  persist(sanitizeFastScripts(scripts));
}

export function addFastScript(draft: FastScriptDraft, repoPath: string): FastScript {
  const now = Date.now();
  const script: FastScript = {
    id: generateFastScriptId(),
    name: deriveFastScriptName(draft),
    type: draft.type,
    scope: draft.scope,
    repoPath: draft.scope === "repo" ? repoPath : "",
    body: draft.body,
    createdAt: now,
    updatedAt: now,
  };

  persist([script, ...getStoredFastScripts()]);
  return script;
}

export function updateFastScript(id: string, draft: FastScriptDraft, repoPath: string): void {
  const next = getStoredFastScripts().map((script) => {
    if (script.id !== id) {
      return script;
    }

    return {
      ...script,
      name: deriveFastScriptName(draft),
      type: draft.type,
      scope: draft.scope,
      repoPath: draft.scope === "repo" ? (repoPath || script.repoPath) : "",
      body: draft.body,
      updatedAt: Date.now(),
    };
  });

  persist(next);
}

export function deleteFastScript(id: string): void {
  persist(getStoredFastScripts().filter((script) => script.id !== id));
}

export function scriptsForRepo(scripts: FastScript[], repoPath: string): FastScript[] {
  return scripts.filter((script) => script.scope === "global" || (repoPath && script.repoPath === repoPath));
}

function sanitizeSequence(raw: unknown): FastScriptSequence | null {
  if (!raw || typeof raw !== "object") {
    return null;
  }

  const value = raw as Record<string, unknown>;
  const scope = VALID_SCOPES.includes(value.scope as FastScriptScope) ? (value.scope as FastScriptScope) : "repo";
  const repoPath = typeof value.repoPath === "string" ? value.repoPath : "";
  const name = typeof value.name === "string" ? value.name.trim() : "";
  const id = typeof value.id === "string" && value.id.trim() ? value.id : generateFastScriptId();
  const createdAt = typeof value.createdAt === "number" && Number.isFinite(value.createdAt) ? value.createdAt : Date.now();
  const updatedAt = typeof value.updatedAt === "number" && Number.isFinite(value.updatedAt) ? value.updatedAt : createdAt;
  const scriptIds = Array.isArray(value.scriptIds)
    ? value.scriptIds.filter((entry): entry is string => typeof entry === "string" && entry.trim().length > 0)
    : [];

  if (scriptIds.length === 0) {
    return null;
  }

  return {
    id,
    name: name || "Untitled sequence",
    scope,
    repoPath,
    scriptIds,
    createdAt,
    updatedAt,
  };
}

export function getStoredSequences(): FastScriptSequence[] {
  const stored = safeStorageGet(FAST_SCRIPT_SEQUENCES_STORAGE_KEY);
  if (!stored) {
    return [];
  }

  try {
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed
      .map((entry) => sanitizeSequence(entry))
      .filter((entry): entry is FastScriptSequence => entry !== null);
  } catch {
    return [];
  }
}

function persistSequences(sequences: FastScriptSequence[]): void {
  safeStorageSet(FAST_SCRIPT_SEQUENCES_STORAGE_KEY, JSON.stringify(sequences));
  globalThis.dispatchEvent(new CustomEvent(FAST_SCRIPTS_EVENT, { detail: sequences }));
}

export function addFastScriptSequence(draft: FastScriptSequenceDraft, repoPath: string): FastScriptSequence {
  const now = Date.now();
  const sequence: FastScriptSequence = {
    id: generateFastScriptId(),
    name: draft.name.trim() || "Untitled sequence",
    scope: draft.scope,
    repoPath: draft.scope === "repo" ? repoPath : "",
    scriptIds: [...draft.scriptIds],
    createdAt: now,
    updatedAt: now,
  };

  persistSequences([sequence, ...getStoredSequences()]);
  return sequence;
}

export function updateFastScriptSequence(id: string, draft: FastScriptSequenceDraft, repoPath: string): void {
  const next = getStoredSequences().map((sequence) => {
    if (sequence.id !== id) {
      return sequence;
    }

    return {
      ...sequence,
      name: draft.name.trim() || sequence.name,
      scope: draft.scope,
      repoPath: draft.scope === "repo" ? (repoPath || sequence.repoPath) : "",
      scriptIds: [...draft.scriptIds],
      updatedAt: Date.now(),
    };
  });

  persistSequences(next);
}

export function deleteFastScriptSequence(id: string): void {
  persistSequences(getStoredSequences().filter((sequence) => sequence.id !== id));
}

export function sequencesForRepo(sequences: FastScriptSequence[], repoPath: string): FastScriptSequence[] {
  return sequences.filter((sequence) => sequence.scope === "global" || (repoPath && sequence.repoPath === repoPath));
}

export function getFastScriptRunStats(): Record<string, FastScriptRunStat> {
  const stored = safeStorageGet(FAST_SCRIPT_RUNS_STORAGE_KEY);
  if (!stored) {
    return {};
  }

  try {
    const parsed = JSON.parse(stored);
    if (!parsed || typeof parsed !== "object") {
      return {};
    }

    const result: Record<string, FastScriptRunStat> = {};
    for (const [key, raw] of Object.entries(parsed as Record<string, unknown>)) {
      if (!raw || typeof raw !== "object") {
        continue;
      }
      const value = raw as Record<string, unknown>;
      const count = typeof value.count === "number" && Number.isFinite(value.count) ? value.count : 0;
      const lastRunAt = typeof value.lastRunAt === "number" && Number.isFinite(value.lastRunAt) ? value.lastRunAt : 0;
      result[key] = { count, lastRunAt };
    }
    return result;
  } catch {
    return {};
  }
}

export function recordFastScriptRun(id: string): void {
  const stats = getFastScriptRunStats();
  const current = stats[id] ?? { count: 0, lastRunAt: 0 };
  stats[id] = { count: current.count + 1, lastRunAt: Date.now() };
  safeStorageSet(FAST_SCRIPT_RUNS_STORAGE_KEY, JSON.stringify(stats));
}

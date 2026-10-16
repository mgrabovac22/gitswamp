import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";
import { invoke } from "@tauri-apps/api/core";
import {
  SIMULATION_PROFILES,
  createDefaultRequest,
  createEmptyAuth,
  createEmptyBody,
  createEmptyCookie,
  createEmptyEnvironment,
  emptyKeyValue,
  httpId,
  normalizeMethod,
  sanitizeSameSite,
} from "./httpClient.utils";
import type {
  HttpAuth,
  HttpBody,
  HttpBodyMode,
  HttpCollection,
  HttpCookie,
  HttpCookieJar,
  HttpEnvironment,
  HttpExtractor,
  HttpExtractorScope,
  HttpExtractorSource,
  HttpHeaderDocument,
  HttpHistoryEntry,
  HttpKeyValue,
  HttpMethod,
  HttpRequestDef,
  HttpResponseHeader,
  HttpRunStatus,
  HttpRunStep,
  HttpScope,
  HttpSettings,
  HttpSimulationConfig,
  HttpSimulationKind,
  HttpSimulationResult,
  HttpTestResult,
} from "./httpClient.types";
import { HTTP_METHODS } from "./httpClient.types";

export const HTTP_COLLECTIONS_KEY = "gitswamp-http-collections";
export const HTTP_REQUESTS_KEY = "gitswamp-http-requests";
export const HTTP_ENVIRONMENTS_KEY = "gitswamp-http-environments";
export const HTTP_JARS_KEY = "gitswamp-http-cookie-jars";
export const HTTP_HISTORY_KEY = "gitswamp-http-history";
export const HTTP_TABS_KEY = "gitswamp-http-tabs";
export const HTTP_SETTINGS_KEY = "gitswamp-http-settings";
export const HTTP_HEADER_DOCS_KEY = "gitswamp-http-header-docs";
export const HTTP_RUNS_KEY = "gitswamp-http-runs";
export const HTTP_SIMULATIONS_KEY = "gitswamp-http-simulations";
export const HTTP_EVENT = "gitswamp-http-client-changed";

const HISTORY_LIMIT = 80;
const RUNS_LIMIT = 400;
const SIMULATIONS_LIMIT = 60;
const APP_STATE_NAME = "gitswamp-http-store";

const MIRROR_KEYS = [
  HTTP_COLLECTIONS_KEY,
  HTTP_REQUESTS_KEY,
  HTTP_ENVIRONMENTS_KEY,
  HTTP_JARS_KEY,
  HTTP_HISTORY_KEY,
  HTTP_TABS_KEY,
  HTTP_SETTINGS_KEY,
  HTTP_HEADER_DOCS_KEY,
  HTTP_RUNS_KEY,
  HTTP_SIMULATIONS_KEY,
];

let mirrorTimer: ReturnType<typeof setTimeout> | null = null;

function snapshotState(): Record<string, string> {
  const snapshot: Record<string, string> = {};
  for (const key of MIRROR_KEYS) {
    const value = safeStorageGet(key);
    if (value !== null) snapshot[key] = value;
  }
  return snapshot;
}

function scheduleMirror(): void {
  if (mirrorTimer) clearTimeout(mirrorTimer);
  mirrorTimer = setTimeout(() => {
    mirrorTimer = null;
    void invoke("write_app_state", { name: APP_STATE_NAME, contents: JSON.stringify(snapshotState()) }).catch(() => {});
  }, 600);
}

export async function hydrateHttpClientStorage(): Promise<void> {
  try {
    const raw = await invoke<string | null>("read_app_state", { name: APP_STATE_NAME });
    if (!raw) return;
    const snapshot = JSON.parse(raw) as Record<string, unknown>;
    for (const key of MIRROR_KEYS) {
      const value = snapshot[key];
      if (typeof value === "string" && safeStorageGet(key) === null) {
        safeStorageSet(key, value);
      }
    }
  } catch {
    /* Not running under Tauri, or no persisted state yet. */
  }
}


const SCOPES: readonly HttpScope[] = ["repo", "global"];
const BODY_MODES: readonly HttpBodyMode[] = ["none", "json", "text", "form", "urlencoded"];
const EXTRACTOR_SOURCES: readonly HttpExtractorSource[] = ["json", "text", "header", "regex", "status", "cookie"];
const EXTRACTOR_SCOPES: readonly HttpExtractorScope[] = ["session", "environment"];

function readJson(key: string): unknown {
  const stored = safeStorageGet(key);
  if (!stored) return null;
  try {
    return JSON.parse(stored);
  } catch {
    return null;
  }
}

function writeJson(key: string, value: unknown): void {
  safeStorageSet(key, JSON.stringify(value));
  scheduleMirror();
  globalThis.dispatchEvent(new CustomEvent(HTTP_EVENT, { detail: { key } }));
}

function asString(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

function asBool(value: unknown, fallback = false): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function asTimestamp(value: unknown, fallback = Date.now()): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function sanitizeScope(value: unknown): HttpScope {
  return SCOPES.includes(value as HttpScope) ? (value as HttpScope) : "repo";
}

function sanitizeKeyValues(raw: unknown): HttpKeyValue[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((entry): HttpKeyValue | null => {
      if (!entry || typeof entry !== "object") return null;
      const value = entry as Record<string, unknown>;
      return {
        id: asString(value.id) || httpId("kv"),
        key: asString(value.key),
        value: asString(value.value),
        description: asString(value.description) || undefined,
        enabled: asBool(value.enabled, true),
      };
    })
    .filter((entry): entry is HttpKeyValue => entry !== null);
}

function sanitizeExtractors(raw: unknown): HttpExtractor[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((entry): HttpExtractor | null => {
      if (!entry || typeof entry !== "object") return null;
      const value = entry as Record<string, unknown>;
      const variable = asString(value.variable).trim();
      if (!variable) return null;
      const source = asString(value.source, "json");
      const scope = asString(value.scope, "session");
      return {
        id: asString(value.id) || httpId("ext"),
        enabled: asBool(value.enabled, true),
        variable,
        source: EXTRACTOR_SOURCES.includes(source as HttpExtractorSource) ? (source as HttpExtractorSource) : "json",
        expression: asString(value.expression),
        scope: EXTRACTOR_SCOPES.includes(scope as HttpExtractorScope) ? (scope as HttpExtractorScope) : "session",
      };
    })
    .filter((entry): entry is HttpExtractor => entry !== null);
}

function sanitizeStringList(raw: unknown): string[] {
  if (!Array.isArray(raw)) return [];
  return raw.filter((entry): entry is string => typeof entry === "string" && entry.length > 0);
}

function sanitizeCookie(raw: unknown): HttpCookie | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;
  const name = asString(value.name).trim();
  const domain = asString(value.domain);
  if (!name && !domain) return null;
  const expiresRaw = value.expires;
  const expires = typeof expiresRaw === "number" && Number.isFinite(expiresRaw)
    ? expiresRaw
    : expiresRaw === null
      ? null
      : null;
  return {
    id: asString(value.id) || httpId("ck"),
    name,
    value: asString(value.value),
    domain,
    path: asString(value.path, "/") || "/",
    expires,
    httpOnly: asBool(value.httpOnly),
    secure: asBool(value.secure),
    sameSite: sanitizeSameSite(asString(value.sameSite)),
    enabled: asBool(value.enabled, true),
    hostOnly: asBool(value.hostOnly, false),
    fromJar: asBool(value.fromJar, false),
  };
}

function sanitizeCookieList(raw: unknown): HttpCookie[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .map(sanitizeCookie)
    .filter((entry): entry is HttpCookie => entry !== null);
}

function sanitizeAuth(raw: unknown): HttpAuth {
  const base = createEmptyAuth();
  if (!raw || typeof raw !== "object") return base;
  const value = raw as Record<string, unknown>;
  const type = asString(value.type, "none");
  return {
    type: (["none", "bearer", "basic", "apikey"] as const).includes(type as HttpAuth["type"])
      ? (type as HttpAuth["type"])
      : "none",
    bearerToken: asString(value.bearerToken),
    basicUser: asString(value.basicUser),
    basicPassword: asString(value.basicPassword),
    apiKeyName: asString(value.apiKeyName),
    apiKeyValue: asString(value.apiKeyValue),
    apiKeyIn: asString(value.apiKeyIn) === "query" ? "query" : "header",
  };
}

function sanitizeBody(raw: unknown): HttpBody {
  const base = createEmptyBody();
  if (!raw || typeof raw !== "object") return base;
  const value = raw as Record<string, unknown>;
  const mode = asString(value.mode, "none");
  return {
    mode: BODY_MODES.includes(mode as HttpBodyMode) ? (mode as HttpBodyMode) : "none",
    raw: asString(value.raw),
    form: sanitizeKeyValues(value.form),
    urlencoded: sanitizeKeyValues(value.urlencoded),
  };
}

function sanitizeRequest(raw: unknown): HttpRequestDef | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;
  const methodRaw = asString(value.method, "GET").toUpperCase();
  const method = (HTTP_METHODS as readonly string[]).includes(methodRaw)
    ? (methodRaw as HttpRequestDef["method"])
    : normalizeMethod(methodRaw);
  const now = Date.now();
  return {
    id: asString(value.id) || httpId("req"),
    name: asString(value.name, "Request") || "Request",
    method,
    url: asString(value.url),
    params: sanitizeKeyValues(value.params),
    headers: sanitizeKeyValues(value.headers),
    cookies: sanitizeCookieList(value.cookies),
    auth: sanitizeAuth(value.auth),
    body: sanitizeBody(value.body),
    scripts: asString(value.scripts),
    tests: asString(value.tests),
    docs: asString(value.docs),
    scope: sanitizeScope(value.scope),
    repoPath: asString(value.repoPath),
    collectionId: typeof value.collectionId === "string" ? value.collectionId : null,
    headerDocumentIds: sanitizeStringList(value.headerDocumentIds),
    extractors: sanitizeExtractors(value.extractors),
    order: typeof value.order === "number" && Number.isFinite(value.order) ? value.order : 0,
    iterations:
      typeof value.iterations === "number" && Number.isFinite(value.iterations)
        ? Math.min(500, Math.max(1, Math.round(value.iterations)))
        : 1,
    delayMs:
      typeof value.delayMs === "number" && Number.isFinite(value.delayMs)
        ? Math.min(60000, Math.max(0, Math.round(value.delayMs)))
        : 0,
    createdAt: asTimestamp(value.createdAt, now),
    updatedAt: asTimestamp(value.updatedAt, now),
  };
}

function sanitizeCollection(raw: unknown): HttpCollection | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;
  const now = Date.now();
  const name = asString(value.name).trim();
  if (!name) return null;
  return {
    id: asString(value.id) || httpId("col"),
    name,
    scope: sanitizeScope(value.scope),
    repoPath: asString(value.repoPath),
    parentId: typeof value.parentId === "string" ? value.parentId : null,
    headers: sanitizeKeyValues(value.headers),
    auth: sanitizeAuth(value.auth),
    order: typeof value.order === "number" && Number.isFinite(value.order) ? value.order : 0,
    createdAt: asTimestamp(value.createdAt, now),
    updatedAt: asTimestamp(value.updatedAt, now),
  };
}

export function getStoredCollections(): HttpCollection[] {
  const parsed = readJson(HTTP_COLLECTIONS_KEY);
  if (!Array.isArray(parsed)) return [];
  return parsed
    .map(sanitizeCollection)
    .filter((entry): entry is HttpCollection => entry !== null);
}

export function storeCollections(collections: HttpCollection[]): void {
  writeJson(HTTP_COLLECTIONS_KEY, collections);
}

export function getStoredRequests(): HttpRequestDef[] {
  const parsed = readJson(HTTP_REQUESTS_KEY);
  if (!Array.isArray(parsed)) return [];
  return parsed
    .map(sanitizeRequest)
    .filter((entry): entry is HttpRequestDef => entry !== null);
}

export function storeRequests(requests: HttpRequestDef[]): void {
  writeJson(HTTP_REQUESTS_KEY, requests);
}

function sanitizeHeaderDocument(raw: unknown): HttpHeaderDocument | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;
  const name = asString(value.name).trim();
  if (!name) return null;
  const now = Date.now();
  return {
    id: asString(value.id) || httpId("hdoc"),
    name,
    description: asString(value.description),
    headers: sanitizeKeyValues(value.headers),
    scope: sanitizeScope(value.scope),
    repoPath: asString(value.repoPath),
    createdAt: asTimestamp(value.createdAt, now),
    updatedAt: asTimestamp(value.updatedAt, now),
  };
}

export function getStoredHeaderDocuments(): HttpHeaderDocument[] {
  const parsed = readJson(HTTP_HEADER_DOCS_KEY);
  if (!Array.isArray(parsed)) return [];
  return parsed
    .map(sanitizeHeaderDocument)
    .filter((entry): entry is HttpHeaderDocument => entry !== null);
}

export function storeHeaderDocuments(documents: HttpHeaderDocument[]): void {
  writeJson(HTTP_HEADER_DOCS_KEY, documents);
}

function sanitizeEnvironment(raw: unknown): HttpEnvironment | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;
  const name = asString(value.name).trim();
  if (!name) return null;
  return {
    id: asString(value.id) || httpId("env"),
    name,
    variables: sanitizeKeyValues(value.variables),
  };
}

export function getStoredEnvironments(): HttpEnvironment[] {
  const parsed = readJson(HTTP_ENVIRONMENTS_KEY);
  if (!Array.isArray(parsed)) return [];
  const environments = parsed
    .map(sanitizeEnvironment)
    .filter((entry): entry is HttpEnvironment => entry !== null);
  if (environments.length === 0) {
    return [createEmptyEnvironment("Local")];
  }
  return environments;
}

export function storeEnvironments(environments: HttpEnvironment[]): void {
  writeJson(HTTP_ENVIRONMENTS_KEY, environments);
}

function sanitizeSimulationConfig(value: unknown): Partial<Record<HttpSimulationKind, HttpSimulationConfig>> {
  const result: Partial<Record<HttpSimulationKind, HttpSimulationConfig>> = {};
  if (!value || typeof value !== "object") return result;
  const map = value as Record<string, unknown>;
  for (const profile of SIMULATION_PROFILES) {
    const entry = map[profile.kind];
    if (!entry || typeof entry !== "object") continue;
    const raw = entry as Record<string, unknown>;
    const iterations = typeof raw.iterations === "number" && Number.isFinite(raw.iterations) ? Math.round(raw.iterations) : profile.iterations;
    const thresholdMs = typeof raw.thresholdMs === "number" && Number.isFinite(raw.thresholdMs) ? Math.round(raw.thresholdMs) : profile.thresholdMs;
    const expectedStatus = typeof raw.expectedStatus === "number" && Number.isFinite(raw.expectedStatus) ? Math.round(raw.expectedStatus) : profile.expectedStatus;
    result[profile.kind] = {
      iterations: Math.min(500, Math.max(1, iterations)),
      thresholdMs: Math.min(600000, Math.max(0, thresholdMs)),
      expectedStatus: Math.min(599, Math.max(100, expectedStatus)),
    };
  }
  return result;
}

export function getStoredSettings(): HttpSettings {
  const parsed = readJson(HTTP_SETTINGS_KEY);
  const base: HttpSettings = {
    sendCookiesFromJar: true,
    autoManageCookies: true,
    followRedirects: true,
    timeoutMs: 30000,
    activeEnvironmentId: "",
    simulationConfig: {},
  };
  if (!parsed || typeof parsed !== "object") return base;
  const value = parsed as Record<string, unknown>;
  return {
    sendCookiesFromJar: asBool(value.sendCookiesFromJar, base.sendCookiesFromJar),
    autoManageCookies: asBool(value.autoManageCookies, base.autoManageCookies),
    followRedirects: asBool(value.followRedirects, base.followRedirects),
    timeoutMs:
      typeof value.timeoutMs === "number" && Number.isFinite(value.timeoutMs)
        ? Math.min(300000, Math.max(1000, value.timeoutMs))
        : base.timeoutMs,
    activeEnvironmentId: asString(value.activeEnvironmentId),
    simulationConfig: sanitizeSimulationConfig(value.simulationConfig),
  };
}

export function storeSettings(settings: HttpSettings): void {
  writeJson(HTTP_SETTINGS_KEY, settings);
}

export function getStoredCookieJars(): HttpCookieJar {
  const parsed = readJson(HTTP_JARS_KEY);
  if (!parsed || typeof parsed !== "object") return {};
  const result: HttpCookieJar = {};
  for (const [environmentId, domains] of Object.entries(parsed as Record<string, unknown>)) {
    if (!domains || typeof domains !== "object") continue;
    const domainMap: Record<string, HttpCookie[]> = {};
    for (const [domain, cookies] of Object.entries(domains as Record<string, unknown>)) {
      const list = sanitizeCookieList(cookies);
      if (list.length > 0) domainMap[domain] = list;
    }
    if (Object.keys(domainMap).length > 0) result[environmentId] = domainMap;
  }
  return result;
}

export function storeCookieJars(jars: HttpCookieJar): void {
  writeJson(HTTP_JARS_KEY, jars);
}

function sanitizeHistoryEntry(raw: unknown): HttpHistoryEntry | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;
  const request = sanitizeRequest(value.request);
  if (!request) return null;
  return {
    id: asString(value.id) || httpId("hist"),
    method: request.method,
    url: asString(value.url, request.url),
    status: typeof value.status === "number" ? value.status : 0,
    statusText: asString(value.statusText),
    durationMs: typeof value.durationMs === "number" ? value.durationMs : 0,
    sizeBytes: typeof value.sizeBytes === "number" ? value.sizeBytes : 0,
    at: asTimestamp(value.at),
    requestId: typeof value.requestId === "string" ? value.requestId : null,
    environmentId: asString(value.environmentId),
    request,
  };
}

export function getStoredHistory(): HttpHistoryEntry[] {
  const parsed = readJson(HTTP_HISTORY_KEY);
  if (!Array.isArray(parsed)) return [];
  return parsed
    .map(sanitizeHistoryEntry)
    .filter((entry): entry is HttpHistoryEntry => entry !== null);
}

export function storeHistory(history: HttpHistoryEntry[]): void {
  writeJson(HTTP_HISTORY_KEY, history.slice(0, HISTORY_LIMIT));
}

function sanitizeMethod(value: unknown): HttpMethod {
  const raw = asString(value, "GET").toUpperCase();
  return (HTTP_METHODS as readonly string[]).includes(raw) ? (raw as HttpMethod) : "GET";
}

function sanitizeResponseHeaders(raw: unknown): HttpResponseHeader[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((entry): HttpResponseHeader | null => {
      if (!entry || typeof entry !== "object") return null;
      const value = entry as Record<string, unknown>;
      return { name: asString(value.name), value: asString(value.value) };
    })
    .filter((entry): entry is HttpResponseHeader => entry !== null);
}

function sanitizeTests(raw: unknown): HttpTestResult[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((entry): HttpTestResult | null => {
      if (!entry || typeof entry !== "object") return null;
      const value = entry as Record<string, unknown>;
      return {
        name: asString(value.name),
        passed: asBool(value.passed, false),
        message: asString(value.message),
      };
    })
    .filter((entry): entry is HttpTestResult => entry !== null);
}

function sanitizeStringRecord(raw: unknown): Record<string, string> {
  if (!raw || typeof raw !== "object") return {};
  const result: Record<string, string> = {};
  for (const [key, value] of Object.entries(raw as Record<string, unknown>)) {
    if (typeof value === "string") result[key] = value;
  }
  return result;
}

const RUN_STATUSES: readonly HttpRunStatus[] = ["pending", "running", "passed", "failed", "skipped"];

function sanitizeRunStep(raw: unknown): HttpRunStep | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;
  const status = asString(value.status, "pending");
  return {
    requestId: asString(value.requestId),
    collectionId: typeof value.collectionId === "string" ? value.collectionId : null,
    name: asString(value.name, "Request"),
    method: sanitizeMethod(value.method),
    status: RUN_STATUSES.includes(status as HttpRunStatus) ? (status as HttpRunStatus) : "passed",
    httpStatus: typeof value.httpStatus === "number" ? value.httpStatus : 0,
    durationMs: typeof value.durationMs === "number" ? value.durationMs : 0,
    sizeBytes: typeof value.sizeBytes === "number" ? value.sizeBytes : 0,
    iterations: typeof value.iterations === "number" ? value.iterations : 1,
    error: typeof value.error === "string" ? value.error : null,
    extracted: sanitizeStringRecord(value.extracted),
    tests: sanitizeTests(value.tests),
    responseHeaders: sanitizeResponseHeaders(value.responseHeaders),
    responseBody: asString(value.responseBody),
    url: asString(value.url),
    at: asTimestamp(value.at),
  };
}

export function getStoredRunSteps(): HttpRunStep[] {
  const parsed = readJson(HTTP_RUNS_KEY);
  if (!Array.isArray(parsed)) return [];
  return parsed
    .map(sanitizeRunStep)
    .filter((entry): entry is HttpRunStep => entry !== null)
    .slice(0, RUNS_LIMIT);
}

export function storeRunSteps(steps: HttpRunStep[]): void {
  writeJson(HTTP_RUNS_KEY, steps.slice(0, RUNS_LIMIT));
}

const SIMULATION_KINDS: readonly HttpSimulationKind[] = [
  "smoke",
  "sanity",
  "latency",
  "load",
  "spike",
  "stress",
  "soak",
  "regression",
];

function sanitizeSimulation(raw: unknown): HttpSimulationResult | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;
  const kind = asString(value.kind, "smoke");
  const num = (input: unknown, fallback = 0): number =>
    typeof input === "number" && Number.isFinite(input) ? input : fallback;
  return {
    id: asString(value.id) || httpId("sim"),
    kind: SIMULATION_KINDS.includes(kind as HttpSimulationKind) ? (kind as HttpSimulationKind) : "smoke",
    label: asString(value.label, "Simulation"),
    requestId: asString(value.requestId),
    requestName: asString(value.requestName),
    method: sanitizeMethod(value.method),
    url: asString(value.url),
    iterations: num(value.iterations, 1),
    simulatedDurationMs: num(value.simulatedDurationMs),
    requests: num(value.requests),
    passed: num(value.passed),
    failed: num(value.failed),
    avgMs: num(value.avgMs),
    minMs: num(value.minMs),
    maxMs: num(value.maxMs),
    p50Ms: num(value.p50Ms),
    p95Ms: num(value.p95Ms),
    errorRate: num(value.errorRate),
    baselineStatus: num(value.baselineStatus),
    thresholdMs: num(value.thresholdMs),
    expectedStatus: num(value.expectedStatus, 200),
    ok: asBool(value.ok, true),
    note: asString(value.note),
    at: asTimestamp(value.at),
  };
}

export function getStoredSimulations(): HttpSimulationResult[] {
  const parsed = readJson(HTTP_SIMULATIONS_KEY);
  if (!Array.isArray(parsed)) return [];
  return parsed
    .map(sanitizeSimulation)
    .filter((entry): entry is HttpSimulationResult => entry !== null)
    .slice(0, SIMULATIONS_LIMIT);
}

export function storeSimulations(results: HttpSimulationResult[]): void {
  writeJson(HTTP_SIMULATIONS_KEY, results.slice(0, SIMULATIONS_LIMIT));
}

export interface StoredTabs {
  open: string[];
  active: string | null;
}

export function getStoredTabs(repoKey: string): StoredTabs {
  const parsed = readJson(HTTP_TABS_KEY);
  const empty: StoredTabs = { open: [], active: null };
  if (!parsed || typeof parsed !== "object") return empty;
  const entry = (parsed as Record<string, unknown>)[repoKey];
  if (!entry || typeof entry !== "object") return empty;
  const value = entry as Record<string, unknown>;
  return {
    open: Array.isArray(value.open) ? value.open.filter((id): id is string => typeof id === "string") : [],
    active: typeof value.active === "string" ? value.active : null,
  };
}

export function storeTabs(repoKey: string, tabs: StoredTabs): void {
  const parsed = readJson(HTTP_TABS_KEY);
  const current = parsed && typeof parsed === "object" ? { ...(parsed as Record<string, unknown>) } : {};
  current[repoKey] = tabs;
  writeJson(HTTP_TABS_KEY, current);
}

export function cloneCookieJar(jars: HttpCookieJar): HttpCookieJar {
  return JSON.parse(JSON.stringify(jars)) as HttpCookieJar;
}

export function createDraftRequest(partial: Partial<HttpRequestDef> = {}): HttpRequestDef {
  return createDefaultRequest(partial);
}

export function createDraftCookie(domain = ""): HttpCookie {
  return createEmptyCookie(domain);
}

export function makeHeader(key: string, value: string): HttpKeyValue {
  const item = emptyKeyValue();
  item.key = key;
  item.value = value;
  return item;
}

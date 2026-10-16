import type {
  HttpAuth,
  HttpBody,
  HttpCollection,
  HttpCookie,
  HttpEnvironment,
  HttpExportBundle,
  HttpExportKind,
  HttpExtractor,
  HttpHeaderDocument,
  HttpKeyValue,
  HttpMethod,
  HttpRequestDef,
  HttpResolvedRequest,
  HttpResponseData,
  HttpResponseCookie,
  HttpResponseHeader,
  HttpRunStats,
  HttpRunStep,
  HttpSameSite,
  HttpSimulationKind,
  HttpSimulationConfig,
  HttpSimulationProfile,
  HttpSimulationResult,
} from "./httpClient.types";
import { HTTP_METHODS } from "./httpClient.types";

let counter = 0;

export function randomGuid(): string {
  try {
    if (globalThis.crypto?.randomUUID) return globalThis.crypto.randomUUID();
  } catch {}
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (char) => {
    const random = Math.floor(Math.random() * 16);
    const value = char === "x" ? random : (random & 0x3) | 0x8;
    return value.toString(16);
  });
}

export function httpId(prefix = "http"): string {
  counter += 1;
  const stamp = Date.now().toString(36);
  const random = Math.random().toString(36).slice(2, 7);
  return `${prefix}_${stamp}${counter.toString(36)}${random}`;
}

export function emptyKeyValue(): HttpKeyValue {
  return { id: httpId("kv"), key: "", value: "", enabled: true };
}

export function keyValue(key: string, value: string, enabled = true): HttpKeyValue {
  return { id: httpId("kv"), key, value, enabled };
}

export function createEmptyAuth(): HttpAuth {
  return {
    type: "none",
    bearerToken: "",
    basicUser: "",
    basicPassword: "",
    apiKeyName: "",
    apiKeyValue: "",
    apiKeyIn: "header",
  };
}

export function createEmptyBody(): HttpBody {
  return {
    mode: "none",
    raw: "",
    form: [],
    urlencoded: [],
  };
}

export function createEmptyCookie(domain = ""): HttpCookie {
  return {
    id: httpId("ck"),
    name: "",
    value: "",
    domain,
    path: "/",
    expires: null,
    httpOnly: false,
    secure: false,
    sameSite: "",
    enabled: true,
  };
}

export function createEmptyEnvironment(name = "New environment"): HttpEnvironment {
  return { id: httpId("env"), name, variables: [] };
}

export function createEmptyExtractor(): HttpExtractor {
  return { id: httpId("ext"), enabled: true, variable: "", source: "json", expression: "", scope: "session" };
}

export function createHeaderDocument(name = "New header set"): HttpHeaderDocument {
  const now = Date.now();
  return {
    id: httpId("hdoc"),
    name,
    description: "",
    headers: [],
    scope: "global",
    repoPath: "",
    createdAt: now,
    updatedAt: now,
  };
}

export function createDefaultRequest(partial: Partial<HttpRequestDef> = {}): HttpRequestDef {
  const now = Date.now();
  return {
    id: httpId("req"),
    name: "New request",
    method: "GET",
    url: "",
    params: [],
    headers: [],
    cookies: [],
    auth: createEmptyAuth(),
    body: createEmptyBody(),
    scripts: "",
    tests: "",
    docs: "",
    scope: "repo",
    repoPath: "",
    collectionId: null,
    headerDocumentIds: [],
    extractors: [],
    order: 0,
    iterations: 1,
    delayMs: 0,
    createdAt: now,
    updatedAt: now,
    ...partial,
  };
}

export function normalizeMethod(value: string): HttpMethod {
  const upper = value.trim().toUpperCase();
  return (HTTP_METHODS as readonly string[]).includes(upper) ? (upper as HttpMethod) : "GET";
}

export function isApiKeyHeaderSafe(name: string): boolean {
  return /^[a-zA-Z0-9-_.]+$/.test(name.trim());
}

export function hostnameFromUrl(url: string): string {
  const value = url.trim();
  if (!value) return "";
  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(value) ? value : `https://${value}`;
  try {
    return new URL(candidate).hostname.toLowerCase();
  } catch {
    return "";
  }
}

export function hostWithPortFromUrl(url: string): string {
  const value = url.trim();
  if (!value) return "";
  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(value) ? value : `https://${value}`;
  try {
    return new URL(candidate).host.toLowerCase();
  } catch {
    return "";
  }
}

export function isHttps(url: string): boolean {
  const value = url.trim();
  if (!value) return false;
  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(value) ? value : `https://${value}`;
  try {
    return new URL(candidate).protocol === "https:";
  } catch {
    return false;
  }
}

export function domainMatches(requestDomain: string, cookieDomain: string): boolean {
  const req = requestDomain.toLowerCase().replace(/^\./, "");
  const cookie = cookieDomain.toLowerCase().replace(/^\./, "");
  if (!req || !cookie) return false;
  if (req === cookie) return true;
  return req.endsWith(`.${cookie}`);
}

export function pathMatches(requestPath: string, cookiePath: string): boolean {
  if (!cookiePath || cookiePath === "/") return true;
  const path = requestPath || "/";
  return path === cookiePath || path.startsWith(cookiePath.endsWith("/") ? cookiePath : `${cookiePath}/`);
}

export function isCookieExpired(cookie: { expires: number | null }): boolean {
  return cookie.expires !== null && cookie.expires <= Date.now();
}

export function sanitizeSameSite(value: string): HttpSameSite {
  const lower = value.trim().toLowerCase();
  if (lower === "lax") return "Lax";
  if (lower === "strict") return "Strict";
  if (lower === "none") return "None";
  return "";
}

export function parseSetCookie(headerValue: string, fallbackDomain = ""): HttpResponseCookie | null {
  const parts = headerValue.split(";");
  const pair = parts.shift();
  if (!pair) return null;

  const eq = pair.indexOf("=");
  if (eq < 0) return null;

  const name = pair.slice(0, eq).trim();
  const value = pair.slice(eq + 1).trim();
  if (!name) return null;

  let domain = fallbackDomain;
  let hostOnly = true;
  let path = "/";
  let expires: number | null = null;
  let httpOnly = false;
  let secure = false;
  let sameSite: HttpSameSite = "";

  for (const attribute of parts) {
    const trimmed = attribute.trim();
    if (!trimmed) continue;
    const lower = trimmed.toLowerCase();

    if (lower.startsWith("domain=")) {
      domain = trimmed.slice(7).trim().replace(/^\./, "").toLowerCase();
      hostOnly = false;
    } else if (lower.startsWith("path=")) {
      path = trimmed.slice(5).trim() || "/";
    } else if (lower.startsWith("expires=")) {
      const parsed = Date.parse(trimmed.slice(8).trim());
      if (!Number.isNaN(parsed)) expires = parsed;
    } else if (lower.startsWith("max-age=")) {
      const seconds = Number(trimmed.slice(8).trim());
      if (Number.isFinite(seconds)) expires = Date.now() + seconds * 1000;
    } else if (lower === "httponly") {
      httpOnly = true;
    } else if (lower === "secure") {
      secure = true;
    } else if (lower.startsWith("samesite=")) {
      sameSite = sanitizeSameSite(trimmed.slice(9));
    }
  }

  return {
    id: httpId("rc"),
    name,
    value,
    domain,
    path,
    expires,
    httpOnly,
    secure,
    sameSite,
    hostOnly,
    raw: headerValue,
  };
}

export function cookieToSetCookieString(cookie: HttpCookie): string {
  const parts = [`${cookie.name}=${cookie.value}`];
  if (cookie.domain) parts.push(`Domain=${cookie.domain}`);
  if (cookie.path) parts.push(`Path=${cookie.path}`);
  if (cookie.expires && cookie.expires > 0) {
    parts.push(`Expires=${new Date(cookie.expires).toUTCString()}`);
  }
  if (cookie.httpOnly) parts.push("HttpOnly");
  if (cookie.secure) parts.push("Secure");
  if (cookie.sameSite) parts.push(`SameSite=${cookie.sameSite}`);
  return parts.join("; ");
}

export function cookieJarKey(cookie: HttpCookie): string {
  return `${cookie.name.toLowerCase()}\u0000${cookie.domain.toLowerCase()}\u0000${cookie.path}`;
}

export interface VariableContext {
  variables: Record<string, string>;
}

const DYNAMIC_VARIABLE_PATTERN = /\{\{\s*\$([a-zA-Z]+)\s*\}\}/g;

export function resolveDynamicVariables(input: string): string {
  return input.replace(DYNAMIC_VARIABLE_PATTERN, (_match, name: string) => {
    const key = name.toLowerCase();
    if (key === "timestamp") return String(Date.now());
    if (key === "isotimestamp") return new Date().toISOString();
    if (key === "guid" || key === "uuid") return randomGuid();
    if (key === "randomint") return String(Math.floor(Math.random() * 1000));
    return _match;
  });
}

export function interpolateVariables(input: string, context: VariableContext): string {
  if (!input || !input.includes("{{")) return input;
  const replaced = input.replace(/\{\{\s*([^}]+?)\s*\}\}/g, (match, name: string) => {
    if (name.startsWith("$")) return match;
    const value = context.variables[name];
    return value === undefined ? match : value;
  });
  return resolveDynamicVariables(replaced);
}

export function findVariables(input: string): string[] {
  const names: string[] = [];
  const pattern = /\{\{\s*([^}$][^}]*?)\s*\}\}/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(input)) !== null) {
    names.push(match[1]);
  }
  return names;
}

function enabledKeyValues(items: HttpKeyValue[]): HttpKeyValue[] {
  return items.filter((item) => item.enabled && item.key.trim().length > 0);
}

export function collectInheritedHeaders(
  request: HttpRequestDef,
  collectionHeaders: HttpKeyValue[],
  headerDocumentHeaders: HttpKeyValue[] = [],
): { own: HttpKeyValue[]; inherited: HttpKeyValue[] } {
  const overridden = new Set(
    enabledKeyValues(request.headers).map((item) => item.key.trim().toLowerCase()),
  );
  const seen = new Set<string>(overridden);
  const inherited: HttpKeyValue[] = [];
  for (const item of [...collectionHeaders, ...headerDocumentHeaders]) {
    const key = item.key.trim().toLowerCase();
    if (!item.enabled || key.length === 0 || seen.has(key)) continue;
    seen.add(key);
    inherited.push(item);
  }
  return { own: enabledKeyValues(request.headers), inherited };
}

export function buildUrl(baseUrl: string, params: HttpKeyValue[]): string {
  const enabled = enabledKeyValues(params);
  if (enabled.length === 0) return baseUrl;

  const [withoutHash, hash = ""] = baseUrl.split("#", 2);
  const [path, existingQuery = ""] = withoutHash.split("?", 2);
  const search = new URLSearchParams(existingQuery);
  for (const param of enabled) {
    search.append(param.key, param.value);
  }
  const query = search.toString();
  return `${path}${query ? `?${query}` : ""}${hash ? `#${hash}` : ""}`;
}

export function buildAuthHeaders(auth: HttpAuth, context: VariableContext): { headers: [string, string][]; params: [string, string][] } {
  const headers: [string, string][] = [];
  const params: [string, string][] = [];

  if (auth.type === "bearer" && auth.bearerToken.trim()) {
    headers.push(["Authorization", `Bearer ${interpolateVariables(auth.bearerToken, context)}`]);
  } else if (auth.type === "basic" && (auth.basicUser || auth.basicPassword)) {
    const raw = `${interpolateVariables(auth.basicUser, context)}:${interpolateVariables(auth.basicPassword, context)}`;
    headers.push(["Authorization", `Basic ${base64EncodeUtf8(raw)}`]);
  } else if (auth.type === "apikey" && auth.apiKeyName.trim() && auth.apiKeyValue.trim()) {
    if (auth.apiKeyIn === "query") {
      params.push([auth.apiKeyName.trim(), interpolateVariables(auth.apiKeyValue, context)]);
    } else {
      headers.push([auth.apiKeyName.trim(), interpolateVariables(auth.apiKeyValue, context)]);
    }
  }

  return { headers, params };
}

export function base64EncodeUtf8(value: string): string {
  try {
    const bytes = new TextEncoder().encode(value);
    let binary = "";
    for (const byte of bytes) {
      binary += String.fromCharCode(byte);
    }
    return btoa(binary);
  } catch {
    return value;
  }
}

export function resolveRequest(
  request: HttpRequestDef,
  collectionHeaders: HttpKeyValue[],
  context: VariableContext,
  headerDocumentHeaders: HttpKeyValue[] = [],
): HttpResolvedRequest {
  const { own, inherited } = collectInheritedHeaders(request, collectionHeaders, headerDocumentHeaders);
  const headers: [string, string][] = [...own, ...inherited].map((item) => [
    interpolateVariables(item.key, context),
    interpolateVariables(item.value, context),
  ]);

  const authHeaders = buildAuthHeaders(request.auth, context);
  headers.push(...authHeaders.headers);

  const params = [...request.params];
  for (const [key, value] of authHeaders.params) {
    params.push(keyValue(key, value));
  }

  let body: string | null = null;
  if (request.body.mode === "json" || request.body.mode === "text") {
    body = interpolateVariables(request.body.raw, context);
    if (request.body.mode === "json" && !hasHeader(headers, "content-type")) {
      headers.push(["Content-Type", "application/json"]);
    }
  } else if (request.body.mode === "form" || request.body.mode === "urlencoded") {
    const source = request.body.mode === "form" ? request.body.form : request.body.urlencoded;
    const search = new URLSearchParams();
    for (const item of enabledKeyValues(source)) {
      search.append(interpolateVariables(item.key, context), interpolateVariables(item.value, context));
    }
    body = search.toString();
    if (!hasHeader(headers, "content-type")) {
      headers.push([
        "Content-Type",
        request.body.mode === "form" ? "multipart/form-data" : "application/x-www-form-urlencoded",
      ]);
    }
  }

  return {
    method: request.method,
    url: interpolateVariables(buildUrl(request.url, params), context),
    headers,
    cookies: request.cookies.filter((cookie) => cookie.enabled && cookie.name.trim().length > 0),
    body,
  };
}

export function hasHeader(headers: [string, string][], name: string): boolean {
  const lower = name.toLowerCase();
  return headers.some(([key]) => key.toLowerCase() === lower);
}

function shellEscape(value: string): string {
  if (value === "") return "''";
  if (/^[a-zA-Z0-9_./:@%+=,-]+$/.test(value)) return value;
  return `'${value.replace(/'/g, "'\\''")}'`;
}

export interface CurlOptions {
  includeCookies?: boolean;
  pretty?: boolean;
}

export function buildCurlCommand(request: HttpResolvedRequest, options: CurlOptions = {}): string {
  const lines: string[] = [`curl -X ${request.method}`];
  lines.push(`  ${shellEscape(request.url)}`);

  for (const [name, value] of request.headers) {
    lines.push(`  -H ${shellEscape(`${name}: ${value}`)}`);
  }

  if (options.includeCookies && request.cookies.length > 0) {
    const cookieString = request.cookies.map((cookie) => `${cookie.name}=${cookie.value}`).join("; ");
    lines.push(`  -b ${shellEscape(cookieString)}`);
  }

  if (request.body && request.method !== "GET" && request.method !== "HEAD") {
    lines.push(`  --data-raw ${shellEscape(request.body)}`);
  }

  return options.pretty === false ? lines.join(" ") : lines.join(" \\\n");
}

export interface ParsedCurl {
  method: HttpMethod;
  url: string;
  headers: HttpKeyValue[];
  body: string;
}

export function parseCurlCommand(input: string): ParsedCurl | null {
  const normalized = input.replace(/\\\r?\n/g, " ").trim();
  if (!/^curl\b/.test(normalized)) return null;

  const tokens = tokenizeShell(normalized);
  let method = "";
  let url = "";
  const headers: HttpKeyValue[] = [];
  let body = "";

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    const next = tokens[index + 1];

    if (token === "-X" || token === "--request") {
      method = next || "";
      index += 1;
    } else if (token === "-H" || token === "--header") {
      if (next) {
        const eq = next.indexOf(":");
        if (eq > 0) {
          headers.push(keyValue(next.slice(0, eq).trim(), next.slice(eq + 1).trim()));
        }
        index += 1;
      }
    } else if (token === "-d" || token === "--data" || token === "--data-raw" || token === "--data-binary") {
      if (next !== undefined) {
        body = next;
        index += 1;
      }
    } else if (token === "--url") {
      url = next || "";
      index += 1;
    } else if (!token.startsWith("-")) {
      const lower = token.toLowerCase();
      if (!method && HTTP_METHODS.includes(lower.toUpperCase() as HttpMethod)) {
        method = token;
      } else if (!url) {
        url = token;
      }
    }
  }

  if (!url) return null;
  if (!method) method = body ? "POST" : "GET";

  return { method: normalizeMethod(method), url, headers, body };
}

function tokenizeShell(input: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let quote: "'" | '"' | null = null;

  for (let index = 0; index < input.length; index += 1) {
    const char = input[index];

    if (quote) {
      if (char === quote) {
        quote = null;
      } else if (char === "\\" && quote === '"') {
        current += input[index + 1] ?? "";
        index += 1;
      } else {
        current += char;
      }
      continue;
    }

    if (char === "'" || char === '"') {
      quote = char;
    } else if (char === " ") {
      if (current) {
        tokens.push(current);
        current = "";
      }
    } else {
      current += char;
    }
  }

  if (current) tokens.push(current);
  return tokens;
}

export function headersToKeyValues(headers: HttpResponseHeader[]): HttpKeyValue[] {
  return headers.map((header) => keyValue(header.name, header.value));
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

export function statusTone(status: number): "success" | "redirect" | "warning" | "danger" | "muted" {
  if (status === 0) return "muted";
  if (status < 300) return "success";
  if (status < 400) return "redirect";
  if (status < 500) return "warning";
  return "danger";
}

export function formatCookieExpiry(expires: number | null): string {
  if (!expires) return "Session";
  return new Date(expires).toLocaleString();
}

export function uniqueName(base: string, existing: string[]): string {
  if (!existing.includes(base)) return base;
  let index = 2;
  while (existing.includes(`${base} ${index}`)) {
    index += 1;
  }
  return `${base} ${index}`;
}

function jsonPathTokens(path: string): (string | number)[] {
  const tokens: (string | number)[] = [];
  const normalized = path.trim().replace(/^\$\.?/, "");
  const pattern = /([^.[\]]+)|\[(\d+)\]|\["([^"]*)"\]|\['([^']*)'\]/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(normalized)) !== null) {
    if (match[1] !== undefined) tokens.push(match[1]);
    else if (match[2] !== undefined) tokens.push(Number(match[2]));
    else if (match[3] !== undefined) tokens.push(match[3]);
    else if (match[4] !== undefined) tokens.push(match[4]);
  }
  return tokens;
}

function valueAtJsonPath(root: unknown, path: string): unknown {
  let current: unknown = root;
  for (const token of jsonPathTokens(path)) {
    if (current === null || current === undefined) return undefined;
    if (typeof token === "number") {
      if (!Array.isArray(current)) return undefined;
      current = current[token];
    } else {
      if (typeof current !== "object") return undefined;
      current = (current as Record<string, unknown>)[token];
    }
  }
  return current;
}

function stringifyExtracted(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

export function evaluateExtractor(response: HttpResponseData, extractor: HttpExtractor): string | null {
  const expression = extractor.expression.trim();
  switch (extractor.source) {
    case "status":
      return String(response.status);
    case "header": {
      if (!expression) return null;
      const header = response.headers.find((item) => item.name.toLowerCase() === expression.toLowerCase());
      return header ? header.value : null;
    }
    case "cookie": {
      if (!expression) return null;
      const fallback = hostnameFromUrl(response.finalUrl);
      for (const header of response.headers) {
        if (header.name.toLowerCase() !== "set-cookie") continue;
        const cookie = parseSetCookie(header.value, fallback);
        if (cookie && cookie.name === expression) return cookie.value;
      }
      return null;
    }
    case "regex": {
      if (!expression) return null;
      try {
        const match = new RegExp(expression).exec(response.body);
        if (!match) return null;
        return match[1] ?? match[0];
      } catch {
        return null;
      }
    }
    case "text": {
      if (!expression) return response.body;
      try {
        const match = new RegExp(expression).exec(response.body);
        if (!match) return null;
        return match[1] ?? match[0];
      } catch {
        return null;
      }
    }
    case "json":
    default: {
      if (!response.body.trim()) return null;
      let parsed: unknown;
      try {
        parsed = JSON.parse(response.body);
      } catch {
        return null;
      }
      if (!expression) return stringifyExtracted(parsed);
      const value = valueAtJsonPath(parsed, expression);
      return value === undefined ? null : stringifyExtracted(value);
    }
  }
}

export function applyExtractors(response: HttpResponseData, extractors: HttpExtractor[]): Record<string, string> {
  const values: Record<string, string> = {};
  for (const extractor of extractors) {
    if (!extractor.enabled || !extractor.variable.trim()) continue;
    const value = evaluateExtractor(response, extractor);
    if (value !== null) values[extractor.variable.trim()] = value;
  }
  return values;
}

export function buildExportBundle(
  kind: HttpExportKind,
  collections: HttpCollection[],
  requests: HttpRequestDef[],
  headerDocuments: HttpHeaderDocument[],
): HttpExportBundle {
  return {
    format: "gitswamp.http",
    version: 1,
    kind,
    exportedAt: Date.now(),
    collections: JSON.parse(JSON.stringify(collections)),
    requests: JSON.parse(JSON.stringify(requests)),
    headerDocuments: JSON.parse(JSON.stringify(headerDocuments)),
  };
}

export function serializeExportBundle(bundle: HttpExportBundle): string {
  return JSON.stringify(bundle, null, 2);
}

export function parseExportBundle(json: string): HttpExportBundle | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    return null;
  }
  if (!parsed || typeof parsed !== "object") return null;
  const value = parsed as Record<string, unknown>;
  if (value.format !== "gitswamp.http") return null;
  const asArray = (input: unknown): unknown[] => (Array.isArray(input) ? input : []);
  return {
    format: "gitswamp.http",
    version: typeof value.version === "number" ? value.version : 1,
    kind: (["workspace", "collection", "request", "header-documents"] as const).includes(value.kind as HttpExportKind)
      ? (value.kind as HttpExportKind)
      : "workspace",
    exportedAt: typeof value.exportedAt === "number" ? value.exportedAt : Date.now(),
    collections: asArray(value.collections) as HttpCollection[],
    requests: asArray(value.requests) as HttpRequestDef[],
    headerDocuments: asArray(value.headerDocuments) as HttpHeaderDocument[],
  };
}

export interface RemapOptions {
  scope?: "repo" | "global";
  repoPath?: string;
  preserveCollectionLinks?: boolean;
}

export function remapBundleIds(bundle: HttpExportBundle, options: RemapOptions = {}): HttpExportBundle {
  const collectionMap = new Map<string, string>();
  const requestMap = new Map<string, string>();
  const headerDocumentMap = new Map<string, string>();

  const collections = bundle.collections.map((collection) => {
    const id = httpId("col");
    collectionMap.set(collection.id, id);
    return { ...collection, id };
  });

  const requests = bundle.requests.map((request) => {
    const id = httpId("req");
    requestMap.set(request.id, id);
    return { ...request, id };
  });

  const headerDocuments = bundle.headerDocuments.map((document) => {
    const id = httpId("hdoc");
    headerDocumentMap.set(document.id, id);
    return { ...document, id };
  });

  return {
    ...bundle,
    collections: collections.map((collection) => {
      const scope = options.scope ?? collection.scope;
      return {
        ...collection,
        parentId: collection.parentId ? collectionMap.get(collection.parentId) ?? null : null,
        scope,
        repoPath: scope === "global" ? "" : options.repoPath ?? collection.repoPath,
      };
    }),
    requests: requests.map((request) => {
      const scope = options.scope ?? request.scope;
      return {
        ...request,
        collectionId: options.preserveCollectionLinks !== false && request.collectionId
          ? collectionMap.get(request.collectionId) ?? null
          : null,
        headerDocumentIds: (request.headerDocumentIds ?? [])
          .map((id) => headerDocumentMap.get(id))
          .filter((id): id is string => typeof id === "string"),
        scope,
        repoPath: scope === "global" ? "" : options.repoPath ?? request.repoPath,
      };
    }),
    headerDocuments: headerDocuments.map((document) => {
      const scope = options.scope ?? document.scope;
      return {
        ...document,
        scope,
        repoPath: scope === "global" ? "" : options.repoPath ?? document.repoPath,
      };
    }),
  };
}

export function jsonPretty(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed) return raw;
  try {
    return JSON.stringify(JSON.parse(trimmed), null, 2);
  } catch {
    return raw;
  }
}

export function percentile(values: number[], ratio: number): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.max(0, Math.ceil(ratio * sorted.length) - 1));
  return sorted[index];
}

export function runStatsFor(steps: HttpRunStep[]): HttpRunStats {
  const durations = steps.filter((step) => step.durationMs > 0).map((step) => step.durationMs);
  const totalMs = durations.reduce((sum, value) => sum + value, 0);
  return {
    total: steps.length,
    passed: steps.filter((step) => step.status === "passed").length,
    failed: steps.filter((step) => step.status === "failed").length,
    skipped: steps.filter((step) => step.status === "skipped").length,
    avgMs: durations.length ? Math.round(totalMs / durations.length) : 0,
    p95Ms: percentile(durations, 0.95),
    totalMs,
    at: steps.reduce((latest, step) => Math.max(latest, step.at), 0),
  };
}

export const SIMULATION_PROFILES: HttpSimulationProfile[] = [
  {
    kind: "smoke",
    label: "Smoke",
    description: "Single reachability pass. Fails fast if the endpoint does not answer.",
    iterations: 1,
    durationLabel: "instant",
    thresholdMs: 0,
    expectedStatus: 200,
  },
  {
    kind: "sanity",
    label: "Sanity",
    description: "A few repeats to confirm the response is stable, not a one-off.",
    iterations: 3,
    durationLabel: "instant",
    thresholdMs: 0,
    expectedStatus: 200,
  },
  {
    kind: "latency",
    label: "Latency",
    description: "Simulates 10 clients and checks the 95th percentile stays under the budget.",
    iterations: 10,
    durationLabel: "~10 requests",
    thresholdMs: 500,
    expectedStatus: 200,
  },
  {
    kind: "load",
    label: "Load",
    description: "Simulates steady traffic and reports throughput and average latency.",
    iterations: 50,
    durationLabel: "~1 minute at 50 rpm",
    thresholdMs: 800,
    expectedStatus: 200,
  },
  {
    kind: "spike",
    label: "Spike",
    description: "Simulates a sudden burst, then recovery. Peaks around 2.6x the baseline.",
    iterations: 40,
    durationLabel: "burst profile",
    thresholdMs: 2000,
    expectedStatus: 200,
  },
  {
    kind: "stress",
    label: "Stress",
    description: "Pushes past the comfortable capacity and tolerates a small error budget.",
    iterations: 60,
    durationLabel: "ramp to 2.2x",
    thresholdMs: 3000,
    expectedStatus: 200,
  },
  {
    kind: "soak",
    label: "Soak",
    description: "Long sustained run to surface memory and connection leaks.",
    iterations: 100,
    durationLabel: "~1 hour at 100 rpm",
    thresholdMs: 1200,
    expectedStatus: 200,
  },
  {
    kind: "regression",
    label: "Regression",
    description: "Compares against the previous run and flags latency regressions.",
    iterations: 5,
    durationLabel: "delta check",
    thresholdMs: 0,
    expectedStatus: 200,
  },
];

export function simulationProfile(kind: HttpSimulationKind): HttpSimulationProfile {
  return SIMULATION_PROFILES.find((profile) => profile.kind === kind) ?? SIMULATION_PROFILES[0];
}

export function defaultSimulationConfig(kind: HttpSimulationKind): HttpSimulationConfig {
  const profile = simulationProfile(kind);
  return {
    iterations: profile.iterations,
    thresholdMs: profile.thresholdMs,
    expectedStatus: profile.expectedStatus,
  };
}

function seededRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state += 0x6d2b79f5;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  };
}

function statusIsSuccess(status: number): boolean {
  return status >= 200 && status < 400;
}

export interface HttpSimulationBaseline {
  status: number;
  durationMs: number;
  sizeBytes: number;
  url: string;
}

export function simulateProfile(
  kind: HttpSimulationKind,
  baseline: HttpSimulationBaseline,
  previousAvgMs = 0,
  config?: HttpSimulationConfig,
): Omit<HttpSimulationResult, "id" | "requestId" | "requestName" | "method" | "at" | "label"> {
  const profile = simulationProfile(kind);
  const iterations = Math.max(1, Math.min(500, Math.round(config?.iterations ?? profile.iterations)));
  const customThreshold = Math.max(0, Math.round(config?.thresholdMs ?? profile.thresholdMs));
  const expectedStatus = Math.min(599, Math.max(100, Math.round(config?.expectedStatus ?? profile.expectedStatus)));
  const base = Math.max(1, Math.round(baseline.durationMs || 1));
  const random = seededRandom((kind.length * 7919 + iterations) >>> 0);
  const factorFor = (index: number, total: number): number => {
    const progress = total <= 1 ? 0 : index / (total - 1);
    switch (kind) {
      case "spike":
        return progress < 0.6 ? 1 + progress * 0.4 : 1.4 + (1 - progress) / 0.4 * 1.2;
      case "stress":
        return 1 + progress * 1.2;
      case "load":
        return 1.15;
      case "soak":
        return 1 + progress * 0.15;
      default:
        return 1;
    }
  };

  const durations: number[] = [];
  let failed = 0;
  const failureBudget = kind === "stress" ? Math.floor(iterations * 0.05) : 0;
  for (let index = 0; index < iterations; index += 1) {
    const jitter = 0.85 + random() * 0.3;
    const duration = Math.round(base * factorFor(index, iterations) * jitter);
    const overBudget = index >= iterations - failureBudget && failureBudget > 0;
    if (overBudget) {
      failed += 1;
      durations.push(Math.round(duration * 1.4));
    } else {
      durations.push(duration);
    }
  }

  const avg = Math.round(durations.reduce((sum, value) => sum + value, 0) / durations.length);
  const p95 = percentile(durations, 0.95);
  const errorRate = Math.round((failed / iterations) * 1000) / 10;
  const simulatedDurationMs = durations.reduce((sum, value) => sum + value, 0);
  const regressionLimit = previousAvgMs > 0 ? Math.round(previousAvgMs * 1.25) : 0;
  const latencyBudget = kind === "regression" ? regressionLimit : customThreshold;
  const statusOk = statusIsSuccess(baseline.status) && (failed === 0 || kind === "stress");
  const latencyOk = latencyBudget <= 0 || p95 <= latencyBudget;
  const ok = statusOk && latencyOk;

  const notes: string[] = ["Simulated profile — the endpoint was probed once, load was modelled."];
  if (kind === "regression") {
    notes.push(previousAvgMs > 0 ? `Baseline average ${previousAvgMs}ms, limit ${regressionLimit}ms.` : "No previous run to compare against.");
  }
  if (customThreshold > 0 && kind !== "regression") {
    notes.push(`p95 budget ${customThreshold}ms.`);
  }
  if (failed > 0) notes.push(`${failed} simulated failure(s) (${errorRate}%).`);

  return {
    kind,
    url: baseline.url,
    iterations,
    simulatedDurationMs,
    requests: iterations,
    passed: iterations - failed,
    failed,
    avgMs: avg,
    minMs: Math.min(...durations),
    maxMs: Math.max(...durations),
    p50Ms: percentile(durations, 0.5),
    p95Ms: p95,
    errorRate,
    baselineStatus: baseline.status,
    thresholdMs: latencyBudget,
    expectedStatus,
    ok,
    note: notes.join(" "),
  };
}

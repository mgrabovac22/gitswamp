export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";

export const HTTP_METHODS: readonly HttpMethod[] = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
];

export type HttpScope = "repo" | "global";

export interface HttpKeyValue {
  id: string;
  key: string;
  value: string;
  description?: string;
  enabled: boolean;
}

export type HttpSameSite = "" | "Lax" | "Strict" | "None";

export interface HttpCookie {
  id: string;
  name: string;
  value: string;
  domain: string;
  path: string;
  expires: number | null;
  httpOnly: boolean;
  secure: boolean;
  sameSite: HttpSameSite;
  enabled: boolean;
  hostOnly?: boolean;
  fromJar?: boolean;
}

export type HttpAuthType = "none" | "bearer" | "basic" | "apikey";

export interface HttpAuth {
  type: HttpAuthType;
  bearerToken: string;
  basicUser: string;
  basicPassword: string;
  apiKeyName: string;
  apiKeyValue: string;
  apiKeyIn: "header" | "query";
}

export type HttpExtractorSource = "json" | "text" | "header" | "regex" | "status" | "cookie";

export type HttpExtractorScope = "session" | "environment";

export interface HttpExtractor {
  id: string;
  enabled: boolean;
  variable: string;
  source: HttpExtractorSource;
  expression: string;
  scope: HttpExtractorScope;
}

export interface HttpHeaderDocument {
  id: string;
  name: string;
  description: string;
  headers: HttpKeyValue[];
  scope: HttpScope;
  repoPath: string;
  createdAt: number;
  updatedAt: number;
}

export type HttpBodyMode = "none" | "json" | "text" | "form" | "urlencoded";

export interface HttpBody {
  mode: HttpBodyMode;
  raw: string;
  form: HttpKeyValue[];
  urlencoded: HttpKeyValue[];
}

export interface HttpRequestDef {
  id: string;
  name: string;
  method: HttpMethod;
  url: string;
  params: HttpKeyValue[];
  headers: HttpKeyValue[];
  cookies: HttpCookie[];
  auth: HttpAuth;
  body: HttpBody;
  scripts: string;
  tests: string;
  docs: string;
  scope: HttpScope;
  repoPath: string;
  collectionId: string | null;
  headerDocumentIds: string[];
  extractors: HttpExtractor[];
  order: number;
  iterations: number;
  delayMs: number;
  createdAt: number;
  updatedAt: number;
}

export interface HttpCollection {
  id: string;
  name: string;
  scope: HttpScope;
  repoPath: string;
  parentId: string | null;
  headers: HttpKeyValue[];
  auth: HttpAuth;
  order: number;
  createdAt: number;
  updatedAt: number;
}

export interface HttpEnvironment {
  id: string;
  name: string;
  variables: HttpKeyValue[];
}

export interface HttpCookieJar {
  [environmentId: string]: {
    [domain: string]: HttpCookie[];
  };
}

export interface HttpHistoryEntry {
  id: string;
  method: HttpMethod;
  url: string;
  status: number;
  statusText: string;
  durationMs: number;
  sizeBytes: number;
  at: number;
  requestId: string | null;
  environmentId: string;
  request: HttpRequestDef;
}

export interface HttpSettings {
  sendCookiesFromJar: boolean;
  autoManageCookies: boolean;
  followRedirects: boolean;
  timeoutMs: number;
  activeEnvironmentId: string;
  simulationConfig: Partial<Record<HttpSimulationKind, HttpSimulationConfig>>;
}

export interface HttpResponseHeader {
  name: string;
  value: string;
}

export interface HttpResponseData {
  status: number;
  statusText: string;
  headers: HttpResponseHeader[];
  body: string;
  bodyBase64: string | null;
  isBinary: boolean;
  sizeBytes: number;
  durationMs: number;
  finalUrl: string;
}

export interface HttpResponseCookie {
  id: string;
  name: string;
  value: string;
  domain: string;
  path: string;
  expires: number | null;
  httpOnly: boolean;
  secure: boolean;
  sameSite: HttpSameSite;
  hostOnly: boolean;
  raw: string;
}

export interface HttpTestResult {
  name: string;
  passed: boolean;
  message: string;
}

export interface HttpTimelineEntry {
  label: string;
  detail?: string;
  durationMs: number;
  offsetMs: number;
  tone: "request" | "response" | "info";
}

export interface HttpResolvedRequest {
  method: string;
  url: string;
  headers: [string, string][];
  cookies: HttpCookie[];
  body: string | null;
}

export type HttpExportKind = "workspace" | "collection" | "request" | "header-documents";

export interface HttpExportBundle {
  format: "gitswamp.http";
  version: number;
  kind: HttpExportKind;
  exportedAt: number;
  collections: HttpCollection[];
  requests: HttpRequestDef[];
  headerDocuments: HttpHeaderDocument[];
}

export type HttpRunStatus = "pending" | "running" | "passed" | "failed" | "skipped";

export interface HttpRunStep {
  requestId: string;
  collectionId: string | null;
  name: string;
  method: HttpMethod;
  status: HttpRunStatus;
  httpStatus: number;
  durationMs: number;
  sizeBytes: number;
  iterations: number;
  error: string | null;
  extracted: Record<string, string>;
  tests: HttpTestResult[];
  responseHeaders: HttpResponseHeader[];
  responseBody: string;
  url: string;
  at: number;
}

export interface HttpRunStats {
  total: number;
  passed: number;
  failed: number;
  skipped: number;
  avgMs: number;
  p95Ms: number;
  totalMs: number;
  at: number;
}

export type HttpSimulationKind =
  | "smoke"
  | "sanity"
  | "latency"
  | "load"
  | "spike"
  | "stress"
  | "soak"
  | "regression";

export interface HttpSimulationProfile {
  kind: HttpSimulationKind;
  label: string;
  description: string;
  iterations: number;
  durationLabel: string;
  thresholdMs: number;
  expectedStatus: number;
}

export interface HttpSimulationConfig {
  iterations: number;
  thresholdMs: number;
  expectedStatus: number;
}

export interface HttpSimulationResult {
  id: string;
  kind: HttpSimulationKind;
  label: string;
  requestId: string;
  requestName: string;
  method: HttpMethod;
  url: string;
  iterations: number;
  simulatedDurationMs: number;
  requests: number;
  passed: number;
  failed: number;
  avgMs: number;
  minMs: number;
  maxMs: number;
  p50Ms: number;
  p95Ms: number;
  errorRate: number;
  baselineStatus: number;
  thresholdMs: number;
  expectedStatus: number;
  ok: boolean;
  note: string;
  at: number;
}

import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "@/shared/notifications/useToast";
import type {
  HttpCollection,
  HttpCookie,
  HttpCookieJar,
  HttpEnvironment,
  HttpExportBundle,
  HttpHeaderDocument,
  HttpHistoryEntry,
  HttpKeyValue,
  HttpMethod,
  HttpRequestDef,
  HttpResponseData,
  HttpRunStats,
  HttpRunStep,
  HttpSettings,
  HttpSimulationConfig,
  HttpSimulationKind,
  HttpSimulationResult,
  HttpTestResult,
  HttpTimelineEntry,
} from "./httpClient.types";
import {
  applyExtractors,
  buildCurlCommand,
  buildExportBundle,
  collectInheritedHeaders,
  cookieJarKey,
  createHeaderDocument as buildHeaderDocument,
  defaultSimulationConfig,
  domainMatches,
  findVariables,
  hostnameFromUrl,
  httpId,
  isCookieExpired,
  parseSetCookie,
  parseCurlCommand,
  pathMatches,
  remapBundleIds,
  resolveRequest,
  runStatsFor,
  simulateProfile,
  simulationProfile,
  uniqueName,
} from "./httpClient.utils";
import {
  createDraftRequest,
  getStoredCollections,
  getStoredCookieJars,
  getStoredEnvironments,
  getStoredHeaderDocuments,
  getStoredHistory,
  getStoredRequests,
  getStoredRunSteps,
  getStoredSettings,
  getStoredSimulations,
  getStoredTabs,
  hydrateHttpClientStorage,
  storeCollections,
  storeCookieJars,
  storeEnvironments,
  storeHeaderDocuments,
  storeHistory,
  storeRequests,
  storeRunSteps,
  storeSettings,
  storeSimulations,
  storeTabs,
} from "./httpClientStorage";
import { runPreRequestScript, runTests } from "./httpClientScripts";

const GLOBAL_TAB_KEY = "__global__";

interface ClipboardLike {
  writeText: (value: string) => Promise<void>;
}

function debounce<T extends (...args: never[]) => void>(fn: T, wait = 260): T {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return ((...args: never[]) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => fn(...args), wait);
  }) as T;
}

function pathnameFromUrl(url: string): string {
  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(url) ? url : `https://${url}`;
  try {
    return new URL(candidate).pathname || "/";
  } catch {
    return "/";
  }
}

export function useHttpClient(getRepoPath: () => string) {
  const toast = useToast();

  const collections = ref<HttpCollection[]>([]);
  const requests = ref<HttpRequestDef[]>([]);
  const environments = ref<HttpEnvironment[]>([]);
  const jars = ref<HttpCookieJar>({});
  const history = ref<HttpHistoryEntry[]>([]);
  const settings = ref<HttpSettings>({
    sendCookiesFromJar: true,
    autoManageCookies: true,
    followRedirects: true,
    timeoutMs: 30000,
    activeEnvironmentId: "",
    simulationConfig: {},
  });

  const openIds = ref<string[]>([]);
  const activeId = ref<string | null>(null);

  const headerDocuments = ref<HttpHeaderDocument[]>([]);

  const response = ref<HttpResponseData | null>(null);
  const responseError = ref<string | null>(null);
  const responseDurationMs = ref(0);
  const testResults = ref<HttpTestResult[]>([]);
  const timeline = ref<HttpTimelineEntry[]>([]);
  const loading = ref(false);

  const sessionVariables = ref<Record<string, string>>({});
  const lastExtracted = ref<Record<string, string>>({});
  const runSteps = ref<HttpRunStep[]>([]);
  const runActive = ref(false);
  const runCursor = ref(-1);
  const simulationResults = ref<HttpSimulationResult[]>([]);
  const simulationActive = ref(false);
  let runCancelled = false;

  let ready = false;

  const repoKey = computed(() => getRepoPath() || GLOBAL_TAB_KEY);

  const visibleCollections = computed(() =>
    collections.value.filter(
      (collection) => collection.scope === "global" || (!!getRepoPath() && collection.repoPath === getRepoPath()),
    ),
  );

  function collectionIsGlobal(collectionId: string | null): boolean {
    if (!collectionId) return false;
    let currentId: string | null = collectionId;
    for (let guard = 0; currentId && guard < 16; guard += 1) {
      const collection = collections.value.find((item) => item.id === currentId);
      if (!collection) return false;
      if (collection.scope === "global") return true;
      currentId = collection.parentId;
    }
    return false;
  }

  const visibleRequests = computed(() =>
    requests.value.filter(
      (request) =>
        request.scope === "global" ||
        (!!getRepoPath() && request.repoPath === getRepoPath()) ||
        collectionIsGlobal(request.collectionId),
    ),
  );

  const visibleHeaderDocuments = computed(() =>
    headerDocuments.value.filter(
      (document) => document.scope === "global" || (!!getRepoPath() && document.repoPath === getRepoPath()),
    ),
  );

  const activeRequest = computed<HttpRequestDef | null>(
    () => requests.value.find((request) => request.id === activeId.value) ?? null,
  );

  const openRequests = computed<HttpRequestDef[]>(() =>
    openIds.value
      .map((id) => requests.value.find((request) => request.id === id))
      .filter((request): request is HttpRequestDef => request !== undefined),
  );

  const activeEnvironment = computed<HttpEnvironment | null>(() => {
    const found = environments.value.find((env) => env.id === settings.value.activeEnvironmentId);
    return found ?? environments.value[0] ?? null;
  });

  const activeEnvironmentId = computed(() => activeEnvironment.value?.id ?? "");

  const environmentVariables = computed<Record<string, string>>(() => {
    const result: Record<string, string> = {};
    for (const variable of activeEnvironment.value?.variables ?? []) {
      if (variable.enabled && variable.key.trim()) {
        result[variable.key.trim()] = variable.value;
      }
    }
    return result;
  });

  const activeJar = computed<Record<string, HttpCookie[]>>(
    () => jars.value[activeEnvironmentId.value] ?? {},
  );

  const jarDomains = computed(() => Object.keys(activeJar.value).sort());

  const activeCookieCount = computed(() => {
    const host = hostnameFromUrl(activeRequest.value?.url ?? "");
    if (!host) return Object.values(activeJar.value).reduce((sum, list) => sum + list.length, 0);
    return Object.entries(activeJar.value)
      .filter(([domain]) => domainMatches(host, domain))
      .reduce((sum, [, list]) => sum + list.filter((cookie) => !isCookieExpired(cookie)).length, 0);
  });

  function collectionHeadersFor(request: HttpRequestDef | null): HttpKeyValue[] {
    if (!request || !request.collectionId) return [];
    const byId = new Map(collections.value.map((collection) => [collection.id, collection]));
    const chain: HttpCollection[] = [];
    let current = byId.get(request.collectionId);
    let guard = 0;
    while (current && guard < 16) {
      chain.unshift(current);
      current = current.parentId ? byId.get(current.parentId) : undefined;
      guard += 1;
    }
    return chain.flatMap((collection) => collection.headers);
  }

  function headerDocumentHeadersFor(request: HttpRequestDef | null): HttpKeyValue[] {
    if (!request || request.headerDocumentIds.length === 0) return [];
    const byId = new Map(headerDocuments.value.map((document) => [document.id, document]));
    return request.headerDocumentIds.flatMap((id) => byId.get(id)?.headers ?? []);
  }

  function inheritedHeadersFor(request: HttpRequestDef | null): HttpKeyValue[] {
    if (!request) return [];
    return collectInheritedHeaders(request, collectionHeadersFor(request), headerDocumentHeadersFor(request)).inherited;
  }

  function currentVariables(): Record<string, string> {
    return { ...environmentVariables.value, ...sessionVariables.value };
  }

  function setSessionVariable(name: string, value: string): void {
    if (!name.trim()) return;
    sessionVariables.value = { ...sessionVariables.value, [name.trim()]: value };
  }

  function clearSessionVariables(): void {
    sessionVariables.value = {};
    toast.success("Session variables cleared.");
  }

  function persistVariables(values: Record<string, string>): void {
    const environment = activeEnvironment.value;
    if (!environment) return;
    const next = [...environment.variables];
    for (const [key, value] of Object.entries(values)) {
      const existing = next.find((item) => item.key === key && item.enabled);
      if (existing) {
        existing.value = value;
      } else if (!next.some((item) => item.key === key)) {
        next.push({ id: httpId("kv"), key, value, enabled: true });
      }
    }
    environment.variables = next;
    storeEnvironments(environments.value);
  }

  async function copyText(value: string, message: string): Promise<void> {
    const clipboard = (globalThis as { navigator?: { clipboard?: ClipboardLike } }).navigator?.clipboard;
    try {
      if (clipboard) {
        await clipboard.writeText(value);
      } else {
        const textarea = document.createElement("textarea");
        textarea.value = value;
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
      }
      toast.success(message);
    } catch {
      toast.error("Could not copy to clipboard.");
    }
  }

  function loadAll(): void {
    ready = false;
    collections.value = getStoredCollections();
    requests.value = getStoredRequests();
    environments.value = getStoredEnvironments();
    headerDocuments.value = getStoredHeaderDocuments();
    jars.value = getStoredCookieJars();
    history.value = getStoredHistory();
    settings.value = getStoredSettings();

    if (!settings.value.activeEnvironmentId || !environments.value.some((env) => env.id === settings.value.activeEnvironmentId)) {
      settings.value.activeEnvironmentId = environments.value[0]?.id ?? "";
    }

    const storedTabs = getStoredTabs(repoKey.value);
    const validIds = new Set(requests.value.map((request) => request.id));
    openIds.value = storedTabs.open.filter((id) => validIds.has(id));
    activeId.value = storedTabs.active && validIds.has(storedTabs.active) ? storedTabs.active : openIds.value[0] ?? null;

    runSteps.value = getStoredRunSteps();
    simulationResults.value = getStoredSimulations();

    ready = true;
  }

  const persistCollections = debounce(() => ready && storeCollections(collections.value));
  const persistRequests = debounce(() => ready && storeRequests(requests.value));
  const persistEnvironments = debounce(() => ready && storeEnvironments(environments.value));
  const persistHeaderDocuments = debounce(() => ready && storeHeaderDocuments(headerDocuments.value));
  const persistJars = debounce(() => ready && storeCookieJars(jars.value));
  const persistHistory = debounce(() => ready && storeHistory(history.value));
  const persistRuns = debounce(() => ready && storeRunSteps(runSteps.value));
  const persistSimulations = debounce(() => ready && storeSimulations(simulationResults.value));
  const persistSettings = debounce(() => ready && storeSettings(settings.value));
  const persistTabs = debounce(() => {
    if (!ready) return;
    storeTabs(repoKey.value, { open: openIds.value, active: activeId.value });
  });

  watch(collections, persistCollections, { deep: true });
  watch(requests, persistRequests, { deep: true });
  watch(environments, persistEnvironments, { deep: true });
  watch(headerDocuments, persistHeaderDocuments, { deep: true });
  watch(jars, persistJars, { deep: true });
  watch(history, persistHistory, { deep: true });
  watch(runSteps, persistRuns, { deep: true });
  watch(simulationResults, persistSimulations, { deep: true });
  watch(settings, persistSettings, { deep: true });
  watch([openIds, activeId], persistTabs, { deep: true });

  watch(
    () => getRepoPath(),
    () => {
      const storedTabs = getStoredTabs(repoKey.value);
      const validIds = new Set(requests.value.map((request) => request.id));
      openIds.value = storedTabs.open.filter((id) => validIds.has(id));
      activeId.value = storedTabs.active && validIds.has(storedTabs.active) ? storedTabs.active : openIds.value[0] ?? null;
      response.value = null;
      responseError.value = null;
      testResults.value = [];
    },
  );

  function openRequest(id: string): void {
    if (!requests.value.some((request) => request.id === id)) return;
    if (!openIds.value.includes(id)) {
      openIds.value = [...openIds.value, id];
    }
    activeId.value = id;
  }

  function closeTab(id: string): void {
    const index = openIds.value.indexOf(id);
    if (index < 0) return;
    openIds.value = openIds.value.filter((openId) => openId !== id);
    if (activeId.value === id) {
      const next = openIds.value[index] ?? openIds.value[index - 1] ?? null;
      activeId.value = next;
    }
  }

  function nextOrderIn(collectionId: string | null): number {
    const siblings = requests.value.filter((request) => (request.collectionId || null) === (collectionId || null));
    return siblings.reduce((max, request) => Math.max(max, request.order || 0), 0) + 1000;
  }

  function newRequest(collectionId: string | null = null, method: HttpMethod = "GET"): HttpRequestDef {
    const existingNames = requests.value.map((request) => request.name);
    const base = collectionId
      ? collections.value.find((collection) => collection.id === collectionId)?.name ?? "Request"
      : "New request";
    const created = createDraftRequest({
      name: uniqueName(base === "New request" ? "New request" : `${base} request`, existingNames),
      method,
      scope: collectionId ? "repo" : getRepoPath() ? "repo" : "global",
      repoPath: getRepoPath(),
      collectionId,
    });
    created.order = nextOrderIn(collectionId);
    requests.value = [...requests.value, created];
    openRequest(created.id);
    return created;
  }

  function duplicateRequest(id: string): void {
    const source = requests.value.find((request) => request.id === id);
    if (!source) return;
    const clone: HttpRequestDef = JSON.parse(JSON.stringify(source));
    clone.id = httpId("req");
    clone.name = uniqueName(`${source.name} copy`, requests.value.map((request) => request.name));
    clone.order = nextOrderIn(source.collectionId || null);
    clone.createdAt = Date.now();
    clone.updatedAt = Date.now();
    requests.value = [...requests.value, clone];
    openRequest(clone.id);
  }

  function deleteRequest(id: string): void {
    requests.value = requests.value.filter((request) => request.id !== id);
    closeTab(id);
    if (activeId.value === id) activeId.value = openIds.value[0] ?? null;
  }

  function renameRequest(id: string, name: string): void {
    const request = requests.value.find((item) => item.id === id);
    if (!request) return;
    const trimmed = name.trim();
    if (!trimmed) return;
    request.name = trimmed;
    request.updatedAt = Date.now();
  }

  function createCollection(parentId: string | null = null): HttpCollection {
    const now = Date.now();
    const collection: HttpCollection = {
      id: httpId("col"),
      name: uniqueName(
        parentId ? "New folder" : "New collection",
        collections.value.map((item) => item.name),
      ),
      scope: getRepoPath() ? "repo" : "global",
      repoPath: getRepoPath(),
      parentId,
      headers: [],
      auth: {
        type: "none",
        bearerToken: "",
        basicUser: "",
        basicPassword: "",
        apiKeyName: "",
        apiKeyValue: "",
        apiKeyIn: "header",
      },
      order: collections.value.length,
      createdAt: now,
      updatedAt: now,
    };
    collections.value = [...collections.value, collection];
    return collection;
  }

  function renameCollection(id: string, name: string): void {
    const collection = collections.value.find((item) => item.id === id);
    if (collection) {
      collection.name = name.trim() || collection.name;
      collection.updatedAt = Date.now();
    }
  }

  function deleteCollection(id: string): void {
    const toDelete = new Set<string>([id]);
    let changed = true;
    while (changed) {
      changed = false;
      for (const collection of collections.value) {
        if (collection.parentId && toDelete.has(collection.parentId) && !toDelete.has(collection.id)) {
          toDelete.add(collection.id);
          changed = true;
        }
      }
    }
    collections.value = collections.value.filter((collection) => !toDelete.has(collection.id));
    const removedRequests = requests.value.filter((request) => request.collectionId && toDelete.has(request.collectionId));
    requests.value = requests.value.filter((request) => !(request.collectionId && toDelete.has(request.collectionId)));
    for (const request of removedRequests) {
      closeTab(request.id);
    }
  }

  function moveRequestToCollection(requestId: string, collectionId: string | null): void {
    const request = requests.value.find((item) => item.id === requestId);
    if (!request) return;
    request.collectionId = collectionId;
    request.updatedAt = Date.now();
    const collection = collectionId ? collections.value.find((item) => item.id === collectionId) : null;
    if (collection) {
      request.scope = collection.scope;
      request.repoPath = collection.repoPath;
    }
  }

  function importCurl(curl: string): void {
    const parsed = parseCurlCommand(curl);
    if (!parsed) {
      toast.error("Could not parse that cURL command.");
      return;
    }
    const created = newRequest(null, parsed.method);
    created.name = uniqueName("Imported request", requests.value.map((request) => request.name));
    created.url = parsed.url;
    created.headers = parsed.headers;
    if (parsed.body) {
      created.body.mode = parsed.body.trim().startsWith("{") || parsed.body.trim().startsWith("[") ? "json" : "text";
      created.body.raw = parsed.body;
    }
    created.updatedAt = Date.now();
    toast.success("Imported cURL command.");
  }

  function addEnvironment(): HttpEnvironment {
    const env: HttpEnvironment = { id: httpId("env"), name: uniqueName("Environment", environments.value.map((item) => item.name)), variables: [] };
    environments.value = [...environments.value, env];
    return env;
  }

  function deleteEnvironment(id: string): void {
    const target = environments.value.find((env) => env.id === id);
    if (target && target.name.trim().toLowerCase() === "local") {
      toast.warning("The Local environment cannot be deleted.");
      return;
    }
    if (environments.value.length <= 1) {
      toast.warning("Keep at least one environment.");
      return;
    }
    environments.value = environments.value.filter((env) => env.id !== id);
    delete jars.value[id];
    if (settings.value.activeEnvironmentId === id) {
      settings.value.activeEnvironmentId = environments.value[0]?.id ?? "";
    }
  }

  function selectEnvironment(id: string): void {
    settings.value.activeEnvironmentId = id;
  }

  function jarCookiesFor(url: string, request: HttpRequestDef): HttpCookie[] {
    const host = hostnameFromUrl(url);
    const path = pathnameFromUrl(url);
    const own = request.cookies.filter((cookie) => cookie.enabled && cookie.name.trim().length > 0);
    const ownNames = new Set(own.map((cookie) => cookie.name));
    const fromJar: HttpCookie[] = [];

    if (settings.value.sendCookiesFromJar) {
      for (const [domain, list] of Object.entries(activeJar.value)) {
        if (!domainMatches(host, domain)) continue;
        for (const cookie of list) {
          if (!cookie.enabled || !cookie.name.trim()) continue;
          if (isCookieExpired(cookie)) continue;
          if (cookie.secure && !url.trim().toLowerCase().startsWith("https")) continue;
          if (ownNames.has(cookie.name)) continue;
          if (!pathMatches(path, cookie.path)) continue;
          fromJar.push(cookie);
        }
      }
    }

    return [...own, ...fromJar];
  }

  function mergeSetCookies(url: string, headers: { name: string; value: string }[]): void {
    if (!settings.value.autoManageCookies) return;
    const host = hostnameFromUrl(url);
    const environmentId = activeEnvironmentId.value;
    if (!environmentId) return;

    const domainMap = { ...(jars.value[environmentId] ?? {}) };
    let mutated = false;

    for (const header of headers) {
      if (header.name.toLowerCase() !== "set-cookie") continue;
      const parsed = parseSetCookie(header.value, host);
      if (!parsed) continue;
      const domain = (parsed.domain || host).toLowerCase();
      const list = [...(domainMap[domain] ?? [])];
      const key = `${parsed.name}\u0000${parsed.path}`;
      const filtered = list.filter((cookie) => `${cookie.name}\u0000${cookie.path}` !== key);
      mutated = true;

      if (parsed.expires !== null && parsed.expires <= Date.now()) {
        domainMap[domain] = filtered;
        continue;
      }

      filtered.push({
        id: httpId("jar"),
        name: parsed.name,
        value: parsed.value,
        domain,
        path: parsed.path,
        expires: parsed.expires,
        httpOnly: parsed.httpOnly,
        secure: parsed.secure,
        sameSite: parsed.sameSite,
        enabled: true,
        hostOnly: parsed.hostOnly,
        fromJar: true,
      });
      domainMap[domain] = filtered;
    }

    if (mutated) {
      jars.value = { ...jars.value, [environmentId]: domainMap };
    }
  }

  function saveCookieToJar(cookie: HttpCookie): void {
    const environmentId = activeEnvironmentId.value;
    if (!environmentId) return;
    const domain = (cookie.domain || hostnameFromUrl(activeRequest.value?.url ?? "")).toLowerCase();
    if (!domain) {
      toast.warning("Set a domain before saving the cookie.");
      return;
    }
    const domainMap = { ...(jars.value[environmentId] ?? {}) };
    const list = (domainMap[domain] ?? []).filter((item) => cookieJarKey(item) !== cookieJarKey({ ...cookie, domain }));
    list.push({ ...cookie, domain, fromJar: true });
    domainMap[domain] = list;
    jars.value = { ...jars.value, [environmentId]: domainMap };
    toast.success(`Saved "${cookie.name}" to the jar.`);
  }

  function removeCookieFromJar(domain: string, id: string): void {
    const environmentId = activeEnvironmentId.value;
    if (!environmentId) return;
    const domainMap = { ...(jars.value[environmentId] ?? {}) };
    const list = (domainMap[domain] ?? []).filter((cookie) => cookie.id !== id);
    if (list.length > 0) domainMap[domain] = list;
    else delete domainMap[domain];
    jars.value = { ...jars.value, [environmentId]: domainMap };
  }

  function clearJarDomain(domain: string): void {
    const environmentId = activeEnvironmentId.value;
    if (!environmentId) return;
    const domainMap = { ...(jars.value[environmentId] ?? {}) };
    delete domainMap[domain];
    jars.value = { ...jars.value, [environmentId]: domainMap };
    toast.success(`Cleared cookies for ${domain}.`);
  }

  function clearAllCookies(): void {
    const environmentId = activeEnvironmentId.value;
    if (!environmentId) return;
    jars.value = { ...jars.value, [environmentId]: {} };
    toast.success("Cleared all cookies for this environment.");
  }

  function upsertJarCookie(domain: string, cookie: HttpCookie): void {
    const environmentId = activeEnvironmentId.value;
    if (!environmentId) return;
    const domainMap = { ...(jars.value[environmentId] ?? {}) };
    const list = [...(domainMap[domain] ?? [])];
    const index = list.findIndex((item) => item.id === cookie.id);
    if (index >= 0) list[index] = { ...cookie, domain };
    else list.push({ ...cookie, domain, fromJar: true });
    domainMap[domain] = list;
    jars.value = { ...jars.value, [environmentId]: domainMap };
  }

  function exportJar(): string {
    const environmentId = activeEnvironmentId.value;
    const domainMap = jars.value[environmentId] ?? {};
    const cookies = Object.values(domainMap).flat();
    return JSON.stringify(cookies, null, 2);
  }

  function importJar(json: string): void {
    try {
      const parsed = JSON.parse(json);
      if (!Array.isArray(parsed)) throw new Error("Expected an array");
      const environmentId = activeEnvironmentId.value;
      const domainMap: Record<string, HttpCookie[]> = {};
      for (const raw of parsed) {
        const domain = String((raw as { domain?: string }).domain || hostnameFromUrl(activeRequest.value?.url ?? "")).toLowerCase();
        if (!domain) continue;
        const cookie = raw as HttpCookie;
        domainMap[domain] = [...(domainMap[domain] ?? []), { ...cookie, fromJar: true }];
      }
      jars.value = { ...jars.value, [environmentId]: { ...(jars.value[environmentId] ?? {}), ...domainMap } };
      toast.success("Cookies imported.");
    } catch {
      toast.error("Invalid cookie JSON.");
    }
  }

  function curlForActive(includeCookies = true): string {
    const request = activeRequest.value;
    if (!request) return "";
    const variables = currentVariables();
    const resolved = resolveRequest(request, collectionHeadersFor(request), { variables }, headerDocumentHeadersFor(request));
    const cookies = jarCookiesFor(resolved.url, request);
    return buildCurlCommand({ ...resolved, cookies }, { includeCookies });
  }

  async function copyAsCurl(): Promise<void> {
    const command = curlForActive(true);
    if (!command) return;
    await copyText(command, "cURL copied to clipboard.");
  }

  function buildTimeline(durationMs: number, sizeBytes: number): HttpTimelineEntry[] {
    const wait = Math.max(0, Math.round(durationMs * 0.85));
    const download = Math.max(0, durationMs - wait);
    return [
      { label: "Request prepared", detail: "Headers and cookies applied", durationMs: 0, offsetMs: 0, tone: "request" },
      { label: "Waiting for server", detail: "Time to first byte", durationMs: wait, offsetMs: 0, tone: "info" },
      { label: "Content download", detail: `${sizeBytes} bytes received`, durationMs: download, offsetMs: wait, tone: "response" },
    ];
  }

  async function sendRequest(request: HttpRequestDef, options: { keepResponse?: boolean } = {}): Promise<HttpResponseData | null> {
    if (loading.value) {
      if (!options.keepResponse) toast.warning("A request or run is already in progress.");
      return null;
    }
    loading.value = true;
    responseError.value = null;
    lastExtracted.value = {};
    const interactive = !options.keepResponse;
    if (interactive) {
      response.value = null;
    }
    if (interactive) {
      timeline.value = [];
    }
    testResults.value = [];

    const variables = currentVariables();
    runPreRequestScript(request.scripts, variables);
    persistVariables(variables);

    const resolved = resolveRequest(request, collectionHeadersFor(request), { variables }, headerDocumentHeadersFor(request));
    const cookies = jarCookiesFor(resolved.url, request);
    const headers: [string, string][] = [...resolved.headers];

    const hasCookieHeader = headers.some(([name]) => name.toLowerCase() === "cookie");
    if (!hasCookieHeader && cookies.length > 0) {
      headers.push(["Cookie", cookies.map((cookie) => `${cookie.name}=${cookie.value}`).join("; ")]);
    }

    const missingVariables = findVariables(resolved.url).filter((name) => variables[name] === undefined);

    try {
      const result = await invoke<HttpResponseData>("http_send_request", {
        payload: {
          method: resolved.method,
          url: resolved.url,
          headers: headers.map(([name, value]) => ({ name, value })),
          body: resolved.body,
          timeoutMs: settings.value.timeoutMs,
          followRedirects: settings.value.followRedirects,
        },
      });

      if (interactive) {
        response.value = result;
        responseDurationMs.value = result.durationMs;
        timeline.value = buildTimeline(result.durationMs, result.sizeBytes);
      }
      mergeSetCookies(result.finalUrl || resolved.url, result.headers);

      testResults.value = runTests(request.tests, { response: result, variables });
      const extracted = applyExtractors(result, request.extractors);
      lastExtracted.value = extracted;
      for (const [name, value] of Object.entries(extracted)) {
        const extractor = request.extractors.find((item) => item.variable.trim() === name);
        if (extractor?.scope === "environment") {
          variables[name] = value;
        } else {
          setSessionVariable(name, value);
        }
      }
      persistVariables(variables);

      history.value = [
        {
          id: httpId("hist"),
          method: request.method,
          url: resolved.url,
          status: result.status,
          statusText: result.statusText,
          durationMs: result.durationMs,
          sizeBytes: result.sizeBytes,
          at: Date.now(),
          requestId: request.id,
          environmentId: activeEnvironmentId.value,
          request: JSON.parse(JSON.stringify(request)),
        },
        ...history.value,
      ].slice(0, 80);

      if (missingVariables.length > 0) {
        toast.warning(`Unresolved variables: ${missingVariables.join(", ")}`);
      }
      return result;
    } catch (error) {
      const message = typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
      responseError.value = message;
      lastExtracted.value = {};
      if (interactive) {
        toast.error(message);
      }
      return null;
    } finally {
      loading.value = false;
    }
  }

  async function sendActive(): Promise<void> {
    if (activeRequest.value) {
      await sendRequest(activeRequest.value);
    }
  }

  function clearResponse(): void {
    response.value = null;
    responseError.value = null;
    testResults.value = [];
    timeline.value = [];
  }

  function clearHistory(): void {
    history.value = [];
    toast.success("History cleared.");
  }

  function createHeaderDocument(name?: string): HttpHeaderDocument {
    const document = buildHeaderDocument(name ? name : uniqueName("Header set", headerDocuments.value.map((item) => item.name)));
    headerDocuments.value = [...headerDocuments.value, document];
    return document;
  }

  function renameHeaderDocument(id: string, name: string): void {
    const document = headerDocuments.value.find((item) => item.id === id);
    if (document) {
      document.name = name.trim() || document.name;
      document.updatedAt = Date.now();
    }
  }

  function deleteHeaderDocument(id: string): void {
    headerDocuments.value = headerDocuments.value.filter((item) => item.id !== id);
    for (const request of requests.value) {
      if (request.headerDocumentIds.includes(id)) {
        request.headerDocumentIds = request.headerDocumentIds.filter((item) => item !== id);
      }
    }
  }

  function requestsForCollection(collectionId: string): HttpRequestDef[] {
    return visibleRequests.value
      .filter((request) => request.collectionId === collectionId)
      .sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
  }

  function collectionRunList(): HttpRequestDef[] {
    const ordered: HttpRequestDef[] = [];
    const sorted = [...visibleCollections.value].sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
    const collectionIds = new Set(visibleCollections.value.map((collection) => collection.id));
    for (const collection of sorted) {
      ordered.push(...requestsForCollection(collection.id));
    }
    ordered.push(
      ...visibleRequests.value
        .filter((request) => !request.collectionId || !collectionIds.has(request.collectionId))
        .sort((a, b) => a.order - b.order || a.name.localeCompare(b.name)),
    );
    return ordered;
  }

  function reorderRequestInCollection(requestId: string, direction: -1 | 1): void {
    const request = requests.value.find((item) => item.id === requestId);
    if (!request) return;
    const siblings = request.collectionId
      ? requestsForCollection(request.collectionId)
      : visibleRequests.value
          .filter((item) => !item.collectionId)
          .sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
    const index = siblings.findIndex((item) => item.id === requestId);
    const swapIndex = index + direction;
    if (index < 0 || swapIndex < 0 || swapIndex >= siblings.length) return;
    const reordered = [...siblings];
    const [moved] = reordered.splice(index, 1);
    reordered.splice(swapIndex, 0, moved);
    reordered.forEach((item, position) => {
      item.order = (position + 1) * 1000;
      item.updatedAt = Date.now();
    });
  }

  function emptyStep(request: HttpRequestDef): HttpRunStep {
    return {
      requestId: request.id,
      collectionId: request.collectionId,
      name: request.name,
      method: request.method,
      status: "pending",
      httpStatus: 0,
      durationMs: 0,
      sizeBytes: 0,
      iterations: 1,
      error: null,
      extracted: {},
      tests: [],
      responseHeaders: [],
      responseBody: "",
      url: request.url,
      at: Date.now(),
    };
  }

  async function runRequests(list: HttpRequestDef[]): Promise<void> {
    if (runActive.value || list.length === 0) return;
    runActive.value = true;
    runCancelled = false;
    runCursor.value = 0;
    runSteps.value = list.map(emptyStep);

    let failed = 0;
    for (let index = 0; index < list.length; index += 1) {
      if (runCancelled) {
        runSteps.value = runSteps.value.map((step, stepIndex) =>
          stepIndex >= index && step.status === "pending" ? { ...step, status: "skipped" } : step,
        );
        break;
      }
      runCursor.value = index;
      const request = list[index];
      const iterations = Math.max(1, request.iterations || 1);
      const delayMs = Math.max(0, request.delayMs || 0);
      runSteps.value = runSteps.value.map((step, stepIndex) =>
        stepIndex === index ? { ...step, status: "running", iterations } : step,
      );

      let durationMs = 0;
      let lastResult: HttpResponseData | null = null;
      let stepFailed = false;
      let lastError: string | null = null;
      for (let attempt = 0; attempt < iterations; attempt += 1) {
        if (runCancelled) break;
        if (attempt > 0 && delayMs > 0) await new Promise((resolve) => setTimeout(resolve, delayMs));
        const result = await sendRequest(request, { keepResponse: true });
        if (result) {
          lastResult = result;
          durationMs += result.durationMs;
        } else {
          stepFailed = true;
          lastError = responseError.value;
          break;
        }
      }

      const testSnapshot = testResults.value.map((test) => ({ ...test }));
      const extracted = { ...lastExtracted.value };
      if (stepFailed || !lastResult) failed += 1;
      runSteps.value = runSteps.value.map((step, stepIndex) =>
        stepIndex === index
          ? {
              ...step,
              status: stepFailed || !lastResult ? "failed" : "passed",
              httpStatus: lastResult?.status ?? 0,
              durationMs,
              sizeBytes: lastResult?.sizeBytes ?? 0,
              error: stepFailed ? lastError : null,
              extracted,
              tests: testSnapshot,
              responseHeaders: (lastResult?.headers ?? []).slice(0, 40),
              responseBody: (lastResult?.body ?? "").slice(0, 4000),
              url: lastResult?.finalUrl || request.url,
              at: Date.now(),
            }
          : step,
      );
    }

    runCursor.value = -1;
    runActive.value = false;
    if (runCancelled) {
      toast.warning("Run stopped.");
    } else if (failed > 0) {
      toast.error(`Run finished with ${failed} failed request(s).`);
    } else {
      toast.success(`Run finished: ${list.length} request(s).`);
    }
    runCancelled = false;
  }

  function runActiveRequest(): void {
    if (activeRequest.value) void runRequests([activeRequest.value]);
  }

  function runCollection(collectionId: string): void {
    void runRequests(requestsForCollection(collectionId));
  }

  function runAll(): void {
    void runRequests(collectionRunList());
  }

  function stopRun(): void {
    runCancelled = true;
  }

  function clearRun(): void {
    runSteps.value = [];
    runCursor.value = -1;
  }

  const runStats = computed<HttpRunStats | null>(() =>
    runSteps.value.length ? runStatsFor(runSteps.value) : null,
  );

  const collectionStats = computed<{ [collectionId: string]: HttpRunStats }>(() => {
    const map: { [collectionId: string]: HttpRunStats } = {};
    const grouped = new Map<string, HttpRunStep[]>();
    for (const step of runSteps.value) {
      const key = step.collectionId ?? "";
      grouped.set(key, [...(grouped.get(key) ?? []), step]);
    }
    for (const [key, steps] of grouped) {
      map[key] = runStatsFor(steps);
    }
    return map;
  });

  function simulationConfigFor(kind: HttpSimulationKind): HttpSimulationConfig {
    const stored = settings.value.simulationConfig[kind];
    if (stored) {
      return {
        iterations: Math.min(500, Math.max(1, Math.round(stored.iterations))),
        thresholdMs: Math.min(600000, Math.max(0, Math.round(stored.thresholdMs))),
        expectedStatus: Math.min(599, Math.max(100, Math.round(stored.expectedStatus))),
      };
    }
    return defaultSimulationConfig(kind);
  }

  function updateSimulationConfig(kind: HttpSimulationKind, config: HttpSimulationConfig): void {
    settings.value = {
      ...settings.value,
      simulationConfig: {
        ...settings.value.simulationConfig,
        [kind]: {
          iterations: Math.min(500, Math.max(1, Math.round(config.iterations))),
          thresholdMs: Math.min(600000, Math.max(0, Math.round(config.thresholdMs))),
          expectedStatus: Math.min(599, Math.max(100, Math.round(config.expectedStatus))),
        },
      },
    };
    persistSettings();
  }

  function resetSimulationConfig(kind: HttpSimulationKind): void {
    const next = { ...settings.value.simulationConfig };
    delete next[kind];
    settings.value = { ...settings.value, simulationConfig: next };
    persistSettings();
  }

  async function runSimulations(kinds: HttpSimulationKind[], requestId?: string): Promise<void> {
    if (simulationActive.value || runActive.value || kinds.length === 0) return;
    const request = requestId
      ? requests.value.find((item) => item.id === requestId) ?? null
      : activeRequest.value;
    if (!request) {
      toast.error("Open a request before running simulated tests.");
      return;
    }
    simulationActive.value = true;
    try {
      for (const kind of kinds) {
        const probe = await sendRequest(request, { keepResponse: true });
        if (!probe) {
          toast.error(`${simulationProfile(kind).label} probe failed.`);
          continue;
        }
        const previous = simulationResults.value.find((item) => item.kind === kind && item.requestId === request.id);
        const simulated = simulateProfile(
          kind,
          {
            status: probe.status,
            durationMs: probe.durationMs,
            sizeBytes: probe.sizeBytes,
            url: probe.finalUrl || request.url,
          },
          previous?.avgMs ?? 0,
          simulationConfigFor(kind),
        );
        simulationResults.value = [
          {
            id: httpId("sim"),
            label: simulationProfile(kind).label,
            requestId: request.id,
            requestName: request.name,
            method: request.method,
            at: Date.now(),
            ...simulated,
          },
          ...simulationResults.value,
        ].slice(0, 40);
      }
      toast.success("Simulated tests finished.");
    } finally {
      simulationActive.value = false;
    }
  }

  function clearSimulationResults(): void {
    simulationResults.value = [];
  }

  function collectCollectionIds(rootId: string): Set<string> {
    const ids = new Set<string>([rootId]);
    let changed = true;
    while (changed) {
      changed = false;
      for (const collection of collections.value) {
        if (collection.parentId && ids.has(collection.parentId) && !ids.has(collection.id)) {
          ids.add(collection.id);
          changed = true;
        }
      }
    }
    return ids;
  }

  function headerDocumentsForRequests(list: HttpRequestDef[]): HttpHeaderDocument[] {
    const ids = new Set(list.flatMap((request) => request.headerDocumentIds));
    return headerDocuments.value.filter((document) => ids.has(document.id));
  }

  function ancestorCollectionIds(rootId: string): Set<string> {
    const ids = new Set<string>();
    let currentId: string | null | undefined = rootId;
    while (currentId && !ids.has(currentId)) {
      ids.add(currentId);
      currentId = collections.value.find((collection) => collection.id === currentId)?.parentId ?? null;
    }
    return ids;
  }

  function bundleForRequest(requestId: string): HttpExportBundle | null {
    const request = requests.value.find((item) => item.id === requestId);
    if (!request) return null;
    const ids = request.collectionId ? ancestorCollectionIds(request.collectionId) : new Set<string>();
    const collectionsSubset = collections.value.filter((item) => ids.has(item.id));
    return buildExportBundle("request", collectionsSubset, [request], headerDocumentsForRequests([request]));
  }

  function bundleForCollection(collectionId: string): HttpExportBundle | null {
    const collection = collections.value.find((item) => item.id === collectionId);
    if (!collection) return null;
    const ids = collectCollectionIds(collectionId);
    const collectionsSubset = collections.value.filter((item) => ids.has(item.id));
    const requestsSubset = requests.value.filter((request) => request.collectionId && ids.has(request.collectionId));
    return buildExportBundle("collection", collectionsSubset, requestsSubset, headerDocumentsForRequests(requestsSubset));
  }

  function bundleForWorkspace(): HttpExportBundle {
    return buildExportBundle("workspace", visibleCollections.value, visibleRequests.value, visibleHeaderDocuments.value);
  }

  function bundleForHeaderDocuments(): HttpExportBundle {
    return buildExportBundle("header-documents", [], [], visibleHeaderDocuments.value);
  }

  function importBundle(
    bundle: HttpExportBundle,
    options: { scope?: "repo" | "global"; repoPath?: string; targetCollectionId?: string | null } = {},
  ): { collections: number; requests: number; headerDocuments: number } {
    const remapped = remapBundleIds(bundle, { scope: options.scope, repoPath: options.repoPath });
    if (options.targetCollectionId !== undefined) {
      remapped.requests = remapped.requests.map((request) => ({ ...request, collectionId: options.targetCollectionId ?? null }));
    }
    const requestNames = requests.value.map((request) => request.name);
    const nextByCollection = new Map<string | null, number>();
    remapped.requests = remapped.requests.map((request) => {
      const collectionId = request.collectionId || null;
      if (!nextByCollection.has(collectionId)) nextByCollection.set(collectionId, nextOrderIn(collectionId));
      const order = nextByCollection.get(collectionId) ?? 1000;
      nextByCollection.set(collectionId, order + 1000);
      const name = uniqueName(request.name, requestNames);
      requestNames.push(name);
      return { ...request, collectionId, order, name };
    });
    collections.value = [...collections.value, ...remapped.collections];
    requests.value = [...requests.value, ...remapped.requests];
    headerDocuments.value = [...headerDocuments.value, ...remapped.headerDocuments];
    return {
      collections: remapped.collections.length,
      requests: remapped.requests.length,
      headerDocuments: remapped.headerDocuments.length,
    };
  }

  onMounted(async () => {
    await hydrateHttpClientStorage();
    loadAll();
  });
  onUnmounted(() => {
    persistCollections();
    persistRequests();
    persistEnvironments();
    persistHeaderDocuments();
    persistJars();
    persistHistory();
    persistRuns();
    persistSimulations();
    persistSettings();
    persistTabs();
  });

  return {
    collections,
    requests,
    environments,
    headerDocuments,
    jars,
    history,
    settings,
    openIds,
    activeId,
    response,
    responseError,
    responseDurationMs,
    testResults,
    timeline,
    loading,
    sessionVariables,
    lastExtracted,
    runSteps,
    runActive,
    runCursor,
    repoKey,
    visibleCollections,
    visibleRequests,
    visibleHeaderDocuments,
    activeRequest,
    openRequests,
    activeEnvironment,
    activeEnvironmentId,
    environmentVariables,
    activeJar,
    jarDomains,
    activeCookieCount,
    collectionHeadersFor,
    headerDocumentHeadersFor,
    inheritedHeadersFor,
    setSessionVariable,
    clearSessionVariables,
    createHeaderDocument,
    renameHeaderDocument,
    deleteHeaderDocument,
    reorderRequestInCollection,
    requestsForCollection,
    runActiveRequest,
    runCollection,
    runAll,
    stopRun,
    clearRun,
    bundleForRequest,
    bundleForCollection,
    bundleForWorkspace,
    bundleForHeaderDocuments,
    importBundle,
    openRequest,
    closeTab,
    newRequest,
    duplicateRequest,
    deleteRequest,
    renameRequest,
    createCollection,
    renameCollection,
    deleteCollection,
    moveRequestToCollection,
    importCurl,
    addEnvironment,
    deleteEnvironment,
    selectEnvironment,
    saveCookieToJar,
    removeCookieFromJar,
    clearJarDomain,
    clearAllCookies,
    upsertJarCookie,
    exportJar,
    importJar,
    curlForActive,
    copyAsCurl,
    copyText,
    sendActive,
    sendRequest,
    clearResponse,
    clearHistory,
    runStats,
    collectionStats,
    simulationResults,
    simulationActive,
    runSimulations,
    clearSimulationResults,
    simulationConfigFor,
    updateSimulationConfig,
    resetSimulationConfig,
    persistVariables,
  };
}

export type HttpClientController = ReturnType<typeof useHttpClient>;

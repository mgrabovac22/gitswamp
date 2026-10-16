<script setup lang="ts">
import { computed, ref } from "vue";
import {
  Send,
  Loader2,
  Wand2,
  Plus,
  Copy,
  Cookie as CookieIcon,
  ChevronDown,
  Sparkles,
  Lock,
  X,
  Link2,
  Settings2,
} from "lucide-vue-next";
import type { HttpClientController } from "../useHttpClient";
import type { HttpCookie, HttpExtractor, HttpExtractorSource, HttpKeyValue, HttpRequestDef } from "../httpClient.types";
import { HTTP_METHODS } from "../httpClient.types";
import HttpKeyValueTable from "./HttpKeyValueTable.vue";
import {
  createEmptyCookie,
  createEmptyExtractor,
  formatCookieExpiry,
  hostWithPortFromUrl,
  isCookieExpired,
  isHttps,
  jsonPretty,
  keyValue,
} from "../httpClient.utils";

const props = defineProps<{
  controller: HttpClientController;
}>();

const emit = defineEmits<{
  openCookieManager: [];
  openHeaderSets: [];
}>();

const editorTabs = [
  { id: "params", label: "Params" },
  { id: "headers", label: "Headers" },
  { id: "cookies", label: "Cookies" },
  { id: "auth", label: "Auth" },
  { id: "body", label: "Body" },
  { id: "scripts", label: "Scripts" },
  { id: "tests", label: "Tests" },
  { id: "extract", label: "Extract" },
  { id: "docs", label: "Docs" },
] as const;

type EditorTab = (typeof editorTabs)[number]["id"];

const activeTab = ref<EditorTab>("params");
const showPresets = ref(false);
const showAdvanced = ref(false);

const request = computed<HttpRequestDef | null>(() => props.controller.activeRequest.value);

const headerKeySuggestions = [
  "Authorization",
  "Content-Type",
  "Accept",
  "Cache-Control",
  "User-Agent",
  "X-API-Key",
  "If-None-Match",
  "Origin",
  "Referer",
];

const headerValueSuggestions = [
  "application/json",
  "application/x-www-form-urlencoded",
  "text/plain",
  "text/html",
  "no-cache",
  "Bearer {{token}}",
  "*/*",
];

const methodTone: Record<string, string> = {
  GET: "text-[#10b981]",
  POST: "text-[#f59e0b]",
  PUT: "text-[#3b82f6]",
  PATCH: "text-[#8b5cf6]",
  DELETE: "text-[#ef4444]",
  HEAD: "text-[#06b6d4]",
  OPTIONS: "text-[#a855f7]",
};

function updateRequest(patch: Partial<HttpRequestDef>): void {
  const target = request.value;
  if (!target) return;
  Object.assign(target, patch, { updatedAt: Date.now() });
}

function setHeaders(headers: HttpKeyValue[]): void {
  updateRequest({ headers });
}

function setParams(params: HttpKeyValue[]): void {
  updateRequest({ params });
}

function setFormBody(form: HttpKeyValue[]): void {
  const target = request.value;
  if (!target) return;
  target.body = { ...target.body, form };
  target.updatedAt = Date.now();
}

function setUrlencodedBody(urlencoded: HttpKeyValue[]): void {
  const target = request.value;
  if (!target) return;
  target.body = { ...target.body, urlencoded };
  target.updatedAt = Date.now();
}

function addHeader(key: string, value: string): void {
  const target = request.value;
  if (!target) return;
  const exists = target.headers.some((header) => header.key.toLowerCase() === key.toLowerCase());
  if (exists) {
    setHeaders(target.headers.map((header) => (header.key.toLowerCase() === key.toLowerCase() ? { ...header, value } : header)));
    return;
  }
  setHeaders([...target.headers, keyValue(key, value)]);
}

const autoHeaders = computed(() => {
  const target = request.value;
  if (!target) return [] as { name: string; value: string; reason: string }[];
  const list: { name: string; value: string; reason: string }[] = [];
  const host = hostWithPortFromUrl(target.url);
  if (host) list.push({ name: "Host", value: host, reason: "Derived from URL" });
  list.push({ name: "User-Agent", value: "GitSwamp/0.3 APIPulse", reason: "Default client" });
  if ((target.body.mode === "json" || target.body.mode === "text") && target.body.raw) {
    list.push({ name: "Content-Length", value: String(new TextEncoder().encode(target.body.raw).length), reason: "Body size" });
  }
  const derivedContentType =
    target.body.mode === "json"
      ? "application/json"
      : target.body.mode === "urlencoded"
        ? "application/x-www-form-urlencoded"
        : target.body.mode === "form"
          ? "multipart/form-data"
          : "";
  if (derivedContentType) list.push({ name: "Content-Type", value: derivedContentType, reason: "From body mode" });
  return list;
});

const inheritedHeaders = computed<HttpKeyValue[]>(() => props.controller.inheritedHeadersFor(request.value));

const missingVariables = computed(() => {
  const target = request.value;
  if (!target) return [] as string[];
  const variables = props.controller.environmentVariables.value;
  const found = new Set<string>();
  const scan = (value: string) => {
    const pattern = /\{\{\s*([^}$][^}]*?)\s*\}\}/g;
    let match: RegExpExecArray | null;
    while ((match = pattern.exec(value)) !== null) {
      if (variables[match[1]] === undefined) found.add(match[1]);
    }
  };
  scan(target.url);
  for (const header of target.headers) {
    scan(header.key);
    scan(header.value);
  }
  return Array.from(found);
});

const manualCookieHeader = computed<HttpKeyValue | null>(() => {
  const target = request.value;
  if (!target) return null;
  return target.headers.find((header) => header.key.toLowerCase() === "cookie") ?? null;
});

const manualCookieEnabled = computed({
  get: () => !!manualCookieHeader.value?.enabled,
  set: (value: boolean) => {
    const target = request.value;
    if (!target) return;
    const existing = manualCookieHeader.value;
    if (existing) {
      setHeaders(target.headers.map((header) => (header === existing ? { ...header, enabled: value } : header)));
    } else if (value) {
      setHeaders([...target.headers, keyValue("Cookie", "")]);
    }
  },
});

const manualCookieValue = computed({
  get: () => manualCookieHeader.value?.value ?? "",
  set: (value: string) => {
    const target = request.value;
    if (!target) return;
    const existing = manualCookieHeader.value;
    if (existing) {
      setHeaders(target.headers.map((header) => (header === existing ? { ...header, value } : header)));
    } else {
      setHeaders([...target.headers, keyValue("Cookie", value)]);
    }
  },
});

const requestJarCookies = computed(() => {
  const target = request.value;
  if (!target || !props.controller.settings.value.sendCookiesFromJar) return [];
  const host = hostWithPortFromUrl(target.url);
  if (!host) return [];
  const domain = host.split(":")[0];
  const list = props.controller.activeJar.value[domain] ?? [];
  return list.filter((cookie) => !isCookieExpired(cookie));
});

const headerCount = computed(() => request.value?.headers.filter((header) => header.enabled && header.key).length ?? 0);
const cookieCount = computed(() => {
  const own = request.value?.cookies.filter((cookie) => cookie.enabled && cookie.name).length ?? 0;
  return own + requestJarCookies.value.length;
});

function addRequestCookie(): void {
  const target = request.value;
  if (!target) return;
  target.cookies = [...target.cookies, createEmptyCookie(hostWithPortFromUrl(target.url).split(":")[0])];
}

function updateRequestCookie(index: number, patch: Partial<HttpCookie>): void {
  const target = request.value;
  if (!target) return;
  target.cookies = target.cookies.map((cookie, i) => (i === index ? { ...cookie, ...patch } : cookie));
}

function removeRequestCookie(index: number): void {
  const target = request.value;
  if (!target) return;
  target.cookies = target.cookies.filter((_, i) => i !== index);
}

function addJarCookieToRequest(cookie: HttpCookie): void {
  const target = request.value;
  if (!target) return;
  if (target.cookies.some((item) => item.name === cookie.name && item.domain === cookie.domain)) return;
  target.cookies = [...target.cookies, { ...cookie, id: `req_${Math.random().toString(36).slice(2, 9)}`, enabled: true }];
}

const presets: { name: string; headers: [string, string][] }[] = [
  { name: "JSON API", headers: [["Content-Type", "application/json"], ["Accept", "application/json"]] },
  { name: "Bearer auth", headers: [["Authorization", "Bearer {{token}}"]] },
  { name: "Form", headers: [["Content-Type", "application/x-www-form-urlencoded"]] },
  { name: "No-cache", headers: [["Cache-Control", "no-cache"], ["Pragma", "no-cache"]] },
];

function applyPreset(preset: { headers: [string, string][] }): void {
  for (const [key, value] of preset.headers) {
    addHeader(key, value);
  }
  showPresets.value = false;
}

function formatJsonBody(): void {
  const target = request.value;
  if (!target) return;
  target.body.raw = jsonPretty(target.body.raw);
}

const headerDocumentOptions = computed(() => props.controller.visibleHeaderDocuments.value);

const attachedHeaderDocuments = computed(() =>
  headerDocumentOptions.value.filter((document) => request.value?.headerDocumentIds.includes(document.id)),
);

function toggleHeaderDocument(id: string): void {
  const target = request.value;
  if (!target) return;
  target.headerDocumentIds = target.headerDocumentIds.includes(id)
    ? target.headerDocumentIds.filter((item) => item !== id)
    : [...target.headerDocumentIds, id];
  target.updatedAt = Date.now();
}

const extractorSources: { value: HttpExtractorSource; label: string; placeholder: string }[] = [
  { value: "json", label: "JSON path", placeholder: "data.token" },
  { value: "text", label: "Text / regex", placeholder: "token=([^&]+)" },
  { value: "regex", label: "Regex group", placeholder: "\"id\":\\s*\"([^\"]+)\"" },
  { value: "header", label: "Header", placeholder: "Location" },
  { value: "cookie", label: "Cookie", placeholder: "session" },
  { value: "status", label: "Status code", placeholder: "" },
];

function addExtractor(): void {
  const target = request.value;
  if (!target) return;
  target.extractors = [...target.extractors, createEmptyExtractor()];
  target.updatedAt = Date.now();
}

function removeExtractor(index: number): void {
  const target = request.value;
  if (!target) return;
  target.extractors = target.extractors.filter((_, itemIndex) => itemIndex !== index);
}

function updateExtractor(index: number, patch: Partial<HttpExtractor>): void {
  const target = request.value;
  if (!target) return;
  target.extractors = target.extractors.map((extractor, itemIndex) =>
    itemIndex === index ? { ...extractor, ...patch } : extractor,
  );
}

function extractorPlaceholder(source: HttpExtractorSource): string {
  return extractorSources.find((item) => item.value === source)?.placeholder ?? "";
}

const bodyPlaceholder = computed(() => {
  if (request.value?.body.mode === "json") {
    return '{\n  "name": "{{name}}"\n}';
  }
  return "Raw text body";
});

function bodyContentType(mode: string): string {
  if (mode === "json") return "JSON";
  if (mode === "text") return "Text";
  if (mode === "form") return "Form data";
  if (mode === "urlencoded") return "URL encoded";
  return "None";
}
</script>

<template>
  <div v-if="request" class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <div class="flex items-center gap-2 border-b border-[var(--border)] px-3 py-2">
      <div class="relative">
        <select
          :value="request.method"
          class="appearance-none rounded-l-md border border-[var(--border)] bg-[var(--input-background)] py-2 pl-2.5 pr-7 text-xs font-semibold focus:outline-none"
          :class="methodTone[request.method]"
          @change="updateRequest({ method: ($event.target as HTMLSelectElement).value as HttpRequestDef['method'] })"
        >
          <option v-for="method in HTTP_METHODS" :key="method" :value="method" class="text-[var(--foreground)]">{{ method }}</option>
        </select>
        <ChevronDown class="pointer-events-none absolute right-1.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--muted-foreground)]" />
      </div>

      <input
        :value="request.url"
        type="text"
        placeholder="https://{{baseUrl}}/users"
        spellcheck="false"
        class="min-w-0 flex-1 rounded-r-md border border-l-0 border-[var(--border)] bg-[var(--input-background)] px-2.5 py-2 font-mono text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
        @input="updateRequest({ url: ($event.target as HTMLInputElement).value })"
        @keydown.enter="controller.sendActive()"
      >

      <button
        type="button"
        class="inline-flex items-center gap-1.5 rounded-md bg-[var(--primary)] px-3.5 py-2 text-xs font-semibold text-white transition-opacity hover:opacity-90 disabled:opacity-60"
        :disabled="controller.loading.value"
        @click="controller.sendActive()"
      >
        <Loader2 v-if="controller.loading.value" class="h-3.5 w-3.5 animate-spin" />
        <Send v-else class="h-3.5 w-3.5" />
        Send
      </button>

      <button
        type="button"
        class="inline-flex items-center gap-1.5 rounded-md border border-[var(--border)] bg-[var(--secondary)] px-2.5 py-2 text-[11px] text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/40"
        title="Copy as cURL (with cookies)"
        @click="controller.copyAsCurl()"
      >
        <Copy class="h-3.5 w-3.5" />
      </button>

      <button
        type="button"
        class="inline-flex items-center gap-1.5 rounded-md border border-[var(--border)] bg-[var(--secondary)] px-2.5 py-2 text-[11px] text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/40"
        title="Open Cookie Manager"
        @click="emit('openCookieManager')"
      >
        <CookieIcon class="h-3.5 w-3.5" />
      </button>
    </div>

    <div
      v-if="missingVariables.length > 0"
      class="flex items-center gap-1.5 border-b border-[#f59e0b]/30 bg-[#f59e0b]/10 px-3 py-1 text-[10px] text-[#f59e0b]"
    >
      <Sparkles class="h-3 w-3" />
      Unresolved variables: {{ missingVariables.join(", ") }}
    </div>

    <div class="http-tabstrip flex items-center gap-0.5 overflow-x-auto border-b border-[var(--border)] px-2">
      <button
        v-for="tab in editorTabs"
        :key="tab.id"
        type="button"
        class="relative whitespace-nowrap px-2.5 py-2 text-[11px] font-medium transition-colors"
        :class="activeTab === tab.id && !showAdvanced ? 'text-[var(--primary)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
        @click="activeTab = tab.id; showAdvanced = false"
      >
        {{ tab.label }}
        <span v-if="tab.id === 'headers' && headerCount" class="ml-1 text-[9px] text-[var(--muted-foreground)]">({{ headerCount }})</span>
        <span v-if="tab.id === 'cookies' && cookieCount" class="ml-1 text-[9px] text-[var(--muted-foreground)]">({{ cookieCount }})</span>
        <span v-if="tab.id === 'params' && request.params.length" class="ml-1 text-[9px] text-[var(--muted-foreground)]">({{ request.params.length }})</span>
        <span
          v-if="activeTab === tab.id && !showAdvanced"
          class="absolute inset-x-1.5 bottom-0 h-0.5 rounded-full bg-[var(--primary)]"
        ></span>
      </button>
      <button
        type="button"
        class="relative ml-auto inline-flex shrink-0 items-center gap-1 whitespace-nowrap px-2.5 py-2 text-[11px] font-medium transition-colors"
        :class="showAdvanced ? 'text-[var(--primary)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
        title="Advanced settings"
        @click="showAdvanced = !showAdvanced"
      >
        <Settings2 class="h-3 w-3" /> Advanced
        <span v-if="request.iterations > 1" class="rounded bg-[var(--primary)]/15 px-1 text-[9px] text-[var(--primary)]">×{{ request.iterations }}</span>
        <span
          v-if="showAdvanced"
          class="absolute inset-x-1.5 bottom-0 h-0.5 rounded-full bg-[var(--primary)]"
        ></span>
      </button>
    </div>

    <div v-if="showAdvanced" class="flex-1 overflow-y-auto p-3">
      <div class="max-w-md rounded-md border border-[var(--border)] bg-[var(--secondary)]/20 p-3">
        <div class="mb-2 flex items-center gap-1.5 text-[11px] font-semibold text-[var(--foreground)]">
          <Settings2 class="h-3.5 w-3.5 text-[var(--primary)]" /> Advanced settings
        </div>
        <label class="flex items-center justify-between gap-3 py-1.5 text-[11px] text-[var(--muted-foreground)]">
          <span>Times this request runs in a run</span>
          <input
            type="number"
            min="1"
            max="500"
            class="w-20 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-0.5 text-[10px] text-[var(--foreground)] focus:outline-none"
            :value="request.iterations"
            @input="updateRequest({ iterations: Math.min(500, Math.max(1, Number(($event.target as HTMLInputElement).value) || 1)) })"
          >
        </label>
        <label class="flex items-center justify-between gap-3 py-1.5 text-[11px] text-[var(--muted-foreground)]">
          <span>Delay between runs (ms)</span>
          <input
            type="number"
            min="0"
            step="50"
            class="w-20 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-0.5 text-[10px] text-[var(--foreground)] focus:outline-none"
            :value="request.delayMs"
            @input="updateRequest({ delayMs: Math.max(0, Number(($event.target as HTMLInputElement).value) || 0) })"
          >
        </label>
        <p class="mt-2 text-[10px] text-[var(--muted-foreground)]">
          Used when this request is executed by a run (per collection or Run all) and by simulated tests.
        </p>
      </div>
    </div>

    <div v-else class="flex-1 overflow-y-auto p-3">
      <div v-if="activeTab === 'params'">
        <HttpKeyValueTable
          :model-value="request.params"
          key-placeholder="Parameter"
          value-placeholder="Value"
          show-description
          @update:model-value="setParams"
        />
      </div>

      <div v-else-if="activeTab === 'headers'" class="flex flex-col gap-3">
        <div class="flex items-center justify-between">
          <span class="text-[10px] text-[var(--muted-foreground)]">Headers sent with this request.</span>
          <div class="relative">
            <button
              type="button"
              class="inline-flex items-center gap-1 rounded-md border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/40"
              @click="showPresets = !showPresets"
            >
              <Wand2 class="h-3 w-3" /> Presets
            </button>
            <div
              v-if="showPresets"
              class="absolute right-0 top-7 z-20 w-40 overflow-hidden rounded-md border border-[var(--border)] bg-[var(--popover)] shadow-xl"
            >
              <button
                v-for="preset in presets"
                :key="preset.name"
                type="button"
                class="block w-full px-3 py-1.5 text-left text-[11px] text-[var(--foreground)] transition-colors hover:bg-[var(--primary)]/12"
                @click="applyPreset(preset)"
              >
                {{ preset.name }}
              </button>
            </div>
          </div>
        </div>

        <div v-if="attachedHeaderDocuments.length" class="flex flex-wrap items-center gap-1">
          <span class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Sets</span>
          <span
            v-for="document in attachedHeaderDocuments"
            :key="document.id"
            class="inline-flex items-center gap-1 rounded-full border border-[var(--primary)]/30 bg-[var(--primary)]/10 px-2 py-0.5 text-[10px] text-[var(--primary)]"
            :title="`${document.headers.length} header(s) from this set`"
          >
            <Link2 class="h-2.5 w-2.5" /> {{ document.name }}
            <button type="button" class="transition-colors hover:text-[#ef4444]" title="Detach" @click="toggleHeaderDocument(document.id)">
              <X class="h-2.5 w-2.5" />
            </button>
          </span>
        </div>

        <HttpKeyValueTable
          :model-value="request.headers"
          show-description
          allow-bulk
          :key-suggestions="headerKeySuggestions"
          :value-suggestions="headerValueSuggestions"
          empty-message="No headers. Add the first one or choose a preset."
          @update:model-value="setHeaders"
        />

        <div v-if="inheritedHeaders.length" class="rounded border border-[var(--border)] bg-[var(--secondary)]/30 p-2">
          <div class="mb-1 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
            <Lock class="h-3 w-3" />
            Inherited
          </div>
          <button
            v-for="header in inheritedHeaders"
            :key="header.id"
            type="button"
            class="flex w-full items-center gap-2 rounded px-1.5 py-1 text-left text-[11px] text-[var(--muted-foreground)] transition-colors hover:bg-[var(--primary)]/8 hover:text-[var(--foreground)]"
            title="Override for this request"
            @click="addHeader(header.key, header.value)"
          >
            <span class="font-medium">{{ header.key }}:</span>
            <span class="truncate font-mono">{{ header.value }}</span>
            <span class="ml-auto text-[9px] uppercase tracking-wider">override</span>
          </button>
        </div>

        <div class="rounded border border-[var(--border)] bg-[var(--secondary)]/30 p-2">
          <div class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">Auto-generated</div>
          <div
            v-for="header in autoHeaders"
            :key="header.name"
            class="flex items-center gap-2 px-1.5 py-1 text-[11px] text-[var(--muted-foreground)]"
          >
            <span class="font-medium">{{ header.name }}:</span>
            <span class="truncate font-mono">{{ header.value }}</span>
            <span class="ml-auto text-[9px]">{{ header.reason }}</span>
            <button type="button" class="text-[9px] uppercase tracking-wider text-[var(--primary)] hover:underline" @click="addHeader(header.name, header.value)">
              Override
            </button>
          </div>
        </div>

        <div class="flex items-center justify-between border-t border-[var(--border)]/60 pt-2">
          <span class="text-[10px] text-[var(--muted-foreground)]">Create a set once, attach it to any request.</span>
          <button
            type="button"
            class="inline-flex items-center gap-1 rounded-md border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/40 hover:text-[var(--primary)]"
            title="Create, edit and attach shared header sets"
            @click="emit('openHeaderSets')"
          >
            <Link2 class="h-3 w-3" /> Header sets…
          </button>
        </div>
      </div>

      <div v-else-if="activeTab === 'cookies'" class="flex flex-col gap-3">
        <div class="flex flex-wrap items-center gap-3">
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded-md bg-[var(--primary)] px-3 py-1.5 text-[11px] font-medium text-white transition-opacity hover:opacity-90"
            @click="addRequestCookie"
          >
            <Plus class="h-3 w-3" /> Add cookie
          </button>
          <label class="flex items-center gap-1.5 text-[11px] text-[var(--muted-foreground)]">
            <input
              type="checkbox"
              class="h-3.5 w-3.5 accent-[var(--primary)]"
              :checked="controller.settings.value.sendCookiesFromJar"
              @change="controller.settings.value.sendCookiesFromJar = ($event.target as HTMLInputElement).checked"
            >
            Send cookies from jar
          </label>
          <label class="flex items-center gap-1.5 text-[11px] text-[var(--muted-foreground)]">
            <input v-model="manualCookieEnabled" type="checkbox" class="h-3.5 w-3.5 accent-[var(--primary)]" >
            Manual Cookie header
          </label>
          <button type="button" class="ml-auto text-[10px] text-[var(--primary)] hover:underline" @click="emit('openCookieManager')">
            Open Cookie Manager
          </button>
        </div>

        <textarea
          v-if="manualCookieEnabled"
          :value="manualCookieValue"
          rows="2"
          spellcheck="false"
          placeholder="session=abc; theme=dark"
          class="w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] p-2 font-mono text-[11px] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          @input="manualCookieValue = ($event.target as HTMLTextAreaElement).value"
        ></textarea>

        <div class="overflow-hidden rounded border border-[var(--border)]">
          <table class="w-full border-collapse text-left">
            <thead class="bg-[var(--secondary)]/50 text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">
              <tr>
                <th class="w-6 px-2 py-1"></th>
                <th class="px-2 py-1 font-semibold">Name</th>
                <th class="px-2 py-1 font-semibold">Value</th>
                <th class="px-2 py-1 font-semibold">Domain</th>
                <th class="px-2 py-1 font-semibold">Path</th>
                <th class="px-2 py-1 font-semibold">Expires</th>
                <th class="px-2 py-1"></th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(cookie, index) in request.cookies"
                :key="cookie.id"
                class="border-b border-[var(--border)]/50"
                :class="!cookie.enabled ? 'opacity-45' : ''"
              >
                <td class="px-2 py-1">
                  <input type="checkbox" class="h-3.5 w-3.5 accent-[var(--primary)]" :checked="cookie.enabled" @change="updateRequestCookie(index, { enabled: ($event.target as HTMLInputElement).checked })" >
                </td>
                <td class="px-2 py-1">
                  <input :value="cookie.name" class="w-full bg-transparent text-[11px] text-[var(--foreground)] focus:outline-none" placeholder="name" @input="updateRequestCookie(index, { name: ($event.target as HTMLInputElement).value })" >
                </td>
                <td class="px-2 py-1">
                  <input :value="cookie.value" class="w-full bg-transparent font-mono text-[11px] text-[var(--foreground)] focus:outline-none" placeholder="value" @input="updateRequestCookie(index, { value: ($event.target as HTMLInputElement).value })" >
                </td>
                <td class="px-2 py-1">
                  <input :value="cookie.domain" class="w-full bg-transparent text-[10px] text-[var(--muted-foreground)] focus:outline-none" placeholder="domain" @input="updateRequestCookie(index, { domain: ($event.target as HTMLInputElement).value })" >
                </td>
                <td class="px-2 py-1">
                  <input :value="cookie.path" class="w-full bg-transparent text-[10px] text-[var(--muted-foreground)] focus:outline-none" placeholder="/" @input="updateRequestCookie(index, { path: ($event.target as HTMLInputElement).value })" >
                </td>
                <td class="whitespace-nowrap px-2 py-1 text-[10px] text-[var(--muted-foreground)]">{{ formatCookieExpiry(cookie.expires) }}</td>
                <td class="px-2 py-1 text-right">
                  <button type="button" class="text-[var(--muted-foreground)] hover:text-[#ef4444]" @click="removeRequestCookie(index)">×</button>
                </td>
              </tr>
              <tr
                v-for="cookie in requestJarCookies"
                :key="cookie.id"
                class="border-b border-[var(--border)]/50 text-[var(--muted-foreground)]"
              >
                <td class="px-2 py-1">
                  <CookieIcon class="h-3.5 w-3.5" />
                </td>
                <td class="px-2 py-1 text-[11px]">
                  {{ cookie.name }}
                  <span class="ml-1 rounded bg-[var(--secondary)] px-1 text-[9px] uppercase tracking-wider">from jar</span>
                </td>
                <td class="max-w-[160px] truncate px-2 py-1 font-mono text-[10px]">{{ cookie.value }}</td>
                <td class="px-2 py-1 text-[10px]">{{ cookie.domain }}</td>
                <td class="px-2 py-1 text-[10px]">{{ cookie.path }}</td>
                <td class="whitespace-nowrap px-2 py-1 text-[10px]">{{ formatCookieExpiry(cookie.expires) }}</td>
                <td class="px-2 py-1 text-right">
                  <button type="button" class="text-[10px] text-[var(--primary)] hover:underline" title="Copy into request" @click="addJarCookieToRequest(cookie)">
                    use
                  </button>
                </td>
              </tr>
              <tr v-if="request.cookies.length === 0 && requestJarCookies.length === 0">
                <td colspan="7" class="px-2 py-8 text-center text-[11px] text-[var(--muted-foreground)]">
                  No cookies for this domain.
                  <button type="button" class="ml-1 text-[var(--primary)] hover:underline" @click="addRequestCookie">Add cookie</button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="isHttps(request.url) === false && request.url.trim()" class="flex items-center gap-1.5 text-[10px] text-[#f59e0b]">
          <Lock class="h-3 w-3" />
          Secure cookies will not be sent over an insecure connection.
        </div>
      </div>

      <div v-else-if="activeTab === 'auth'" class="flex flex-col gap-3">
        <div class="flex items-center gap-2">
          <select
            :value="request.auth.type"
            class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-[11px] text-[var(--foreground)] focus:outline-none"
            @change="updateRequest({ auth: { ...request.auth, type: ($event.target as HTMLSelectElement).value as HttpRequestDef['auth']['type'] } })"
          >
            <option value="none">No auth</option>
            <option value="bearer">Bearer token</option>
            <option value="basic">Basic auth</option>
            <option value="apikey">API key</option>
          </select>
        </div>

        <div v-if="request.auth.type === 'bearer'" class="flex flex-col gap-1">
          <label class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Token</label>
          <input :value="request.auth.bearerToken" placeholder="{{token}}" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] focus:outline-none" @input="updateRequest({ auth: { ...request.auth, bearerToken: ($event.target as HTMLInputElement).value } })" >
        </div>

        <div v-else-if="request.auth.type === 'basic'" class="grid grid-cols-2 gap-2">
          <label class="flex flex-col gap-1">
            <span class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Username</span>
            <input :value="request.auth.basicUser" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-[11px] text-[var(--foreground)] focus:outline-none" @input="updateRequest({ auth: { ...request.auth, basicUser: ($event.target as HTMLInputElement).value } })" >
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Password</span>
            <input :value="request.auth.basicPassword" type="password" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-[11px] text-[var(--foreground)] focus:outline-none" @input="updateRequest({ auth: { ...request.auth, basicPassword: ($event.target as HTMLInputElement).value } })" >
          </label>
        </div>

        <div v-else-if="request.auth.type === 'apikey'" class="grid grid-cols-2 gap-2">
          <label class="flex flex-col gap-1">
            <span class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Key</span>
            <input :value="request.auth.apiKeyName" placeholder="X-API-Key" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-[11px] text-[var(--foreground)] focus:outline-none" @input="updateRequest({ auth: { ...request.auth, apiKeyName: ($event.target as HTMLInputElement).value } })" >
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Value</span>
            <input :value="request.auth.apiKeyValue" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[11px] text-[var(--foreground)] focus:outline-none" @input="updateRequest({ auth: { ...request.auth, apiKeyValue: ($event.target as HTMLInputElement).value } })" >
          </label>
          <label class="col-span-2 flex items-center gap-1.5 text-[11px] text-[var(--muted-foreground)]">
            <input type="checkbox" class="h-3.5 w-3.5 accent-[var(--primary)]" :checked="request.auth.apiKeyIn === 'query'" @change="updateRequest({ auth: { ...request.auth, apiKeyIn: ($event.target as HTMLInputElement).checked ? 'query' : 'header' } })" >
            Add to query string instead of header
          </label>
        </div>

        <p v-else class="text-[11px] text-[var(--muted-foreground)]">
          No authentication. Add it here instead of setting headers manually.
        </p>
      </div>

      <div v-else-if="activeTab === 'body'" class="flex flex-col gap-2">
        <div class="flex flex-wrap items-center gap-1">
          <button
            v-for="mode in ['none', 'json', 'text', 'form', 'urlencoded']"
            :key="mode"
            type="button"
            class="rounded px-2 py-1 text-[11px] transition-colors"
            :class="request.body.mode === mode ? 'bg-[var(--primary)]/15 text-[var(--primary)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
            @click="updateRequest({ body: { ...request.body, mode: mode as HttpRequestDef['body']['mode'] } })"
          >
            {{ bodyContentType(mode) }}
          </button>
          <button
            v-if="request.body.mode === 'json' || request.body.mode === 'text'"
            type="button"
            class="ml-auto inline-flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--foreground)]"
            @click="formatJsonBody"
          >
            <Wand2 class="h-3 w-3" /> Format
          </button>
        </div>

        <template v-if="request.body.mode === 'json' || request.body.mode === 'text'">
          <textarea
            :value="request.body.raw"
            rows="12"
            spellcheck="false"
            :placeholder="bodyPlaceholder"
            class="w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] p-2.5 font-mono text-[11px] leading-relaxed text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            @input="updateRequest({ body: { ...request.body, raw: ($event.target as HTMLTextAreaElement).value } })"
          ></textarea>
        </template>

        <template v-else-if="request.body.mode === 'form'">
          <HttpKeyValueTable
            :model-value="request.body.form"
            key-placeholder="Field"
            value-placeholder="Value"
            @update:model-value="setFormBody"
          />
        </template>

        <template v-else-if="request.body.mode === 'urlencoded'">
          <HttpKeyValueTable
            :model-value="request.body.urlencoded"
            key-placeholder="Field"
            value-placeholder="Value"
            @update:model-value="setUrlencodedBody"
          />
        </template>

        <p v-else class="py-10 text-center text-[11px] text-[var(--muted-foreground)]">This request has no body.</p>
      </div>

      <div v-else-if="activeTab === 'scripts'" class="flex h-full flex-col gap-2">
        <p class="text-[10px] text-[var(--muted-foreground)]">
          Runs before the request. Use <code class="text-[var(--primary)]">pm.environment.set("name", "value")</code> to create variables for <code class="text-[var(--primary)]" v-text="'{{name}}'"></code>.
        </p>
        <textarea
          :value="request.scripts"
          rows="14"
          spellcheck="false"
          placeholder="// pm.environment.set('token', 'abc')"
          class="w-full flex-1 resize-y rounded border border-[var(--border)] bg-[var(--input-background)] p-2.5 font-mono text-[11px] leading-relaxed text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          @input="updateRequest({ scripts: ($event.target as HTMLTextAreaElement).value })"
        ></textarea>
      </div>

      <div v-else-if="activeTab === 'tests'" class="flex h-full flex-col gap-2">
        <p class="text-[10px] text-[var(--muted-foreground)]">
          Assertions run after the response. Use <code class="text-[var(--primary)]">pm.test("name", () =&gt; pm.expect(pm.response.code).toBe(200))</code>.
        </p>
        <textarea
          :value="request.tests"
          rows="14"
          spellcheck="false"
          placeholder="pm.test('status is 200', () => pm.expect(pm.response.code).toBe(200));"
          class="w-full flex-1 resize-y rounded border border-[var(--border)] bg-[var(--input-background)] p-2.5 font-mono text-[11px] leading-relaxed text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          @input="updateRequest({ tests: ($event.target as HTMLTextAreaElement).value })"
        ></textarea>
      </div>

      <div v-else-if="activeTab === 'extract'" class="flex flex-col gap-2">
        <p class="text-[10px] text-[var(--muted-foreground)]">
          Capture values from this response into variables that later requests can use as
          <code class="text-[var(--primary)]" v-text="'{{name}}'"></code>, even across collections.
        </p>

        <div class="flex items-center justify-between">
          <span class="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">Extractors</span>
          <button
            type="button"
            class="inline-flex items-center gap-1 rounded-md bg-[var(--primary)] px-2.5 py-1 text-[10px] font-medium text-white transition-opacity hover:opacity-90"
            @click="addExtractor"
          >
            <Plus class="h-3 w-3" /> Add extractor
          </button>
        </div>

        <div v-if="request.extractors.length === 0" class="rounded border border-dashed border-[var(--border)] py-8 text-center text-[11px] text-[var(--muted-foreground)]">
          No extractors yet.
        </div>

        <div v-else class="overflow-hidden rounded border border-[var(--border)]">
          <div class="grid items-center gap-2 border-b border-[var(--border)] bg-[var(--secondary)]/50 px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]"
            :style="{ gridTemplateColumns: '24px 1fr 130px 1.4fr 110px 28px' }">
            <span></span>
            <span>Variable</span>
            <span>Source</span>
            <span>Expression</span>
            <span>Scope</span>
            <span></span>
          </div>
          <div
            v-for="(extractor, index) in request.extractors"
            :key="extractor.id"
            class="grid items-center gap-2 border-b border-[var(--border)]/60 px-2 py-1"
            :class="!extractor.enabled ? 'opacity-45' : ''"
            :style="{ gridTemplateColumns: '24px 1fr 130px 1.4fr 110px 28px' }"
          >
            <input
              type="checkbox"
              class="h-3.5 w-3.5 accent-[var(--primary)]"
              :checked="extractor.enabled"
              @change="updateExtractor(index, { enabled: ($event.target as HTMLInputElement).checked })"
            >
            <input
              :value="extractor.variable"
              placeholder="token"
              class="rounded bg-transparent px-1.5 py-1 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:bg-[var(--input-background)] focus:outline-none"
              @input="updateExtractor(index, { variable: ($event.target as HTMLInputElement).value })"
            >
            <select
              :value="extractor.source"
              class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1 py-1 text-[11px] text-[var(--foreground)] focus:outline-none"
              @change="updateExtractor(index, { source: ($event.target as HTMLSelectElement).value as HttpExtractorSource })"
            >
              <option v-for="option in extractorSources" :key="option.value" :value="option.value">{{ option.label }}</option>
            </select>
            <input
              :value="extractor.expression"
              :placeholder="extractorPlaceholder(extractor.source)"
              :disabled="extractor.source === 'status'"
              class="rounded bg-transparent px-1.5 py-1 font-mono text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:bg-[var(--input-background)] focus:outline-none disabled:opacity-40"
              @input="updateExtractor(index, { expression: ($event.target as HTMLInputElement).value })"
            >
            <select
              :value="extractor.scope"
              class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1 py-1 text-[11px] text-[var(--foreground)] focus:outline-none"
              @change="updateExtractor(index, { scope: ($event.target as HTMLSelectElement).value as HttpExtractor['scope'] })"
            >
              <option value="session">Session</option>
              <option value="environment">Environment</option>
            </select>
            <button type="button" class="flex h-6 w-6 items-center justify-center rounded text-[var(--muted-foreground)] transition-colors hover:bg-[#ef4444]/15 hover:text-[#ef4444]" @click="removeExtractor(index)">
              <X class="h-3 w-3" />
            </button>
          </div>
        </div>

        <div v-if="Object.keys(controller.lastExtracted.value).length" class="rounded border border-[var(--border)] bg-[var(--secondary)]/30 p-2">
          <div class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">Last captured</div>
          <div v-for="(value, name) in controller.lastExtracted.value" :key="name" class="flex items-baseline gap-1 py-0.5 text-[10px]">
            <span class="font-mono text-[var(--primary)]">{{ name }}</span>
            <span class="text-[var(--muted-foreground)]">=</span>
            <span class="min-w-0 flex-1 truncate font-mono text-[var(--foreground)]" :title="String(value)">{{ value }}</span>
          </div>
        </div>
      </div>

      <div v-else-if="activeTab === 'docs'" class="flex h-full flex-col gap-2">
        <textarea
          :value="request.docs"
          rows="14"
          spellcheck="false"
          placeholder="Document what this request does, expected responses, parameters..."
          class="w-full flex-1 resize-y rounded border border-[var(--border)] bg-[var(--input-background)] p-2.5 text-[11px] leading-relaxed text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          @input="updateRequest({ docs: ($event.target as HTMLTextAreaElement).value })"
        ></textarea>
      </div>
    </div>
  </div>

  <div v-else class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
    <Send class="h-8 w-8 text-[var(--muted-foreground)]" />
    <p class="text-xs text-[var(--muted-foreground)]">Open or create a request to start.</p>
    <button
      type="button"
      class="inline-flex items-center gap-1.5 rounded-md bg-[var(--primary)] px-3 py-1.5 text-[11px] font-medium text-white"
      @click="controller.newRequest()"
    >
      <Plus class="h-3 w-3" /> New request
    </button>
  </div>
</template>

<style scoped>
.http-tabstrip {
  scrollbar-width: thin;
  scrollbar-color: transparent transparent;
}

.http-tabstrip:hover {
  scrollbar-color: #8a8f98 transparent;
}

.http-tabstrip::-webkit-scrollbar {
  height: 2px;
}

.http-tabstrip::-webkit-scrollbar-track {
  background: transparent;
}

.http-tabstrip::-webkit-scrollbar-thumb {
  background: transparent;
}

.http-tabstrip:hover::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.25);
}
</style>

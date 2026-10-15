<script setup lang="ts">
import { computed, ref } from "vue";
import {
  Clock,
  Download,
  Copy,
  Image as ImageIcon,
  Search,
  ShieldCheck,
  ShieldOff,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  X,
  FileJson,
  FileCode2,
  FileText,
  Save,
} from "lucide-vue-next";
import { useToast } from "@/shared/notifications/useToast";
import { highlightCode } from "@/shared/config/code-view";
import { saveTextFile } from "../httpClientFiles";
import type { HttpClientController } from "../useHttpClient";
import type { HttpTimelineEntry } from "../httpClient.types";
import {
  formatBytes,
  formatDuration,
  jsonPretty,
  parseSetCookie,
  statusTone,
} from "../httpClient.utils";

const props = defineProps<{
  controller: HttpClientController;
}>();

const toast = useToast();

const activeTab = ref<"body" | "headers" | "cookies" | "timeline" | "tests">("body");
const viewMode = ref<"pretty" | "raw" | "hex">("pretty");
const search = ref("");
const showSaveDialog = ref(false);
const saveFormat = ref<"original" | "json" | "html" | "text">("original");
const saving = ref(false);

const response = computed(() => props.controller.response.value);
const error = computed(() => props.controller.responseError.value);

const contentType = computed(() => {
  const header = response.value?.headers.find((item) => item.name.toLowerCase() === "content-type");
  return header?.value ?? "";
});

const isImage = computed(() => /^image\//i.test(contentType.value) && !!response.value?.bodyBase64);

const previewBody = computed(() => {
  const body = response.value?.body ?? "";
  if (viewMode.value !== "pretty") return body;
  if (contentType.value.includes("json") || /^[\s]*[[{]/.test(body.trim())) {
    return jsonPretty(body);
  }
  return body;
});

const highlightLanguage = computed<string | null>(() => {
  const ct = contentType.value.toLowerCase();
  if (ct.includes("json")) return "json";
  if (ct.includes("html")) return "xml";
  if (ct.includes("xml")) return "xml";
  if (ct.includes("javascript")) return "javascript";
  if (ct.includes("css")) return "css";
  const trimmed = (response.value?.body ?? "").trim();
  if (/^[[{]/.test(trimmed)) return "json";
  if (/^</.test(trimmed)) return "xml";
  return null;
});

const highlightedBody = computed(() => highlightCode(previewBody.value, highlightLanguage.value));

const useHighlight = computed(
  () => viewMode.value === "pretty" && !search.value.trim() && !isImage.value && highlightLanguage.value !== null,
);

const bodySegments = computed(() => {
  const text = previewBody.value;
  const query = search.value.trim();
  if (!query) return [{ text, match: false }];
  const segments: { text: string; match: boolean }[] = [];
  const lower = text.toLowerCase();
  const needle = query.toLowerCase();
  let index = 0;
  while (index < text.length) {
    const found = lower.indexOf(needle, index);
    if (found < 0) {
      segments.push({ text: text.slice(index), match: false });
      break;
    }
    if (found > index) segments.push({ text: text.slice(index, found), match: false });
    segments.push({ text: text.slice(found, found + needle.length), match: true });
    index = found + needle.length;
  }
  return segments;
});

const hexDump = computed(() => {
  const body = response.value;
  if (!body) return "";
  const bytes = base64ToBytes(body.bodyBase64 ?? "") ?? new TextEncoder().encode(body.body);
  const lines: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += 16) {
    const slice = bytes.slice(offset, offset + 16);
    const hex = Array.from(slice).map((byte) => byte.toString(16).padStart(2, "0")).join(" ");
    const ascii = Array.from(slice).map((byte) => (byte >= 32 && byte < 127 ? String.fromCharCode(byte) : ".")).join("");
    lines.push(`${offset.toString(16).padStart(8, "0")}  ${hex.padEnd(47, " ")}  ${ascii}`);
    if (lines.length > 4000) {
      lines.push("… truncated");
      break;
    }
  }
  return lines.join("\n");
});

const responseCookies = computed(() => {
  const body = response.value;
  if (!body) return [];
  const fallback = hostnameFromUrl(body.finalUrl);
  return body.headers
    .filter((header) => header.name.toLowerCase() === "set-cookie")
    .map((header) => parseSetCookie(header.value, fallback))
    .filter((cookie): cookie is NonNullable<typeof cookie> => cookie !== null);
});

const tone = computed(() => statusTone(response.value?.status ?? 0));

const toneClass: Record<string, string> = {
  success: "text-[#10b981]",
  redirect: "text-[#3b82f6]",
  warning: "text-[#f59e0b]",
  danger: "text-[#ef4444]",
  muted: "text-[var(--muted-foreground)]",
};

const passedTests = computed(() => props.controller.testResults.value.filter((test) => test.passed).length);
const failedTests = computed(() => props.controller.testResults.value.filter((test) => !test.passed).length);

function hostnameFromUrl(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}

function base64ToBytes(value: string): Uint8Array | null {
  if (!value) return null;
  try {
    const binary = atob(value);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
    return bytes;
  } catch {
    return null;
  }
}

function copyBody(): void {
  props.controller.copyText(previewBody.value, "Response body copied.");
}

function saveBody(): void {
  if (!response.value) return;
  saveFormat.value = "original";
  showSaveDialog.value = true;
}

function originalExtension(): string {
  const body = response.value;
  if (!body) return "txt";
  const name = fileNameFromUrl(body.finalUrl || "");
  const ext = name.includes(".") ? name.split(".").pop() ?? "" : "";
  if (ext) return ext;
  const ct = contentType.value.toLowerCase();
  if (ct.includes("json")) return "json";
  if (ct.includes("html")) return "html";
  if (ct.includes("xml")) return "xml";
  if (ct.includes("csv")) return "csv";
  if (ct.includes("javascript")) return "js";
  return "txt";
}

function buildSavePayload(): { content: string; extension: string; filters: { name: string; extensions: string[] }[] } {
  const body = response.value?.body ?? "";
  switch (saveFormat.value) {
    case "json": {
      let content = body;
      try {
        content = JSON.stringify(JSON.parse(body), null, 2);
      } catch {
        content = jsonPretty(body);
      }
      return { content, extension: "json", filters: [{ name: "JSON", extensions: ["json"] }] };
    }
    case "html": {
      const content =
        /^\s*</.test(body) && /<\/html>/i.test(body)
          ? body
          : `<!doctype html>\n<html>\n<head>\n<meta charset="utf-8">\n<title>Response</title>\n</head>\n<body>\n<pre>${body
              .replace(/&/g, "&amp;")
              .replace(/</g, "&lt;")
              .replace(/>/g, "&gt;")}</pre>\n</body>\n</html>\n`;
      return { content, extension: "html", filters: [{ name: "HTML", extensions: ["html", "htm"] }] };
    }
    case "text":
      return { content: body, extension: "txt", filters: [{ name: "Text", extensions: ["txt"] }] };
    default: {
      const extension = originalExtension();
      return { content: body, extension, filters: [{ name: "All files", extensions: ["*"] }] };
    }
  }
}

async function performSave(): Promise<void> {
  if (saving.value) return;
  saving.value = true;
  try {
    const { content, extension, filters } = buildSavePayload();
    const baseName = fileNameFromUrl(response.value?.finalUrl || "").replace(/\.[^.]+$/, "") || "response";
    const saved = await saveTextFile(baseName, content, filters, extension);
    if (saved) {
      toast.success("Response saved.");
      showSaveDialog.value = false;
    }
  } catch (error) {
    toast.error(`Save failed: ${String(error)}`);
  } finally {
    saving.value = false;
  }
}

function fileNameFromUrl(url: string): string {
  try {
    const path = new URL(url).pathname;
    const last = path.split("/").filter(Boolean).pop();
    return last || "response";
  } catch {
    return "response";
  }
}

function timelineSegments(entries: HttpTimelineEntry[]): { entry: HttpTimelineEntry; percent: number }[] {
  const total = entries.reduce((sum, entry) => sum + entry.durationMs, 0) || 1;
  return entries.map((entry) => ({ entry, percent: Math.max(2, (entry.durationMs / total) * 100) }));
}

const timelineTone: Record<string, string> = {
  request: "bg-[#8b5cf6]",
  response: "bg-[#10b981]",
  info: "bg-[#06b6d4]",
};
</script>

<template>
  <div class="relative flex min-h-0 flex-1 flex-col overflow-hidden border-t border-[var(--border)]">
    <div class="flex items-center gap-3 border-b border-[var(--border)] px-3 py-2">
      <template v-if="response">
        <span class="text-sm font-semibold" :class="toneClass[tone]">
          {{ response.status }} {{ response.statusText }}
        </span>
        <span class="inline-flex items-center gap-1 text-[11px] text-[var(--muted-foreground)]">
          <Clock class="h-3 w-3" /> {{ formatDuration(response.durationMs) }}
        </span>
        <span class="text-[11px] text-[var(--muted-foreground)]">{{ formatBytes(response.sizeBytes) }}</span>
        <span v-if="response.finalUrl && response.finalUrl !== controller.activeRequest.value?.url" class="max-w-[280px] truncate font-mono text-[10px] text-[var(--muted-foreground)]" :title="response.finalUrl">
          → {{ response.finalUrl }}
        </span>
      </template>
      <span v-else class="text-xs text-[var(--muted-foreground)]">No response yet.</span>

      <button
        v-if="response || error"
        type="button"
        class="ml-auto text-[10px] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
        @click="controller.clearResponse()"
      >
        Clear
      </button>
    </div>

    <div class="flex items-center gap-0.5 border-b border-[var(--border)] px-2">
      <button
        v-for="tab in ['body', 'headers', 'cookies', 'timeline', 'tests'] as const"
        :key="tab"
        type="button"
        class="relative px-2.5 py-2 text-[11px] font-medium capitalize transition-colors"
        :class="activeTab === tab ? 'text-[var(--primary)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
        @click="activeTab = tab"
      >
        {{ tab }}
        <span v-if="tab === 'headers' && response" class="ml-1 text-[9px] text-[var(--muted-foreground)]">({{ response.headers.length }})</span>
        <span v-if="tab === 'cookies' && responseCookies.length" class="ml-1 text-[9px] text-[var(--muted-foreground)]">({{ responseCookies.length }})</span>
        <span v-if="tab === 'tests' && controller.testResults.value.length" class="ml-1 text-[9px]" :class="failedTests ? 'text-[#ef4444]' : 'text-[#10b981]'">
          ({{ passedTests }}/{{ controller.testResults.value.length }})
        </span>
        <span v-if="activeTab === tab" class="absolute inset-x-1.5 bottom-0 h-0.5 rounded-full bg-[var(--primary)]"></span>
      </button>
    </div>

    <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div v-if="error" class="flex flex-1 flex-col items-start gap-2 overflow-y-auto p-4">
        <div class="flex items-center gap-1.5 text-sm font-semibold text-[#ef4444]">
          <AlertTriangle class="h-4 w-4" /> Request failed
        </div>
        <pre class="w-full whitespace-pre-wrap rounded border border-[#ef4444]/30 bg-[#ef4444]/8 p-3 font-mono text-[11px] text-[var(--foreground)]">{{ error }}</pre>
      </div>

      <template v-else-if="response">
        <div v-if="activeTab === 'body'" class="flex min-h-0 flex-1 flex-col">
          <div class="flex items-center gap-2 border-b border-[var(--border)] px-3 py-1.5">
            <div class="flex items-center gap-0.5">
              <button
                v-for="mode in ['pretty', 'raw', 'hex'] as const"
                :key="mode"
                type="button"
                class="rounded px-2 py-0.5 text-[10px] capitalize transition-colors"
                :class="viewMode === mode ? 'bg-[var(--primary)]/15 text-[var(--primary)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="viewMode = mode"
              >
                {{ mode }}
              </button>
            </div>
            <div class="relative">
              <Search class="pointer-events-none absolute left-1.5 top-1/2 h-3 w-3 -translate-y-1/2 text-[var(--muted-foreground)]" />
              <input
                v-model="search"
                type="text"
                placeholder="Search"
                class="w-40 rounded border border-[var(--border)] bg-[var(--input-background)] py-0.5 pl-6 pr-2 text-[10px] text-[var(--foreground)] focus:outline-none"
              >
            </div>
            <span class="text-[10px] text-[var(--muted-foreground)]">{{ contentType || "unknown type" }}</span>
            <button type="button" class="ml-auto inline-flex items-center gap-1 text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)]" @click="copyBody">
              <Copy class="h-3 w-3" /> Copy
            </button>
            <button type="button" class="inline-flex items-center gap-1 text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)]" @click="saveBody">
              <Download class="h-3 w-3" /> Save
            </button>
          </div>

          <div class="min-h-0 flex-1 overflow-auto p-3">
            <div v-if="isImage" class="flex flex-col items-center gap-2">
              <ImageIcon class="h-4 w-4 text-[var(--muted-foreground)]" />
              <img :src="`data:${contentType};base64,${response.bodyBase64}`" alt="Response image" class="max-h-[70vh] max-w-full rounded border border-[var(--border)]" >
            </div>
            <pre v-else-if="viewMode === 'hex'" class="whitespace-pre font-mono text-[10px] leading-tight text-[var(--foreground)]">{{ hexDump }}</pre>
            <pre v-else-if="useHighlight" class="whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed text-[var(--foreground)]"><code class="hljs bg-transparent p-0" v-html="highlightedBody"></code></pre>
            <pre v-else class="whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed text-[var(--foreground)]"><template v-for="(segment, index) in bodySegments" :key="index"><mark v-if="segment.match" class="rounded bg-[#f59e0b]/40 text-[var(--foreground)]">{{ segment.text }}</mark><span v-else>{{ segment.text }}</span></template></pre>
          </div>
        </div>

        <div v-else-if="activeTab === 'headers'" class="min-h-0 flex-1 overflow-auto p-3">
          <table class="w-full border-collapse text-left">
            <tbody>
              <tr v-for="(header, index) in response.headers" :key="index" class="border-b border-[var(--border)]/50">
                <td class="w-1/3 align-top py-1.5 pr-3 font-mono text-[11px] font-semibold text-[var(--primary)]">{{ header.name }}</td>
                <td class="break-all py-1.5 font-mono text-[11px] text-[var(--foreground)]">{{ header.value }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-else-if="activeTab === 'cookies'" class="min-h-0 flex-1 overflow-auto p-3">
          <div v-if="responseCookies.length === 0" class="py-10 text-center text-[11px] text-[var(--muted-foreground)]">No cookies in the response.</div>
          <table v-else class="w-full border-collapse text-left">
            <thead class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">
              <tr>
                <th class="px-2 py-1 font-semibold">Name</th>
                <th class="px-2 py-1 font-semibold">Value</th>
                <th class="px-2 py-1 font-semibold">Domain</th>
                <th class="px-2 py-1 font-semibold">Path</th>
                <th class="px-2 py-1 font-semibold">Flags</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="cookie in responseCookies" :key="cookie.id" class="border-b border-[var(--border)]/50">
                <td class="px-2 py-1 text-[11px] text-[var(--foreground)]">{{ cookie.name }}</td>
                <td class="max-w-[220px] truncate px-2 py-1 font-mono text-[10px] text-[var(--foreground)]">{{ cookie.value }}</td>
                <td class="px-2 py-1 text-[10px] text-[var(--muted-foreground)]">{{ cookie.domain }}</td>
                <td class="px-2 py-1 text-[10px] text-[var(--muted-foreground)]">{{ cookie.path }}</td>
                <td class="flex items-center gap-1 px-2 py-1 text-[10px] text-[var(--muted-foreground)]">
                  <ShieldCheck v-if="cookie.secure" class="h-3 w-3 text-[#10b981]" title="Secure" />
                  <ShieldOff v-else class="h-3 w-3 opacity-40" title="Not secure" />
                  <span v-if="cookie.httpOnly" class="rounded bg-[var(--secondary)] px-1 text-[9px]">HttpOnly</span>
                  <span v-if="cookie.sameSite" class="rounded bg-[var(--secondary)] px-1 text-[9px]">{{ cookie.sameSite }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-else-if="activeTab === 'timeline'" class="min-h-0 flex-1 overflow-auto p-3">
          <div class="mb-3 flex h-2 w-full overflow-hidden rounded-full bg-[var(--secondary)]">
            <div
              v-for="segment in timelineSegments(controller.timeline.value)"
              :key="segment.entry.label"
              class="h-full"
              :class="timelineTone[segment.entry.tone]"
              :style="{ width: `${segment.percent}%` }"
              :title="`${segment.entry.label}: ${formatDuration(segment.entry.durationMs)}`"
            ></div>
          </div>
          <div
            v-for="entry in controller.timeline.value"
            :key="entry.label"
            class="flex items-center gap-2 border-b border-[var(--border)]/50 py-1.5"
          >
            <span class="h-2 w-2 rounded-full" :class="timelineTone[entry.tone]"></span>
            <span class="text-[11px] text-[var(--foreground)]">{{ entry.label }}</span>
            <span v-if="entry.detail" class="text-[10px] text-[var(--muted-foreground)]">· {{ entry.detail }}</span>
            <span class="ml-auto font-mono text-[10px] text-[var(--muted-foreground)]">{{ formatDuration(entry.durationMs) }}</span>
          </div>
        </div>

        <div v-else-if="activeTab === 'tests'" class="min-h-0 flex-1 overflow-auto p-3">
          <div v-if="controller.testResults.value.length === 0" class="py-10 text-center text-[11px] text-[var(--muted-foreground)]">
            No tests defined. Add assertions in the Tests tab.
          </div>
          <div
            v-for="test in controller.testResults.value"
            :key="test.name"
            class="flex items-start gap-2 border-b border-[var(--border)]/50 py-1.5"
          >
            <CheckCircle2 v-if="test.passed" class="mt-0.5 h-3.5 w-3.5 shrink-0 text-[#10b981]" />
            <XCircle v-else class="mt-0.5 h-3.5 w-3.5 shrink-0 text-[#ef4444]" />
            <div>
              <div class="text-[11px] text-[var(--foreground)]">{{ test.name }}</div>
              <div v-if="test.message" class="text-[10px]" :class="test.passed ? 'text-[var(--muted-foreground)]' : 'text-[#ef4444]'">{{ test.message }}</div>
            </div>
          </div>
        </div>
      </template>

      <div v-else class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
        <Clock class="h-8 w-8 text-[var(--muted-foreground)]" />
        <p class="text-xs text-[var(--muted-foreground)]">Send a request to see the response.</p>
      </div>
    </div>

    <div
      v-if="showSaveDialog"
      class="absolute inset-0 z-30 flex items-center justify-center bg-black/40 p-4"
      @click.self="showSaveDialog = false"
    >
      <div class="w-full max-w-sm rounded-lg border border-[var(--border)] bg-[var(--popover)] p-3 shadow-xl">
        <div class="mb-2 flex items-center gap-1.5">
          <Save class="h-3.5 w-3.5 text-[var(--primary)]" />
          <span class="text-[12px] font-semibold text-[var(--foreground)]">Save response body</span>
          <button type="button" class="ml-auto text-[var(--muted-foreground)] hover:text-[var(--foreground)]" @click="showSaveDialog = false">
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
        <p class="mb-2 text-[10px] text-[var(--muted-foreground)]">Choose the format to write to disk.</p>
        <div class="grid grid-cols-2 gap-1.5">
          <button
            type="button"
            class="flex items-center gap-1.5 rounded border px-2 py-1.5 text-[11px] transition-colors"
            :class="saveFormat === 'original' ? 'border-[var(--primary)] bg-[var(--primary)]/10 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
            @click="saveFormat = 'original'"
          >
            <Download class="h-3 w-3" /> Original
          </button>
          <button
            type="button"
            class="flex items-center gap-1.5 rounded border px-2 py-1.5 text-[11px] transition-colors"
            :class="saveFormat === 'json' ? 'border-[var(--primary)] bg-[var(--primary)]/10 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
            @click="saveFormat = 'json'"
          >
            <FileJson class="h-3 w-3" /> JSON
          </button>
          <button
            type="button"
            class="flex items-center gap-1.5 rounded border px-2 py-1.5 text-[11px] transition-colors"
            :class="saveFormat === 'html' ? 'border-[var(--primary)] bg-[var(--primary)]/10 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
            @click="saveFormat = 'html'"
          >
            <FileCode2 class="h-3 w-3" /> HTML
          </button>
          <button
            type="button"
            class="flex items-center gap-1.5 rounded border px-2 py-1.5 text-[11px] transition-colors"
            :class="saveFormat === 'text' ? 'border-[var(--primary)] bg-[var(--primary)]/10 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
            @click="saveFormat = 'text'"
          >
            <FileText class="h-3 w-3" /> Text
          </button>
        </div>
        <div class="mt-3 flex items-center justify-end gap-2">
          <button type="button" class="rounded px-2.5 py-1 text-[11px] text-[var(--muted-foreground)] hover:text-[var(--foreground)]" @click="showSaveDialog = false">
            Cancel
          </button>
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded-md bg-[var(--primary)] px-3 py-1.5 text-[11px] font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50"
            :disabled="saving"
            @click="performSave"
          >
            <Save class="h-3.5 w-3.5" /> {{ saving ? "Saving…" : "Save" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

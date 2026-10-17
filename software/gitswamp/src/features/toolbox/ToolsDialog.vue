<script setup lang="ts">
import { computed, ref, watch, type Component } from "vue";
import { Regex, Braces, Link2, Fingerprint, Hash, Type, Columns2, CalendarClock, ShieldCheck, FileCode2, Wrench, Search, X, Palette, Binary, Table, CaseSensitive, Ruler, Code, PackageCheck } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import type { ToolId } from "./toolboxStorage";
import RegexBuilderTool from "./tools/RegexBuilderTool.vue";
import JsonToolkitTool from "./tools/JsonToolkitTool.vue";
import UrlToolkitTool from "./tools/UrlToolkitTool.vue";
import IdTimeTool from "./tools/IdTimeTool.vue";
import HashTool from "./tools/HashTool.vue";
import TextTool from "./tools/TextTool.vue";
import DiffTool from "./tools/DiffTool.vue";
import CronTool from "./tools/CronTool.vue";
import JwtTool from "./tools/JwtTool.vue";
import SchemaTool from "./tools/SchemaTool.vue";
import ColorTool from "./tools/ColorTool.vue";
import NumberBaseTool from "./tools/NumberBaseTool.vue";
import CsvJsonTool from "./tools/CsvJsonTool.vue";
import SlugTool from "./tools/SlugTool.vue";
import HtmlEntitiesTool from "./tools/HtmlEntitiesTool.vue";
import LevenshteinTool from "./tools/LevenshteinTool.vue";
import SemverTool from "./tools/SemverTool.vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    initialTool?: ToolId;
  }>(),
  { initialTool: "regex" },
);

const emit = defineEmits<{
  close: [];
}>();

interface ToolEntry {
  id: ToolId;
  label: string;
  description: string;
  keywords: string;
  icon: Component;
  component: Component;
}

const TOOLS: ToolEntry[] = [
  { id: "regex", label: "Regex Builder", description: "Build regular expressions step by step without memorising syntax.", keywords: "regex regular expression pattern match test contains", icon: Regex, component: RegexBuilderTool },
  { id: "json", label: "JSON Toolkit", description: "Format, minify, sort, validate and query JSON.", keywords: "json format minify prettify validate query path", icon: Braces, component: JsonToolkitTool },
  { id: "schema", label: "JSON to TypeScript", description: "Generate TypeScript interfaces from a JSON sample.", keywords: "json typescript interface schema types generate", icon: FileCode2, component: SchemaTool },
  { id: "url", label: "URL Workshop", description: "Break URLs apart, edit query strings and encode text.", keywords: "url query params encode decode base64 percent link", icon: Link2, component: UrlToolkitTool },
  { id: "text", label: "Text Toolkit", description: "Change case, sort lines and count characters.", keywords: "text case camel snake kebab uppercase sort lines stats", icon: Type, component: TextTool },
  { id: "diff", label: "Diff Checker", description: "Compare two texts line by line.", keywords: "diff compare difference text changes merge", icon: Columns2, component: DiffTool },
  { id: "hash", label: "Hash Generator", description: "Compute MD5, SHA-1, SHA-256, SHA-384 and SHA-512.", keywords: "hash md5 sha1 sha256 sha512 checksum digest", icon: Hash, component: HashTool },
  { id: "jwt", label: "JWT Decoder", description: "Inspect the header, payload and expiry of a token.", keywords: "jwt json web token decode bearer auth claims exp", icon: ShieldCheck, component: JwtTool },
  { id: "cron", label: "Cron Builder", description: "Build and explain cron schedules with next run times.", keywords: "cron schedule timer crontab expression job", icon: CalendarClock, component: CronTool },
  { id: "ids", label: "ID & Time", description: "Generate UUIDs and tokens and convert between timestamps.", keywords: "uuid guid token random id timestamp epoch unix date", icon: Fingerprint, component: IdTimeTool },
  { id: "color", label: "Color Converter", description: "Convert between hex, RGB and HSL and check contrast.", keywords: "color colour hex rgb hsl contrast wcag palette theme", icon: Palette, component: ColorTool },
  { id: "base", label: "Number Base", description: "Convert numbers between binary, octal, decimal and hex.", keywords: "number base binary octal decimal hex convert radix", icon: Binary, component: NumberBaseTool },
  { id: "csv", label: "CSV / JSON", description: "Convert CSV to JSON and JSON back to CSV.", keywords: "csv json table convert spreadsheet export import", icon: Table, component: CsvJsonTool },
  { id: "slug", label: "Slugifier", description: "Turn titles and filenames into clean URL slugs.", keywords: "slug urlify slugify filename title seo kebab anchor", icon: CaseSensitive, component: SlugTool },
  { id: "html", label: "HTML Entities", description: "Escape and unescape HTML special characters.", keywords: "html escape unescape entities encode decode brackets amp", icon: Code, component: HtmlEntitiesTool },
  { id: "similar", label: "String Similarity", description: "Measure edit distance and similarity between two strings.", keywords: "string similarity levenshtein distance fuzzy match compare diff", icon: Ruler, component: LevenshteinTool },
  { id: "semver", label: "Semver & Release", description: "Compare versions and calculate the next major, minor or patch release.", keywords: "semver semantic version version release bump major minor patch npm tag", icon: PackageCheck, component: SemverTool },
];

const activeTool = ref<ToolId>(props.initialTool);
const query = ref("");

const filteredTools = computed(() => {
  const needle = query.value.trim().toLowerCase();
  if (!needle) return TOOLS;
  return TOOLS.filter((tool) => `${tool.label} ${tool.description} ${tool.keywords}`.toLowerCase().includes(needle));
});

const activeEntry = computed(() => TOOLS.find((tool) => tool.id === activeTool.value) ?? TOOLS[0]);
const activeVisible = computed(() => filteredTools.value.some((tool) => tool.id === activeTool.value));

watch(
  () => props.open,
  (value) => {
    if (value) {
      activeTool.value = props.initialTool;
      query.value = "";
    }
  },
);
watch(
  () => props.initialTool,
  (value) => {
    if (props.open) activeTool.value = value;
  },
);
watch(filteredTools, (list) => {
  if (list.length && !list.some((tool) => tool.id === activeTool.value)) {
    activeTool.value = list[0].id;
  }
});

function clearSearch() {
  query.value = "";
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-[7250] flex items-center justify-center bg-black/55 backdrop-blur-sm"
      @click.self="emit('close')"
    >
      <div class="flex h-[560px] max-h-[85vh] w-[1080px] max-w-[95vw] flex-col overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--card)] shadow-2xl">
        <div class="flex flex-shrink-0 items-center gap-3 border-b border-[var(--border)] px-4 py-2.5">
          <div class="flex items-center gap-2">
            <Wrench class="h-4 w-4 text-[var(--primary)]" />
            <h2 class="text-sm font-semibold text-[var(--foreground)]">Toolbox</h2>
          </div>
          <div class="flex flex-1 items-center gap-1.5 rounded-md border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 focus-within:border-[var(--primary)]/60">
            <Search class="h-3.5 w-3.5 flex-shrink-0 text-[var(--muted-foreground)]" />
            <input
              v-model="query"
              placeholder="Search tools… (regex, hash, cron, diff…)"
              class="h-5 flex-1 bg-transparent text-[11px] text-[var(--foreground)] outline-none placeholder:text-[var(--muted-foreground)]/60"
            />
            <button v-if="query" class="rounded p-0.5 text-[var(--muted-foreground)] hover:text-[var(--foreground)]" title="Clear search" @click="clearSearch">
              <X class="h-3 w-3" />
            </button>
          </div>
          <CloseIconButton title="Close toolbox" size="sm" @click="emit('close')" />
        </div>

        <div class="flex min-h-0 flex-1">
          <div class="flex w-[200px] flex-shrink-0 flex-col gap-1 overflow-y-auto border-r border-[var(--border)] bg-[var(--secondary)]/35 p-2">
            <p v-if="!filteredTools.length" class="px-1.5 py-2 text-[10px] text-[var(--muted-foreground)]">No tools match “{{ query }}”.</p>
            <button
              v-for="tool in filteredTools"
              :key="tool.id"
              class="flex items-start gap-2 rounded-md px-2.5 py-2 text-left transition-colors"
              :class="activeTool === tool.id
                ? 'bg-[var(--primary)]/15 text-[var(--foreground)]'
                : 'text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]'"
              @click="activeTool = tool.id"
            >
              <component :is="tool.icon" class="mt-0.5 h-3.5 w-3.5 flex-shrink-0" :class="activeTool === tool.id ? 'text-[var(--primary)]' : ''" />
              <span class="min-w-0">
                <span class="block text-[11px] font-medium">{{ tool.label }}</span>
                <span class="block text-[9px] leading-3 text-[var(--muted-foreground)]">{{ tool.description }}</span>
              </span>
            </button>
          </div>

          <div class="flex min-w-0 flex-1 flex-col overflow-hidden">
            <div v-if="!activeVisible" class="flex h-full items-center justify-center px-4 text-center text-[11px] text-[var(--muted-foreground)]">
              Pick a tool from the list, or clear the search to browse everything.
            </div>
            <template v-else>
              <div class="flex-shrink-0 border-b border-[var(--border)] bg-[var(--secondary)]/20 px-3 py-1.5">
            <span class="text-[10px] text-[var(--muted-foreground)]">{{ activeEntry.description }}</span>
          </div>
          <div class="min-w-0 flex-1 overflow-auto p-3">
            <component :is="activeEntry.component" />
          </div>
            </template>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

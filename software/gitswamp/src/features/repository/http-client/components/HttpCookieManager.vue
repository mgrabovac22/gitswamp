<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Cookie, Plus, Trash2, Search, Lock, ShieldCheck, Pencil, Download, Upload, X, Ban } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import type { HttpClientController } from "../useHttpClient";
import type { HttpCookie, HttpSameSite } from "../httpClient.types";
import { createEmptyCookie, formatCookieExpiry, isCookieExpired } from "../httpClient.utils";

const props = defineProps<{
  open: boolean;
  controller: HttpClientController;
}>();

const emit = defineEmits<{
  close: [];
}>();

const selectedDomain = ref("");
const search = ref("");
const editing = ref<HttpCookie | null>(null);
const isNew = ref(false);
const importText = ref("");
const showImport = ref(false);

const jars = computed(() => props.controller.activeJar.value);

const domains = computed(() => {
  const term = search.value.trim().toLowerCase();
  const list = Object.keys(jars.value).sort();
  if (!term) return list;
  return list.filter((domain) => domain.toLowerCase().includes(term));
});

const activeDomain = computed(() => {
  if (selectedDomain.value && domains.value.includes(selectedDomain.value)) return selectedDomain.value;
  return domains.value[0] ?? "";
});

const domainCookies = computed(() => {
  const term = search.value.trim().toLowerCase();
  const list = jars.value[activeDomain.value] ?? [];
  if (!term) return [...list].sort((a, b) => a.name.localeCompare(b.name));
  const localTerm = activeDomain.value.toLowerCase().includes(term) ? "" : term;
  if (!localTerm) return [...list].sort((a, b) => a.name.localeCompare(b.name));
  return [...list]
    .filter((cookie) => cookie.name.toLowerCase().includes(localTerm) || cookie.value.toLowerCase().includes(localTerm))
    .sort((a, b) => a.name.localeCompare(b.name));
});

const totalCookies = computed(() => Object.values(jars.value).reduce((sum, list) => sum + list.length, 0));

watch(
  () => props.open,
  (open) => {
    if (open) {
      selectedDomain.value = selectedDomain.value || domains.value[0] || "";
      editing.value = null;
      showImport.value = false;
    }
  },
);

function startAdd(): void {
  const draft = createEmptyCookie(activeDomain.value);
  editing.value = draft;
  isNew.value = true;
}

function startEdit(cookie: HttpCookie): void {
  editing.value = { ...cookie };
  isNew.value = false;
}

function saveEditing(): void {
  const cookie = editing.value;
  if (!cookie || !cookie.name.trim()) return;
  const domain = cookie.domain || activeDomain.value;
  props.controller.upsertJarCookie(domain, { ...cookie, domain });
  selectedDomain.value = domain;
  editing.value = null;
}

function removeCookie(cookie: HttpCookie): void {
  props.controller.removeCookieFromJar(activeDomain.value, cookie.id);
}

function setSameSite(value: string): void {
  if (editing.value) {
    editing.value.sameSite = value as HttpSameSite;
  }
}

function exportCookies(): void {
  void props.controller.copyText(props.controller.exportJar(), "Cookie jar JSON copied.");
}

function runImport(): void {
  if (!importText.value.trim()) return;
  props.controller.importJar(importText.value);
  importText.value = "";
  showImport.value = false;
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-[7400] flex items-center justify-center bg-black/55 backdrop-blur-sm"
      @click.self="emit('close')"
    >
      <div class="flex h-[640px] max-h-[90vh] w-[880px] max-w-[94vw] overflow-hidden rounded-2xl border border-[var(--border)] bg-[var(--card)] shadow-2xl">
        <div class="flex w-[240px] flex-shrink-0 flex-col border-r border-[var(--border)]">
          <div class="flex items-center gap-2 border-b border-[var(--border)] px-3 py-3">
            <Cookie class="h-4 w-4 text-[var(--primary)]" />
            <div class="min-w-0">
              <div class="text-xs font-semibold text-[var(--foreground)]">Cookie Manager</div>
              <div class="truncate text-[10px] text-[var(--muted-foreground)]">
                {{ controller.activeEnvironment.value?.name || "Environment" }} · {{ totalCookies }} cookies
              </div>
            </div>
          </div>

          <div class="border-b border-[var(--border)] p-2">
            <div class="relative">
              <Search class="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--muted-foreground)]" />
              <input
                v-model="search"
                type="text"
                placeholder="Search cookies..."
                class="w-full rounded border border-[var(--border)] bg-[var(--input-background)] py-1.5 pl-8 pr-2 text-[11px] text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              >
            </div>
          </div>

          <div class="flex-1 overflow-y-auto p-1.5">
            <p v-if="domains.length === 0" class="px-2 py-6 text-center text-[11px] text-[var(--muted-foreground)]">
              No cookies stored.
            </p>
            <button
              v-for="domain in domains"
              :key="domain"
              type="button"
              class="mb-0.5 flex w-full items-center justify-between gap-2 rounded px-2 py-1.5 text-left transition-colors"
              :class="domain === activeDomain ? 'bg-[var(--primary)]/12 text-[var(--foreground)]' : 'text-[var(--muted-foreground)] hover:bg-[var(--secondary)]/60 hover:text-[var(--foreground)]'"
              @click="selectedDomain = domain"
            >
              <span class="truncate text-[11px]">{{ domain }}</span>
              <span class="rounded-full bg-[var(--secondary)] px-1.5 text-[10px]">{{ (jars[domain] || []).length }}</span>
            </button>
          </div>

          <div class="flex flex-col gap-1 border-t border-[var(--border)] p-2">
            <label class="flex items-center justify-between gap-2 text-[10px] text-[var(--muted-foreground)]">
              <span>Auto-manage cookies</span>
              <input
                type="checkbox"
                class="h-3.5 w-3.5 accent-[var(--primary)]"
                :checked="controller.settings.value.autoManageCookies"
                @change="controller.settings.value.autoManageCookies = ($event.target as HTMLInputElement).checked"
              >
            </label>
            <div class="mt-1 flex gap-1">
              <button
                type="button"
                class="inline-flex flex-1 items-center justify-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/40"
                @click="exportCookies"
              >
                <Download class="h-3 w-3" /> Export
              </button>
              <button
                type="button"
                class="inline-flex flex-1 items-center justify-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--foreground)] transition-colors hover:border-[var(--primary)]/40"
                @click="showImport = !showImport"
              >
                <Upload class="h-3 w-3" /> Import
              </button>
            </div>
          </div>
        </div>

        <div class="flex flex-1 flex-col overflow-hidden">
          <div class="flex items-center justify-between gap-2 border-b border-[var(--border)] px-3 py-2.5">
            <div class="min-w-0">
              <div class="truncate text-xs font-semibold text-[var(--foreground)]">
                {{ activeDomain || "Cookies" }}
              </div>
              <div class="text-[10px] text-[var(--muted-foreground)]">
                Cookies for the active environment. Session and secure cookies are never committed to Git.
              </div>
            </div>
            <div class="flex flex-shrink-0 items-center gap-1">
              <button
                type="button"
                class="inline-flex items-center gap-1 rounded-md bg-[var(--primary)] px-2.5 py-1.5 text-[11px] font-medium text-white transition-opacity hover:opacity-90"
                @click="startAdd"
              >
                <Plus class="h-3 w-3" /> New cookie
              </button>
              <button
                v-if="activeDomain"
                type="button"
                class="inline-flex items-center gap-1 rounded-md border border-[var(--border)] px-2 py-1.5 text-[11px] text-[#ef4444] transition-colors hover:bg-[#ef4444]/10"
                @click="controller.clearJarDomain(activeDomain)"
              >
                <Ban class="h-3 w-3" /> Clear domain
              </button>
              <CloseIconButton size="sm" title="Close cookie manager" @click="emit('close')" />
            </div>
          </div>

          <div v-if="showImport" class="border-b border-[var(--border)] p-2">
            <textarea
              v-model="importText"
              rows="3"
              spellcheck="false"
              placeholder='[{"name":"session","value":"abc","domain":"api.example.com","path":"/"}]'
              class="w-full resize-y rounded border border-[var(--border)] bg-[var(--input-background)] p-2 font-mono text-[10px] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
            ></textarea>
            <div class="mt-1 flex justify-end gap-1">
              <button type="button" class="rounded px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)]" @click="showImport = false">Cancel</button>
              <button type="button" class="rounded bg-[var(--primary)] px-2 py-1 text-[10px] font-medium text-white" @click="runImport">Import</button>
            </div>
          </div>

          <div v-if="editing" class="border-b border-[var(--border)] bg-[var(--secondary)]/30 p-3">
            <div class="mb-2 flex items-center justify-between">
              <span class="text-[11px] font-semibold text-[var(--foreground)]">{{ isNew ? "New cookie" : "Edit cookie" }}</span>
              <button type="button" class="text-[var(--muted-foreground)] hover:text-[var(--foreground)]" @click="editing = null">
                <X class="h-3.5 w-3.5" />
              </button>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <label class="flex flex-col gap-0.5">
                <span class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Name</span>
                <input v-model="editing.name" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[11px] text-[var(--foreground)] focus:outline-none" >
              </label>
              <label class="flex flex-col gap-0.5">
                <span class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Value</span>
                <input v-model="editing.value" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 font-mono text-[11px] text-[var(--foreground)] focus:outline-none" >
              </label>
              <label class="flex flex-col gap-0.5">
                <span class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Domain</span>
                <input v-model="editing.domain" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[11px] text-[var(--foreground)] focus:outline-none" >
              </label>
              <label class="flex flex-col gap-0.5">
                <span class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Path</span>
                <input v-model="editing.path" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[11px] text-[var(--foreground)] focus:outline-none" >
              </label>
              <label class="flex flex-col gap-0.5">
                <span class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">Expires (empty = session)</span>
                <input
                  type="datetime-local"
                  class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[11px] text-[var(--foreground)] focus:outline-none"
                  :value="editing.expires ? new Date(editing.expires - new Date().getTimezoneOffset() * 60000).toISOString().slice(0, 16) : ''"
                  @input="editing.expires = ($event.target as HTMLInputElement).value ? new Date(($event.target as HTMLInputElement).value).getTime() : null"
                >
              </label>
              <label class="flex flex-col gap-0.5">
                <span class="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)]">SameSite</span>
                <select
                  class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 text-[11px] text-[var(--foreground)] focus:outline-none"
                  :value="editing.sameSite"
                  @change="setSameSite(($event.target as HTMLSelectElement).value)"
                >
                  <option value="">(unset)</option>
                  <option value="Lax">Lax</option>
                  <option value="Strict">Strict</option>
                  <option value="None">None</option>
                </select>
              </label>
            </div>
            <div class="mt-2 flex items-center gap-4 text-[10px] text-[var(--muted-foreground)]">
              <label class="flex items-center gap-1.5">
                <input v-model="editing.httpOnly" type="checkbox" class="h-3.5 w-3.5 accent-[var(--primary)]" > HttpOnly
              </label>
              <label class="flex items-center gap-1.5">
                <input v-model="editing.secure" type="checkbox" class="h-3.5 w-3.5 accent-[var(--primary)]" > Secure
              </label>
              <button
                type="button"
                class="ml-auto inline-flex items-center gap-1 rounded-md bg-[var(--primary)] px-3 py-1 text-[11px] font-medium text-white"
                @click="saveEditing"
              >
                Save
              </button>
            </div>
          </div>

          <div class="flex-1 overflow-auto">
            <table class="w-full border-collapse text-left">
              <thead class="sticky top-0 z-10 bg-[var(--secondary)]/70 text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">
                <tr>
                  <th class="px-3 py-1.5 font-semibold">Name</th>
                  <th class="px-3 py-1.5 font-semibold">Value</th>
                  <th class="px-3 py-1.5 font-semibold">Path</th>
                  <th class="px-3 py-1.5 font-semibold">Expires</th>
                  <th class="px-3 py-1.5 font-semibold">Flags</th>
                  <th class="px-3 py-1.5"></th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="cookie in domainCookies"
                  :key="cookie.id"
                  class="border-b border-[var(--border)]/50"
                  :class="isCookieExpired(cookie) ? 'text-[var(--muted-foreground)] line-through' : 'text-[var(--foreground)]'"
                >
                  <td class="max-w-[160px] truncate px-3 py-1.5 text-[11px] font-medium">{{ cookie.name }}</td>
                  <td class="max-w-[220px] truncate px-3 py-1.5 font-mono text-[10px]">{{ cookie.value }}</td>
                  <td class="px-3 py-1.5 text-[10px] text-[var(--muted-foreground)]">{{ cookie.path }}</td>
                  <td class="whitespace-nowrap px-3 py-1.5 text-[10px] text-[var(--muted-foreground)]">{{ formatCookieExpiry(cookie.expires) }}</td>
                  <td class="px-3 py-1.5">
                    <span class="flex items-center gap-1.5 text-[var(--muted-foreground)]">
                      <Lock v-if="cookie.httpOnly" class="h-3 w-3" title="HttpOnly" />
                      <ShieldCheck v-if="cookie.secure" class="h-3 w-3 text-[#10b981]" title="Secure" />
                      <Lock v-if="cookie.fromJar && !cookie.httpOnly && !cookie.secure" class="h-3 w-3 text-[#f59e0b]" title="Session cookie, excluded from Git" />
                    </span>
                  </td>
                  <td class="px-3 py-1.5">
                    <div class="flex items-center justify-end gap-1">
                      <button type="button" class="flex h-6 w-6 items-center justify-center rounded text-[var(--muted-foreground)] hover:text-[var(--foreground)]" title="Edit" @click="startEdit(cookie)">
                        <Pencil class="h-3 w-3" />
                      </button>
                      <button type="button" class="flex h-6 w-6 items-center justify-center rounded text-[var(--muted-foreground)] hover:bg-[#ef4444]/15 hover:text-[#ef4444]" title="Delete" @click="removeCookie(cookie)">
                        <Trash2 class="h-3 w-3" />
                      </button>
                    </div>
                  </td>
                </tr>
                <tr v-if="domainCookies.length === 0">
                  <td colspan="6" class="px-3 py-10 text-center text-[11px] text-[var(--muted-foreground)]">
                    No cookies for this domain.
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="flex items-center justify-between border-t border-[var(--border)] px-3 py-2">
            <span class="text-[10px] text-[var(--muted-foreground)]">Separate jars per environment keep sessions from mixing.</span>
            <button type="button" class="text-[10px] text-[#ef4444] hover:underline" @click="controller.clearAllCookies()">Clear all cookies</button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

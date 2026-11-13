<script setup lang="ts">
import { computed, onMounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Download, FileArchive, HardDrive, Loader2, Plus, RefreshCw, Trash2, Upload } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";

type Overview = {
  version: string | null; configured: boolean; snapshot: string | null; warnings: string[]; remotes: string[];
  large_files: { path: string; size: number }[];
  rules: { pattern: string; source: string; tracked: boolean; lockable: boolean }[];
  files: { name: string; size: number; downloaded: boolean; checkout: boolean }[];
};
const props = defineProps<{ repoPath: string; requestedFiles?: string[]; blocked?: boolean }>();
const emit = defineEmits<{ close: []; changed: []; busy: [busy: boolean] }>();
const data = shallowRef<Overview | null>(null);
const selected = shallowRef(new Set<string>());
const mode = ref<"files" | "rules">("files");
const filter = ref("");
const page = ref(0);
const pattern = ref("");
const remote = ref("");
const working = ref("");
const error = ref("");
const notice = ref("");
const removeRule = ref("");
const pageSize = 50;
const busy = computed(() => !!working.value || !!props.blocked);
const filteredFiles = computed(() => {
  const query = filter.value.trim().toLowerCase();
  const files = data.value?.files ?? [];
  return query ? files.filter(file => file.name.toLowerCase().includes(query)) : files;
});
const pages = computed(() => Math.max(1, Math.ceil(filteredFiles.value.length / pageSize)));
const visibleFiles = computed(() => filteredFiles.value.slice(page.value * pageSize, (page.value + 1) * pageSize));
const exactRules = computed(() => data.value?.rules.filter(rule => rule.tracked) ?? []);

function sizeLabel(bytes: number) {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(2)} GiB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MiB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${bytes} B`;
}

async function load(first = false) {
  const result = await invoke<Overview>("get_lfs_overview", { path: props.repoPath });
  data.value = result;
  const desired = first ? (props.requestedFiles?.length ? new Set(props.requestedFiles) : null) : selected.value;
  selected.value = new Set(result.large_files.filter(file => !desired || desired.has(file.path)).slice(0, 100).map(file => file.path));
  if (!result.remotes.includes(remote.value)) remote.value = result.remotes.includes("origin") ? "origin" : (result.remotes[0] ?? "");
  page.value = Math.min(page.value, pages.value - 1);
}

async function run(label: string, action: () => Promise<void>, refreshRepo = false) {
  if (busy.value) return;
  working.value = label;
  error.value = "";
  notice.value = "";
  emit("busy", true);
  try { await action(); }
  catch (cause) { error.value = String(cause); }
  finally {
    if (refreshRepo) emit("changed");
    working.value = "";
    emit("busy", false);
  }
}

function changeSelection(path: string, checked: boolean) {
  const next = new Set(selected.value);
  if (checked && next.size < 100) next.add(path);
  if (!checked) next.delete(path);
  selected.value = next;
}

function mutate(command: string, payload: Record<string, unknown>, message: string) {
  return run(message, async () => {
    await invoke(command, { path: props.repoPath, ...payload });
    notice.value = message;
    await load();
  }, true);
}

async function convert() {
  if (!data.value?.snapshot || !selected.value.size) return;
  await mutate("convert_staged_lfs", { files: [...selected.value], snapshot: data.value.snapshot }, "LFS pointers and tracking rules staged. No commit created.");
}

async function addRule() {
  const value = pattern.value.trim();
  if (!value) return;
  await mutate("change_lfs_rule", { pattern: value, remove: false }, "Tracking rule added to .gitattributes.");
  if (!error.value) pattern.value = "";
}

async function confirmRemoval() {
  const value = removeRule.value;
  removeRule.value = "";
  await mutate("change_lfs_rule", { pattern: value, remove: true }, "Tracking rule removed. Existing commits are unchanged.");
}

onMounted(() => void run("Loading Git LFS", () => load(true)));
</script>

<template>
  <section class="lfs-manager" aria-label="Git LFS Manager">
    <header class="lfs-header">
      <HardDrive :size="19" class="lfs-accent" />
      <div class="lfs-heading"><h2>Git LFS Manager</h2><span v-if="data?.version" class="lfs-version">{{ data.version.split(' ', 1)[0] }}</span></div>
      <div class="lfs-header-actions">
        <button type="button" title="Refresh Git LFS" aria-label="Refresh Git LFS" :disabled="busy" @click="run('Refreshing Git LFS', () => load())"><RefreshCw :size="15" :class="{ 'animate-spin': !!working }" /></button>
        <CloseIconButton title="Close Git LFS Manager" :disabled="busy" @click="emit('close')" />
      </div>
    </header>
    <div class="lfs-scroll">
      <p v-if="working" role="status" class="lfs-note"><Loader2 :size="14" class="animate-spin" /> {{ working }}</p>
      <p v-if="error" role="alert" class="lfs-note lfs-error">{{ error }}</p>
      <p v-if="notice && !working" role="status" class="lfs-note">{{ notice }}</p>
      <template v-if="data">
        <div v-if="!data.version" class="lfs-band lfs-toolbar"><strong>Git LFS is not installed</strong><button type="button" @click="openUrl('https://git-lfs.com/')">Get Git LFS</button></div>
        <div v-else-if="!data.configured" class="lfs-band lfs-toolbar"><strong>Git LFS is not configured</strong><button type="button" :disabled="busy" @click="mutate('setup_lfs', {}, 'Git LFS is configured for this repository.')">Set up Git LFS</button></div>
        <p v-for="warning in data.warnings" :key="warning" class="lfs-note lfs-error">{{ warning }}</p>

        <section v-if="data.large_files.length" class="lfs-band" aria-label="Oversized staged files">
          <div class="lfs-toolbar"><h3>Oversized staged files <small>{{ data.large_files.length }}</small></h3><button type="button" class="lfs-primary" :disabled="busy || !data.version || !selected.size || !data.snapshot" @click="convert"><FileArchive :size="15" /> Convert &amp; Stage ({{ selected.size }})</button></div>
          <ul class="lfs-candidates">
            <li v-for="file in data.large_files" :key="file.path"><label><input type="checkbox" :checked="selected.has(file.path)" :disabled="busy || (!selected.has(file.path) && selected.size >= 100)" @change="changeSelection(file.path, ($event.target as HTMLInputElement).checked)" /><span>{{ file.path }}</span><small>{{ sizeLabel(file.size) }}</small></label></li>
          </ul>
        </section>

        <div class="lfs-band lfs-toolbar">
          <div class="lfs-segment" role="group" aria-label="Git LFS view">
            <button type="button" :aria-pressed="mode === 'files'" @click="mode = 'files'">Files <small>{{ data.files.length }}</small></button>
            <button type="button" :aria-pressed="mode === 'rules'" @click="mode = 'rules'">Tracking rules <small>{{ exactRules.length }}</small></button>
          </div>
          <div v-if="data.remotes.length && data.version" class="lfs-transfer">
            <select v-model="remote" aria-label="LFS remote" :disabled="busy"><option v-for="name in data.remotes" :key="name">{{ name }}</option></select>
            <button type="button" :disabled="busy || !remote" title="Download LFS objects for the current checkout" @click="mutate('transfer_lfs', { remote, upload: false }, 'LFS download complete.')"><Download :size="14" /> Download</button>
            <button type="button" :disabled="busy || !remote" title="Upload only LFS objects for HEAD, without pushing Git commits" @click="mutate('transfer_lfs', { remote, upload: true }, 'LFS upload complete.')"><Upload :size="14" /> Upload</button>
          </div>
        </div>

        <section v-if="mode === 'files'" class="lfs-band" aria-label="Git LFS files">
          <input v-model="filter" class="lfs-filter" placeholder="Filter files…" aria-label="Filter LFS files" @input="page = 0" />
          <table v-if="visibleFiles.length" class="lfs-table lfs-files"><colgroup><col /><col class="lfs-number-col" /><col class="lfs-status-col" /></colgroup><thead><tr><th>Path</th><th class="lfs-number">Size</th><th class="lfs-status">Local status</th></tr></thead><tbody>
            <tr v-for="file in visibleFiles" :key="file.name"><td class="lfs-path">{{ file.name }}</td><td class="lfs-number">{{ sizeLabel(file.size) }}</td><td class="lfs-status"><span class="lfs-state" :data-state="file.checkout ? 'checkout' : file.downloaded ? 'cached' : 'pointer'">{{ file.checkout ? 'Checked out' : file.downloaded ? 'Cached' : 'Pointer only' }}</span></td></tr>
          </tbody></table>
          <p v-else class="lfs-empty">{{ filter ? 'No matching files' : 'No LFS files in this checkout' }}</p>
          <div v-if="pages > 1" class="lfs-toolbar lfs-pagination"><button type="button" :disabled="page === 0" @click="page--">Previous</button><span>{{ page + 1 }} / {{ pages }}</span><button type="button" :disabled="page + 1 >= pages" @click="page++">Next</button></div>
        </section>

        <section v-else class="lfs-band" aria-label="Tracking rules">
          <form class="lfs-rule-form" @submit.prevent="addRule"><input v-model="pattern" aria-label="LFS pattern" placeholder="*.psd" :disabled="busy || !data.version" /><button type="submit" :disabled="busy || !data.version || !pattern.trim()"><Plus :size="15" /> Add pattern</button></form>
          <table v-if="exactRules.length" class="lfs-table lfs-rules"><colgroup><col /><col class="lfs-source-col" /><col class="lfs-action-col" /></colgroup><thead><tr><th>Pattern</th><th>Source</th><th class="lfs-action"><span class="sr-only">Actions</span></th></tr></thead><tbody>
            <tr v-for="rule in exactRules" :key="`${rule.source}:${rule.pattern}`"><td><div class="lfs-pattern"><code>{{ rule.pattern }}</code><span v-if="rule.lockable" class="lfs-tag">lockable</span></div></td><td><span class="lfs-source">{{ rule.source }}</span></td><td class="lfs-action"><button v-if="rule.source === '.gitattributes'" type="button" class="lfs-delete" :title="`Remove ${rule.pattern}`" :aria-label="`Remove ${rule.pattern}`" :disabled="busy" @click="removeRule = rule.pattern"><Trash2 :size="14" /></button></td></tr>
          </tbody></table>
          <p v-else class="lfs-empty">No tracking rules</p>
          <div v-if="removeRule" class="lfs-toolbar lfs-remove"><span>Remove <code>{{ removeRule }}</code> from .gitattributes?</span><button type="button" @click="removeRule = ''">Cancel</button><button type="button" :disabled="busy" @click="confirmRemoval">Remove</button></div>
        </section>
      </template>
    </div>
  </section>
</template>

<style scoped>
.lfs-manager { min-width: 0; min-height: 0; display: flex; flex-direction: column; background: var(--background); color: var(--foreground); font-size: 12px; }
.lfs-header, .lfs-toolbar, .lfs-transfer, .lfs-segment, .lfs-header-actions, .lfs-rule-form { display: flex; align-items: center; gap: 8px; }
.lfs-header { padding: 10px 14px; border-bottom: 1px solid var(--border); background: var(--card); }
.lfs-heading { display: flex; align-items: center; gap: 8px; min-width: 0; }
.lfs-header h2 { font-size: 15px; font-weight: 650; }
.lfs-header span, small, .lfs-empty { color: var(--muted-foreground); }
.lfs-version { padding: 1px 6px; border: 1px solid var(--border); border-radius: 999px; background: var(--secondary); font: 10px/1.6 ui-monospace, monospace; white-space: nowrap; }
.lfs-header-actions { margin-left: auto; }
.lfs-accent { color: var(--primary); flex: none; }
.lfs-scroll { min-height: 0; overflow: auto; }
.lfs-band { padding: 10px 14px; border-bottom: 1px solid var(--border); }
.lfs-toolbar { flex-wrap: wrap; justify-content: space-between; }
.lfs-toolbar h3 { font-weight: 650; }
.lfs-transfer { flex-wrap: wrap; }
.lfs-segment { gap: 3px; }
.lfs-segment button[aria-pressed='true'] { color: var(--primary); border-color: var(--primary); }
.lfs-segment small { display: inline-flex; align-items: center; justify-content: center; min-width: 18px; padding: 0 4px; border-radius: 999px; background: color-mix(in srgb, var(--foreground) 8%, transparent); font-variant-numeric: tabular-nums; }
.lfs-segment button[aria-pressed='true'] small { background: color-mix(in srgb, var(--primary) 18%, transparent); color: var(--primary); }
.lfs-manager button, .lfs-manager input:not([type=checkbox]), .lfs-manager select { border: 1px solid var(--border); border-radius: 5px; background: var(--secondary); color: var(--foreground); min-height: 28px; padding: 3px 7px; }
.lfs-manager button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; }
.lfs-manager button:not(:disabled):hover { background: var(--accent); }
.lfs-manager button:disabled { opacity: .5; }
.lfs-manager button.lfs-primary { background: var(--primary); color: var(--primary-foreground, white); border-color: var(--primary); }
.lfs-manager input[type=checkbox] { accent-color: var(--primary); }
.lfs-candidates { max-height: 200px; overflow: auto; margin-top: 5px; }
.lfs-candidates label { display: flex; gap: 7px; align-items: center; padding: 5px 0; border-bottom: 1px solid var(--border); }
.lfs-candidates label span, td { overflow-wrap: anywhere; }
.lfs-candidates label small { margin-left: auto; flex: none; }
.lfs-note { display: flex; align-items: center; gap: 6px; margin: 8px 14px; overflow-wrap: anywhere; }
.lfs-error { color: var(--destructive); }
.lfs-filter, .lfs-rule-form input { min-width: 0; width: min(260px, 100%); }
.lfs-rule-form button { flex: none; }
.lfs-table { width: 100%; margin-top: 8px; text-align: left; table-layout: fixed; border-collapse: collapse; }
.lfs-table th, .lfs-table td { padding: 6px 8px; border-bottom: 1px solid var(--border); vertical-align: middle; }
.lfs-table thead th { color: var(--muted-foreground); font-size: 10px; font-weight: 600; text-transform: uppercase; letter-spacing: .04em; }
.lfs-table tbody tr:last-child td { border-bottom: 0; }
.lfs-table tbody tr:hover { background: color-mix(in srgb, var(--secondary) 55%, transparent); }
.lfs-number-col { width: 92px; }
.lfs-status-col { width: 124px; }
.lfs-source-col { width: 30%; }
.lfs-action-col { width: 44px; }
.lfs-number { text-align: center; font-variant-numeric: tabular-nums; white-space: nowrap; }
.lfs-status { text-align: center; }
.lfs-action { width: 44px; text-align: center; }
.lfs-path { overflow-wrap: anywhere; }
.lfs-pattern { display: flex; align-items: center; gap: 7px; min-width: 0; }
.lfs-pattern code { overflow-wrap: anywhere; }
.lfs-tag, .lfs-state { display: inline-flex; align-items: center; justify-content: center; min-height: 18px; padding: 1px 7px; border: 1px solid var(--border); border-radius: 999px; background: var(--secondary); color: var(--muted-foreground); font-size: 10px; white-space: nowrap; }
.lfs-tag { flex: none; color: var(--primary); border-color: color-mix(in srgb, var(--primary) 35%, var(--border)); }
.lfs-state[data-state='checkout'] { color: #3fb950; border-color: color-mix(in srgb, #3fb950 40%, var(--border)); }
.lfs-state[data-state='cached'] { color: var(--primary); }
.lfs-source { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted-foreground); }
.lfs-delete { width: 28px; min-height: 28px; padding: 0; color: var(--muted-foreground); }
.lfs-delete:not(:disabled):hover { color: var(--destructive); border-color: color-mix(in srgb, var(--destructive) 45%, var(--border)); }
.lfs-pagination, .lfs-remove { margin-top: 9px; justify-content: flex-end; }
.lfs-empty { padding: 12px 0; }
</style>

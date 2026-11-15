<script setup lang="ts">
import { computed, ref } from "vue";
import { CheckCircle2, CircleAlert, Clipboard, Download, Loader2, RefreshCw } from "lucide-vue-next";
import { handoffReport, type Handoff, type WorkflowComparison } from "./branchWorkflow";

const props = defineProps<{ handoff: Handoff; comparison: WorkflowComparison; supportsPullRequest: boolean; canFetch: boolean; busy: boolean }>();
const emit = defineEmits<{ refresh: []; fetch: []; open: [description?: string]; back: [] }>();
const includeNotes = ref(false);
const copyStatus = ref("");
const status = computed(() => {
  const h = props.handoff, review = props.comparison;
  if (review.ahead === 0) return { title: "No commits to hand off", detail: "The target already contains this branch tip." };
  if (review.conflicts === true) return { title: "Resolve merge conflicts first", detail: "Update the source branch and resolve the affected paths before opening a pull request." };
  if (h.sourceSync === "no-remote") return { title: "Local preview only", detail: "Add a remote to hand this branch off for review." };
  if (!h.targetFetched) return { title: "Fetch the target branch", detail: "The preview uses the local target until the remote target is available." };
  if (h.sourceSync === "not-published") return { title: "Publish the source branch", detail: "Push the source, then fetch and recheck its published tip." };
  if (h.sourceSync === "local-ahead") return { title: "Push the reviewed commits", detail: "The fetched source is behind this local branch." };
  if (h.sourceSync === "remote-ahead") return { title: "Update the local source", detail: "The fetched source contains commits that are not in the local branch." };
  if (h.sourceSync !== "in-sync") return { title: "Compare the source tips", detail: "The local and fetched tips differ. Review their history before pushing." };
  if (review.conflicts === null) return { title: "Merge preview unavailable", detail: "Make sure there is enough Git history to compare these branches." };
  return { title: "Ready for pull request review", detail: props.supportsPullRequest
    ? "The reviewed tip matches the fetched source."
    : "Open the pull request with your remote provider." };
});
const ready = computed(() => props.handoff.published && props.comparison.ahead > 0 && props.comparison.conflicts === false);
const report = computed(() => handoffReport(props.handoff, props.comparison));

async function copy() {
  try {
    await navigator.clipboard.writeText(report.value);
    copyStatus.value = "Handoff notes copied.";
  } catch {
    copyStatus.value = "Clipboard is unavailable.";
  }
}
</script>

<template>
  <section class="handoff" aria-label="Integration review">
    <div class="readiness">
      <CheckCircle2 v-if="ready" :size="17" class="ready-icon" aria-hidden="true" />
      <CircleAlert v-else :size="17" class="attention-icon" aria-hidden="true" />
      <div><strong>{{ status.title }}</strong><p>{{ status.detail }}</p></div>
    </div>
    <div class="route">
      <code>{{ handoff.source }}</code><span aria-label="into">→</span><code>{{ handoff.remote && handoff.targetFetched ? handoff.remote + '/' + handoff.target : handoff.target }}</code>
    </div>
    <p class="muted">{{ comparison.ahead }} ahead · {{ comparison.behind }} behind · {{ handoff.targetFetched ? 'Fetched target' : 'Local target' }}</p>
    <ul v-if="comparison.conflictPaths?.length" class="paths" aria-label="Possible conflict paths">
      <li v-for="file in comparison.conflictPaths" :key="file"><code>{{ file }}</code></li>
    </ul>
    <template v-if="handoff.checkpoint">
      <p class="muted">{{ handoff.checkpoint.changedPaths }} changed paths · {{ handoff.checkpoint.testPaths }} test paths touched. Tests were not run.</p>
      <p v-if="handoff.checkpoint.reviewAreas.length"><strong>Review focus:</strong> {{ handoff.checkpoint.reviewAreas.join(' · ') }}</p>
      <details v-if="handoff.checkpoint.commits.length" class="commits">
        <summary>Recent source commits ({{ handoff.checkpoint.commits.length }})</summary>
        <ul><li v-for="commit in handoff.checkpoint.commits" :key="commit.sha"><code>{{ commit.sha }}</code> {{ commit.subject }}</li></ul>
      </details>
    </template>
    <div class="tools">
      <button type="button" :disabled="busy" @click="emit('refresh')"><RefreshCw :size="14" /> Recheck</button>
      <button v-if="canFetch" type="button" :disabled="busy" @click="emit('fetch')"><Download :size="14" /> Fetch &amp; recheck</button>
      <button type="button" :disabled="busy" @click="copy"><Clipboard :size="14" /> Copy notes</button>
    </div>
    <p v-if="copyStatus" class="muted" role="status">{{ copyStatus }}</p>
    <label v-if="ready && supportsPullRequest" class="include"><input v-model="includeNotes" type="checkbox" :disabled="busy" /> Include handoff notes in PR</label>
    <footer>
      <button type="button" :disabled="busy" @click="emit('back')">Back</button>
      <button type="button" class="primary" :disabled="busy || !ready || !supportsPullRequest" @click="emit('open', includeNotes ? report : undefined)">
        <Loader2 v-if="busy" :size="14" class="animate-spin" /> Open pull request form
      </button>
    </footer>
  </section>
</template>

<style scoped>
.handoff { display: grid; gap: 9px; min-width: 0; overflow-wrap: anywhere; }
.readiness { display: grid; grid-template-columns: 17px minmax(0, 1fr); gap: 7px; padding: 4px 0 6px; }
.readiness p, .muted { color: var(--muted-foreground); }
.ready-icon { color: var(--primary); }
.attention-icon { color: #cf8520; }
.route, .tools, footer, .include { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
.route code { min-width: 0; overflow-wrap: anywhere; }
.paths, .commits { padding: 6px 0; border-block: 1px solid var(--border); }
.commits summary { cursor: pointer; }
.commits li { padding-top: 4px; }
.commits code { color: var(--muted-foreground); }
.tools button, footer button { min-height: 30px; padding: 4px 8px; display: inline-flex; gap: 5px; align-items: center; border: 1px solid var(--border); border-radius: 4px; }
button:not(:disabled):hover { filter: brightness(.94); background: var(--secondary); }
footer { justify-content: flex-end; }
footer .primary { background: var(--primary); color: var(--primary-foreground); }
footer .primary:not(:disabled):hover { background: var(--primary); }
</style>

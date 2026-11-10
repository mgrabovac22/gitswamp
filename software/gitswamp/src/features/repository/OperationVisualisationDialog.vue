<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from "vue";
import { Archive, ArchiveRestore, ArrowDownToLine, ArrowUpFromLine, Check, Download, GitBranch, GitCommitHorizontal, GitMerge, PencilLine, RotateCcw, Trash2, Undo2 } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import RefOperationScene from "./RefOperationScene.vue";
import TransferOperationScene from "./TransferOperationScene.vue";
import type { OperationVisualisation } from "./useOperationVisualisation";
import { usePreviewEscape } from "./usePreviewEscape";
import { UNDOABLE_DESTRUCTIVE_DELAY_MS } from "@/shared/notifications/destructiveScheduler";

const props = defineProps<{ preview: OperationVisualisation }>();
const emit = defineEmits<{ close: [] }>();
usePreviewEscape(() => emit("close"));
const reduceMotion = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
const undo = computed(() => "pendingUndo" in props.preview ? props.preview.pendingUndo : undefined);
const duration = undo.value ? UNDOABLE_DESTRUCTIVE_DELAY_MS : "incoming" in props.preview ? 7000 : 5500;
const incomingPath = "M 28 31 H 148 C 218 31 239 94 312 94";
const copy = {
  merge: { title: "Merge completed", source: "Incoming branch", list: "Merged commits", result: "Merge commit", icon: GitMerge },
  "cherry-pick": { title: "Cherry-pick completed", source: "Original commit", list: "Picked commit", result: "New commit", icon: GitCommitHorizontal },
  revert: { title: "Revert completed", source: "Original commit", list: "Reverted commit", result: "Revert commit", icon: Undo2 },
} as const;
const refCopy = {
  "branch-created": { title: "Branch created", icon: GitBranch },
  "branch-deleted": { title: "Local branch deleted", icon: Trash2 },
  "branch-renamed": { title: "Branch renamed", icon: PencilLine },
  reset: { title: "Reset completed", icon: RotateCcw },
  push: { title: "Push completed", icon: ArrowUpFromLine },
  pull: { title: "Pull completed", icon: ArrowDownToLine },
  fetch: { title: "Fetch completed", icon: Download },
  "stash-save": { title: "Stash saved", icon: Archive },
  "stash-restore": { title: "Stash restored", icon: ArchiveRestore },
} as const;
const labels = computed(() => "incoming" in props.preview ? copy[props.preview.kind] : null);
const heading = computed(() => "incoming" in props.preview ? copy[props.preview.kind] : refCopy[props.preview.kind]);
const title = computed(() => undo.value ? props.preview.kind === "reset" ? "Reset pending" : "Branch deletion pending" : heading.value.title);
let timer: ReturnType<typeof setTimeout> | undefined;

onMounted(() => {
  timer = setTimeout(() => emit("close"), duration);
});
onBeforeUnmount(() => {
  clearTimeout(timer);
});
</script>

<template>
  <Teleport to="body">
    <dialog open class="operation-preview" :data-kind="preview.kind" :data-pending="Boolean(undo)" aria-labelledby="operation-preview-title"
      :style="{ '--preview-duration': `${duration}ms` }">
      <header class="operation-header">
        <span class="operation-icon"><component :is="heading.icon" :size="18" /></span>
        <h2 id="operation-preview-title">{{ title }}</h2>
        <button v-if="undo" type="button" class="preview-undo" @click="undo()"><Undo2 :size="14" />Undo</button>
        <CloseIconButton title="Close operation preview" size="sm" @click="emit('close')" />
      </header>

      <TransferOperationScene v-if="'endpoint' in preview" :preview="preview" />
      <RefOperationScene v-else-if="!('incoming' in preview)" :preview="preview" />
      <template v-else-if="labels">
      <div class="operation-graph">
        <div class="graph-refs">
          <div class="graph-ref source-ref">
            <component :is="preview.kind === 'merge' ? GitBranch : GitCommitHorizontal" :size="15" />
            <div><span>{{ labels.source }}</span><strong :title="preview.source">{{ preview.source }}</strong></div>
          </div>
          <div class="graph-ref target-ref">
            <GitBranch :size="15" />
            <div><span>Target</span><strong :title="preview.target">{{ preview.target }}</strong></div>
          </div>
        </div>

        <svg class="commit-flow" viewBox="0 0 548 122" aria-hidden="true" focusable="false">
          <path class="graph-guide" d="M 28 31 H 520 M 28 63 H 520 M 28 94 H 520" />
          <path class="incoming-track" :d="incomingPath" />
          <path v-if="preview.kind === 'merge'" class="incoming-rail" :d="incomingPath" pathLength="1" />
          <path class="target-rail" d="M 28 94 H 312" />
          <path class="result-rail" d="M 312 94 H 520" pathLength="1" />
          <circle class="source-point" cx="28" cy="31" r="6" />
          <circle class="target-point" cx="28" cy="94" r="6" />
          <circle v-if="!reduceMotion" v-for="(commit, index) in preview.incoming" :key="commit.sha"
            class="incoming-commit" r="4.5" :style="{ '--particle-delay': `${250 + index * 122}ms` }">
            <animateMotion :path="incomingPath" :begin="`${250 + index * 122}ms`" dur="1.25s" fill="freeze" />
          </circle>
          <circle class="join-point" cx="312" cy="94" r="5" />
          <g class="result-point" transform="translate(520 94)">
            <circle class="result-pulse" r="13" />
            <circle class="result-outline" r="11" />
            <circle class="result-core" r="6" />
          </g>
        </svg>

        <div class="graph-tips">
          <span>Previous tip <code :title="preview.before.sha">{{ preview.before.sha.slice(0, 7) }}</code></span>
          <span class="new-tip">{{ labels.result }} <code :title="preview.result.sha">{{ preview.result.sha.slice(0, 7) }}</code></span>
        </div>
      </div>

      <section class="operation-commits" aria-labelledby="operation-commits-title">
        <h3 id="operation-commits-title">{{ labels.list }}</h3>
        <ul v-if="preview.incoming.length">
          <li v-for="commit in preview.incoming" :key="commit.sha">
            <code :title="commit.sha">{{ commit.sha.slice(0, 7) }}</code>
            <span :title="commit.subject">{{ commit.subject || '(No subject)' }}</span>
          </li>
        </ul>
        <p v-else>No new commits in the incoming history.</p>
        <p v-if="preview.hasMore">More commits in the graph</p>
      </section>

      <footer class="operation-result">
        <Check :size="15" />
        <span :title="preview.result.subject">{{ preview.result.subject || labels.result }}</span>
      </footer>
      </template>
      <div class="preview-progress" aria-hidden="true" />
    </dialog>
  </Teleport>
</template>

<style scoped>
.operation-preview {
  --incoming: #a28cff;
  --current: #4c9fee;
  --joined: #3bc998;
  --panel: var(--card, var(--background));
  position: fixed;
  z-index: 45;
  inset: 0;
  width: min(602px, calc(100% - 24px));
  max-height: calc(100% - 24px);
  margin: auto;
  padding: 0;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel);
  color: var(--foreground);
  box-shadow: 0 18px 60px #0005;
}
.operation-preview[data-kind="revert"] { --incoming: #e2a84c; }
.operation-header, .operation-result { display: flex; align-items: center; gap: 10px; padding: 12px 16px; }
.operation-header h2 { flex: 1; margin: 0; min-width: 0; font-size: 16px; font-weight: 650; }
.preview-undo { display: inline-flex; flex: none; align-items: center; gap: 6px; padding: 4px 8px; border: 1px solid var(--border); border-radius: 5px; background: var(--secondary); font-size: 12px; }
.preview-undo:hover, .preview-undo:focus-visible { background: var(--accent); }
.operation-preview[data-pending="true"] .preview-progress { background: var(--chart-3); }
.operation-icon { display: grid; flex: 0 0 30px; height: 30px; place-items: center; border-radius: 6px; background: var(--secondary); }
.operation-graph { padding: 10px 16px; background: color-mix(in srgb, var(--secondary) 58%, var(--panel)); border-block: 1px solid var(--border); }
.graph-refs, .graph-tips { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.graph-ref { display: flex; align-items: center; gap: 7px; min-width: 0; }
.graph-ref > div { min-width: 0; }
.graph-ref svg, .operation-result svg { flex-shrink: 0; }
.graph-ref span, .graph-ref strong { display: block; }
.graph-ref span, .graph-tips, .operation-commits p { font-size: 11px; color: var(--muted-foreground); }
.graph-ref strong { font-size: 13px; }
.source-ref svg { color: var(--incoming); }
.target-ref { justify-content: flex-end; text-align: right; }
.target-ref svg { color: var(--current); }
.graph-tips span { overflow-wrap: anywhere; }
.new-tip { text-align: right; }
.graph-tips code { color: var(--foreground); }
.commit-flow { width: 100%; height: auto; display: block; }
.commit-flow path { fill: none; stroke-linecap: round; }
.graph-guide { stroke: var(--border); stroke-dasharray: 2 7; }
.incoming-track { stroke: var(--incoming); stroke-width: 2; opacity: .23; }
.operation-preview:not([data-kind="merge"]) .incoming-track { stroke-dasharray: 4 6; opacity: .7; }
.incoming-rail, .result-rail { stroke-dasharray: 1; stroke-dashoffset: 0; }
.incoming-rail { stroke: var(--incoming); stroke-width: 3; animation: draw-rail 1.45s ease-out both; }
.target-rail { stroke: var(--current); stroke-width: 4; }
.result-rail { stroke: var(--joined); stroke-width: 6; animation: draw-rail .55s 2s ease-out both; }
.source-point, .incoming-commit { fill: var(--incoming); }
.target-point { fill: var(--current); }
.incoming-commit { stroke: var(--panel); stroke-width: 1.2; animation: travel 1.25s var(--particle-delay) linear both; }
.join-point { fill: var(--joined); opacity: 0; animation: travel .6s 1.45s ease-out both; }
.result-point { animation: arrive .35s 2.4s ease-out both; }
.result-outline { fill: var(--panel); stroke: var(--joined); stroke-width: 3; }
.result-core { fill: var(--joined); }
.result-pulse { fill: none; stroke: var(--joined); transform-box: fill-box; transform-origin: center; animation: pulse .95s 2.55s ease-out both; }
.operation-commits { padding: 10px 16px 8px; }
.operation-commits h3 { margin: 0 0 5px; font-size: 12px; font-weight: 650; }
.operation-commits ul { margin: 0; padding: 0; list-style: none; }
.operation-commits li { display: grid; grid-template-columns: 8ch minmax(0, 1fr); align-items: center; gap: 8px; min-height: 22px; font-size: 12px; }
.operation-commits li code { color: var(--muted-foreground); }
.operation-commits p { margin: 4px 0 0; }
.operation-commits li span, .operation-result span, .graph-ref strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.operation-result { min-height: 37px; padding-block: 8px; border-top: 1px solid var(--border); font-size: 12px; }
.operation-result svg { color: var(--joined); }
.preview-progress { height: 2px; background: var(--joined); transform-origin: left; animation: elapsed var(--preview-duration) linear both; }
@keyframes draw-rail { from { stroke-dashoffset: 1; } }
@keyframes travel { 0%, 100% { opacity: 0; } 8%, 88% { opacity: 1; } }
@keyframes arrive { from { opacity: 0; } }
@keyframes pulse { 0% { opacity: 0; } 20% { opacity: .8; } 100% { opacity: 0; transform: scale(2.2); } }
@keyframes elapsed { to { transform: scaleX(0); } }
@media (prefers-reduced-motion: reduce) {
  .operation-preview *, .operation-preview *::before { animation: none !important; }
  .result-pulse { display: none; }
}
</style>

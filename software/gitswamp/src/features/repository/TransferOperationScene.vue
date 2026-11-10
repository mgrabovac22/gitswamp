<script setup lang="ts">
import { computed } from "vue";
import { Archive, Check, Cloud, Files, GitBranch, GitCommitHorizontal, Laptop } from "lucide-vue-next";
import { providerIconSvg } from "@/view/commit/graph/graph.utils";
import type { TransferOperationPreview } from "@/shared/notifications/gitOperationPreview";

const props = defineProps<{ preview: TransferOperationPreview }>();
const incoming = computed(() => ["pull", "fetch", "stash-restore"].includes(props.preview.kind));
const stash = computed(() => props.preview.kind.startsWith("stash-"));
const providerNames = { github: "GitHub", gitlab: "GitLab", bitbucket: "Bitbucket", azure: "Azure DevOps", unknown: "" };
const provider = computed(() => props.preview.endpoint.provider ?? "unknown");
const providerName = computed(() => providerNames[provider.value]);
const finishLabel = {
  push: "Remote accepted the push",
  pull: "Local branch updated",
  fetch: "Remote-tracking refs refreshed",
  "stash-save": "Changes stored locally",
  "stash-restore": "Changes restored to the working tree",
} as const;
</script>

<template>
  <div class="transfer-scene" :class="{ incoming }" :data-provider="provider" :data-stash="stash">
    <div class="transfer-rail">
      <span class="endpoint local" :class="{ destination: incoming }">
        <component :is="stash ? Files : preview.kind === 'fetch' ? Laptop : GitBranch" :size="24" />
      </span>
      <svg class="transfer-track" viewBox="0 0 320 78" aria-hidden="true" focusable="false">
        <path class="track-guide" d="M 0 39 H 320" />
        <path class="track-progress" :d="incoming ? 'M 320 39 H 0' : 'M 0 39 H 320'" pathLength="1" />
        <g class="transfer-packet" :class="{ 'transfer-packet--in': incoming }">
          <circle cx="24" cy="39" r="13" />
          <path v-if="stash" d="M 19 37 H 29 V 43 H 19 Z M 21 35 H 27" />
          <path v-else d="M 17 39 H 21 M 27 39 H 31 M 27 39 A 3 3 0 1 1 21 39 A 3 3 0 1 1 27 39" />
        </g>
      </svg>
      <span class="endpoint remote" :class="{ destination: !incoming }">
        <Archive v-if="stash" :size="24" />
        <!-- This helper returns one of a fixed set of built-in icons. -->
        <span v-else-if="providerName" class="provider-icon" aria-hidden="true" v-html="providerIconSvg(provider)" />
        <Cloud v-else :size="24" />
      </span>
    </div>
    <div class="transfer-labels">
      <span><small>Local</small><strong :title="preview.target">{{ preview.target }}</strong></span>
      <span><small>{{ stash ? 'Local archive' : providerName || 'Remote' }}</small><strong :title="preview.endpoint.label">{{ preview.endpoint.label }}</strong></span>
    </div>
  </div>
  <footer class="transfer-result">
    <Check :size="15" />
    <span>{{ finishLabel[preview.kind] }}</span>
    <span v-if="preview.sha" class="transfer-sha" :title="preview.sha"><GitCommitHorizontal :size="14" /><code>{{ preview.sha.slice(0, 7) }}</code></span>
  </footer>
</template>

<style scoped>
.transfer-scene { --destination: #3bc998; padding: 14px 16px 10px; border-block: 1px solid var(--border); background: color-mix(in srgb, var(--secondary) 58%, var(--panel)); }
.transfer-scene[data-provider="gitlab"] { --destination: #f08d55; }
.transfer-scene[data-provider="bitbucket"], .transfer-scene[data-provider="azure"] { --destination: #4c9fee; }
.transfer-scene[data-stash="true"] { --destination: #e1ad58; }
.transfer-rail { display: grid; grid-template-columns: 50px minmax(0, 1fr) 50px; align-items: center; }
.endpoint { height: 50px; display: grid; place-items: center; position: relative; border: 1px solid var(--border); border-radius: 8px; background: var(--panel); color: var(--foreground); }
.endpoint.destination { animation: destination 1.1s 1.4s both; }
.provider-icon :deep(svg) { width: 24px; height: 24px; opacity: 1; }
.transfer-track { display: block; width: 100%; height: auto; overflow: visible; }
.track-guide { fill: none; stroke: var(--muted-foreground); stroke-width: 1.5; stroke-dasharray: 3 7; opacity: .45; }
.track-progress { fill: none; stroke: var(--destination); stroke-width: 3; stroke-dasharray: 1; animation: link-up 1.4s .2s both; }
.transfer-packet { animation: send 1.45s .2s cubic-bezier(.24, .74, .29, 1) both; }
.transfer-packet--in { animation-name: receive; }
.transfer-packet circle { fill: var(--panel); stroke: var(--destination); stroke-width: 2; }
.transfer-packet path { fill: none; stroke: var(--destination); stroke-width: 1.5; stroke-linecap: round; }
.transfer-labels { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; margin-top: 9px; }
.transfer-labels small, .transfer-labels strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.transfer-labels small { font-size: 11px; color: var(--muted-foreground); }
.transfer-labels strong { font-size: 12px; }
.transfer-labels span:last-child { text-align: right; }
.transfer-result { display: flex; align-items: center; gap: 8px; padding: 10px 16px; font-size: 12px; }
.transfer-result svg { flex: none; color: var(--joined); }
.transfer-sha { display: inline-flex; align-items: center; gap: 5px; margin-inline-start: auto; }
.transfer-sha svg { color: var(--muted-foreground); }
@keyframes send { 0% { opacity: 0; transform: translateX(0); } 12%, 87% { opacity: 1; } 100% { opacity: 0; transform: translateX(272px); } }
@keyframes receive { 0% { opacity: 0; transform: translateX(272px); } 12%, 87% { opacity: 1; } 100% { opacity: 0; transform: translateX(0); } }
@keyframes link-up { from { stroke-dashoffset: 1; } }
@keyframes destination { 0% { border-color: var(--border); } 30% { box-shadow: 0 0 0 5px color-mix(in srgb, var(--destination), transparent 80%); } 100% { border-color: var(--destination); color: var(--destination); } }
@media (prefers-reduced-motion: reduce) {
  .transfer-scene * { animation: none !important; }
  .transfer-packet { display: none; }
  .endpoint.destination { border-color: var(--destination); color: var(--destination); }
}
</style>

<script setup lang="ts">
import { Check, GitBranch } from "lucide-vue-next";
import type { RefOperationPreview } from "@/shared/notifications/gitOperationPreview";

defineProps<{ preview: RefOperationPreview }>();
</script>

<template>
  <div class="ref-scene" :data-scene="preview.kind" :data-pending="Boolean(preview.pendingUndo)">
    <div class="ref-name">
      <GitBranch :size="15" />
      <strong :title="preview.target">{{ preview.target }}</strong>
      <span v-if="preview.kind === 'reset'" class="mode">{{ preview.mode }}</span>
    </div>

    <svg viewBox="0 0 548 118" aria-hidden="true" focusable="false">
      <template v-if="preview.kind === 'reset'">
        <path class="ticks" d="M 78 68 V 76 M 106 68 V 76 M 134 68 V 76 M 162 68 V 76 M 190 68 V 76 M 218 68 V 76 M 246 68 V 76 M 274 68 V 76 M 302 68 V 76 M 330 68 V 76 M 358 68 V 76 M 386 68 V 76 M 414 68 V 76 M 442 68 V 76 M 470 68 V 76" />
        <path class="reference-track" d="M 42 54 H 506" />
        <path class="rewind-trail" d="M 488 54 H 60" pathLength="1" />
        <circle class="old-tip" cx="488" cy="54" r="8" />
        <circle class="new-tip" cx="60" cy="54" r="10" />
        <g class="rewind-head">
          <circle cx="60" cy="54" r="14" />
          <path d="M 65 48 L 59 54 L 65 60 M 60 48 L 54 54 L 60 60" />
        </g>
      </template>

      <template v-else-if="preview.kind === 'branch-deleted'">
        <path class="stable-track" d="M 38 87 H 351" />
        <circle class="track-node" cx="54" cy="87" r="5" />
        <circle class="track-node" cx="134" cy="87" r="5" />
        <circle class="track-node" cx="228" cy="87" r="5" />
        <path class="removed-branch" d="M 134 87 C 168 87 167 45 202 45 H 268" />
        <path class="drop-guide" d="M 267 45 C 339 45 345 21 413 30" />
        <g class="trash">
          <path d="M 394 48 L 400 94 H 438 L 444 48 M 411 59 V 81 M 427 59 V 81" />
          <path class="trash-lid" d="M 388 45 H 450 M 409 45 V 37 H 429 V 45" />
        </g>
        <g class="discarded-ref">
          <rect x="236" y="32" width="63" height="26" rx="6" />
          <path d="M 251 39 V 51 M 251 47 Q 264 47 264 39 M 272 45 H 287" />
        </g>
      </template>

      <template v-else-if="preview.kind === 'branch-created'">
        <path class="stable-track" d="M 42 83 H 506" />
        <circle class="track-node" cx="60" cy="83" r="5" />
        <circle class="track-node" cx="160" cy="83" r="7" />
        <circle class="track-node" cx="488" cy="83" r="5" />
        <path class="branch-growth" d="M 160 83 C 226 83 235 34 306 34 H 488" pathLength="1" />
        <circle class="grown-tip" cx="488" cy="34" r="9" />
      </template>

      <template v-else>
        <path class="stable-track" d="M 42 84 H 506" />
        <circle class="track-node" cx="60" cy="84" r="5" />
        <circle class="track-node" cx="488" cy="84" r="5" />
        <path class="rename-stem" d="M 274 60 V 84" />
        <circle class="track-node" cx="274" cy="84" r="7" />
        <g class="old-label"><rect x="219" y="31" width="110" height="32" rx="6" /><path d="M 236 47 H 312" /></g>
        <g class="new-label"><rect x="219" y="31" width="110" height="32" rx="6" /><path d="M 236 47 H 312" /></g>
      </template>
    </svg>

    <div v-if="preview.kind === 'reset'" class="scene-ends">
      <span>Reset target <code :title="preview.resultSha">{{ preview.resultSha.slice(0, 7) }}</code></span>
      <span>Previous tip <code :title="preview.previousSha">{{ preview.previousSha.slice(0, 7) }}</code></span>
    </div>
    <div v-else-if="preview.kind === 'branch-renamed'" class="scene-ends">
      <span>Previous name <code :title="preview.previousName">{{ preview.previousName }}</code></span>
      <span>New name <code :title="preview.target">{{ preview.target }}</code></span>
    </div>
  </div>

  <footer class="ref-result">
    <Check v-if="!preview.pendingUndo" :size="15" />
    <span v-if="preview.pendingUndo">Scheduled</span>
    <span v-else-if="preview.kind === 'branch-deleted'">Branch reference removed</span>
    <span v-else-if="preview.kind === 'branch-created'">Starting commit <code :title="preview.sourceSha">{{ preview.sourceSha.slice(0, 7) }}</code></span>
    <span v-else-if="preview.kind === 'branch-renamed'">Commit history unchanged</span>
    <span v-else-if="preview.mode === 'soft'">Index and working tree unchanged</span>
    <span v-else-if="preview.mode === 'mixed'">Index reset, working tree unchanged</span>
    <span v-else>Index and tracked files reset</span>
  </footer>
</template>

<style scoped>
.ref-scene { padding: 10px 16px; background: color-mix(in srgb, var(--secondary) 58%, var(--panel)); border-block: 1px solid var(--border); }
.ref-name, .ref-result { display: flex; align-items: center; gap: 8px; min-width: 0; font-size: 13px; }
.ref-name svg, .ref-result svg { flex-shrink: 0; }
.ref-name strong, .scene-ends code { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mode { margin-inline-start: auto; color: var(--muted-foreground); font: 12px monospace; }
.ref-scene > svg { display: block; width: 100%; height: auto; }
.ref-scene path { fill: none; stroke-linecap: round; }
.reference-track { stroke: var(--border); stroke-width: 4; }
.ticks { stroke: var(--muted-foreground); opacity: .45; }
.old-tip { fill: var(--muted-foreground); }
.new-tip, .grown-tip { fill: var(--joined); }
.new-tip { animation: reveal .3s 1.8s both; }
.rewind-trail { stroke: var(--incoming); stroke-width: 3; stroke-dasharray: 1; animation: trace 1.65s .15s ease-in-out both; }
.rewind-head circle { fill: var(--panel); stroke: var(--incoming); stroke-width: 2; }
.rewind-head path { stroke: var(--incoming); stroke-width: 2; }
.rewind-head { animation: rewind 1.85s ease-in-out both; }
.stable-track { stroke: var(--current); stroke-width: 4; }
.track-node { fill: var(--current); }
.removed-branch { stroke: var(--incoming); stroke-width: 3; animation: fade-removed 1.4s .25s both; }
.drop-guide { stroke: var(--border); stroke-dasharray: 4 7; }
.trash { color: var(--muted-foreground); animation: trash-ready 1.7s both; }
.trash path { stroke: currentColor; stroke-width: 2.5; }
.trash-lid { transform-origin: 444px 45px; animation: open-lid 1.65s both; }
.discarded-ref rect { fill: var(--panel); stroke: var(--incoming); stroke-width: 2; }
.discarded-ref path { stroke: var(--incoming); stroke-width: 2; }
.discarded-ref { transform-origin: 267px 45px; animation: drop-ref 1.65s cubic-bezier(.5, 0, .2, 1) both; }
.branch-growth { stroke: var(--joined); stroke-width: 4; stroke-dasharray: 1; animation: trace 1.5s .15s ease-out both; }
.grown-tip { animation: reveal .35s 1.35s both; }
.rename-stem { stroke: var(--muted-foreground); stroke-width: 2; }
.old-label, .new-label { transform-origin: 274px 47px; }
.old-label { animation: old-label .75s .25s both; }
.new-label { animation: new-label .65s .9s both; }
.old-label rect, .new-label rect { fill: var(--panel); stroke-width: 2; }
.old-label rect, .old-label path { stroke: var(--muted-foreground); }
.new-label rect, .new-label path { stroke: var(--joined); }
.scene-ends { display: flex; justify-content: space-between; gap: 12px; color: var(--muted-foreground); font-size: 11px; }
.scene-ends span { min-width: 0; }
.scene-ends span:last-child { text-align: right; }
.scene-ends code { display: block; color: var(--foreground); }
.ref-result { padding: 10px 16px; font-size: 12px; }
.ref-result svg { color: var(--joined); }
.ref-scene[data-pending="true"] * { animation: none; }
.ref-scene[data-pending="true"] .rewind-trail, .ref-scene[data-pending="true"] .rewind-head { opacity: 0; }
.ref-scene[data-pending="true"] .new-tip { fill: none; stroke: var(--incoming); stroke-width: 2; }
@keyframes rewind { 0%, 9% { transform: translateX(428px); opacity: 1; } 95% { opacity: 1; } 100% { transform: translateX(0); opacity: 0; } }
@keyframes trace { from { stroke-dashoffset: 1; } }
@keyframes reveal { from { opacity: 0; } }
@keyframes drop-ref {
  0%, 10% { transform: none; opacity: 1; }
  66% { transform: translate(152px, -25px) rotate(12deg) scale(.65); opacity: 1; }
  100% { transform: translate(152px, 31px) rotate(25deg) scale(.28); opacity: 0; }
}
@keyframes open-lid { 0%, 26%, 100% { transform: none; } 53%, 81% { transform: rotate(31deg); } }
@keyframes trash-ready { 72% { color: var(--muted-foreground); } 87% { color: var(--joined); } }
@keyframes fade-removed { to { opacity: .12; } }
@keyframes old-label { 0% { opacity: 1; } 100% { opacity: 0; transform: scaleX(.12); } }
@keyframes new-label { 0% { opacity: 0; transform: scaleX(.12); } 100% { opacity: 1; transform: none; } }
@media (prefers-reduced-motion: reduce) {
  .ref-scene * { animation: none !important; }
  .rewind-head, .discarded-ref, .old-label { display: none; }
  .removed-branch { opacity: .12; }
}
</style>

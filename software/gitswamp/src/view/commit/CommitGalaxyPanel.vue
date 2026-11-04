<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { Crosshair, GitBranch, RotateCcw, Settings2, X } from "lucide-vue-next";
import type { BranchInfo, CommitInfo } from "@/types";
import { buildGalaxyScene, type GalaxyLayout, type GalaxyMotion, type GalaxyNode } from "./galaxyLayout";
import { GalaxyThree } from "./galaxyThree";

const props = defineProps<{ commits: CommitInfo[]; branches: BranchInfo[]; currentBranch: string; selectedSha?: string | null; hasMore?: boolean }>();
const emit = defineEmits<{ close: []; loadMore: []; loadAll: []; select: [payload: { commit: CommitInfo | null; additive?: boolean }] }>();
const container = ref<HTMLElement | null>(null), canvas = ref<HTMLCanvasElement | null>(null), mapCanvas = ref<HTMLCanvasElement | null>(null);
const layout = ref<GalaxyLayout>("spiral"), motion = ref<GalaxyMotion>("threeD"), menu = ref(false), failure = ref("");
const labels = ref(true), guides = ref(true), rings = ref(false), aura = ref(true), depth = ref(true), merges = ref(true), miniMap = ref(false);
const hovered = shallowRef<GalaxyNode | null>(null);
const tooltip = ref({ x: 0, y: 0 });
const projectedLabels = shallowRef<{ sha: string; text: string; color: string; x: number; y: number }[]>([]);
const model = computed(() => buildGalaxyScene(props.commits, props.branches, props.currentBranch));
const labelNodes = computed(() => model.value.nodes.filter(node => node.label).slice(-24));
let engine: GalaxyThree | null = null, observer: ResizeObserver | null = null, mounted = false;
let pointerStart = { x: 0, y: 0 }, moved = false, lastHover = 0;

function updateLabels() {
  if (!engine || !labels.value) { projectedLabels.value = []; return; }
  const occupied: { x: number; y: number }[] = [];
  projectedLabels.value = labelNodes.value.flatMap(node => {
    const point = engine!.project(node);
    if (!point.visible || occupied.some(other => Math.abs(point.x - other.x) < 110 && Math.abs(point.y - other.y) < 25)) return [];
    occupied.push(point);
    return [{ sha: node.commit.sha, text: node.label, color: node.color, x: point.x, y: point.y }];
  });
}
function optionsChanged() {
  engine?.setOptions({ guides: guides.value, rings: rings.value, aura: aura.value, depth: depth.value, merges: merges.value });
  updateLabels();
}
function sceneChanged() { hovered.value = null; engine?.setScene(model.value, layout.value); optionsChanged(); drawMap(); }
function resetView() { engine?.reset(); }
function focusSelected() { const sha = props.selectedSha || model.value.nodes[model.value.nodes.length - 1]?.commit.sha; if (sha) engine?.focus(sha); }
function point(event: PointerEvent | MouseEvent) { const rect = canvas.value!.getBoundingClientRect(); return { x: event.clientX - rect.left, y: event.clientY - rect.top }; }
function pointerDown(event: PointerEvent) { pointerStart = point(event); moved = false; }
function pointerMove(event: PointerEvent) {
  if (!engine || !canvas.value) return;
  const p = point(event);
  if (event.buttons) { moved ||= Math.hypot(p.x - pointerStart.x, p.y - pointerStart.y) > 4; return; }
  if (performance.now() - lastHover < 40) return;
  lastHover = performance.now();
  const node = engine.pick(p.x, p.y);
  tooltip.value = { x: Math.max(8, Math.min(p.x + 14, canvas.value.clientWidth - 290)), y: Math.max(8, Math.min(p.y + 14, canvas.value.clientHeight - 125)) };
  if (node?.commit.sha !== hovered.value?.commit.sha) { hovered.value = node; engine.setHighlight(props.selectedSha ?? null, node?.commit.sha ?? null); }
}
function pointerLeave() { hovered.value = null; engine?.setHighlight(props.selectedSha ?? null, null); }
function selectPoint(event: MouseEvent) {
  if (moved || !engine) return;
  const p = point(event), node = engine.pick(p.x, p.y);
  if (node) emit("select", { commit: node.commit, additive: event.ctrlKey || event.metaKey });
}
function drawMap() {
  const target = mapCanvas.value, ctx = target?.getContext("2d");
  if (!target || !ctx) return;
  ctx.clearRect(0, 0, target.width, target.height);
  const stride = Math.max(1, Math.ceil(model.value.nodes.length / 1200));
  for (let i = 0; i < model.value.nodes.length; i += stride) {
    const node = model.value.nodes[i], [x, , z] = node.positions.spiral;
    ctx.fillStyle = node.color; ctx.fillRect(70 + x * 0.14, 70 + z * 0.14, 1.5, 1.5);
  }
}
function mapFocus(event: MouseEvent) {
  const rect = mapCanvas.value!.getBoundingClientRect();
  const x = ((event.clientX - rect.left) / rect.width * 140 - 70) / 0.14, z = ((event.clientY - rect.top) / rect.height * 140 - 70) / 0.14;
  let nearest: GalaxyNode | undefined, distance = Infinity;
  for (const node of model.value.nodes) { const p = node.positions.spiral, d = Math.hypot(x - p[0], z - p[2]); if (d < distance) { nearest = node; distance = d; } }
  if (nearest) engine?.focus(nearest.commit.sha);
}
function onEscape(event: KeyboardEvent) { if (event.key === "Escape") { menu.value = false; pointerLeave(); } }
onMounted(async () => {
  mounted = true; await nextTick();
  if (!mounted || !canvas.value || !container.value) return;
  try {
    engine = new GalaxyThree(canvas.value, updateLabels, () => { failure.value = "Graphics context was lost. Reopen Galaxy to retry."; });
    const resize = () => { if (container.value) engine?.resize(container.value.clientWidth, container.value.clientHeight); };
    observer = new ResizeObserver(resize); observer.observe(container.value);
    engine.setMotion(motion.value); resize(); sceneChanged();
  } catch { failure.value = "Galaxy requires WebGL. The regular Git graph remains available."; }
});
onUnmounted(() => { mounted = false; observer?.disconnect(); engine?.dispose(); engine = null; projectedLabels.value = []; });
watch([model, layout], sceneChanged);
watch(motion, value => engine?.setMotion(value));
watch([labels, guides, rings, aura, depth, merges], optionsChanged);
watch(miniMap, async () => { await nextTick(); drawMap(); });
watch(() => props.selectedSha, sha => engine?.setHighlight(sha ?? null, hovered.value?.commit.sha ?? null));
</script>

<template>
  <section class="galaxy-panel" @keydown="onEscape">
    <header class="galaxy-toolbar">
      <div class="galaxy-heading"><strong>Commit Galaxy</strong><span>{{ model.nodes.length.toLocaleString() }} commits · {{ model.lanes.length }} lanes</span></div>
      <div class="galaxy-controls">
        <select v-model="layout" aria-label="Galaxy layout" title="Galaxy layout"><option value="spiral">Spiral</option><option value="layers">Branch layers</option><option value="constellation">Constellation</option><option value="tree">Tree</option></select>
        <select v-model="motion" aria-label="Galaxy motion" title="Galaxy motion"><option value="threeD">3D</option><option value="balanced">Balanced</option><option value="circling">Circling</option></select>
        <button title="Reset view" aria-label="Reset view" @click="resetView"><RotateCcw :size="16" /></button>
        <button title="Focus selected commit" aria-label="Focus selected commit" @click="focusSelected"><Crosshair :size="16" /></button>
        <button title="Toggle tree view" aria-label="Toggle tree view" :aria-pressed="layout === 'tree'" @click="layout = layout === 'tree' ? 'spiral' : 'tree'"><GitBranch :size="16" /></button>
        <div class="galaxy-menu-anchor">
          <button title="View options" aria-label="View options" :aria-expanded="menu" @click="menu = !menu"><Settings2 :size="16" /></button>
          <div v-if="menu" class="galaxy-options">
            <label><input v-model="labels" type="checkbox" />Labels</label><label><input v-model="guides" type="checkbox" />Parent connections</label><label><input v-model="rings" type="checkbox" />Time rings</label><label><input v-model="aura" type="checkbox" />Galaxy stars</label><label><input v-model="depth" type="checkbox" />Depth</label><label><input v-model="merges" type="checkbox" />Merge highlights</label><label><input v-model="miniMap" type="checkbox" />Mini map</label>
          </div>
        </div>
        <button v-if="hasMore" class="galaxy-command" @click="emit('loadMore')">Load more</button>
        <button v-if="hasMore" class="galaxy-command" @click="emit('loadAll')">Load all</button>
        <button title="Back to Git graph" aria-label="Back to Git graph" @click="emit('close')"><X :size="17" /></button>
      </div>
    </header>
    <div ref="container" class="galaxy-stage">
      <canvas ref="canvas" class="galaxy-canvas" aria-label="Interactive commit galaxy" @pointerdown="pointerDown" @pointermove="pointerMove" @pointerleave="pointerLeave" @click="selectPoint" />
      <div v-if="failure" class="galaxy-state" role="status">{{ failure }}</div>
      <div v-else-if="!model.nodes.length" class="galaxy-state">No commits to display</div>
      <template v-else>
        <span v-for="label in projectedLabels" :key="label.sha" class="galaxy-label" :style="{ left: label.x + 'px', top: label.y + 'px', borderColor: label.color }">{{ label.text }}</span>
        <div v-if="hovered" class="galaxy-tooltip" :style="{ left: tooltip.x + 'px', top: tooltip.y + 'px' }"><strong>{{ hovered.commit.short_sha }}</strong><p>{{ hovered.commit.message }}</p><span>{{ hovered.commit.author_name }} · {{ hovered.commit.time_ago }}</span><span>{{ model.lanes[hovered.lane]?.name }}{{ hovered.commit.parent_shas.length > 1 ? ' · merge' : '' }}</span></div>
        <canvas v-if="miniMap" ref="mapCanvas" width="140" height="140" class="galaxy-map" title="Focus a region" aria-label="Galaxy mini map" @click="mapFocus" />
      </template>
    </div>
  </section>
</template>

<style scoped>
.galaxy-panel { height: 100%; min-height: 0; display: flex; flex-direction: column; color: #e7e9ed; background: #050609; }
.galaxy-toolbar { display: flex; align-items: center; flex-wrap: wrap; gap: 10px 18px; padding: 10px 12px; border-bottom: 1px solid #ffffff1a; background: #101216; z-index: 3; }
.galaxy-heading { display: flex; flex-direction: column; gap: 2px; margin-right: auto; } .galaxy-heading strong { font-size: 14px; } .galaxy-heading span { color: #989fa9; font-size: 11px; }
.galaxy-controls { display: flex; align-items: center; flex-wrap: wrap; gap: 5px; }
button, select { height: 30px; border: 1px solid #ffffff24; border-radius: 4px; background: #1b1f25; color: #e7e9ed; font-size: 12px; } button { display: grid; place-items: center; width: 30px; flex-shrink: 0; } button:hover, button[aria-pressed="true"] { background: #323942; } select { padding: 0 6px; max-width: 145px; } .galaxy-command { padding-inline: 8px; width: auto; }
button:focus-visible, select:focus-visible { outline: 2px solid #8ed9f5; outline-offset: 2px; }
.galaxy-stage { position: relative; flex: 1; min-height: 0; overflow: hidden; } .galaxy-canvas { display: block; width: 100%; height: 100%; cursor: grab; touch-action: none; } .galaxy-canvas:active { cursor: grabbing; }
.galaxy-menu-anchor { position: relative; } .galaxy-options { position: absolute; top: 36px; right: 0; width: 185px; max-height: 60dvh; overflow: auto; padding: 8px; border: 1px solid #ffffff30; border-radius: 6px; background: #181c22; box-shadow: 0 8px 24px #0008; } .galaxy-options label { display: flex; gap: 8px; align-items: center; padding: 6px; font-size: 12px; } input { accent-color: #8ed9f5; }
.galaxy-label { position: absolute; pointer-events: none; transform: translate(9px, -50%); padding: 3px 6px; max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; border-left: 2px solid; background: #0b0e13d9; font-size: 10px; }
.galaxy-tooltip { position: absolute; pointer-events: none; max-width: min(280px, calc(100% - 16px)); padding: 10px; border: 1px solid #ffffff30; border-radius: 4px; background: #13171ff2; font-size: 12px; box-shadow: 0 8px 24px #0006; overflow-wrap: anywhere; } .galaxy-tooltip strong { color: #8ed9f5; } .galaxy-tooltip p { margin-block: 5px; max-height: 75px; overflow: hidden; } .galaxy-tooltip span { display: block; font-size: 11px; color: #a5adb8; }
.galaxy-state { position: absolute; inset: 0; display: grid; place-items: center; padding: 24px; text-align: center; color: #b9c0ca; pointer-events: none; font-size: 13px; } .galaxy-map { position: absolute; bottom: 12px; right: 12px; width: 140px; height: 140px; background: #080b10d9; border: 1px solid #ffffff20; border-radius: 4px; cursor: crosshair; }
@media(max-width: 600px) { .galaxy-toolbar { padding: 8px; } .galaxy-controls { width: 100%; } select { max-width: 125px; } .galaxy-label { max-width: 90px; } .galaxy-map { width: 100px; height: 100px; } }
</style>

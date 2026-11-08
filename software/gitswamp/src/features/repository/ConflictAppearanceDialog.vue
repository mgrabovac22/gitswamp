<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { FileCode2, GitMerge } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import { CONFLICT_PREVIEW_MS, type ConflictVisualisation } from "./useConflictVisualisation";
import { usePreviewEscape } from "./usePreviewEscape";

interface PreviewLine {
  number: number;
  text: string;
  conflicted: boolean;
}

interface PreviewFile {
  ours: string | null;
  theirs: string | null;
  ours_code: PreviewLine[];
  theirs_code: PreviewLine[];
  ranges: [number, number][];
  partial: boolean;
}

const props = defineProps<{ preview: ConflictVisualisation }>();
const emit = defineEmits<{ close: [] }>();
usePreviewEscape(() => emit("close"));
const file = shallowRef<PreviewFile | null>(null);
const loading = ref(true);
const frame = ref(0);
const path = computed(() => props.preview.paths[frame.value]);
const range = computed(() => file.value?.ranges[0]);
const sides = [
  { id: "ours", title: "Current", code: "ours_code" },
  { id: "theirs", title: "Incoming", code: "theirs_code" },
] as const;
let timer: ReturnType<typeof setTimeout> | undefined;
let mounted = false;

async function showFile() {
  file.value = null;
  loading.value = true;
  try {
    const result = await invoke<PreviewFile | null>("get_conflict_preview", {
      path: props.preview.repoPath,
      filePath: path.value,
    });
    if (mounted) file.value = result;
  } catch {
    // A file can disappear or be resolved externally during the animation.
  }
  if (!mounted) return;
  loading.value = false;
  await nextTick();
  if (!mounted) return;
  timer = setTimeout(() => {
    if (frame.value + 1 >= props.preview.paths.length) emit("close");
    else {
      frame.value++;
      void showFile();
    }
  }, CONFLICT_PREVIEW_MS);
}

onMounted(() => {
  mounted = true;
  void showFile();
});

onBeforeUnmount(() => {
  mounted = false;
  clearTimeout(timer);
});
</script>

<template>
  <Teleport to="body">
    <dialog open class="conflict-preview" aria-labelledby="conflict-preview-title"
      :style="{ '--file-duration': CONFLICT_PREVIEW_MS + 'ms' }">
      <header class="preview-header">
        <GitMerge class="h-5 w-5 text-[var(--destructive)] shrink-0" />
        <h2 id="conflict-preview-title">New conflicts</h2>
        <span class="file-count" aria-label="File progress">{{ frame + 1 }} / {{ preview.paths.length }}</span>
        <CloseIconButton title="Close conflict preview" size="sm" @click="emit('close')" />
      </header>
      <div class="file-name" :title="path">
        <FileCode2 class="h-4 w-4 shrink-0" />
        <span>{{ path }}</span>
      </div>
      <div class="code-deck" :aria-busy="loading">
        <p v-if="loading" class="placeholder">Reading conflict…</p>
        <template v-else-if="file">
          <section v-for="side in sides" :key="frame + side.id" class="code-sheet" :class="'code-sheet--' + side.id" :aria-label="side.title + ' code'">
            <div class="sheet-header">
              <strong>{{ side.title }}</strong>
              <span :title="file[side.id] ?? undefined">{{ file[side.id] ?? 'No file on this side' }}</span>
            </div>
            <div v-if="file[side.code].length" class="code-lines">
              <div v-for="line in file[side.code]" :key="line.number" class="code-row" :class="{ 'conflicted-line': line.conflicted }">
                <span class="line-number">{{ line.number }}</span>
                <code :title="line.text">{{ line.text || ' ' }}</code>
              </div>
            </div>
            <p v-else class="empty-code">No text lines in this excerpt</p>
          </section>
        </template>
        <p v-else class="placeholder">Conflict preview unavailable</p>
      </div>
      <footer>
        <span v-if="range" class="range">Working tree lines {{ range[0] }}–{{ range[1] }}</span>
        <span v-else-if="!loading">No text conflict markers available</span>
        <span v-if="file && file.ranges.length > 1">+{{ file.ranges.length - 1 }}{{ file.partial ? '+' : '' }} more regions</span>
      </footer>
      <div class="time-track" aria-hidden="true"><div v-if="!loading" :key="frame" /></div>
    </dialog>
  </Teleport>
</template>

<style scoped>
.conflict-preview {
  z-index: 45;
  width: min(700px, calc(100vw - 24px));
  max-height: calc(100dvh - 24px);
  margin: auto;
  padding: 0;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--foreground);
  background: var(--popover);
  box-shadow: 0 12px 40px #0005;
}
.preview-header { display: flex; align-items: center; gap: 10px; padding: 11px 14px; border-bottom: 1px solid var(--border); }
h2 { flex: 1; font-size: 14px; font-weight: 650; }
.file-count { color: var(--muted-foreground); font-size: 11px; font-variant-numeric: tabular-nums; }
.file-name { display: flex; align-items: center; gap: 7px; padding: 12px 14px 0; font-size: 12px; font-weight: 600; }
.file-name span, .sheet-header span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.code-deck { position: relative; height: 242px; overflow: hidden; isolation: isolate; perspective: 950px; }
.code-sheet {
  position: absolute;
  inset: 22px 42px auto;
  height: 186px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--background);
  box-shadow: 0 8px 20px #0003;
  animation-duration: var(--file-duration);
  animation-timing-function: cubic-bezier(.22, .61, .36, 1);
  animation-fill-mode: both;
}
.code-sheet--ours { animation-name: current-sheet, current-order; }
.code-sheet--theirs { animation-name: incoming-sheet, incoming-order; }
.sheet-header { display: flex; gap: 9px; align-items: center; height: 34px; padding-inline: 10px; border-bottom: 1px solid var(--border); background: var(--secondary); font-size: 11px; }
.sheet-header span { color: var(--muted-foreground); }
.code-sheet--ours .sheet-header { border-left: 3px solid var(--primary); }
.code-sheet--theirs .sheet-header { border-left: 3px solid var(--muted-foreground); }
.code-lines { padding-block: 5px; }
.code-row {
  display: grid;
  grid-template-columns: 46px minmax(0, 1fr);
  height: 20px;
  border-left: 3px solid transparent;
  font: 11px/20px ui-monospace, Consolas, monospace;
}
.line-number { text-align: right; padding-right: 9px; color: var(--muted-foreground); }
.code-row code { overflow: hidden; text-overflow: ellipsis; white-space: pre; tab-size: 4; }
.conflicted-line { animation: collision-highlight 240ms 850ms ease-out both; }
.placeholder { display: grid; height: 100%; place-items: center; font-size: 12px; color: var(--muted-foreground); }
.empty-code { padding: 13px; font-size: 11px; color: var(--muted-foreground); }
footer { min-height: 31px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 5px; padding: 4px 14px 9px; font-size: 11px; color: var(--muted-foreground); }
.range { border-left: 3px solid var(--destructive); padding-left: 6px; color: var(--foreground); }
.time-track { height: 2px; background: var(--secondary); }
.time-track div { height: 100%; background: var(--primary); transform-origin: left; animation: run-out var(--file-duration) linear both; }
@keyframes current-sheet {
  0% { transform: translate3d(-35px, 16px, 0) rotateY(12deg); }
  28%, 54% { transform: translate3d(-10px, 10px, 0) rotateY(0); }
  68%, 100% { transform: translate3d(-17px, -12px, -28px) rotateY(4deg); opacity: .72; }
}
@keyframes incoming-sheet {
  0% { transform: translate3d(42px, -12px, -28px) rotateY(-16deg); opacity: .52; }
  28%, 54% { transform: translate3d(17px, -12px, -28px) rotateY(-4deg); opacity: .72; }
  68%, 100% { transform: translate3d(10px, 10px, 0) rotateY(0); opacity: 1; }
}
@keyframes current-order { 0%, 60% { z-index: 2; } 61%, 100% { z-index: 1; } }
@keyframes incoming-order { 0%, 60% { z-index: 1; } 61%, 100% { z-index: 2; } }
@keyframes collision-highlight {
  to { border-color: var(--destructive); background: color-mix(in srgb, var(--destructive) 17%, var(--background)); }
}
@keyframes run-out { to { transform: scaleX(0); } }
@media (max-width: 500px) { .code-sheet { inset-inline: 22px; } .code-row { grid-template-columns: 35px minmax(0, 1fr); } }
@media (prefers-reduced-motion: reduce) {
  .code-sheet { animation-timing-function: steps(1, end); }
  .code-sheet--ours { animation-name: current-order; transform: translate(-5px, 8px); }
  .code-sheet--theirs { animation-name: incoming-order; transform: translate(5px, -8px); }
  .conflicted-line { animation: none; border-color: var(--destructive); background: color-mix(in srgb, var(--destructive) 17%, var(--background)); }
  .time-track div { animation: none; }
}
</style>

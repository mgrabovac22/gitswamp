<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, ref, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertTriangle,
  ArrowLeftRight,
  Blend,
  Columns2,
  FileText,
  Image as ImageIcon,
  Loader2,
  RotateCw,
} from "lucide-vue-next";
import type { BinaryDiffInfo, BinarySideInfo } from "@/types";
import {
  MAX_IMAGE_PREVIEW_BYTES,
  imageMimeType,
  isImageFilePath,
} from "@/shared/config/imagePreview";
import { validateImagePreview, withBinaryPreviewSlot } from "@/shared/config/binaryPreview";

const props = defineProps<{
  repoPath: string;
  filePath: string;
  oldPath?: string | null;
  commitSha?: string | null;
  staged?: boolean;
}>();

type BinaryViewMode = "side-by-side" | "slider" | "difference";

const modeOptions: Array<{ id: BinaryViewMode; label: string }> = [
  { id: "side-by-side", label: "Side by side" },
  { id: "slider", label: "Slider" },
  { id: "difference", label: "Difference" },
];

const mode = ref<BinaryViewMode>("side-by-side");
const info = shallowRef<BinaryDiffInfo | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);
const oldError = ref<string | null>(null);
const newError = ref<string | null>(null);
const oldUrl = ref<string | null>(null);
const newUrl = ref<string | null>(null);
const oldDimensions = reactive({ width: 0, height: 0 });
const newDimensions = reactive({ width: 0, height: 0 });
const sliderPosition = ref(50);
const dragging = ref(false);
const sliderStage = ref<HTMLElement | null>(null);
const stageSize = reactive({ width: 0, height: 0 });
let stageObserver: ResizeObserver | null = null;
let loadToken = 0;
let loadedKey = "";
let disposed = false;
let refreshing = false;

const isImage = computed(() => isImageFilePath(props.filePath));
const fileName = computed(() => {
  const segments = props.filePath.split(/[\\/]/);
  return segments[segments.length - 1] || props.filePath;
});
const oldTargetPath = computed(() => props.oldPath || props.filePath);
const canCompare = computed(() => Boolean(oldUrl.value && newUrl.value));
const comparisonStyles = computed(() => {
  const width = Math.max(oldDimensions.width, newDimensions.width, 1);
  const height = Math.max(oldDimensions.height, newDimensions.height, 1);
  const scale = Math.min(1, stageSize.width / width, stageSize.height / height);
  const left = `${(stageSize.width - width * scale) / 2}px`;
  const top = `${(stageSize.height - height * scale) / 2}px`;
  const style = (dimensions: typeof oldDimensions) => ({
    width: `${dimensions.width * scale}px`, height: `${dimensions.height * scale}px`, left, top,
  });
  return { old: style(oldDimensions), new: style(newDimensions) };
});
const oldTooLarge = computed(
  () => Boolean(info.value?.old.exists) && (info.value?.old.size ?? 0) > MAX_IMAGE_PREVIEW_BYTES,
);
const newTooLarge = computed(
  () => Boolean(info.value?.new.exists) && (info.value?.new.size ?? 0) > MAX_IMAGE_PREVIEW_BYTES,
);
const compareHint = computed(() => {
  if (!info.value) return "";
  if (!info.value.old.exists) return "No previous version to compare.";
  if (!info.value.new.exists) return "This file has no current version to compare.";
  if (!canCompare.value) return "Both images must be available for comparison.";
  return "";
});

function resetUrls() {
  for (const url of new Set([oldUrl.value, newUrl.value])) {
    if (url) URL.revokeObjectURL(url);
  }
  oldUrl.value = null;
  newUrl.value = null;
}

function toBytes(value: ArrayBuffer | Uint8Array | number[]): Uint8Array | null {
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  if (value instanceof Uint8Array) return value;
  if (Array.isArray(value)) return Uint8Array.from(value);
  return null;
}

function bytesToUrl(bytes: Uint8Array, path: string): string {
  const blob = new Blob([bytes], { type: imageMimeType(path) });
  return URL.createObjectURL(blob);
}

async function loadSide(
  side: "old" | "new",
  sideInfo: BinarySideInfo,
  token: number,
): Promise<string | null> {
  if (!sideInfo.exists || sideInfo.size > MAX_IMAGE_PREVIEW_BYTES || token !== loadToken) return null;
  const args = {
    path: props.repoPath,
    filePath: props.filePath,
    oldPath: props.oldPath ?? null,
    sha: props.commitSha ?? null,
    staged: props.staged ?? false,
    side,
  };
  const imagePath = side === "old" ? oldTargetPath.value : props.filePath;
  try {
    return await withBinaryPreviewSlot(() => !disposed && token === loadToken, async () => {
      const raw = await invoke<ArrayBuffer | Uint8Array | number[]>("get_file_bytes", args);
      if (disposed || token !== loadToken) return null;
      const bytes = toBytes(raw);
      if (!bytes) throw new Error("Image data could not be read");
      validateImagePreview(bytes, imagePath);
      return bytesToUrl(bytes, imagePath);
    }) ?? null;
  } catch (err) {
    if (token === loadToken && !disposed) {
      (side === "old" ? oldError : newError).value = err instanceof Error ? err.message : String(err);
    }
    return null;
  }
}

async function loadBinary(checkOnly = false) {
  if (disposed || (checkOnly && (loading.value || refreshing))) return;
  refreshing = true;
  const token = ++loadToken;
  if (!checkOnly) {
    resetUrls();
    info.value = null;
    error.value = null;
    loading.value = true;
    loadedKey = "";
  }

  try {
    const result = await invoke<BinaryDiffInfo>("get_binary_diff_info", {
      path: props.repoPath,
      filePath: props.filePath,
      oldPath: props.oldPath ?? null,
      sha: props.commitSha ?? null,
      staged: props.staged ?? false,
    });

    if (token !== loadToken) return;
    info.value = result;
    error.value = null;
    const key = JSON.stringify([result.old, result.new]);
    if (checkOnly && key === loadedKey) return;
    loadedKey = key;
    resetUrls();
    oldError.value = null;
    newError.value = null;
    error.value = null;
    loading.value = true;
    mode.value = "side-by-side";
    sliderPosition.value = 50;
    dragging.value = false;
    oldDimensions.width = oldDimensions.height = 0;
    newDimensions.width = newDimensions.height = 0;

    if (!isImage.value) return;

    // Publish each side immediately so invalidation/unmount always owns its URL.
    const oldValue = await loadSide("old", result.old, token);
    if (token !== loadToken) {
      if (oldValue) URL.revokeObjectURL(oldValue);
      return;
    }
    oldUrl.value = oldValue;
    const sameBlob = result.old.id && result.old.id === result.new.id;
    const newValue = sameBlob && oldValue ? oldValue : await loadSide("new", result.new, token);
    if (token !== loadToken) {
      if (newValue && newValue !== oldValue) URL.revokeObjectURL(newValue);
      return;
    }
    newUrl.value = newValue;
  } catch (err) {
    if (token === loadToken) {
      error.value = err instanceof Error ? err.message : String(err);
    }
  } finally {
    if (token === loadToken) {
      loading.value = false;
      refreshing = false;
    }
  }
}

defineExpose({ refresh: () => loadBinary(true) });

function updateSlider(clientX: number) {
  const stage = sliderStage.value;
  if (!stage) return;
  const rect = stage.getBoundingClientRect();
  if (rect.width <= 0) return;
  sliderPosition.value = Math.min(100, Math.max(0, ((clientX - rect.left) / rect.width) * 100));
}

function onSliderPointerDown(event: PointerEvent) {
  if (!canCompare.value) return;
  dragging.value = true;
  (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
  updateSlider(event.clientX);
}

function onSliderPointerMove(event: PointerEvent) {
  if (dragging.value) updateSlider(event.clientX);
}

function onSliderPointerUp(event: PointerEvent) {
  dragging.value = false;
  const target = event.currentTarget as HTMLElement | null;
  if (target?.hasPointerCapture?.(event.pointerId)) {
    target.releasePointerCapture(event.pointerId);
  }
}

function onImageLoad(side: "old" | "new", event: Event) {
  const image = event.target as HTMLImageElement | null;
  if (!image || image.getAttribute("src") !== (side === "old" ? oldUrl.value : newUrl.value)) return;
  const dimensions = side === "old" ? oldDimensions : newDimensions;
  dimensions.width = image.naturalWidth;
  dimensions.height = image.naturalHeight;
}

function onImageError(side: "old" | "new", event: Event) {
  const source = (event.target as HTMLImageElement | null)?.getAttribute("src");
  const url = side === "old" ? oldUrl : newUrl;
  if (!source || url.value !== source) return;
  (side === "old" ? oldError : newError).value = "Image could not be decoded by this browser";
  url.value = null;
  if (source !== oldUrl.value && source !== newUrl.value) URL.revokeObjectURL(source);
  mode.value = "side-by-side";
}

function formatSize(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 || unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}

function formatDimensions(dimensions: { width: number; height: number }): string {
  return dimensions.width > 0 ? `${dimensions.width} × ${dimensions.height}` : "";
}

function emptyMessage(side: "old" | "new"): string {
  const sideInfo = side === "old" ? info.value?.old : info.value?.new;
  if (!sideInfo?.exists) {
    return side === "old" ? "No previous version" : "No current version";
  }
  const tooLarge = side === "old" ? oldTooLarge.value : newTooLarge.value;
  if (tooLarge) return "File is too large to preview";
  const sideError = side === "old" ? oldError.value : newError.value;
  if (sideError) return sideError;
  return "Image could not be loaded";
}

watch(
  () => [props.repoPath, props.filePath, props.oldPath, props.commitSha, props.staged] as const,
  () => {
    void loadBinary();
  },
  { immediate: true },
);

watch(sliderStage, stage => {
  stageObserver?.disconnect();
  stageObserver = null;
  if (!stage) return;
  const update = () => { stageSize.width = stage.clientWidth; stageSize.height = stage.clientHeight; };
  update();
  stageObserver = new ResizeObserver(update);
  stageObserver.observe(stage);
});

onBeforeUnmount(() => {
  disposed = true;
  stageObserver?.disconnect();
  loadToken += 1;
  resetUrls();
});
</script>

<template>
  <div class="binary-diff-viewer">
    <div class="binary-diff-header">
      <div class="binary-diff-title">
        <ImageIcon v-if="isImage" class="binary-diff-title-icon" />
        <FileText v-else class="binary-diff-title-icon" />
        <span class="binary-diff-name">{{ fileName }}</span>
        <span class="binary-diff-kind">{{ isImage ? "image" : "binary" }}</span>
      </div>
      <div v-if="info" class="binary-diff-meta">
        <span>Previous: {{ info.old.exists ? formatSize(info.old.size) : "—" }}</span>
        <span>Current: {{ info.new.exists ? formatSize(info.new.size) : "—" }}</span>
      </div>
      <button type="button" title="Reload preview" aria-label="Reload preview" :disabled="loading"
        class="binary-diff-reload" @click="loadBinary()">
        <RotateCw class="binary-diff-title-icon" />
      </button>
    </div>

    <div v-if="loading" class="binary-diff-state">
      <Loader2 class="binary-diff-state-icon animate-spin" />
      <span>Loading binary data…</span>
    </div>

    <div v-else-if="error" class="binary-diff-state binary-diff-state--error">
      <AlertTriangle class="binary-diff-state-icon" />
      <span>{{ error }}</span>
    </div>

    <div v-else-if="!isImage" class="binary-diff-state">
      <div class="binary-diff-notice">
        <p class="binary-diff-notice-title">Binary file</p>
        <p class="binary-diff-notice-text">
          Text diff is not available for this file type.
        </p>
      </div>
    </div>

    <template v-else>
      <div class="binary-diff-toolbar">
        <div class="binary-diff-modes" role="group" aria-label="Image comparison mode">
          <button
            v-for="option in modeOptions"
            :key="option.id"
            type="button"
            class="binary-diff-mode"
            :class="{ 'binary-diff-mode--active': mode === option.id }"
            :disabled="option.id !== 'side-by-side' && !canCompare"
            :aria-pressed="mode === option.id"
            :title="option.label"
            @click="mode = option.id"
          >
            <Columns2 v-if="option.id === 'side-by-side'" class="binary-diff-mode-icon" />
            <ArrowLeftRight v-else-if="option.id === 'slider'" class="binary-diff-mode-icon" />
            <Blend v-else class="binary-diff-mode-icon" />
            <span>{{ option.label }}</span>
          </button>
        </div>
        <span v-if="!canCompare" class="binary-diff-hint">{{ compareHint }}</span>
      </div>

      <div class="binary-diff-body">
        <div v-if="mode === 'side-by-side'" class="binary-diff-panes">
          <div class="binary-diff-pane">
            <div class="binary-diff-pane-header">
              <span>{{ info?.old.exists ? "Previous" : "Previous (missing)" }}</span>
              <span v-if="oldDimensions.width">{{ formatDimensions(oldDimensions) }}</span>
            </div>
            <div class="binary-diff-viewport">
              <img
                v-if="oldUrl"
                :src="oldUrl"
                :alt="`Previous version of ${fileName}`"
                class="binary-diff-image"
                @load="onImageLoad('old', $event)"
                @error="onImageError('old', $event)"
              />
              <p v-else class="binary-diff-empty">{{ emptyMessage("old") }}</p>
            </div>
          </div>
          <div class="binary-diff-pane">
            <div class="binary-diff-pane-header">
              <span>{{ info?.new.exists ? "Current" : "Current (missing)" }}</span>
              <span v-if="newDimensions.width">{{ formatDimensions(newDimensions) }}</span>
            </div>
            <div class="binary-diff-viewport">
              <img
                v-if="newUrl"
                :src="newUrl"
                :alt="`Current version of ${fileName}`"
                class="binary-diff-image"
                @load="onImageLoad('new', $event)"
                @error="onImageError('new', $event)"
              />
              <p v-else class="binary-diff-empty">{{ emptyMessage("new") }}</p>
            </div>
          </div>
        </div>

        <div v-else-if="mode === 'slider'" class="binary-diff-slider-wrap">
          <div
            v-if="canCompare"
            ref="sliderStage"
            class="binary-diff-stage"
            @pointerdown="onSliderPointerDown"
            @pointermove="onSliderPointerMove"
            @pointerup="onSliderPointerUp"
            @pointercancel="onSliderPointerUp"
          >
            <span class="binary-diff-side-label binary-diff-side-label--old">Previous</span>
            <span class="binary-diff-side-label binary-diff-side-label--new">Current</span>
            <img
              :src="oldUrl ?? ''"
              :alt="`Previous version of ${fileName}`"
              class="binary-diff-stage-image"
              :style="comparisonStyles.old"
              @error="onImageError('old', $event)"
            />
            <div
              class="binary-diff-slider-layer"
              :style="{ clipPath: `inset(0 0 0 ${sliderPosition}%)` }"
            >
              <img
                :src="newUrl ?? ''"
                :alt="`Current version of ${fileName}`"
                class="binary-diff-stage-image"
                :style="comparisonStyles.new"
                @error="onImageError('new', $event)"
              />
            </div>
            <div class="binary-diff-slider-line" :style="{ left: `${sliderPosition}%` }">
              <span class="binary-diff-slider-handle">
                <ArrowLeftRight class="binary-diff-slider-handle-icon" />
              </span>
            </div>
          </div>
          <input v-if="canCompare" v-model.number="sliderPosition" type="range" min="0" max="100" step="1"
            aria-label="Image comparison position" class="binary-diff-range" />
          <p v-else class="binary-diff-empty">{{ compareHint }}</p>
        </div>

        <div v-else class="binary-diff-slider-wrap">
          <div v-if="canCompare" ref="sliderStage" class="binary-diff-stage binary-diff-stage--difference">
            <img
              :src="oldUrl ?? ''"
              :alt="`Previous version of ${fileName}`"
              class="binary-diff-stage-image"
              :style="comparisonStyles.old"
              @error="onImageError('old', $event)"
            />
            <img
              :src="newUrl ?? ''"
              :alt="`Current version of ${fileName}`"
              class="binary-diff-stage-image binary-diff-stage-image--difference"
              :style="comparisonStyles.new"
              @error="onImageError('new', $event)"
            />
          </div>
          <p v-else class="binary-diff-empty">{{ compareHint }}</p>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.binary-diff-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  min-width: 0;
  background: var(--card);
  color: var(--foreground);
}

.binary-diff-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem 1rem;
  flex-shrink: 0;
  padding: 0.6rem 1rem;
  border-bottom: 1px solid var(--border);
  background: var(--secondary);
}

.binary-diff-title {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
}

.binary-diff-title-icon {
  width: 1rem;
  height: 1rem;
  flex-shrink: 0;
  color: var(--muted-foreground);
}

.binary-diff-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.85rem;
  font-weight: 600;
}

.binary-diff-kind {
  flex-shrink: 0;
  border: 1px solid var(--diff-border);
  border-radius: 0.25rem;
  padding: 0.05rem 0.35rem;
  font-size: 0.6rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0;
  color: var(--muted-foreground);
}

.binary-diff-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  font-size: 0.68rem;
  color: var(--muted-foreground);
  margin-left: auto;
}

.binary-diff-reload { display: grid; place-items: center; width: 1.75rem; height: 1.75rem; flex-shrink: 0; border-radius: 0.25rem; }
.binary-diff-reload:disabled { opacity: 0.45; }

.binary-diff-state {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.6rem;
  padding: 1.5rem;
  text-align: center;
  font-size: 0.8rem;
  color: var(--muted-foreground);
}

.binary-diff-state--error {
  color: var(--destructive);
}

.binary-diff-state-icon {
  width: 1.25rem;
  height: 1.25rem;
}

.binary-diff-notice {
  max-width: 32rem;
  border: 1px solid var(--diff-border);
  border-radius: 0.5rem;
  background: var(--secondary);
  padding: 1rem 1.25rem;
}

.binary-diff-notice-title {
  margin-bottom: 0.35rem;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--foreground);
}

.binary-diff-notice-text {
  line-height: 1.5;
  color: var(--muted-foreground);
}

.binary-diff-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  flex-shrink: 0;
  padding: 0.45rem 1rem;
  border-bottom: 1px solid var(--diff-border);
  background: var(--card);
}

.binary-diff-modes {
  display: flex;
  flex-wrap: wrap;
  border: 1px solid var(--diff-border);
  border-radius: 0.4rem;
}

.binary-diff-mode {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.28rem 0.55rem;
  border: 0;
  border-right: 1px solid var(--diff-border);
  background: var(--secondary);
  font-size: 0.7rem;
  color: var(--muted-foreground);
  transition: background-color 120ms ease, color 120ms ease;
}

.binary-diff-mode:last-child {
  border-right: 0;
}

.binary-diff-mode:hover:not(:disabled) {
  color: var(--foreground);
}

.binary-diff-mode--active {
  background: var(--primary);
  color: #fff;
}

.binary-diff-mode--active:hover:not(:disabled) { color: #fff; }

.binary-diff-mode:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.binary-diff-mode-icon {
  width: 0.8rem;
  height: 0.8rem;
}

.binary-diff-hint {
  font-size: 0.68rem;
  color: var(--muted-foreground);
}

.binary-diff-body {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  padding: 0.75rem;
  background: var(--diff-bg);
}

.binary-diff-panes {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 0.65rem;
  width: 100%;
  min-height: 0;
}

.binary-diff-pane {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--diff-border);
  border-radius: 0.5rem;
  background: var(--card);
}

.binary-diff-pane-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  flex-shrink: 0;
  padding: 0.35rem 0.6rem;
  border-bottom: 1px solid var(--diff-border);
  background: var(--secondary);
  font-size: 0.68rem;
  font-weight: 600;
  color: var(--muted-foreground);
}

.binary-diff-viewport {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
  align-items: center;
  justify-content: center;
  overflow: auto;
  padding: 0.5rem;
  background-image: repeating-conic-gradient(
    color-mix(in srgb, var(--foreground) 7%, transparent) 0% 25%,
    transparent 0% 50%
  );
  background-size: 16px 16px;
}

.binary-diff-image {
  display: block;
  min-width: 0;
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.binary-diff-empty {
  padding: 1rem;
  text-align: center;
  font-size: 0.74rem;
  color: var(--muted-foreground);
}

.binary-diff-slider-wrap {
  display: flex;
  width: 100%;
  min-height: 0;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 0.5rem;
}

.binary-diff-range { width: 100%; flex-shrink: 0; accent-color: var(--primary); }
.binary-diff-side-label { position: absolute; top: 0.5rem; z-index: 2; padding: 0.2rem 0.4rem; background: var(--card); font-size: 0.7rem; pointer-events: none; }
.binary-diff-side-label--old { left: 0.5rem; }
.binary-diff-side-label--new { right: 0.5rem; }

.binary-diff-stage {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--diff-border);
  border-radius: 0.5rem;
  background-color: var(--card);
  background-image: repeating-conic-gradient(
    color-mix(in srgb, var(--foreground) 7%, transparent) 0% 25%,
    transparent 0% 50%
  );
  background-size: 16px 16px;
  cursor: ew-resize;
  touch-action: none;
  isolation: isolate;
}

.binary-diff-stage-image {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: contain;
  pointer-events: none;
  user-select: none;
}

.binary-diff-slider-layer {
  position: absolute;
  inset: 0;
}

.binary-diff-slider-line {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 2px;
  background: var(--primary);
  transform: translateX(-1px);
  pointer-events: none;
}

.binary-diff-slider-handle {
  position: absolute;
  top: 50%;
  left: 50%;
  display: flex;
  width: 2rem;
  height: 2rem;
  align-items: center;
  justify-content: center;
  transform: translate(-50%, -50%);
  border: 2px solid var(--primary);
  border-radius: 9999px;
  background: var(--card);
  color: var(--primary);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
}

.binary-diff-slider-handle-icon {
  width: 1rem;
  height: 1rem;
}

.binary-diff-stage--difference {
  background-color: #fff;
  background-image: none;
  cursor: default;
}

.binary-diff-stage-image--difference {
  mix-blend-mode: difference;
}

@media (max-width: 600px) {
  .binary-diff-panes { grid-template-columns: minmax(0, 1fr); grid-template-rows: minmax(0, 1fr) minmax(0, 1fr); }
}
</style>

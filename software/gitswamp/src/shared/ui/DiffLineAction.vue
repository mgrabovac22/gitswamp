<script setup lang="ts">
import { shallowRef, watch } from "vue";
import { Minus, Plus } from "lucide-vue-next";

const props = defineProps<{ scrollport: HTMLElement | null; staged: boolean; busy: boolean }>();
const emit = defineEmits<{ apply: [hunkIndex: number, lineIndex: number] }>();
const target = shallowRef<{ row: HTMLElement; top: number; right: number; height: number } | null>(null);
const label = () => props.staged ? "Unstage line" : "Stage line";
const clear = () => { target.value = null; };
defineExpose({ clear });

function pick(event: Event) {
  const row = (event.target as Element | null)?.closest<HTMLElement>("[data-diff-line]");
  const port = props.scrollport;
  if (!row || !port || props.busy) { clear(); return; }
  if (target.value?.row === row) return;
  const bounds = port.getBoundingClientRect();
  const line = row.getBoundingClientRect();
  const top = line.top - bounds.top;
  if (top < 0 || top + line.height > port.clientHeight) { clear(); return; }
  const pane = row.closest(".split-diff-scroll");
  // In split view the old and new cells are different Git lines, even on the same row.
  const right = Math.max(port.offsetWidth - port.clientWidth, bounds.right - (pane?.getBoundingClientRect().right ?? bounds.right));
  target.value = { row, top, right, height: line.height };
}

function apply(row: HTMLElement) {
  if (props.busy || !props.scrollport?.contains(row)) return;
  const hunk = row.closest<HTMLElement>("[data-diff-hunk]");
  if (!hunk) return;
  clear();
  emit("apply", Number(hunk.dataset.diffHunk), Number(row.dataset.diffLine));
}

watch(() => props.scrollport, (port, _, cleanup) => {
  clear();
  const surface = port?.parentElement;
  if (!port || !surface) return;
  let scrollTop = port.scrollTop;
  const onScroll = () => {
    if (port.scrollTop !== scrollTop) clear();
    scrollTop = port.scrollTop;
  };
  const onFocusOut = (event: FocusEvent) => {
    if (!surface.contains(event.relatedTarget as Node | null)) clear();
  };
  const onKeydown = (event: KeyboardEvent) => {
    if (event.key !== "Enter" || event.repeat || !(event.target instanceof HTMLElement)
        || event.target.dataset.diffLine === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    apply(event.target);
  };
  port.addEventListener("pointerover", pick);
  port.addEventListener("focusin", pick);
  port.addEventListener("keydown", onKeydown);
  port.addEventListener("scroll", onScroll, { passive: true });
  surface.addEventListener("pointerleave", clear);
  surface.addEventListener("focusout", onFocusOut);
  window.addEventListener("resize", clear);
  cleanup(() => {
    port.removeEventListener("pointerover", pick);
    port.removeEventListener("focusin", pick);
    port.removeEventListener("keydown", onKeydown);
    port.removeEventListener("scroll", onScroll);
    surface.removeEventListener("pointerleave", clear);
    surface.removeEventListener("focusout", onFocusOut);
    window.removeEventListener("resize", clear);
  });
}, { immediate: true });

watch(() => props.busy, busy => { if (busy) clear(); });
</script>

<template>
  <button v-if="target" class="diff-line-action" type="button" tabindex="-1"
    :title="label()" :aria-label="label()" :data-unstage="staged"
    :style="{ top: `${target.top}px`, right: `${target.right}px`, height: `${target.height}px` }"
    @pointerdown.prevent @click="apply(target.row)">
    <Minus v-if="staged" :size="14" /><Plus v-else :size="14" />
  </button>
</template>

<style scoped>
.diff-line-action {
  position: absolute; z-index: 9; width: 24px; display: grid; place-items: center;
  border: 1px solid var(--diff-border); border-radius: 4px 0 0 4px;
  background: color-mix(in srgb, var(--diff-bg) 76%, transparent); backdrop-filter: blur(4px);
  color: var(--diff-sign-add);
}
.diff-line-action[data-unstage="true"] { color: var(--diff-sign-del); }
.diff-line-action:hover { background: var(--secondary); }
</style>

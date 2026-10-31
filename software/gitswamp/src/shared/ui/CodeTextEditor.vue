<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { highlightFilePreview } from "@/shared/codeView";

const props = defineProps<{ modelValue: string; filePath: string }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();
const backdrop = ref<HTMLElement | null>(null);
const textarea = ref<HTMLTextAreaElement | null>(null);
const html = ref<string | null>(null);
const highlightedSource = ref("");
const highlightedPath = ref("");
const colored = computed(() => html.value !== null
  && highlightedSource.value === props.modelValue && highlightedPath.value === props.filePath);
let timer: ReturnType<typeof setTimeout> | undefined;

function syncScroll() {
  if (!backdrop.value || !textarea.value) return;
  backdrop.value.scrollTop = textarea.value.scrollTop;
  backdrop.value.scrollLeft = textarea.value.scrollLeft;
}

watch(() => [props.modelValue, props.filePath] as const, ([source, path]) => {
  clearTimeout(timer);
  timer = setTimeout(() => {
    html.value = highlightFilePreview(source, path);
    highlightedSource.value = source;
    highlightedPath.value = path;
  }, 120);
}, { immediate: true });
watch(colored, syncScroll, { flush: "post" });
onUnmounted(() => clearTimeout(timer));
</script>

<template>
  <div class="code-editor">
    <pre v-show="colored" ref="backdrop" aria-hidden="true" class="code-editor-layer code-editor-backdrop"><code v-html="(html ?? '') + '\n'" /></pre>
    <textarea
      ref="textarea"
      :value="modelValue"
      :class="{ 'code-editor-colored': colored }"
      class="code-editor-layer code-editor-input"
      aria-label="File content"
      spellcheck="false"
      autocapitalize="off"
      autocomplete="off"
      wrap="off"
      @input="emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
      @scroll="syncScroll"
    />
  </div>
</template>

<style scoped>
.code-editor { position: relative; width: 100%; height: 100%; min-height: 0; background: var(--diff-bg); }
.code-editor-layer {
  position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; border: 0;
  box-sizing: border-box; padding: 12px; font-family: monospace; font-size: 12px;
  font-weight: 400; line-height: 18px; letter-spacing: 0; tab-size: 4;
  white-space: pre; overflow-wrap: normal; word-break: normal;
}
.code-editor-backdrop { overflow: hidden; pointer-events: none; color: var(--diff-text); }
.code-editor-backdrop code { font: inherit; letter-spacing: inherit; }
.code-editor-input { overflow: auto; resize: none; outline: none; background: transparent; color: var(--diff-text); caret-color: var(--diff-text); }
.code-editor-colored { color: transparent; }
.code-editor-colored::selection { color: transparent; background: var(--selection-bg, #538ac855); }
</style>

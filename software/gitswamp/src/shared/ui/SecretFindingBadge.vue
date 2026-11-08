<script setup lang="ts">
import { computed, ref, useId } from "vue";
import { AlertTriangle } from "lucide-vue-next";

const props = defineProps<{ findings: string[] }>();
const id = useId();
const anchor = ref<HTMLElement | null>(null);
const pointer = ref(false), focused = ref(false);
const position = ref({ x: 0, y: 0, above: false });
const shown = computed(() => pointer.value || focused.value);

function locate() {
  const rect = anchor.value?.getBoundingClientRect();
  if (!rect) return;
  const width = Math.min(248, window.innerWidth - 16);
  const above = window.innerHeight - rect.bottom < 130 && rect.top >= 130;
  position.value = {
    x: Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)),
    y: above ? rect.top - 6 : rect.bottom + 6,
    above,
  };
}
</script>

<template>
  <span
    ref="anchor"
    role="img"
    tabindex="0"
    class="inline-flex shrink-0 items-center text-amber-600 dark:text-amber-400 rounded-sm focus-visible:outline focus-visible:outline-[var(--ring)]"
    aria-label="Possible secret"
    :aria-describedby="shown ? id : undefined"
    @pointerenter="locate(); pointer = true"
    @pointerleave="pointer = false"
    @focus="locate(); focused = true"
    @blur="focused = false"
    @keydown.esc.stop="pointer = false; focused = false"
  >
    <AlertTriangle class="w-3.5 h-3.5" aria-hidden="true" />
  </span>
  <Teleport to="body">
    <div v-if="shown" :id="id" role="tooltip" class="secret-tooltip" :style="{ left: position.x + 'px', top: position.y + 'px', transform: position.above ? 'translateY(-100%)' : undefined }">
      <strong>Possible secret</strong>
      <div v-for="finding in props.findings.slice(0, 3)" :key="finding">{{ finding }}</div>
      <div v-if="props.findings.length > 3">+{{ props.findings.length - 3 }} more</div>
    </div>
  </Teleport>
</template>

<style scoped>
.secret-tooltip {
  position: fixed;
  z-index: 100;
  pointer-events: none;
  width: min(248px, calc(100vw - 16px));
  padding: 6px 9px;
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--foreground);
  background: var(--popover);
  box-shadow: 0 3px 12px #0003;
  font-size: 11px;
  overflow-wrap: anywhere;
}
</style>

<script setup lang="ts">
import { computed, ref } from "vue";
import { Palette, Copy, Check } from "lucide-vue-next";
import { contrastRatio, hexToRgb, hslToRgb, rgbToHex, rgbToHsl } from "../colorTool";

const hex = ref("#4a9dff");
const copied = ref("");
const bgHex = ref("#111111");
const textHex = ref("#4a9dff");
const pickerOpen = ref(false);

const PALETTE = [
  "#ef4444", "#f97316", "#f59e0b", "#eab308", "#84cc16", "#22c55e",
  "#10b981", "#14b8a6", "#06b6d4", "#0ea5e9", "#3b82f6", "#6366f1",
  "#8b5cf6", "#a855f7", "#d946ef", "#ec4899", "#f43f5e", "#64748b",
  "#0f172a", "#334155", "#94a3b8", "#e2e8f0", "#000000", "#ffffff",
];

const rgb = computed(() => hexToRgb(hex.value));
const hsl = computed(() => (rgb.value ? rgbToHsl(rgb.value) : null));
const ratio = computed(() => contrastRatio(textHex.value, bgHex.value));

function setColor(value: string) {
  hex.value = value.toLowerCase();
  pickerOpen.value = false;
}

function updateFromRgb(r: number, g: number, b: number) {
  const h = rgbToHex({ r: Math.max(0, Math.min(255, r)), g: Math.max(0, Math.min(255, g)), b: Math.max(0, Math.min(255, b)) });
  hex.value = h.toLowerCase();
}

function updateFromHsl(h: number, s: number, l: number) {
  const rgbc = hslToRgb({ h, s, l });
  hex.value = rgbToHex(rgbc).toLowerCase();
}

async function copyValue(key: string, value: string) {
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    copied.value = key;
    setTimeout(() => {
      if (copied.value === key) copied.value = "";
    }, 1200);
  } catch {
    copied.value = "";
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-2 overflow-y-auto pr-1">
    <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
      <div class="flex items-center gap-1.5">
        <Palette class="h-3.5 w-3.5 text-[var(--primary)]" />
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Color converter</span>
      </div>
      <div class="mt-2 flex items-center gap-2">
        <button class="relative h-10 w-10 flex-shrink-0 rounded border border-[var(--border)]" :style="{ backgroundColor: hex }" title="Pick a color" @click="pickerOpen = !pickerOpen">
          <span class="absolute inset-0 flex items-center justify-center text-[9px] font-semibold text-white mix-blend-difference">Pick</span>
        </button>
        <input v-model="hex" spellcheck="false" class="flex-1 rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 font-mono text-[11px] text-[var(--foreground)]" />
        <input :value="hex" type="color" class="h-8 w-8 cursor-pointer rounded border border-[var(--border)] bg-transparent p-0" title="Open color picker" @input="setColor(($event.target as HTMLInputElement).value)" />
        <button class="flex items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copyValue('hex', hex)">
          <component :is="copied === 'hex' ? Check : Copy" class="h-3 w-3" /> {{ copied === 'hex' ? 'Copied' : 'Copy' }}
        </button>
      </div>
      <div v-if="pickerOpen" class="mt-2 rounded-md border border-[var(--border)] bg-[var(--card)] p-2">
        <div class="grid grid-cols-12 gap-1">
          <button
            v-for="swatch in PALETTE"
            :key="swatch"
            class="h-5 w-5 rounded-sm border transition-transform hover:scale-110"
            :class="hex.toLowerCase() === swatch.toLowerCase() ? 'border-[var(--primary)] ring-1 ring-[var(--primary)]' : 'border-[var(--border)]'"
            :style="{ backgroundColor: swatch }"
            :title="swatch"
            @click="setColor(swatch)"
          />
        </div>
      </div>
    </div>

    <div class="grid grid-cols-2 gap-2">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <span class="text-[11px] font-semibold text-[var(--foreground)]">RGB</span>
        <div v-if="rgb" class="mt-1.5 grid grid-cols-3 gap-1.5">
          <input type="number" min="0" max="255" :value="Math.round(rgb.r)" @input="updateFromRgb(Number(($event.target as HTMLInputElement).value||0), rgb.g, rgb.b)" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-center font-mono text-[11px]" />
          <input type="number" min="0" max="255" :value="Math.round(rgb.g)" @input="updateFromRgb(rgb.r, Number(($event.target as HTMLInputElement).value||0), rgb.b)" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-center font-mono text-[11px]" />
          <input type="number" min="0" max="255" :value="Math.round(rgb.b)" @input="updateFromRgb(rgb.r, rgb.g, Number(($event.target as HTMLInputElement).value||0))" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-center font-mono text-[11px]" />
        </div>
        <button v-if="rgb" class="mt-1.5 flex w-full items-center justify-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copyValue('rgb', `rgb(${Math.round(rgb.r)}, ${Math.round(rgb.g)}, ${Math.round(rgb.b)})`)">
          <component :is="copied === 'rgb' ? Check : Copy" class="h-3 w-3" /> {{ copied === 'rgb' ? 'Copied' : 'Copy RGB' }}
        </button>
      </div>
      <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
        <span class="text-[11px] font-semibold text-[var(--foreground)]">HSL</span>
        <div v-if="hsl" class="mt-1.5 grid grid-cols-3 gap-1.5">
          <input type="number" min="0" max="360" :value="Math.round(hsl.h)" @input="updateFromHsl(Number(($event.target as HTMLInputElement).value||0), hsl.s, hsl.l)" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-center font-mono text-[11px]" />
          <input type="number" step="0.01" min="0" max="1" :value="hsl.s" @input="updateFromHsl(hsl.h, Number(($event.target as HTMLInputElement).value||0), hsl.l)" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-center font-mono text-[11px]" />
          <input type="number" step="0.01" min="0" max="1" :value="hsl.l" @input="updateFromHsl(hsl.h, hsl.s, Number(($event.target as HTMLInputElement).value||0))" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 text-center font-mono text-[11px]" />
        </div>
        <button v-if="hsl" class="mt-1.5 flex w-full items-center justify-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copyValue('hsl', `hsl(${Math.round(hsl.h)}, ${Math.round(hsl.s*100)}%, ${Math.round(hsl.l*100)}%)`)">
          <component :is="copied === 'hsl' ? Check : Copy" class="h-3 w-3" /> {{ copied === 'hsl' ? 'Copied' : 'Copy HSL' }}
        </button>
      </div>
    </div>

    <div class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
      <span class="text-[11px] font-semibold text-[var(--foreground)]">Contrast ratio</span>
      <div class="mt-1.5 flex flex-wrap items-center gap-2">
        <div class="flex items-center gap-1.5">
          <span class="text-[10px] text-[var(--muted-foreground)]">Text</span>
          <input v-model="textHex" class="w-20 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 font-mono text-[11px]" />
          <span class="text-[10px] text-[var(--muted-foreground)]">Background</span>
          <input v-model="bgHex" class="w-20 rounded border border-[var(--border)] bg-[var(--input-background)] px-1.5 py-1 font-mono text-[11px]" />
        </div>
        <span v-if="ratio" class="text-[11px] font-semibold text-[var(--foreground)]">{{ ratio }} : 1 <span class="text-[10px] text-[var(--muted-foreground)]">(WCAG: AA needs ≥4.5 for normal text)</span></span>
        <span v-else class="text-[10px] text-[var(--destructive)]">Invalid color</span>
      </div>
    </div>
  </div>
</template>

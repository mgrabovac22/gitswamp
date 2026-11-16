<script setup lang="ts">
import { computed, ref, useId, watchEffect } from "vue";
import logo from "@/assets/logo_croc.png";
import PetLimb from "./PetLimb.vue";
import { PET_LIMB_ANCHORS, type Limb } from "./petLimbMotion";
import { PET_COLORS, type PetAppearance } from "./petPreferences";
import type { ScheduledTrick } from "./petTricks";

const props = defineProps<{ appearance: PetAppearance; walking?: boolean; outing?: boolean; schedule?: readonly ScheduledTrick[] }>();
const id = useId();
const part = (name: string) => `${id}-${name}`;
const paint = (name: string) => `url(#${part(name)})`;
const limbPaint = (kind: Limb) => ({ kind, paint: paint(`surface-${kind}`), outline: paint(`edge-${kind}`), texture: paint("skin-grain") });
const tint = computed(() => PET_COLORS.find(option => option.id === props.appearance.color)?.value);
const previewMotion = ref(false);
const moving = computed(() => props.walking && (props.outing || previewMotion.value));

watchEffect(onCleanup => {
  if (!props.walking || props.outing) return;
  const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
  const sync = () => { previewMotion.value = !preference.matches && !document.documentElement.classList.contains("reduced-motion"); };
  const observer = new MutationObserver(sync);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
  preference.addEventListener("change", sync);
  sync();
  onCleanup(() => { preference.removeEventListener("change", sync); observer.disconnect(); });
});
</script>

<template>
  <svg viewBox="0 0 260 193" aria-hidden="true" class="pet-sprite"
    :class="{ 'pet-sprite--walking': moving }" :style="{ '--pet-outfit': tint }">
    <defs>
      <image :id="part('logo')" :href="logo" width="805" height="635" />
      <clipPath :id="part('head')">
        <path d="M207 194Q217 124 299 88Q301 37 347 25Q391 16 409 54Q422 16 460 19Q491 24 501 78L523 113Q570 111 611 124Q649 109 666 134Q688 160 672 208Q654 243 618 246Q577 248 532 253Q504 263 481 282Q447 310 389 309Q319 302 266 267Q213 245 207 194Z" />
      </clipPath>
      <clipPath :id="part('chest')">
        <path d="M219 220Q258 225 304 229L459 259Q483 268 495 309L527 444Q544 505 542 563L533 607Q407 639 216 606L198 599Q216 557 224 521Q239 469 236 424L221 398Q224 361 231 338L207 270Z" />
      </clipPath>
      <clipPath :id="part('arm-near')">
        <path d="M0 0H124L131 123Q163 186 196 230Q223 268 263 297Q298 326 316 371Q327 415 296 449Q277 455 247 428Q178 393 126 326Q85 278 56 206L0 158Z" />
      </clipPath>
      <clipPath :id="part('arm-far')">
        <path d="M667 0H805V163L748 187Q700 270 646 320Q594 367 521 408L488 407Q469 373 487 327Q528 308 569 278L603 242Q648 219 662 201L665 130Z" />
      </clipPath>
      <pattern :id="part('skin-grain')" width="15" height="17" patternUnits="userSpaceOnUse">
        <svg width="15" height="17" viewBox="124 235 51 52" preserveAspectRatio="none">
          <use :href="`#${part('logo')}`" />
        </svg>
      </pattern>
      <template v-for="(anchor, limb) in PET_LIMB_ANCHORS" :key="limb">
        <radialGradient :id="part(`surface-${limb}`)" :cx="anchor.x" :cy="anchor.y" r="62" gradientUnits="userSpaceOnUse">
          <stop offset="0" :stop-color="limb.endsWith('near') ? '#817530' : '#71652d'" />
          <stop offset=".29" :stop-color="limb.endsWith('near') ? '#91813d' : '#84753b'" />
          <stop offset=".55" :stop-color="limb.endsWith('near') ? '#a09047' : '#8c7d40'" />
          <stop offset=".81" :stop-color="limb.endsWith('near') ? '#84743a' : '#6c5c31'" />
          <stop offset="1" stop-color="#5e4d28" />
        </radialGradient>
        <radialGradient :id="part(`edge-${limb}`)" :cx="anchor.x" :cy="anchor.y" r="53" gradientUnits="userSpaceOnUse">
          <stop offset=".22" stop-color="#392c19" stop-opacity="0" />
          <stop offset=".42" stop-color="#392c19" stop-opacity=".72" />
          <stop offset=".64" stop-color="#392c19" />
        </radialGradient>
      </template>
      <linearGradient :id="part('chest-fade')" x1="0" y1="533" x2="0" y2="609" gradientUnits="userSpaceOnUse">
        <stop offset="0" stop-color="#fff" />
        <stop offset=".3" stop-color="#fff" />
        <stop offset="1" stop-color="#000" />
      </linearGradient>
      <mask :id="part('chest-join')" x="198" y="220" width="352" height="398" maskUnits="userSpaceOnUse">
        <rect x="198" y="220" width="352" height="398" :fill="paint('chest-fade')" />
      </mask>
      <linearGradient :id="part('olive')" x1=".14" y1="0" x2=".84" y2="1" gradientUnits="objectBoundingBox">
        <stop offset="0" stop-color="#b5a34c" /><stop offset=".27" stop-color="#93863d" /><stop offset=".72" stop-color="#766832" /><stop offset="1" stop-color="#4a3d24" />
      </linearGradient>
      <linearGradient :id="part('belly')" x2=".86" y2=".75" gradientUnits="objectBoundingBox">
        <stop stop-color="#b49b53" /><stop offset=".58" stop-color="#d5bb65" /><stop offset="1" stop-color="#a08448" />
      </linearGradient>
    </defs>

    <g stroke="#392c19" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <g class="pet-tail">
        <path d="M103 119C91 126 76 138 55 142C39 146 24 144 11 136C18 152 37 163 59 163C82 164 100 156 116 143Z" :fill="paint('olive')" />
        <path d="M103 132C84 146 51 158 25 148C43 162 76 162 98 149L113 143Z" fill="#584829" opacity=".63" stroke="none" />
        <path d="M25 142L29 135Q31 132 38 146L46 137Q51 133 56 142L65 132Q70 126 77 137L84 125Q90 118 97 127" fill="#65522b" stroke-width="1.55" />
        <path d="M67 144q2 5 7 6m-18-1q2 4 6 5m23-15q1 5 7 5" fill="none" stroke="#4e3d24" stroke-width="1.5" />
        <path d="M33 145Q66 154 98 131" fill="none" stroke="#d3b85f" stroke-opacity=".55" stroke-width="1.5" />
      </g>
    </g>

    <PetLimb v-bind="limbPaint('leg-far')" :moving="moving" :outing="outing" :schedule="schedule" />

    <g class="pet-upper">
      <PetLimb v-if="moving" v-bind="limbPaint('arm-far')" moving :outing="outing" :schedule="schedule" />
      <g v-else transform="translate(38 5) scale(.222)">
        <use :href="`#${part('logo')}`" :clip-path="paint('arm-far')" />
      </g>
      <g stroke="#392c19" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M98 114C111 105 135 106 147 115C156 124 159 144 151 154Q144 164 124 162C98 166 83 155 82 140C82 129 88 119 98 114Z" :fill="paint('olive')" />
        <path d="M116 131Q132 127 154 135Q156 146 151 154Q140 162 121 157Q113 154 112 143Z" :fill="paint('belly')" />
        <path d="M117 153q16 8 33-1m-40-29q3 5 9 3" fill="none" stroke="#745e32" stroke-width="1.5" />
      </g>
      <g transform="translate(38 5) scale(.222)">
        <use :href="`#${part('logo')}`" :clip-path="paint('chest')" :mask="paint('chest-join')" />
      </g>
    </g>

    <PetLimb v-bind="limbPaint('leg-near')" :moving="moving" :outing="outing" :schedule="schedule" />

    <PetLimb v-if="moving" v-bind="limbPaint('arm-near')" moving :outing="outing" :schedule="schedule" />
    <g v-else transform="translate(38 5) scale(.222)">
      <use :href="`#${part('logo')}`" :clip-path="paint('arm-near')" />
    </g>

    <g class="pet-head">
      <g transform="translate(38 5) scale(.222)">
        <use :href="`#${part('logo')}`" :clip-path="paint('head')" />
      </g>
      <g class="pet-blink" fill="#8d8040" stroke="#4e3c23" stroke-width="1">
        <path d="M113 27Q114 17 122 17Q131 17 132 28Q131 37 123 37Q115 36 113 27Z" />
        <path d="M136 23Q136 15 142 15Q149 15 151 24Q150 31 144 31Q138 31 136 23Z" />
        <path d="M115 29q7 5 15-1m7-3q6 4 12-1" fill="none" />
      </g>
      <g transform="translate(132 15) rotate(-10)" fill="var(--pet-outfit)" stroke="#302517" stroke-width="1.8" stroke-linejoin="round">
        <path v-if="appearance.hat === 'top'" d="M-11 1l1-18h20l1 18zm-7 0h37v4h-37z" />
        <path v-else-if="appearance.hat === 'crown'" d="M-15-10l8 5 7-10 7 10 8-5-4 15h-22z" />
        <path v-else-if="appearance.hat === 'beanie'" d="M-16 2C-16-18 13-22 16 2v4h-32z" />
        <path v-else-if="appearance.hat === 'party'" d="M-13 4L2-16 13 4z M2-16l3-4 1 5z" />
        <path v-else-if="appearance.hat === 'explorer'" d="M-13 1l4-14h16l4 13q9 0 10 4-16 6-42 1 1-4 8-4z" />
        <path v-if="appearance.hat !== 'none'" d="M-8 0h15" stroke="#fff4bd" stroke-opacity=".7" stroke-width="1.2"/>
      </g>
      <g transform="translate(130 77)" fill="var(--pet-outfit)" stroke="#392c19" stroke-width="1.6" stroke-linejoin="round">
        <path v-if="appearance.accessory === 'bandana'" d="M-21-5Q-8 1 15-2L3 18q-10-7-24-23z M-18-3l-9 7 8 3" />
        <path v-else-if="appearance.accessory === 'bow'" d="M-17-5l13 5 14-5v13l-14-5-13 5z M-8-1h7v5h-7z" />
      </g>
    </g>
  </svg>
</template>

<style scoped>
.pet-sprite { display: block; width: 100%; height: auto; overflow: visible; }
.pet-tail, .pet-head { transform-box: view-box; }
.pet-tail { transform-origin: 100px 138px; }
.pet-head { transform-origin: 111px 65px; }
.pet-blink { opacity: 0; }
.pet-sprite--walking .pet-tail { animation: pet-tail 1.12s ease-in-out infinite; }
.pet-sprite--walking .pet-head { animation: pet-head 660ms ease-in-out infinite; }
.pet-sprite--walking .pet-blink { animation: pet-blink 4.7s step-end infinite; }
@keyframes pet-tail { 0%, 100% { transform: rotate(3deg); } 50% { transform: rotate(-8deg); } }
@keyframes pet-head { 0%, 100% { transform: rotate(-2deg) translateY(-1px); } 50% { transform: rotate(2deg) translateY(1px); } }
@keyframes pet-blink { 0%, 91%, 100% { opacity: 0; } 94% { opacity: 1; } 97% { opacity: 0; } }
@media (prefers-reduced-motion: reduce) { .pet-sprite--walking * { animation: none !important; } }
:global(html.reduced-motion) .pet-sprite--walking * { animation: none !important; }
</style>

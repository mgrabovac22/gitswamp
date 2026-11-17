<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import PetSprite from "./PetSprite.vue";
import { floorStridesFor, PET_WALK_SPEED } from "./petLimbMotion";
import { petAppearance } from "./petPreferences";
import { outingTricks, type ScheduledTrick } from "./petTricks";
import { usePetTrickMotion } from "./usePetTrickMotion";

const INTERVAL = 120_000;
const OUTING = 20_000;
const MIN_WIDTH = 112, MAX_WIDTH = 156, WIDTH_RATIO = .16, SIDE_MARGIN = 10;
const strideDistance = {
  "--pet-width": `clamp(${MIN_WIDTH}px, ${WIDTH_RATIO * 100}vw, ${MAX_WIDTH}px)`,
  "--pet-floor-margin": `${SIDE_MARGIN}px`,
  "--pet-wall-approach": PET_WALK_SPEED * 3.2,
  "--pet-walk-distance": PET_WALK_SPEED * 9.6,
};
const visible = ref(false);
const schedule = shallowRef<ScheduledTrick[]>([]);
const floorStrides = ref(0);
const wallShow = computed(() => schedule.value[0]?.grand === true);
const body = ref<HTMLDivElement>();
const shadow = ref<HTMLSpanElement>();
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
let next: ReturnType<typeof setTimeout> | undefined;
let finish: ReturnType<typeof setTimeout> | undefined;
let observer: MutationObserver | undefined;
let allowed = false;
let visitNumber = 0;

usePetTrickMotion(body, shadow, () => schedule.value);

function stop() {
  clearTimeout(next);
  clearTimeout(finish);
  visible.value = false;
}

function visit() {
  schedule.value = petAppearance.value.tricks ? outingTricks(visitNumber++) : [];
  if (!wallShow.value && schedule.value.length) {
    const width = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, window.innerWidth * WIDTH_RATIO));
    floorStrides.value = floorStridesFor(window.innerWidth - width - 2 * SIDE_MARGIN, width);
  } else floorStrides.value = 0;
  visible.value = true;
  finish = setTimeout(() => { visible.value = false; }, OUTING);
  next = setTimeout(visit, INTERVAL);
}

function syncMotion() {
  const shouldRun = !document.hidden && !reducedMotion.matches && !document.documentElement.classList.contains("reduced-motion");
  if (shouldRun === allowed) return;
  allowed = shouldRun;
  if (allowed) visit(); else stop();
}

onMounted(() => {
  document.addEventListener("visibilitychange", syncMotion);
  reducedMotion.addEventListener("change", syncMotion);
  observer = new MutationObserver(syncMotion);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
  syncMotion();
});
onUnmounted(() => {
  stop();
  observer?.disconnect();
  reducedMotion.removeEventListener("change", syncMotion);
  document.removeEventListener("visibilitychange", syncMotion);
});
</script>

<template>
  <div v-if="visible" class="desktop-pet" :style="strideDistance" aria-hidden="true">
    <div v-if="wallShow" class="desktop-pet__rope" data-pet-wall="rope"><i></i><i></i><i></i></div>
    <div v-if="wallShow" class="desktop-pet__rocks" data-pet-wall="rock"><i v-for="hold in 5" :key="hold"></i></div>
    <div class="desktop-pet__walker" :class="{ 'desktop-pet__walker--wall': wallShow, 'desktop-pet__walker--floor': floorStrides }"
      :data-tricks="schedule.map(trick => trick.id).join(' ')">
      <span ref="shadow" class="desktop-pet__shadow"></span>
      <svg v-if="wallShow" class="desktop-pet__impact" viewBox="0 0 120 20" aria-hidden="true">
        <path d="M44 15Q30 5 22 12m16 3L16 15m50 0q17-13 30-6m-17 7 25 0" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      </svg>
      <div class="desktop-pet__facing">
        <div ref="body" class="desktop-pet__tricks"><PetSprite :appearance="petAppearance" walking outing :schedule="schedule" :floor-strides="floorStrides" /></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.desktop-pet {
  --pet-height: calc(var(--pet-width) * 193 / 260);
  --wall-limit: max(30px, calc(100vh - var(--pet-height) - 14px));
  --rope-rise: min(var(--wall-limit), clamp(65px, 24vh, 215px));
  --rock-catch: min(var(--wall-limit), calc(var(--rope-rise) + clamp(20px, 6vh, 49px)));
  --rock-rise: min(var(--wall-limit), calc(var(--rock-catch) + clamp(21px, 5vh, 44px)));
  --jump-top: min(var(--wall-limit), calc(var(--rock-catch) + clamp(25px, 11vh, 100px)));
  --finale-rise: min(calc(50vh - var(--pet-height) / 2), max(48px, calc(100vh - var(--pet-height) * 2.24)));
  --wall-right: calc(100vw - var(--pet-width) * .77 - 9px);
  --wall-left: calc(10px - var(--pet-width) * .24);
  --wall-span: calc(var(--wall-right) - var(--wall-left));
  --wall-middle: calc(50vw - var(--pet-width) / 2);
  position: fixed; inset: 0; pointer-events: none; z-index: 136; overflow: hidden; contain: strict;
}
.desktop-pet__walker {
  --pet-distance: clamp(0px, calc(var(--pet-width) * var(--pet-walk-distance)), calc(100vw - var(--pet-width) - 16px));
  position: absolute;
  bottom: 5px;
  left: max(8px, calc((100vw - var(--pet-width) - var(--pet-distance)) / 2));
  width: var(--pet-width);
  animation: pet-walk-route 20s linear both;
}
.desktop-pet__walker--wall {
  left: 0;
  animation-name: pet-wall-route;
}
.desktop-pet__walker--floor {
  --pet-floor-distance: max(0px, calc(100vw - var(--pet-width) - 2 * var(--pet-floor-margin)));
  left: var(--pet-floor-margin);
  animation-name: pet-floor-route;
}
.desktop-pet__facing { animation: pet-turn 20s ease-in-out both; }
.desktop-pet__walker--wall .desktop-pet__facing { animation-name: pet-wall-facing; }
.desktop-pet__walker--floor .desktop-pet__facing { animation: none; }
.desktop-pet__shadow { position: absolute; width: 44%; height: 6px; bottom: 3%; left: 28%; border-radius: 50%; background: #13100b; opacity: .2; }
.desktop-pet__walker--wall .desktop-pet__shadow { animation: pet-wall-shadow 20s linear both; }
.desktop-pet__impact { position: absolute; width: 92%; height: 20px; bottom: 1px; left: 4%; color: #d3b67c; opacity: 0; animation: pet-wall-impact 20s linear both; }
.desktop-pet__rope {
  position: absolute; bottom: 8px; right: 13px; width: 4px;
  height: min(calc(100vh - 16px), calc(var(--rope-rise) + var(--pet-height) * .9));
  border-radius: 2px; border-left: 1px solid #a48651;
  background: repeating-linear-gradient(138deg, #9d7944 0 2px, #c8a66a 2px 4px);
  box-shadow: 1px 0 #372b1d; opacity: 0;
  animation: pet-rope-entrance 20s linear both;
}
.desktop-pet__rope i { position: absolute; left: -2px; width: 7px; height: 5px; border-radius: 2px; border: 1px solid #a68450; background: #a78951; transform: rotate(-18deg); }
.desktop-pet__rope i:nth-child(1) { bottom: 24%; }
.desktop-pet__rope i:nth-child(2) { bottom: 51%; }
.desktop-pet__rope i:nth-child(3) { bottom: 75%; }
.desktop-pet__rocks {
  position: absolute; bottom: calc(var(--rope-rise) + var(--pet-height) * .28); left: 0; width: 23px;
  height: calc(var(--rock-rise) - var(--rope-rise) + var(--pet-height) * .5);
  opacity: 0; animation: pet-rock-entrance 20s linear both;
}
.desktop-pet__rocks i {
  position: absolute; left: 0; width: 19px; height: 8px;
  background: #957950; border-top: 1px solid #c0a477;
  clip-path: polygon(0 23%, 38% 0, 79% 15%, 100% 58%, 69% 88%, 4% 100%);
}
.desktop-pet__rocks i:nth-child(1) { bottom: 2%; width: 14px; transform: rotate(-8deg); }
.desktop-pet__rocks i:nth-child(2) { bottom: 22%; width: 22px; }
.desktop-pet__rocks i:nth-child(3) { bottom: 53%; width: 17px; transform: rotate(8deg); }
.desktop-pet__rocks i:nth-child(4) { bottom: 68%; width: 12px; transform: rotate(-8deg); }
.desktop-pet__rocks i:nth-child(5) { bottom: 91%; width: 19px; }
@keyframes pet-walk-route {
  0%, 100% { transform: translateX(0); opacity: 0; }
  3%, 97% { opacity: 1; }
  49%, 53% { transform: translateX(var(--pet-distance)); }
}
/* The pauses at 3.65s, 9.1s and 14.55s follow the floor trick schedule. */
@keyframes pet-floor-route {
  0% { transform: translateX(0); opacity: 0; }
  3%, 97% { opacity: 1; }
  18.25%, 27.25% { transform: translateX(calc(var(--pet-floor-distance) * .25)); }
  45.5%, 54.5% { transform: translateX(calc(var(--pet-floor-distance) * .5)); }
  72.75%, 81.75% { transform: translateX(calc(var(--pet-floor-distance) * .75)); }
  100% { transform: translateX(var(--pet-floor-distance)); opacity: 0; }
}
/* Ground approach, rope, airborne crossing, rock, then the centre. */
@keyframes pet-wall-route {
  0% { transform: translate(calc(var(--wall-right) - var(--pet-width) * var(--pet-wall-approach)), 0px); opacity: 0; }
  3%, 97% { opacity: 1; }
  16% { transform: translate(var(--wall-right), 0px); animation-timing-function: ease-out; }
  18.1%, 19.2% { transform: translate(calc(var(--wall-right) - 4px), calc(var(--rope-rise) * -.19)); }
  21.5%, 22.8% { transform: translate(var(--wall-right), calc(var(--rope-rise) * -.36)); animation-timing-function: ease-in-out; }
  25.35% { transform: translate(calc(var(--wall-right) - 7px), calc(var(--rope-rise) * -.61)); }
  28.15% { transform: translate(var(--wall-right), calc(var(--rope-rise) * -.76)); }
  30.2%, 31.9% { transform: translate(calc(var(--wall-right) - 4px), calc(var(--rope-rise) * -.83)); animation-timing-function: ease-in-out; }
  34%, 35.2% { transform: translate(var(--wall-right), calc(-1 * var(--rope-rise))); }
  36.5% { transform: translate(calc(var(--wall-right) - 18px), calc(-1 * var(--rope-rise) + 4px)); animation-timing-function: cubic-bezier(.24,.5,.63,1); }
  41% { transform: translate(calc(var(--wall-left) + var(--wall-span) * .58), calc(-1 * var(--jump-top))); animation-timing-function: cubic-bezier(.32,0,.78,.69); }
  46.2% { transform: translate(calc(var(--wall-left) + var(--wall-span) * .15), calc(var(--jump-top) * -.88 - var(--rock-catch) * .12)); }
  47.75% { transform: translate(var(--wall-left), calc(-1 * var(--rock-catch))); animation-timing-function: ease-out; }
  49.25% { transform: translate(calc(var(--wall-left) + 5px), calc(-1 * var(--rock-catch) + 6px)); }
  51.7%, 55.8% { transform: translate(var(--wall-left), calc(var(--rock-rise) * -.64 + var(--rock-catch) * -.36)); }
  57.1% { transform: translate(calc(var(--wall-left) + 9px), calc(var(--rock-rise) * -.46 + var(--rock-catch) * -.54)); animation-timing-function: ease-out; }
  59.4%, 61.6% { transform: translate(var(--wall-left), calc(-1 * var(--rock-rise))); }
  63.6% { transform: translate(calc(var(--wall-left) + 22px), calc(-1 * var(--rock-rise) + 4px)); animation-timing-function: cubic-bezier(.19,.57,.7,1); }
  69.4% { transform: translate(calc((var(--wall-middle) + var(--wall-left)) * .5), calc(var(--rock-rise) * -.61 + var(--finale-rise) * -.39 - 13px)); }
  77%, 91% { transform: translate(var(--wall-middle), calc(-1 * var(--finale-rise))); }
  92% { transform: translate(var(--wall-middle), calc(-1 * var(--finale-rise) - 3px)); animation-timing-function: cubic-bezier(.6,.04,.83,.6); }
  96.55%, 100% { transform: translate(var(--wall-middle), 0px); }
  100% { opacity: 0; }
}
@keyframes pet-turn {
  0%, 49% { transform: scaleX(1); }
  50.6% { transform: scaleX(.22) skewY(-4deg); }
  51.3% { transform: scaleX(-.22) skewY(4deg); }
  53%, 100% { transform: scaleX(-1); }
}
@keyframes pet-wall-facing {
  0%, 16% { transform: scaleX(1) rotate(0); }
  19%, 24% { transform: scaleX(1) rotate(-9deg); }
  27%, 31% { transform: scaleX(1) rotate(-3deg); }
  35.2% { transform: scaleX(1) rotate(-11deg); }
  36.5% { transform: scaleX(1) rotate(-29deg); }
  39% { transform: scaleX(.64) rotate(-23deg); }
  41% { transform: scaleX(-.67) rotate(21deg); }
  46.2%, 48.6% { transform: scaleX(-1) rotate(12deg); }
  52%, 55% { transform: scaleX(-1) rotate(6deg); }
  59%, 61.6% { transform: scaleX(-1) rotate(12deg); }
  63.6% { transform: scaleX(-1) rotate(22deg); }
  67% { transform: scaleX(-.34) rotate(12deg); }
  69.5% { transform: scaleX(.73) rotate(-8deg); }
  77%, 91% { transform: scaleX(1) rotate(0); }
  94.4% { transform: scaleX(1) rotate(9deg); }
  96.55% { transform: scale(1.08, .84) rotate(-4deg); }
  98%, 100% { transform: scaleX(1) rotate(0); }
}
@keyframes pet-rope-entrance { 0%, 9%, 42%, 100% { opacity: 0; } 14%, 36% { opacity: .52; } }
@keyframes pet-rock-entrance { 0%, 40%, 69%, 100% { opacity: 0; } 46%, 63% { opacity: .55; } }
@keyframes pet-wall-shadow { 0%, 16% { opacity: .2; } 19%, 94% { opacity: 0; } 96.55% { opacity: .28; } 98%, 100% { opacity: .16; } }
@keyframes pet-wall-impact { 0%, 96.4% { opacity: 0; transform: scale(.35, .6); } 97.2% { opacity: .58; } 100% { opacity: 0; transform: scale(1.3, 1); } }
:global(html.reduced-motion) .desktop-pet { display: none; }
@media (prefers-reduced-motion: reduce) { .desktop-pet { display: none; } }
</style>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { getStoredAppPalettePreference, type AppPalettePreference } from "@/shared/themePreferences";

interface AmbienceConfig {
  glyphs: string[];
  count: number;
  variant?: "float" | "snow" | "bloom" | "bounce";
}

interface AmbienceGlyph {
  id: number;
  glyph: string;
  left: number;
  top: number;
  size: number;
  delay: number;
  duration: number;
  drift: number;
  rotate: number;
}

const AMBIENCE_CONFIGS: Partial<Record<AppPalettePreference, AmbienceConfig>> = {
  "kitty": { glyphs: ["🎀", "💗", "🐱", "🌸", "♡"], count: 14 },
  "kawaii-pastel": { glyphs: ["⭐", "💖", "🌈", "☁️", "🧸"], count: 14 },
  "anime-sakura": { glyphs: ["🌸", "✨", "⚡", "🌙", "✧"], count: 16 },
  "pixel-arcade": { glyphs: ["👾", "🕹️", "💠", "⭐", "🎮"], count: 14 },
  "starry-night": { glyphs: ["⭐", "✦", "🌙", "✧", "💫"], count: 16 },
  snowy: { glyphs: ["❄", "❅", "❆", "•", "✧"], count: 64, variant: "snow" },
  "lux-special": { glyphs: ["🌸", "🌷", "🌼", "💠", "🦋", "✦"], count: 22, variant: "bloom" },
  "all-star-ogre": { glyphs: ["🧅", "🐸", "🫏", "🍪", "🌿", "💚"], count: 18, variant: "float" },
  "toon-town": { glyphs: ["⭐", "🌈", "🎈", "🎨", "☀️", "💫"], count: 18, variant: "bounce" },
  "comic-boom": { glyphs: ["💥", "⚡", "💫", "❗", "✴️", "💢"], count: 18, variant: "bounce" },
  "rubber-hose": { glyphs: ["🎩", "🥕", "🎬", "🎞️", "⭐", "☁️"], count: 14, variant: "float" },
  "cartoon-carnival": { glyphs: ["🎪", "🎠", "🍭", "🤹", "🎟️", "🎈"], count: 18, variant: "bounce" },
};

function readActiveTheme(): AppPalettePreference {
  const value = document.documentElement.dataset.appTheme;
  if (value) return value as AppPalettePreference;
  return getStoredAppPalettePreference();
}

const activeTheme = ref<AppPalettePreference>(readActiveTheme());
let observer: MutationObserver | null = null;

function syncTheme() {
  activeTheme.value = readActiveTheme();
}

const config = computed(() => AMBIENCE_CONFIGS[activeTheme.value] ?? null);

function pseudoRandom(seed: number): number {
  const value = Math.sin(seed * 12.9898) * 43758.5453;
  return value - Math.floor(value);
}

const glyphs = computed<AmbienceGlyph[]>(() => {
  const current = config.value;
  if (!current) return [];

  const variant = current.variant ?? "float";

  return Array.from({ length: current.count }, (_, index) => {
    const randomA = pseudoRandom(index + 1);
    const randomB = pseudoRandom(index + 101);
    const randomC = pseudoRandom(index + 211);

    if (variant === "snow") {
      return {
        id: index,
        glyph: current.glyphs[index % current.glyphs.length],
        left: Math.round(randomA * 100 * 10) / 10,
        top: 0,
        size: 8 + Math.round(randomC * 16),
        delay: -Math.round(randomC * 14 * 10) / 10,
        duration: 7 + Math.round((1 - randomB) * 12 * 10) / 10,
        drift: Math.round((randomA - 0.5) * 90),
        rotate: Math.round((randomB - 0.5) * 320),
      };
    }

    const edgeBand = index % 3;

    let left = randomA * 94 + 3;
    let top = randomB * 88 + 4;

    if (edgeBand === 0) {
      left = randomA < 0.5 ? randomA * 16 + 1 : 83 + randomA * 16;
      top = randomB * 90 + 5;
    } else if (edgeBand === 1) {
      left = randomA * 90 + 5;
      top = randomB < 0.5 ? randomB * 14 + 1 : 85 + randomB * 14;
    }

    return {
      id: index,
      glyph: current.glyphs[index % current.glyphs.length],
      left: Math.round(left * 10) / 10,
      top: Math.round(top * 10) / 10,
      size: 13 + Math.round(randomC * 15),
      delay: Math.round(randomC * 9 * 10) / 10,
      duration: 7 + Math.round((1 - randomC) * 9 * 10) / 10,
      drift: Math.round((randomB - 0.5) * 70),
      rotate: Math.round((randomA - 0.5) * 90),
    };
  });
});

onMounted(() => {
  syncTheme();
  observer = new MutationObserver(syncTheme);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-app-theme"],
  });
});

onUnmounted(() => {
  observer?.disconnect();
  observer = null;
});
</script>

<template>
  <div v-if="config" class="theme-ambience" aria-hidden="true">
    <span
      v-for="glyph in glyphs"
      :key="glyph.id"
      :class="[
        'theme-ambience__glyph',
        config?.variant === 'snow' ? 'theme-ambience__glyph--snow' : '',
        config?.variant === 'bloom' ? 'theme-ambience__glyph--bloom' : '',
        config?.variant === 'bounce' ? 'theme-ambience__glyph--bounce' : '',
      ]"
      :style="{
        left: `${glyph.left}%`,
        top: `${glyph.top}%`,
        fontSize: `${glyph.size}px`,
        animationDelay: `${glyph.delay}s`,
        animationDuration: `${glyph.duration}s`,
        '--theme-drift': `${glyph.drift}px`,
        '--theme-rotate': `${glyph.rotate}deg`,
      }"
    >
      {{ glyph.glyph }}
    </span>

    <div v-if="activeTheme === 'snowy'" class="theme-ambience__santa">
      <svg class="theme-ambience__santa-svg" viewBox="0 0 200 100" role="presentation">
        <path d="M104 62 C120 50 134 50 150 56" fill="none" stroke="#7c4a1e" stroke-width="2.5" stroke-linecap="round" />
        <g fill="#6b4423">
          <rect x="122" y="68" width="5" height="22" rx="2" transform="rotate(6 124 68)" />
          <rect x="140" y="68" width="5" height="22" rx="2" transform="rotate(-6 142 68)" />
          <rect x="154" y="68" width="5" height="22" rx="2" transform="rotate(8 156 68)" />
          <rect x="166" y="68" width="5" height="22" rx="2" transform="rotate(-8 168 68)" />
        </g>
        <ellipse cx="146" cy="56" rx="27" ry="13" fill="#8a5a2b" />
        <path d="M160 52 L170 30 L178 32 L168 58 Z" fill="#8a5a2b" />
        <circle cx="176" cy="29" r="9.5" fill="#8a5a2b" />
        <ellipse cx="169" cy="22" rx="3.5" ry="6" fill="#6b4423" transform="rotate(-25 169 22)" />
        <path d="M176 20 L170 8 M176 20 L178 5 M176 20 L184 9 M170 12 L165 9 M178 9 L183 4" fill="none" stroke="#6b4423" stroke-width="2.2" stroke-linecap="round" />
        <circle cx="186" cy="31" r="3.4" fill="#ef4444" />
        <circle cx="179" cy="26" r="1.4" fill="#111827" />
        <path d="M120 50 q-6 -4 -9 2" fill="none" stroke="#6b4423" stroke-width="3" stroke-linecap="round" />
        <path d="M12 84 Q6 96 24 96 L104 96" fill="none" stroke="#fbbf24" stroke-width="4" stroke-linecap="round" />
        <path d="M14 62 Q10 46 30 44 L92 44 Q106 44 106 60 L102 78 Q100 86 90 86 L30 86 Q18 86 14 74 Z" fill="#c81e1e" />
        <path d="M18 54 L100 54" stroke="#fbbf24" stroke-width="3" stroke-linecap="round" />
        <path d="M92 44 Q106 44 106 60" fill="none" stroke="#fbbf24" stroke-width="3" stroke-linecap="round" />
        <ellipse cx="34" cy="46" rx="13" ry="10" fill="#92400e" />
        <path d="M28 37 q6 -5 12 0" fill="none" stroke="#78350f" stroke-width="2.5" stroke-linecap="round" />
        <path d="M50 64 q0 -14 14 -14 q14 0 14 14 l-2 14 q-12 6 -24 0 Z" fill="#dc2626" />
        <rect x="49" y="66" width="30" height="5" rx="2" fill="#111827" />
        <rect x="61" y="64" width="6" height="9" rx="1.5" fill="#fbbf24" />
        <path d="M76 54 q10 4 14 12" fill="none" stroke="#dc2626" stroke-width="7" stroke-linecap="round" />
        <circle cx="91" cy="68" r="4" fill="#fca5a5" />
        <circle cx="64" cy="42" r="9" fill="#fca5a5" />
        <path d="M55 42 q0 16 9 18 q9 -2 9 -18 q-4 5 -9 5 q-5 0 -9 -5 Z" fill="#f8fafc" />
        <path d="M59 41 q5 4 10 0" fill="none" stroke="#f8fafc" stroke-width="3" stroke-linecap="round" />
        <path d="M55 36 q9 -14 20 -2 l1 5 q-11 -4 -21 2 Z" fill="#dc2626" />
        <path d="M53 37 q11 -4 23 1 l0 3 q-12 -4 -24 0 Z" fill="#f8fafc" />
        <circle cx="78" cy="30" r="3.5" fill="#f8fafc" />
        <circle cx="67" cy="41" r="1.1" fill="#111827" />
      </svg>
    </div>
  </div>
</template>

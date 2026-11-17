<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import type { SpecialThemeScene } from "@/shared/themePreferences";

const { scene } = defineProps<{ scene: SpecialThemeScene }>();
const paused = ref(document.hidden);

function syncVisibility() {
  paused.value = document.hidden;
}

// Scene parameters are deterministic and computed once; CSS handles all motion.
const particles = Array.from({ length: scene.particles }, (_, index) => {
  const x = (index * 71 + 19) % 101 / 100;
  const y = (index * 37 + 11) % 101 / 100;
  const duration = scene.period * (0.82 + ((index * 43) % 11) / 25);
  return {
    "--x": `${(index % 2 === 0 ? x * 16 : 84 + x * 15)}%`,
    "--y": `${14 + y * 77}%`,
    "--lane": `${68 + y * 28}%`,
    "--size": `${8 + (index * 7) % 15}px`,
    "--sway": `${index % 2 === 0 ? 46 : -46}px`,
    animationDelay: `${-(index + 0.32) * duration / scene.particles}s`,
    animationDuration: `${duration}s`,
  };
});

onMounted(() => document.addEventListener("visibilitychange", syncVisibility));
onUnmounted(() => document.removeEventListener("visibilitychange", syncVisibility));
</script>

<template>
  <div class="theme-scene" :data-scene="scene.id" :class="{ 'theme-scene--paused': paused }" aria-hidden="true">
    <span v-for="(particle, index) in particles" :key="index" class="theme-scene__particle" :style="particle" />
    <div v-if="scene.id === 'meteor-watch' || scene.id === 'firework-studio'" class="theme-scene__flyby"><i></i></div>
  </div>
</template>

<style src="./specialThemeMotion.css"></style>

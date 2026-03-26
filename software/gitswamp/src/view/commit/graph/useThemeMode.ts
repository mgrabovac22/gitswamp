import { computed, onMounted, onUnmounted, ref } from "vue";

export function useThemeMode() {
  const isLight = ref(document.documentElement.classList.contains("light"));
  const themeObserver = new MutationObserver(() => {
    isLight.value = document.documentElement.classList.contains("light");
  });

  onMounted(() => {
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
  });

  onUnmounted(() => {
    themeObserver.disconnect();
  });

  const svgBgOuter = computed(() => (isLight.value ? "#bcc6d4" : "#111520"));
  const svgBgInner = computed(() => (isLight.value ? "#cdd5e0" : "#1a1f30"));

  return { isLight, svgBgOuter, svgBgInner };
}

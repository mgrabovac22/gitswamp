import { onBeforeUnmount, onMounted } from "vue";

export function usePreviewEscape(close: () => void) {
  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape") close();
  }
  onMounted(() => globalThis.addEventListener("keydown", onKey, true));
  onBeforeUnmount(() => globalThis.removeEventListener("keydown", onKey, true));
}

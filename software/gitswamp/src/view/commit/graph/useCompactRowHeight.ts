import { onMounted, onUnmounted, ref } from "vue";

import { ROW_HEIGHT_COMPACT, ROW_HEIGHT_NORMAL } from "./graph.constants";

export function useCompactRowHeight() {
  const rowHeight = ref(ROW_HEIGHT_NORMAL);
  let compactObserver: MutationObserver | null = null;

  function updateRowHeight() {
    rowHeight.value = document.documentElement.classList.contains("compact")
      ? ROW_HEIGHT_COMPACT
      : ROW_HEIGHT_NORMAL;
  }

  onMounted(() => {
    updateRowHeight();
    compactObserver = new MutationObserver(() => {
      updateRowHeight();
    });
    compactObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
  });

  onUnmounted(() => {
    compactObserver?.disconnect();
  });

  return { rowHeight };
}

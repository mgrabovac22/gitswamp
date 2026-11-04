import { computed, onScopeDispose, ref, watch, type Ref } from "vue";

export function useWindowedRows<T>(items: () => T[], container: Ref<HTMLElement | null>, rowHeight: () => number) {
  const top = ref(0);
  const height = ref(600);
  const update = () => {
    top.value = container.value?.scrollTop ?? 0;
    height.value = container.value?.clientHeight || 600;
  };
  const stop = watch(container, (element, _previous, onCleanup) => {
    if (!element) return;
    element.addEventListener("scroll", update, { passive: true });
    const resize = new ResizeObserver(update);
    resize.observe(element);
    update();
    onCleanup(() => { element.removeEventListener("scroll", update); resize.disconnect(); });
  }, { flush: "post" });
  onScopeDispose(stop);
  const window = computed(() => {
    const rows = items();
    if (rows.length <= 200) return { rows, before: 0, after: 0, active: false };
    const size = rowHeight();
    const start = Math.min(Math.max(0, rows.length - 1), Math.max(0, Math.floor((top.value - 48) / size) - 10));
    const end = Math.min(rows.length, start + Math.ceil(height.value / size) + 22);
    return { rows: rows.slice(start, end), before: start * size, after: (rows.length - end) * size, active: true };
  });
  return window;
}

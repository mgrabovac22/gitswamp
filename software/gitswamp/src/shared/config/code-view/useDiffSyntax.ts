import { onScopeDispose, ref, watch } from "vue";
import { createSyntaxQueue } from "./syntaxQueue";

export function useDiffSyntax(path: () => string) {
  const revision = ref(0);
  const queue = createSyntaxQueue(
    () => new Worker(new URL("./syntax.worker.ts", import.meta.url), { type: "module" }),
    () => { revision.value += 1; },
  );
  watch(path, value => queue.reset(value), { immediate: true, flush: "sync" });
  onScopeDispose(queue.dispose);
  return { get(line: string) { void revision.value; return queue.get(line); } };
}

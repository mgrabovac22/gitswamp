type SyntaxWorker = Pick<Worker, "postMessage" | "terminate" | "onmessage" | "onerror">;

export function createSyntaxQueue(createWorker: () => SyntaxWorker, changed: () => void) {
  let worker: SyntaxWorker | undefined;
  let generation = 0;
  let path = "";
  let scheduled: ReturnType<typeof setTimeout> | undefined;
  let notification: ReturnType<typeof setTimeout> | undefined;
  let inFlight = false;
  let disabled = false;
  const cache = new Map<string, string>();
  const pending = new Set<string>();
  const requested = new Set<string>();

  function flush() {
    scheduled = undefined;
    if (inFlight || disabled || pending.size === 0) return;
    try {
      if (!worker) {
        const workerGeneration = generation;
        worker = createWorker();
        worker.onmessage = event => {
          if (event.data.generation !== generation) return;
          inFlight = false;
          for (const [line, html] of event.data.results as [string, string][]) cache.set(line, html);
          if (notification === undefined) notification = setTimeout(() => {
            notification = undefined;
            changed();
          }, 32);
          schedule();
        };
        worker.onerror = () => {
          if (workerGeneration !== generation) return;
          disabled = true;
          pending.clear(); requested.clear();
          worker?.terminate(); worker = undefined;
        };
      }
      const lines = [...pending].slice(0, 64);
      for (const line of lines) pending.delete(line);
      inFlight = true;
      worker.postMessage({ generation, path, lines });
    } catch {
      disabled = true;
      pending.clear();
    }
  }
  function schedule() {
    if (scheduled === undefined && !inFlight && pending.size) scheduled = setTimeout(flush, 0);
  }
  function get(line: string): string | undefined {
    const html = cache.get(line);
    if (html !== undefined || disabled) return html;
    if (!requested.has(line) && requested.size < 6000) {
      requested.add(line);
      pending.add(line);
      schedule();
    }
    return undefined;
  }
  function reset(nextPath: string) {
    generation += 1;
    path = nextPath;
    cache.clear(); pending.clear(); requested.clear();
    worker?.terminate(); worker = undefined;
    inFlight = false;
    clearTimeout(scheduled); scheduled = undefined;
    clearTimeout(notification); notification = undefined;
  }
  function dispose() { reset(""); disabled = true; }
  return { get, reset, dispose };
}

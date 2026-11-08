import type { useToast } from "./useToast";
export const UNDOABLE_DESTRUCTIVE_DELAY_MS = 5000;
export const DESTRUCTIVE_ACTION_UNDONE_EVENT = "gitswamp-destructive-action-undone";

export interface DestructiveAction {
  message: string;
  detail?: string;
  undoMessage?: string;
  run: () => unknown | Promise<unknown>;
  onPending?: (undo: () => void) => (() => void) | void;
}

export function createDestructiveScheduler(
  context: () => string,
  toast: ReturnType<typeof useToast>,
  undoEnabled: () => boolean,
  canExecute: () => boolean = () => true,
) {
  const pending = new Set<() => void>();

  function scheduleDestructiveAction(options: DestructiveAction) {
    const originalContext = context();
    let settled = false;
    let toastId: number | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let dismissPending: (() => void) | void;
    const cancel = () => {
      if (settled) return;
      settled = true;
      if (timer !== undefined) clearTimeout(timer);
      if (toastId !== undefined) toast.remove(toastId);
      pending.delete(cancel);
      dismissPending?.();
    };
    const execute = () => {
      if (settled) return;
      if (originalContext !== context() || !canExecute()) {
        cancel();
        toast.info("Repository state changed or another operation is running. Action cancelled.");
        return;
      }
      cancel();
      try {
        Promise.resolve(options.run()).catch(error => toast.error(error instanceof Error ? error.message : String(error)));
      } catch (error) {
        toast.error(error instanceof Error ? error.message : String(error));
      }
    };
    if (!undoEnabled()) { execute(); return; }
    const undo = () => {
      if (settled) return;
      cancel();
      globalThis.dispatchEvent?.(new Event(DESTRUCTIVE_ACTION_UNDONE_EVENT));
      toast.info(options.undoMessage ?? "Action cancelled.", 2500);
    };
    pending.add(cancel);
    timer = setTimeout(execute, UNDOABLE_DESTRUCTIVE_DELAY_MS);
    toastId = toast.action("warning", options.message, [{
      label: "Undo", style: "neutral", onClick: undo,
    }], UNDOABLE_DESTRUCTIVE_DELAY_MS, options.detail ?? "Runs in 5 seconds unless you undo.");
    dismissPending = options.onPending?.(undo);
  }

  function cancelPendingActions() {
    for (const cancel of [...pending]) cancel();
  }
  return { scheduleDestructiveAction, cancelPendingActions };
}

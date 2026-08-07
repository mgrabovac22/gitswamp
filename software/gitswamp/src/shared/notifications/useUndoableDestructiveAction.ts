import { onUnmounted } from "vue";
import { getStoredDestructiveUndoEnabled } from "@/shared/config/undoPreferences";
import { useToast } from "@/shared/notifications/useToast";

export const UNDOABLE_DESTRUCTIVE_DELAY_MS = 5000;

interface UndoableDestructiveActionOptions {
  message: string;
  detail?: string;
  undoMessage?: string;
  run: () => void | Promise<void>;
}

export function useUndoableDestructiveAction() {
  const toast = useToast();
  const timers = new Set<ReturnType<typeof setTimeout>>();

  function scheduleDestructiveAction(options: UndoableDestructiveActionOptions) {
    if (!getStoredDestructiveUndoEnabled()) {
      Promise.resolve(options.run()).catch((error) => {
        toast.error(error instanceof Error ? error.message : String(error));
      });
      return;
    }

    let cancelled = false;
    const timer = setTimeout(() => {
      timers.delete(timer);
      if (cancelled) {
        return;
      }

      Promise.resolve(options.run()).catch((error) => {
        toast.error(error instanceof Error ? error.message : String(error));
      });
    }, UNDOABLE_DESTRUCTIVE_DELAY_MS);

    timers.add(timer);

    toast.action(
      "warning",
      options.message,
      [
        {
          label: "Undo",
          style: "neutral",
          onClick: () => {
            cancelled = true;
            clearTimeout(timer);
            timers.delete(timer);
            toast.info(options.undoMessage ?? "Action cancelled.", 2500);
          },
        },
      ],
      UNDOABLE_DESTRUCTIVE_DELAY_MS,
      options.detail ?? "Runs in 5 seconds unless you undo.",
    );
  }

  onUnmounted(() => {
    for (const timer of timers) {
      clearTimeout(timer);
    }
    timers.clear();
  });

  return { scheduleDestructiveAction };
}

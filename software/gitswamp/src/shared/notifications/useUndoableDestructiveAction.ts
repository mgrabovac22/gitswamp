import { onUnmounted, watch } from "vue";
import { getStoredDestructiveUndoEnabled } from "@/shared/config/undoPreferences";
import { useToast } from "@/shared/notifications/useToast";
import { createDestructiveScheduler } from "./destructiveScheduler";

export { UNDOABLE_DESTRUCTIVE_DELAY_MS } from "./destructiveScheduler";

export function useUndoableDestructiveAction(context: () => string = () => "", canExecute: () => boolean = () => true) {
  const scheduler = createDestructiveScheduler(context, useToast(), getStoredDestructiveUndoEnabled, canExecute);
  watch(context, scheduler.cancelPendingActions, { flush: "sync" });
  onUnmounted(scheduler.cancelPendingActions);
  return scheduler;
}

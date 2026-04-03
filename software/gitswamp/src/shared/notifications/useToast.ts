import { ref } from "vue";

export type ToastType = "success" | "error" | "info" | "warning" | "loading" | "progress";

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
  duration?: number;
  actions?: ToastAction[];
  progress?: number;
  detail?: string;
}

export interface ToastAction {
  label: string;
  style?: "primary" | "danger" | "neutral" | "success" | "warning";
  onClick: () => void | Promise<void>;
}

const toasts = ref<Toast[]>([]);
let nextId = 1;

function clampProgress(value: number): number {
  return Math.max(0, Math.min(100, value));
}

function addToast(
  type: Toast["type"],
  message: string,
  duration = 7000,
  actions?: ToastAction[],
  meta?: Pick<Partial<Toast>, "progress" | "detail">
) {
  const id = nextId++;
  toasts.value.push({
    id,
    type,
    message,
    duration,
    actions,
    progress: meta?.progress,
    detail: meta?.detail,
  });
  
  if (duration > 0) {
    setTimeout(() => {
      removeToast(id);
    }, duration);
  }
  
  return id;
}

function removeToast(id: number) {
  const index = toasts.value.findIndex(t => t.id === id);
  if (index !== -1) {
    toasts.value.splice(index, 1);
  }
}

function success(message: string, duration?: number) {
  return addToast("success", message, duration);
}

function error(message: string, duration?: number) {
  return addToast("error", message, duration ?? 9000);
}

function info(message: string, duration?: number) {
  return addToast("info", message, duration);
}

function warning(message: string, duration?: number) {
  return addToast("warning", message, duration ?? 8000);
}

function action(type: Toast["type"], message: string, actions: ToastAction[], duration = 15000) {
  return addToast(type, message, duration, actions);
}

function loading(message: string) {
  return addToast("loading", message, 0);
}

function progress(message: string, value = 0, detail?: string) {
  return addToast("progress", message, 0, undefined, {
    progress: clampProgress(value),
    detail,
  });
}

function update(id: number, updates: Partial<Omit<Toast, "id">>): boolean {
  const index = toasts.value.findIndex((toast) => toast.id === id);
  if (index === -1) {
    return false;
  }

  const merged: Toast = {
    ...toasts.value[index],
    ...updates,
  };

  if (typeof merged.progress === "number") {
    merged.progress = clampProgress(merged.progress);
  }

  toasts.value[index] = merged;
  return true;
}

export function useToast() {
  return {
    toasts,
    success,
    error,
    info,
    warning,
    loading,
    progress,
    update,
    action,
    remove: removeToast,
  };
}

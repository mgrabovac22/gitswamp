import { ref } from "vue";

export interface Toast {
  id: number;
  type: "success" | "error" | "info" | "warning" | "loading";
  message: string;
  duration?: number;
  actions?: ToastAction[];
}

export interface ToastAction {
  label: string;
  style?: "primary" | "danger" | "neutral" | "success" | "warning";
  onClick: () => void | Promise<void>;
}

const toasts = ref<Toast[]>([]);
let nextId = 1;

function addToast(type: Toast["type"], message: string, duration = 7000, actions?: ToastAction[]) {
  const id = nextId++;
  toasts.value.push({ id, type, message, duration, actions });
  
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

export function useToast() {
  return {
    toasts,
    success,
    error,
    info,
    warning,
    loading,
    action,
    remove: removeToast,
  };
}

<script setup lang="ts">
import { useToast, type Toast, type ToastAction } from "@/shared/notifications/useToast";
import { CheckCircle, XCircle, Info, AlertTriangle, Loader2, X } from "lucide-vue-next";
import logoCrocLoading from "@/assets/logo_croc_loading.gif";

const { toasts, remove } = useToast();
const loadingLetters = ["L", "o", "a", "d", "i", "n", "g"];

const icons = {
  success: CheckCircle,
  error: XCircle,
  info: Info,
  warning: AlertTriangle,
  loading: Info,
  progress: Loader2,
};

const colors = {
  success: {
    backgroundColor: "color-mix(in srgb, var(--card) 90%, var(--chart-5) 10%)",
    borderColor: "color-mix(in srgb, var(--chart-5) 55%, transparent)",
    textColor: "var(--card-foreground)",
    icon: "var(--chart-5)",
  },
  error: {
    backgroundColor: "color-mix(in srgb, var(--card) 88%, var(--destructive) 12%)",
    borderColor: "color-mix(in srgb, var(--destructive) 60%, transparent)",
    textColor: "var(--card-foreground)",
    icon: "var(--destructive)",
  },
  info: {
    backgroundColor: "color-mix(in srgb, var(--card) 90%, var(--chart-2) 10%)",
    borderColor: "color-mix(in srgb, var(--chart-2) 55%, transparent)",
    textColor: "var(--card-foreground)",
    icon: "var(--chart-2)",
  },
  warning: {
    backgroundColor: "color-mix(in srgb, var(--card) 88%, var(--chart-3) 12%)",
    borderColor: "color-mix(in srgb, var(--chart-3) 60%, transparent)",
    textColor: "var(--card-foreground)",
    icon: "var(--chart-3)",
  },
  loading: {
    backgroundColor: "color-mix(in srgb, var(--card) 90%, var(--chart-2) 10%)",
    borderColor: "color-mix(in srgb, var(--chart-2) 55%, transparent)",
    textColor: "var(--card-foreground)",
    icon: "var(--chart-2)",
  },
  progress: {
    backgroundColor: "color-mix(in srgb, var(--card) 90%, var(--primary) 10%)",
    borderColor: "color-mix(in srgb, var(--primary) 55%, transparent)",
    textColor: "var(--card-foreground)",
    icon: "var(--primary)",
  },
};

function toastStyle(type: Toast["type"]): Record<string, string> {
  const tone = colors[type];
  return {
    backgroundColor: tone.backgroundColor,
    borderColor: tone.borderColor,
    color: tone.textColor,
  };
}

function actionButtonStyle(style?: ToastAction["style"]): Record<string, string> {
  switch (style) {
    case "danger":
      return {
        backgroundColor: "var(--destructive)",
        color: "var(--destructive-foreground)",
        borderColor: "color-mix(in srgb, var(--destructive) 65%, transparent)",
      };
    case "success":
      return {
        backgroundColor: "var(--chart-5)",
        color: "var(--primary-foreground)",
        borderColor: "color-mix(in srgb, var(--chart-5) 65%, transparent)",
      };
    case "warning":
      return {
        backgroundColor: "color-mix(in srgb, var(--chart-3) 85%, var(--card) 15%)",
        color: "var(--foreground)",
        borderColor: "color-mix(in srgb, var(--chart-3) 60%, transparent)",
      };
    case "primary":
      return {
        backgroundColor: "var(--primary)",
        color: "var(--primary-foreground)",
        borderColor: "color-mix(in srgb, var(--primary) 65%, transparent)",
      };
    default:
      return {
        backgroundColor: "var(--secondary)",
        color: "var(--secondary-foreground)",
        borderColor: "var(--border)",
      };
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="fixed bottom-4 right-4 z-[2147483647] flex flex-col gap-2 max-w-sm">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="flex items-start gap-3 px-4 py-3 rounded-lg border shadow-xl backdrop-blur-md"
          :style="toastStyle(toast.type)"
        >
          <div v-if="toast.type === 'loading'" class="flex items-center gap-2.5 flex-shrink-0 mt-0.5">
            <img :src="logoCrocLoading" alt="Loading" class="toast-loader-logo" />
            <div class="toast-loader-wave" aria-label="Loading">
              <span
                v-for="(letter, idx) in loadingLetters"
                :key="toast.id + '-loading-' + idx"
                class="toast-loader-letter"
                :style="{ animationDelay: `${idx * 0.06}s` }"
              >
                {{ letter }}
              </span>
            </div>
          </div>
          <component
            v-else
            :is="icons[toast.type]"
            class="w-5 h-5 flex-shrink-0 mt-0.5"
            :class="toast.type === 'progress' ? 'animate-spin' : ''"
            :style="{ color: colors[toast.type].icon }"
          />
          <div class="flex-1 min-w-0">
            <p class="text-sm">{{ toast.message }}</p>
            <p v-if="toast.detail" class="text-[11px] text-[var(--muted-foreground)] mt-1">
              {{ toast.detail }}
            </p>
            <div v-if="toast.type === 'progress'" class="mt-2">
              <div class="h-1.5 rounded-full bg-[var(--muted)]/70 overflow-hidden">
                <div
                  class="h-full rounded-full bg-[var(--primary)] transition-[width] duration-200 ease-out"
                  :style="{ width: `${Math.max(0, Math.min(100, toast.progress ?? 0))}%` }"
                />
              </div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-1 text-right tabular-nums">
                {{ Math.round(toast.progress ?? 0) }}%
              </p>
            </div>
            <div v-if="toast.actions?.length" class="mt-2 flex flex-wrap gap-1.5">
              <button
                v-for="(action, idx) in toast.actions"
                :key="idx"
                class="px-2 py-1 text-[11px] rounded border transition-colors hover:brightness-95"
                :style="actionButtonStyle(action.style)"
                @click="action.onClick(); remove(toast.id)"
              >
                {{ action.label }}
              </button>
            </div>
          </div>
          <button
            @click="remove(toast.id)"
            class="p-0.5 rounded hover:bg-[var(--secondary)] transition-colors flex-shrink-0"
          >
            <X class="w-4 h-4 text-[var(--muted-foreground)]" />
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-enter-active {
  animation: toast-in 0.3s ease-out;
}
.toast-leave-active {
  animation: toast-out 0.2s ease-in;
}

.toast-loader-logo {
  width: 22px;
  height: 22px;
  object-fit: contain;
  filter: drop-shadow(0 0 6px rgba(20, 184, 166, 0.45));
}

.toast-loader-wave {
  display: inline-flex;
  gap: 0.5px;
  line-height: 1;
}

.toast-loader-letter {
  font-size: 11px;
  font-weight: 700;
  color: #5eead4;
  text-transform: uppercase;
  animation: toast-loader-bounce 1s ease-in-out infinite;
}

@keyframes toast-loader-bounce {
  0%,
  50%,
  100% {
    transform: translateY(0);
    opacity: 0.5;
  }
  25% {
    transform: translateY(-3px);
    opacity: 1;
  }
}

@keyframes toast-in {
  from {
    opacity: 0;
    transform: translateX(100%);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

@keyframes toast-out {
  from {
    opacity: 1;
    transform: translateX(0);
  }
  to {
    opacity: 0;
    transform: translateX(100%);
  }
}
</style>

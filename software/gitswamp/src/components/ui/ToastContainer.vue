<script setup lang="ts">
import { useToast } from "@/composables/useToast";
import { CheckCircle, XCircle, Info, AlertTriangle, X } from "lucide-vue-next";

const { toasts, remove } = useToast();

const icons = {
  success: CheckCircle,
  error: XCircle,
  info: Info,
  warning: AlertTriangle,
};

const colors = {
  success: { bg: "bg-[#0b1f1a]/96", border: "border-[#10b981]/70", text: "text-[#34d399]", icon: "#34d399" },
  error: { bg: "bg-[#2a1316]/96", border: "border-[#ef4444]/75", text: "text-[#f87171]", icon: "#f87171" },
  info: { bg: "bg-[#111c2f]/96", border: "border-[#3b82f6]/75", text: "text-[#60a5fa]", icon: "#60a5fa" },
  warning: { bg: "bg-[#2a2210]/96", border: "border-[#f59e0b]/75", text: "text-[#fbbf24]", icon: "#fbbf24" },
};
</script>

<template>
  <Teleport to="body">
    <div class="fixed bottom-4 right-4 z-[200] flex flex-col gap-2 max-w-sm">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="flex items-start gap-3 px-4 py-3 rounded-lg border shadow-xl backdrop-blur-md"
          :class="[colors[toast.type].bg, colors[toast.type].border]"
        >
          <component
            :is="icons[toast.type]"
            class="w-5 h-5 flex-shrink-0 mt-0.5"
            :style="{ color: colors[toast.type].icon }"
          />
          <div class="flex-1 min-w-0">
            <p class="text-sm text-[var(--foreground)]">{{ toast.message }}</p>
            <div v-if="toast.actions?.length" class="mt-2 flex flex-wrap gap-1.5">
              <button
                v-for="(action, idx) in toast.actions"
                :key="idx"
                class="px-2 py-1 text-[11px] rounded transition-colors"
                :class="[
                  action.style === 'danger'
                    ? 'bg-[#ef4444] text-white hover:bg-[#dc2626]'
                    : action.style === 'primary'
                      ? 'bg-[#2563eb] text-white hover:bg-[#1d4ed8]'
                      : 'bg-[#374151] text-white hover:bg-[#4b5563]'
                ]"
                @click="action.onClick(); remove(toast.id)"
              >
                {{ action.label }}
              </button>
            </div>
          </div>
          <button
            @click="remove(toast.id)"
            class="p-0.5 rounded hover:bg-white/10 transition-colors flex-shrink-0"
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

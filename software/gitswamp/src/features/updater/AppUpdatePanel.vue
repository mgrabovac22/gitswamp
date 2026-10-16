<script setup lang="ts">
import { ArrowDownToLine, Loader2, X } from "lucide-vue-next";
import { useAppUpdate } from "./useAppUpdate";

const {
  status,
  info: updateInfo,
  errorMessage: errorText,
  percent: progressPercent,
  downloaded: downloadedBytes,
  total: totalBytes,
  isVisible: panelVisible,
  install,
  dismiss,
} = useAppUpdate();

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<template>
  <Transition
    enter-active-class="transition-all duration-200 ease-out"
    enter-from-class="opacity-0 translate-y-2"
    leave-active-class="transition-all duration-150 ease-in"
    leave-to-class="opacity-0 translate-y-2"
  >
    <div
      v-if="panelVisible && updateInfo"
      class="flex-shrink-0 border-t border-[var(--sidebar-border)] bg-[var(--sidebar-accent)]/60 px-3 py-2.5"
    >
      <div class="flex items-start gap-2">
        <div class="mt-0.5 flex-shrink-0">
          <Loader2
            v-if="status === 'installing'"
            class="w-4 h-4 text-[var(--sidebar-primary)] animate-spin"
          />
          <ArrowDownToLine v-else class="w-4 h-4 text-[var(--sidebar-primary)]" />
        </div>

        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <span class="text-[11px] font-semibold text-[var(--sidebar-foreground)] truncate">
              {{ status === "installing" ? "Installing update…" : "Update available" }}
            </span>
            <span
              v-if="updateInfo.prerelease"
              class="px-1 py-px rounded text-[8px] font-bold uppercase tracking-wide bg-[var(--sidebar-primary)]/20 text-[var(--sidebar-primary)] border border-[var(--sidebar-primary)]/40"
            >
              pre
            </span>
          </div>

          <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5 truncate">
            v{{ updateInfo.currentVersion }} → v{{ updateInfo.version }}
          </div>

          <p
            v-if="status !== 'installing' && updateInfo.notes"
            class="text-[10px] text-[var(--muted-foreground)]/80 mt-1 line-clamp-2 whitespace-pre-line"
          >
            {{ updateInfo.notes }}
          </p>

          <div v-if="status === 'installing'" class="mt-1.5">
            <div class="h-1 rounded-full bg-[var(--sidebar-border)] overflow-hidden">
              <div
                class="h-full bg-[var(--sidebar-primary)] transition-all duration-150"
                :style="progressPercent !== null ? { width: progressPercent + '%' } : { width: '35%' }"
                :class="progressPercent === null ? 'animate-pulse' : ''"
              />
            </div>
            <div class="text-[9px] text-[var(--muted-foreground)] mt-1 tabular-nums">
              <template v-if="progressPercent !== null">
                {{ progressPercent }}%
                <template v-if="totalBytes">
                  · {{ formatBytes(downloadedBytes) }} / {{ formatBytes(totalBytes) }}
                </template>
              </template>
              <template v-else>Downloading…</template>
            </div>
          </div>

          <p v-if="status === 'available' && errorText" class="text-[10px] text-[var(--destructive)] mt-1">
            {{ errorText }}
          </p>

          <div class="flex items-center gap-1.5 mt-2">
            <button
              @click="install"
              :disabled="status === 'installing'"
              class="flex-1 flex items-center justify-center gap-1 px-2 py-1 rounded text-[10px] font-semibold bg-[var(--sidebar-primary)] text-[var(--sidebar-primary-foreground)] hover:opacity-90 disabled:opacity-50 transition-all"
            >
              <ArrowDownToLine class="w-3 h-3" />
              Install
            </button>
            <button
              @click="dismiss"
              :disabled="status === 'installing'"
              class="flex items-center justify-center gap-1 px-2 py-1 rounded text-[10px] font-medium text-[var(--muted-foreground)] hover:text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)] disabled:opacity-50 transition-all"
            >
              <X class="w-3 h-3" />
              Dismiss
            </button>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

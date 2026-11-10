<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { Loader2, Trash2 } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import type { WorkspaceLocation } from "./branchWorkspaceTypes";

defineProps<{ location: WorkspaceLocation; busy: boolean; error: string }>();
const emit = defineEmits<{ cancel: []; keep: []; remove: [] }>();
const dialog = ref<HTMLDialogElement | null>(null);
onMounted(() => dialog.value?.showModal());
onBeforeUnmount(() => dialog.value?.close());
</script>

<template>
  <Teleport to="body">
    <dialog ref="dialog" class="worktree-removal" aria-labelledby="worktree-removal-title" @cancel.prevent="!busy && emit('cancel')">
      <header>
        <Trash2 :size="17" class="removal-icon" />
        <h2 id="worktree-removal-title">Remove worktree?</h2>
        <CloseIconButton title="Cancel worktree removal" size="sm" subtle :disabled="busy" @click="emit('cancel')" />
      </header>
      <div class="removal-body">
        <strong>{{ location.branch }}</strong>
        <code>{{ location.path }}</code>
        <p>The folder will be removed. The branch and its commits will stay in the repository.</p>
        <p v-if="error" class="removal-error" role="alert">{{ error }}</p>
      </div>
      <footer>
        <button type="button" autofocus :disabled="busy" @click="emit('cancel')">Cancel</button>
        <button type="button" :disabled="busy" @click="emit('keep')">Close without removing</button>
        <button type="button" class="remove-button" :disabled="busy" @click="emit('remove')">
          <Loader2 v-if="busy" :size="14" class="animate-spin" /><Trash2 v-else :size="14" />Remove worktree
        </button>
      </footer>
    </dialog>
  </Teleport>
</template>

<style scoped>
.worktree-removal { position: fixed; inset: 0; margin: auto; width: min(450px, calc(100% - 24px)); max-height: calc(100% - 24px); overflow: auto; padding: 0; border: 1px solid var(--border); border-radius: 8px; background: var(--popover); color: var(--foreground); box-shadow: 0 18px 46px #0006; }
.worktree-removal::backdrop { background: #0007; }
header, footer { display: flex; align-items: center; gap: 8px; padding: 10px 12px; }
header { border-bottom: 1px solid var(--border); }
h2 { flex: 1; font-size: 14px; font-weight: 650; }
.removal-icon, .removal-error { color: var(--destructive); }
.removal-body { display: grid; gap: 8px; padding: 13px 14px; font-size: 12px; overflow-wrap: anywhere; }
.removal-body code { font-size: 11px; color: var(--muted-foreground); }
.removal-error { padding: 8px; border: 1px solid color-mix(in srgb, var(--destructive), transparent 65%); border-radius: 5px; }
footer { justify-content: flex-end; flex-wrap: wrap; border-top: 1px solid var(--border); }
footer button { display: inline-flex; align-items: center; gap: 5px; padding: 5px 8px; border: 1px solid var(--border); border-radius: 5px; background: var(--secondary); font-size: 12px; }
footer button:not(:disabled):hover { filter: brightness(.94); }
footer button:disabled { opacity: .55; }
footer .remove-button { background: var(--destructive); color: var(--destructive-foreground); }
</style>

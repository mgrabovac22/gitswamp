<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Download, FileDiff, Loader2 } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import type { PatchScope } from "./historyEdits";

const props = defineProps<{ kind: "import" | "export"; file?: string; branch: string; initialStaged?: boolean; busy: boolean; error: string }>();
const emit = defineEmits<{ close: []; import: [staged: boolean]; export: [scope: PatchScope] }>();
const dialog = ref<HTMLDialogElement>();
const scope = ref<PatchScope>("all-tracked");
const staged = ref(!!props.initialStaged);
onMounted(() => dialog.value?.showModal());
const close = () => { if (!props.busy) emit("close"); };
function run() { if (!props.busy) props.kind === "import" ? emit("import", staged.value) : emit("export", scope.value); }
</script>

<template>
  <dialog ref="dialog" class="patch-dialog" aria-labelledby="patch-title" @cancel.prevent="close">
    <form @submit.prevent="run">
      <header><FileDiff :size="18" /><h2 id="patch-title">{{ kind === 'import' ? 'Import patch' : 'Export patch' }}</h2><CloseIconButton title="Close patch dialog" :disabled="busy" @click="close" /></header>
      <section>
        <template v-if="kind === 'import'">
          <p class="route"><code :title="file">{{ file?.split(/[\\/]/).pop() }}</code><span>{{ branch }}</span></p>
          <label class="check"><input v-model="staged" type="checkbox" :disabled="busy" /> Stage changes after applying</label>
          <p class="muted">Applies the file changes without creating a commit.</p>
        </template>
        <template v-else>
          <label>Changes to export<select v-model="scope" :disabled="busy"><option value="all-tracked">All tracked changes</option><option value="staged">Staged changes</option><option value="unstaged">Unstaged changes</option></select></label>
          <p class="muted">New untracked files must be staged before export.</p>
        </template>
        <p v-if="error" class="error" role="alert">{{ error }}</p>
      </section>
      <footer><button type="button" :disabled="busy" @click="close">Cancel</button><button class="primary" type="submit" :disabled="busy"><Loader2 v-if="busy" :size="15" class="animate-spin" /><Download v-else-if="kind === 'export'" :size="15" />{{ kind === 'import' ? 'Apply patch' : 'Save patch' }}</button></footer>
    </form>
  </dialog>
</template>

<style scoped>
.patch-dialog { margin: auto; width: min(450px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); padding: 0; border: 1px solid var(--border); border-radius: 8px; color: var(--foreground); background: var(--popover); font-size: 13px; }
.patch-dialog::backdrop { background: #0008; }
header { display: grid; grid-template-columns: 18px minmax(0, 1fr) auto; align-items: center; gap: 8px; padding: 10px 12px; border-bottom: 1px solid var(--border); }
header h2 { font-size: 15px; font-weight: 650; }
section, label { display: grid; gap: 7px; }
section { padding: 12px; overflow-wrap: anywhere; }
.route { display: flex; flex-wrap: wrap; gap: 7px; align-items: baseline; }
.route code { max-width: 100%; background: var(--secondary); padding: 2px 4px; }
.muted, .route span { color: var(--muted-foreground); }
.check { display: flex; gap: 6px; align-items: center; }
select { width: 100%; padding: 5px 8px; background: var(--input-background); border: 1px solid var(--border); border-radius: 5px; }
.error { color: var(--destructive); white-space: pre-wrap; }
footer { padding: 9px 12px; display: flex; justify-content: end; gap: 7px; border-top: 1px solid var(--border); }
footer button { display: inline-flex; align-items: center; gap: 6px; min-height: 32px; padding: 4px 8px; border: 1px solid var(--border); border-radius: 5px; }
.primary { background: var(--primary); color: var(--primary-foreground); }
button:disabled { opacity: .55; }
</style>

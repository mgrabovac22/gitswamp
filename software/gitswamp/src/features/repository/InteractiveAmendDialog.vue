<script setup lang="ts">
import { computed, nextTick, ref, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { GitCommitHorizontal, Loader2, RotateCw } from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import { amendActions, type AmendAction, type AmendConfirmation } from "./historyEdits";

type Commit = { sha: string; message: string };
type Plan = { branch: string; head: string; target: Commit; parent: Commit | null; replay: number; published: boolean; dirty: boolean };
const props = defineProps<{ repoPath: string; sha: string; action: AmendAction; busy: boolean; executionError: string }>();
const emit = defineEmits<{ close: []; start: [confirmation: AmendConfirmation] }>();
const dialog = ref<HTMLDialogElement>();
const plan = shallowRef<Plan>();
const error = ref("");
const loading = ref(false);
const message = ref("");
const acknowledged = ref(false);
const changedMessage = computed(() => props.action === "reword" || props.action === "squash");
const combines = computed(() => props.action === "fixup" || props.action === "squash");
const canStart = computed(() => !props.busy && !loading.value && !!plan.value && !plan.value.dirty && (!plan.value.published || acknowledged.value)
  && (!changedMessage.value || !!message.value.trim()) && (props.action !== "reword" || message.value.trim() !== plan.value.target.message.trim()));
let sequence = 0;

async function load() {
  const request = ++sequence;
  loading.value = true;
  error.value = "";
  plan.value = undefined;
  acknowledged.value = false;
  try {
    const result = await invoke<Plan>("get_interactive_amend_plan", { path: props.repoPath, sha: props.sha, action: props.action });
    if (request !== sequence) return;
    plan.value = result;
    message.value = props.action === "squash" ? [result.parent?.message, result.target.message].filter(Boolean).join("\n\n") : result.target.message;
  } catch (failure) { if (request === sequence) error.value = String(failure); }
  finally { if (request === sequence) loading.value = false; }
}

watch(dialog, async element => { await nextTick(); if (element && !element.open) element.showModal(); });
watch([() => props.repoPath, () => props.sha, () => props.action], load, { immediate: true });
const close = () => { if (!props.busy) emit("close"); };
function start() {
  if (!canStart.value || !plan.value) return;
  emit("start", { head: plan.value.head, branch: plan.value.branch, acknowledgePublished: acknowledged.value,
    ...(changedMessage.value ? { message: message.value } : {}) });
}
</script>

<template>
  <dialog ref="dialog" class="amend-dialog" aria-labelledby="interactive-amend-title" @cancel.prevent="close">
    <form @submit.prevent="start">
      <header><component :is="amendActions[action].icon" :size="18" /><div><h2 id="interactive-amend-title">Interactive Amend</h2><small>{{ amendActions[action].label.replace('…', '') }}</small></div><CloseIconButton title="Close Interactive Amend" :disabled="busy" @click="close" /></header>
      <section>
        <p v-if="loading" class="line" role="status"><Loader2 :size="15" class="animate-spin" /> Reviewing branch history</p>
        <p v-if="error || executionError" class="error" role="alert">{{ error || executionError }}</p>
        <template v-if="plan">
          <p class="muted"><code>{{ plan.branch }}</code><template v-if="plan.replay"> · {{ plan.replay }} later {{ plan.replay === 1 ? 'commit' : 'commits' }} to replay</template></p>
          <div v-if="combines && plan.parent" class="commit"><GitCommitHorizontal :size="15" /><code>{{ plan.parent.sha.slice(0, 8) }}</code><span>{{ plan.parent.message.split('\n')[0] }}</span></div>
          <div class="commit" :class="{ dropped: action === 'drop' }"><GitCommitHorizontal :size="15" /><code>{{ plan.target.sha.slice(0, 8) }}</code><span>{{ plan.target.message.split('\n')[0] }}</span></div>
          <p v-if="plan.dirty" class="error" role="status">Commit or stash working changes before rewriting history. You can also export a patch first.</p>
          <p v-else-if="action === 'drop'" class="muted">This removes the selected commit from the current branch. Its changes are not moved to Working Changes.</p>
          <p v-else-if="action === 'edit'" class="muted">Stage the files you want to add or edit at the pause, then use Amend Last Commit and Continue rebase.</p>
          <p v-else-if="action === 'fixup'" class="muted">The parent's message is kept unchanged.</p>
          <label v-if="changedMessage" class="field">{{ action === 'squash' ? 'Combined commit message' : 'New commit message' }}<textarea v-model="message" rows="6" spellcheck="false" :disabled="busy" /></label>
          <label v-if="plan.published" class="line"><input v-model="acknowledged" type="checkbox" :disabled="busy" /> I understand this rewrites commits on the tracked remote branch.</label>
        </template>
      </section>
      <footer>
        <button v-if="plan?.dirty || error || executionError" type="button" :disabled="loading || busy" @click="load"><RotateCw :size="14" /> Review again</button>
        <button type="button" :disabled="busy" @click="close">Cancel</button>
        <button class="primary" type="submit" :disabled="!canStart"><Loader2 v-if="busy" :size="15" class="animate-spin" />{{ busy ? 'Rewriting…' : action === 'edit' ? 'Start editing' : 'Confirm amend' }}</button>
      </footer>
    </form>
  </dialog>
</template>

<style scoped>
.amend-dialog { margin: auto; width: min(545px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); padding: 0; border: 1px solid var(--border); border-radius: 8px; background: var(--popover); color: var(--foreground); font-size: 13px; }
.amend-dialog::backdrop { background: #0008; }
header { display: grid; grid-template-columns: 18px minmax(0, 1fr) auto; align-items: center; gap: 8px; padding: 12px; border-bottom: 1px solid var(--border); }
header h2 { font-size: 15px; font-weight: 650; }
header small, .muted, .commit code { color: var(--muted-foreground); }
section { display: grid; gap: 8px; padding: 12px; overflow-wrap: anywhere; }
.commit { display: grid; grid-template-columns: 15px auto minmax(0, 1fr); align-items: start; gap: 7px; border-bottom: 1px solid var(--border); padding: 4px 0; }
.commit.dropped { color: var(--muted-foreground); }
.line, footer, footer button { display: flex; align-items: center; gap: 6px; }
.field { display: grid; gap: 5px; }
textarea { width: 100%; resize: vertical; padding: 7px; background: var(--input-background); border: 1px solid var(--border); border-radius: 5px; }
.error { color: var(--destructive); }
footer { justify-content: end; flex-wrap: wrap; padding: 9px 12px; border-top: 1px solid var(--border); }
footer button { min-height: 32px; padding: 4px 8px; border-radius: 5px; border: 1px solid var(--border); }
footer button:hover:not(:disabled) { background: var(--secondary); }
footer button.primary { background: var(--primary); color: var(--primary-foreground); }
footer button:disabled { opacity: .5; }
</style>

<script setup lang="ts">
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, GitBranch, GitFork, GitPullRequest, GitMerge, Terminal } from "lucide-vue-next";
import { strategies, workflowDefaults, workflowName, type Strategy, type WorkflowConfig } from "./branchWorkflow";

const props = defineProps<{
  initial?: WorkflowConfig;
  suggestion?: Strategy | null;
  primary: string;
  development?: string | null;
  branches?: string[];
  label: string;
}>();
const emit = defineEmits<{ save: [config: WorkflowConfig]; cancel: [] }>();
const icons = { swamp: GitPullRequest, github: GitFork, trunk: GitMerge, gitflow: GitBranch, manual: Terminal };
const draft = ref<WorkflowConfig>();
const error = ref("");
const busy = ref(false);
const prefixFields = computed(() => {
  const config = draft.value;
  if (!config || config.strategy === "manual") return [] as { key: "feature" | "fix" | "release" | "hotfix" | "support" | "tag"; label: string }[];
  const fields: { key: "feature" | "fix" | "release" | "hotfix" | "support" | "tag"; label: string }[] = [];
  if (config.strategy !== "trunk" || config.shortBranches) fields.push({ key: "feature", label: "Feature prefix" });
  if (config.strategy !== "gitflow" && (config.strategy !== "trunk" || config.shortBranches)) fields.push({ key: "fix", label: "Fix prefix" });
  if (config.strategy === "gitflow" || config.strategy === "trunk" && config.releaseBranches) fields.push({ key: "release", label: "Release prefix" });
  if (config.strategy === "gitflow") fields.push({ key: "hotfix", label: "Hotfix prefix" }, { key: "support", label: "Support prefix" });
  if (config.strategy === "gitflow" || config.strategy === "trunk") fields.push({ key: "tag", label: "Tag prefix" });
  return fields;
});

function choose(strategy: Strategy) {
  error.value = "";
  draft.value = props.initial?.strategy === strategy
    ? { ...props.initial }
    : workflowDefaults(strategy, props.primary, props.development ?? "develop");
}

async function save() {
  if (!draft.value || busy.value) return;
  const config = { ...draft.value };
  for (const key of ["primary", "development", "feature", "fix", "release", "hotfix", "support", "tag"] as const) {
    config[key] = config[key].trim();
  }
  busy.value = true;
  error.value = "";
  try {
    await invoke("validate_branch_workflow", { config });
    emit("save", config);
  } catch (failure) { error.value = String(failure); }
  finally { busy.value = false; }
}
</script>

<template>
  <div class="setup">
    <template v-if="!draft">
      <div class="choices" role="group" aria-label="Branching strategies">
        <button v-for="item in strategies" :key="item.id" type="button" class="choice" @click="choose(item.id)">
          <component :is="icons[item.id]" :size="17" aria-hidden="true" />
          <span><strong>{{ item.name }}</strong><small>{{ item.summary }}</small></span>
          <small v-if="initial?.strategy === item.id" class="hint">Current</small>
          <small v-else-if="suggestion === item.id" class="hint">Suggested</small>
        </button>
      </div>
      <footer><button type="button" class="plain" @click="emit('cancel')">Cancel</button></footer>
    </template>
    <form v-else class="setup-form" @submit.prevent="save">
      <div class="form-title">
        <button type="button" class="plain icon" title="Back to strategies" aria-label="Back to strategies" @click="draft = undefined"><ArrowLeft :size="16" /></button>
        <strong>{{ workflowName(draft.strategy) }}</strong>
      </div>
      <template v-if="draft.strategy !== 'manual'">
        <datalist v-if="branches?.length" id="workflow-branch-names">
          <option v-for="branch in branches" :key="branch" :value="branch" />
        </datalist>
        <div class="fields">
          <label>{{ draft.strategy === 'trunk' ? 'Trunk branch' : 'Primary branch' }}
            <input v-model="draft.primary" maxlength="160" required :list="branches?.length ? 'workflow-branch-names' : undefined" />
          </label>
          <label v-if="draft.strategy === 'gitflow'">Development branch
            <input v-model="draft.development" maxlength="160" required :list="branches?.length ? 'workflow-branch-names' : undefined" />
          </label>
        </div>
        <template v-if="draft.strategy === 'trunk'">
          <label class="check"><input v-model="draft.shortBranches" type="checkbox" /> Short-lived feature and bugfix branches</label>
          <label class="check"><input v-model="draft.releaseBranches" type="checkbox" /> Release branches</label>
        </template>
        <div v-if="prefixFields.length" class="fields">
          <label v-for="field in prefixFields" :key="field.key">{{ field.label }}<input v-model="draft[field.key]" maxlength="80" /></label>
        </div>
      </template>
      <p v-if="error" role="alert" class="error">{{ error }}</p>
      <footer><button type="button" class="plain" :disabled="busy" @click="emit('cancel')">Cancel</button><button type="submit" class="primary" :disabled="busy">{{ label }}</button></footer>
    </form>
  </div>
</template>

<style scoped>
.setup, .setup-form { display: grid; gap: 12px; }
.choices, .fields { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
.choice { display: grid; grid-template-columns: 18px minmax(0, 1fr); text-align: left; gap: 2px 8px; align-items: start; border: 1px solid var(--border); border-radius: 5px; padding: 10px; }
.choice svg { color: var(--primary); flex-shrink: 0; }
.choice span { display: grid; gap: 3px; }
.choice strong { font-size: 12px; }
.choice small { font-size: 11px; color: var(--muted-foreground); overflow-wrap: anywhere; }
.choice .hint { grid-column: 2; color: var(--primary); }
.choice:hover, .plain:hover { background: var(--secondary); }
.fields label { display: grid; gap: 4px; font-size: 12px; }
.fields input { width: 100%; padding: 5px 7px; border: 1px solid var(--border); border-radius: 4px; background: var(--input-background); }
.check, .form-title { display: flex; align-items: center; gap: 7px; }
.check { font-size: 12px; }
footer { display: flex; justify-content: flex-end; gap: 6px; }
.plain, .primary { padding: 5px 9px; border: 1px solid var(--border); border-radius: 4px; }
.plain.icon { display: grid; place-items: center; padding: 4px; }
.primary { background: var(--primary); color: var(--primary-foreground); }
.error { color: var(--destructive); overflow-wrap: anywhere; }
@media (max-width: 410px) { .choices { grid-template-columns: 1fr; } }
</style>

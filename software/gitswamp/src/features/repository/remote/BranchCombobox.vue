<script setup lang="ts">
import { computed, nextTick, ref, useId, watch } from "vue";
import { Check, ChevronDown, GitBranch, Search } from "lucide-vue-next";
import { orderBranches } from "./branchOptions";

const props = defineProps<{ label: string; modelValue: string; branches: readonly string[]; excluded: string; preferred: string[]; recommendation?: { branch: string; reason: string } | null }>();
const emit = defineEmits<{ "update:modelValue": [branch: string] }>();
const id = useId();
const trigger = ref<HTMLButtonElement>();
const input = ref<HTMLInputElement>();
const list = ref<HTMLElement>();
const opened = ref(false);
const query = ref("");
const active = ref(0);
const pageSize = 50;
const limit = ref(pageSize);
const options = computed(() => orderBranches(props.branches, props.preferred, props.excluded));
const rows = computed(() => {
  const term = query.value.trim().toLowerCase();
  return [...options.value.pinned, ...options.value.other].filter(branch => branch.toLowerCase().includes(term));
});
watch(query, () => { active.value = 0; limit.value = pageSize; if (list.value) list.value.scrollTop = 0; });
watch(() => rows.value.length, length => { active.value = Math.max(0, Math.min(active.value, length - 1)); });

async function toggle() {
  if (opened.value) { close(); return; }
  query.value = "";
  active.value = 0;
  limit.value = pageSize;
  opened.value = true;
  await nextTick();
  input.value?.focus();
}

function close(restoreFocus = false) {
  opened.value = false;
  if (restoreFocus) trigger.value?.focus();
}

function choose(branch?: string) {
  if (!branch) return;
  emit("update:modelValue", branch);
  close(true);
}

function focusLeft(event: FocusEvent) {
  if (event.currentTarget instanceof HTMLElement && !event.currentTarget.contains(event.relatedTarget as Node | null)) close();
}

async function move(offset: number) {
  active.value = Math.max(0, Math.min(rows.value.length - 1, active.value + offset));
  if (active.value >= limit.value) limit.value += pageSize;
  await nextTick();
  list.value?.querySelector(`[data-index="${active.value}"]`)?.scrollIntoView({ block: "nearest" });
}
</script>

<template>
  <div class="branch-picker" @focusout="focusLeft" @keydown.esc.stop.prevent="close(true)">
    <button ref="trigger" type="button" class="trigger" :aria-label="label + ': ' + (modelValue || 'Select branch')"
      :aria-expanded="opened" :aria-controls="opened ? `${id}-branches` : undefined" @click="toggle" @keydown.down.prevent="opened ? move(1) : toggle()">
      <GitBranch :size="15" /><strong>{{ label }}:</strong><span :title="modelValue">{{ modelValue || 'Select branch' }}</span><ChevronDown :size="14" />
    </button>
    <div v-if="opened" class="dropdown">
      <div class="search"><Search :size="15" /><input ref="input" v-model="query" role="combobox" :aria-label="`Search ${label.toLowerCase()} branches`" aria-expanded="true"
        aria-autocomplete="list" :aria-controls="`${id}-branches`" :aria-activedescendant="rows.length ? `${id}-${active}` : undefined"
        autocomplete="off" spellcheck="false" placeholder="Find a branch" @keydown.down.prevent="move(1)" @keydown.up.prevent="move(-1)" @keydown.enter.prevent="choose(rows[active])" /></div>
      <div ref="list" :id="`${id}-branches`" class="list" role="listbox" :aria-label="label + ' branches'">
        <div v-for="(branch, index) in rows.slice(0, limit)" :id="`${id}-${index}`" :key="branch" role="option" :aria-selected="branch === modelValue"
          :data-index="index" :class="{ active: index === active }" class="option" @pointermove="active = index" @mousedown.prevent @click="choose(branch)">
          <Check v-if="branch === modelValue" :size="14" /><span v-else aria-hidden="true" />
          <span class="name">{{ branch }}</span><small v-if="recommendation?.branch === branch">{{ recommendation.reason }}</small>
        </div>
      </div>
      <p v-if="!rows.length" class="empty" role="status">{{ query ? 'No matching branches' : 'No branches available' }}</p>
      <button v-if="rows.length > limit" class="more" type="button" @click="limit += pageSize; input?.focus()">Show more branches</button>
    </div>
  </div>
</template>

<style scoped>
.branch-picker { position: relative; min-width: 0; }
.trigger, .search, .option { display: flex; align-items: center; gap: 8px; }
.trigger { min-height: 36px; width: 100%; border: 1px solid var(--border); border-radius: 6px; padding: 5px 8px; background: var(--secondary); text-align: left; }
.trigger:hover { border-color: var(--primary); }
.trigger strong { font-weight: 500; color: var(--muted-foreground); }
.trigger span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: monospace; }
.trigger svg, .search svg { flex-shrink: 0; }
.dropdown { position: absolute; z-index: 10; inset-inline-start: 0; top: calc(100% + 4px); width: 100%; background: var(--popover); border: 1px solid var(--border); border-radius: 8px; box-shadow: 0 7px 24px #0004; }
.search { border-bottom: 1px solid var(--border); padding: 8px; }
.search input { min-width: 0; width: 100%; padding: 4px; border: 0; background: transparent; outline: none; }
.search:focus-within { box-shadow: inset 0 -2px var(--primary); }
.list { max-height: min(230px, 42vh); overflow: auto; }
.option { min-height: 32px; padding: 5px 8px; cursor: pointer; }
.option > :first-child { width: 14px; flex-shrink: 0; }
.option.active { background: var(--secondary); }
.option .name { flex: 1; overflow-wrap: anywhere; }
.option small, .empty { color: var(--muted-foreground); }
.option small { max-width: 40%; text-align: right; }
.empty, .more { padding: 7px 9px; }
.more { width: 100%; text-align: left; border-top: 1px solid var(--border); color: var(--primary); }
.more:hover { background: var(--secondary); }
</style>

<script setup lang="ts">
import { ref, nextTick, watch } from "vue";
import { Terminal, X, ChevronUp } from "lucide-vue-next";

const props = defineProps<{
  output: string[];
  repoPath: string;
}>();

const emit = defineEmits<{
  run: [command: string];
  close: [];
}>();

const inputRef = ref<HTMLInputElement | null>(null);
const outputRef = ref<HTMLElement | null>(null);
const command = ref("");
const history = ref<string[]>([]);
const historyIndex = ref(-1);

watch(() => props.output.length, () => {
  nextTick(() => {
    if (outputRef.value) {
      outputRef.value.scrollTop = outputRef.value.scrollHeight;
    }
  });
});

function submit() {
  const cmd = command.value.trim();
  if (!cmd) return;
  history.value.push(cmd);
  historyIndex.value = -1;
  emit("run", cmd);
  command.value = "";
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === "ArrowUp") {
    e.preventDefault();
    if (history.value.length === 0) return;
    if (historyIndex.value < 0) historyIndex.value = history.value.length;
    historyIndex.value = Math.max(0, historyIndex.value - 1);
    command.value = history.value[historyIndex.value];
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    if (historyIndex.value < 0) return;
    historyIndex.value++;
    if (historyIndex.value >= history.value.length) {
      historyIndex.value = -1;
      command.value = "";
    } else {
      command.value = history.value[historyIndex.value];
    }
  }
}

function focusInput() {
  inputRef.value?.focus();
}
</script>

<template>
  <div class="bg-[#0a0e14] border-t border-[#8b5cf6]/15 flex flex-col" @click="focusInput">
    <!-- Header -->
    <div class="flex items-center justify-between px-3 py-1.5 bg-[#0d1017] border-b border-[#8b5cf6]/10 flex-shrink-0">
      <div class="flex items-center gap-2 text-xs text-[#64748b]">
        <Terminal class="w-3.5 h-3.5 text-[#a78bfa]" />
        <span class="font-medium text-[#e2e8f0]">Terminal</span>
        <span class="text-[10px] text-[#475569]">git commands only</span>
      </div>
      <button
        @click="emit('close')"
        class="p-1 rounded hover:bg-[#1e293b] transition-colors"
      >
        <X class="w-3.5 h-3.5 text-[#64748b]" />
      </button>
    </div>

    <!-- Output area -->
    <div ref="outputRef" class="flex-1 overflow-y-auto px-3 py-2 font-mono text-xs min-h-0">
      <div v-if="output.length === 0" class="text-[#475569] py-2">
        Type a git command (e.g., "git status", "git log --oneline -5")
      </div>
      <div
        v-for="(line, i) in output"
        :key="i"
        class="whitespace-pre-wrap mb-2"
      >
        <template v-for="(part, j) in line.split('\n')" :key="j">
          <div :class="part.startsWith('$') ? 'text-[#a78bfa] font-medium' : part.startsWith('Error:') ? 'text-[#ef4444]' : 'text-[#94a3b8]'">
            {{ part }}
          </div>
        </template>
      </div>
    </div>

    <!-- Input -->
    <div class="flex items-center gap-2 px-3 py-2 border-t border-[#8b5cf6]/10 flex-shrink-0 bg-[#0d1017]">
      <span class="text-[#a78bfa] text-xs font-mono font-bold">$</span>
      <input
        ref="inputRef"
        v-model="command"
        @keyup.enter="submit"
        @keydown="onKeyDown"
        placeholder="git ..."
        class="flex-1 bg-transparent text-xs text-[#e2e8f0] font-mono placeholder:text-[#334155] focus:outline-none"
      />
    </div>
  </div>
</template>

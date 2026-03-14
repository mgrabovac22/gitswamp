<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Shield, Eye, EyeOff, Check, Trash2, X } from "lucide-vue-next";
import AppButton from "@/components/ui/AppButton.vue";

const props = defineProps<{
  token: string | null;
}>();

const emit = defineEmits<{
  save: [token: string];
  delete: [];
  close: [];
}>();

const tokenInput = ref("");
const showToken = ref(false);
const saved = ref(false);

onMounted(() => {
  if (props.token) {
    tokenInput.value = props.token;
  }
});

function handleSave() {
  if (!tokenInput.value.trim()) return;
  emit("save", tokenInput.value.trim());
  saved.value = true;
  setTimeout(() => { saved.value = false; }, 2000);
}

function handleDelete() {
  tokenInput.value = "";
  emit("delete");
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close')">
    <div class="w-[440px] bg-[#111520] border border-[#8b5cf6]/20 rounded-xl shadow-2xl overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-4 border-b border-[#8b5cf6]/10">
        <div class="flex items-center gap-2">
          <Shield class="w-4 h-4 text-[#8b5cf6]" />
          <h2 class="text-sm font-semibold text-[#e2e8f0]">Settings</h2>
        </div>
        <button @click="emit('close')" class="p-1 rounded hover:bg-[#252b3d] transition-colors">
          <X class="w-4 h-4 text-[#64748b]" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-5 space-y-4">
        <div>
          <label class="text-xs font-medium text-[#e2e8f0] mb-1.5 block">GitHub Personal Access Token</label>
          <p class="text-[10px] text-[#64748b] mb-3">
            Used for push, pull, and fetch over HTTPS. Generate one at
            <span class="text-[#a78bfa]">GitHub → Settings → Developer settings → Personal access tokens</span>.
          </p>
          <div class="flex items-center gap-2">
            <div class="relative flex-1">
              <input
                v-model="tokenInput"
                :type="showToken ? 'text' : 'password'"
                placeholder="ghp_xxxxxxxxxxxxxxxxxxxx"
                class="w-full px-3 py-2 pr-8 bg-[#151d28] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] placeholder:text-[#334155] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40 font-mono"
              />
              <button
                @click="showToken = !showToken"
                class="absolute right-2 top-1/2 -translate-y-1/2 text-[#64748b] hover:text-[#e2e8f0] transition-colors"
              >
                <EyeOff v-if="showToken" class="w-3.5 h-3.5" />
                <Eye v-else class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <AppButton
            class="flex-1 bg-[#8b5cf6] hover:bg-[#7c3aed] text-white text-xs font-medium h-8"
            :disabled="!tokenInput.trim()"
            @click="handleSave"
          >
            <Check v-if="saved" class="w-3.5 h-3.5 mr-1" />
            {{ saved ? 'Saved!' : 'Save Token' }}
          </AppButton>
          <AppButton
            v-if="token"
            class="bg-[#ef4444]/20 hover:bg-[#ef4444]/30 text-[#ef4444] text-xs font-medium h-8 px-3"
            @click="handleDelete"
          >
            <Trash2 class="w-3.5 h-3.5" />
          </AppButton>
        </div>

        <div v-if="token" class="flex items-center gap-2 text-[10px] text-[#10b981]">
          <div class="w-1.5 h-1.5 rounded-full bg-[#10b981]" />
          Token configured
        </div>
        <div v-else class="flex items-center gap-2 text-[10px] text-[#f59e0b]">
          <div class="w-1.5 h-1.5 rounded-full bg-[#f59e0b]" />
          No token configured — push/pull may fail for private repos
        </div>
      </div>
    </div>
  </div>
</template>

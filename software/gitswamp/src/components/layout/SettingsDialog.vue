<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Shield, Eye, EyeOff, Check, Trash2, X, Sun, Moon } from "lucide-vue-next";
import AppButton from "@/components/ui/AppButton.vue";

const props = defineProps<{
  token: string | null;
  gitPath?: string;
}>();

const emit = defineEmits<{
  save: [token: string];
  delete: [];
  close: [];
}>();

const tokenInput = ref("");
const showToken = ref(false);
const saved = ref(false);
const isDark = ref(true);

onMounted(() => {
  if (props.token) {
    tokenInput.value = props.token;
  }
  isDark.value = !document.documentElement.classList.contains("light");
});

function toggleTheme() {
  isDark.value = !isDark.value;
  if (isDark.value) {
    document.documentElement.classList.remove("light");
    localStorage.setItem("gitswamp-theme", "dark");
  } else {
    document.documentElement.classList.add("light");
    localStorage.setItem("gitswamp-theme", "light");
  }
}

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
    <div class="w-[440px] bg-[var(--card)] border border-[var(--border)] rounded-xl shadow-2xl overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-4 border-b border-[var(--border)]">
        <div class="flex items-center gap-2">
          <Shield class="w-4 h-4 text-[var(--primary)]" />
          <h2 class="text-sm font-semibold text-[var(--foreground)]">Settings</h2>
        </div>
        <button @click="emit('close')" class="p-1 rounded hover:bg-[var(--secondary)] transition-colors">
          <X class="w-4 h-4 text-[var(--muted-foreground)]" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-5 space-y-4">
        <!-- Theme Toggle -->
        <div class="flex items-center justify-between">
          <div>
            <label class="text-xs font-medium text-[var(--foreground)] block">Appearance</label>
            <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Switch between dark and light mode</p>
          </div>
          <button
            @click="toggleTheme"
            class="relative w-14 h-7 rounded-full transition-colors duration-300 flex-shrink-0"
            :class="isDark ? 'bg-[var(--accent)]' : 'bg-[var(--primary)]'"
          >
            <div
              class="absolute top-0.5 w-6 h-6 rounded-full bg-white shadow-md flex items-center justify-center transition-all duration-300"
              :class="isDark ? 'left-0.5' : 'left-[calc(100%-1.625rem)]'"
            >
              <Moon v-if="isDark" class="w-3.5 h-3.5 text-[var(--accent)]" />
              <Sun v-else class="w-3.5 h-3.5 text-[var(--primary)]" />
            </div>
          </button>
        </div>

        <div class="border-t border-[var(--border)] pt-4">
          <label class="text-xs font-medium text-[var(--foreground)] mb-1.5 block">GitHub Personal Access Token</label>
          <p class="text-[10px] text-[var(--muted-foreground)] mb-3">
            Used for push, pull, and fetch over HTTPS. Generate one at
            <span class="text-[var(--primary)]">GitHub → Settings → Developer settings → Personal access tokens</span>.
          </p>
          <div class="flex items-center gap-2">
            <div class="relative flex-1">
              <input
                v-model="tokenInput"
                :type="showToken ? 'text' : 'password'"
                placeholder="ghp_xxxxxxxxxxxxxxxxxxxx"
                class="w-full px-3 py-2 pr-8 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 font-mono"
              />
              <button
                @click="showToken = !showToken"
                class="absolute right-2 top-1/2 -translate-y-1/2 text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors"
              >
                <EyeOff v-if="showToken" class="w-3.5 h-3.5" />
                <Eye v-else class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <AppButton
            class="flex-1 bg-[var(--primary)] hover:opacity-90 text-white text-xs font-medium h-8"
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

        <!-- Git Path -->
        <div class="border-t border-[var(--border)] pt-4 mt-4">
          <label class="text-xs font-medium text-[var(--foreground)] mb-1.5 block">Git Executable</label>
          <div class="flex items-center gap-2 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs font-mono text-[var(--primary)]">
            {{ gitPath || 'Detecting...' }}
          </div>
          <p class="text-[10px] text-[var(--muted-foreground)] mt-1.5">
            Auto-detected git path. If push/pull fails with "program not found", ensure git is installed and in your PATH.
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

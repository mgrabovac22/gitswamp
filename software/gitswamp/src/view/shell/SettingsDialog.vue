<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { Shield, Eye, EyeOff, Check, Trash2, X, Layout, Monitor, Sun, Moon } from "lucide-vue-next";
import AppButton from "@/shared/ui/AppButton.vue";
import {
  applyAppPalettePreference,
  applyThemeModePreference,
  getStoredAppPalettePreference,
  getStoredThemeModePreference,
  storeAppPalettePreference,
  storeThemeModePreference,
  type AppPalettePreference,
  type ThemeModePreference,
} from "@/shared/themePreferences";

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
const themeMode = ref<ThemeModePreference>("dark");
const appPalette = ref<AppPalettePreference>("default");
const fontSize = ref<"small" | "medium" | "large">("medium");
const compactMode = ref(false);
const showAvatars = ref(true);

const appPaletteOptions: { value: AppPalettePreference; label: string }[] = [
  { value: "swamp", label: "Swamp" },
  { value: "default", label: "Default" },
  { value: "github", label: "GitHub" },
  { value: "dark-red", label: "Dark Red" },
  { value: "emerald", label: "Emerald" },
];

const fontSizes = {
  small: "14px",
  medium: "16px",
  large: "18px",
};

onMounted(() => {
  if (props.token) {
    tokenInput.value = props.token;
  }
  themeMode.value = getStoredThemeModePreference();
  appPalette.value = getStoredAppPalettePreference();
  
  const savedFontSize = localStorage.getItem("gitswamp-font-size");
  if (savedFontSize && (savedFontSize === "small" || savedFontSize === "medium" || savedFontSize === "large")) {
    fontSize.value = savedFontSize;
  }
  const savedCompact = localStorage.getItem("gitswamp-compact-mode");
  if (savedCompact) {
    compactMode.value = savedCompact === "true";
  }
  const savedAvatars = localStorage.getItem("gitswamp-show-avatars");
  if (savedAvatars !== null) {
    showAvatars.value = savedAvatars !== "false";
  }
  
  applySettings();
});

function applySettings() {
  document.documentElement.style.setProperty("--font-size", fontSizes[fontSize.value]);
  document.documentElement.classList.toggle("compact", compactMode.value);
  document.documentElement.classList.toggle("hide-avatars", !showAvatars.value);
}

watch([fontSize, compactMode, showAvatars], () => {
  applySettings();
  localStorage.setItem("gitswamp-font-size", fontSize.value);
  localStorage.setItem("gitswamp-compact-mode", String(compactMode.value));
  localStorage.setItem("gitswamp-show-avatars", String(showAvatars.value));
});

watch(themeMode, (value) => {
  applyThemeModePreference(value);
  storeThemeModePreference(value);
});

watch(appPalette, (value) => {
  applyAppPalettePreference(value);
  storeAppPalettePreference(value);
});

function toggleThemeMode() {
  themeMode.value = themeMode.value === "dark" ? "light" : "dark";
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
    <div class="w-[480px] max-h-[90vh] bg-[var(--card)] border border-[var(--border)] rounded-xl shadow-2xl overflow-hidden flex flex-col">
      <div class="flex items-center justify-between px-5 py-4 border-b border-[var(--border)] flex-shrink-0">
        <div class="flex items-center gap-2">
          <Shield class="w-4 h-4 text-[var(--primary)]" />
          <h2 class="text-sm font-semibold text-[var(--foreground)]">Settings</h2>
        </div>
        <button @click="emit('close')" class="p-1 rounded hover:bg-[var(--secondary)] transition-colors">
          <X class="w-4 h-4 text-[var(--muted-foreground)]" />
        </button>
      </div>

      <div class="p-5 space-y-4 overflow-y-auto flex-1">
        <div class="space-y-3">
          <div class="flex items-center gap-2 text-[10px] font-semibold text-[var(--muted-foreground)] uppercase tracking-wider">
            <Monitor class="w-3 h-3" />
            Appearance
          </div>
          
          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Theme</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Dark or light mode</p>
            </div>
            <button
              @click="toggleThemeMode"
              class="relative w-14 h-7 rounded-full transition-colors duration-300 flex-shrink-0"
              :class="themeMode === 'dark' ? 'bg-[var(--accent)]' : 'bg-[var(--primary)]'"
            >
              <div
                class="absolute top-0.5 w-6 h-6 rounded-full bg-white shadow-md flex items-center justify-center transition-all duration-300"
                :class="themeMode === 'dark' ? 'left-0.5' : 'left-[calc(100%-1.625rem)]'"
              >
                <Moon v-if="themeMode === 'dark'" class="w-3.5 h-3.5 text-[var(--accent)]" />
                <Sun v-else class="w-3.5 h-3.5 text-[var(--primary)]" />
              </div>
            </button>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Theme Preset</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Colors for app UI and code diff</p>
            </div>
            <div class="flex gap-1.5 flex-wrap justify-end max-w-[250px]">
              <button
                v-for="option in appPaletteOptions"
                :key="option.value"
                @click="appPalette = option.value"
                class="px-2.5 py-1.5 text-[10px] rounded transition-colors"
                :class="appPalette === option.value
                  ? 'bg-[var(--primary)] text-white'
                  : 'bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
              >
                {{ option.label }}
              </button>
            </div>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Font Size</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Adjust text size</p>
            </div>
            <div class="flex gap-1">
              <button
                v-for="size in ['small', 'medium', 'large'] as const"
                :key="size"
                @click="fontSize = size"
                class="px-3 py-1.5 text-[10px] rounded transition-colors capitalize"
                :class="fontSize === size 
                  ? 'bg-[var(--primary)] text-white' 
                  : 'bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
              >
                {{ size }}
              </button>
            </div>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Compact Mode</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Reduce spacing in commit list</p>
            </div>
            <button
              @click="compactMode = !compactMode"
              class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
              :class="compactMode ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all duration-200"
                :class="compactMode ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'"
              />
            </button>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Show Avatars</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Display author avatars in graph</p>
            </div>
            <button
              @click="showAvatars = !showAvatars"
              class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
              :class="showAvatars ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all duration-200"
                :class="showAvatars ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'"
              />
            </button>
          </div>
        </div>

        <div class="border-t border-[var(--border)] pt-4 space-y-3">
          <div class="flex items-center gap-2 text-[10px] font-semibold text-[var(--muted-foreground)] uppercase tracking-wider">
            <Shield class="w-3 h-3" />
            Authentication
          </div>
          
          <div>
            <div class="text-xs font-medium text-[var(--foreground)] mb-1.5 block">GitHub Personal Access Token</div>
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
        </div>

        <div class="border-t border-[var(--border)] pt-4">
          <div class="flex items-center gap-2 text-[10px] font-semibold text-[var(--muted-foreground)] uppercase tracking-wider mb-3">
            <Layout class="w-3 h-3" />
            System
          </div>
          
          <div>
            <div class="text-xs font-medium text-[var(--foreground)] mb-1.5 block">Git Executable</div>
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
  </div>
</template>


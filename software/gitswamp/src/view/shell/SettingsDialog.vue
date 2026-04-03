<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { Shield, Eye, EyeOff, Check, Trash2, X, Layout, Monitor, Sun, Moon } from "lucide-vue-next";
import AppButton from "@/shared/ui/AppButton.vue";
import {
  APP_THEME_OPTIONS,
  applyAppPalettePreference,
  applyThemeModePreference,
  getStoredAppPalettePreference,
  getStoredThemeModePreference,
  storeAppPalettePreference,
  storeThemeModePreference,
  type AppThemeOption,
  type AppPalettePreference,
  type ThemeModePreference,
} from "@/shared/themePreferences";
import {
  getStoredCommitAnalyzerSettings,
  updateCommitAnalyzerSettings,
} from "@/shared/config/commitAnalyzerPreferences";

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
const dummyMode = ref(false);
const showAvatars = ref(true);
const restoreSession = ref(true);
const reducedMotion = ref(false);
const wrapDiffLines = ref(false);
const showDiffLineNumbers = ref(true);
const notifyGitkeep = ref(true);
const commitAnalyzerEnabled = ref(true);

const appThemeOptions = APP_THEME_OPTIONS;
const darkThemeOptions = appThemeOptions.filter((theme) => theme.group === "dark");
const lightThemeOptions = appThemeOptions.filter((theme) => theme.group === "light");

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
  const savedDummyMode = localStorage.getItem("gitswamp-dummy-mode");
  if (savedDummyMode) {
    dummyMode.value = savedDummyMode === "true";
  }
  const savedAvatars = localStorage.getItem("gitswamp-show-avatars");
  if (savedAvatars !== null) {
    showAvatars.value = savedAvatars !== "false";
  }
  const savedRestoreSession = localStorage.getItem("gitswamp-restore-session");
  if (savedRestoreSession !== null) {
    restoreSession.value = savedRestoreSession !== "false";
  }
  const savedReducedMotion = localStorage.getItem("gitswamp-reduced-motion");
  if (savedReducedMotion !== null) {
    reducedMotion.value = savedReducedMotion === "true";
  }
  const savedWrapDiffLines = localStorage.getItem("gitswamp-wrap-diff-lines");
  if (savedWrapDiffLines !== null) {
    wrapDiffLines.value = savedWrapDiffLines === "true";
  }
  const savedShowDiffLineNumbers = localStorage.getItem("gitswamp-show-diff-line-numbers");
  if (savedShowDiffLineNumbers !== null) {
    showDiffLineNumbers.value = savedShowDiffLineNumbers !== "false";
  }
  const savedNotifyGitkeep = localStorage.getItem("gitswamp-notify-gitkeep");
  if (savedNotifyGitkeep === null) {
    localStorage.setItem("gitswamp-notify-gitkeep", "true");
  } else {
    notifyGitkeep.value = savedNotifyGitkeep !== "false";
  }

  commitAnalyzerEnabled.value = getStoredCommitAnalyzerSettings().enabled;
  
  applySettings();
});

function applySettings() {
  document.documentElement.style.setProperty("--font-size", fontSizes[fontSize.value]);
  document.documentElement.classList.toggle("compact", compactMode.value);
  document.documentElement.classList.toggle("dummy-mode", dummyMode.value);
  document.documentElement.classList.toggle("hide-avatars", !showAvatars.value);
  document.documentElement.classList.toggle("reduced-motion", reducedMotion.value);
  document.documentElement.classList.toggle("diff-wrap-lines", wrapDiffLines.value);
  document.documentElement.classList.toggle("hide-diff-line-numbers", !showDiffLineNumbers.value);
}

watch([fontSize, compactMode, dummyMode, showAvatars, restoreSession, reducedMotion, wrapDiffLines, showDiffLineNumbers, notifyGitkeep], () => {
  applySettings();
  localStorage.setItem("gitswamp-font-size", fontSize.value);
  localStorage.setItem("gitswamp-compact-mode", String(compactMode.value));
  localStorage.setItem("gitswamp-dummy-mode", String(dummyMode.value));
  localStorage.setItem("gitswamp-show-avatars", String(showAvatars.value));
  localStorage.setItem("gitswamp-restore-session", String(restoreSession.value));
  localStorage.setItem("gitswamp-reduced-motion", String(reducedMotion.value));
  localStorage.setItem("gitswamp-wrap-diff-lines", String(wrapDiffLines.value));
  localStorage.setItem("gitswamp-show-diff-line-numbers", String(showDiffLineNumbers.value));
  localStorage.setItem("gitswamp-notify-gitkeep", String(notifyGitkeep.value));
});

watch(themeMode, (value) => {
  applyThemeModePreference(value);
  storeThemeModePreference(value);
});

watch(commitAnalyzerEnabled, (value) => {
  updateCommitAnalyzerSettings({ enabled: value });
});

watch(appPalette, (value) => {
  applyAppPalettePreference(value);
  storeAppPalettePreference(value);
});

function onThemePresetChange(event: Event) {
  const selected = (event.target as HTMLSelectElement).value as AppPalettePreference;
  const selectedTheme = appThemeOptions.find((theme) => theme.id === selected);
  if (!selectedTheme) return;

  appPalette.value = selectedTheme.id;
  if (themeMode.value !== selectedTheme.mode) {
    themeMode.value = selectedTheme.mode;
  }
}

function themeLabel(theme: AppThemeOption): string {
  return theme.mode === "dark" ? `${theme.label} (Dark)` : `${theme.label} (Light)`;
}

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
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Grouped dark/light themes, auto-switches mode</p>
            </div>
            <select
              :value="appPalette"
              @change="onThemePresetChange"
              class="w-[220px] px-2.5 py-1.5 text-[11px] rounded bg-[var(--input-background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]"
            >
              <optgroup label="Dark Themes">
                <option v-for="theme in darkThemeOptions" :key="theme.id" :value="theme.id">
                  {{ themeLabel(theme) }}
                </option>
              </optgroup>
              <optgroup label="Light Themes">
                <option v-for="theme in lightThemeOptions" :key="theme.id" :value="theme.id">
                  {{ themeLabel(theme) }}
                </option>
              </optgroup>
            </select>
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
              <div class="text-xs font-medium text-[var(--foreground)] block">Dummy Mode</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Show extra beginner descriptions in right-click, View and hamburger menus</p>
            </div>
            <button
              @click="dummyMode = !dummyMode"
              class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
              :class="dummyMode ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all duration-200"
                :class="dummyMode ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'"
              />
            </button>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Show Avatars</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Show detailed avatars, or colored circles when off</p>
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

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Restore Session on Start</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Reopen last active tabs and repository</p>
            </div>
            <button
              @click="restoreSession = !restoreSession"
              class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
              :class="restoreSession ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all duration-200"
                :class="restoreSession ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'"
              />
            </button>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Reduced Motion</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Disable graph and UI animations</p>
            </div>
            <button
              @click="reducedMotion = !reducedMotion"
              class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
              :class="reducedMotion ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all duration-200"
                :class="reducedMotion ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'"
              />
            </button>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Wrap Long Diff Lines</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Wrap long lines instead of horizontal scrolling</p>
            </div>
            <button
              @click="wrapDiffLines = !wrapDiffLines"
              class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
              :class="wrapDiffLines ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all duration-200"
                :class="wrapDiffLines ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'"
              />
            </button>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Show Diff Line Numbers</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Display old/new line numbers in diff view</p>
            </div>
            <button
              @click="showDiffLineNumbers = !showDiffLineNumbers"
              class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
              :class="showDiffLineNumbers ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all duration-200"
                :class="showDiffLineNumbers ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'"
              />
            </button>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Notify for .gitkeep Need</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Warn when empty folders are detected and offer one-click .gitkeep</p>
            </div>
            <button
              @click="notifyGitkeep = !notifyGitkeep"
              class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
              :class="notifyGitkeep ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all duration-200"
                :class="notifyGitkeep ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'"
              />
            </button>
          </div>

          <div class="flex items-center justify-between py-2">
            <div>
              <div class="text-xs font-medium text-[var(--foreground)] block">Commit Analyzer</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Realtime commit quality analysis near message counter (~1 MB RAM)</p>
            </div>
            <button
              @click="commitAnalyzerEnabled = !commitAnalyzerEnabled"
              class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
              :class="commitAnalyzerEnabled ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all duration-200"
                :class="commitAnalyzerEnabled ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'"
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


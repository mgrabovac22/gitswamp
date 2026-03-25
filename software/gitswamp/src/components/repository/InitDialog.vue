<script setup lang="ts">
import { ref, computed } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  X,
  Monitor,
  Github,
  GitBranch,
  Loader2,
  ArrowLeft,
} from "lucide-vue-next";

const props = defineProps<{
  visible: boolean;
  providerTokens?: Record<string, string | null>;
}>();

const emit = defineEmits<{
  close: [];
  init: [path: string, branchName: string];
  saveProviderToken: [provider: string, token: string];
}>();

const sources = [
  { id: "local", label: "Local Only", icon: Monitor, color: "#10b981", desc: "Initialize locally" },
  { id: "github", label: "GitHub", icon: Github, color: "#ffffff", desc: "Push to GitHub" },
  { id: "github-enterprise", label: "GitHub Enterprise", icon: Github, color: "#6e7681", desc: "Enterprise server" },
  { id: "gitlab", label: "GitLab", icon: GitBranch, color: "#fc6d26", desc: "Push to GitLab" },
  { id: "gitlab-self", label: "GitLab Self-Hosted", icon: GitBranch, color: "#e24329", desc: "Self-managed" },
  { id: "bitbucket", label: "Bitbucket", icon: GitBranch, color: "#0052cc", desc: "Push to Bitbucket" },
  { id: "bitbucket-dc", label: "Bitbucket DC", icon: GitBranch, color: "#2684ff", desc: "Data Center" },
  { id: "azure", label: "Azure DevOps", icon: GitBranch, color: "#0078d4", desc: "Push to Azure" },
] as const;

const providerNames: Record<string, string> = {
  "github": "GitHub", "github-enterprise": "GitHub Enterprise",
  "gitlab": "GitLab", "gitlab-self": "GitLab Self-Hosted",
  "bitbucket": "Bitbucket", "bitbucket-dc": "Bitbucket DC",
  "azure": "Azure DevOps",
};

const gitignoreTemplates = [
  "None", "Node", "Python", "Rust", "Java", "Go", "C++", "C#", "Ruby",
  "Swift", "Kotlin", "Unity", "Unreal", "VisualStudio", "JetBrains",
];

const licenseOptions = [
  "None", "MIT", "Apache-2.0", "GPL-3.0", "BSD-2-Clause", "BSD-3-Clause",
  "ISC", "MPL-2.0", "LGPL-3.0", "AGPL-3.0", "Unlicense",
];

const activeSource = ref<string | null>(null);
const repoName = ref("");
const initPath = ref("C:\\Repozitoriji");
const branchName = ref("main");
const gitignoreTemplate = ref("None");
const licenseTemplate = ref("None");
const initWithLfs = ref(false);
const repoVisibility = ref<"public" | "private">("private");
const initError = ref<string | null>(null);
const initializing = ref(false);

const showGrid = computed(() => activeSource.value === null);

function isProviderConnected(provider: string): boolean {
  if (provider === "local") return true;
  return !!props.providerTokens?.[provider];
}

function isProviderSupported(provider: string): boolean {
  // Only local and GitHub are supported for init
  return provider === "local" || provider === "github";
}

function selectSource(id: string) {
  if (!isProviderSupported(id)) {
    return; // Don't allow unsupported providers
  }
  activeSource.value = id;
}

function backToGrid() {
  activeSource.value = null;
}

const fullPath = computed(() => {
  const base = initPath.value.replace(/[/\\]$/, "");
  return repoName.value ? `${base}\\${repoName.value}` : `${base}\\`;
});

async function browsePath() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "Select folder to initialize in",
    });
    if (selected) {
      initPath.value = selected as string;
    }
  } catch {}
}

function onInit() {
  if (!repoName.value.trim() || !initPath.value.trim()) return;
  initializing.value = true;
  initError.value = null;
  emit("init", fullPath.value, branchName.value.trim() || "main");
}
</script>

<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close')">
    <div class="w-[720px] bg-[var(--popover)] rounded-xl border border-[var(--border)] shadow-2xl flex flex-col overflow-hidden" :style="showGrid ? 'height: auto' : 'height: 520px'">
      <div class="flex items-center justify-between px-5 py-3.5 border-b border-[var(--border)]">
        <div class="flex items-center gap-2">
          <button v-if="!showGrid" @click="backToGrid" class="p-1 rounded hover:bg-[var(--secondary)] transition-colors mr-1">
            <ArrowLeft class="w-4 h-4 text-[var(--muted-foreground)]" />
          </button>
          <span class="text-sm font-semibold text-[var(--foreground)]">Initialize a Repository</span>
        </div>
        <button @click="emit('close')" class="p-1 rounded hover:bg-[var(--secondary)] transition-colors">
          <X class="w-4 h-4 text-[var(--muted-foreground)]" />
        </button>
      </div>

      <div v-if="showGrid" class="p-6">
        <p class="text-xs text-[var(--muted-foreground)] mb-4">Choose where to create your repository</p>
        <div class="grid grid-cols-4 gap-3">
          <button
            v-for="src in sources"
            :key="src.id"
            @click="selectSource(src.id)"
            :disabled="!isProviderSupported(src.id)"
            :class="[
              'flex flex-col items-center gap-2.5 px-3 py-4 rounded-xl border transition-all group',
              isProviderSupported(src.id)
                ? 'border-[var(--border)] hover:border-[var(--primary)]/40 hover:bg-[var(--primary)]/5 cursor-pointer'
                : 'border-[var(--border)]/50 opacity-50 cursor-not-allowed'
            ]"
          >
            <div class="w-10 h-10 rounded-xl flex items-center justify-center transition-all" :class="isProviderSupported(src.id) ? 'group-hover:scale-110' : ''" :style="{ backgroundColor: src.color + '15' }">
              <component :is="src.icon" class="w-5 h-5" :style="{ color: src.color }" />
            </div>
            <div class="text-center">
              <span class="text-[11px] font-semibold text-[var(--foreground)] block">{{ src.label }}</span>
              <span v-if="isProviderSupported(src.id)" class="text-[9px] text-[var(--muted-foreground)] mt-0.5 block">{{ src.desc }}</span>
              <span v-else class="text-[9px] text-[#f59e0b] mt-0.5 block font-medium">To be continued</span>
            </div>
          </button>
        </div>
      </div>

      <div v-if="!showGrid" class="flex-1 flex overflow-hidden">
        <div class="w-44 border-r border-[var(--border)] bg-[var(--sidebar-background)] py-2 flex-shrink-0 overflow-y-auto">
          <button
            v-for="src in sources"
            :key="src.id"
            @click="selectSource(src.id)"
            :disabled="!isProviderSupported(src.id)"
            :class="[
              'w-full flex items-center gap-2.5 px-3 py-2 text-left transition-colors text-[11px]',
              !isProviderSupported(src.id)
                ? 'opacity-50 cursor-not-allowed'
                : activeSource === src.id
                ? 'bg-[var(--primary)]/15 text-[var(--primary)] font-semibold'
                : 'text-[var(--sidebar-foreground)] hover:bg-[var(--sidebar-accent)]'
            ]"
          >
            <component :is="src.icon" class="w-3.5 h-3.5 flex-shrink-0" :style="{ color: src.color }" />
            <span class="truncate">{{ src.label }}</span>
            <div v-if="isProviderSupported(src.id) && src.id !== 'local' && isProviderConnected(src.id)" class="w-1.5 h-1.5 rounded-full bg-[#10b981] flex-shrink-0 ml-auto"></div>
            <span v-if="!isProviderSupported(src.id)" class="text-[9px] text-[#f59e0b] font-medium ml-auto flex-shrink-0">TBC</span>
          </button>
        </div>

        <div class="flex-1 p-5 flex flex-col overflow-y-auto">
          <div v-if="activeSource && activeSource !== 'local' && !isProviderConnected(activeSource)" class="mb-4 p-3 rounded-lg border border-[#f59e0b]/30 bg-[#f59e0b]/10">
            <div class="text-xs text-[#f59e0b] font-medium mb-1">{{ providerNames[activeSource] }} is not connected</div>
            <p class="text-[10px] text-[var(--muted-foreground)]">The repository will be created locally. Connect in Clone dialog or Settings to push automatically.</p>
          </div>

          <div class="space-y-3.5 flex-1">
            <div class="flex items-center gap-3">
              <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">Name</label>
              <input
                v-model="repoName"
                placeholder="my-project"
                class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                autofocus
              />
            </div>

            <div class="flex items-center gap-3">
              <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">Initialize in</label>
              <div class="flex-1 flex gap-2">
                <input
                  v-model="initPath"
                  class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                />
                <button
                  @click="browsePath"
                  class="px-3 py-2 bg-[var(--secondary)] hover:opacity-80 text-xs text-[var(--foreground)] rounded border border-[var(--border)] transition-colors"
                >
                  Browse
                </button>
              </div>
            </div>

            <div class="flex items-center gap-3">
              <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">Full path</label>
              <span class="text-xs text-[var(--muted-foreground)] font-mono truncate">{{ fullPath }}</span>
            </div>

            <div class="flex items-center gap-3">
              <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">Default branch</label>
              <input
                v-model="branchName"
                placeholder="main"
                class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              />
            </div>

            <div v-if="activeSource === 'github' || activeSource === 'github-enterprise'" class="flex items-center gap-3">
              <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">Visibility</label>
              <select
                v-model="repoVisibility"
                class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              >
                <option value="private">Private</option>
                <option value="public">Public</option>
              </select>
            </div>

            <div class="flex items-center gap-3">
              <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">.gitignore</label>
              <select
                v-model="gitignoreTemplate"
                class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              >
                <option v-for="t in gitignoreTemplates" :key="t" :value="t">{{ t }}</option>
              </select>
            </div>

            <div class="flex items-center gap-3">
              <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">License</label>
              <select
                v-model="licenseTemplate"
                class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              >
                <option v-for="l in licenseOptions" :key="l" :value="l">{{ l }}</option>
              </select>
            </div>

            <div class="flex items-center gap-3">
              <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0"></label>
              <label class="flex items-center gap-2 cursor-pointer text-xs text-[var(--muted-foreground)]">
                <input type="checkbox" v-model="initWithLfs" class="w-3.5 h-3.5 rounded border-[var(--border)] bg-[var(--background)] accent-[var(--primary)]" />
                Initialize with Git LFS
              </label>
            </div>

            <div v-if="initError" class="text-xs text-[#ef4444] bg-[#ef4444]/10 rounded px-3 py-2 border border-[#ef4444]/20">
              {{ initError }}
            </div>
          </div>

          <div class="flex justify-end mt-4 flex-shrink-0">
            <button
              @click="onInit"
              :disabled="!repoName.trim() || !initPath.trim() || initializing"
              class="px-5 py-2 bg-[var(--primary)] hover:opacity-90 text-xs text-white font-medium rounded-lg border border-[var(--primary)] transition-colors disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-2"
            >
              <Loader2 v-if="initializing" class="w-3.5 h-3.5 animate-spin" />
              Create Repository
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


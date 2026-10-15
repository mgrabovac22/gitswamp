<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  Monitor,
  Github,
  GitBranch,
  Loader2,
  ArrowLeft,
  Key,
} from "lucide-vue-next";
import AzureDevOpsIcon from "@/shared/ui/AzureDevOpsIcon.vue";
import BitbucketIcon from "@/shared/ui/BitbucketIcon.vue";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";

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
  { id: "bitbucket", label: "Bitbucket", icon: BitbucketIcon, color: "#0052cc", desc: "Push to Bitbucket" },
  { id: "bitbucket-dc", label: "Bitbucket DC", icon: BitbucketIcon, color: "#2684ff", desc: "Data Center" },
  { id: "azure", label: "Azure DevOps", icon: AzureDevOpsIcon, color: "#0078d4", desc: "Push to Azure" },
] as const;

const providerNames: Record<string, string> = {
  "github": "GitHub", "github-enterprise": "GitHub Enterprise",
  "gitlab": "GitLab", "gitlab-self": "GitLab Self-Hosted",
  "bitbucket": "Bitbucket", "bitbucket-dc": "Bitbucket DC",
  "azure": "Azure DevOps",
};

const tokenInstructions: Record<string, string> = {
  "github": "Go to GitHub → Settings → Developer settings → Personal access tokens → Generate new token with 'repo' scope.",
  "github-enterprise": "Go to your GitHub Enterprise → Settings → Developer settings → Personal access tokens.",
  "gitlab": "Go to GitLab → Preferences → Access Tokens → Create a token with 'api' and 'read_repository' scopes.",
  "gitlab-self": "Go to your GitLab instance → Preferences → Access Tokens.",
  "bitbucket": "Bitbucket API token: Bitbucket → Personal settings → API tokens → Create token. Enter your Bitbucket username with the token, or the token alone.",
  "bitbucket-dc": "Go to your Bitbucket DC → Manage account → HTTP access tokens → Create token.",
  "azure": "Go to Azure DevOps → User settings → Personal access tokens → New Token with 'Code: Read & write' scope. Host example: dev.azure.com/myorg",
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
const initPath = ref(String.raw`C:\Repozitoriji`);
const branchName = ref("main");
const gitignoreTemplate = ref("None");
const licenseTemplate = ref("None");
const initWithLfs = ref(false);
const repoVisibility = ref<"public" | "private">("private");
const initError = ref<string | null>(null);
const initializing = ref(false);

const tokenInput = ref("");
const showConnectForm = ref(false);
const connectError = ref<string | null>(null);
const gitlabDomain = ref("");
const githubEnterpriseDomain = ref("");
const bitbucketDcDomain = ref("");
const azureDomain = ref("");
const bitbucketUsernameInput = ref("");

const showGrid = computed(() => activeSource.value === null);

const selfHostedProviders = new Set(["github-enterprise", "gitlab-self", "bitbucket-dc"]);

function isProviderSupported(provider: string): boolean {
  return provider === "local" || provider in providerNames;
}

function normalizeHostInput(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";

  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  try {
    const parsed = new URL(candidate);
    return parsed.host.toLowerCase();
  } catch {
    const withoutProtocol = trimmed.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "");
    return (withoutProtocol.split("/")[0] || "").trim().toLowerCase();
  }
}

function normalizeAzureDomainInput(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";

  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  try {
    const parsed = new URL(candidate);
    const host = parsed.host.toLowerCase();
    const pathSegments = parsed.pathname.split("/");
    const firstPathSegment = pathSegments.find((segment) => segment.length > 0) || "";
    if (host === "dev.azure.com") {
      const organization = firstPathSegment;
      return organization ? `${host}/${organization.toLowerCase()}` : host;
    }
    return host;
  } catch {
    const withoutProtocol = trimmed.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "");
    const firstPart = withoutProtocol.split("/");
    const hostPart = firstPart.find((segment) => segment.trim().length > 0) || "";
    if (!hostPart) return "";
    const hostPartLower = hostPart.toLowerCase();
    if (hostPartLower === "dev.azure.com") {
      const orgPart = firstPart.find((segment, index) => index > 0 && segment.trim().length > 0) || "";
      return orgPart ? `${hostPartLower}/${orgPart.toLowerCase()}` : hostPartLower;
    }
    return hostPartLower;
  }
}

function getStoredAzureDomain(): string {
  return normalizeAzureDomainInput(props.providerTokens?.["azure-domain"] || "");
}

function getProviderToken(provider: string): string | null {
  const stored = props.providerTokens?.[provider] || null;
  if (stored && selfHostedProviders.has(provider)) {
    const separatorIndex = stored.indexOf("|");
    if (separatorIndex > 0) {
      const rawDomain = stored.slice(0, separatorIndex).trim();
      const token = stored.slice(separatorIndex + 1).trim();
      const normalizedDomain = normalizeHostInput(rawDomain);
      if (provider === "gitlab-self") {
        gitlabDomain.value = normalizedDomain;
      } else if (provider === "github-enterprise") {
        githubEnterpriseDomain.value = normalizedDomain;
      } else {
        bitbucketDcDomain.value = normalizedDomain;
      }
      return token || null;
    }
  }
  return stored;
}

function isProviderConnected(provider: string): boolean {
  if (provider === "local") return true;
  const token = getProviderToken(provider);
  if (!token) return false;
  if (provider === "gitlab-self") {
    return !!normalizeHostInput(gitlabDomain.value);
  }
  if (provider === "github-enterprise") {
    return !!normalizeHostInput(githubEnterpriseDomain.value);
  }
  if (provider === "bitbucket-dc") {
    return !!normalizeHostInput(bitbucketDcDomain.value);
  }
  if (provider === "azure") {
    return !!normalizeAzureDomainInput(azureDomain.value || getStoredAzureDomain());
  }
  return true;
}

function isProviderDomainRequired(provider: string | null): boolean {
  return provider === "github-enterprise" || provider === "gitlab-self" || provider === "bitbucket-dc";
}

const needsConnection = computed(() => {
  if (!activeSource.value || activeSource.value === "local") return false;
  return !isProviderConnected(activeSource.value);
});

const requiresDomain = computed(() => isProviderDomainRequired(activeSource.value));

const connectedLabel = computed(() => {
  switch (activeSource.value) {
    case "github-enterprise":
      return githubEnterpriseDomain.value;
    case "gitlab-self":
      return gitlabDomain.value;
    case "bitbucket-dc":
      return bitbucketDcDomain.value;
    case "azure":
      return azureDomain.value || getStoredAzureDomain();
    default:
      return providerNames[activeSource.value || ""] || "";
  }
});

function storedBitbucketUsername(): string {
  const raw = getProviderToken("bitbucket") || "";
  const idx = raw.indexOf(":");
  if (idx <= 0) return "";
  const user = raw.slice(0, idx).trim();
  if (!user || raw.slice(idx + 1).length === 0) return "";
  return user;
}

function seedDomainFor(src: string | null): void {
  if (src === "github-enterprise" && !githubEnterpriseDomain.value) {
    const stored = props.providerTokens?.["github-enterprise"] || "";
    githubEnterpriseDomain.value = normalizeHostInput(stored.split("|")[0] || "");
  }
  if (src === "gitlab-self" && !gitlabDomain.value) {
    const stored = props.providerTokens?.["gitlab-self"] || "";
    gitlabDomain.value = normalizeHostInput(stored.split("|")[0] || "");
  }
  if (src === "bitbucket-dc" && !bitbucketDcDomain.value) {
    const stored = props.providerTokens?.["bitbucket-dc"] || "";
    bitbucketDcDomain.value = normalizeHostInput(stored.split("|")[0] || "");
  }
  if (src === "azure" && !azureDomain.value) {
    azureDomain.value = getStoredAzureDomain();
  }
}

function resetConnectState(): void {
  showConnectForm.value = false;
  tokenInput.value = "";
  connectError.value = null;
  bitbucketUsernameInput.value = storedBitbucketUsername();
}

function selectSource(id: string) {
  if (!isProviderSupported(id)) return;
  activeSource.value = id;
  resetConnectState();
  seedDomainFor(id);
}

function backToGrid() {
  activeSource.value = null;
  resetConnectState();
}

function startConnect() {
  showConnectForm.value = true;
  tokenInput.value = "";
  connectError.value = null;
  seedDomainFor(activeSource.value);
}

async function saveToken() {
  const provider = activeSource.value;
  const token = tokenInput.value.trim();
  if (!provider || !token) return;
  connectError.value = null;

  try {
    if (provider === "github-enterprise") {
      const domain = normalizeHostInput(githubEnterpriseDomain.value);
      if (!domain) {
        connectError.value = "Valid GitHub Enterprise domain is required (e.g. github.example.com)";
        return;
      }
      await invoke<string>("verify_github_enterprise_token", { domain, token });
      githubEnterpriseDomain.value = domain;
      emit("saveProviderToken", provider, `${domain}|${token}`);
    } else if (provider === "gitlab-self") {
      const domain = normalizeHostInput(gitlabDomain.value);
      if (!domain) {
        connectError.value = "Valid GitLab domain is required (e.g. gitlab.example.com)";
        return;
      }
      await invoke<string>("verify_gitlab_token", { domain, token });
      gitlabDomain.value = domain;
      emit("saveProviderToken", provider, `${domain}|${token}`);
    } else if (provider === "bitbucket-dc") {
      const domain = normalizeHostInput(bitbucketDcDomain.value);
      if (!domain) {
        connectError.value = "Valid Bitbucket Data Center domain is required (e.g. bitbucket.example.com)";
        return;
      }
      await invoke<string>("verify_bitbucket_dc_token", { domain, token });
      bitbucketDcDomain.value = domain;
      emit("saveProviderToken", provider, `${domain}|${token}`);
    } else if (provider === "azure") {
      const domain = normalizeAzureDomainInput(azureDomain.value);
      if (!domain) {
        connectError.value = "Valid Azure host domain is required (e.g. dev.azure.com/myorg)";
        return;
      }
      emit("saveProviderToken", "azure", token);
      emit("saveProviderToken", "azure-domain", domain);
      azureDomain.value = domain;
    } else if (provider === "bitbucket") {
      const username = bitbucketUsernameInput.value.trim();
      emit("saveProviderToken", provider, username ? `${username}:${token}` : token);
    } else {
      emit("saveProviderToken", provider, token);
    }
  } catch (e) {
    connectError.value = `Connection failed: ${e}`;
    return;
  }

  showConnectForm.value = false;
  tokenInput.value = "";
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

watch(
  () => props.visible,
  (visible) => {
    if (visible) {
      bitbucketUsernameInput.value = storedBitbucketUsername();
      seedDomainFor(activeSource.value);
    } else {
      activeSource.value = null;
      resetConnectState();
      initError.value = null;
      initializing.value = false;
    }
  },
);
</script>

<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close')">
    <div class="w-[720px] bg-[var(--popover)] rounded-xl border border-[var(--border)] shadow-2xl flex flex-col overflow-hidden" :style="showGrid ? 'height: auto' : 'height: 540px'">
      <div class="flex items-center justify-between px-5 py-3.5 border-b border-[var(--border)]">
        <div class="flex items-center gap-2">
          <button v-if="!showGrid" @click="backToGrid" class="p-1 rounded hover:bg-[var(--secondary)] transition-colors mr-1">
            <ArrowLeft class="w-4 h-4 text-[var(--muted-foreground)]" />
          </button>
          <span class="text-sm font-semibold text-[var(--foreground)]">Initialize a Repository</span>
        </div>
        <CloseIconButton title="Close init dialog" @click="emit('close')" />
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
              <span class="text-[9px] text-[var(--muted-foreground)] mt-0.5 block">{{ src.desc }}</span>
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
            <div v-if="src.id !== 'local' && isProviderConnected(src.id)" class="w-1.5 h-1.5 rounded-full bg-[#10b981] flex-shrink-0 ml-auto"></div>
          </button>
        </div>

        <div class="flex-1 p-5 flex flex-col overflow-y-auto">
          <template v-if="activeSource && activeSource !== 'local'">
            <div v-if="needsConnection && !showConnectForm" class="mb-4 p-3 rounded-lg border border-[#f59e0b]/30 bg-[#f59e0b]/10">
              <div class="text-xs text-[#f59e0b] font-medium mb-1">{{ providerNames[activeSource] }} is not connected</div>
              <p class="text-[10px] text-[var(--muted-foreground)] mb-2">Connect to push to this provider later. Without a connection the repository is created locally only.</p>
              <button @click="startConnect" class="px-3 py-1.5 bg-[var(--primary)] text-white text-[11px] font-medium rounded hover:opacity-90 transition-colors flex items-center gap-1.5">
                <Key class="w-3 h-3" />
                Connect to {{ providerNames[activeSource] }}
              </button>
            </div>

            <div v-else-if="needsConnection && showConnectForm" class="mb-4 p-3 rounded-lg border border-[var(--border)] bg-[var(--secondary)]/40 space-y-3">
              <div class="text-xs text-[var(--foreground)] font-medium">Connect to {{ providerNames[activeSource] }}</div>

              <div v-if="requiresDomain" class="flex items-center gap-3">
                <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">Domain</label>
                <input
                  v-if="activeSource === 'github-enterprise'"
                  v-model="githubEnterpriseDomain"
                  placeholder="github.example.com"
                  class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                />
                <input
                  v-else-if="activeSource === 'gitlab-self'"
                  v-model="gitlabDomain"
                  placeholder="gitlab.example.com"
                  class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                />
                <input
                  v-else
                  v-model="bitbucketDcDomain"
                  placeholder="bitbucket.example.com"
                  class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                />
              </div>

              <div v-if="activeSource === 'azure'" class="flex items-center gap-3">
                <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">Host</label>
                <input
                  v-model="azureDomain"
                  placeholder="dev.azure.com/myorg"
                  class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                />
              </div>

              <div v-if="activeSource === 'bitbucket'" class="flex items-center gap-3">
                <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">Username</label>
                <input
                  v-model="bitbucketUsernameInput"
                  placeholder="Optional (for app passwords)"
                  class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                />
              </div>

              <div class="flex items-center gap-3">
                <label class="text-xs text-[var(--muted-foreground)] w-28 text-right flex-shrink-0">Token</label>
                <input
                  v-model="tokenInput"
                  type="password"
                  placeholder="Personal access token..."
                  class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  @keyup.enter="saveToken"
                />
              </div>

              <div v-if="connectError" class="text-[10px] text-[#ef4444] px-2">{{ connectError }}</div>

              <div class="flex justify-end gap-2">
                <button @click="showConnectForm = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
                <button @click="saveToken" :disabled="!tokenInput.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Connect</button>
              </div>
              <p class="text-[9px] text-[var(--muted-foreground)]">{{ tokenInstructions[activeSource] }}</p>
            </div>

            <div v-else class="mb-4 flex items-center gap-2 px-1">
              <div class="w-2 h-2 rounded-full bg-[#10b981]"></div>
              <span class="text-[10px] text-[#10b981] truncate">Connected to {{ connectedLabel }}</span>
            </div>
          </template>

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

            <div v-if="activeSource && activeSource !== 'local'" class="flex items-center gap-3">
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

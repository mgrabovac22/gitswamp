<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  X,
  Globe,
  Github,
  GitBranch,
  Loader2,
  Lock,
  Star,
  ArrowLeft,
  Key,
} from "lucide-vue-next";
import type { GithubRepo, GitlabRepo } from "@/types";

const props = defineProps<{
  visible: boolean;
  token?: string | null;
  providerTokens?: Record<string, string | null>;
}>();

const emit = defineEmits<{
  close: [];
  clone: [url: string, path: string, shallow: boolean, done?: (ok: boolean, error?: string) => void];
  searchGithub: [query: string];
  saveProviderToken: [provider: string, token: string];
}>();

const sources = [
  { id: "url", label: "With URL", icon: Globe, color: "#8b5cf6", desc: "Any git repository" },
  { id: "github", label: "GitHub", icon: Github, color: "#ffffff", desc: "Browse your repos" },
  { id: "github-enterprise", label: "GitHub Enterprise", icon: Github, color: "#6e7681", desc: "Enterprise server" },
  { id: "gitlab", label: "GitLab", icon: GitBranch, color: "#fc6d26", desc: "GitLab.com repos" },
  { id: "gitlab-self", label: "GitLab Self-Hosted", icon: GitBranch, color: "#e24329", desc: "Self-managed instance" },
  { id: "bitbucket", label: "Bitbucket", icon: GitBranch, color: "#0052cc", desc: "Bitbucket.org repos" },
  { id: "bitbucket-dc", label: "Bitbucket DC", icon: GitBranch, color: "#2684ff", desc: "Data Center repos" },
  { id: "azure", label: "Azure DevOps", icon: GitBranch, color: "#0078d4", desc: "Azure repos" },
] as const;

const providerNames: Record<string, string> = {
  "github": "GitHub",
  "github-enterprise": "GitHub Enterprise",
  "gitlab": "GitLab",
  "gitlab-self": "GitLab Self-Hosted",
  "bitbucket": "Bitbucket",
  "bitbucket-dc": "Bitbucket DC",
  "azure": "Azure DevOps",
};

const providerUrlHints: Record<string, string> = {
  "github": "https://github.com/user/repo.git",
  "github-enterprise": "https://github.example.com/user/repo.git",
  "gitlab": "https://gitlab.com/user/repo.git",
  "gitlab-self": "https://gitlab.example.com/user/repo.git",
  "bitbucket": "https://bitbucket.org/user/repo.git",
  "bitbucket-dc": "https://bitbucket.example.com/scm/project/repo.git",
  "azure": "https://dev.azure.com/org/project/_git/repo",
};

const tokenInstructions: Record<string, string> = {
  "github": "Go to GitHub → Settings → Developer settings → Personal access tokens → Generate new token with 'repo' scope.",
  "github-enterprise": "Go to your GitHub Enterprise → Settings → Developer settings → Personal access tokens.",
  "gitlab": "Go to GitLab → Preferences → Access Tokens → Create a token with 'read_api' and 'read_repository' scopes.",
  "gitlab-self": "Go to your GitLab instance → Preferences → Access Tokens.",
  "bitbucket": "Go to Bitbucket → Personal settings → App passwords → Create with 'Repositories: Read' permission.",
  "bitbucket-dc": "Go to your Bitbucket DC → Manage account → HTTP access tokens → Create token.",
  "azure": "Go to Azure DevOps → User settings → Personal access tokens → New Token with 'Code: Read' scope.",
};

const activeSource = ref<string | null>(null);
const clonePath = ref("C:\\Repozitoriji");
const cloneUrl = ref("");
const shallowClone = ref(false);
const cloning = ref(false);
const cloneError = ref<string | null>(null);

const tokenInput = ref("");
const showTokenInput = ref(false);
const gitlabDomain = ref(""); // For self-hosted GitLab

const githubSearch = ref("");
const githubRepos = ref<GithubRepo[]>([]);
const githubLoading = ref(false);
const githubError = ref<string | null>(null);
let searchTimer: ReturnType<typeof setTimeout> | null = null;

const gitlabSearch = ref("");
const gitlabRepos = ref<GitlabRepo[]>([]);
const gitlabLoading = ref(false);
const gitlabError = ref<string | null>(null);
const gitlabSshKeyGenerated = ref(false);
const gitlabPublicKey = ref("");
const gitlabSearchInput = ref<HTMLInputElement | null>(null);

const showGrid = computed(() => activeSource.value === null);

function getProviderToken(provider: string): string | null {
  if (provider === "github") return props.token || null;
  const stored = props.providerTokens?.[provider] || null;
  if (provider === "gitlab-self" && stored && stored.includes("|")) {
    const parts = stored.split("|");
    gitlabDomain.value = parts[0];
    return parts[1];
  }
  return stored;
}

function isProviderConnected(provider: string): boolean {
  return !!getProviderToken(provider);
}

function isProviderFullySupported(provider: string): boolean {
  // Clone supports URL, GitHub, and GitLab (all variants)
  return provider === "url" || provider === "github" || provider === "github-enterprise" || provider === "gitlab" || provider === "gitlab-self";
}

const needsConnection = computed(() => {
  if (!activeSource.value || activeSource.value === "url") return false;
  return !isProviderConnected(activeSource.value);
});

function selectSource(id: string) {
  activeSource.value = id;
  showTokenInput.value = false;
  tokenInput.value = "";
}

function backToGrid() {
  activeSource.value = null;
  showTokenInput.value = false;
  tokenInput.value = "";
}

function startTokenConnect() {
  showTokenInput.value = true;
  tokenInput.value = "";
}

async function saveToken() {
  if (!tokenInput.value.trim() || !activeSource.value) return;
  
  if (activeSource.value === "gitlab-self" && gitlabDomain.value.trim()) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke<string>("verify_gitlab_token", {
        domain: gitlabDomain.value.trim(),
        token: tokenInput.value.trim(),
      });
      emit("saveProviderToken", activeSource.value, `${gitlabDomain.value.trim()}|${tokenInput.value.trim()}`);
    } catch (e) {
      gitlabError.value = `Token verification failed: ${e}`;
      return;
    }
  } else {
    emit("saveProviderToken", activeSource.value, tokenInput.value.trim());
  }
  
  showTokenInput.value = false;
  tokenInput.value = "";
}

async function browsePath() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "Select folder to clone into",
    });
    if (selected) {
      clonePath.value = selected as string;
    }
  } catch {}
}

async function onClone() {
  if (!cloneUrl.value.trim() || !clonePath.value.trim()) return;
  cloning.value = true;
  cloneError.value = null;
  emit(
    "clone",
    cloneUrl.value.trim(),
    clonePath.value.trim(),
    shallowClone.value,
    (ok: boolean, error?: string) => {
      cloning.value = false;
      if (!ok) {
        cloneError.value = error || "Clone failed.";
      }
    }
  );
}

function copyGitlabPublicKey() {
  window.navigator.clipboard.writeText(gitlabPublicKey.value);
}

function selectRepo(repo: GithubRepo) {
  cloneUrl.value = repo.clone_url;
}

function onGithubSearch() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    doGithubSearch();
  }, 400);
}

async function doGithubSearch() {
  if (!props.token) {
    githubError.value = "No GitHub token. Add one in Settings first.";
    return;
  }
  githubLoading.value = true;
  githubError.value = null;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    githubRepos.value = await invoke<GithubRepo[]>("search_github_repos", {
      token: props.token,
      query: githubSearch.value,
    });
  } catch (e) {
    githubError.value = String(e);
    githubRepos.value = [];
  } finally {
    githubLoading.value = false;
  }
}

function onGitlabSearch() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    doGitlabSearch();
  }, 400);
}

async function doGitlabSearch() {
  const token = getProviderToken(activeSource.value || "");
  if (!token || !gitlabDomain.value) {
    gitlabError.value = "Domain and token required";
    return;
  }
  gitlabLoading.value = true;
  gitlabError.value = null;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    gitlabRepos.value = await invoke<GitlabRepo[]>("search_gitlab_repos", {
      domain: gitlabDomain.value,
      token: token,
      query: gitlabSearch.value,
    });
  } catch (e) {
    gitlabError.value = String(e);
    gitlabRepos.value = [];
  } finally {
    gitlabLoading.value = false;
  }
}

function selectGitlabRepo(repo: GitlabRepo) {
  cloneUrl.value = repo.clone_url_https;
}

async function generateSshKey() {
  const token = getProviderToken(activeSource.value || "");
  if (!token || !gitlabDomain.value) return;
  
  gitlabError.value = null;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const [_, pubKey] = await invoke<[string, string]>("generate_ssh_key", {
      email: "gitswamp@local",
      keyName: `gitswamp_${gitlabDomain.value.replace(/\./g, "_")}`,
    });
    gitlabPublicKey.value = pubKey;
    
    try {
      await invoke("add_gitlab_ssh_key", {
        domain: gitlabDomain.value,
        token: token,
        title: "GitSwamp SSH Key",
        key: pubKey,
      });
      gitlabSshKeyGenerated.value = true;
    } catch (addErr) {
      const msg = String(addErr || "");
      if (msg.includes("already exists") || msg.includes("has already been taken")) {
        gitlabSshKeyGenerated.value = true;
        gitlabError.value = null;
        return;
      }
      gitlabError.value = `Key generated. Add it manually to GitLab SSH keys: ${addErr}`;
    }
  } catch (e) {
    gitlabError.value = `SSH key error: ${e}`;
  }
}

watch(activeSource, (src) => {
  if (src === "github" && githubRepos.value.length === 0 && props.token) {
    doGithubSearch();
  }
  if (src === "gitlab-self" && !needsConnection.value && gitlabRepos.value.length === 0) {
    doGitlabSearch();
  }
  if (src !== "gitlab-self") {
    gitlabRepos.value = [];
    gitlabSearch.value = "";
    gitlabPublicKey.value = "";
    gitlabSshKeyGenerated.value = false;
  }
});

watch(needsConnection, (needs) => {
  if (!needs && activeSource.value === "gitlab-self" && gitlabRepos.value.length === 0) {
    doGitlabSearch();
  }
});

const isUrlMode = computed(() => activeSource.value === "url");
const isGithubMode = computed(() => activeSource.value === "github");
const isGitlabSelfMode = computed(() => activeSource.value === "gitlab-self");

const unimplementedProviders = ["github-enterprise", "gitlab", "bitbucket", "bitbucket-dc", "azure"];
const isUnimplemented = computed(() => activeSource.value && unimplementedProviders.includes(activeSource.value));
</script>

<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close')">
    <div class="w-[680px] bg-[var(--popover)] rounded-xl border border-[var(--border)] shadow-2xl flex flex-col overflow-hidden" :style="showGrid ? 'height: auto' : 'height: 500px'">
      <div class="flex items-center justify-between px-5 py-3.5 border-b border-[var(--border)]">
        <div class="flex items-center gap-2">
          <button v-if="!showGrid" @click="backToGrid" class="p-1 rounded hover:bg-[var(--secondary)] transition-colors mr-1">
            <ArrowLeft class="w-4 h-4 text-[var(--muted-foreground)]" />
          </button>
          <span class="text-sm font-semibold text-[var(--foreground)]">Clone a Repository</span>
        </div>
        <button @click="emit('close')" class="p-1 rounded hover:bg-[var(--secondary)] transition-colors">
          <X class="w-4 h-4 text-[var(--muted-foreground)]" />
        </button>
      </div>

      <div v-if="showGrid" class="p-6">
        <p class="text-xs text-[var(--muted-foreground)] mb-4">Choose a source to clone from</p>
        <div class="grid grid-cols-4 gap-3">
          <button
            v-for="src in sources"
            :key="src.id"
            @click="selectSource(src.id)"
            :disabled="!isProviderFullySupported(src.id)"
            :class="[
              'flex flex-col items-center gap-2.5 px-3 py-4 rounded-xl border transition-all group',
              isProviderFullySupported(src.id)
                ? 'border-[var(--border)] hover:border-[var(--primary)]/40 hover:bg-[var(--primary)]/5 cursor-pointer'
                : 'border-[var(--border)]/50 opacity-50 cursor-not-allowed'
            ]"
          >
            <div class="w-10 h-10 rounded-xl flex items-center justify-center transition-all" :class="isProviderFullySupported(src.id) ? 'group-hover:scale-110' : ''" :style="{ backgroundColor: src.color + '15' }">
              <component :is="src.icon" class="w-5 h-5" :style="{ color: src.color }" />
            </div>
            <div class="text-center">
              <span class="text-[11px] font-semibold text-[var(--foreground)] block">{{ src.label }}</span>
              <span v-if="isProviderFullySupported(src.id)" class="text-[9px] text-[var(--muted-foreground)] mt-0.5 block">{{ src.desc }}</span>
              <span v-else class="text-[9px] text-[#f59e0b] mt-0.5 block font-medium">To be continued</span>
            </div>
          </button>
        </div>
      </div>

      <div v-if="!showGrid" class="flex-1 p-5 flex flex-col overflow-hidden">
        <div class="space-y-3 flex-1 overflow-hidden flex flex-col">
          <div class="flex items-center gap-3 flex-shrink-0">
            <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">Clone to</label>
            <div class="flex-1 flex gap-2">
              <input
                v-model="clonePath"
                class="flex-1 px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              />
              <button
                @click="browsePath"
                class="px-3 py-1.5 bg-[var(--secondary)] hover:opacity-80 text-xs text-[var(--foreground)] rounded border border-[var(--border)] transition-colors"
              >
                Browse
              </button>
            </div>
          </div>

          <div v-if="isUrlMode" class="flex items-center gap-3 flex-shrink-0">
            <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">URL</label>
            <input
              v-model="cloneUrl"
              placeholder="https://github.com/user/repo.git"
              class="flex-1 px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
              autofocus
            />
          </div>

          <template v-if="isGithubMode">
            <div class="flex items-center gap-3 flex-shrink-0">
              <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">Search</label>
              <div class="flex-1 relative">
                <input
                  v-model="githubSearch"
                  @input="onGithubSearch"
                  placeholder="Search your repositories..."
                  class="w-full px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  autofocus
                />
                <Loader2 v-if="githubLoading" class="absolute right-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 animate-spin text-[var(--muted-foreground)]" />
              </div>
            </div>

            <div v-if="githubError" class="text-[10px] text-[#ef4444] px-2 flex-shrink-0">{{ githubError }}</div>

            <div class="flex-1 overflow-y-auto min-h-0 border border-[var(--border)] rounded-lg bg-[var(--background)]">
              <div v-if="!token" class="flex items-center justify-center h-full text-xs text-[var(--muted-foreground)] p-4 text-center">
                No GitHub token configured.<br/>Go to Settings and add a Personal Access Token first.
              </div>
              <div v-else-if="!githubRepos.length && !githubLoading" class="flex items-center justify-center h-full text-xs text-[var(--muted-foreground)]">
                {{ githubSearch ? 'No repos found' : 'Loading your repos...' }}
              </div>
              <button
                v-for="repo in githubRepos"
                :key="repo.full_name"
                @click="selectRepo(repo)"
                :class="[
                  'w-full text-left px-3 py-2.5 border-b border-[var(--border)] hover:bg-[var(--primary)]/10 transition-colors',
                  cloneUrl === repo.clone_url ? 'bg-[var(--primary)]/15' : ''
                ]"
              >
                <div class="flex items-center gap-2">
                  <Lock v-if="repo.is_private" class="w-3 h-3 text-[#f59e0b] flex-shrink-0" />
                  <Github v-else class="w-3 h-3 text-[var(--muted-foreground)] flex-shrink-0" />
                  <span class="text-xs text-[var(--foreground)] font-medium truncate">{{ repo.full_name }}</span>
                  <span v-if="repo.stars > 0" class="flex items-center gap-0.5 text-[9px] text-[#f59e0b] flex-shrink-0 ml-auto">
                    <Star class="w-2.5 h-2.5" />{{ repo.stars }}
                  </span>
                </div>
                <p v-if="repo.description" class="text-[10px] text-[var(--muted-foreground)] truncate mt-0.5 pl-5">{{ repo.description }}</p>
              </button>
            </div>

            <div v-if="cloneUrl" class="flex items-center gap-3 flex-shrink-0">
              <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">URL</label>
              <div class="flex-1 px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-[10px] font-mono text-[var(--primary)] truncate">
                {{ cloneUrl }}
              </div>
            </div>
          </template>

          <template v-if="isGitlabSelfMode">
            <div v-if="needsConnection && !showTokenInput" class="flex flex-col items-center justify-center py-8 gap-4">
              <div class="w-16 h-16 rounded-2xl flex items-center justify-center bg-[#e24329]/20">
                <GitBranch class="w-8 h-8 text-[#e24329]" />
              </div>
              <div class="text-sm text-[var(--foreground)] font-medium">Connect to GitLab Self-Hosted</div>
              <p class="text-[10px] text-[var(--muted-foreground)] text-center max-w-xs leading-relaxed">
                Enter your GitLab instance domain and personal access token
              </p>
              <button @click="startTokenConnect" class="px-4 py-2 bg-[var(--primary)] text-white text-xs font-medium rounded-lg hover:opacity-90 transition-colors flex items-center gap-2">
                <Key class="w-3.5 h-3.5" />
                Connect to GitLab
              </button>
            </div>

            <div v-if="needsConnection && showTokenInput" class="space-y-3">
              <div class="flex items-center gap-3 flex-shrink-0">
                <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">Domain</label>
                <input
                  v-model="gitlabDomain"
                  placeholder="gitlab.example.com"
                  class="flex-1 px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  autofocus
                />
              </div>
              <div class="flex items-center gap-3 flex-shrink-0">
                <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">Token</label>
                <input
                  v-model="tokenInput"
                  type="password"
                  placeholder="Personal access token..."
                  class="flex-1 px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  @keyup.enter="saveToken"
                />
              </div>
              <div v-if="gitlabError" class="text-[10px] text-[#ef4444] px-2 flex-shrink-0">{{ gitlabError }}</div>
              <div class="flex justify-end gap-2 mt-2">
                <button @click="showTokenInput = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
                <button @click="saveToken" :disabled="!tokenInput.trim() || !gitlabDomain.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Connect</button>
              </div>
              <p class="text-[9px] text-[var(--muted-foreground)] px-2">
                Go to your GitLab → User Settings → Access Tokens → Create token with 'read_api' and 'read_repository' scopes.
              </p>
            </div>

            <template v-if="!needsConnection">
              <div class="flex items-center gap-3 flex-shrink-0">
                <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">Search</label>
                <div class="flex-1 relative">
                  <input
                    ref="gitlabSearchInput"
                    v-model="gitlabSearch"
                    @input="onGitlabSearch"
                    placeholder="Search your repositories..."
                    class="w-full px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <Loader2 v-if="gitlabLoading" class="absolute right-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 animate-spin text-[var(--muted-foreground)]" />
                </div>
              </div>

              <div class="flex items-center gap-2 px-2 flex-shrink-0">
                <div class="w-2 h-2 rounded-full bg-[#10b981]"></div>
                <span class="text-[10px] text-[#10b981]">Connected to {{ gitlabDomain }}</span>
                <button v-if="!gitlabSshKeyGenerated && !gitlabPublicKey" @click="generateSshKey" class="ml-auto text-[10px] text-[var(--primary)] hover:underline flex items-center gap-1">
                  <Key class="w-3 h-3" />
                  Generate SSH Key
                </button>
                <span v-else-if="gitlabSshKeyGenerated" class="ml-auto text-[10px] text-[#10b981] flex items-center gap-1">
                  <Key class="w-3 h-3" />
                  SSH Key Added to GitLab
                </span>
              </div>

              <div v-if="gitlabPublicKey && !gitlabSshKeyGenerated" class="px-2 flex-shrink-0 space-y-1">
                <div class="text-[10px] text-[var(--muted-foreground)]">Add this SSH key to your GitLab settings:</div>
                <div class="relative">
                  <textarea
                    :value="gitlabPublicKey"
                    readonly
                    class="w-full h-16 px-2 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-[9px] font-mono text-[var(--foreground)] resize-none focus:outline-none"
                  />
                  <button 
                    @click="copyGitlabPublicKey"
                    class="absolute top-1 right-1 px-2 py-0.5 text-[9px] bg-[var(--primary)] text-white rounded hover:opacity-90"
                  >Copy</button>
                </div>
              </div>

              <div v-if="gitlabError" class="text-[10px] text-[#ef4444] px-2 flex-shrink-0">{{ gitlabError }}</div>

              <div class="flex-1 overflow-y-auto min-h-0 border border-[var(--border)] rounded-lg bg-[var(--background)]">
                <div v-if="!gitlabRepos.length && !gitlabLoading" class="flex items-center justify-center h-full text-xs text-[var(--muted-foreground)]">
                  {{ gitlabSearch ? 'No repos found' : 'Loading your projects...' }}
                </div>
                <button
                  v-for="repo in gitlabRepos"
                  :key="repo.path_with_namespace"
                  @click="selectGitlabRepo(repo)"
                  :class="[
                    'w-full text-left px-3 py-2.5 border-b border-[var(--border)] hover:bg-[var(--primary)]/10 transition-colors',
                    cloneUrl === repo.clone_url_https ? 'bg-[var(--primary)]/15' : ''
                  ]"
                >
                  <div class="flex items-center gap-2">
                    <Lock v-if="repo.is_private" class="w-3 h-3 text-[#f59e0b] flex-shrink-0" />
                    <GitBranch v-else class="w-3 h-3 text-[#e24329] flex-shrink-0" />
                    <span class="text-xs text-[var(--foreground)] font-medium truncate">{{ repo.full_name }}</span>
                    <span v-if="repo.stars > 0" class="flex items-center gap-0.5 text-[9px] text-[#f59e0b] flex-shrink-0 ml-auto">
                      <Star class="w-2.5 h-2.5" />{{ repo.stars }}
                    </span>
                  </div>
                  <p v-if="repo.description" class="text-[10px] text-[var(--muted-foreground)] truncate mt-0.5 pl-5">{{ repo.description }}</p>
                </button>
              </div>

              <div v-if="cloneUrl" class="flex items-center gap-3 flex-shrink-0">
                <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">URL</label>
                <div class="flex-1 px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-[10px] font-mono text-[var(--primary)] truncate">
                  {{ cloneUrl }}
                </div>
              </div>
            </template>
          </template>

          <template v-if="isUnimplemented">
            <div class="flex flex-col items-center justify-center py-12 gap-4">
              <div class="w-20 h-20 rounded-2xl flex items-center justify-center" :style="{ backgroundColor: sources.find(s => s.id === activeSource)?.color + '15' }">
                <component :is="sources.find(s => s.id === activeSource)?.icon" class="w-10 h-10" :style="{ color: sources.find(s => s.id === activeSource)?.color }" />
              </div>
              <div class="text-lg text-[var(--foreground)] font-semibold">{{ providerNames[activeSource!] }}</div>
              <div class="px-4 py-2 bg-[var(--primary)]/10 text-[var(--primary)] text-xs font-medium rounded-full border border-[var(--primary)]/30">
                Coming Soon
              </div>
              <p class="text-[11px] text-[var(--muted-foreground)] text-center max-w-xs leading-relaxed mt-2">
                Support for {{ providerNames[activeSource!] }} is being developed.<br/>
                In the meantime, you can use <strong>Clone with URL</strong> to clone any repository.
              </p>
              <button @click="selectSource('url')" class="mt-2 px-4 py-2 bg-[var(--secondary)] text-[var(--foreground)] text-xs font-medium rounded-lg hover:bg-[var(--secondary)]/80 transition-colors">
                Use URL Instead
              </button>
            </div>
          </template>

          <template v-if="!isUrlMode && !isGithubMode && !isGitlabSelfMode && !isUnimplemented && activeSource">
            <div v-if="needsConnection && !showTokenInput" class="flex flex-col items-center justify-center py-8 gap-4">
              <div class="w-16 h-16 rounded-2xl flex items-center justify-center" :style="{ backgroundColor: sources.find(s => s.id === activeSource)?.color + '20' }">
                <component :is="sources.find(s => s.id === activeSource)?.icon" class="w-8 h-8" :style="{ color: sources.find(s => s.id === activeSource)?.color }" />
              </div>
              <div class="text-sm text-[var(--foreground)] font-medium">{{ providerNames[activeSource] }} is not connected</div>
              <p class="text-[10px] text-[var(--muted-foreground)] text-center max-w-xs leading-relaxed">
                {{ tokenInstructions[activeSource] }}
              </p>
              <button @click="startTokenConnect" class="px-4 py-2 bg-[var(--primary)] text-white text-xs font-medium rounded-lg hover:opacity-90 transition-colors flex items-center gap-2">
                <Key class="w-3.5 h-3.5" />
                Connect to {{ providerNames[activeSource] }}
              </button>
            </div>

            <div v-if="needsConnection && showTokenInput" class="space-y-3">
              <div class="flex items-center gap-3 flex-shrink-0">
                <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">Token</label>
                <input
                  v-model="tokenInput"
                  type="password"
                  placeholder="Paste your personal access token..."
                  class="flex-1 px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  @keyup.enter="saveToken"
                  autofocus
                />
              </div>
              <div class="flex justify-end gap-2 mt-2">
                <button @click="showTokenInput = false" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
                <button @click="saveToken" :disabled="!tokenInput.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Save Token</button>
              </div>
              <p class="text-[9px] text-[var(--muted-foreground)] px-2">
                {{ tokenInstructions[activeSource] }}
              </p>
            </div>

            <div v-if="!needsConnection" class="flex items-center gap-3 flex-shrink-0">
              <label class="text-xs text-[var(--muted-foreground)] w-20 text-right flex-shrink-0">URL</label>
              <input
                v-model="cloneUrl"
                :placeholder="providerUrlHints[activeSource] || 'Repository URL'"
                class="flex-1 px-3 py-1.5 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                autofocus
              />
            </div>
            <div v-if="!needsConnection" class="flex items-center gap-2 px-2">
              <div class="w-2 h-2 rounded-full bg-[#10b981]"></div>
              <span class="text-[10px] text-[#10b981]">Connected to {{ providerNames[activeSource] }}</span>
            </div>
          </template>

          <div class="flex items-center gap-6 flex-shrink-0">
            <label class="flex items-center gap-2 cursor-pointer text-xs text-[var(--muted-foreground)]">
              <input type="checkbox" v-model="shallowClone" class="w-3.5 h-3.5 rounded border-[var(--border)] bg-[var(--background)] accent-[var(--primary)]" />
              Shallow clone
            </label>
          </div>

          <div v-if="cloneError" class="text-xs text-[#ef4444] bg-[#ef4444]/10 rounded px-3 py-2 border border-[#ef4444]/20 flex-shrink-0">
            {{ cloneError }}
          </div>
        </div>

        <div class="flex justify-end mt-4 flex-shrink-0">
          <button
            @click="onClone"
            :disabled="!cloneUrl.trim() || !clonePath.trim() || cloning"
            class="px-5 py-2 bg-[var(--primary)] hover:opacity-90 text-xs text-white font-medium rounded-lg transition-colors disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-2"
          >
            <Loader2 v-if="cloning" class="w-3.5 h-3.5 animate-spin" />
            Clone Repository
          </button>
        </div>
      </div>
    </div>
  </div>
</template>


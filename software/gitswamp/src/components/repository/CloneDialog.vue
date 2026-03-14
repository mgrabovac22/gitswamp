<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  X,
  Globe,
  Github,
  GitBranch,
  FolderOpen,
  Loader2,
  Lock,
  Star,
} from "lucide-vue-next";
import type { GithubRepo } from "@/types";

const props = defineProps<{
  visible: boolean;
  token?: string | null;
}>();

const emit = defineEmits<{
  close: [];
  clone: [url: string, path: string, shallow: boolean];
  searchGithub: [query: string];
}>();

const sources = [
  { id: "url", label: "Clone with URL", icon: Globe },
  { id: "github", label: "GitHub.com", icon: Github },
  { id: "github-enterprise", label: "GitHub Enterprise Server", icon: Github },
  { id: "gitlab", label: "GitLab.com", icon: GitBranch },
  { id: "gitlab-self", label: "GitLab (Self-Managed)", icon: GitBranch },
  { id: "bitbucket", label: "Bitbucket.org", icon: GitBranch },
  { id: "bitbucket-dc", label: "Bitbucket Data Center", icon: GitBranch },
  { id: "azure", label: "Azure DevOps", icon: GitBranch },
] as const;

const activeSource = ref("url");
const clonePath = ref("C:\\Repozitoriji");
const cloneUrl = ref("");
const shallowClone = ref(false);
const sparseCheckout = ref(false);
const cloning = ref(false);
const cloneError = ref<string | null>(null);

// GitHub search state
const githubSearch = ref("");
const githubRepos = ref<GithubRepo[]>([]);
const githubLoading = ref(false);
const githubError = ref<string | null>(null);
let searchTimer: ReturnType<typeof setTimeout> | null = null;

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
  emit("clone", cloneUrl.value.trim(), clonePath.value.trim(), shallowClone.value);
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

// Auto-load repos when switching to github source
watch(activeSource, (src) => {
  if (src === "github" && githubRepos.value.length === 0 && props.token) {
    doGithubSearch();
  }
});

const isUrlMode = computed(() => activeSource.value === "url");
const isGithubMode = computed(() => activeSource.value === "github");
</script>

<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60" @click.self="emit('close')">
    <div class="w-[700px] h-[500px] bg-[#1c2130] rounded-lg border border-[#8b5cf6]/20 shadow-2xl flex flex-col overflow-hidden">
      <!-- Title bar -->
      <div class="flex items-center justify-between px-4 py-3 border-b border-[#8b5cf6]/15 bg-[#151921]">
        <span class="text-sm font-medium text-[#e2e8f0]">Clone a Repository</span>
        <button @click="emit('close')" class="p-1 rounded hover:bg-[#252b3d] transition-colors">
          <X class="w-4 h-4 text-[#64748b]" />
        </button>
      </div>

      <div class="flex flex-1 overflow-hidden">
        <!-- Left sidebar -->
        <div class="w-48 bg-[#151921] border-r border-[#8b5cf6]/10 overflow-y-auto flex-shrink-0">
          <button
            v-for="src in sources"
            :key="src.id"
            @click="activeSource = src.id"
            :class="[
              'w-full flex items-center gap-2 px-4 py-2.5 text-xs transition-colors text-left',
              activeSource === src.id
                ? 'bg-[#8b5cf6]/15 text-[#a78bfa] font-medium'
                : 'text-[#94a3b8] hover:text-[#e2e8f0] hover:bg-[#1c2130]',
            ]"
          >
            <component :is="src.icon" class="w-3.5 h-3.5 flex-shrink-0" />
            <span class="truncate">{{ src.label }}</span>
          </button>
        </div>

        <!-- Right content -->
        <div class="flex-1 p-5 flex flex-col overflow-hidden">
          <h3 class="text-base font-medium text-[#e2e8f0] mb-4">Clone a Repo</h3>

          <div class="space-y-3 flex-1 overflow-hidden flex flex-col">
            <!-- Where to clone to -->
            <div class="flex items-center gap-3 flex-shrink-0">
              <label class="text-xs text-[#94a3b8] w-28 text-right flex-shrink-0">Clone to</label>
              <div class="flex-1 flex gap-2">
                <input
                  v-model="clonePath"
                  class="flex-1 px-3 py-1.5 bg-[#0d1017] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40"
                />
                <button
                  @click="browsePath"
                  class="px-3 py-1.5 bg-[#252b3d] hover:bg-[#2d3548] text-xs text-[#e2e8f0] rounded border border-[#8b5cf6]/20 transition-colors"
                >
                  Browse
                </button>
              </div>
            </div>

            <!-- URL input for url mode -->
            <div v-if="isUrlMode" class="flex items-center gap-3 flex-shrink-0">
              <label class="text-xs text-[#94a3b8] w-28 text-right flex-shrink-0">URL</label>
              <input
                v-model="cloneUrl"
                placeholder="https://github.com/user/repo.git"
                class="flex-1 px-3 py-1.5 bg-[#0d1017] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] placeholder:text-[#475569] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40"
              />
            </div>

            <!-- GitHub search mode -->
            <template v-if="isGithubMode">
              <div class="flex items-center gap-3 flex-shrink-0">
                <label class="text-xs text-[#94a3b8] w-28 text-right flex-shrink-0">Search</label>
                <div class="flex-1 relative">
                  <input
                    v-model="githubSearch"
                    @input="onGithubSearch"
                    placeholder="Search your repositories..."
                    class="w-full px-3 py-1.5 bg-[#0d1017] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] placeholder:text-[#475569] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40"
                  />
                  <Loader2 v-if="githubLoading" class="absolute right-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 animate-spin text-[#64748b]" />
                </div>
              </div>

              <div v-if="githubError" class="text-[10px] text-[#ef4444] px-2 flex-shrink-0">{{ githubError }}</div>

              <!-- Repo list -->
              <div class="flex-1 overflow-y-auto min-h-0 border border-[#8b5cf6]/10 rounded bg-[#0d1017]">
                <div v-if="!token" class="flex items-center justify-center h-full text-xs text-[#64748b] p-4 text-center">
                  No GitHub token configured.<br/>Go to Settings and add a Personal Access Token first.
                </div>
                <div v-else-if="!githubRepos.length && !githubLoading" class="flex items-center justify-center h-full text-xs text-[#64748b]">
                  {{ githubSearch ? 'No repos found' : 'Loading your repos...' }}
                </div>
                <button
                  v-for="repo in githubRepos"
                  :key="repo.full_name"
                  @click="selectRepo(repo)"
                  :class="[
                    'w-full text-left px-3 py-2 border-b border-[#8b5cf6]/5 hover:bg-[#8b5cf6]/10 transition-colors',
                    cloneUrl === repo.clone_url ? 'bg-[#8b5cf6]/15' : ''
                  ]"
                >
                  <div class="flex items-center gap-2">
                    <Lock v-if="repo.is_private" class="w-3 h-3 text-[#f59e0b] flex-shrink-0" />
                    <Github v-else class="w-3 h-3 text-[#64748b] flex-shrink-0" />
                    <span class="text-xs text-[#e2e8f0] font-medium truncate">{{ repo.full_name }}</span>
                    <span v-if="repo.stars > 0" class="flex items-center gap-0.5 text-[9px] text-[#f59e0b] flex-shrink-0 ml-auto">
                      <Star class="w-2.5 h-2.5" />{{ repo.stars }}
                    </span>
                  </div>
                  <p v-if="repo.description" class="text-[10px] text-[#64748b] truncate mt-0.5 pl-5">{{ repo.description }}</p>
                </button>
              </div>

              <!-- Selected URL display -->
              <div v-if="cloneUrl" class="flex items-center gap-3 flex-shrink-0">
                <label class="text-xs text-[#94a3b8] w-28 text-right flex-shrink-0">URL</label>
                <div class="flex-1 px-3 py-1.5 bg-[#0d1017] border border-[#8b5cf6]/15 rounded text-[10px] font-mono text-[#a78bfa] truncate">
                  {{ cloneUrl }}
                </div>
              </div>
            </template>

            <!-- Not-url, not-github mode -->
            <div v-if="!isUrlMode && !isGithubMode" class="flex items-center gap-3 flex-shrink-0">
              <label class="text-xs text-[#94a3b8] w-28 text-right flex-shrink-0">URL</label>
              <input
                v-model="cloneUrl"
                placeholder="Repository URL"
                class="flex-1 px-3 py-1.5 bg-[#0d1017] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] placeholder:text-[#475569] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40"
              />
            </div>

            <!-- Options -->
            <div class="flex items-center gap-6 flex-shrink-0">
              <label class="flex items-center gap-2 cursor-pointer text-xs text-[#94a3b8]">
                <input type="checkbox" v-model="shallowClone" class="w-3.5 h-3.5 rounded border-[#8b5cf6]/30 bg-[#0d1017] accent-[#8b5cf6]" />
                Shallow clone
              </label>
            </div>

            <!-- Error -->
            <div v-if="cloneError" class="text-xs text-[#ef4444] bg-[#ef4444]/10 rounded px-3 py-2 border border-[#ef4444]/20 flex-shrink-0">
              {{ cloneError }}
            </div>
          </div>

          <!-- Clone button -->
          <div class="flex justify-end mt-3 flex-shrink-0">
            <button
              @click="onClone"
              :disabled="!cloneUrl.trim() || !clonePath.trim() || cloning"
              class="px-5 py-2 bg-[#8b5cf6] hover:bg-[#7c3aed] text-xs text-white font-medium rounded transition-colors disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-2"
            >
              <Loader2 v-if="cloning" class="w-3.5 h-3.5 animate-spin" />
              Clone the repo!
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

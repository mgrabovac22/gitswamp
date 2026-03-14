<script setup lang="ts">
import { ref, computed } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  X,
  Globe,
  Github,
  GitBranch,
  FolderOpen,
  Loader2,
} from "lucide-vue-next";

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  close: [];
  clone: [url: string, path: string, shallow: boolean];
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

// For service-based sources, show different field
const isUrlMode = computed(() => activeSource.value === "url");
</script>

<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60" @click.self="emit('close')">
    <div class="w-[700px] h-[460px] bg-[#1c2130] rounded-lg border border-[#8b5cf6]/20 shadow-2xl flex flex-col overflow-hidden">
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
        <div class="flex-1 p-6 flex flex-col">
          <h3 class="text-base font-medium text-[#e2e8f0] mb-5">Clone a Repo</h3>

          <div class="space-y-4 flex-1">
            <!-- Where to clone to -->
            <div class="flex items-center gap-3">
              <label class="text-xs text-[#94a3b8] w-32 text-right flex-shrink-0">Where to clone to</label>
              <div class="flex-1 flex gap-2">
                <input
                  v-model="clonePath"
                  class="flex-1 px-3 py-2 bg-[#0d1017] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40"
                />
                <button
                  @click="browsePath"
                  class="px-3 py-2 bg-[#252b3d] hover:bg-[#2d3548] text-xs text-[#e2e8f0] rounded border border-[#8b5cf6]/20 transition-colors"
                >
                  Browse
                </button>
              </div>
            </div>

            <!-- URL / Search Remotes -->
            <div class="flex items-center gap-3">
              <label class="text-xs text-[#94a3b8] w-32 text-right flex-shrink-0">
                {{ isUrlMode ? 'URL' : 'Repository to clone' }}
              </label>
              <input
                v-model="cloneUrl"
                :placeholder="isUrlMode ? 'https://github.com/user/repo.git' : 'Search Remotes'"
                class="flex-1 px-3 py-2 bg-[#0d1017] border border-[#8b5cf6]/15 rounded text-xs text-[#e2e8f0] placeholder:text-[#475569] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40"
              />
            </div>

            <!-- Shallow Clone -->
            <div class="flex items-center gap-3">
              <label class="text-xs text-[#94a3b8] w-32 text-right flex-shrink-0">Shallow Clone</label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  v-model="shallowClone"
                  class="w-3.5 h-3.5 rounded border-[#8b5cf6]/30 bg-[#0d1017] accent-[#8b5cf6]"
                />
              </label>
            </div>

            <!-- Sparse Checkout -->
            <div class="flex items-center gap-3">
              <label class="text-xs text-[#94a3b8] w-32 text-right flex-shrink-0">Sparse Checkout</label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  v-model="sparseCheckout"
                  class="w-3.5 h-3.5 rounded border-[#8b5cf6]/30 bg-[#0d1017] accent-[#8b5cf6]"
                />
              </label>
            </div>

            <!-- Error -->
            <div v-if="cloneError" class="text-xs text-[#ef4444] bg-[#ef4444]/10 rounded px-3 py-2 border border-[#ef4444]/20">
              {{ cloneError }}
            </div>
          </div>

          <!-- Clone button -->
          <div class="flex justify-end mt-4">
            <button
              @click="onClone"
              :disabled="!cloneUrl.trim() || !clonePath.trim() || cloning"
              class="px-5 py-2 bg-[#252b3d] hover:bg-[#2d3548] text-xs text-[#e2e8f0] rounded border border-[#8b5cf6]/30 transition-colors disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-2"
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

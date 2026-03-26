<script setup lang="ts">
import { ref, computed } from "vue"
import { X, Github, GitBranch, Loader2 } from "lucide-vue-next"

const props = defineProps<{
  visible: boolean
  repoName: string
  availablePlatforms: Record<string, string | null>
  pushing: boolean
}>()

const emit = defineEmits<{
  close: []
  pushTo: [platform: string]
}>()

const selectedPlatform = ref<string | null>(null)

const availablePlatformsList = computed(() => {
  return Object.entries(props.availablePlatforms)
    .filter(([_, token]) => token !== null && token !== "")
    .map(([platform]) => {
      const platformNames: Record<string, string> = {
        github: "GitHub",
        "github-enterprise": "GitHub Enterprise",
        gitlab: "GitLab",
        "gitlab-self-hosted": "GitLab (Self-Hosted)",
        bitbucket: "Bitbucket",
        azure: "Azure DevOps",
      }
      return {
        id: platform,
        name: platformNames[platform] || platform,
        icon: platform === "github" || platform === "github-enterprise" ? Github : GitBranch,
      }
    })
})

function handlePush() {
  if (selectedPlatform.value) {
    emit("pushTo", selectedPlatform.value)
  }
}

function handleClose() {
  selectedPlatform.value = null
  emit("close")
}
</script>

<template>
  <div v-if="visible" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-slate-900 border border-slate-700 rounded-lg shadow-xl max-w-md w-full mx-4">
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-slate-700">
        <h2 class="text-lg font-semibold text-slate-100">Push to Platform</h2>
        <button
          @click="handleClose"
          class="p-1 hover:bg-slate-800 rounded transition-colors"
          :disabled="pushing"
        >
          <X :size="20" class="text-slate-400" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-4 space-y-4">
        <p class="text-slate-400 text-sm">
          No origin remote found. Select a platform to push <code class="bg-slate-800 px-2 py-1 rounded text-slate-300">{{ repoName }}</code>
        </p>

        <!-- Platform Selection -->
        <div v-if="availablePlatformsList.length > 0" class="space-y-2">
          <p class="text-slate-300 text-sm font-medium">Connected Platforms:</p>
          <div class="space-y-2">
            <button
              v-for="platform in availablePlatformsList"
              :key="platform.id"
              @click="selectedPlatform = platform.id"
              :class="[
                'w-full flex items-center gap-3 px-4 py-3 rounded border transition-colors',
                selectedPlatform === platform.id
                  ? 'border-blue-500 bg-blue-500/10 text-slate-100'
                  : 'border-slate-700 bg-slate-800/50 hover:border-slate-600 text-slate-300 hover:text-slate-100',
              ]"
              :disabled="pushing"
            >
              <component :is="platform.icon" :size="20" />
              <span>{{ platform.name }}</span>
            </button>
          </div>
        </div>

        <!-- No Platforms -->
        <div v-else class="text-center py-4">
          <p class="text-slate-400 text-sm">
            No platforms are connected. <br />
            <span class="text-slate-500">Please configure provider tokens in settings.</span>
          </p>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex gap-3 p-4 border-t border-slate-700">
        <button
          @click="handleClose"
          class="flex-1 px-4 py-2 text-slate-300 hover:text-slate-100 border border-slate-700 rounded hover:border-slate-600 transition-colors disabled:opacity-50"
          :disabled="pushing"
        >
          Cancel
        </button>
        <button
          @click="handlePush"
          class="flex-1 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          :disabled="!selectedPlatform || pushing"
        >
          <Loader2 v-if="pushing" :size="16" class="animate-spin" />
          <span>{{ pushing ? "Pushing..." : "Push" }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

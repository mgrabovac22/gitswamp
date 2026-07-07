<script setup lang="ts">
import {
  Upload,
  RefreshCw,
  Archive,
  Terminal,
  Settings,
  Loader2,
  Download,
} from "lucide-vue-next";
import { computed, onMounted, onUnmounted, ref } from "vue";
import AppButton from "@/shared/ui/AppButton.vue";
import BranchQuickActions from "@/view/shell/BranchQuickActions.vue";
import headerIcon from "@/assets/logo_git_croc.gif";
import headerTextLogoDark from "@/assets/logo_dark.png";
import headerTextLogoLight from "@/assets/logo_light.png";
import GitRpgShield from "@/features/repository/rpg/GitRpgShield.vue";
import type { GitRpgProfile } from "@/features/repository/rpg/gitRpgProfiler";

const props = defineProps<{
  loading: boolean;
  activeAction?: "pull" | "push" | "fetch" | null;
  ghostActive?: boolean;
  originConflictRisk?: {
    level: "none" | "medium" | "high";
    label: string;
  };
  rpgProfile?: GitRpgProfile | null;
  rpgLoading?: boolean;
}>();

const emit = defineEmits<{
  pull: [];
  push: [];
  fetch: [];
  branch: [];
  ghostBranch: [];
  materializeGhostBranch: [];
  discardGhostBranch: [];
  explainGitState: [];
  stash: [];
  terminal: [];
  settings: [];
}>();

const isLight = ref(document.documentElement.classList.contains("light"));
const textLogo = computed(() => (isLight.value ? headerTextLogoLight : headerTextLogoDark));
const rpgRole = computed(() => props.rpgProfile?.primaryRole || null);
const rpgShieldTitle = computed(() => {
  if (rpgRole.value) {
    return `${rpgRole.value.title}, click me`;
  }

  return props.rpgLoading ? "Git RPG profile loading, click me" : "Git RPG profile, click me";
});

const themeObserver = new MutationObserver(() => {
  isLight.value = document.documentElement.classList.contains("light");
});

onMounted(() => {
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["class"],
  });
});

onUnmounted(() => {
  themeObserver.disconnect();
});
</script>

<template>
  <div class="relative z-[40] h-12 bg-gradient-to-b from-[var(--header-bg)] to-[var(--secondary)] border-b border-[var(--border)] flex items-center justify-between px-2 shadow-lg flex-shrink-0">
    <div class="flex items-center gap-2">
      <div class="flex items-center gap-1.5">
        <div class="w-12 h-8 flex items-center justify-center">
          <img :src="headerIcon" alt="GitSwamp icon" class="w-full h-full object-contain" />
        </div>
        <h1 class="text-[var(--foreground)] font-bold text-sm tracking-tight overflow-hidden whitespace-nowrap">
          <div class="h-6 inline-flex items-center">
            <img :src="textLogo" alt="GitSwamp" class="h-6 w-auto object-contain" style="transform: translateY(4px);" />
          </div>
          <span class="sr-only">GitSwamp</span>
        </h1>
      </div>

      <div class="w-px h-6 bg-[var(--foreground)]/10" />

      <div class="flex items-center gap-1">
        <AppButton
          variant="ghost"
          size="sm"
          class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
          :disabled="loading"
          @click="emit('pull')"
        >
          <Loader2 v-if="loading && activeAction === 'pull'" class="w-3.5 h-3.5 animate-spin" />
          <Download v-else class="w-3.5 h-3.5" />
          Pull
        </AppButton>
        <AppButton
          variant="ghost"
          size="sm"
          class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
          :disabled="loading"
          @click="emit('push')"
        >
          <Loader2 v-if="loading && activeAction === 'push'" class="w-3.5 h-3.5 animate-spin" />
          <Upload v-else class="w-3.5 h-3.5" />
          Push
        </AppButton>
        <AppButton
          variant="ghost"
          size="sm"
          class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
          :disabled="loading"
          @click="emit('fetch')"
        >
          <Loader2 v-if="loading && activeAction === 'fetch'" class="w-3.5 h-3.5 animate-spin" />
          <RefreshCw v-else class="w-3.5 h-3.5" />
          Fetch
        </AppButton>
      </div>
    </div>

    <div class="flex items-center gap-1">
      <span
        v-if="originConflictRisk && originConflictRisk.level !== 'none'"
        class="px-2 py-0.5 rounded-full text-[10px] font-semibold border"
        :class="originConflictRisk.level === 'high'
          ? 'text-[#ef4444] border-[#ef4444]/50 bg-[#ef4444]/10'
          : 'text-[#f59e0b] border-[#f59e0b]/45 bg-[#f59e0b]/10'"
        :title="originConflictRisk.label"
      >
        {{ originConflictRisk.level === 'high' ? 'Conflict Risk' : 'Merge Warning' }}
      </span>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-8 w-8 px-0 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] transition-all"
        :title="rpgShieldTitle"
        @click="emit('explainGitState')"
      >
        <GitRpgShield :role="rpgRole" :loading="props.rpgLoading" size="header" />
      </AppButton>
      <BranchQuickActions
        :loading="loading"
        :ghost-active="!!ghostActive"
        @branch="emit('branch')"
        @ghost="emit('ghostBranch')"
        @materialize="emit('materializeGhostBranch')"
        @discard="emit('discardGhostBranch')"
      />
      <AppButton
        variant="ghost"
        size="sm"
        class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
        @click="emit('stash')"
      >
        <Archive class="w-3.5 h-3.5" />
        Stash
      </AppButton>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
        @click="emit('terminal')"
      >
        <Terminal class="w-3.5 h-3.5" />
        Terminal
      </AppButton>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-8 text-[var(--foreground)] hover:bg-[var(--header-hover)] hover:text-[var(--primary)] gap-1.5 transition-all text-xs"
        @click="emit('settings')"
      >
        <Settings class="w-3.5 h-3.5" />
      </AppButton>
    </div>
  </div>
</template>

<style scoped>
@keyframes fade-in-letter {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

</style>

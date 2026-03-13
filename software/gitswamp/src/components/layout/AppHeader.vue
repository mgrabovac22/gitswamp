<script setup lang="ts">
import {
  ArrowLeft,
  ArrowRight,
  Download,
  Upload,
  RefreshCw,
  GitBranch,
  Archive,
  GitPullRequest,
  Terminal,
  Settings,
  User,
} from "lucide-vue-next";
import AppButton from "@/components/ui/AppButton.vue";
</script>

<template>
  <div class="h-14 bg-gradient-to-b from-[#2e3e50] to-[#263342] border-b border-[#1e2e3e] flex items-center justify-between px-4 shadow-lg">
    <div class="flex items-center gap-4">
      <div class="flex items-center gap-2">
        <div class="w-8 h-8 bg-gradient-to-br from-[#4ecdc4] to-[#3aa89f] rounded-lg flex items-center justify-center shadow-md">
          <span class="text-[#0a1016] text-sm font-bold">GS</span>
        </div>
        <h1 class="text-[#e8f0f8] font-bold text-lg tracking-tight">GitSwamp</h1>
      </div>

      <div class="flex items-center gap-2 ml-4">
        <AppButton
          variant="ghost"
          size="sm"
          class="h-9 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] gap-2 transition-all"
        >
          <Download class="w-4 h-4" />
          Pull
        </AppButton>
        <AppButton
          variant="ghost"
          size="sm"
          class="h-9 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] gap-2 transition-all"
        >
          <Upload class="w-4 h-4" />
          Push
        </AppButton>
        <AppButton
          variant="ghost"
          size="sm"
          class="h-9 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] gap-2 transition-all"
        >
          <RefreshCw class="w-4 h-4" />
          Fetch
        </AppButton>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <AppButton
        variant="ghost"
        size="sm"
        class="h-9 px-3 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] gap-2 transition-all"
      >
        <ArrowLeft class="w-4 h-4" />
      </AppButton>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-9 px-3 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] gap-2 transition-all"
      >
        <ArrowRight class="w-4 h-4" />
      </AppButton>

      <div class="w-px h-6 bg-[#e8f0f8]/20 mx-2" />

      <AppButton
        variant="ghost"
        size="sm"
        class="h-9 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] gap-2 transition-all"
      >
        <GitBranch class="w-4 h-4" />
        Branch
      </AppButton>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-9 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] gap-2 transition-all"
      >
        <Archive class="w-4 h-4" />
        Stash
      </AppButton>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-9 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] gap-2 transition-all"
      >
        <GitPullRequest class="w-4 h-4" />
        PR
      </AppButton>
      <AppButton
        variant="ghost"
        size="sm"
        class="h-9 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] gap-2 transition-all"
      >
        <Terminal class="w-4 h-4" />
        Terminal
      </AppButton>

      <div class="w-px h-6 bg-[#e8f0f8]/20 mx-2" />

      <AppButton
        variant="ghost"
        size="icon"
        class="h-9 w-9 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] transition-all"
      >
        <Settings class="w-4 h-4" />
      </AppButton>
      <AppButton
        variant="ghost"
        size="icon"
        class="h-9 w-9 text-[#e8f0f8] hover:bg-[#1e2e3e] hover:text-[#4ecdc4] transition-all"
      >
        <User class="w-4 h-4" />
      </AppButton>
    </div>
  </div>
</template>

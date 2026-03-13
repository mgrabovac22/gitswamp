<script setup lang="ts">
import { reactive } from "vue";
import {
  Folder,
  Search,
  GitBranch,
  GitPullRequest,
  Archive,
  Tag,
  Users,
} from "lucide-vue-next";
import AppInput from "@/components/ui/AppInput.vue";
import SidebarSection from "./SidebarSection.vue";

interface SidebarSections {
  local: boolean;
  pullRequests: boolean;
  stashes: boolean;
  tags: boolean;
  teams: boolean;
}

const expandedSections = reactive<SidebarSections>({
  local: true,
  pullRequests: true,
  stashes: false,
  tags: false,
  teams: false,
});

function toggleSection(section: keyof SidebarSections) {
  expandedSections[section] = !expandedSections[section];
}

const localBranches = [
  "main",
  "dev_pro/KNAUF_...",
  "feature/new-ui",
  "hotfix/critical-bug",
];

const pullRequests = [
  "#42 New feature implementation",
  "#38 Bug fixes",
];
</script>

<template>
  <div class="w-60 bg-[#121a21] border-r border-[#4ecdc4]/20 flex flex-col h-full">
    <div class="p-4 border-b border-[#4ecdc4]/20">
      <div class="flex items-center gap-2 mb-3">
        <Folder class="w-4 h-4 text-[#4ecdc4]" />
        <div>
          <div class="text-xs text-[#8a9fb2]">EC</div>
          <div class="text-sm text-[#e0eaf2]">Repository</div>
        </div>
      </div>
      <div class="relative">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-[#8a9fb2]" />
        <AppInput
          placeholder="Filter branches..."
          class="h-8 pl-9 bg-[#182028] border-[#4ecdc4]/20 text-[#e0eaf2] placeholder:text-[#2d7a80] text-sm focus-visible:ring-[#4ecdc4]"
        />
      </div>
    </div>

    <div class="flex-1 overflow-y-auto">
      <SidebarSection
        label="LOCAL"
        :count="4"
        :icon="GitBranch"
        :expanded="expandedSections.local"
        @toggle="toggleSection('local')"
      >
        <button
          v-for="(branch, i) in localBranches"
          :key="i"
          class="w-full flex items-center gap-2 px-4 py-1.5 pl-12 text-sm text-[#8a9fb2] hover:text-[#e0eaf2] hover:bg-[#182028] transition-all"
        >
          {{ branch }}
        </button>
      </SidebarSection>

      <SidebarSection
        label="PULL REQUESTS"
        :count="2"
        :icon="GitPullRequest"
        :expanded="expandedSections.pullRequests"
        @toggle="toggleSection('pullRequests')"
      >
        <button
          v-for="(pr, i) in pullRequests"
          :key="i"
          class="w-full flex items-center gap-2 px-4 py-1.5 pl-12 text-sm text-[#8a9fb2] hover:text-[#e0eaf2] hover:bg-[#182028] transition-all"
        >
          {{ pr }}
        </button>
      </SidebarSection>

      <SidebarSection
        label="STASHES"
        :count="1"
        :icon="Archive"
        :expanded="expandedSections.stashes"
        @toggle="toggleSection('stashes')"
      />

      <SidebarSection
        label="TAGS"
        :count="32"
        :icon="Tag"
        :expanded="expandedSections.tags"
        @toggle="toggleSection('tags')"
      />

      <SidebarSection
        label="TEAMS"
        count=""
        :icon="Users"
        :expanded="expandedSections.teams"
        @toggle="toggleSection('teams')"
      />
    </div>
  </div>
</template>

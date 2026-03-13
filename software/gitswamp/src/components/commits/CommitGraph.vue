<script setup lang="ts">
import { computed } from "vue";
import { GitCommit } from "lucide-vue-next";
import { commits } from "@/data/commits";

const branchHeads = computed(() => {
  const heads = new Set<string>();
  const branches = new Map<string, string>();

  commits.forEach((commit) => {
    if (!branches.has(commit.branch)) {
      branches.set(commit.branch, commit.id);
      heads.add(commit.id);
    }
  });

  return heads;
});
</script>

<template>
  <div class="flex-1 bg-[#151d25] overflow-auto">
    <div class="sticky top-0 bg-[#151d25] border-b border-[#4ecdc4]/15 z-10">
      <div class="grid grid-cols-[180px_100px_80px_1fr_180px_100px] gap-4 px-6 py-3">
        <div class="text-xs text-[#8a9fb2] uppercase tracking-wider font-medium">Branch</div>
        <div class="text-xs text-[#8a9fb2] uppercase tracking-wider font-medium">Graph</div>
        <div class="text-xs text-[#8a9fb2] uppercase tracking-wider font-medium">Commit</div>
        <div class="text-xs text-[#8a9fb2] uppercase tracking-wider font-medium">Message</div>
        <div class="text-xs text-[#8a9fb2] uppercase tracking-wider font-medium">Author</div>
        <div class="text-xs text-[#8a9fb2] uppercase tracking-wider font-medium">SHA</div>
      </div>
    </div>

    <div class="pb-20">
      <div
        v-for="(commit, index) in commits"
        :key="commit.id"
        class="grid grid-cols-[180px_100px_80px_1fr_180px_100px] gap-4 px-6 py-3 border-b border-[#4ecdc4]/8 hover:bg-[#1a2a36]/50 transition-all group cursor-pointer"
      >
        <div class="text-sm text-[#8a9fb2] truncate">{{ commit.branch }}</div>

        <div class="flex items-center gap-2">
          <div class="relative flex items-center">
            <div
              v-if="index > 0"
              class="absolute -top-3 left-3 w-0.5 h-3"
              :style="{ backgroundColor: commit.color }"
            />

            <div
              :class="[
                'w-7 h-7 rounded-full overflow-hidden bg-[#1a2a36] transition-all',
                branchHeads.has(commit.id)
                  ? 'border-[3px] shadow-lg shadow-[currentColor]/40'
                  : 'border-2',
              ]"
              :style="{
                borderColor: commit.color,
                ...(branchHeads.has(commit.id)
                  ? { boxShadow: `0 0 20px ${commit.color}40, 0 0 10px ${commit.color}60` }
                  : {}),
              }"
            >
              <img
                :src="commit.avatar"
                :alt="commit.author"
                class="w-full h-full object-cover"
              />
            </div>

            <div
              v-if="index < commits.length - 1 && !branchHeads.has(commit.id)"
              class="absolute -bottom-3 left-3 w-0.5 h-3"
              :style="{ backgroundColor: commits[index + 1].color }"
            />

            <div
              v-if="branchHeads.has(commit.id)"
              class="absolute -right-1 -top-1 w-2.5 h-2.5 rounded-full animate-pulse"
              :style="{
                backgroundColor: commit.color,
                boxShadow: `0 0 8px ${commit.color}`,
              }"
            />
          </div>
          <GitCommit class="w-3 h-3 text-[#8a9fb2]" />
        </div>

        <div class="flex items-center gap-1.5 text-sm text-[#8a9fb2]">
          <GitCommit class="w-3 h-3" />
          <span>{{ commit.time }}</span>
        </div>

        <div class="text-sm text-[#c8d8e8] truncate group-hover:text-[#4ecdc4] transition-colors">
          <span class="font-medium" :style="{ color: commit.color }">{{ commit.number }}</span>
          {{ commit.message }}
        </div>

        <div class="text-sm text-[#c8d8e8]">{{ commit.author }}</div>

        <div class="flex items-center">
          <div
            class="px-2.5 py-1 bg-[#1e2e3a] text-xs font-mono rounded transition-all"
            :style="{
              color: commit.color,
              ...(branchHeads.has(commit.id)
                ? {
                    backgroundColor: `${commit.color}20`,
                    borderLeft: `3px solid ${commit.color}`,
                  }
                : {}),
            }"
          >
            {{ commit.sha }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

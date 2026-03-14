<script setup lang="ts">
import { ref, computed } from "vue";
import {
  FolderOpen,
  Star,
  Clock,
  GitBranch,
  X,
  Search,
  ChevronDown,
  ChevronRight,
  Plus,
} from "lucide-vue-next";

const props = defineProps<{
  openRepos: { name: string; path: string; branch: string }[];
  recentRepos: { name: string; path: string; branch: string; owner?: string }[];
}>();

const emit = defineEmits<{
  open: [path: string];
  browse: [];
  clone: [];
  init: [];
  removeRecent: [path: string];
  clearRecent: [];
}>();

const search = ref("");
const expandedSections = ref({
  open: true,
  favorites: true,
  recent: true,
});

const filteredRecent = computed(() => {
  if (!search.value.trim()) return props.recentRepos;
  const q = search.value.toLowerCase();
  return props.recentRepos.filter(
    (r) =>
      r.name.toLowerCase().includes(q) ||
      r.path.toLowerCase().includes(q) ||
      (r.owner && r.owner.toLowerCase().includes(q))
  );
});
</script>

<template>
  <div class="flex-1 overflow-y-auto bg-[#0d1017]">
    <div class="max-w-5xl mx-auto px-8 py-6">
      <!-- Title -->
      <div class="flex items-center justify-between mb-6">
        <h1 class="text-xl font-bold text-[#e2e8f0]">Repository Management</h1>
      </div>

      <!-- Action buttons -->
      <div class="flex items-center gap-2 mb-6">
        <button @click="emit('browse')" class="px-4 py-2 bg-[#1c2130] hover:bg-[#252b3d] text-[#e2e8f0] text-xs font-medium rounded-md border border-[#8b5cf6]/15 transition-colors flex items-center gap-2">
          <FolderOpen class="w-3.5 h-3.5" />
          Browse
        </button>
        <button @click="emit('clone')" class="px-4 py-2 bg-[#8b5cf6] hover:bg-[#7c3aed] text-white text-xs font-medium rounded-md transition-colors flex items-center gap-2">
          <GitBranch class="w-3.5 h-3.5" />
          Clone
        </button>
        <button @click="emit('init')" class="px-4 py-2 bg-[#1c2130] hover:bg-[#252b3d] text-[#e2e8f0] text-xs font-medium rounded-md border border-[#8b5cf6]/15 transition-colors flex items-center gap-2">
          <Plus class="w-3.5 h-3.5" />
          Init
        </button>
      </div>

      <!-- Search -->
      <div class="relative mb-6">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[#64748b]" />
        <input
          v-model="search"
          placeholder="Search repositories..."
          class="w-full pl-10 pr-4 py-2.5 bg-[#151921] border border-[#8b5cf6]/15 rounded-lg text-sm text-[#e2e8f0] placeholder:text-[#475569] focus:outline-none focus:ring-1 focus:ring-[#8b5cf6]/40"
        />
      </div>

      <!-- Open repositories -->
      <div class="mb-6">
        <button
          @click="expandedSections.open = !expandedSections.open"
          class="flex items-center gap-2 mb-3 text-sm text-[#e2e8f0] hover:text-[#a78bfa] transition-colors w-full"
        >
          <ChevronDown v-if="expandedSections.open" class="w-4 h-4" />
          <ChevronRight v-else class="w-4 h-4" />
          <span class="font-medium">Open repositories</span>
          <span class="text-[10px] text-[#64748b] bg-[#1c2130] px-2 py-0.5 rounded-full ml-1">{{ openRepos.length }}</span>
        </button>
        <div v-if="expandedSections.open && openRepos.length > 0" class="space-y-1">
          <div
            v-for="repo in openRepos"
            :key="repo.path"
            @click="emit('open', repo.path)"
            class="flex items-center gap-3 px-4 py-2.5 rounded-lg hover:bg-[#151921] cursor-pointer transition-colors group"
          >
            <FolderOpen class="w-4 h-4 text-[#a78bfa] flex-shrink-0" />
            <span class="text-sm text-[#e2e8f0] font-medium flex-shrink-0">{{ repo.name }}</span>
            <div class="flex items-center gap-2 ml-auto">
              <span class="px-2 py-0.5 text-[10px] font-medium rounded bg-[#8b5cf6]/15 text-[#a78bfa] border border-[#8b5cf6]/20 flex items-center gap-1">
                <GitBranch class="w-3 h-3" />
                {{ repo.branch }}
              </span>
            </div>
          </div>
        </div>
        <div v-if="expandedSections.open && openRepos.length === 0" class="text-xs text-[#475569] italic px-4">
          No repositories open
        </div>
      </div>

      <!-- Favorites -->
      <div class="mb-6">
        <button
          @click="expandedSections.favorites = !expandedSections.favorites"
          class="flex items-center gap-2 mb-3 text-sm text-[#e2e8f0] hover:text-[#a78bfa] transition-colors w-full"
        >
          <ChevronDown v-if="expandedSections.favorites" class="w-4 h-4" />
          <ChevronRight v-else class="w-4 h-4" />
          <Star class="w-4 h-4 text-[#f59e0b]" />
          <span class="font-medium">Favorites</span>
          <span class="text-[10px] text-[#64748b] bg-[#1c2130] px-2 py-0.5 rounded-full ml-1">0</span>
        </button>
        <div v-if="expandedSections.favorites" class="text-xs text-[#475569] italic px-4">
          No favorites yet. Star a repository to add it here.
        </div>
      </div>

      <!-- Recent repositories -->
      <div>
        <div class="flex items-center justify-between mb-3">
          <button
            @click="expandedSections.recent = !expandedSections.recent"
            class="flex items-center gap-2 text-sm text-[#e2e8f0] hover:text-[#a78bfa] transition-colors"
          >
            <ChevronDown v-if="expandedSections.recent" class="w-4 h-4" />
            <ChevronRight v-else class="w-4 h-4" />
            <Clock class="w-4 h-4 text-[#64748b]" />
            <span class="font-medium">Recent repositories</span>
            <span class="text-[10px] text-[#64748b] bg-[#1c2130] px-2 py-0.5 rounded-full ml-1">{{ recentRepos.length }}</span>
          </button>
          <button
            v-if="recentRepos.length > 0"
            @click="emit('clearRecent')"
            class="text-[10px] text-[#64748b] hover:text-[#ef4444] transition-colors flex items-center gap-1"
          >
            <X class="w-3 h-3" />
            Remove all
          </button>
        </div>
        <div v-if="expandedSections.recent && filteredRecent.length > 0" class="space-y-1">
          <div
            v-for="repo in filteredRecent"
            :key="repo.path"
            @click="emit('open', repo.path)"
            class="flex items-center gap-3 px-4 py-2.5 rounded-lg hover:bg-[#151921] cursor-pointer transition-colors group"
          >
            <FolderOpen class="w-4 h-4 text-[#64748b] flex-shrink-0" />
            <span class="text-sm text-[#e2e8f0] font-medium min-w-0 truncate">{{ repo.name }}</span>
            <span v-if="repo.owner" class="text-xs text-[#64748b] flex-shrink-0">{{ repo.owner }}</span>
            <div class="flex items-center gap-2 ml-auto flex-shrink-0">
              <span class="px-2 py-0.5 text-[10px] font-medium rounded bg-[#151921] text-[#64748b] border border-[#8b5cf6]/10 flex items-center gap-1">
                <GitBranch class="w-3 h-3" />
                {{ repo.branch }}
              </span>
              <button
                @click.stop="emit('removeRecent', repo.path)"
                class="p-1 rounded hover:bg-[#ef4444]/20 opacity-0 group-hover:opacity-100 transition-opacity"
              >
                <X class="w-3 h-3 text-[#64748b]" />
              </button>
            </div>
          </div>
        </div>
        <div v-if="expandedSections.recent && filteredRecent.length === 0 && search" class="text-xs text-[#475569] italic px-4">
          No matching repositories
        </div>
        <div v-if="expandedSections.recent && recentRepos.length === 0" class="text-xs text-[#475569] italic px-4">
          No recent repositories
        </div>
      </div>
    </div>
  </div>
</template>

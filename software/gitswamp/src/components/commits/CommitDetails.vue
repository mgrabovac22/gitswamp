<script setup lang="ts">
import { ref, reactive } from "vue";
import {
  Calendar,
  Hash,
  ChevronDown,
  FileText,
  Trash2,
  Eye,
  MoreVertical,
} from "lucide-vue-next";
import AppButton from "@/components/ui/AppButton.vue";
import FileItem from "./FileItem.vue";
import GitCommitIcon from "@/components/ui/GitCommitIcon.vue";

const activeTab = ref<"changes" | "commit">("changes");

const expandedFiles = reactive({
  staged: true,
  unstaged: true,
});
</script>

<template>
  <div class="w-96 bg-[#121a21] border-l border-[#4ecdc4]/20 flex flex-col h-full overflow-hidden">
    <div class="flex flex-col flex-1">
      <div class="border-b border-[#4ecdc4]/20">
        <div class="w-full bg-transparent h-12 p-0 grid grid-cols-2">
          <button
            @click="activeTab = 'changes'"
            :class="[
              'h-full text-sm font-medium transition-colors',
              activeTab === 'changes'
                ? 'bg-[#182028] text-[#4ecdc4] border-b-2 border-[#4ecdc4]'
                : 'text-[#8a9fb2] hover:text-[#e0eaf2]',
            ]"
          >
            Changes
          </button>
          <button
            @click="activeTab = 'commit'"
            :class="[
              'h-full text-sm font-medium transition-colors',
              activeTab === 'commit'
                ? 'bg-[#182028] text-[#4ecdc4] border-b-2 border-[#4ecdc4]'
                : 'text-[#8a9fb2] hover:text-[#e0eaf2]',
            ]"
          >
            Commit Info
          </button>
        </div>
      </div>

      <div v-show="activeTab === 'changes'" class="flex-1 flex flex-col overflow-hidden">
        <div class="flex-1 overflow-y-auto">
          <div class="border-b border-[#4ecdc4]/20">
            <div class="flex items-center justify-between px-4 py-3 bg-[#182028]">
              <button
                @click="expandedFiles.staged = !expandedFiles.staged"
                class="flex items-center gap-2 text-sm text-[#e0eaf2] hover:text-[#4ecdc4] transition-all flex-1"
              >
                <ChevronDown
                  :class="[
                    'w-4 h-4 transition-transform',
                    !expandedFiles.staged ? '-rotate-90' : '',
                  ]"
                />
                <span class="font-medium">Staged Files</span>
                <span class="text-xs bg-[#4ecdc4] text-[#0a1016] px-2 py-0.5 rounded-full font-medium">3</span>
              </button>

              <div class="flex items-center gap-1">
                <button
                  class="p-1.5 rounded hover:bg-[#1e2c38] transition-all group"
                  title="Discard all staged changes"
                >
                  <Trash2 class="w-4 h-4 text-[#8a9fb2] group-hover:text-[#ff6b6b]" />
                </button>
                <button
                  class="p-1.5 rounded hover:bg-[#1e2c38] transition-all"
                  title="More options"
                >
                  <MoreVertical class="w-4 h-4 text-[#8a9fb2] hover:text-[#e0eaf2]" />
                </button>
              </div>
            </div>

            <div v-if="expandedFiles.staged" class="pb-2">
              <FileItem
                name="src/components/ToplinjeGrijanja.tsx"
                :additions="23"
                :deletions="8"
              />
              <FileItem
                name="src/utils/calculations.ts"
                :additions="45"
                :deletions="12"
              />
              <FileItem
                name="src/types/index.ts"
                :additions="67"
                :deletions="0"
              />
            </div>
          </div>

          <div>
            <div class="flex items-center justify-between px-4 py-3 bg-[#182028]">
              <button
                @click="expandedFiles.unstaged = !expandedFiles.unstaged"
                class="flex items-center gap-2 text-sm text-[#e0eaf2] hover:text-[#4ecdc4] transition-all flex-1"
              >
                <ChevronDown
                  :class="[
                    'w-4 h-4 transition-transform',
                    !expandedFiles.unstaged ? '-rotate-90' : '',
                  ]"
                />
                <span class="font-medium">Unstaged Files</span>
                <span class="text-xs bg-[#1e2c38] text-[#8a9fb2] px-2 py-0.5 rounded-full">0</span>
              </button>
            </div>

            <div v-if="expandedFiles.unstaged" class="px-6 py-8 flex flex-col items-center justify-center">
              <div class="w-16 h-16 rounded-full bg-[#182028] flex items-center justify-center mb-3">
                <FileText class="w-8 h-8 text-[#2d7a80]" />
              </div>
              <p class="text-sm text-[#8a9fb2] text-center">No unstaged changes</p>
            </div>
          </div>
        </div>

        <div class="border-t border-[#4ecdc4]/20 p-4 bg-[#182028]">
          <div class="mb-3">
            <label class="text-xs text-[#8a9fb2] uppercase tracking-wider mb-2 block">
              Commit Summary
            </label>
            <input
              type="text"
              placeholder="Brief description of changes..."
              class="w-full px-3 py-2 bg-[#121a21] border border-[#4ecdc4]/20 rounded text-sm text-[#e0eaf2] placeholder:text-[#2d7a80] focus:outline-none focus:ring-2 focus:ring-[#4ecdc4] transition-all"
            />
          </div>

          <div class="mb-4">
            <label class="text-xs text-[#8a9fb2] uppercase tracking-wider mb-2 block">
              Description
            </label>
            <textarea
              placeholder="Optional detailed description..."
              rows="2"
              class="w-full px-3 py-2 bg-[#121a21] border border-[#4ecdc4]/20 rounded text-sm text-[#e0eaf2] placeholder:text-[#2d7a80] focus:outline-none focus:ring-2 focus:ring-[#4ecdc4] resize-none transition-all"
            />
          </div>

          <AppButton
            class="w-full bg-[#4ecdc4] hover:bg-[#3eb8b0] text-[#0a1016] font-medium transition-all shadow-md"
            disabled
          >
            Commit Changes
          </AppButton>
        </div>
      </div>

      <div v-show="activeTab === 'commit'" class="flex-1 flex flex-col overflow-hidden">
        <div class="flex-1 overflow-y-auto">
          <div class="p-6 border-b border-[#4ecdc4]/20">
            <div class="flex items-start gap-3 mb-4">
              <div class="w-12 h-12 rounded-full border-2 border-[#4ecdc4] overflow-hidden bg-[#182028] flex-shrink-0 shadow-md">
                <img
                  src="https://images.unsplash.com/photo-1609402267734-b8798e6ee52f?w=100&h=100&fit=crop"
                  alt="Marin Grahovec"
                  class="w-full h-full object-cover"
                />
              </div>
              <div class="flex-1 min-w-0">
                <div class="text-[#e0eaf2] font-medium mb-1">Marin Grahovec</div>
                <div class="text-xs text-[#8a9fb2]">marin@example.com</div>
                <div class="text-xs text-[#8a9fb2] mt-1 flex items-center gap-1.5">
                  <span class="w-1.5 h-1.5 rounded-full bg-[#4ecdc4]"></span>
                  Authored &amp; Committed
                </div>
              </div>
            </div>

            <div class="space-y-2 mb-4">
              <div class="flex items-center gap-2 text-sm">
                <GitCommitIcon class="w-4 h-4 text-[#4ecdc4]" />
                <span class="text-[#8a9fb2] uppercase text-xs tracking-wider">Commit Message</span>
              </div>
              <div class="text-sm text-[#e0eaf2] leading-relaxed bg-[#182028] p-3 rounded">
                <span class="text-[#4ecdc4] font-medium">#490</span> Implementirano da za t_ci_m ne koristi t_d i rubne izračune...
              </div>
            </div>

            <div class="my-4 h-px w-full bg-[#4ecdc4]/20" />

            <div class="space-y-4">
              <div>
                <div class="flex items-center gap-2 text-xs text-[#8a9fb2] mb-2">
                  <Hash class="w-3.5 h-3.5" />
                  <span class="uppercase tracking-wider">Commit SHA</span>
                </div>
                <div class="px-3 py-2 bg-[#1e2c38] text-[#4ecdc4] text-sm font-mono rounded inline-block">
                  2e7725a4f3c1
                </div>
              </div>

              <div>
                <div class="flex items-center gap-2 text-xs text-[#8a9fb2] mb-2">
                  <Calendar class="w-3.5 h-3.5" />
                  <span class="uppercase tracking-wider">Date &amp; Time</span>
                </div>
                <div class="text-sm text-[#e0eaf2]">March 12, 2026, 10:34 AM</div>
                <div class="text-xs text-[#8a9fb2] mt-1">2 hours ago</div>
              </div>

              <div>
                <div class="flex items-center gap-2 text-xs text-[#8a9fb2] mb-2">
                  <GitCommitIcon class="w-3.5 h-3.5" />
                  <span class="uppercase tracking-wider">Parent Commit</span>
                </div>
                <div class="px-3 py-2 bg-[#1e2c38] text-[#4ecdc4] text-sm font-mono rounded inline-block hover:bg-[#243442] cursor-pointer transition-all">
                  e88cb42a
                </div>
              </div>

              <div>
                <div class="flex items-center gap-2 text-xs text-[#8a9fb2] mb-2">
                  <span class="uppercase tracking-wider">Branch</span>
                </div>
                <div class="flex flex-wrap gap-2">
                  <span class="px-2.5 py-1 bg-[#4ecdc4] text-[#0a1016] text-xs font-medium rounded">
                    dev_pro/KNAUF_...
                  </span>
                </div>
              </div>

              <div>
                <div class="text-xs text-[#8a9fb2] uppercase tracking-wider mb-2">Changed Files</div>
                <div class="text-sm text-[#e0eaf2]">3 files changed</div>
                <div class="flex items-center gap-3 mt-1 text-xs">
                  <span class="text-[#4ecdc4]">+135 additions</span>
                  <span class="text-[#ff6b6b]">-20 deletions</span>
                </div>
                <div class="mt-2 space-y-1">
                  <div class="text-xs text-[#8a9fb2] flex items-center gap-2">
                    <FileText class="w-3 h-3" />
                    <span class="truncate">src/components/ToplinjeGrijanja.tsx</span>
                  </div>
                  <div class="text-xs text-[#8a9fb2] flex items-center gap-2">
                    <FileText class="w-3 h-3" />
                    <span class="truncate">src/utils/calculations.ts</span>
                  </div>
                  <div class="text-xs text-[#8a9fb2] flex items-center gap-2">
                    <FileText class="w-3 h-3" />
                    <span class="truncate">src/types/index.ts</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="p-4 space-y-2">
            <AppButton
              class="w-full bg-[#1e2c38] hover:bg-[#243442] text-[#e0eaf2] font-medium transition-all justify-start gap-2"
            >
              <Eye class="w-4 h-4" />
              View Full Diff
            </AppButton>
            <AppButton
              class="w-full bg-[#1e2c38] hover:bg-[#243442] text-[#e0eaf2] font-medium transition-all justify-start gap-2"
            >
              <Hash class="w-4 h-4" />
              Copy SHA
            </AppButton>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

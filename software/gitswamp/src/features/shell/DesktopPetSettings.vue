<script setup lang="ts">
import { defineAsyncComponent, ref } from "vue";
import { SlidersHorizontal } from "lucide-vue-next";
import AppButton from "@/shared/ui/AppButton.vue";
import { petAppearance, petEnabled, setPetEnabled } from "./petPreferences";

const PetEditorDialog = defineAsyncComponent(() => import("./PetEditorDialog.vue"));
const editing = ref(false);
</script>

<template>
  <div class="flex flex-wrap items-center justify-between gap-2 py-2" data-option="desktop-pet">
    <div class="min-w-0">
      <span class="text-xs font-medium text-[var(--foreground)]">Desktop pet</span>
      <p v-if="petEnabled" class="text-[10px] text-[var(--muted-foreground)] break-all">{{ petAppearance.name }}</p>
    </div>
    <div class="flex shrink-0 items-center gap-2">
      <AppButton variant="outline" size="sm" :disabled="!petEnabled" class="text-xs" @click="editing = true">
        <SlidersHorizontal :size="14" /> Edit pet
      </AppButton>
      <button type="button" role="switch" aria-label="Desktop pet" :aria-checked="petEnabled"
        class="relative h-6 w-11 shrink-0 rounded-full border border-[var(--border)] focus-visible:outline-2 focus-visible:outline-[var(--ring)]"
        :class="petEnabled ? 'bg-[var(--switch-background)]' : 'bg-[var(--secondary)]'"
        @click="setPetEnabled(!petEnabled)">
        <span aria-hidden="true" class="absolute left-[3px] top-[3px] h-4 w-4 rounded-full border border-[var(--border)] bg-[var(--foreground)] transition-transform"
          :class="petEnabled ? 'translate-x-[19px] bg-[var(--primary-foreground)]' : ''" />
      </button>
    </div>
    <PetEditorDialog v-if="editing && petEnabled" @close="editing = false" />
  </div>
</template>

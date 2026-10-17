<script setup lang="ts">
import { computed, ref } from "vue";
import { ArrowUpDown, Check, Copy, PackageCheck } from "lucide-vue-next";
import { bumpSemver, compareSemver, parseSemver } from "../semverTool";

const current = ref("0.4.0");
const other = ref("0.5.0");
const copied = ref(false);

const currentVersion = computed(() => parseSemver(current.value));
const otherVersion = computed(() => parseSemver(other.value));
const comparison = computed(() => {
  if (!currentVersion.value || !otherVersion.value) return null;
  return compareSemver(currentVersion.value, otherVersion.value);
});

const result = computed(() => {
  if (comparison.value === null) return "Enter two valid versions";
  if (comparison.value === 0) return "Versions are equal";
  return comparison.value < 0 ? `${other.value.trim()} is newer` : `${current.value.trim()} is newer`;
});

const bumps = computed(() => {
  if (!currentVersion.value) return null;
  return {
    major: bumpSemver(currentVersion.value, "major"),
    minor: bumpSemver(currentVersion.value, "minor"),
    patch: bumpSemver(currentVersion.value, "patch"),
  };
});

async function copy(value: string) {
  try {
    await navigator.clipboard.writeText(value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1200);
  } catch {
    copied.value = false;
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-2 overflow-y-auto pr-1">
    <div class="rounded-lg border border-[var(--border)] bg-[var(--secondary)]/25 p-2.5">
      <div class="flex items-center gap-1.5">
        <PackageCheck class="h-3.5 w-3.5 text-[var(--primary)]" />
        <span class="text-[11px] font-semibold text-[var(--foreground)]">Semantic versioning</span>
      </div>
      <div class="mt-1.5 grid grid-cols-2 gap-2">
        <input v-model="current" spellcheck="false" aria-label="Current version" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 font-mono text-[11px]" />
        <input v-model="other" spellcheck="false" aria-label="Version to compare" class="rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1 font-mono text-[11px]" />
      </div>
      <div class="mt-2 flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)]">
        <ArrowUpDown class="h-3 w-3" />
        <span>{{ result }}</span>
      </div>
    </div>

    <div v-if="bumps" class="rounded-lg border border-[var(--border)] bg-[var(--card)] p-2.5">
      <span class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Next release versions</span>
      <div class="mt-1.5 grid grid-cols-3 gap-1.5">
        <button v-for="type in ['major', 'minor', 'patch']" :key="type" class="rounded border border-[var(--border)] px-1.5 py-1 text-left hover:bg-[var(--secondary)]" @click="copy(bumps[type as keyof typeof bumps])">
          <span class="block text-[9px] uppercase text-[var(--muted-foreground)]">{{ type }}</span>
          <span class="font-mono text-[11px] text-[var(--foreground)]">{{ bumps[type as keyof typeof bumps] }}</span>
        </button>
      </div>
    </div>
    <span v-if="copied" class="flex items-center gap-1 text-[10px] text-[var(--muted-foreground)]"><Check class="h-3 w-3" /> Copied</span>
    <button class="flex w-fit items-center gap-1 rounded border border-[var(--border)] px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:bg-[var(--secondary)] hover:text-[var(--foreground)]" @click="copy(current)">
      <Copy class="h-3 w-3" /> Copy current version
    </button>
  </div>
</template>

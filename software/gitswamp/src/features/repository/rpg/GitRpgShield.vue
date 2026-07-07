<script setup lang="ts">
import { computed } from "vue";
import type { GitRpgRoleDefinition } from "@/features/repository/rpg/gitRpgProfiler";

type ShieldSize = "header" | "help";
type RoleIcon =
  | "scalpel"
  | "moon"
  | "chisel"
  | "ghost"
  | "blueprint"
  | "flame"
  | "sprout"
  | "map"
  | "test"
  | "refactor"
  | "rocket"
  | "chain"
  | "merge"
  | "compass"
  | "anchor"
  | "hammer"
  | "potion"
  | "anvil"
  | "bolt"
  | "lock"
  | "gear"
  | "branch"
  | "database"
  | "broom"
  | "calendar"
  | "book"
  | "juggle"
  | "diamond";

const props = withDefaults(defineProps<{
  role?: GitRpgRoleDefinition | null;
  loading?: boolean;
  size?: ShieldSize;
}>(), {
  role: null,
  loading: false,
  size: "header",
});

const roleIconById: Record<string, RoleIcon> = {
  "surgeon": "scalpel",
  "night-cowboy": "moon",
  "tiny-sculptor": "chisel",
  "ghost-rider": "ghost",
  "architect": "blueprint",
  "firefighter": "flame",
  "gardener": "sprout",
  "cartographer": "map",
  "test-guardian": "test",
  "refactor-monk": "refactor",
  "release-captain": "rocket",
  "dependency-tamer": "chain",
  "merge-diplomat": "merge",
  "explorer": "compass",
  "stabilizer": "anchor",
  "builder": "hammer",
  "ui-alchemist": "potion",
  "backend-smith": "anvil",
  "performance-sprinter": "bolt",
  "security-sentinel": "lock",
  "automation-pilot": "gear",
  "conflict-whisperer": "branch",
  "data-steward": "database",
  "janitor": "broom",
  "weekend-sprinter": "calendar",
  "storyteller": "book",
  "risk-juggler": "juggle",
  "balanced-adventurer": "diamond",
};

const icon = computed<RoleIcon>(() => roleIconById[props.role?.id || ""] || "diamond");
const accent = computed(() => props.role?.accent || "#64748b");
const iconA = computed(() => colorForRole(props.role?.id || "balanced-adventurer", 0));
const iconB = computed(() => colorForRole(props.role?.id || "balanced-adventurer", 1));

function colorForRole(id: string, index: 0 | 1): string {
  const palette: Record<string, [string, string]> = {
    "surgeon": ["#34d399", "#f8fafc"],
    "night-cowboy": ["#facc15", "#a78bfa"],
    "tiny-sculptor": ["#67e8f9", "#f97316"],
    "ghost-rider": ["#fb923c", "#f8fafc"],
    "architect": ["#38bdf8", "#facc15"],
    "firefighter": ["#fb7185", "#facc15"],
    "gardener": ["#86efac", "#22c55e"],
    "cartographer": ["#fde68a", "#22d3ee"],
    "test-guardian": ["#2dd4bf", "#f8fafc"],
    "refactor-monk": ["#c084fc", "#60a5fa"],
    "release-captain": ["#fbbf24", "#38bdf8"],
    "dependency-tamer": ["#f472b6", "#fde047"],
    "merge-diplomat": ["#818cf8", "#34d399"],
    "explorer": ["#38bdf8", "#f97316"],
    "stabilizer": ["#4ade80", "#f8fafc"],
    "builder": ["#60a5fa", "#facc15"],
    "ui-alchemist": ["#f0abfc", "#22d3ee"],
    "backend-smith": ["#2dd4bf", "#f97316"],
    "performance-sprinter": ["#fde047", "#38bdf8"],
    "security-sentinel": ["#fb7185", "#f8fafc"],
    "automation-pilot": ["#fb923c", "#60a5fa"],
    "conflict-whisperer": ["#c084fc", "#facc15"],
    "data-steward": ["#a3e635", "#38bdf8"],
    "janitor": ["#cbd5e1", "#22d3ee"],
    "weekend-sprinter": ["#fb7185", "#fde047"],
    "storyteller": ["#facc15", "#f8fafc"],
    "risk-juggler": ["#fb7185", "#fbbf24"],
    "balanced-adventurer": ["#94a3b8", "#38bdf8"],
  };
  return (palette[id] || palette["balanced-adventurer"])[index];
}
</script>

<template>
  <span
    class="git-rpg-shield"
    :class="[`git-rpg-shield-${size}`, { 'git-rpg-shield-loading': loading }]"
    :style="{ '--rpg-accent': accent, '--rpg-icon-a': iconA, '--rpg-icon-b': iconB }"
    aria-hidden="true"
  >
    <svg viewBox="0 0 48 52" focusable="false" class="git-rpg-shield-svg">
      <path class="shield-bg" d="M24 2 43 9 39 37 24 50 9 37 5 9 24 2Z" />
      <path class="shield-inner" d="M24 7 38 12 35 34 24 44 13 34 10 12 24 7Z" />

      <g v-if="icon === 'scalpel'" class="icon-stroke">
        <path d="M15 34 33 16" />
        <path d="M29 14 36 11 34 18" class="icon-fill-secondary" />
        <path d="M14 35 20 38" />
      </g>

      <g v-else-if="icon === 'moon'" class="icon-stroke">
        <path d="M30 13c-7 1-11 6-10 13 1 6 7 10 14 8-3 4-11 5-17 0-7-5-7-16 0-21 5-4 10-3 13 0Z" class="icon-fill-primary" />
        <path d="M34 18h.1M37 26h.1M28 34h.1" />
      </g>

      <g v-else-if="icon === 'chisel'" class="icon-stroke">
        <path d="M17 32 30 19" />
        <path d="M29 16 34 21 31 24 26 19Z" class="icon-fill-primary" />
        <path d="M16 35h12" />
        <path d="M21 27 25 31" />
      </g>

      <g v-else-if="icon === 'ghost'" class="icon-stroke">
        <path d="M15 34V21c0-6 4-10 9-10s9 4 9 10v13l-4-3-3 3-3-3-4 3-4-3Z" class="icon-fill-primary" />
        <path d="M20 22h.1M28 22h.1" />
        <path d="M21 28c2 1 4 1 6 0" />
      </g>

      <g v-else-if="icon === 'blueprint'" class="icon-stroke">
        <path d="M14 16h20v20H14Z" class="icon-fill-primary" />
        <path d="M18 20h12M18 25h7M18 30h12" />
        <path d="M29 25h5v11" />
      </g>

      <g v-else-if="icon === 'flame'" class="icon-stroke">
        <path d="M25 10c1 7-7 8-7 16 0 6 4 10 6 10 5 0 9-4 9-10 0-5-4-8-4-12-1 3-3 5-5 6 1-4 1-7 1-10Z" class="icon-fill-primary" />
        <path d="M24 35c-3-2-4-5-1-9 1 3 5 4 3 9" class="icon-fill-secondary" />
      </g>

      <g v-else-if="icon === 'sprout'" class="icon-stroke">
        <path d="M24 36V20" />
        <path d="M23 22c-7-1-9-6-8-10 6 0 9 3 9 10Z" class="icon-fill-primary" />
        <path d="M25 25c7 0 10-4 9-9-6 0-9 4-9 9Z" class="icon-fill-secondary" />
        <path d="M18 36h12" />
      </g>

      <g v-else-if="icon === 'map'" class="icon-stroke">
        <path d="M13 16 21 13l7 3 7-3v23l-7 3-7-3-8 3Z" class="icon-fill-primary" />
        <path d="M21 13v23M28 16v23" />
        <path d="M16 24c4-2 6 3 11 1 3-1 4-4 7-3" />
      </g>

      <g v-else-if="icon === 'test'" class="icon-stroke">
        <path d="M24 12 35 17v8c0 7-4 11-11 14-7-3-11-7-11-14v-8Z" class="icon-fill-primary" />
        <path d="m18 26 4 4 8-10" />
      </g>

      <g v-else-if="icon === 'refactor'" class="icon-stroke">
        <path d="M16 21c3-6 12-6 16 0" />
        <path d="m32 21-1-6 5 3" />
        <path d="M32 31c-3 6-12 6-16 0" />
        <path d="m16 31 1 6-5-3" />
      </g>

      <g v-else-if="icon === 'rocket'" class="icon-stroke">
        <path d="M20 31 17 36l6-2 5-5c6-6 8-12 7-16-4-1-10 1-16 7l-5 5-2 6 5-3Z" class="icon-fill-primary" />
        <path d="M27 20h.1" />
        <path d="M18 34c-2 0-4 2-5 5 3-1 5-3 5-5Z" class="icon-fill-secondary" />
      </g>

      <g v-else-if="icon === 'chain'" class="icon-stroke">
        <path d="M19 22 16 25c-3 3-3 7 0 9s6 2 9-1l2-2" />
        <path d="M29 20 31 18c3-3 6-3 9-1s3 6 0 9l-3 3" />
        <path d="M21 31 32 20" />
      </g>

      <g v-else-if="icon === 'merge'" class="icon-stroke">
        <path d="M16 14v24" />
        <path d="M32 14v7c0 5-4 8-16 11" />
        <circle cx="16" cy="14" r="3" class="icon-fill-primary" />
        <circle cx="32" cy="14" r="3" class="icon-fill-secondary" />
        <circle cx="16" cy="38" r="3" class="icon-fill-primary" />
      </g>

      <g v-else-if="icon === 'compass'" class="icon-stroke">
        <circle cx="24" cy="26" r="12" class="icon-fill-primary" />
        <path d="m29 17-3 12-8 5 3-12Z" class="icon-fill-secondary" />
        <path d="M24 14v3M24 35v3M12 26h3M33 26h3" />
      </g>

      <g v-else-if="icon === 'anchor'" class="icon-stroke">
        <circle cx="24" cy="15" r="3" class="icon-fill-primary" />
        <path d="M24 18v19" />
        <path d="M17 24h14" />
        <path d="M14 30c1 6 5 9 10 9s9-3 10-9" />
        <path d="m14 30 4 1M34 30l-4 1" />
      </g>

      <g v-else-if="icon === 'hammer'" class="icon-stroke">
        <path d="M19 16h12l4 4-4 4H19Z" class="icon-fill-primary" />
        <path d="M22 24 14 34l4 4 10-12" />
        <path d="m30 16 5-4" />
      </g>

      <g v-else-if="icon === 'potion'" class="icon-stroke">
        <path d="M21 12h6" />
        <path d="M23 12v9l-7 12c-2 4 1 7 8 7s10-3 8-7l-7-12v-9" class="icon-fill-primary" />
        <path d="M18 31c4 2 8-2 12 0" />
        <path d="M20 35h8" class="icon-secondary-stroke" />
      </g>

      <g v-else-if="icon === 'anvil'" class="icon-stroke">
        <path d="M13 22h22l-5 5h-7l-3 7h-7l4-7h-4Z" class="icon-fill-primary" />
        <path d="M18 34h13" />
        <path d="M28 18h8" />
      </g>

      <g v-else-if="icon === 'bolt'" class="icon-stroke">
        <path d="M28 10 15 28h8l-3 14 13-19h-8Z" class="icon-fill-primary" />
      </g>

      <g v-else-if="icon === 'lock'" class="icon-stroke">
        <path d="M16 24h16v13H16Z" class="icon-fill-primary" />
        <path d="M19 24v-5c0-4 2-7 5-7s5 3 5 7v5" />
        <path d="M24 30v3" />
      </g>

      <g v-else-if="icon === 'gear'" class="icon-stroke">
        <path d="M24 13v4M24 35v4M13 26h4M31 26h4M16 18l3 3M29 31l3 3M32 18l-3 3M19 31l-3 3" />
        <circle cx="24" cy="26" r="8" class="icon-fill-primary" />
        <circle cx="24" cy="26" r="3" class="icon-fill-secondary" />
      </g>

      <g v-else-if="icon === 'branch'" class="icon-stroke">
        <path d="M17 14v24" />
        <path d="M17 25c8 0 14-4 14-11" />
        <path d="M17 30c7 0 11 3 13 8" />
        <circle cx="17" cy="14" r="3" class="icon-fill-primary" />
        <circle cx="31" cy="14" r="3" class="icon-fill-secondary" />
        <circle cx="30" cy="38" r="3" class="icon-fill-secondary" />
      </g>

      <g v-else-if="icon === 'database'" class="icon-stroke">
        <ellipse cx="24" cy="17" rx="10" ry="4" class="icon-fill-primary" />
        <path d="M14 17v16c0 2 4 4 10 4s10-2 10-4V17" class="icon-fill-primary" />
        <path d="M14 25c0 2 4 4 10 4s10-2 10-4" />
      </g>

      <g v-else-if="icon === 'broom'" class="icon-stroke">
        <path d="M30 13 18 29" />
        <path d="M15 30 23 36l-8 3-4-3Z" class="icon-fill-primary" />
        <path d="M18 31 15 38M22 34l-4 4" />
      </g>

      <g v-else-if="icon === 'calendar'" class="icon-stroke">
        <path d="M15 17h18v19H15Z" class="icon-fill-primary" />
        <path d="M15 23h18M20 13v7M28 13v7" />
        <path d="m25 27-4 6h5l-2 5 5-7h-5Z" class="icon-fill-secondary" />
      </g>

      <g v-else-if="icon === 'book'" class="icon-stroke">
        <path d="M14 15h9c3 0 5 2 5 5v18c0-3-2-5-5-5h-9Z" class="icon-fill-primary" />
        <path d="M28 20c0-3 2-5 5-5h1v18h-1c-3 0-5 2-5 5" />
        <path d="M18 22h6M18 27h5" />
      </g>

      <g v-else-if="icon === 'juggle'" class="icon-stroke">
        <path d="M16 32 21 17l5 15Z" class="icon-fill-primary" />
        <path d="M28 32 33 17l5 15Z" class="icon-fill-secondary" />
        <path d="M14 35h13M26 35h13" />
        <circle cx="24" cy="14" r="2" class="icon-fill-secondary" />
      </g>

      <g v-else class="icon-stroke">
        <path d="M24 12 35 26 24 40 13 26Z" class="icon-fill-primary" />
        <path d="M24 18 30 26 24 34 18 26Z" class="icon-fill-secondary" />
      </g>
    </svg>
  </span>
</template>

<style scoped>
.git-rpg-shield {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  filter: drop-shadow(0 0 3px color-mix(in srgb, var(--rpg-accent) 14%, transparent));
}

.git-rpg-shield-help {
  width: 28px;
  height: 28px;
}

.git-rpg-shield-svg {
  width: 100%;
  height: 100%;
  display: block;
  overflow: visible;
}

.shield-bg {
  fill: color-mix(in srgb, var(--rpg-accent) 13%, var(--card));
  stroke: color-mix(in srgb, var(--rpg-accent) 38%, var(--border));
  stroke-width: 1.2;
}

.shield-inner {
  fill: color-mix(in srgb, var(--rpg-accent) 9%, transparent);
  stroke: color-mix(in srgb, var(--rpg-accent) 20%, transparent);
  stroke-width: 0.85;
}

.icon-stroke {
  fill: none;
  stroke: color-mix(in srgb, var(--rpg-icon-a) 68%, var(--foreground));
  stroke-width: 2.05;
  stroke-linecap: round;
  stroke-linejoin: round;
  transform-box: view-box;
  transform-origin: 24px 26px;
  transform: scale(0.72);
}

.icon-fill-primary {
  fill: color-mix(in srgb, var(--rpg-icon-a) 38%, transparent);
}

.icon-fill-secondary {
  fill: color-mix(in srgb, var(--rpg-icon-b) 42%, transparent);
}

.icon-secondary-stroke {
  stroke: color-mix(in srgb, var(--rpg-icon-b) 70%, var(--foreground));
}

.git-rpg-shield-loading {
  opacity: 0.62;
}
</style>

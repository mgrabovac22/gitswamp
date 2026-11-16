<script setup lang="ts">
import { computed, useId } from "vue";
import { limbTracks, restingLimb, type Limb } from "./petLimbMotion";
import type { ScheduledTrick } from "./petTricks";

const props = defineProps<{ kind: Limb; paint: string; outline: string; texture: string; moving?: boolean; outing?: boolean; schedule?: readonly ScheduledTrick[] }>();
const outlineId = useId();
const rest = computed(() => restingLimb(props.kind));
const tracks = computed(() => props.moving ? limbTracks(props.kind, !!props.outing, props.schedule ?? []) : []);
</script>

<template>
  <g class="pet-limb" :data-limb="kind" stroke-linecap="round" stroke-linejoin="round">
    <defs>
      <path :id="outlineId" class="pet-limb__geometry" :d="rest.skin">
        <animate v-for="track in tracks" :key="track.begin" attributeName="d" :begin="track.begin" :dur="track.dur" :keyTimes="track.times" :values="track.values.skin" :repeatCount="track.repeat" />
      </path>
    </defs>
    <use class="pet-limb__skin" :href="`#${outlineId}`" :fill="paint" />
    <use class="pet-limb__texture" :href="`#${outlineId}`" :fill="texture" opacity=".29" />
    <use :href="`#${outlineId}`" fill="none" :stroke="outline" stroke-width="1.6" />
    <path :d="rest.detail" fill="none" stroke="#4e3e20" stroke-width="1.25" stroke-opacity=".8">
      <animate v-for="track in tracks" :key="track.begin" attributeName="d" :begin="track.begin" :dur="track.dur" :keyTimes="track.times" :values="track.values.detail" :repeatCount="track.repeat" />
    </path>
    <path :d="rest.shine" fill="none" stroke="#dfc66f" stroke-width="1.2" stroke-opacity=".57">
      <animate v-for="track in tracks" :key="track.begin" attributeName="d" :begin="track.begin" :dur="track.dur" :keyTimes="track.times" :values="track.values.shine" :repeatCount="track.repeat" />
    </path>
    <path v-if="rest.claws" :d="rest.claws" fill="#d8ba80" stroke="#493921" stroke-width="1">
      <animate v-for="track in tracks" :key="track.begin" attributeName="d" :begin="track.begin" :dur="track.dur" :keyTimes="track.times" :values="track.values.claws" :repeatCount="track.repeat" />
    </path>
  </g>
</template>

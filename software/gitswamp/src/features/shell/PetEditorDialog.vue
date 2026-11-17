<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef, useId } from "vue";
import { Play, Sparkles } from "lucide-vue-next";
import AppButton from "@/shared/ui/AppButton.vue";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import PetSprite from "./PetSprite.vue";
import { petAppearance, PET_ACCESSORIES, PET_COLORS, PET_HATS, savePetAppearance } from "./petPreferences";
import { PET_TRICKS, type PetTrickId, type ScheduledTrick } from "./petTricks";
import { usePetTrickMotion } from "./usePetTrickMotion";

const emit = defineEmits<{ close: [] }>();
const id = useId();
const dialog = ref<HTMLDialogElement>();
const draft = ref({ ...petAppearance.value });
const preview = ref(0);
const previewing = ref(false);
const selectedTrick = ref<PetTrickId>("somersault");
const schedule = shallowRef<ScheduledTrick[]>([]);
const sprite = ref<HTMLDivElement>();
const shadow = ref<HTMLSpanElement>();
usePetTrickMotion(sprite, shadow, () => schedule.value, true);
let previewTimer: ReturnType<typeof setTimeout> | undefined;
onMounted(() => dialog.value?.showModal());
onUnmounted(() => clearTimeout(previewTimer));

function tryMotion() {
  clearTimeout(previewTimer);
  preview.value++;
  previewing.value = true;
  schedule.value = draft.value.tricks ? [{ id: selectedTrick.value, start: 0, duration: 2.5 }] : [];
  previewTimer = setTimeout(() => { previewing.value = false; schedule.value = []; }, 2500);
}

function save() {
  savePetAppearance(draft.value);
  emit("close");
}
</script>

<template>
  <dialog ref="dialog" class="pet-editor" :aria-labelledby="id" @cancel.prevent="emit('close')">
    <form @submit.prevent="save">
      <header>
        <Sparkles :size="17" aria-hidden="true" /><h2 :id="id">Edit pet</h2>
        <CloseIconButton title="Close pet editor" @click="emit('close')" />
      </header>
      <div class="pet-editor__content">
        <div class="pet-editor__preview">
          <div class="pet-editor__stage">
            <div :key="preview" class="pet-editor__platform">
              <span ref="shadow" class="pet-editor__shadow"></span>
              <div ref="sprite" class="pet-editor__sprite" :class="{ 'pet-editor__sprite--trick': schedule.length }" :data-trick="schedule[0]?.id">
                <PetSprite :appearance="draft" :walking="previewing" :schedule="schedule" />
              </div>
            </div>
          </div>
          <div class="pet-editor__preview-tools">
            <label :for="`${id}-trick`">Trick</label>
            <select :id="`${id}-trick`" v-model="selectedTrick" :disabled="!draft.tricks">
              <option v-for="(item, key) in PET_TRICKS" :key="key" :value="key">{{ item.label }}</option>
            </select>
            <AppButton variant="outline" size="sm" type="button" @click="tryMotion"><Play :size="13" /> Preview motion</AppButton>
          </div>
        </div>
        <section class="pet-editor__controls">
          <div class="pet-editor__field"><label :for="`${id}-name`">Name</label><input :id="`${id}-name`" v-model="draft.name" maxlength="24" name="pet-name" /></div>
          <div class="pet-editor__field"><label :for="`${id}-hat`">Hat</label><select :id="`${id}-hat`" v-model="draft.hat"><option v-for="hat in PET_HATS" :key="hat.id" :value="hat.id">{{ hat.label }}</option></select></div>
          <div class="pet-editor__field"><label :for="`${id}-accessory`">Accessory</label><select :id="`${id}-accessory`" v-model="draft.accessory"><option v-for="item in PET_ACCESSORIES" :key="item.id" :value="item.id">{{ item.label }}</option></select></div>
          <fieldset><legend>Outfit color</legend><label v-for="color in PET_COLORS" :key="color.id" :title="color.label" class="pet-editor__swatch" :style="{ '--swatch': color.value }">
            <input v-model="draft.color" type="radio" name="pet-color" :aria-label="color.label" :value="color.id" /><span></span>
          </label></fieldset>
          <label class="pet-editor__tricks"><input v-model="draft.tricks" type="checkbox" /> Acrobatic tricks</label>
        </section>
      </div>
      <footer>
        <AppButton variant="outline" size="sm" type="button" @click="emit('close')">Cancel</AppButton>
        <AppButton size="sm" type="submit">Save pet</AppButton>
      </footer>
    </form>
  </dialog>
</template>

<style scoped>
.pet-editor { margin: auto; width: min(540px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); padding: 0; border: 1px solid var(--border); border-radius: 8px; background: var(--popover); color: var(--foreground); font-size: 13px; }
.pet-editor::backdrop { background: #0009; }
header, footer { display: flex; align-items: center; gap: 8px; padding: 12px; }
header { border-bottom: 1px solid var(--border); }
header h2 { flex: 1; font-size: 15px; font-weight: 650; }
footer { justify-content: flex-end; border-top: 1px solid var(--border); }
.pet-editor__content { display: grid; grid-template-columns: minmax(140px, 1fr) minmax(0, 1.15fr); }
.pet-editor__preview { display: grid; grid-template-rows: 1fr auto; min-width: 0; background: var(--background); border-right: 1px solid var(--border); }
.pet-editor__stage { display: grid; place-items: end center; overflow: hidden; min-height: 247px; padding-bottom: 10px; }
.pet-editor__platform { position: relative; width: min(100%, 188px); }
.pet-editor__shadow { position: absolute; width: 44%; height: 6px; bottom: 3%; left: 28%; border-radius: 50%; background: #13100b; opacity: .2; }
.pet-editor__sprite { position: relative; }
.pet-editor__preview-tools { display: grid; grid-template-columns: minmax(0, 1fr); gap: 5px; padding: 0 14px 14px; }
.pet-editor__preview-tools button { justify-self: start; }
.pet-editor__controls { padding: 14px; display: grid; gap: 10px; }
.pet-editor__field { display: grid; gap: 4px; }
.pet-editor__controls input:not([type]), .pet-editor select { width: 100%; min-width: 0; min-height: 32px; padding: 4px 8px; border: 1px solid var(--border); border-radius: 4px; background: var(--input-background); color: var(--foreground); }
.pet-editor select:disabled { opacity: .55; }
.pet-editor__controls fieldset { display: flex; gap: 7px; min-width: 0; }
.pet-editor__controls legend { margin-bottom: 4px; }
.pet-editor__swatch { position: relative; display: block; }
.pet-editor__swatch input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
.pet-editor__swatch span { display: block; width: 23px; height: 23px; border: 2px solid var(--border); border-radius: 5px; background: var(--swatch); }
.pet-editor__swatch input:is(:checked, :focus-visible) + span { outline: 2px solid var(--ring); outline-offset: 1px; }
.pet-editor__tricks { display: flex; align-items: center; gap: 6px; }
@media (max-width: 430px) {
  .pet-editor__content { grid-template-columns: 1fr; }
  .pet-editor__preview { border: 0; border-bottom: 1px solid var(--border); }
  .pet-editor__stage { min-height: 177px; }
  .pet-editor__platform { width: 132px; }
  .pet-editor__preview-tools { grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; }
  .pet-editor__preview-tools button { padding-inline: 7px; }
}
</style>

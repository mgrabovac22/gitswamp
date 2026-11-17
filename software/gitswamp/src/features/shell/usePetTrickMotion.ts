import { watchEffect, type Ref } from "vue";
import { trickFrames, type ScheduledTrick } from "./petTricks";

export function usePetTrickMotion(
  body: Ref<HTMLElement | undefined>,
  shadow: Ref<HTMLElement | undefined>,
  schedule: () => readonly ScheduledTrick[],
  preview = false,
) {
  watchEffect(onCleanup => {
    const target = body.value;
    const ground = shadow.value;
    const tricks = schedule();
    if (!target || !tricks.length) return;

    const active: Animation[] = [];
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    const reduced = () => preference.matches || document.documentElement.classList.contains("reduced-motion");
    const cancel = () => { for (const animation of active.splice(0)) animation.cancel(); };
    if (!reduced()) {
      const startTime = document.timeline.currentTime;
      const tail = target.querySelector(".pet-tail");
      for (const { id, start, duration, grand } of tricks) {
        const timing = { delay: start * 1000, duration: duration * 1000 };
        active.push(target.animate(trickFrames(id, "body", grand), timing));
        if (ground && !grand) active.push(ground.animate(trickFrames(id, "shadow"), timing));
        if (tail) active.push(tail.animate(trickFrames(id, "tail"), timing));
      }
      // Limb tracks use the same schedule on the SVG clock; no per-frame JS is needed.
      if (typeof startTime === "number") for (const animation of active) animation.startTime = startTime;
    }
    const sync = () => { if (reduced()) cancel(); };
    const observer = preview ? new MutationObserver(sync) : undefined;
    observer?.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    if (preview) preference.addEventListener("change", sync);
    onCleanup(() => {
      cancel();
      observer?.disconnect();
      if (preview) preference.removeEventListener("change", sync);
    });
  }, { flush: "post" });
}

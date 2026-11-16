import type { Limb } from "./petLimbMotion";

const AIRBORNE = "cubic-bezier(.1,.46,.47,1)";
const FALL = "cubic-bezier(.55,0,.88,.56)";
const RECOVER = "cubic-bezier(.18,.76,.4,1)";

type JointOffset = readonly [x: number, y: number];
export type PetPose = { weight?: number; joints: Partial<Record<Limb, JointOffset>> };
export const PET_POSES = {
  rest: { joints: {} },
  crouch: { weight: 4, joints: { "arm-near": [-11, -6], "arm-far": [3, 12], "leg-near": [-7, 0], "leg-far": [6, 0] } },
  tuck: { joints: { "leg-near": [9, -20], "leg-far": [-9, -16], "arm-near": [10, -13], "arm-far": [-10, -9] } },
  reach: { joints: { "leg-near": [7, -9], "leg-far": [-8, -7], "arm-near": [-18, -54], "arm-far": [12, -57] } },
  star: { joints: { "leg-near": [-21, -13], "leg-far": [26, -16], "arm-near": [-28, -51], "arm-far": [24, -54] } },
  brace: { weight: 5, joints: { "leg-near": [-11, 0], "leg-far": [14, 0], "arm-near": [-21, -3], "arm-far": [19, -5] } },
  cartwheel: { joints: { "leg-near": [-18, -11], "leg-far": [17, -23], "arm-near": [-20, -47], "arm-far": [24, -44] } },
  split: { joints: { "leg-near": [30, -20], "leg-far": [-34, -11], "arm-near": [-18, -45], "arm-far": [19, -30] } },
  balance: { joints: { "leg-near": [12, -19], "leg-far": [18, -13], "arm-near": [-22, -30], "arm-far": [21, -33] } },
  kick: { joints: { "leg-near": [28, -22], "leg-far": [-8, -7], "arm-near": [-17, -35], "arm-far": [-6, -6] } },
  recover: { joints: { "leg-near": [-6, -9], "leg-far": [6, 0], "arm-near": [-7, -11], "arm-far": [4, -4] } },
  wave: { joints: { "arm-near": [-17, -78] } },
  waveLow: { joints: { "arm-near": [-25, -67] } },
  bow: { joints: { "arm-near": [-18, 7], "arm-far": [-3, 12], "leg-near": [-5, 0], "leg-far": [3, 0] } },
  wallReach: { joints: { "arm-near": [66, -85], "arm-far": [19, -41], "leg-near": [34, -43], "leg-far": [26, -20] } },
  wallPull: { joints: { "arm-near": [65, -54], "arm-far": [21, -64], "leg-near": [36, -33], "leg-far": [20, -48] } },
  wallSwap: { joints: { "arm-near": [56, -38], "arm-far": [24, -86], "leg-near": [33, -21], "leg-far": [22, -59] } },
  wallGrip: { joints: { "arm-near": [55, -70], "arm-far": [26, -64], "leg-near": [33, -35], "leg-far": [24, -44] } },
  wallPush: { joints: { "arm-near": [42, -38], "arm-far": [-9, -55], "leg-near": [38, -21], "leg-far": [29, -25] } },
  wallSail: { joints: { "arm-near": [29, -61], "arm-far": [-27, -52], "leg-near": [26, -36], "leg-far": [-27, -29] } },
  wallCatch: { joints: { "arm-near": [68, -74], "arm-far": [23, -76], "leg-near": [27, -48], "leg-far": [20, -27] } },
} satisfies Record<string, PetPose>;

export type PetPoseName = keyof typeof PET_POSES;
export type PetTrickFrame = { at: number; pose?: PetPoseName; x?: number; y?: number; turn?: number; sx?: number; sy?: number; ease?: string };
type PetTrick = { label: string; origin?: string; frames: readonly PetTrickFrame[] };
const finish = (turn = 0): PetTrickFrame => ({ at: 1, turn });

export const PET_TRICKS = {
  somersault: { label: "Forward somersault", frames: [
    { at: 0 }, { at: .14 }, { at: .2, turn: -9, sx: 1.08, sy: .87, pose: "crouch", ease: AIRBORNE },
    { at: .43, y: -60, turn: -173, sx: .94, sy: .96, pose: "tuck", ease: FALL },
    { at: .69, turn: -360, sx: 1.1, sy: .85, pose: "brace", ease: RECOVER },
    { at: .82, y: -3, turn: -362, sy: 1.04 }, finish(-360),
  ] },
  wave: { label: "Double hop salute", frames: [
    { at: 0 }, { at: .12, pose: "crouch", sy: .9, ease: AIRBORNE }, { at: .25, y: -16, turn: -9, pose: "wave" },
    { at: .43, pose: "waveLow", sy: .93, ease: AIRBORNE }, { at: .58, y: -19, turn: 4, pose: "wave" },
    { at: .72, pose: "waveLow", sy: .93 }, { at: .87 }, finish(),
  ] },
  cartwheel: { label: "Cartwheel", origin: "center 70%", frames: [
    { at: 0 }, { at: .15, x: -5, turn: -16, pose: "reach", ease: AIRBORNE },
    { at: .31, x: 1, y: -26, turn: 88, pose: "cartwheel" }, { at: .48, x: 11, y: -44, turn: 183, pose: "star" },
    { at: .64, x: 19, y: -27, turn: 279, pose: "cartwheel", ease: FALL },
    { at: .77, x: 16, turn: 360, sx: 1.04, sy: .9, pose: "brace", ease: RECOVER }, { at: .88, x: 7, turn: 360, pose: "recover" }, finish(360),
  ] },
  star: { label: "Star jump", frames: [
    { at: 0 }, { at: .19, turn: 4, sx: 1.1, sy: .84, pose: "crouch", ease: AIRBORNE },
    { at: .35, y: -41, pose: "reach", sx: .93, sy: 1.04 }, { at: .48, y: -62, pose: "star", sx: 1.04, ease: FALL },
    { at: .66, y: -29, pose: "reach" }, { at: .78, pose: "brace", sx: 1.08, sy: .87 }, { at: .9, y: -3, sy: 1.02 }, finish(),
  ] },
  backflip: { label: "Backflip", frames: [
    { at: 0 }, { at: .16, x: 3, turn: 12, sy: .87, pose: "crouch", ease: AIRBORNE },
    { at: .34, x: -6, y: -44, turn: 87, pose: "reach" }, { at: .48, x: -10, y: -62, turn: 193, pose: "tuck", ease: FALL },
    { at: .64, x: -14, y: -35, turn: 287, pose: "tuck" }, { at: .78, x: -12, turn: 360, sy: .87, pose: "brace", ease: RECOVER }, { at: .89, x: -5, turn: 360, pose: "recover" }, finish(360),
  ] },
  balance: { label: "Tail balance", frames: [
    { at: 0 }, { at: .17, x: 6, turn: 10, pose: "reach" }, { at: .34, x: -8, y: -5, turn: -37, pose: "balance" },
    { at: .5, x: -14, y: -7, turn: -47, pose: "balance" }, { at: .62, x: -12, turn: -41, pose: "kick" },
    { at: .78, x: -5, turn: -16, sy: .91, pose: "brace", ease: RECOVER }, finish(),
  ] },
  split: { label: "Split leap", frames: [
    { at: 0 }, { at: .19, x: -8, turn: -7, sy: .86, pose: "crouch", ease: AIRBORNE },
    { at: .35, x: -2, y: -32, turn: -16, pose: "reach" }, { at: .49, x: 8, y: -57, turn: 7, pose: "split", ease: FALL },
    { at: .63, x: 15, y: -32, turn: 14, pose: "kick" }, { at: .78, x: 14, turn: 9, sy: .86, pose: "brace", ease: RECOVER }, { at: .9, x: 6, turn: 3, pose: "recover" }, finish(),
  ] },
  twist: { label: "Corkscrew", origin: "center 73%", frames: [
    { at: 0 }, { at: .18, x: -6, turn: -19, sy: .86, pose: "crouch", ease: AIRBORNE },
    { at: .32, x: 1, y: -29, turn: 48, sx: .86, pose: "reach" }, { at: .44, x: 6, y: -59, turn: 137, sx: .48, pose: "tuck" },
    { at: .57, x: 13, y: -48, turn: 256, sx: .57, pose: "tuck", ease: FALL },
    { at: .73, x: 17, y: -10, turn: 347, pose: "kick" }, { at: .8, x: 15, turn: 360, sy: .88, pose: "brace", ease: RECOVER }, { at: .91, x: 6, turn: 360, pose: "recover" }, finish(360),
  ] },
  kip: { label: "Roll and spring", origin: "center 76%", frames: [
    { at: 0 }, { at: .14, x: -5, turn: -14, sy: .78, pose: "crouch" }, { at: .31, x: 5, y: -10, turn: 97, sx: .89, sy: .8, pose: "tuck" },
    { at: .48, x: 15, y: -15, turn: 260, sx: .88, sy: .78, pose: "tuck", ease: AIRBORNE },
    { at: .62, x: 19, y: -40, turn: 365, pose: "reach", ease: FALL }, { at: .8, x: 13, turn: 360, sx: 1.08, sy: .86, pose: "brace", ease: RECOVER }, { at: .9, x: 6, turn: 360, pose: "recover" }, finish(360),
  ] },
  victory: { label: "Victory bounce", frames: [
    { at: 0 }, { at: .16, turn: 15, sy: .89, pose: "bow" }, { at: .32, y: -21, turn: -12, pose: "reach" },
    { at: .47, turn: 8, sy: .85, pose: "crouch", ease: AIRBORNE }, { at: .62, y: -32, turn: -9, pose: "star", ease: FALL },
    { at: .77, sy: .9, pose: "brace" }, { at: .88, turn: -3, pose: "reach" }, finish(),
  ] },
} satisfies Record<string, PetTrick>;

export type PetTrickId = keyof typeof PET_TRICKS;
export type ScheduledTrick = { id: PetTrickId; start: number; duration: number; grand?: boolean };
const finales = ["somersault", "cartwheel", "backflip", "twist", "kip"] satisfies PetTrickId[];

export function outingTricks(visit: number): ScheduledTrick[] {
  return [{ id: finales[visit % finales.length], start: 15.4, duration: 2.8, grand: true }];
}

function wallPoses(start: number, end: number, poses: [number, PetPoseName][]) {
  return { start, duration: +(end - start).toFixed(2), frames: poses.map(([time, pose]) => ({
    at: +((time - start) / (end - start)).toFixed(5), pose,
  })) };
}

// The addresses in pet-wall-route use the same seconds on the 20-second timeline.
export const PET_WALL_ASCENT = wallPoses(3.2, 15.4, [
  [3.2, "rest"], [3.5, "wallReach"], [3.98, "wallPull"], [4.28, "wallGrip"],
  [4.56, "wallGrip"], [4.81, "wallSwap"], [5.21, "wallPull"], [5.63, "wallReach"],
  [6.01, "wallGrip"], [6.39, "wallGrip"], [6.75, "wallPull"], [7.03, "wallGrip"],
  [7.3, "wallPush"], [7.87, "split"], [8.53, "tuck"], [9.04, "wallSail"],
  [9.55, "wallCatch"], [9.86, "wallGrip"], [10.18, "wallReach"], [10.5, "wallPull"],
  [10.82, "wallGrip"], [11.23, "wallGrip"], [11.46, "wallSwap"], [11.79, "wallCatch"],
  [12.08, "wallPull"], [12.44, "wallGrip"], [12.72, "wallPush"], [13.3, "wallSail"],
  [14.09, "split"], [14.69, "reach"], [15.14, "tuck"], [15.4, "rest"],
]);

export const PET_WALL_LANDING = wallPoses(18.2, 20, [
  [18.2, "rest"], [18.42, "tuck"], [18.93, "reach"], [19.31, "brace"],
  [19.57, "bow"], [19.85, "rest"], [20, "rest"],
]);

const tailByPose: Partial<Record<PetPoseName, number>> = {
  crouch: -15, tuck: 24, reach: 11, star: 16, brace: -12, cartwheel: 21,
  split: -11, balance: -25, kick: -16, recover: -5, wave: 8, waveLow: -3, bow: 17,
};

export function trickFrames(id: PetTrickId, layer: "body" | "shadow" | "tail" = "body", grand = false): Keyframe[] {
  const trick: PetTrick = PET_TRICKS[id];
  return trick.frames.map(({ at, pose = "rest", x = 0, y = 0, turn = 0, sx = 1, sy = 1, ease }) => {
    const timing = { offset: at, easing: ease ?? "ease-in-out" };
    if (layer === "tail") return { ...timing, transform: `rotate(${tailByPose[pose] ?? 0}deg)` };
    if (layer === "shadow") return {
      ...timing,
      opacity: Math.max(.08, .2 + y * .002),
      transform: `translateX(${x / .44}%) scale(${Math.max(.5, 1 + y * .008)}, ${Math.max(.62, 1 + y * .005)})`,
    };
    const boost = grand ? 1 + Math.sin(at * Math.PI) * .21 : 1;
    return {
      ...timing,
      transform: `translate(${x}%, ${y * (grand ? 1.32 : 1)}%) rotate(${turn}deg) scale(${sx * boost},${sy * boost})`,
      transformOrigin: trick.origin ?? "center 92%",
    };
  });
}

import { PET_POSES, PET_TRICKS, PET_WALL_ASCENT, PET_WALL_LANDING, type PetPose, type PetTrickFrame, type ScheduledTrick } from "./petTricks";

export type Limb = "leg-near" | "leg-far" | "arm-near" | "arm-far";
type Point = { x: number; y: number };
type Pose = { end: Point; lift?: number };
type Shape = { skin: string; detail: string; shine: string; claws: string };
type Track = { begin: string; dur: string; times?: string; values: Shape; repeat: "1" | "indefinite" };

const CYCLE = .66;
const CONTACT = .63;
const STRIDE = 30;
export const PET_WALK_SPEED = STRIDE / (CYCLE * CONTACT * 260);
const SAMPLES = 16;
const transitionCache = new Map<string, Track>();
const walkCache = new Map<Limb, Track>();

const point = (x: number, y: number): Point => ({ x, y });
const plus = (a: Point, b: Point) => point(a.x + b.x, a.y + b.y);
const minus = (a: Point, b: Point) => point(a.x - b.x, a.y - b.y);
const scale = (a: Point, n: number) => point(a.x * n, a.y * n);
const mix = (a: Point, b: Point, t: number) => plus(a, scale(minus(b, a), t));
const shift = (a: Point, x: number, y: number) => point(a.x + x, a.y + y);
const unit = (a: Point) => scale(a, 1 / (Math.hypot(a.x, a.y) || 1));
const normal = (a: Point) => point(-a.y, a.x);
const at = (a: Point) => `${+a.x.toFixed(1)} ${+a.y.toFixed(1)}`;

export const PET_LIMB_ANCHORS = {
  "leg-near": point(104, 132), "leg-far": point(138, 132),
  "arm-near": point(103, 80), "arm-far": point(144, 79),
};

const rig = {
  "leg-near": { rest: point(109, 174), upper: 24, lower: 25, bend: -1, phase: 0, widths: [12, 11.7, 6.3] },
  "leg-far": { rest: point(149, 172), upper: 23, lower: 24, bend: -1, phase: .5, widths: [11, 9.5, 5.5] },
  "arm-near": { rest: point(86, 121), upper: 28, lower: 27, bend: 1, phase: .5, widths: [11, 9.2, 5.7] },
  "arm-far": { rest: point(167, 116), upper: 27, lower: 26, bend: -1, phase: 0, widths: [10, 7.5, 5] },
} as const;

function shape(limb: Limb, pose: Pose): Shape {
  const joint = rig[limb];
  const anchor = PET_LIMB_ANCHORS[limb];
  const start = shift(anchor, 0, (pose.lift ?? 0) * .45);
  const axis = unit(minus(pose.end, start));
  const length = Math.min(Math.hypot(pose.end.x - start.x, pose.end.y - start.y), joint.upper + joint.lower - .05);
  const end = plus(start, scale(axis, length));
  // The elbow or knee is the intersection of the two fixed-length bone circles.
  const pivot = (joint.upper ** 2 - joint.lower ** 2 + length ** 2) / (2 * length);
  const height = Math.sqrt(Math.max(0, joint.upper ** 2 - pivot ** 2));
  const middle = plus(plus(start, scale(axis, pivot)), scale(normal(axis), height * joint.bend));
  const startSide = normal(unit(minus(middle, start)));
  const endSide = normal(unit(minus(end, middle)));
  const midSide = unit(plus(startSide, endSide));
  const [a, b, c] = joint.widths;
  // Keep the cross-section inside the torso, even when the arm rises for a wave.
  const rootSide = normal(unit(minus(joint.rest, anchor)));
  const s0 = plus(start, scale(rootSide, a));
  const s1 = minus(start, scale(rootSide, a));
  const m0 = plus(middle, scale(midSide, b));
  const m1 = minus(middle, scale(midSide, b));
  const e0 = plus(end, scale(endSide, c));
  const e1 = minus(end, scale(endSide, c));
  // Do not stroke a cap across the shoulder or hip, where it blends into the body.
  const lead = `M${at(s0)} Q${at(mix(s0, m0, .3))} ${at(mix(s0, m0, .7))} Q${at(m0)} ${at(mix(m0, e0, .38))} Q${at(mix(m0, e0, .72))} ${at(e0)}`;
  const follow = `Q${at(mix(e1, m1, .34))} ${at(mix(e1, m1, .68))} Q${at(m1)} ${at(mix(m1, s1, .38))} Q${at(mix(m1, s1, .73))} ${at(s1)}`;
  const fold = mix(middle, m1, .5);
  const crease = (from: Point, to: Point, side: Point, t: number, width: number, radius: number) => {
    const center = plus(mix(from, to, t), scale(side, width));
    const tangent = unit(minus(to, from));
    return `M${at(minus(center, scale(tangent, radius)))} Q${at(plus(center, scale(side, radius)))} ${at(plus(center, scale(tangent, radius)))}`;
  };
  const scales = [
    crease(start, middle, startSide, .52, a * .39, 2.1),
    crease(start, middle, startSide, .74, a * .58, 1.6),
    crease(middle, end, endSide, .31, c * .42, 1.55),
    crease(middle, end, endSide, .6, c * .56, 1.3),
  ].join(" ");
  const detail = `M${at(mix(s1, m1, .44))} Q${at(mix(s1, m1, .86))} ${at(fold)} ${scales}`;
  const highlight = `M${at(mix(mix(s0, start, .16), mix(m0, middle, .2), .31))} Q${at(mix(m0, middle, .22))} ${at(mix(mix(m0, middle, .25), mix(e0, end, .28), .67))}`;

  if (limb.startsWith("leg")) {
    const heel = shift(end, -7, 8);
    const sole = shift(end, 18, 9);
    const toe = shift(end, 23, 5);
    return {
      skin: `${lead} Q${at(shift(heel, -4, 0))} ${at(heel)} Q${at(shift(end, 4, 11))} ${at(sole)} Q${at(shift(toe, 3, -1))} ${at(toe)} Q${at(shift(end, 13, -1))} ${at(e1)} ${follow}`,
      detail,
      shine: `${highlight} M${at(shift(end, 7, 4))} Q${at(shift(end, 15, 1))} ${at(shift(toe, -1, 2))}`,
      claws: `M${at(shift(sole, -7, -1))} Q${at(shift(sole, -4, -9))} ${at(shift(sole, -1, -1))}Z M${at(shift(sole, -1, -1))} Q${at(shift(toe, 3, -5))} ${at(shift(sole, 4, -2))}Z`,
    };
  }

  const palm = shift(end, -1, 2);
  const forward = scale(endSide, -1);
  const along = unit(minus(end, middle));
  const digit = (x: number, y: number) => plus(palm, plus(scale(forward, x), scale(along, y)));
  return {
    skin: `${lead} Q${at(digit(-9, 1))} ${at(digit(-7, 6))} Q${at(digit(-2, 10))} ${at(digit(3, 7))} Q${at(digit(10, 7))} ${at(digit(9, 3))} Q${at(digit(15, -2))} ${at(digit(11, -4))} Q${at(digit(8, -3))} ${at(e1)} ${follow}`,
    detail: `${detail} M${at(digit(-3, 6))} Q${at(digit(0, 0))} ${at(digit(4, 4))} M${at(digit(4, 6))} Q${at(digit(4, 1))} ${at(digit(8, 3))}`,
    shine: `${highlight} M${at(digit(-6, 4))} Q${at(digit(-3, 8))} ${at(digit(0, 6))}`,
    claws: "",
  };
}

function step(limb: Limb, time: number): Pose {
  const joint = rig[limb];
  const phase = (time / CYCLE + joint.phase) % 1;
  const bob = Math.cos(4 * Math.PI * phase) * 1.4;
  if (limb.startsWith("arm")) return { end: shift(joint.rest, Math.cos(2 * Math.PI * phase) * 12, Math.sin(2 * Math.PI * phase) * 4), lift: bob };
  if (phase <= CONTACT) return { end: shift(joint.rest, STRIDE / 2 - STRIDE * phase / CONTACT, 0), lift: bob };
  const swing = (phase - CONTACT) / (1 - CONTACT);
  return { end: shift(joint.rest, -STRIDE / 2 + STRIDE * swing * swing * (3 - 2 * swing), -Math.sin(Math.PI * swing) * 11), lift: bob };
}

const keys = (frames: Shape[]) => ({
  skin: frames.map(frame => frame.skin).join(";"), detail: frames.map(frame => frame.detail).join(";"),
  shine: frames.map(frame => frame.shine).join(";"),
  claws: frames.map(frame => frame.claws).join(";"),
});

function restPose(limb: Limb, lift = 0): Pose { return { end: rig[limb].rest, lift }; }

export function restingLimb(limb: Limb): Shape { return shape(limb, restPose(limb)); }

function walkTrack(limb: Limb): Track {
  const existing = walkCache.get(limb);
  if (existing) return existing;
  const phases = Array.from({ length: SAMPLES + 1 }, (_, i) => i / SAMPLES);
  if (limb.startsWith("leg")) {
    phases.push((CONTACT - rig[limb].phase + 1) % 1, 1 - rig[limb].phase);
    phases.sort((a, b) => a - b);
  }
  const unique = [...new Set(phases)];
  const track: Track = { begin: "0s", dur: `${CYCLE}s`, repeat: "indefinite", times: unique.map(value => +value.toFixed(5)).join(";"), values: keys(unique.map(phase => shape(limb, step(limb, phase === 1 ? 0 : phase * CYCLE)))) };
  walkCache.set(limb, track);
  return track;
}

function transition(limb: Limb, name: string, start: number, duration: number, frames: readonly PetTrickFrame[], walkStart = false, walkEnd = false): Track {
  const key = `${limb}/${name}/${start}/${duration}`;
  const existing = transitionCache.get(key);
  if (existing) return existing;
  const poses = frames.map(({ at: time, pose: preset }) => {
    if ((walkStart && time === 0) || (walkEnd && time === 1)) return step(limb, start + time * duration);
    const pose: PetPose = PET_POSES[preset ?? "rest"];
    const [x, y] = pose.joints[limb] ?? [0, 0];
    return { end: shift(rig[limb].rest, x, y), lift: pose.weight };
  });
  const track: Track = { begin: `${start}s`, dur: `${duration}s`, repeat: "1", times: frames.map(frame => frame.at).join(";"), values: keys(poses.map(pose => shape(limb, pose))) };
  transitionCache.set(key, track);
  return track;
}

const turnFrames = [{ at: 0 }, { at: .23 }, { at: .76 }, { at: 1 }];
export function limbTracks(limb: Limb, outing: boolean, schedule: readonly ScheduledTrick[]): Track[] {
  const tracks = outing || !schedule.length ? [walkTrack(limb)] : [];
  const wallShow = outing && schedule.some(trick => trick.grand);
  if (wallShow) tracks.push(transition(limb, "wall-course", PET_WALL_ASCENT.start, PET_WALL_ASCENT.duration, PET_WALL_ASCENT.frames, true));
  else if (outing) tracks.push(transition(limb, "turn", 9.8, .8, turnFrames, true, true));
  for (const { id, start, duration } of schedule) {
    tracks.push(transition(limb, id, start, duration, PET_TRICKS[id].frames, outing && !wallShow, outing && !wallShow));
  }
  if (wallShow) tracks.push(transition(limb, "wall-landing", PET_WALL_LANDING.start, PET_WALL_LANDING.duration, PET_WALL_LANDING.frames));
  return tracks;
}

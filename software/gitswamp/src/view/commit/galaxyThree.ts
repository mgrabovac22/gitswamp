import {
  AdditiveBlending, Box3, BufferGeometry, Color, Float32BufferAttribute, Group,
  LineBasicMaterial, LineSegments, MOUSE, PerspectiveCamera, Points, Scene,
  ShaderMaterial, Sphere, Vector3, WebGLRenderer,
} from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import {
  canOrbit, GALAXY_ARMS, GALAXY_RADIUS, orbitDelta, spiralPoint,
  type GalaxyLayout, type GalaxyMotion, type GalaxyNode, type GalaxyScene, type OrbitState,
} from "./galaxyLayout";

export const GALAXY_DUST_COUNT = 16000;
const MAX_PIXEL_RATIO = 1.5;
type StarCloud = Points<BufferGeometry, ShaderMaterial>;
type Lines = LineSegments<BufferGeometry, LineBasicMaterial>;
export type GalaxyOptions = { guides: boolean; rings: boolean; aura: boolean; depth: boolean; merges: boolean };

function stars(positions: number[], colors: number[], sizes: number[]): StarCloud {
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new Float32BufferAttribute(positions, 3));
  geometry.setAttribute("color", new Float32BufferAttribute(colors, 3));
  geometry.setAttribute("size", new Float32BufferAttribute(sizes, 1));
  const material = new ShaderMaterial({
    uniforms: { pixelRatio: { value: 1 }, depthFade: { value: 0 }, opacity: { value: 1 } },
    vertexColors: true, transparent: true, depthWrite: false, blending: AdditiveBlending,
    vertexShader: `
      attribute float size;
      uniform float pixelRatio;
      uniform float depthFade;
      varying vec3 starColor;
      varying float visibility;
      void main() {
        vec4 point = modelViewMatrix * vec4(position, 1.0);
        gl_Position = projectionMatrix * point;
        gl_PointSize = clamp(size * pixelRatio * 1300.0 / max(80.0, -point.z), pixelRatio, 28.0 * pixelRatio);
        starColor = color;
        visibility = mix(1.0, clamp(1800.0 / max(1.0, -point.z), 0.45, 1.0), depthFade);
      }`,
    fragmentShader: `
      uniform float opacity;
      varying vec3 starColor;
      varying float visibility;
      void main() {
        float radius = length(gl_PointCoord - 0.5) * 2.0;
        if (radius > 1.0) discard;
        float light = exp(-5.0 * radius * radius) * (1.0 - smoothstep(0.7, 1.0, radius));
        gl_FragColor = vec4(starColor, light * opacity * visibility);
        #include <tonemapping_fragment>
        #include <colorspace_fragment>
      }`,
  });
  return new Points(geometry, material);
}

function lines(positions: number[], color: string, opacity: number, colors?: number[]): Lines {
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new Float32BufferAttribute(positions, 3));
  if (colors) geometry.setAttribute("color", new Float32BufferAttribute(colors, 3));
  return new LineSegments(geometry, new LineBasicMaterial({
    color, vertexColors: !!colors, transparent: true, opacity, depthWrite: false,
  }));
}

function disposeGroup(group: Group) {
  group.traverse(object => {
    if (object instanceof Points || object instanceof LineSegments) {
      object.geometry.dispose();
      const materials = Array.isArray(object.material) ? object.material : [object.material];
      materials.forEach(material => material.dispose());
    }
  });
  group.clear();
}

function makeDust(): StarCloud {
  let seed = 71391;
  const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296; };
  const positions: number[] = [], colors: number[] = [], sizes: number[] = [];
  const warm = new Color("#f4d7a3"), cool = new Color("#9bcce0"), white = new Color("#f5efdf");
  const color = new Color();
  for (let i = 0; i < GALAXY_DUST_COUNT; i++) {
    const core = i < 3400;
    const progress = Math.pow(random(), 0.78);
    const arm = i % GALAXY_ARMS;
    const point = spiralPoint(progress, arm);
    const angle = Math.atan2(point[2], point[0]);
    const spread = (random() + random() + random() - 1.5);
    const radius = core ? Math.pow(random(), 1.8) * 110 : Math.hypot(point[0], point[2]) + spread * 20;
    const theta = core ? random() * Math.PI * 2 : angle + spread * (0.11 + 0.12 * (1 - progress));
    const height = (random() + random() - 1) * (core ? 28 * (1 - radius / 130) : 9 * (1 - progress) + 2);
    positions.push(Math.cos(theta) * radius, height, Math.sin(theta) * radius);
    color.copy(warm).lerp(cool, core ? radius / 220 : Math.min(1, progress + 0.18)).lerp(white, random() * 0.45);
    color.multiplyScalar(core ? 0.65 : 0.45 + random() * 0.4);
    colors.push(color.r, color.g, color.b);
    sizes.push(core ? 2 + random() * 3 : 1.2 + random() * 2.8);
  }
  return stars(positions, colors, sizes);
}

export class GalaxyThree {
  readonly renderer: WebGLRenderer;
  readonly camera = new PerspectiveCamera(42, 1, 0.1, 50000);
  readonly controls: OrbitControls;
  private readonly scene = new Scene();
  private readonly content = new Group();
  private readonly atmosphere = new Group();
  private readonly selection = new Group();
  private readonly dust = makeDust();
  private readonly vector = new Vector3();
  private readonly bounds = new Sphere(new Vector3(), GALAXY_RADIUS);
  private model: GalaxyScene = { nodes: [], edges: [], lanes: [] };
  private layout: GalaxyLayout = "spiral";
  private options: GalaxyOptions = { guides: true, rings: false, aura: true, depth: true, merges: true };
  private orbit: OrbitState = {
    motion: "threeD", layout: "spiral", hidden: document.hidden, reducedMotion: false,
    interacting: false, hovering: false, resumeAt: 0,
  };
  private frame: number | null = null;
  private previousTime: number | null = null;
  private disposed = false;
  private width = 1;
  private height = 1;
  private fittedDistance = 1400;
  private pristine = true;
  private selectedSha: string | null = null;
  private hoveredSha: string | null = null;
  private edgeLines?: Lines;
  private mergeStars?: StarCloud;
  private ringLines?: Lines;
  private readonly reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");

  constructor(canvas: HTMLCanvasElement, private readonly onFrame: () => void, private readonly onLost: () => void) {
    this.renderer = new WebGLRenderer({ canvas, antialias: true, alpha: false, powerPreference: "low-power" });
    this.renderer.setClearColor("#050609");
    this.renderer.setPixelRatio(Math.min(MAX_PIXEL_RATIO, window.devicePixelRatio || 1));
    this.scene.add(this.content, this.atmosphere, this.selection);
    this.atmosphere.add(this.dust);
    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.09;
    this.controls.rotateSpeed = 0.55;
    this.controls.zoomSpeed = 0.8;
    this.controls.autoRotateSpeed = 0.45;
    this.controls.minPolarAngle = 0.12;
    this.controls.maxPolarAngle = Math.PI - 0.12;
    this.controls.addEventListener("change", this.requestFrame);
    this.controls.addEventListener("start", this.onInteractionStart);
    this.controls.addEventListener("end", this.onInteractionEnd);
    this.orbit.reducedMotion = this.reducedMotion.matches;
    this.reducedMotion.addEventListener("change", this.onReducedMotion);
    document.addEventListener("visibilitychange", this.onVisibility);
    canvas.addEventListener("webglcontextlost", this.onContextLost);
    this.reset();
  }

  private onContextLost = (event: Event) => {
    event.preventDefault();
    this.suspend();
    this.onLost();
  };
  private onReducedMotion = () => {
    this.orbit.reducedMotion = this.reducedMotion.matches;
    this.previousTime = null;
    this.requestFrame();
  };
  private onVisibility = () => {
    this.orbit.hidden = document.hidden;
    this.previousTime = null;
    if (document.hidden) this.suspend();
    else this.requestFrame();
  };
  private onInteractionStart = () => {
    this.pristine = false;
    this.orbit.interacting = true;
    this.controls.autoRotate = false;
    this.requestFrame();
  };
  private onInteractionEnd = () => {
    this.orbit.interacting = false;
    this.orbit.resumeAt = performance.now() + 1400;
    this.requestFrame();
  };

  private suspend() {
    if (this.frame !== null) cancelAnimationFrame(this.frame);
    this.frame = null;
    this.previousTime = null;
  }

  requestFrame = () => {
    if (this.disposed || this.orbit.hidden || this.frame !== null) return;
    this.frame = requestAnimationFrame(this.draw);
  };

  private draw = (now: number) => {
    this.frame = null;
    if (this.disposed || this.orbit.hidden) return;
    this.controls.autoRotate = canOrbit(this.orbit, now);
    const moving = this.controls.update(orbitDelta(this.previousTime, now));
    this.previousTime = now;
    this.renderer.render(this.scene, this.camera);
    this.onFrame();
    if (moving || this.controls.autoRotate || (this.orbit.motion === "circling"
      && !this.orbit.reducedMotion && this.layout !== "tree" && now < this.orbit.resumeAt)) this.requestFrame();
    else this.previousTime = null;
  };

  resize(width: number, height: number) {
    this.width = Math.max(1, width);
    this.height = Math.max(1, height);
    this.renderer.setSize(this.width, this.height, false);
    this.camera.aspect = this.width / this.height;
    this.camera.updateProjectionMatrix();
    this.updateFitDistance();
    if (this.pristine) this.reset();
    this.requestFrame();
  }

  setScene(model: GalaxyScene, layout: GalaxyLayout) {
    const layoutChanged = layout !== this.layout;
    this.model = model;
    this.layout = layout;
    this.orbit.layout = layout;
    disposeGroup(this.content);
    this.edgeLines = undefined;
    this.mergeStars = undefined;
    this.ringLines = undefined;
    const positions: number[] = [], colors: number[] = [], sizes: number[] = [];
    const mergePositions: number[] = [], mergeColors: number[] = [], mergeSizes: number[] = [];
    const box = new Box3();
    const color = new Color();
    for (const node of model.nodes) {
      const point = node.positions[layout];
      positions.push(...point);
      color.set(node.color);
      colors.push(color.r, color.g, color.b);
      sizes.push(node.label ? 9 : model.nodes.length > 6000 ? 3.4 : 6);
      box.expandByPoint(this.vector.fromArray(point));
      if (node.commit.parent_shas.length > 1) {
        mergePositions.push(...point);
        mergeColors.push(1, 0.5, 0.22);
        mergeSizes.push(12);
      }
    }
    this.content.add(stars(positions, colors, sizes));
    this.mergeStars = stars(mergePositions, mergeColors, mergeSizes);
    this.content.add(this.mergeStars);
    const edgePositions: number[] = [], edgeColors: number[] = [];
    for (const [from, to] of model.edges) {
      const a = model.nodes[from], b = model.nodes[to];
      color.set(b.color);
      const curved = layout === "spiral" && a.lane === b.lane;
      const steps = curved ? 3 : 1;
      let last = a.positions[layout];
      for (let step = 1; step <= steps; step++) {
        const t = step / steps;
        const next = step === steps ? b.positions[layout] : spiralPoint(a.progress + (b.progress - a.progress) * t,
          a.lane, last[1] + (b.positions[layout][1] - last[1]) * t);
        edgePositions.push(...last, ...next);
        edgeColors.push(color.r, color.g, color.b, color.r, color.g, color.b);
        last = next;
      }
    }
    this.edgeLines = lines(edgePositions, "#ffffff", layout === "spiral" ? 0.14 : 0.32, edgeColors);
    this.content.add(this.edgeLines);
    const rings: number[] = [];
    for (const progress of [0.25, 0.5, 0.75, 1]) {
      const radius = 36 + progress * (GALAXY_RADIUS - 36);
      for (let i = 0; i < 128; i++) {
        for (const angle of [i / 128 * Math.PI * 2, (i + 1) / 128 * Math.PI * 2]) {
          rings.push(Math.cos(angle) * radius, -5, Math.sin(angle) * radius);
        }
      }
    }
    this.ringLines = lines(rings, "#c4cbd1", 0.09);
    this.content.add(this.ringLines);
    if (layout === "spiral" || !model.nodes.length) this.bounds.set(new Vector3(), GALAXY_RADIUS + 24);
    else box.getBoundingSphere(this.bounds);
    this.bounds.radius = Math.max(80, this.bounds.radius);
    this.updateFitDistance();
    this.setOptions(this.options);
    this.setHighlight(this.selectedSha, this.hoveredSha);
    if (layoutChanged || this.pristine) this.reset();
    this.requestFrame();
  }

  private updateFitDistance() {
    const vertical = this.camera.fov * Math.PI / 360;
    const horizontal = Math.atan(Math.tan(vertical) * this.camera.aspect);
    this.fittedDistance = this.bounds.radius / Math.sin(Math.min(vertical, horizontal)) * 1.12;
    this.controls.minDistance = Math.max(30, this.bounds.radius * 0.12);
    this.controls.maxDistance = Math.max(4000, this.fittedDistance * 3);
    this.camera.far = Math.max(10000, this.controls.maxDistance * 2);
    this.camera.updateProjectionMatrix();
  }

  setMotion(motion: GalaxyMotion) {
    this.orbit.motion = motion;
    this.controls.autoRotate = false;
    this.previousTime = null;
    if (this.pristine) this.reset();
    this.requestFrame();
  }

  setOptions(options: GalaxyOptions) {
    this.options = { ...options };
    this.dust.visible = options.aura && this.layout === "spiral" && this.model.nodes.length > 0;
    if (this.edgeLines) this.edgeLines.visible = options.guides;
    if (this.mergeStars) this.mergeStars.visible = options.merges;
    if (this.ringLines) this.ringLines.visible = options.rings && this.layout === "spiral";
    this.scene.traverse(object => {
      if (object instanceof Points && object.material instanceof ShaderMaterial) {
        object.material.uniforms.pixelRatio.value = this.renderer.getPixelRatio();
        object.material.uniforms.depthFade.value = Number(options.depth);
      }
    });
    this.requestFrame();
  }

  setHighlight(selected: string | null, hovered: string | null) {
    this.selectedSha = selected;
    this.hoveredSha = hovered;
    this.orbit.hovering = !!hovered;
    disposeGroup(this.selection);
    const positions: number[] = [], colors: number[] = [], sizes: number[] = [], connections: number[] = [];
    for (const node of this.model.nodes) {
      if (node.commit.sha !== selected && node.commit.sha !== hovered) continue;
      positions.push(...node.positions[this.layout]); colors.push(1, 1, 1); sizes.push(16);
    }
    for (const [a, b] of this.model.edges) {
      const from = this.model.nodes[a], to = this.model.nodes[b];
      if ([selected, hovered].some(sha => sha && (from.commit.sha === sha || to.commit.sha === sha))) {
        connections.push(...from.positions[this.layout], ...to.positions[this.layout]);
      }
    }
    const points = stars(positions, colors, sizes);
    points.material.uniforms.pixelRatio.value = this.renderer.getPixelRatio();
    this.selection.add(points, lines(connections, "#f3eedc", 0.8));
    this.requestFrame();
  }

  reset() {
    this.controls.autoRotate = false;
    this.controls.enableDamping = false;
    this.controls.update(0);
    this.controls.target.copy(this.bounds.center);
    const spiral = this.layout === "spiral";
    const elevation = this.orbit.motion === "balanced" ? 1.25 : 0.76;
    this.camera.position.copy(this.bounds.center).add(spiral
      ? new Vector3(0.12, Math.sin(elevation), Math.cos(elevation)).normalize().multiplyScalar(this.fittedDistance)
      : new Vector3(0, 0, this.fittedDistance));
    this.controls.enableRotate = this.layout !== "tree";
    this.controls.mouseButtons.LEFT = this.layout === "tree" ? MOUSE.PAN : MOUSE.ROTATE;
    this.controls.update(0);
    this.controls.enableDamping = !this.orbit.reducedMotion;
    this.pristine = true;
    this.previousTime = null;
    this.requestFrame();
  }

  focus(sha: string) {
    const node = this.model.nodes.find(item => item.commit.sha === sha);
    if (!node) return;
    this.controls.autoRotate = false;
    this.controls.enableDamping = false;
    this.controls.update(0);
    const offset = this.camera.position.clone().sub(this.controls.target).normalize();
    this.controls.target.fromArray(node.positions[this.layout]);
    this.camera.position.copy(this.controls.target).addScaledVector(offset, Math.min(this.controls.getDistance(), 280));
    this.controls.update(0);
    this.controls.enableDamping = !this.orbit.reducedMotion;
    this.orbit.resumeAt = performance.now() + 1400;
    this.pristine = false;
    this.requestFrame();
  }

  project(node: GalaxyNode): { x: number; y: number; visible: boolean } {
    this.vector.fromArray(node.positions[this.layout]).project(this.camera);
    return { x: (this.vector.x + 1) * this.width / 2, y: (1 - this.vector.y) * this.height / 2,
      visible: this.vector.z > -1 && this.vector.z < 1 && Math.abs(this.vector.x) < 1 && Math.abs(this.vector.y) < 1 };
  }

  pick(x: number, y: number): GalaxyNode | null {
    let best: GalaxyNode | null = null, score = Infinity;
    for (const node of this.model.nodes) {
      this.vector.fromArray(node.positions[this.layout]).project(this.camera);
      if (this.vector.z < -1 || this.vector.z > 1) continue;
      const distance = Math.hypot((this.vector.x + 1) * this.width / 2 - x, (1 - this.vector.y) * this.height / 2 - y);
      const radius = node.label ? 11 : 7;
      const candidate = distance / radius + this.vector.z * 0.001;
      if (distance < radius && candidate < score) { best = node; score = candidate; }
    }
    return best;
  }

  dispose() {
    this.disposed = true;
    this.suspend();
    document.removeEventListener("visibilitychange", this.onVisibility);
    this.reducedMotion.removeEventListener("change", this.onReducedMotion);
    this.renderer.domElement.removeEventListener("webglcontextlost", this.onContextLost);
    this.controls.removeEventListener("change", this.requestFrame);
    this.controls.removeEventListener("start", this.onInteractionStart);
    this.controls.removeEventListener("end", this.onInteractionEnd);
    this.controls.dispose();
    disposeGroup(this.content);
    disposeGroup(this.atmosphere);
    disposeGroup(this.selection);
    this.renderer.dispose();
    this.renderer.forceContextLoss();
    this.model = { nodes: [], edges: [], lanes: [] };
  }
}

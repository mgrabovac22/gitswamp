import {
  ACESFilmicToneMapping,
  AmbientLight,
  BoxGeometry,
  BufferGeometry,
  Color,
  ConeGeometry,
  CylinderGeometry,
  DirectionalLight,
  DynamicDrawUsage,
  Euler,
  FogExp2,
  Float32BufferAttribute,
  Group,
  HemisphereLight,
  InstancedMesh,
  MathUtils,
  Matrix4,
  Mesh,
  MeshStandardMaterial,
  PerspectiveCamera,
  PlaneGeometry,
  Quaternion,
  Raycaster,
  Scene,
  SphereGeometry,
  SRGBColorSpace,
  StaticDrawUsage,
  Texture,
  TorusGeometry,
  Vector2,
  Vector3,
  WebGLRenderer,
} from "three";
import type { Light, Material } from "three";
import type {
  CityBuilding,
  CityCameraMode,
  CityDistrict,
  CityPark,
  CityRadarMarker,
  CityRoad,
  CityScene,
} from "./repositoryCity.types";

const CLEAR_COLOR = 0x0a1320;
const MAX_PIXEL_RATIO = 1.25;
const MAX_TREES = 960;
const MAX_ROOF_MARKERS = 180;
const MAX_ROOF_DETAILS = 620;
const MAX_LANE_MARKS = 1800;
const MAX_TRAFFIC_LIGHTS = 96;
const MAX_WAVE_LINES = 240;
const MAX_GRASS_PATCHES = 2200;
const MAX_MEADOW_DETAILS = 1500;
const MAX_COASTAL_DETAILS = 520;
const MAX_SHRUBS = 820;
const MAX_FOREST_FLOOR_DETAILS = 980;
const MAX_LAKE_EDGE_DETAILS = 760;
const MAX_PARK_ENTITY_PARTS = 720;
const MAX_PARK_CREATURES = 96;
const MAX_SIDEWALK_CORNERS = 1200;
const MAX_PARK_LAKES = 42;
const MAX_CANNONBALLS = 32;
const MAX_DEBRIS_BLOCKS = 720;
const MAX_IMPACT_PARTICLES = 192;
const VISUAL_ROAD_WIDTH = 12;
const SIDEWALK_WIDTH = 2.2;
const SHORE_BELT_WIDTH = 18;
const GROUND_COLOR = 0x123421;
const ROAD_COLOR = 0x10171a;
const SIDEWALK_COLOR = 0x535a63;
const WALK_EYE_HEIGHT = 4.8;
const WALK_STEP = 0.72;
const WALK_BOB_AMOUNT = 0.12;
const WALK_ROAD_PADDING = 5;
const BOAT_STEP = 1.32;
const BOAT_TURN_STEP = 0.048;
const CANNON_GRAVITY = 19.5;

export interface CityScreenLabel {
  id: string;
  name: string;
  fileCount: number;
  x: number;
  y: number;
  visible: boolean;
}

export interface CityBoatInfo {
  id: string;
  name: string;
  kind: string;
}

interface BuildingVisual {
  building: CityBuilding;
  index: number;
  baseColor: Color;
}

interface ParkLake {
  x: number;
  y: number;
  radiusX: number;
  radiusY: number;
}

interface WaveInstance {
  x: number;
  z: number;
  rotation: number;
  length: number;
  depth: number;
  phase: number;
  travel: number;
  speed: number;
  driftX: number;
  driftZ: number;
  amplitude: number;
}

interface DockLayout {
  id: string;
  name: string;
  x: number;
  z: number;
  length: number;
  width: number;
  outward: -1 | 1;
}

interface BoatVisual extends CityBoatInfo {
  group: Group;
  yaw: number;
  bobPhase: number;
}

interface CannonBall {
  active: boolean;
  x: number;
  y: number;
  z: number;
  vx: number;
  vy: number;
  vz: number;
  age: number;
}

interface DebrisBlock {
  active: boolean;
  sleeping: boolean;
  x: number;
  y: number;
  z: number;
  vx: number;
  vy: number;
  vz: number;
  rx: number;
  ry: number;
  rz: number;
  vrx: number;
  vry: number;
  vrz: number;
  sx: number;
  sy: number;
  sz: number;
  age: number;
}

interface ImpactParticle {
  active: boolean;
  x: number;
  y: number;
  z: number;
  vx: number;
  vy: number;
  vz: number;
  scale: number;
  gravity: number;
  age: number;
  ttl: number;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function hashText(value: string): number {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function heatColor(heat: number, enabled: boolean): Color {
  if (!enabled) return new Color("#38bdf8");
  if (heat >= 0.78) return new Color("#f43f5e");
  if (heat >= 0.52) return new Color("#f59e0b");
  if (heat >= 0.24) return new Color("#38bdf8");
  return new Color("#34d399");
}

function markerColor(value: string): Color {
  return new Color(value || "#2dd4bf");
}

function roadOrientation(road: CityRoad): "horizontal" | "vertical" {
  return road.width >= road.depth ? "horizontal" : "vertical";
}

function roadIntersects(a: CityRoad, b: CityRoad): boolean {
  return a.x < b.x + b.width
    && a.x + a.width > b.x
    && a.y < b.y + b.depth
    && a.y + a.depth > b.y;
}

function pointNearRoad(x: number, y: number, roads: CityRoad[], padding: number): boolean {
  return roads.some((road) =>
    x >= road.x - padding
    && x <= road.x + road.width + padding
    && y >= road.y - padding
    && y <= road.y + road.depth + padding,
  );
}

function parkLakeFor(park: CityPark): ParkLake | null {
  if (park.width < 34 || park.depth < 30) return null;
  const seed = hashText(`city-lake:${park.id}`);
  const largeParkBonus = park.width * park.depth > 2200 ? 1 : 0;
  if ((seed + largeParkBonus) % 4 === 0) return null;
  const radiusX = clamp(park.width * (0.13 + ((seed >>> 7) % 9) / 100), 5.5, Math.min(park.width * 0.34, 18));
  const radiusY = clamp(park.depth * (0.12 + ((seed >>> 17) % 8) / 100), 4.8, Math.min(park.depth * 0.32, 15));
  const usableWidth = Math.max(1, park.width - radiusX * 2 - 6);
  const usableDepth = Math.max(1, park.depth - radiusY * 2 - 6);
  return {
    x: park.x + radiusX + 3 + ((seed >>> 3) % 1000) / 1000 * usableWidth,
    y: park.y + radiusY + 3 + ((seed >>> 13) % 1000) / 1000 * usableDepth,
    radiusX,
    radiusY,
  };
}

function pointInLake(x: number, y: number, lake: ParkLake | null, padding = 0): boolean {
  if (!lake) return false;
  const radiusX = lake.radiusX + padding;
  const radiusY = lake.radiusY + padding;
  const dx = (x - lake.x) / Math.max(0.1, radiusX);
  const dy = (y - lake.y) / Math.max(0.1, radiusY);
  return dx * dx + dy * dy <= 1;
}

function segmentBoxHit(
  startX: number,
  startY: number,
  startZ: number,
  endX: number,
  endY: number,
  endZ: number,
  minX: number,
  minY: number,
  minZ: number,
  maxX: number,
  maxY: number,
  maxZ: number,
): number | null {
  let near = 0;
  let far = 1;
  let delta = endX - startX;
  let first = 0;
  let second = 0;
  if (Math.abs(delta) < 0.00001) {
    if (startX < minX || startX > maxX) return null;
  } else {
    first = (minX - startX) / delta;
    second = (maxX - startX) / delta;
    if (first > second) [first, second] = [second, first];
    near = Math.max(near, first);
    far = Math.min(far, second);
    if (near > far) return null;
  }
  delta = endY - startY;
  if (Math.abs(delta) < 0.00001) {
    if (startY < minY || startY > maxY) return null;
  } else {
    first = (minY - startY) / delta;
    second = (maxY - startY) / delta;
    if (first > second) [first, second] = [second, first];
    near = Math.max(near, first);
    far = Math.min(far, second);
    if (near > far) return null;
  }
  delta = endZ - startZ;
  if (Math.abs(delta) < 0.00001) {
    if (startZ < minZ || startZ > maxZ) return null;
  } else {
    first = (minZ - startZ) / delta;
    second = (maxZ - startZ) / delta;
    if (first > second) [first, second] = [second, first];
    near = Math.max(near, first);
    far = Math.min(far, second);
    if (near > far) return null;
  }
  return near >= 0 && near <= 1 ? near : null;
}

function createBoatHullGeometry(): BufferGeometry {
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new Float32BufferAttribute([
    -0.72, 0.42, -0.5, 0.72, 0.42, -0.5,
    -0.46, -0.46, -0.44, 0.46, -0.46, -0.44,
    -1, 0.46, 0.08, 1, 0.46, 0.08,
    -0.58, -0.52, 0.05, 0.58, -0.52, 0.05,
    -0.62, 0.32, 0.38, 0.62, 0.32, 0.38,
    -0.3, -0.38, 0.36, 0.3, -0.38, 0.36,
    0, 0.12, 0.56, 0, -0.2, 0.5,
  ], 3));
  geometry.setIndex([
    0, 4, 5, 0, 5, 1, 2, 3, 7, 2, 7, 6,
    0, 2, 6, 0, 6, 4, 1, 5, 7, 1, 7, 3,
    4, 6, 10, 4, 10, 8, 5, 9, 11, 5, 11, 7,
    8, 10, 13, 8, 13, 12, 9, 12, 13, 9, 13, 11,
    4, 8, 9, 4, 9, 5, 6, 7, 11, 6, 11, 10,
    8, 12, 9, 10, 11, 13, 0, 1, 3, 0, 3, 2,
  ]);
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();
  return geometry;
}

function splitByBlocks(start: number, end: number, blocks: Array<{ start: number; end: number }>): Array<{ start: number; end: number }> {
  const segments: Array<{ start: number; end: number }> = [];
  let cursor = start;
  const ordered = blocks
    .map((block) => ({ start: clamp(block.start, start, end), end: clamp(block.end, start, end) }))
    .filter((block) => block.end > block.start)
    .sort((a, b) => a.start - b.start);
  for (const block of ordered) {
    if (block.start - cursor > 2.5) segments.push({ start: cursor, end: block.start });
    cursor = Math.max(cursor, block.end);
  }
  if (end - cursor > 2.5) segments.push({ start: cursor, end });
  return segments;
}

function createBuildingMaterial(): MeshStandardMaterial {
  const material = new MeshStandardMaterial({
    color: 0xffffff,
    vertexColors: true,
    emissive: 0x101a28,
    emissiveIntensity: 0.48,
    roughness: 0.72,
    metalness: 0.08,
  });

  material.onBeforeCompile = (shader) => {
    shader.vertexShader = shader.vertexShader
      .replace(
        "#include <common>",
        [
          "#include <common>",
          "varying vec3 vCityLocalPosition;",
          "varying vec3 vCityObjectNormal;",
        ].join("\n"),
      )
      .replace(
        "#include <begin_vertex>",
        [
          "#include <begin_vertex>",
          "vCityObjectNormal = normal;",
          "#ifdef USE_INSTANCING",
          "  vCityLocalPosition = (instanceMatrix * vec4(position, 1.0)).xyz;",
          "#else",
          "  vCityLocalPosition = position;",
          "#endif",
        ].join("\n"),
      )
      .replace(
        "#include <project_vertex>",
        "#include <project_vertex>",
      );
    shader.fragmentShader = shader.fragmentShader
      .replace(
        "#include <common>",
        [
          "#include <common>",
          "varying vec3 vCityLocalPosition;",
          "varying vec3 vCityObjectNormal;",
        ].join("\n"),
      )
      .replace(
        "#include <opaque_fragment>",
        [
          "vec3 cityNormal = normalize(vCityObjectNormal);",
          "float cityFacade = 1.0 - step(0.45, abs(cityNormal.y));",
          "float cityAxis = abs(cityNormal.x) > abs(cityNormal.z) ? vCityLocalPosition.z : vCityLocalPosition.x;",
          "vec2 cityGrid = vec2(cityAxis / 4.8, vCityLocalPosition.y / 4.8);",
          "vec2 cityCell = fract(cityGrid);",
          "float cityWindow = step(0.2, cityCell.x) * step(cityCell.x, 0.58)",
          "  * step(0.2, cityCell.y) * step(cityCell.y, 0.58) * cityFacade;",
          "vec2 cityBlock = floor(cityGrid);",
          "float cityNoise = fract(sin(dot(cityBlock, vec2(12.9898, 78.233))) * 43758.5453);",
          "vec3 cityLight = mix(vec3(0.2, 0.72, 0.9), vec3(1.0, 0.72, 0.3), step(0.78, cityNoise));",
          "outgoingLight += cityLight * cityWindow * step(0.38, cityNoise) * 0.34;",
          "#include <opaque_fragment>",
        ].join("\n"),
      );
  };
  material.customProgramCacheKey = () => "gitswamp-city-windows-v1";
  return material;
}

function disposeMaterial(material: Material | Material[]) {
  const materials = Array.isArray(material) ? material : [material];
  for (const item of materials) {
    const values = Object.values(item) as unknown[];
    for (const value of values) {
      if (value instanceof Texture) value.dispose();
    }
    item.dispose();
  }
}

export class RepositoryCityThreeRenderer {
  private readonly renderer: WebGLRenderer;
  private readonly threeScene = new Scene();
  private readonly camera = new PerspectiveCamera(48, 1, 0.1, 10000);
  private readonly raycaster = new Raycaster();
  private readonly pointer = new Vector2();
  private readonly target = new Vector3();
  private readonly walkPosition = new Vector3();
  private readonly buildingByPath = new Map<string, BuildingVisual>();
  private readonly buildingByIndex: CityBuilding[] = [];
  private readonly districtByIndex: CityDistrict[] = [];
  private readonly disposableGeometries = new Set<BufferGeometry>();
  private readonly disposableMaterials = new Set<Material>();
  private roadMesh: InstancedMesh | null = null;
  private buildingMesh: InstancedMesh | null = null;
  private districtMesh: InstancedMesh | null = null;
  private radarMesh: InstancedMesh | null = null;
  private waveMesh: InstancedMesh | null = null;
  private oceanWaveTime: { value: number } | null = null;
  private waveAnimationFrame: number | null = null;
  private lastWaveAnimationAt = 0;
  private readonly waveInstances: WaveInstance[] = [];
  private readonly waveMatrix = new Matrix4();
  private readonly wavePosition = new Vector3();
  private readonly waveScale = new Vector3();
  private readonly waveQuaternion = new Quaternion();
  private readonly waveAxis = new Vector3(0, 1, 0);
  private readonly boatById = new Map<string, BoatVisual>();
  private readonly boatPickTargets: Mesh[] = [];
  private readonly boatByMesh = new Map<Mesh, BoatVisual>();
  private activeBoat: BoatVisual | null = null;
  private boatCannonSide: -1 | 1 = 1;
  private boatCameraYawOffset = Math.PI;
  private boatCameraPitch = 0.34;
  private boatAiming = false;
  private boatAimPitch = 0.22;
  private cannonBallMesh: InstancedMesh | null = null;
  private cannonAnimationFrame: number | null = null;
  private lastCannonAnimationAt = 0;
  private readonly cannonBalls: CannonBall[] = [];
  private aimReticle: Group | null = null;
  private readonly cannonMatrix = new Matrix4();
  private readonly cannonPosition = new Vector3();
  private readonly cannonScale = new Vector3();
  private readonly cannonQuaternion = new Quaternion();
  private debrisMesh: InstancedMesh | null = null;
  private readonly debrisBlocks: DebrisBlock[] = [];
  private readonly collapsedBuildingPaths = new Set<string>();
  private readonly debrisMatrix = new Matrix4();
  private readonly debrisPosition = new Vector3();
  private readonly debrisScale = new Vector3();
  private readonly debrisQuaternion = new Quaternion();
  private readonly debrisEuler = new Euler();
  private impactMesh: InstancedMesh | null = null;
  private readonly impactParticles: ImpactParticle[] = [];
  private readonly impactMatrix = new Matrix4();
  private readonly impactPosition = new Vector3();
  private readonly impactScale = new Vector3();
  private readonly boatLookTarget = new Vector3();
  private readonly boatMove = new Vector3();
  private sceneData: CityScene | null = null;
  private hoveredPath = "";
  private heatVisible = true;
  private width = 1;
  private height = 1;
  private yaw = -0.72;
  private pitch = 1.05;
  private distance = 600;
  private mode: CityCameraMode = "bird";
  private walkYaw = -0.72;
  private walkPitch = -0.08;
  private walkSpeed = 0.65;
  private walkPhase = 0;
  private disposed = false;

  constructor(private readonly canvas: HTMLCanvasElement) {
    this.renderer = new WebGLRenderer({
      canvas,
      antialias: false,
      alpha: false,
      depth: true,
      stencil: false,
      powerPreference: "low-power",
      preserveDrawingBuffer: false,
    });
    this.renderer.setPixelRatio(Math.min(MAX_PIXEL_RATIO, globalThis.devicePixelRatio || 1));
    this.renderer.setClearColor(CLEAR_COLOR, 1);
    this.renderer.outputColorSpace = SRGBColorSpace;
    this.renderer.toneMapping = ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.18;
    this.threeScene.background = new Color(CLEAR_COLOR);
    this.threeScene.fog = new FogExp2(CLEAR_COLOR, 0.00048);

    const ambient = new AmbientLight(0xcfe8ff, 1.15);
    const hemisphere = new HemisphereLight(0xd8f2ff, 0x17243a, 2.4);
    const keyLight = new DirectionalLight(0xffffff, 3.2);
    keyLight.position.set(-180, 300, 120);
    const rimLight = new DirectionalLight(0x5eead4, 1.05);
    rimLight.position.set(220, 130, -180);
    this.threeScene.add(ambient, hemisphere, keyLight, rimLight);
  }

  setScene(scene: CityScene) {
    this.clearDynamicScene();
    this.sceneData = scene;
    this.camera.far = Math.max(4000, Math.max(scene.width, scene.depth) * 8);
    this.camera.updateProjectionMatrix();
    this.addOcean(scene);
    this.addGround(scene);
    this.addCoastalDetails(scene);
    this.addHarbors(scene);
    this.addCannonSystem();
    this.addDistricts(scene);
    this.addParks(scene);
    this.addParkLakes(scene);
    this.addLakeEdgeDetails(scene);
    this.addGrassDetails(scene);
    this.addMeadowDetails(scene);
    this.addShrubs(scene);
    this.addForestFloorDetails(scene);
    this.addParkEntities(scene);
    this.addRoads(scene);
    this.addBuildings(scene);
    this.addTrees(scene);
    this.addRoofDetails(scene);
    this.addRoofMarkers(scene);
    this.fit("bird");
  }

  private addOcean(scene: CityScene) {
    const margin = Math.max(720, Math.max(scene.width, scene.depth) * 0.92);
    const oceanWidth = scene.width + margin * 2;
    const oceanDepth = scene.depth + margin * 2;
    const geometry = new PlaneGeometry(oceanWidth, oceanDepth, 8, 8);
    const material = new MeshStandardMaterial({
      color: 0x0b425f,
      emissive: 0x06243a,
      emissiveIntensity: 0.42,
      roughness: 0.82,
      metalness: 0.02,
    });
    const oceanWaveTime = { value: 0 };
    material.onBeforeCompile = (shader) => {
      shader.uniforms.cityOceanTime = oceanWaveTime;
      shader.vertexShader = shader.vertexShader
        .replace("#include <common>", "#include <common>\nuniform float cityOceanTime;")
        .replace(
          "#include <begin_vertex>",
          [
            "vec3 transformed = vec3(position);",
            "float cityOceanA = sin(position.x * 0.018 + cityOceanTime * 0.72);",
            "float cityOceanB = cos(position.y * 0.014 - cityOceanTime * 0.54);",
            "float cityOceanC = sin((position.x + position.y) * 0.009 + cityOceanTime * 0.31);",
            "transformed.z += (cityOceanA * 0.075 + cityOceanB * 0.055 + cityOceanC * 0.03);",
          ].join("\n"),
        );
    };
    material.customProgramCacheKey = () => "gitswamp-city-ocean-v2";
    this.oceanWaveTime = oceanWaveTime;
    const ocean = new Mesh(geometry, material);
    ocean.rotation.x = -Math.PI / 2;
    ocean.position.y = -1.9;
    this.threeScene.add(ocean);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);

    const waveGeometry = new BoxGeometry(1, 1, 1);
    const waveMaterial = new MeshStandardMaterial({
      color: 0x8bd0e9,
      emissive: 0x1e5f88,
      emissiveIntensity: 0.28,
      roughness: 0.35,
      metalness: 0,
      transparent: true,
      opacity: 0.26,
    });
    const waves = new InstancedMesh(waveGeometry, waveMaterial, MAX_WAVE_LINES);
    const matrix = new Matrix4();
    const position = new Vector3();
    const scale = new Vector3();
    const quaternion = new Quaternion();
    let used = 0;
    this.waveInstances.length = 0;
    for (let index = 0; index < MAX_WAVE_LINES; index += 1) {
      const seed = hashText(`city-wave:${index}:${scene.width}:${scene.depth}`);
      const side = index % 4;
      const row = Math.floor(index / 4) % 12;
      const progress = ((seed >>> 6) % 1000) / 1000;
      const offset = SHORE_BELT_WIDTH * 2 + 10 + row * 17 + ((seed >>> 16) % 18);
      const x = side < 2
        ? -scene.width / 2 + progress * scene.width
        : (side === 2 ? -scene.width / 2 - offset : scene.width / 2 + offset);
      const z = side >= 2
        ? -scene.depth / 2 + progress * scene.depth
        : (side === 0 ? -scene.depth / 2 - offset : scene.depth / 2 + offset);
      const coastAngle = side < 2 ? 0 : Math.PI / 2;
      const rotation = coastAngle + (((seed >>> 24) % 24) - 12) / 100;
      const length = 16 + ((seed >>> 12) % 48);
      const depth = 0.22 + ((seed >>> 20) % 16) / 100;
      quaternion.setFromAxisAngle(this.waveAxis, rotation);
      position.set(x, -1.55, z);
      scale.set(length, 0.035, depth);
      matrix.compose(position, quaternion, scale);
      waves.setMatrixAt(used, matrix);
      waves.setColorAt(used, new Color(seed % 5 === 0 ? "#d2f3fa" : seed % 3 === 0 ? "#82cde6" : "#61b7d6"));
      this.waveInstances.push({
        x,
        z,
        rotation,
        length,
        depth,
        phase: ((seed >>> 5) % 628) / 100,
        travel: 6 + ((seed >>> 9) % 12),
        speed: 0.42 + ((seed >>> 18) % 55) / 100,
        driftX: side >= 2 ? (side === 2 ? -1 : 1) : 0,
        driftZ: side < 2 ? (side === 0 ? -1 : 1) : 0,
        amplitude: 0.026 + ((seed >>> 21) % 30) / 1000,
      });
      used += 1;
    }
    waves.count = used;
    waves.instanceMatrix.setUsage(DynamicDrawUsage);
    if (waves.instanceColor) waves.instanceColor.needsUpdate = true;
    this.waveMesh = waves;
    this.threeScene.add(waves);
    this.disposableGeometries.add(waveGeometry);
    this.disposableMaterials.add(waveMaterial);
    this.startWaveAnimation();
  }

  private addGround(scene: CityScene) {
    const margin = SHORE_BELT_WIDTH * 2;
    const geometry = new PlaneGeometry(scene.width + margin, scene.depth + margin);
    const material = new MeshStandardMaterial({
      color: GROUND_COLOR,
      emissive: 0x07150f,
      emissiveIntensity: 0.18,
      roughness: 0.96,
      metalness: 0,
    });
    const ground = new Mesh(geometry, material);
    ground.rotation.x = -Math.PI / 2;
    ground.position.y = -1.5;
    this.threeScene.add(ground);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);

    const shoreGeometry = new BoxGeometry(1, 1, 1);
    const shoreMaterial = new MeshStandardMaterial({
      color: 0x3e5a41,
      roughness: 0.9,
      metalness: 0,
    });
    const shore = new InstancedMesh(shoreGeometry, shoreMaterial, 4);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    const strips = [
      { x: 0, z: -scene.depth / 2 - SHORE_BELT_WIDTH / 2, width: scene.width + margin, depth: SHORE_BELT_WIDTH },
      { x: 0, z: scene.depth / 2 + SHORE_BELT_WIDTH / 2, width: scene.width + margin, depth: SHORE_BELT_WIDTH },
      { x: -scene.width / 2 - SHORE_BELT_WIDTH / 2, z: 0, width: SHORE_BELT_WIDTH, depth: scene.depth + margin },
      { x: scene.width / 2 + SHORE_BELT_WIDTH / 2, z: 0, width: SHORE_BELT_WIDTH, depth: scene.depth + margin },
    ];
    strips.forEach((strip, index) => {
      matrix.compose(
        new Vector3(strip.x, -1.22, strip.z),
        quaternion,
        new Vector3(strip.width, 0.22, strip.depth),
      );
      shore.setMatrixAt(index, matrix);
    });
    shore.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(shore);
    this.disposableGeometries.add(shoreGeometry);
    this.disposableMaterials.add(shoreMaterial);
  }

  private trackMesh(mesh: Mesh) {
    this.disposableGeometries.add(mesh.geometry);
    const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    materials.forEach((material) => this.disposableMaterials.add(material));
  }

  private addCoastalDetails(scene: CityScene) {
    const details: Array<{ x: number; z: number; scale: number; kind: "tree" | "shrub" | "grass"; color: Color }> = [];
    const halfWidth = scene.width / 2;
    const halfDepth = scene.depth / 2;
    const perimeter = (scene.width + scene.depth) * 2;
    const count = clamp(Math.floor(perimeter / 3.4), 160, MAX_COASTAL_DETAILS);
    for (let index = 0; index < count; index += 1) {
      const seed = hashText(`coast:${scene.width}:${scene.depth}:${index}`);
      const side = seed % 4;
      const progress = ((seed >>> 5) % 1000) / 1000;
      const belt = 2.4 + ((seed >>> 15) % 1000) / 1000 * (SHORE_BELT_WIDTH - 5);
      const x = side < 2
        ? -halfWidth + progress * scene.width
        : (side === 2 ? -halfWidth - belt : halfWidth + belt);
      const z = side >= 2
        ? -halfDepth + progress * scene.depth
        : (side === 0 ? -halfDepth - belt : halfDepth + belt);
      const innerX = x + scene.width / 2;
      const innerZ = z + scene.depth / 2;
      if (pointNearRoad(innerX, innerZ, scene.roads, SIDEWALK_WIDTH + 0.6)) continue;
      const roll = (seed >>> 22) % 10;
      details.push({
        x,
        z,
        scale: 0.48 + ((seed >>> 24) % 60) / 100,
        kind: roll < 3 ? "tree" : roll < 7 ? "shrub" : "grass",
        color: new Color(roll < 3 ? "#3b9a66" : roll < 7 ? "#4f9f58" : "#7fca68"),
      });
    }
    if (details.length === 0) return;

    const trunkGeometry = new CylinderGeometry(0.28, 0.34, 2.4, 5);
    const canopyGeometry = new ConeGeometry(1.55, 3.4, 6);
    const shrubGeometry = new SphereGeometry(0.78, 7, 5);
    const grassGeometry = new ConeGeometry(0.22, 0.78, 5);
    const trunkMaterial = new MeshStandardMaterial({ color: 0x6b4936, roughness: 1 });
    const canopyMaterial = new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.94 });
    const shrubMaterial = new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.98 });
    const grassMaterial = new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.98 });
    const trunks = new InstancedMesh(trunkGeometry, trunkMaterial, details.length);
    const canopies = new InstancedMesh(canopyGeometry, canopyMaterial, details.length);
    const shrubs = new InstancedMesh(shrubGeometry, shrubMaterial, details.length);
    const grasses = new InstancedMesh(grassGeometry, grassMaterial, details.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    let treeCount = 0;
    let shrubCount = 0;
    let grassCount = 0;
    details.forEach((detail, index) => {
      if (detail.kind === "tree") {
        matrix.compose(
          new Vector3(detail.x, -0.02, detail.z),
          quaternion,
          new Vector3(detail.scale, detail.scale, detail.scale),
        );
        trunks.setMatrixAt(treeCount, matrix);
        matrix.compose(
          new Vector3(detail.x, 2.25 * detail.scale, detail.z),
          quaternion,
          new Vector3(detail.scale, detail.scale, detail.scale),
        );
        canopies.setMatrixAt(treeCount, matrix);
        canopies.setColorAt(treeCount, detail.color);
        treeCount += 1;
      } else if (detail.kind === "shrub") {
        matrix.compose(
          new Vector3(detail.x, -0.92, detail.z),
          quaternion,
          new Vector3(detail.scale * 1.3, detail.scale * 0.46, detail.scale),
        );
        shrubs.setMatrixAt(shrubCount, matrix);
        shrubs.setColorAt(shrubCount, detail.color);
        shrubCount += 1;
      } else {
        quaternion.setFromAxisAngle(this.waveAxis, (hashText(`coast-grass:${index}`) % 628) / 100);
        matrix.compose(
          new Vector3(detail.x, -0.88, detail.z),
          quaternion,
          new Vector3(detail.scale, detail.scale, detail.scale),
        );
        grasses.setMatrixAt(grassCount, matrix);
        grasses.setColorAt(grassCount, detail.color);
        grassCount += 1;
      }
    });
    trunks.count = treeCount;
    canopies.count = treeCount;
    shrubs.count = shrubCount;
    grasses.count = grassCount;
    trunks.instanceMatrix.setUsage(StaticDrawUsage);
    canopies.instanceMatrix.setUsage(StaticDrawUsage);
    shrubs.instanceMatrix.setUsage(StaticDrawUsage);
    grasses.instanceMatrix.setUsage(StaticDrawUsage);
    if (canopies.instanceColor) canopies.instanceColor.needsUpdate = true;
    if (shrubs.instanceColor) shrubs.instanceColor.needsUpdate = true;
    if (grasses.instanceColor) grasses.instanceColor.needsUpdate = true;
    this.threeScene.add(trunks, canopies, shrubs, grasses);
    this.disposableGeometries.add(trunkGeometry);
    this.disposableGeometries.add(canopyGeometry);
    this.disposableGeometries.add(shrubGeometry);
    this.disposableGeometries.add(grassGeometry);
    this.disposableMaterials.add(trunkMaterial);
    this.disposableMaterials.add(canopyMaterial);
    this.disposableMaterials.add(shrubMaterial);
    this.disposableMaterials.add(grassMaterial);
  }

  private harborLayouts(scene: CityScene): DockLayout[] {
    const halfWidth = scene.width / 2;
    const halfDepth = scene.depth / 2;
    const length = clamp(Math.max(118, Math.min(scene.depth * 0.34, 168)), 118, 168);
    const width = 15;
    const xOffset = Math.min(Math.max(70, scene.width * 0.28), halfWidth * 0.72);
    return [
      {
        id: "north-harbor",
        name: "North Harbor",
        x: -xOffset,
        z: -halfDepth - length / 2 + SHORE_BELT_WIDTH * 0.42,
        length,
        width,
        outward: -1,
      },
      {
        id: "south-harbor",
        name: "South Harbor",
        x: xOffset,
        z: halfDepth + length / 2 - SHORE_BELT_WIDTH * 0.42,
        length,
        width,
        outward: 1,
      },
    ];
  }

  private addHarbors(scene: CityScene) {
    const docks = this.harborLayouts(scene);
    this.addDocks(docks);
    docks.forEach((dock, index) => {
      const boatZ = dock.z + dock.outward * (dock.length / 2 + 22);
      const yaw = dock.outward === 1 ? 0 : Math.PI;
      if (index === 0) {
        this.addSpeedBoat({
          id: "harbor-runner",
          name: "Harbor Runner",
          kind: "speed boat",
          x: dock.x,
          z: boatZ,
          yaw,
        });
      } else {
        this.addTugBoat({
          id: "code-tug",
          name: "Code Tug",
          kind: "tug boat",
          x: dock.x,
          z: boatZ,
          yaw,
        });
      }
    });
  }

  private addDocks(docks: DockLayout[]) {
    const planks: Array<{ x: number; z: number; width: number; depth: number; color: Color }> = [];
    const beams: Array<{ x: number; z: number; width: number; depth: number }> = [];
    const posts: Array<{ x: number; z: number; height: number }> = [];
    const lamps: Array<{ x: number; z: number }> = [];

    for (const dock of docks) {
      const plankCount = Math.max(18, Math.floor(dock.length / 3.2));
      for (let index = 0; index < plankCount; index += 1) {
        const progress = (index + 0.5) / plankCount - 0.5;
        const seed = hashText(`${dock.id}:plank:${index}`);
        planks.push({
          x: dock.x + (((seed >>> 8) % 100) / 100 - 0.5) * 0.34,
          z: dock.z + progress * dock.length * dock.outward,
          width: dock.width + (((seed >>> 16) % 100) / 100 - 0.5) * 0.45,
          depth: 2.3,
          color: new Color(seed % 2 === 0 ? "#7a5434" : "#8a6240"),
        });
      }

      beams.push(
        { x: dock.x - dock.width / 2 + 1.1, z: dock.z, width: 0.72, depth: dock.length + 6 },
        { x: dock.x + dock.width / 2 - 1.1, z: dock.z, width: 0.72, depth: dock.length + 6 },
        { x: dock.x, z: dock.z - dock.outward * (dock.length / 2 - 3), width: dock.width + 2.2, depth: 0.78 },
        { x: dock.x, z: dock.z + dock.outward * (dock.length / 2 - 3), width: dock.width + 2.2, depth: 0.78 },
      );

      const postCount = Math.max(6, Math.floor(dock.length / 13));
      for (let index = 0; index < postCount; index += 1) {
        const progress = index / Math.max(1, postCount - 1) - 0.5;
        const z = dock.z + progress * dock.length * dock.outward;
        posts.push(
          { x: dock.x - dock.width / 2 - 0.35, z, height: index % 2 === 0 ? 2.7 : 2.25 },
          { x: dock.x + dock.width / 2 + 0.35, z, height: index % 2 === 0 ? 2.7 : 2.25 },
        );
        if (index % 2 === 0) {
          lamps.push({ x: dock.x - dock.width / 2 - 1.3, z });
          lamps.push({ x: dock.x + dock.width / 2 + 1.3, z });
        }
      }
    }

    const plankGeometry = new BoxGeometry(1, 1, 1);
    const plankMaterial = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      roughness: 0.9,
      metalness: 0.02,
    });
    const plankMesh = new InstancedMesh(plankGeometry, plankMaterial, planks.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    planks.forEach((plank, index) => {
      matrix.compose(
        new Vector3(plank.x, -0.72, plank.z),
        quaternion,
        new Vector3(plank.width, 0.28, plank.depth),
      );
      plankMesh.setMatrixAt(index, matrix);
      plankMesh.setColorAt(index, plank.color);
    });
    plankMesh.instanceMatrix.setUsage(StaticDrawUsage);
    if (plankMesh.instanceColor) plankMesh.instanceColor.needsUpdate = true;
    this.threeScene.add(plankMesh);
    this.disposableGeometries.add(plankGeometry);
    this.disposableMaterials.add(plankMaterial);

    const beamGeometry = new BoxGeometry(1, 1, 1);
    const beamMaterial = new MeshStandardMaterial({ color: 0x4b3222, roughness: 0.94, metalness: 0.01 });
    const beamMesh = new InstancedMesh(beamGeometry, beamMaterial, beams.length);
    beams.forEach((beam, index) => {
      matrix.compose(
        new Vector3(beam.x, -0.96, beam.z),
        quaternion,
        new Vector3(beam.width, 0.55, beam.depth),
      );
      beamMesh.setMatrixAt(index, matrix);
    });
    beamMesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(beamMesh);
    this.disposableGeometries.add(beamGeometry);
    this.disposableMaterials.add(beamMaterial);

    const postGeometry = new CylinderGeometry(0.28, 0.34, 1, 6);
    const postMaterial = new MeshStandardMaterial({ color: 0x5a3b27, roughness: 0.96, metalness: 0.02 });
    const postMesh = new InstancedMesh(postGeometry, postMaterial, posts.length);
    posts.forEach((post, index) => {
      matrix.compose(
        new Vector3(post.x, -1.08 + post.height / 2, post.z),
        quaternion,
        new Vector3(1, post.height, 1),
      );
      postMesh.setMatrixAt(index, matrix);
    });
    postMesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(postMesh);
    this.disposableGeometries.add(postGeometry);
    this.disposableMaterials.add(postMaterial);

    const lampGeometry = new SphereGeometry(0.42, 8, 6);
    const lampMaterial = new MeshStandardMaterial({
      color: 0xf9e7a4,
      emissive: 0xfbbf24,
      emissiveIntensity: 0.72,
      roughness: 0.42,
    });
    const lampMesh = new InstancedMesh(lampGeometry, lampMaterial, lamps.length);
    lamps.forEach((lamp, index) => {
      matrix.compose(
        new Vector3(lamp.x, 1.72, lamp.z),
        quaternion,
        new Vector3(1, 1, 1),
      );
      lampMesh.setMatrixAt(index, matrix);
    });
    lampMesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(lampMesh);
    this.disposableGeometries.add(lampGeometry);
    this.disposableMaterials.add(lampMaterial);
  }

  private addBoatPart(
    boat: BoatVisual,
    geometry: BufferGeometry,
    material: Material,
    position: Vector3,
    scale: Vector3,
    rotation?: Vector3,
    pickable = true,
  ): Mesh {
    const mesh = new Mesh(geometry, material);
    mesh.position.copy(position);
    mesh.scale.copy(scale);
    if (rotation) mesh.rotation.set(rotation.x, rotation.y, rotation.z);
    boat.group.add(mesh);
    this.trackMesh(mesh);
    if (pickable) {
      this.boatPickTargets.push(mesh);
      this.boatByMesh.set(mesh, boat);
    }
    return mesh;
  }

  private registerBoat(boat: BoatVisual, x: number, z: number, yaw: number) {
    boat.group.position.set(x, -1.02, z);
    boat.group.rotation.y = yaw;
    boat.yaw = yaw;
    boat.bobPhase = (hashText(boat.id) % 628) / 100;
    this.boatById.set(boat.id, boat);
    this.threeScene.add(boat.group);
  }

  private addSpeedBoat(options: CityBoatInfo & { x: number; z: number; yaw: number }) {
    const boat: BoatVisual = {
      id: options.id,
      name: options.name,
      kind: options.kind,
      group: new Group(),
      yaw: options.yaw,
      bobPhase: 0,
    };
    const box = new BoxGeometry(1, 1, 1);
    const hullGeometry = createBoatHullGeometry();
    const cylinder = new CylinderGeometry(1, 1, 1, 10);
    const sphere = new SphereGeometry(1, 8, 6);
    const ring = new TorusGeometry(1, 0.18, 5, 12);
    const hull = new MeshStandardMaterial({ color: 0x1f6f8b, roughness: 0.58, metalness: 0.08 });
    const trim = new MeshStandardMaterial({ color: 0x8ef2ff, emissive: 0x0a5268, emissiveIntensity: 0.22, roughness: 0.48 });
    const cabin = new MeshStandardMaterial({ color: 0xe2e8f0, roughness: 0.46, metalness: 0.04 });
    const glass = new MeshStandardMaterial({ color: 0x67e8f9, emissive: 0x155e75, emissiveIntensity: 0.36, roughness: 0.18, metalness: 0.16 });
    const dark = new MeshStandardMaterial({ color: 0x18222d, roughness: 0.68, metalness: 0.12 });
    const brass = new MeshStandardMaterial({ color: 0xc7923a, emissive: 0x3a2608, emissiveIntensity: 0.12, roughness: 0.48, metalness: 0.22 });
    const seat = new MeshStandardMaterial({ color: 0x0f172a, roughness: 0.7, metalness: 0.04 });
    const portLight = new MeshStandardMaterial({ color: 0xf87171, emissive: 0xdc2626, emissiveIntensity: 0.75, roughness: 0.3 });
    const starboardLight = new MeshStandardMaterial({ color: 0x4ade80, emissive: 0x16a34a, emissiveIntensity: 0.75, roughness: 0.3 });
    const wake = new MeshStandardMaterial({ color: 0xa5f3fc, transparent: true, opacity: 0.28, roughness: 0.34 });

    this.addBoatPart(boat, hullGeometry, hull, new Vector3(0, 0.28, -0.35), new Vector3(3.05, 1.62, 15.2));
    this.addBoatPart(boat, box, dark, new Vector3(0, -0.3, -1.1), new Vector3(3.2, 0.26, 10.6), undefined, false);
    this.addBoatPart(boat, box, trim, new Vector3(0, 0.9, 1.3), new Vector3(5.75, 0.24, 8.2));
    this.addBoatPart(boat, box, cabin, new Vector3(0, 1.72, -1.8), new Vector3(3.45, 2.05, 4.4));
    this.addBoatPart(boat, box, glass, new Vector3(0, 2.55, 0.58), new Vector3(3.65, 0.76, 0.24));
    this.addBoatPart(boat, box, glass, new Vector3(-1.84, 2.12, -1.5), new Vector3(0.18, 0.86, 2.3));
    this.addBoatPart(boat, box, glass, new Vector3(1.84, 2.12, -1.5), new Vector3(0.18, 0.86, 2.3));
    this.addBoatPart(boat, box, dark, new Vector3(-3.05, 1.12, 1.4), new Vector3(0.22, 0.24, 7.2));
    this.addBoatPart(boat, box, dark, new Vector3(3.05, 1.12, 1.4), new Vector3(0.22, 0.24, 7.2));
    this.addBoatPart(boat, cylinder, dark, new Vector3(-1.25, 0.36, -8.1), new Vector3(0.48, 1.2, 0.48), new Vector3(Math.PI / 2, 0, 0));
    this.addBoatPart(boat, cylinder, dark, new Vector3(1.25, 0.36, -8.1), new Vector3(0.48, 1.2, 0.48), new Vector3(Math.PI / 2, 0, 0));
    for (let index = 0; index < 12; index += 1) {
      const z = -5.4 + index * 0.9;
      this.addBoatPart(boat, box, brass, new Vector3(-3.12, 1.58, z), new Vector3(0.18, 0.9, 0.18), undefined, false);
      this.addBoatPart(boat, box, brass, new Vector3(3.12, 1.58, z), new Vector3(0.18, 0.9, 0.18), undefined, false);
    }
    this.addBoatPart(boat, box, brass, new Vector3(-3.12, 2.05, 0), new Vector3(0.2, 0.16, 10.8), undefined, false);
    this.addBoatPart(boat, box, brass, new Vector3(3.12, 2.05, 0), new Vector3(0.2, 0.16, 10.8), undefined, false);
    for (let row = 0; row < 2; row += 1) {
      for (let column = 0; column < 2; column += 1) {
        this.addBoatPart(
          boat,
          box,
          seat,
          new Vector3(-0.9 + column * 1.8, 1.38, -4.45 + row * 1.25),
          new Vector3(0.82, 0.55, 0.76),
          undefined,
          false,
        );
        this.addBoatPart(
          boat,
          box,
          seat,
          new Vector3(-0.9 + column * 1.8, 1.76, -4.18 + row * 1.25),
          new Vector3(0.82, 0.68, 0.14),
          undefined,
          false,
        );
      }
    }
    for (let index = 0; index < 7; index += 1) {
      this.addBoatPart(boat, box, trim, new Vector3(-2.92, 0.95, -4.8 + index * 1.35), new Vector3(0.18, 0.24, 0.58), undefined, false);
      this.addBoatPart(boat, box, trim, new Vector3(2.92, 0.95, -4.8 + index * 1.35), new Vector3(0.18, 0.24, 0.58), undefined, false);
    }
    this.addBoatPart(boat, box, dark, new Vector3(3.42, 1.74, 0.95), new Vector3(0.9, 0.54, 0.76), undefined, false);
    this.addBoatPart(boat, cylinder, dark, new Vector3(3.98, 1.82, 0.95), new Vector3(0.21, 1.42, 0.21), new Vector3(0, 0, Math.PI / 2), false);
    this.addBoatPart(boat, box, dark, new Vector3(-3.42, 1.74, 0.95), new Vector3(0.9, 0.54, 0.76), undefined, false);
    this.addBoatPart(boat, cylinder, dark, new Vector3(-3.98, 1.82, 0.95), new Vector3(0.21, 1.42, 0.21), new Vector3(0, 0, Math.PI / 2), false);
    for (let index = 0; index < 8; index += 1) {
      const z = -5.8 + index * 1.45;
      this.addBoatPart(boat, box, brass, new Vector3(-0.28, 1.16, z), new Vector3(0.26, 0.28, 0.66), undefined, false);
      this.addBoatPart(boat, box, dark, new Vector3(0.38, 1.18, z + 0.18), new Vector3(0.3, 0.18, 0.42), undefined, false);
    }
    this.addBoatPart(boat, box, brass, new Vector3(0, 3.35, -3.25), new Vector3(0.18, 2.2, 0.18), undefined, false);
    this.addBoatPart(boat, box, brass, new Vector3(0, 4.48, -3.25), new Vector3(1.4, 0.12, 0.12), undefined, false);
    this.addBoatPart(boat, box, glass, new Vector3(-0.78, 4.48, -3.25), new Vector3(0.22, 0.22, 0.22), undefined, false);
    this.addBoatPart(boat, box, glass, new Vector3(0.78, 4.48, -3.25), new Vector3(0.22, 0.22, 0.22), undefined, false);
    this.addBoatPart(boat, box, dark, new Vector3(0, 1.16, 4.55), new Vector3(2.2, 0.12, 1.8), undefined, false);
    this.addBoatPart(boat, cylinder, brass, new Vector3(0, 1.45, 4.55), new Vector3(0.32, 0.18, 0.32), new Vector3(Math.PI / 2, 0, 0), false);
    this.addBoatPart(boat, sphere, portLight, new Vector3(-3.08, 2.28, 1.65), new Vector3(0.24, 0.24, 0.24), undefined, false);
    this.addBoatPart(boat, sphere, starboardLight, new Vector3(3.08, 2.28, 1.65), new Vector3(0.24, 0.24, 0.24), undefined, false);
    this.addBoatPart(boat, ring, brass, new Vector3(-3.18, 1.52, -2.25), new Vector3(0.62, 0.62, 0.62), new Vector3(0, Math.PI / 2, 0), false);
    this.addBoatPart(boat, ring, brass, new Vector3(3.18, 1.52, -2.25), new Vector3(0.62, 0.62, 0.62), new Vector3(0, Math.PI / 2, 0), false);
    this.addBoatPart(boat, box, wake, new Vector3(-1.8, -0.18, -10.6), new Vector3(0.28, 0.04, 8.5), new Vector3(0, -0.14, 0), false);
    this.addBoatPart(boat, box, wake, new Vector3(1.8, -0.18, -10.6), new Vector3(0.28, 0.04, 8.5), new Vector3(0, 0.14, 0), false);
    this.registerBoat(boat, options.x, options.z, options.yaw);
  }

  private addTugBoat(options: CityBoatInfo & { x: number; z: number; yaw: number }) {
    const boat: BoatVisual = {
      id: options.id,
      name: options.name,
      kind: options.kind,
      group: new Group(),
      yaw: options.yaw,
      bobPhase: 0,
    };
    const box = new BoxGeometry(1, 1, 1);
    const hullGeometry = createBoatHullGeometry();
    const cylinder = new CylinderGeometry(1, 1, 1, 10);
    const sphere = new SphereGeometry(1, 8, 6);
    const ring = new TorusGeometry(1, 0.19, 5, 12);
    const hull = new MeshStandardMaterial({ color: 0x7f1d1d, roughness: 0.64, metalness: 0.07 });
    const deck = new MeshStandardMaterial({ color: 0x334155, roughness: 0.75, metalness: 0.06 });
    const cabin = new MeshStandardMaterial({ color: 0xf8fafc, roughness: 0.5, metalness: 0.04 });
    const glass = new MeshStandardMaterial({ color: 0x7dd3fc, emissive: 0x0c4a6e, emissiveIntensity: 0.34, roughness: 0.2 });
    const stack = new MeshStandardMaterial({ color: 0x111827, roughness: 0.7, metalness: 0.14 });
    const rope = new MeshStandardMaterial({ color: 0xd6b47a, roughness: 0.92 });
    const rail = new MeshStandardMaterial({ color: 0xd8e3ec, roughness: 0.58, metalness: 0.14 });
    const crate = new MeshStandardMaterial({ color: 0x7c4a2d, roughness: 0.86, metalness: 0.02 });
    const brass = new MeshStandardMaterial({ color: 0xc8943a, emissive: 0x3b2608, emissiveIntensity: 0.12, roughness: 0.48, metalness: 0.22 });
    const portLight = new MeshStandardMaterial({ color: 0xf87171, emissive: 0xdc2626, emissiveIntensity: 0.72, roughness: 0.3 });
    const starboardLight = new MeshStandardMaterial({ color: 0x4ade80, emissive: 0x16a34a, emissiveIntensity: 0.72, roughness: 0.3 });
    const wake = new MeshStandardMaterial({ color: 0xbae6fd, transparent: true, opacity: 0.22, roughness: 0.34 });

    this.addBoatPart(boat, hullGeometry, hull, new Vector3(0, 0.16, 0.15), new Vector3(3.45, 1.9, 13.6));
    this.addBoatPart(boat, box, stack, new Vector3(0, -0.36, -0.35), new Vector3(3.65, 0.3, 9.4), undefined, false);
    this.addBoatPart(boat, box, deck, new Vector3(0, 0.92, 1.3), new Vector3(5.4, 0.34, 8.8));
    this.addBoatPart(boat, box, cabin, new Vector3(0, 2.1, -1.5), new Vector3(3.7, 2.65, 4.15));
    this.addBoatPart(boat, box, cabin, new Vector3(0, 3.7, -1.9), new Vector3(2.7, 1.25, 2.8));
    this.addBoatPart(boat, box, glass, new Vector3(0, 2.75, 0.65), new Vector3(3.92, 0.62, 0.22));
    this.addBoatPart(boat, box, glass, new Vector3(-2.02, 2.32, -1.55), new Vector3(0.2, 0.72, 2.2));
    this.addBoatPart(boat, box, glass, new Vector3(2.02, 2.32, -1.55), new Vector3(0.2, 0.72, 2.2));
    this.addBoatPart(boat, cylinder, stack, new Vector3(0, 4.65, -3.55), new Vector3(0.45, 1.75, 0.45));
    this.addBoatPart(boat, cylinder, rope, new Vector3(-2.45, 1.45, 3.65), new Vector3(0.58, 0.28, 0.58), new Vector3(Math.PI / 2, 0, 0));
    this.addBoatPart(boat, cylinder, rope, new Vector3(2.45, 1.45, 3.65), new Vector3(0.58, 0.28, 0.58), new Vector3(Math.PI / 2, 0, 0));
    this.addBoatPart(boat, sphere, rope, new Vector3(-3.4, 0.7, -2.2), new Vector3(0.42, 0.42, 0.18));
    this.addBoatPart(boat, sphere, rope, new Vector3(3.4, 0.7, -2.2), new Vector3(0.42, 0.42, 0.18));
    for (let index = 0; index < 11; index += 1) {
      const z = -4.7 + index * 0.95;
      this.addBoatPart(boat, box, rail, new Vector3(-3.18, 1.72, z), new Vector3(0.16, 0.92, 0.16), undefined, false);
      this.addBoatPart(boat, box, rail, new Vector3(3.18, 1.72, z), new Vector3(0.16, 0.92, 0.16), undefined, false);
    }
    this.addBoatPart(boat, box, rail, new Vector3(-3.18, 2.22, 0.2), new Vector3(0.18, 0.14, 10.4), undefined, false);
    this.addBoatPart(boat, box, rail, new Vector3(3.18, 2.22, 0.2), new Vector3(0.18, 0.14, 10.4), undefined, false);
    for (let index = 0; index < 6; index += 1) {
      this.addBoatPart(boat, box, glass, new Vector3(-1.35 + index * 0.54, 4.02, -0.45), new Vector3(0.28, 0.34, 0.16), undefined, false);
    }
    for (let index = 0; index < 5; index += 1) {
      this.addBoatPart(boat, box, crate, new Vector3(-1.9 + index * 0.92, 1.36, 3.9), new Vector3(0.62, 0.58, 0.62), undefined, false);
    }
    for (let index = 0; index < 4; index += 1) {
      this.addBoatPart(boat, sphere, stack, new Vector3(-3.55, 0.8, -4.6 + index * 2.1), new Vector3(0.36, 0.36, 0.16), undefined, false);
      this.addBoatPart(boat, sphere, stack, new Vector3(3.55, 0.8, -4.6 + index * 2.1), new Vector3(0.36, 0.36, 0.16), undefined, false);
    }
    this.addBoatPart(boat, box, stack, new Vector3(3.58, 1.86, 1.4), new Vector3(0.98, 0.64, 0.86), undefined, false);
    this.addBoatPart(boat, cylinder, stack, new Vector3(4.22, 1.96, 1.4), new Vector3(0.24, 1.5, 0.24), new Vector3(0, 0, Math.PI / 2), false);
    this.addBoatPart(boat, box, stack, new Vector3(-3.58, 1.86, 1.4), new Vector3(0.98, 0.64, 0.86), undefined, false);
    this.addBoatPart(boat, cylinder, stack, new Vector3(-4.22, 1.96, 1.4), new Vector3(0.24, 1.5, 0.24), new Vector3(0, 0, Math.PI / 2), false);
    for (let index = 0; index < 9; index += 1) {
      const z = -5.1 + index * 1.12;
      this.addBoatPart(boat, box, brass, new Vector3(0.42, 1.2, z), new Vector3(0.28, 0.24, 0.48), undefined, false);
      this.addBoatPart(boat, box, crate, new Vector3(-0.42, 1.22, z + 0.24), new Vector3(0.34, 0.22, 0.44), undefined, false);
    }
    this.addBoatPart(boat, box, brass, new Vector3(0, 5.35, -1.9), new Vector3(0.22, 2.1, 0.22), undefined, false);
    this.addBoatPart(boat, box, brass, new Vector3(0, 6.45, -1.9), new Vector3(1.55, 0.14, 0.14), undefined, false);
    this.addBoatPart(boat, box, glass, new Vector3(-0.88, 6.45, -1.9), new Vector3(0.24, 0.24, 0.24), undefined, false);
    this.addBoatPart(boat, box, glass, new Vector3(0.88, 6.45, -1.9), new Vector3(0.24, 0.24, 0.24), undefined, false);
    this.addBoatPart(boat, box, deck, new Vector3(0, 4.45, -1.9), new Vector3(3.45, 0.2, 3.65), undefined, false);
    this.addBoatPart(boat, cylinder, brass, new Vector3(0, 1.52, 3.65), new Vector3(0.68, 0.36, 0.68), new Vector3(Math.PI / 2, 0, 0), false);
    this.addBoatPart(boat, cylinder, rope, new Vector3(0, 1.72, 3.65), new Vector3(0.32, 1.28, 0.32), new Vector3(0, 0, Math.PI / 2), false);
    this.addBoatPart(boat, ring, rope, new Vector3(-3.3, 2.58, -0.4), new Vector3(0.72, 0.72, 0.72), new Vector3(0, Math.PI / 2, 0), false);
    this.addBoatPart(boat, ring, rope, new Vector3(3.3, 2.58, -0.4), new Vector3(0.72, 0.72, 0.72), new Vector3(0, Math.PI / 2, 0), false);
    this.addBoatPart(boat, sphere, portLight, new Vector3(-1.58, 5.45, -1.7), new Vector3(0.25, 0.25, 0.25), undefined, false);
    this.addBoatPart(boat, sphere, starboardLight, new Vector3(1.58, 5.45, -1.7), new Vector3(0.25, 0.25, 0.25), undefined, false);
    this.addBoatPart(boat, box, wake, new Vector3(0, -0.2, -9.8), new Vector3(4.2, 0.04, 7.8), undefined, false);
    this.registerBoat(boat, options.x, options.z, options.yaw);
  }

  private addCannonSystem() {
    this.cannonBalls.length = 0;
    for (let index = 0; index < MAX_CANNONBALLS; index += 1) {
      this.cannonBalls.push({ active: false, x: 0, y: -100, z: 0, vx: 0, vy: 0, vz: 0, age: 0 });
    }
    const ballGeometry = new SphereGeometry(0.78, 10, 8);
    const ballMaterial = new MeshStandardMaterial({
      color: 0x16181c,
      roughness: 0.52,
      metalness: 0.28,
    });
    const balls = new InstancedMesh(ballGeometry, ballMaterial, MAX_CANNONBALLS);
    for (let index = 0; index < MAX_CANNONBALLS; index += 1) {
      this.cannonMatrix.compose(
        new Vector3(0, -100, 0),
        this.cannonQuaternion,
        new Vector3(0.001, 0.001, 0.001),
      );
      balls.setMatrixAt(index, this.cannonMatrix);
    }
    balls.instanceMatrix.setUsage(DynamicDrawUsage);
    this.cannonBallMesh = balls;
    this.threeScene.add(balls);
    this.disposableGeometries.add(ballGeometry);
    this.disposableMaterials.add(ballMaterial);

    this.debrisBlocks.length = 0;
    for (let index = 0; index < MAX_DEBRIS_BLOCKS; index += 1) {
      this.debrisBlocks.push({
        active: false,
        sleeping: false,
        x: 0,
        y: -120,
        z: 0,
        vx: 0,
        vy: 0,
        vz: 0,
        rx: 0,
        ry: 0,
        rz: 0,
        vrx: 0,
        vry: 0,
        vrz: 0,
        sx: 0.001,
        sy: 0.001,
        sz: 0.001,
        age: 0,
      });
    }
    const debrisGeometry = new BoxGeometry(1, 1, 1);
    const debrisMaterial = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      roughness: 0.72,
      metalness: 0.04,
    });
    const debris = new InstancedMesh(debrisGeometry, debrisMaterial, MAX_DEBRIS_BLOCKS);
    for (let index = 0; index < MAX_DEBRIS_BLOCKS; index += 1) {
      this.debrisMatrix.compose(
        new Vector3(0, -120, 0),
        this.debrisQuaternion,
        new Vector3(0.001, 0.001, 0.001),
      );
      debris.setMatrixAt(index, this.debrisMatrix);
      debris.setColorAt(index, new Color("#1f2937"));
    }
    debris.instanceMatrix.setUsage(DynamicDrawUsage);
    if (debris.instanceColor) debris.instanceColor.needsUpdate = true;
    this.debrisMesh = debris;
    this.threeScene.add(debris);
    this.disposableGeometries.add(debrisGeometry);
    this.disposableMaterials.add(debrisMaterial);

    this.impactParticles.length = 0;
    for (let index = 0; index < MAX_IMPACT_PARTICLES; index += 1) {
      this.impactParticles.push({
        active: false,
        x: 0,
        y: -120,
        z: 0,
        vx: 0,
        vy: 0,
        vz: 0,
        scale: 0.001,
        gravity: 14,
        age: 0,
        ttl: 1,
      });
    }
    const impactGeometry = new SphereGeometry(1, 6, 4);
    const impactMaterial = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      roughness: 0.76,
      transparent: true,
      opacity: 0.84,
    });
    const impacts = new InstancedMesh(impactGeometry, impactMaterial, MAX_IMPACT_PARTICLES);
    for (let index = 0; index < MAX_IMPACT_PARTICLES; index += 1) {
      this.impactMatrix.compose(
        new Vector3(0, -120, 0),
        this.cannonQuaternion,
        new Vector3(0.001, 0.001, 0.001),
      );
      impacts.setMatrixAt(index, this.impactMatrix);
      impacts.setColorAt(index, new Color("#9bdcf0"));
    }
    impacts.instanceMatrix.setUsage(DynamicDrawUsage);
    if (impacts.instanceColor) impacts.instanceColor.needsUpdate = true;
    this.impactMesh = impacts;
    this.threeScene.add(impacts);
    this.disposableGeometries.add(impactGeometry);
    this.disposableMaterials.add(impactMaterial);

    const reticle = new Group();
    const reticleMaterial = new MeshStandardMaterial({
      color: 0x67e8f9,
      emissive: 0x0891b2,
      emissiveIntensity: 0.72,
      roughness: 0.36,
      metalness: 0,
    });
    const barGeometry = new BoxGeometry(1, 1, 1);
    const horizontal = new Mesh(barGeometry, reticleMaterial);
    horizontal.scale.set(4.4, 0.08, 0.08);
    const vertical = new Mesh(barGeometry, reticleMaterial);
    vertical.scale.set(0.08, 2.5, 0.08);
    const center = new Mesh(new SphereGeometry(0.24, 8, 6), reticleMaterial);
    reticle.add(horizontal, vertical, center);
    reticle.visible = false;
    this.aimReticle = reticle;
    this.threeScene.add(reticle);
    this.trackMesh(horizontal);
    this.trackMesh(vertical);
    this.trackMesh(center);
  }

  private addDistricts(scene: CityScene) {
    if (scene.districts.length === 0) return;
    const geometry = new BoxGeometry(1, 1, 1);
    const material = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      roughness: 0.86,
      metalness: 0.04,
    });
    const mesh = new InstancedMesh(geometry, material, scene.districts.length);
    const matrix = new Matrix4();
    const position = new Vector3();
    const scale = new Vector3();
    const quaternion = new Quaternion();
    this.districtByIndex.length = 0;
    scene.districts.forEach((district, index) => {
      position.set(
        district.x + district.width / 2 - scene.width / 2,
        -0.25,
        district.y + district.depth / 2 - scene.depth / 2,
      );
      scale.set(district.width, 1.2, district.depth);
      matrix.compose(position, quaternion, scale);
      mesh.setMatrixAt(index, matrix);
      mesh.setColorAt(
        index,
        district.averageHeat > 0.66
          ? new Color("#33402a")
          : new Color(index % 2 === 0 ? "#173822" : "#1b4026"),
      );
      this.districtByIndex.push(district);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    this.districtMesh = mesh;
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addParks(scene: CityScene) {
    void scene;
  }

  private addParkLakes(scene: CityScene) {
    const lakes = scene.parks
      .map((park) => parkLakeFor(park))
      .filter((lake): lake is ParkLake => lake !== null)
      .slice(0, MAX_PARK_LAKES);
    if (lakes.length === 0) return;
    const geometry = new CylinderGeometry(1, 1, 0.08, 22);
    const material = new MeshStandardMaterial({
      color: 0x155b76,
      emissive: 0x0a3148,
      emissiveIntensity: 0.38,
      roughness: 0.42,
      metalness: 0,
    });
    const mesh = new InstancedMesh(geometry, material, lakes.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    lakes.forEach((lake, index) => {
      matrix.compose(
        new Vector3(lake.x - scene.width / 2, 0.035, lake.y - scene.depth / 2),
        quaternion,
        new Vector3(lake.radiusX, 0.5, lake.radiusY),
      );
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addLakeEdgeDetails(scene: CityScene) {
    const stones: Array<{ x: number; z: number; scale: number; color: Color }> = [];
    const reeds: Array<{ x: number; z: number; scale: number; rotation: number }> = [];
    const lilies: Array<{ x: number; z: number; scale: number; rotation: number; color: Color }> = [];
    const ripples: Array<{ x: number; z: number; scaleX: number; scaleZ: number; rotation: number }> = [];
    for (const park of scene.parks) {
      const lake = parkLakeFor(park);
      if (!lake) continue;
      const rimCount = clamp(Math.round((lake.radiusX + lake.radiusY) * 1.35), 18, 48);
      for (let index = 0; index < rimCount && stones.length < MAX_LAKE_EDGE_DETAILS; index += 1) {
        const seed = hashText(`${park.id}:lake-stone:${index}`);
        const angle = (index / rimCount) * Math.PI * 2 + ((seed >>> 7) % 22) / 100;
        const radiusNoise = 0.45 + ((seed >>> 14) % 90) / 100;
        stones.push({
          x: lake.x + Math.cos(angle) * (lake.radiusX + radiusNoise) - scene.width / 2,
          z: lake.y + Math.sin(angle) * (lake.radiusY + radiusNoise) - scene.depth / 2,
          scale: 0.28 + ((seed >>> 22) % 45) / 100,
          color: new Color(seed % 3 === 0 ? "#64756b" : seed % 3 === 1 ? "#708078" : "#52655c"),
        });
        if (index % 3 === 0 && reeds.length < MAX_LAKE_EDGE_DETAILS) {
          const inner = 0.78 + ((seed >>> 18) % 18) / 100;
          reeds.push({
            x: lake.x + Math.cos(angle + 0.08) * lake.radiusX * inner - scene.width / 2,
            z: lake.y + Math.sin(angle + 0.08) * lake.radiusY * inner - scene.depth / 2,
            scale: 0.56 + ((seed >>> 25) % 48) / 100,
            rotation: angle,
          });
        }
      }
      const lilyCount = clamp(Math.floor((lake.radiusX + lake.radiusY) / 3.8), 3, 10);
      for (let index = 0; index < lilyCount && lilies.length < MAX_LAKE_EDGE_DETAILS; index += 1) {
        const seed = hashText(`${park.id}:lily:${index}`);
        const angle = ((seed >>> 3) % 628) / 100;
        const radius = 0.18 + ((seed >>> 13) % 55) / 100;
        lilies.push({
          x: lake.x + Math.cos(angle) * lake.radiusX * radius - scene.width / 2,
          z: lake.y + Math.sin(angle) * lake.radiusY * radius - scene.depth / 2,
          scale: 0.42 + ((seed >>> 23) % 42) / 100,
          rotation: angle,
          color: new Color(seed % 4 === 0 ? "#76a84f" : "#4f8f46"),
        });
      }
      const rippleCount = clamp(Math.floor(lilyCount / 2), 2, 5);
      for (let index = 0; index < rippleCount; index += 1) {
        const seed = hashText(`${park.id}:lake-ripple:${index}`);
        ripples.push({
          x: lake.x + (((seed >>> 4) % 1000) / 1000 - 0.5) * lake.radiusX - scene.width / 2,
          z: lake.y + (((seed >>> 14) % 1000) / 1000 - 0.5) * lake.radiusY - scene.depth / 2,
          scaleX: 0.8 + ((seed >>> 22) % 55) / 100,
          scaleZ: 0.48 + ((seed >>> 27) % 34) / 100,
          rotation: ((seed >>> 8) % 628) / 100,
        });
      }
    }
    if (stones.length > 0) {
      const geometry = new SphereGeometry(0.7, 5, 4);
      const material = new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.98 });
      const mesh = new InstancedMesh(geometry, material, stones.length);
      const matrix = new Matrix4();
      const quaternion = new Quaternion();
      stones.forEach((stone, index) => {
        matrix.compose(
          new Vector3(stone.x, 0.18, stone.z),
          quaternion,
          new Vector3(stone.scale * 1.35, stone.scale * 0.58, stone.scale),
        );
        mesh.setMatrixAt(index, matrix);
        mesh.setColorAt(index, stone.color);
      });
      mesh.instanceMatrix.setUsage(StaticDrawUsage);
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
      this.threeScene.add(mesh);
      this.disposableGeometries.add(geometry);
      this.disposableMaterials.add(material);
    }
    if (reeds.length > 0) {
      const geometry = new ConeGeometry(0.18, 1.45, 4);
      const material = new MeshStandardMaterial({ color: 0x71934d, roughness: 0.98 });
      const mesh = new InstancedMesh(geometry, material, reeds.length);
      const matrix = new Matrix4();
      const quaternion = new Quaternion();
      reeds.forEach((reed, index) => {
        quaternion.setFromAxisAngle(this.waveAxis, reed.rotation);
        matrix.compose(
          new Vector3(reed.x, 0.68 * reed.scale, reed.z),
          quaternion,
          new Vector3(reed.scale * 1.4, reed.scale, reed.scale * 0.65),
        );
        mesh.setMatrixAt(index, matrix);
      });
      mesh.instanceMatrix.setUsage(StaticDrawUsage);
      this.threeScene.add(mesh);
      this.disposableGeometries.add(geometry);
      this.disposableMaterials.add(material);
    }
    if (lilies.length > 0) {
      const geometry = new CylinderGeometry(0.72, 0.72, 0.06, 9);
      const material = new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.8 });
      const mesh = new InstancedMesh(geometry, material, lilies.length);
      const matrix = new Matrix4();
      const quaternion = new Quaternion();
      lilies.forEach((lily, index) => {
        quaternion.setFromAxisAngle(this.waveAxis, lily.rotation);
        matrix.compose(new Vector3(lily.x, 0.12, lily.z), quaternion, new Vector3(lily.scale, 1, lily.scale * 0.82));
        mesh.setMatrixAt(index, matrix);
        mesh.setColorAt(index, lily.color);
      });
      mesh.instanceMatrix.setUsage(StaticDrawUsage);
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
      this.threeScene.add(mesh);
      this.disposableGeometries.add(geometry);
      this.disposableMaterials.add(material);
    }
    if (ripples.length > 0) {
      const geometry = new TorusGeometry(1, 0.055, 4, 18);
      const material = new MeshStandardMaterial({
        color: 0x83cfe3,
        emissive: 0x164e63,
        emissiveIntensity: 0.28,
        roughness: 0.35,
        transparent: true,
        opacity: 0.52,
      });
      const mesh = new InstancedMesh(geometry, material, ripples.length);
      const matrix = new Matrix4();
      const quaternion = new Quaternion();
      const euler = new Euler(Math.PI / 2, 0, 0);
      ripples.forEach((ripple, index) => {
        euler.z = ripple.rotation;
        quaternion.setFromEuler(euler);
        matrix.compose(
          new Vector3(ripple.x, 0.16, ripple.z),
          quaternion,
          new Vector3(ripple.scaleX, ripple.scaleZ, 1),
        );
        mesh.setMatrixAt(index, matrix);
      });
      mesh.instanceMatrix.setUsage(StaticDrawUsage);
      this.threeScene.add(mesh);
      this.disposableGeometries.add(geometry);
      this.disposableMaterials.add(material);
    }
  }

  private addGrassDetails(scene: CityScene) {
    const patches: Array<{ x: number; z: number; scale: number; color: Color }> = [];
    for (const park of scene.parks) {
      const count = clamp(Math.floor((park.width * park.depth) / 18), 10, 84);
      const lake = parkLakeFor(park);
      for (let index = 0; index < count && patches.length < MAX_GRASS_PATCHES; index += 1) {
        const seed = hashText(`${park.id}:grass:${index}`);
        const x = park.x + ((seed >>> 6) % 1000) / 1000 * park.width;
        const z = park.y + ((seed >>> 16) % 1000) / 1000 * park.depth;
        if (pointInLake(x, z, lake, 1.6) || pointNearRoad(x, z, scene.roads, SIDEWALK_WIDTH + 0.5)) continue;
        patches.push({
          x: x - scene.width / 2,
          z: z - scene.depth / 2,
          scale: 0.65 + ((seed >>> 26) % 38) / 100,
          color: new Color(seed % 4 === 0 ? "#6ca95d" : seed % 3 === 0 ? "#4f914b" : "#3f7f43"),
        });
      }
    }
    for (const district of scene.districts) {
      const count = clamp(Math.ceil(Math.sqrt(district.fileCount)), 2, 12);
      for (let index = 0; index < count && patches.length < MAX_GRASS_PATCHES; index += 1) {
        const seed = hashText(`${district.id}:grass:${index}`);
        const edge = seed % 4;
        const progress = ((seed >>> 8) % 1000) / 1000;
        patches.push({
          x: (edge < 2 ? district.x + progress * district.width : district.x + (edge === 2 ? -5 : district.width + 5))
            - scene.width / 2,
          z: (edge >= 2 ? district.y + progress * district.depth : district.y + (edge === 0 ? -5 : district.depth + 5))
            - scene.depth / 2,
          scale: 0.58 + ((seed >>> 20) % 36) / 100,
          color: new Color(seed % 3 === 0 ? "#609f54" : "#477f46"),
        });
      }
    }
    if (patches.length === 0) return;
    const geometry = new ConeGeometry(0.16, 0.56, 5);
    const material = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      emissive: 0x102f18,
      emissiveIntensity: 0.18,
      roughness: 0.98,
    });
    const mesh = new InstancedMesh(geometry, material, patches.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    patches.forEach((patch, index) => {
      quaternion.setFromAxisAngle(new Vector3(0, 1, 0), (hashText(`grass-rot:${index}`) % 628) / 100);
      matrix.compose(
        new Vector3(patch.x, 0.28, patch.z),
        quaternion,
        new Vector3(patch.scale, patch.scale * 0.92, patch.scale),
      );
      mesh.setMatrixAt(index, matrix);
      mesh.setColorAt(index, patch.color);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addMeadowDetails(scene: CityScene) {
    const details: Array<{ x: number; z: number; scale: number; color: Color }> = [];
    const colors = ["#7ddf75", "#8fd46b", "#d9f99d", "#facc15", "#f0abfc", "#67e8f9"];
    for (const park of scene.parks) {
      const lake = parkLakeFor(park);
      const count = clamp(Math.floor((park.width * park.depth) / 55), 8, 44);
      for (let index = 0; index < count && details.length < MAX_MEADOW_DETAILS; index += 1) {
        const seed = hashText(`${park.id}:meadow:${index}`);
        const x = park.x + 1.1 + ((seed >>> 5) % 1000) / 1000 * Math.max(1, park.width - 2.2);
        const z = park.y + 1.1 + ((seed >>> 15) % 1000) / 1000 * Math.max(1, park.depth - 2.2);
        if (pointInLake(x, z, lake, 1.2) || pointNearRoad(x, z, scene.roads, SIDEWALK_WIDTH + 0.45)) continue;
        details.push({
          x: x - scene.width / 2,
          z: z - scene.depth / 2,
          scale: 0.28 + ((seed >>> 24) % 34) / 100,
          color: new Color(colors[seed % colors.length]),
        });
      }
    }
    if (details.length === 0) return;
    const geometry = new SphereGeometry(0.32, 6, 4);
    const material = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      roughness: 0.95,
      metalness: 0,
    });
    const mesh = new InstancedMesh(geometry, material, details.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    details.forEach((detail, index) => {
      matrix.compose(
        new Vector3(detail.x, 0.18, detail.z),
        quaternion,
        new Vector3(detail.scale * 1.15, detail.scale * 0.42, detail.scale),
      );
      mesh.setMatrixAt(index, matrix);
      mesh.setColorAt(index, detail.color);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addShrubs(scene: CityScene) {
    const shrubs: Array<{ x: number; z: number; scale: number; color: Color }> = [];
    for (const park of scene.parks) {
      const lake = parkLakeFor(park);
      const count = clamp(Math.floor((park.width * park.depth) / 125), 4, 30);
      for (let index = 0; index < count && shrubs.length < MAX_SHRUBS; index += 1) {
        const seed = hashText(`${park.id}:shrub:${index}`);
        const x = park.x + 1.8 + ((seed >>> 6) % 1000) / 1000 * Math.max(1, park.width - 3.6);
        const z = park.y + 1.8 + ((seed >>> 16) % 1000) / 1000 * Math.max(1, park.depth - 3.6);
        if (pointInLake(x, z, lake, 1.6) || pointNearRoad(x, z, scene.roads, SIDEWALK_WIDTH + 0.7)) continue;
        shrubs.push({
          x: x - scene.width / 2,
          z: z - scene.depth / 2,
          scale: 0.55 + ((seed >>> 24) % 42) / 100,
          color: new Color(seed % 3 === 0 ? "#3f8f55" : seed % 3 === 1 ? "#2f7d4a" : "#5a9b50"),
        });
      }
    }
    if (shrubs.length === 0) return;
    const geometry = new SphereGeometry(0.72, 7, 5);
    const material = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      roughness: 0.98,
      metalness: 0,
    });
    const mesh = new InstancedMesh(geometry, material, shrubs.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    shrubs.forEach((shrub, index) => {
      matrix.compose(
        new Vector3(shrub.x, 0.32, shrub.z),
        quaternion,
        new Vector3(shrub.scale * 1.25, shrub.scale * 0.42, shrub.scale),
      );
      mesh.setMatrixAt(index, matrix);
      mesh.setColorAt(index, shrub.color);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addForestFloorDetails(scene: CityScene) {
    const rocks: Array<{ x: number; z: number; scale: number; rotation: number; color: Color }> = [];
    const logs: Array<{ x: number; z: number; length: number; rotation: number }> = [];
    const mushrooms: Array<{ x: number; z: number; scale: number; color: Color }> = [];
    for (const park of scene.parks) {
      const lake = parkLakeFor(park);
      const count = clamp(Math.floor((park.width * park.depth) / 90), 8, 58);
      for (let index = 0; index < count && rocks.length + logs.length + mushrooms.length < MAX_FOREST_FLOOR_DETAILS; index += 1) {
        const seed = hashText(`${park.id}:forest-floor:${index}`);
        const x = park.x + 1.4 + ((seed >>> 5) % 1000) / 1000 * Math.max(1, park.width - 2.8);
        const z = park.y + 1.4 + ((seed >>> 15) % 1000) / 1000 * Math.max(1, park.depth - 2.8);
        if (pointInLake(x, z, lake, 2.1) || pointNearRoad(x, z, scene.roads, SIDEWALK_WIDTH + 0.65)) continue;
        const centeredX = x - scene.width / 2;
        const centeredZ = z - scene.depth / 2;
        const roll = seed % 10;
        if (roll < 4) {
          rocks.push({
            x: centeredX,
            z: centeredZ,
            scale: 0.22 + ((seed >>> 24) % 55) / 100,
            rotation: ((seed >>> 12) % 628) / 100,
            color: new Color(roll % 2 === 0 ? "#56675d" : "#65766b"),
          });
        } else if (roll < 6) {
          logs.push({
            x: centeredX,
            z: centeredZ,
            length: 1.6 + ((seed >>> 22) % 22) / 10,
            rotation: ((seed >>> 9) % 628) / 100,
          });
        } else {
          mushrooms.push({
            x: centeredX,
            z: centeredZ,
            scale: 0.18 + ((seed >>> 25) % 28) / 100,
            color: new Color(roll === 9 ? "#d9b38c" : roll === 8 ? "#b66a52" : "#d4c0a5"),
          });
        }
      }
    }
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    const euler = new Euler();
    if (rocks.length > 0) {
      const geometry = new SphereGeometry(0.9, 5, 4);
      const material = new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 1 });
      const mesh = new InstancedMesh(geometry, material, rocks.length);
      rocks.forEach((rock, index) => {
        quaternion.setFromAxisAngle(this.waveAxis, rock.rotation);
        matrix.compose(
          new Vector3(rock.x, 0.18, rock.z),
          quaternion,
          new Vector3(rock.scale * 1.25, rock.scale * 0.55, rock.scale),
        );
        mesh.setMatrixAt(index, matrix);
        mesh.setColorAt(index, rock.color);
      });
      mesh.instanceMatrix.setUsage(StaticDrawUsage);
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
      this.threeScene.add(mesh);
      this.disposableGeometries.add(geometry);
      this.disposableMaterials.add(material);
    }
    if (logs.length > 0) {
      const geometry = new CylinderGeometry(0.28, 0.34, 1, 7);
      const material = new MeshStandardMaterial({ color: 0x715039, roughness: 1 });
      const mesh = new InstancedMesh(geometry, material, logs.length);
      logs.forEach((log, index) => {
        euler.set(0, log.rotation, Math.PI / 2);
        quaternion.setFromEuler(euler);
        matrix.compose(new Vector3(log.x, 0.3, log.z), quaternion, new Vector3(1, log.length, 1));
        mesh.setMatrixAt(index, matrix);
      });
      mesh.instanceMatrix.setUsage(StaticDrawUsage);
      this.threeScene.add(mesh);
      this.disposableGeometries.add(geometry);
      this.disposableMaterials.add(material);
    }
    if (mushrooms.length > 0) {
      const stemGeometry = new CylinderGeometry(0.16, 0.2, 0.7, 5);
      const capGeometry = new SphereGeometry(0.52, 6, 4);
      const stemMaterial = new MeshStandardMaterial({ color: 0xcdbfa9, roughness: 1 });
      const capMaterial = new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.95 });
      const stems = new InstancedMesh(stemGeometry, stemMaterial, mushrooms.length);
      const caps = new InstancedMesh(capGeometry, capMaterial, mushrooms.length);
      mushrooms.forEach((mushroom, index) => {
        quaternion.identity();
        matrix.compose(
          new Vector3(mushroom.x, mushroom.scale * 0.48, mushroom.z),
          quaternion,
          new Vector3(mushroom.scale, mushroom.scale, mushroom.scale),
        );
        stems.setMatrixAt(index, matrix);
        matrix.compose(
          new Vector3(mushroom.x, mushroom.scale * 0.86, mushroom.z),
          quaternion,
          new Vector3(mushroom.scale, mushroom.scale * 0.42, mushroom.scale),
        );
        caps.setMatrixAt(index, matrix);
        caps.setColorAt(index, mushroom.color);
      });
      stems.instanceMatrix.setUsage(StaticDrawUsage);
      caps.instanceMatrix.setUsage(StaticDrawUsage);
      if (caps.instanceColor) caps.instanceColor.needsUpdate = true;
      this.threeScene.add(stems, caps);
      this.disposableGeometries.add(stemGeometry);
      this.disposableGeometries.add(capGeometry);
      this.disposableMaterials.add(stemMaterial);
      this.disposableMaterials.add(capMaterial);
    }
  }

  private addParkEntities(scene: CityScene) {
    type ParkPart = {
      x: number;
      y: number;
      z: number;
      sx: number;
      sy: number;
      sz: number;
      rotation: number;
      color: Color;
    };
    const woodParts: ParkPart[] = [];
    const supportParts: ParkPart[] = [];
    const trailStones: ParkPart[] = [];
    const lampBulbs: Array<{ x: number; y: number; z: number; scale: number }> = [];
    const creatureParts: ParkPart[] = [];
    let creatureCount = 0;

    const addPart = (
      target: ParkPart[],
      centerX: number,
      centerZ: number,
      rotation: number,
      localX: number,
      localZ: number,
      y: number,
      sx: number,
      sy: number,
      sz: number,
      color: string,
    ) => {
      if (target.length >= MAX_PARK_ENTITY_PARTS) return;
      const cosine = Math.cos(rotation);
      const sine = Math.sin(rotation);
      target.push({
        x: centerX + localX * cosine + localZ * sine,
        y,
        z: centerZ - localX * sine + localZ * cosine,
        sx,
        sy,
        sz,
        rotation,
        color: new Color(color),
      });
    };

    const findSpot = (park: CityPark, lake: ParkLake | null, key: string): { x: number; z: number; rotation: number } | null => {
      if (park.width < 13 || park.depth < 13) return null;
      for (let attempt = 0; attempt < 7; attempt += 1) {
        const seed = hashText(`${park.id}:${key}:${attempt}`);
        const x = park.x + 4 + ((seed >>> 5) % 1000) / 1000 * Math.max(1, park.width - 8);
        const z = park.y + 4 + ((seed >>> 15) % 1000) / 1000 * Math.max(1, park.depth - 8);
        if (pointInLake(x, z, lake, 3.2) || pointNearRoad(x, z, scene.roads, SIDEWALK_WIDTH + 1.2)) continue;
        return {
          x: x - scene.width / 2,
          z: z - scene.depth / 2,
          rotation: ((seed >>> 22) % 628) / 100,
        };
      }
      return null;
    };

    const addBench = (spot: { x: number; z: number; rotation: number }, seed: number) => {
      const wood = seed % 2 === 0 ? "#8a5d36" : "#75502f";
      addPart(woodParts, spot.x, spot.z, spot.rotation, 0, 0, 0.62, 3.4, 0.22, 0.82, wood);
      addPart(woodParts, spot.x, spot.z, spot.rotation, 0, 0.42, 1.28, 3.4, 1.05, 0.18, wood);
      for (const localX of [-1.2, 1.2]) {
        addPart(supportParts, spot.x, spot.z, spot.rotation, localX, -0.2, 0.3, 0.18, 0.62, 0.18, "#374151");
        addPart(supportParts, spot.x, spot.z, spot.rotation, localX, 0.32, 0.58, 0.18, 1.12, 0.18, "#374151");
      }
    };

    const addPicnicTable = (spot: { x: number; z: number; rotation: number }, seed: number) => {
      const wood = seed % 2 === 0 ? "#91623b" : "#7c5534";
      addPart(woodParts, spot.x, spot.z, spot.rotation, 0, 0, 0.96, 3.8, 0.24, 1.55, wood);
      addPart(woodParts, spot.x, spot.z, spot.rotation, 0, -1.3, 0.56, 3.55, 0.2, 0.54, wood);
      addPart(woodParts, spot.x, spot.z, spot.rotation, 0, 1.3, 0.56, 3.55, 0.2, 0.54, wood);
      for (const localX of [-1.25, 1.25]) {
        addPart(supportParts, spot.x, spot.z, spot.rotation, localX, -0.42, 0.42, 0.2, 0.86, 0.2, "#414852");
        addPart(supportParts, spot.x, spot.z, spot.rotation, localX, 0.42, 0.42, 0.2, 0.86, 0.2, "#414852");
      }
    };

    const addCreature = (x: number, z: number, rotation: number, scale: number, duck: boolean, seed: number) => {
      if (creatureCount >= MAX_PARK_CREATURES) return;
      creatureCount += 1;
      const forwardX = Math.sin(rotation);
      const forwardZ = Math.cos(rotation);
      const rightX = Math.cos(rotation);
      const rightZ = -Math.sin(rotation);
      const baseY = duck ? 0.38 : 0.44;
      const bodyColor = duck
        ? (seed % 2 === 0 ? "#6f7f52" : "#8b6f47")
        : (seed % 2 === 0 ? "#64748b" : "#7c6f64");
      creatureParts.push(
        { x, y: baseY, z, sx: scale * 0.62, sy: scale * 0.38, sz: scale * 0.88, rotation, color: new Color(bodyColor) },
        {
          x: x + forwardX * scale * 0.58,
          y: baseY + scale * 0.34,
          z: z + forwardZ * scale * 0.58,
          sx: scale * 0.34,
          sy: scale * 0.34,
          sz: scale * 0.34,
          rotation,
          color: new Color(duck ? "#405345" : "#59636f"),
        },
        {
          x: x + forwardX * scale * 0.92,
          y: baseY + scale * 0.31,
          z: z + forwardZ * scale * 0.92,
          sx: scale * 0.2,
          sy: scale * 0.12,
          sz: scale * 0.3,
          rotation,
          color: new Color(duck ? "#d59a35" : "#a78662"),
        },
        {
          x: x + rightX * scale * 0.42,
          y: baseY + scale * 0.08,
          z: z + rightZ * scale * 0.42,
          sx: scale * 0.16,
          sy: scale * 0.28,
          sz: scale * 0.54,
          rotation,
          color: new Color("#38444c"),
        },
        {
          x: x - rightX * scale * 0.42,
          y: baseY + scale * 0.08,
          z: z - rightZ * scale * 0.42,
          sx: scale * 0.16,
          sy: scale * 0.28,
          sz: scale * 0.54,
          rotation,
          color: new Color("#38444c"),
        },
      );
    };

    for (const park of scene.parks) {
      const lake = parkLakeFor(park);
      const area = park.width * park.depth;
      const amenityCount = clamp(Math.floor(area / 1050), 1, 3);
      for (let index = 0; index < amenityCount; index += 1) {
        const spot = findSpot(park, lake, `amenity:${index}`);
        if (!spot) continue;
        const seed = hashText(`${park.id}:amenity-kind:${index}`);
        if (seed % 3 === 0 && area > 650) addPicnicTable(spot, seed);
        else addBench(spot, seed);
      }

      const signSpot = findSpot(park, lake, "park-sign");
      if (signSpot && area > 420) {
        addPart(supportParts, signSpot.x, signSpot.z, signSpot.rotation, 0, 0, 1.1, 0.16, 2.2, 0.16, "#4b5563");
        addPart(woodParts, signSpot.x, signSpot.z, signSpot.rotation, 0, 0, 2.02, 1.75, 0.72, 0.18, "#6d5138");
      }

      const lampSpot = area > 760 ? findSpot(park, lake, "park-lamp") : null;
      if (lampSpot && lampBulbs.length < 96) {
        addPart(supportParts, lampSpot.x, lampSpot.z, lampSpot.rotation, 0, 0, 1.55, 0.12, 3.1, 0.12, "#303842");
        lampBulbs.push({ x: lampSpot.x, y: 3.18, z: lampSpot.z, scale: 0.32 });
      }

      const trailCount = clamp(Math.floor(Math.sqrt(area) / 5), 4, 13);
      for (let index = 0; index < trailCount && trailStones.length < MAX_PARK_ENTITY_PARTS; index += 1) {
        const progress = (index + 0.5) / trailCount;
        const seed = hashText(`${park.id}:trail:${index}`);
        const x = park.x + 3 + progress * Math.max(1, park.width - 6) + Math.sin(progress * Math.PI * 2) * 1.4;
        const z = park.y + park.depth * 0.22 + progress * park.depth * 0.52 + (((seed >>> 14) % 100) / 100 - 0.5) * 1.2;
        if (pointInLake(x, z, lake, 1.4) || pointNearRoad(x, z, scene.roads, SIDEWALK_WIDTH + 0.4)) continue;
        trailStones.push({
          x: x - scene.width / 2,
          y: 0.08,
          z: z - scene.depth / 2,
          sx: 0.65 + (seed % 28) / 100,
          sy: 0.12,
          sz: 0.48 + ((seed >>> 20) % 25) / 100,
          rotation: ((seed >>> 7) % 628) / 100,
          color: new Color(seed % 2 === 0 ? "#7b877d" : "#69766d"),
        });
      }

      if (lake) {
        const duckCount = clamp(Math.floor((lake.radiusX + lake.radiusY) / 10), 1, 3);
        for (let index = 0; index < duckCount; index += 1) {
          const seed = hashText(`${park.id}:duck:${index}`);
          const angle = ((seed >>> 8) % 628) / 100;
          const radius = 0.18 + ((seed >>> 18) % 45) / 100;
          addCreature(
            lake.x + Math.cos(angle) * lake.radiusX * radius - scene.width / 2,
            lake.y + Math.sin(angle) * lake.radiusY * radius - scene.depth / 2,
            ((seed >>> 4) % 628) / 100,
            0.58 + ((seed >>> 25) % 24) / 100,
            true,
            seed,
          );
        }
      } else {
        const birdSpot = findSpot(park, null, "ground-bird");
        if (birdSpot && area > 500) {
          const seed = hashText(`${park.id}:ground-bird`);
          addCreature(birdSpot.x, birdSpot.z, birdSpot.rotation, 0.46 + (seed % 20) / 100, false, seed);
        }
      }
    }

    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    const addPartMesh = (parts: ParkPart[], geometry: BufferGeometry, material: MeshStandardMaterial) => {
      if (parts.length === 0) {
        geometry.dispose();
        material.dispose();
        return;
      }
      const mesh = new InstancedMesh(geometry, material, parts.length);
      parts.forEach((part, index) => {
        quaternion.setFromAxisAngle(this.waveAxis, part.rotation);
        matrix.compose(new Vector3(part.x, part.y, part.z), quaternion, new Vector3(part.sx, part.sy, part.sz));
        mesh.setMatrixAt(index, matrix);
        mesh.setColorAt(index, part.color);
      });
      mesh.instanceMatrix.setUsage(StaticDrawUsage);
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
      this.threeScene.add(mesh);
      this.disposableGeometries.add(geometry);
      this.disposableMaterials.add(material);
    };

    addPartMesh(
      woodParts,
      new BoxGeometry(1, 1, 1),
      new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.9 }),
    );
    addPartMesh(
      supportParts,
      new BoxGeometry(1, 1, 1),
      new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.75, metalness: 0.1 }),
    );
    addPartMesh(
      trailStones,
      new SphereGeometry(1, 6, 4),
      new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 1 }),
    );
    addPartMesh(
      creatureParts,
      new SphereGeometry(1, 7, 5),
      new MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.92 }),
    );

    if (lampBulbs.length > 0) {
      const geometry = new SphereGeometry(0.5, 7, 5);
      const material = new MeshStandardMaterial({
        color: 0xf5ddb0,
        emissive: 0xf59e0b,
        emissiveIntensity: 0.52,
        roughness: 0.4,
      });
      const mesh = new InstancedMesh(geometry, material, lampBulbs.length);
      lampBulbs.forEach((bulb, index) => {
        quaternion.identity();
        matrix.compose(
          new Vector3(bulb.x, bulb.y, bulb.z),
          quaternion,
          new Vector3(bulb.scale, bulb.scale, bulb.scale),
        );
        mesh.setMatrixAt(index, matrix);
      });
      mesh.instanceMatrix.setUsage(StaticDrawUsage);
      this.threeScene.add(mesh);
      this.disposableGeometries.add(geometry);
      this.disposableMaterials.add(material);
    }
  }

  private addRoads(scene: CityScene) {
    if (scene.roads.length === 0) return;
    const geometry = new BoxGeometry(1, 1, 1);
    const material = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      emissive: 0x080b0f,
      emissiveIntensity: 0.18,
      roughness: 0.9,
      metalness: 0.02,
    });
    const mesh = new InstancedMesh(geometry, material, scene.roads.length);
    const matrix = new Matrix4();
    const position = new Vector3();
    const scale = new Vector3();
    const quaternion = new Quaternion();
    scene.roads.forEach((road, index) => {
      position.set(
        road.x + road.width / 2 - scene.width / 2,
        0.46,
        road.y + road.depth / 2 - scene.depth / 2,
      );
      scale.set(road.width, 0.14, road.depth);
      matrix.compose(position, quaternion, scale);
      mesh.setMatrixAt(index, matrix);
      mesh.setColorAt(
        index,
        new Color(ROAD_COLOR),
      );
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    this.roadMesh = mesh;
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
    this.addRoadIntersections(scene);
    this.addRoadSidewalks(scene);
    this.addSidewalkCorners(scene);
    this.addRoadLaneMarks(scene);
    this.addTrafficLights(scene);
  }

  private addRoadIntersections(scene: CityScene) {
    const horizontal = scene.roads.filter((road) => roadOrientation(road) === "horizontal");
    const vertical = scene.roads.filter((road) => roadOrientation(road) === "vertical");
    const intersections: Array<{ x: number; y: number; width: number; depth: number }> = [];
    const seen = new Set<string>();
    for (const hRoad of horizontal) {
      for (const vRoad of vertical) {
        if (!roadIntersects(hRoad, vRoad)) continue;
        const left = Math.max(hRoad.x, vRoad.x);
        const right = Math.min(hRoad.x + hRoad.width, vRoad.x + vRoad.width);
        const top = Math.max(hRoad.y, vRoad.y);
        const bottom = Math.min(hRoad.y + hRoad.depth, vRoad.y + vRoad.depth);
        const key = `${Math.round(left / 6)}:${Math.round(top / 6)}`;
        if (seen.has(key)) continue;
        seen.add(key);
        intersections.push({
          x: left - 0.2,
          y: top - 0.2,
          width: Math.max(VISUAL_ROAD_WIDTH, right - left + 0.4),
          depth: Math.max(VISUAL_ROAD_WIDTH, bottom - top + 0.4),
        });
      }
    }
    if (intersections.length === 0) return;
    const geometry = new BoxGeometry(1, 1, 1);
    const material = new MeshStandardMaterial({
      color: ROAD_COLOR,
      emissive: 0x080b0f,
      emissiveIntensity: 0.18,
      roughness: 0.9,
      metalness: 0.02,
    });
    const mesh = new InstancedMesh(geometry, material, intersections.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    intersections.forEach((intersection, index) => {
      matrix.compose(
        new Vector3(
          intersection.x + intersection.width / 2 - scene.width / 2,
          0.532,
          intersection.y + intersection.depth / 2 - scene.depth / 2,
        ),
        quaternion,
        new Vector3(intersection.width, 0.006, intersection.depth),
      );
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addRoadSidewalks(scene: CityScene) {
    const sidewalks: Array<{ x: number; y: number; width: number; depth: number }> = [];
    const roadSidewalks = scene.roads.filter((road) => road.kind !== "avenue");
    const horizontalRoads = scene.roads.filter((road) => roadOrientation(road) === "horizontal");
    const verticalRoads = scene.roads.filter((road) => roadOrientation(road) === "vertical");
    const sidewalkWidth = SIDEWALK_WIDTH;
    for (const road of roadSidewalks) {
      const horizontal = roadOrientation(road) === "horizontal";
      if (horizontal) {
        const blocks = verticalRoads
          .filter((other) => roadIntersects(road, other))
          .map((other) => ({ start: other.x, end: other.x + other.width }));
        for (const segment of splitByBlocks(road.x, road.x + road.width, blocks)) {
          sidewalks.push(
            { x: segment.start, y: road.y - sidewalkWidth, width: segment.end - segment.start, depth: sidewalkWidth },
            { x: segment.start, y: road.y + road.depth, width: segment.end - segment.start, depth: sidewalkWidth },
          );
        }
      } else {
        const blocks = horizontalRoads
          .filter((other) => roadIntersects(road, other))
          .map((other) => ({ start: other.y, end: other.y + other.depth }));
        for (const segment of splitByBlocks(road.y, road.y + road.depth, blocks)) {
          sidewalks.push(
            { x: road.x - sidewalkWidth, y: segment.start, width: sidewalkWidth, depth: segment.end - segment.start },
            { x: road.x + road.width, y: segment.start, width: sidewalkWidth, depth: segment.end - segment.start },
          );
        }
      }
    }
    if (sidewalks.length === 0) return;
    const geometry = new BoxGeometry(1, 1, 1);
    const material = new MeshStandardMaterial({
      color: SIDEWALK_COLOR,
      roughness: 0.92,
      metalness: 0,
    });
    const mesh = new InstancedMesh(geometry, material, sidewalks.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    sidewalks.forEach((sidewalk, index) => {
      matrix.compose(
        new Vector3(
          sidewalk.x + sidewalk.width / 2 - scene.width / 2,
          0.555,
          sidewalk.y + sidewalk.depth / 2 - scene.depth / 2,
        ),
        quaternion,
        new Vector3(sidewalk.width, 0.05, sidewalk.depth),
      );
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addSidewalkCorners(scene: CityScene) {
    const sidewalkRoads = scene.roads.filter((road) => road.kind !== "avenue");
    const horizontal = sidewalkRoads.filter((road) => roadOrientation(road) === "horizontal");
    const vertical = sidewalkRoads.filter((road) => roadOrientation(road) === "vertical");
    const sidewalkWidth = SIDEWALK_WIDTH;
    const corners: Array<{ x: number; y: number; width: number; depth: number }> = [];
    const seen = new Set<string>();

    const addCorner = (x: number, y: number, width = sidewalkWidth * 1.15, depth = sidewalkWidth * 1.15) => {
      if (corners.length >= MAX_SIDEWALK_CORNERS) return;
      const key = `${Math.round(x * 2)}:${Math.round(y * 2)}:${Math.round(width * 2)}:${Math.round(depth * 2)}`;
      if (seen.has(key)) return;
      seen.add(key);
      corners.push({ x, y, width, depth });
    };

    for (const hRoad of horizontal) {
      for (const vRoad of vertical) {
        if (corners.length >= MAX_SIDEWALK_CORNERS) break;
        if (!roadIntersects(hRoad, vRoad)) continue;
        const left = Math.max(hRoad.x, vRoad.x);
        const right = Math.min(hRoad.x + hRoad.width, vRoad.x + vRoad.width);
        const top = Math.max(hRoad.y, vRoad.y);
        const bottom = Math.min(hRoad.y + hRoad.depth, vRoad.y + vRoad.depth);
        const width = right - left;
        const depth = bottom - top;
        addCorner(left - sidewalkWidth, top - sidewalkWidth, width + sidewalkWidth * 2, sidewalkWidth);
        addCorner(left - sidewalkWidth, bottom, width + sidewalkWidth * 2, sidewalkWidth);
        addCorner(left - sidewalkWidth, top - sidewalkWidth, sidewalkWidth, depth + sidewalkWidth * 2);
        addCorner(right, top - sidewalkWidth, sidewalkWidth, depth + sidewalkWidth * 2);
      }
    }

    for (const road of sidewalkRoads) {
      if (corners.length >= MAX_SIDEWALK_CORNERS) break;
      const horizontalRoad = roadOrientation(road) === "horizontal";
      if (horizontalRoad) {
        addCorner(road.x - sidewalkWidth, road.y - sidewalkWidth, sidewalkWidth, sidewalkWidth);
        addCorner(road.x - sidewalkWidth, road.y + road.depth, sidewalkWidth, sidewalkWidth);
        addCorner(road.x + road.width, road.y - sidewalkWidth, sidewalkWidth, sidewalkWidth);
        addCorner(road.x + road.width, road.y + road.depth, sidewalkWidth, sidewalkWidth);
      } else {
        addCorner(road.x - sidewalkWidth, road.y - sidewalkWidth, sidewalkWidth, sidewalkWidth);
        addCorner(road.x + road.width, road.y - sidewalkWidth, sidewalkWidth, sidewalkWidth);
        addCorner(road.x - sidewalkWidth, road.y + road.depth, sidewalkWidth, sidewalkWidth);
        addCorner(road.x + road.width, road.y + road.depth, sidewalkWidth, sidewalkWidth);
      }
    }

    if (corners.length === 0) return;
    const geometry = new BoxGeometry(1, 1, 1);
    const material = new MeshStandardMaterial({
      color: SIDEWALK_COLOR,
      roughness: 0.92,
      metalness: 0,
    });
    const mesh = new InstancedMesh(geometry, material, corners.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    corners.forEach((corner, index) => {
      matrix.compose(
        new Vector3(corner.x + corner.width / 2 - scene.width / 2, 0.555, corner.y + corner.depth / 2 - scene.depth / 2),
        quaternion,
        new Vector3(corner.width, 0.05, corner.depth),
      );
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addRoadLaneMarks(scene: CityScene) {
    const marks: Array<{ x: number; y: number; width: number; depth: number }> = [];
    for (const road of scene.roads) {
      if (marks.length >= MAX_LANE_MARKS) break;
      if (road.kind === "avenue") continue;
      const horizontal = roadOrientation(road) === "horizontal";
      const length = horizontal ? road.width : road.depth;
      if (length < 24) continue;
      const dashCount = Math.min(60, Math.max(1, Math.floor(length / 15)));
      for (let index = 0; index < dashCount && marks.length < MAX_LANE_MARKS; index += 1) {
        const progress = (index + 0.5) / dashCount;
        if (horizontal) {
          marks.push({
            x: road.x + progress * road.width - 3.4,
            y: road.y + road.depth / 2 - 0.32,
            width: 6.8,
            depth: 0.64,
          });
        } else {
          marks.push({
            x: road.x + road.width / 2 - 0.32,
            y: road.y + progress * road.depth - 3.4,
            width: 0.64,
            depth: 6.8,
          });
        }
      }
    }
    if (marks.length === 0) return;
    const geometry = new BoxGeometry(1, 1, 1);
    const material = new MeshStandardMaterial({
      color: 0xc1c5cb,
      emissive: 0x171a1f,
      emissiveIntensity: 0.12,
      roughness: 0.58,
      metalness: 0,
    });
    const mesh = new InstancedMesh(geometry, material, marks.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    marks.forEach((mark, index) => {
      matrix.compose(
        new Vector3(
          mark.x + mark.width / 2 - scene.width / 2,
          0.56,
          mark.y + mark.depth / 2 - scene.depth / 2,
        ),
        quaternion,
        new Vector3(mark.width, 0.06, mark.depth),
      );
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addTrafficLights(scene: CityScene) {
    const signalRoads = scene.roads.filter((road) =>
      road.kind !== "avenue" && Math.max(road.width, road.depth) >= 34,
    );
    const horizontal = signalRoads.filter((road) => roadOrientation(road) === "horizontal");
    const vertical = signalRoads.filter((road) => roadOrientation(road) === "vertical");
    const intersections: Array<{ x: number; y: number }> = [];
    const seen = new Set<string>();
    for (const hRoad of horizontal) {
      for (const vRoad of vertical) {
        if (intersections.length >= MAX_TRAFFIC_LIGHTS) break;
        if (!roadIntersects(hRoad, vRoad)) continue;
        const right = Math.min(hRoad.x + hRoad.width, vRoad.x + vRoad.width);
        const bottom = Math.min(hRoad.y + hRoad.depth, vRoad.y + vRoad.depth);
        const x = right + SIDEWALK_WIDTH * 0.55;
        const y = bottom + SIDEWALK_WIDTH * 0.55;
        const key = `${Math.round(x / 8)}:${Math.round(y / 8)}`;
        if (seen.has(key)) continue;
        seen.add(key);
        intersections.push({ x, y });
      }
    }
    if (intersections.length === 0) return;

    const poleGeometry = new CylinderGeometry(0.08, 0.1, 3.2, 5);
    const poleMaterial = new MeshStandardMaterial({ color: 0x1f2933, roughness: 0.72, metalness: 0.2 });
    const poles = new InstancedMesh(poleGeometry, poleMaterial, intersections.length);
    const housingGeometry = new BoxGeometry(1, 1, 1);
    const housingMaterial = new MeshStandardMaterial({ color: 0x111827, roughness: 0.66, metalness: 0.12 });
    const housings = new InstancedMesh(housingGeometry, housingMaterial, intersections.length);
    const armGeometry = new BoxGeometry(1, 1, 1);
    const armMaterial = new MeshStandardMaterial({ color: 0x222a33, roughness: 0.7, metalness: 0.18 });
    const arms = new InstancedMesh(armGeometry, armMaterial, intersections.length);
    const bulbGeometry = new SphereGeometry(0.33, 8, 6);
    const bulbMaterial = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      emissive: 0x262626,
      emissiveIntensity: 0.8,
      roughness: 0.3,
    });
    const bulbs = new InstancedMesh(bulbGeometry, bulbMaterial, intersections.length * 3);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    const bulbColors = [new Color("#ef4444"), new Color("#f59e0b"), new Color("#22c55e")];
    intersections.forEach((intersection, index) => {
      const lightX = intersection.x - scene.width / 2;
      const lightZ = intersection.y - scene.depth / 2;
      matrix.compose(
        new Vector3(lightX, 2.25, lightZ),
        quaternion,
        new Vector3(1, 1, 1),
      );
      poles.setMatrixAt(index, matrix);
      matrix.compose(
        new Vector3(lightX + 0.68, 3.08, lightZ + 0.2),
        quaternion,
        new Vector3(0.74, 1.82, 0.34),
      );
      housings.setMatrixAt(index, matrix);
      matrix.compose(
        new Vector3(lightX + 0.24, 3.64, lightZ + 0.2),
        quaternion,
        new Vector3(1.15, 0.12, 0.12),
      );
      arms.setMatrixAt(index, matrix);
      for (let bulb = 0; bulb < 3; bulb += 1) {
        const bulbIndex = index * 3 + bulb;
        matrix.compose(
          new Vector3(lightX + 0.66, 3.56 - bulb * 0.48, lightZ + 0.39),
          quaternion,
          new Vector3(0.86, 0.86, 0.86),
        );
        bulbs.setMatrixAt(bulbIndex, matrix);
        bulbs.setColorAt(bulbIndex, bulbColors[bulb]);
      }
    });
    poles.instanceMatrix.setUsage(StaticDrawUsage);
    housings.instanceMatrix.setUsage(StaticDrawUsage);
    arms.instanceMatrix.setUsage(StaticDrawUsage);
    bulbs.instanceMatrix.setUsage(StaticDrawUsage);
    if (bulbs.instanceColor) bulbs.instanceColor.needsUpdate = true;
    this.threeScene.add(poles, arms, housings, bulbs);
    this.disposableGeometries.add(poleGeometry);
    this.disposableGeometries.add(housingGeometry);
    this.disposableGeometries.add(armGeometry);
    this.disposableGeometries.add(bulbGeometry);
    this.disposableMaterials.add(poleMaterial);
    this.disposableMaterials.add(housingMaterial);
    this.disposableMaterials.add(armMaterial);
    this.disposableMaterials.add(bulbMaterial);
  }

  private addBuildings(scene: CityScene) {
    const geometry = new BoxGeometry(1, 1, 1);
    const material = createBuildingMaterial();
    const mesh = new InstancedMesh(geometry, material, scene.buildings.length);
    const matrix = new Matrix4();
    const position = new Vector3();
    const scale = new Vector3();
    const quaternion = new Quaternion();
    this.buildingByIndex.length = 0;

    scene.buildings.forEach((building, index) => {
      position.set(
        building.x + building.width / 2 - scene.width / 2,
        building.height / 2,
        building.y + building.depth / 2 - scene.depth / 2,
      );
      scale.set(building.width, building.height, building.depth);
      matrix.compose(position, quaternion, scale);
      mesh.setMatrixAt(index, matrix);
      const baseColor = heatColor(building.file.heat, this.heatVisible);
      mesh.setColorAt(index, baseColor);
      this.buildingByIndex.push(building);
      this.buildingByPath.set(building.file.path, { building, index, baseColor });
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    mesh.computeBoundingSphere();
    this.buildingMesh = mesh;
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addTrees(scene: CityScene) {
    const candidates: Array<{ x: number; z: number; scale: number; kind: "pine" | "round" }> = [];
    const addTreeCandidate = (x: number, z: number, seed: number, scaleBoost = 0) => {
      if (candidates.length >= MAX_TREES) return;
      if (pointNearRoad(x, z, scene.roads, SIDEWALK_WIDTH + 0.7)) return;
      candidates.push({
        x: x - scene.width / 2,
        z: z - scene.depth / 2,
        scale: 0.68 + ((seed >>> 24) % 55) / 100 + scaleBoost,
        kind: seed % 3 === 0 ? "round" : "pine",
      });
    };
    for (const park of scene.parks) {
      const lake = parkLakeFor(park);
      const count = clamp(Math.ceil(park.treeCount * 1.75), 5, 52);
      for (let index = 0; index < count && candidates.length < MAX_TREES; index += 1) {
        const seed = hashText(`${park.id}:tree:${index}`);
        const x = park.x + 1.6 + ((seed >>> 4) % 1000) / 1000 * Math.max(1, park.width - 3.2);
        const z = park.y + 1.6 + ((seed >>> 14) % 1000) / 1000 * Math.max(1, park.depth - 3.2);
        if (pointInLake(x, z, lake, 2.2)) continue;
        addTreeCandidate(x, z, seed);
      }
      if (lake) {
        const rimCount = clamp(Math.floor((lake.radiusX + lake.radiusY) / 2.6), 6, 16);
        for (let index = 0; index < rimCount && candidates.length < MAX_TREES; index += 1) {
          const seed = hashText(`${park.id}:lake-rim:${index}`);
          const angle = (index / rimCount) * Math.PI * 2 + ((seed >>> 6) % 40) / 100;
          const x = lake.x + Math.cos(angle) * (lake.radiusX + 2.8 + (seed % 18) / 10);
          const z = lake.y + Math.sin(angle) * (lake.radiusY + 2.2 + ((seed >>> 10) % 14) / 10);
          if (x < park.x + 1 || x > park.x + park.width - 1 || z < park.y + 1 || z > park.y + park.depth - 1) continue;
          addTreeCandidate(x, z, seed, 0.08);
        }
      }
    }
    for (const district of scene.districts) {
      const count = clamp(Math.ceil(Math.sqrt(district.fileCount)), 2, 9);
      for (let index = 0; index < count && candidates.length < MAX_TREES; index += 1) {
        const seed = hashText(`${district.name}:${index}`);
        const edge = seed % 4;
        const progress = ((seed >>> 4) % 1000) / 1000;
        const x = edge < 2
          ? district.x + progress * district.width
          : district.x + (edge === 2 ? 3 : district.width - 3);
        const z = edge >= 2
          ? district.y + progress * district.depth
          : district.y + (edge === 0 ? 3 : district.depth - 3);
        if (pointNearRoad(x, z, scene.roads, 0.6)) continue;
        candidates.push({
          x: x - scene.width / 2,
          z: z - scene.depth / 2,
          scale: 0.72 + ((seed >>> 12) % 40) / 100,
          kind: seed % 4 === 0 ? "round" : "pine",
        });
      }
    }
    if (candidates.length === 0) return;

    const trunkGeometry = new CylinderGeometry(0.5, 0.65, 4, 5);
    const pineGeometry = new ConeGeometry(2.6, 7, 7);
    const roundGeometry = new SphereGeometry(2.8, 8, 6);
    const trunkMaterial = new MeshStandardMaterial({ color: 0x6b4936, roughness: 1 });
    const pineMaterial = new MeshStandardMaterial({ color: 0x2f8f69, roughness: 0.94 });
    const roundMaterial = new MeshStandardMaterial({ color: 0x45a66d, roughness: 0.94 });
    const trunks = new InstancedMesh(trunkGeometry, trunkMaterial, candidates.length);
    const pines = new InstancedMesh(pineGeometry, pineMaterial, candidates.length);
    const rounds = new InstancedMesh(roundGeometry, roundMaterial, candidates.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    let pineCount = 0;
    let roundCount = 0;
    candidates.forEach((tree, index) => {
      matrix.compose(
        new Vector3(tree.x, 2, tree.z),
        quaternion,
        new Vector3(tree.scale, tree.scale, tree.scale),
      );
      trunks.setMatrixAt(index, matrix);
      if (tree.kind === "round") {
        matrix.compose(
          new Vector3(tree.x, 5.35 * tree.scale, tree.z),
          quaternion,
          new Vector3(tree.scale, tree.scale * 0.78, tree.scale),
        );
        rounds.setMatrixAt(roundCount, matrix);
        roundCount += 1;
      } else {
        matrix.compose(
          new Vector3(tree.x, 6.2 * tree.scale, tree.z),
          quaternion,
          new Vector3(tree.scale, tree.scale, tree.scale),
        );
        pines.setMatrixAt(pineCount, matrix);
        pineCount += 1;
      }
    });
    trunks.instanceMatrix.setUsage(StaticDrawUsage);
    pines.instanceMatrix.setUsage(StaticDrawUsage);
    rounds.instanceMatrix.setUsage(StaticDrawUsage);
    pines.count = pineCount;
    rounds.count = roundCount;
    this.threeScene.add(trunks, pines, rounds);
    this.disposableGeometries.add(trunkGeometry);
    this.disposableGeometries.add(pineGeometry);
    this.disposableGeometries.add(roundGeometry);
    this.disposableMaterials.add(trunkMaterial);
    this.disposableMaterials.add(pineMaterial);
    this.disposableMaterials.add(roundMaterial);
  }

  private addRoofDetails(scene: CityScene) {
    const roofCandidates = scene.buildings
      .filter((building) => building.width >= 5 && building.depth >= 5 && hashText(building.file.path) % 5 !== 0)
      .sort((a, b) => b.file.heat - a.file.heat)
      .slice(0, MAX_ROOF_DETAILS);
    if (roofCandidates.length === 0) return;

    const boxGeometry = new BoxGeometry(1, 1, 1);
    const boxMaterial = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      roughness: 0.64,
      metalness: 0.12,
    });
    const boxes = new InstancedMesh(boxGeometry, boxMaterial, roofCandidates.length);
    const chimneyGeometry = new CylinderGeometry(0.35, 0.42, 2.2, 6);
    const chimneyMaterial = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      roughness: 0.78,
      metalness: 0.02,
    });
    const chimneys = new InstancedMesh(chimneyGeometry, chimneyMaterial, Math.ceil(roofCandidates.length / 2));
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    let chimneyCount = 0;

    roofCandidates.forEach((building, index) => {
      const seed = hashText(`${building.file.path}:roof`);
      const centerX = building.x + building.width / 2 - scene.width / 2;
      const centerZ = building.y + building.depth / 2 - scene.depth / 2;
      const offsetX = (((seed >>> 7) % 1000) / 1000 - 0.5) * building.width * 0.35;
      const offsetZ = (((seed >>> 17) % 1000) / 1000 - 0.5) * building.depth * 0.35;
      matrix.compose(
        new Vector3(centerX + offsetX, building.height + 0.38, centerZ + offsetZ),
        quaternion,
        new Vector3(
          Math.max(1.3, building.width * (0.18 + (seed % 15) / 100)),
          0.76,
          Math.max(1.1, building.depth * (0.16 + ((seed >>> 4) % 14) / 100)),
        ),
      );
      boxes.setMatrixAt(index, matrix);
      boxes.setColorAt(index, new Color("#1f2937").lerp(heatColor(building.file.heat, this.heatVisible), 0.32));

      if (seed % 2 === 0 && chimneyCount < chimneys.count) {
        matrix.compose(
          new Vector3(
            centerX - building.width * 0.24,
            building.height + 1.08,
            centerZ + building.depth * 0.22,
          ),
          quaternion,
          new Vector3(1, 1, 1),
        );
        chimneys.setMatrixAt(chimneyCount, matrix);
        chimneys.setColorAt(chimneyCount, new Color("#2b2325").lerp(heatColor(building.file.heat, this.heatVisible), 0.22));
        chimneyCount += 1;
      }
    });
    boxes.instanceMatrix.setUsage(StaticDrawUsage);
    chimneys.instanceMatrix.setUsage(StaticDrawUsage);
    if (boxes.instanceColor) boxes.instanceColor.needsUpdate = true;
    if (chimneys.instanceColor) chimneys.instanceColor.needsUpdate = true;
    chimneys.count = chimneyCount;
    this.threeScene.add(boxes, chimneys);
    this.disposableGeometries.add(boxGeometry);
    this.disposableGeometries.add(chimneyGeometry);
    this.disposableMaterials.add(boxMaterial);
    this.disposableMaterials.add(chimneyMaterial);
  }

  private addRoofMarkers(scene: CityScene) {
    const marked = scene.buildings
      .filter((building) => building.file.heat >= 0.72)
      .sort((a, b) => b.file.heat - a.file.heat)
      .slice(0, MAX_ROOF_MARKERS);
    if (marked.length === 0) return;
    const geometry = new ConeGeometry(1.4, 4.2, 6);
    const material = new MeshStandardMaterial({
      color: 0xfb7185,
      emissive: 0x7f1d36,
      emissiveIntensity: 0.75,
      roughness: 0.45,
    });
    const mesh = new InstancedMesh(geometry, material, marked.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    marked.forEach((building, index) => {
      matrix.compose(
        new Vector3(
          building.x + building.width / 2 - scene.width / 2,
          building.height + 2.1,
          building.y + building.depth / 2 - scene.depth / 2,
        ),
        quaternion,
        new Vector3(1, 1, 1),
      );
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  setRadarMarkers(markers: CityRadarMarker[]) {
    if (this.radarMesh) {
      this.threeScene.remove(this.radarMesh);
      this.radarMesh.geometry.dispose();
      disposeMaterial(this.radarMesh.material);
      this.disposableGeometries.delete(this.radarMesh.geometry);
      const oldMaterials = Array.isArray(this.radarMesh.material) ? this.radarMesh.material : [this.radarMesh.material];
      oldMaterials.forEach((material) => this.disposableMaterials.delete(material));
      this.radarMesh.dispose();
      this.radarMesh = null;
    }
    if (!this.sceneData || markers.length === 0) {
      this.render();
      return;
    }

    const visible = markers.filter((marker) => this.buildingByPath.has(marker.path));
    if (visible.length === 0) {
      this.render();
      return;
    }
    const geometry = new SphereGeometry(1, 8, 6);
    const material = new MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
      emissive: 0x10263a,
      emissiveIntensity: 0.5,
      roughness: 0.32,
    });
    const mesh = new InstancedMesh(geometry, material, visible.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    visible.forEach((marker, index) => {
      const building = this.buildingByPath.get(marker.path)!.building;
      const scale = marker.source === "local" ? 2.7 : 1.9;
      matrix.compose(
        new Vector3(
          building.x + building.width / 2 - this.sceneData!.width / 2,
          building.height + scale + 4.5,
          building.y + building.depth / 2 - this.sceneData!.depth / 2,
        ),
        quaternion,
        new Vector3(scale, scale, scale),
      );
      mesh.setMatrixAt(index, matrix);
      mesh.setColorAt(index, markerColor(marker.color));
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    this.radarMesh = mesh;
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
    this.threeScene.add(mesh);
    this.render();
  }

  setHeatVisible(visible: boolean) {
    if (this.heatVisible === visible && this.buildingMesh) return;
    this.heatVisible = visible;
    if (!this.buildingMesh) return;
    this.buildingByPath.forEach((visual) => {
      visual.baseColor = heatColor(visual.building.file.heat, visible);
      this.buildingMesh!.setColorAt(visual.index, visual.baseColor);
    });
    if (this.buildingMesh.instanceColor) this.buildingMesh.instanceColor.needsUpdate = true;
    this.render();
  }

  setWalkSpeed(speed: number) {
    this.walkSpeed = clamp(speed, 0.25, 1.6);
  }

  setHoveredPath(path: string) {
    if (this.hoveredPath === path || !this.buildingMesh) return;
    const previous = this.buildingByPath.get(this.hoveredPath);
    if (previous) this.buildingMesh.setColorAt(previous.index, previous.baseColor);
    this.hoveredPath = path;
    const next = this.buildingByPath.get(path);
    if (next) this.buildingMesh.setColorAt(next.index, new Color("#e0f2fe"));
    if (this.buildingMesh.instanceColor) this.buildingMesh.instanceColor.needsUpdate = true;
    this.render();
  }

  resize(width: number, height: number) {
    this.width = Math.max(1, Math.floor(width));
    this.height = Math.max(1, Math.floor(height));
    this.renderer.setSize(this.width, this.height, false);
    this.camera.aspect = this.width / this.height;
    this.camera.updateProjectionMatrix();
    this.render();
  }

  fit(mode: CityCameraMode) {
    if (!this.sceneData) return;
    this.mode = mode;
    if (mode === "boat") {
      if (this.activeBoat) this.updateBoatCamera();
      else this.fit("bird");
      return;
    }
    this.activeBoat = null;
    this.boatAiming = false;
    this.updateAimReticle();
    if (mode === "walking") {
      this.walkPosition.copy(this.nearestRoadPoint(this.target.x, this.target.z));
      this.updateWalkingCamera();
      return;
    }
    const maxDimension = Math.max(this.sceneData.width, this.sceneData.depth);
    this.target.set(0, 0, -maxDimension * 0.035);
    this.yaw = -0.72;
    this.pitch = mode === "bird" ? 1.34 : 0.68;
    const fitDistance = maxDimension / (2 * Math.tan(MathUtils.degToRad(this.camera.fov / 2)));
    this.distance = Math.max(90, fitDistance * (mode === "bird" ? 1.04 : 1.08));
    this.updateCamera();
  }

  setMode(mode: CityCameraMode) {
    this.mode = mode;
    if (mode !== "boat") {
      this.activeBoat = null;
      this.boatAiming = false;
      this.updateAimReticle();
    }
    if (mode === "boat") {
      if (this.activeBoat) this.updateBoatCamera();
      return;
    }
    if (mode === "walking") {
      this.walkPosition.copy(this.nearestRoadPoint(this.target.x, this.target.z));
      this.updateWalkingCamera();
      return;
    }
    this.fit(mode);
  }

  orbit(deltaX: number, deltaY: number) {
    if (this.activeBoat) {
      this.boatCameraYawOffset -= deltaX * 0.006;
      this.boatCameraPitch = clamp(this.boatCameraPitch + deltaY * 0.003, 0.12, 0.72);
      this.updateBoatCamera();
      return;
    }
    if (this.mode === "walking") {
      this.walkYaw -= deltaX * 0.006;
      this.walkPitch = clamp(this.walkPitch + deltaY * 0.003, -0.42, 0.28);
      this.updateWalkingCamera();
      return;
    }
    this.yaw -= deltaX * 0.006;
    this.pitch = clamp(this.pitch - deltaY * 0.005, 0.32, 1.48);
    this.updateCamera();
  }

  pan(deltaX: number, deltaY: number) {
    if (this.activeBoat) {
      this.orbit(deltaX, deltaY);
      return;
    }
    if (this.mode === "walking") {
      this.walkBy(-deltaY * 0.018, -deltaX * 0.018);
      return;
    }
    const scale = this.distance * 0.00125;
    const right = new Vector3().setFromMatrixColumn(this.camera.matrix, 0).setY(0).normalize();
    const forward = new Vector3();
    this.camera.getWorldDirection(forward);
    forward.setY(0).normalize();
    this.target.addScaledVector(right, -deltaX * scale);
    this.target.addScaledVector(forward, deltaY * scale);
    this.updateCamera();
  }

  zoom(factor: number) {
    this.zoomAt(factor);
  }

  zoomAt(factor: number, anchor?: { x: number; y: number }) {
    if (this.mode === "walking" || this.activeBoat) return;
    const before = anchor ? this.groundPointFromScreen(anchor.x, anchor.y) : null;
    this.distance = clamp(this.distance * factor, 24, Math.max(5000, (this.sceneData?.width ?? 500) * 6));
    this.updateCamera();
    if (!before || !anchor) return;
    const after = this.groundPointFromScreen(anchor.x, anchor.y);
    if (!after) return;
    this.target.add(before.sub(after));
    this.updateCamera();
  }

  moveByKeyboard(forward: number, right: number, fast = false, deltaSeconds = 1 / 60) {
    if (forward === 0 && right === 0) return;
    const frameScale = clamp(deltaSeconds * 60, 0.35, 2.4);
    if (this.activeBoat) {
      this.pilotBoat(forward, right, fast, frameScale);
      return;
    }
    if (this.mode === "walking") {
      this.walkBy(forward * (fast ? 1.65 : 1) * frameScale, right * (fast ? 1.65 : 1) * frameScale);
      return;
    }
    const amount = this.distance * (fast ? 0.052 : 0.028) * frameScale;
    const forwardVector = new Vector3();
    this.camera.getWorldDirection(forwardVector);
    forwardVector.setY(0).normalize();
    const rightVector = new Vector3().setFromMatrixColumn(this.camera.matrix, 0).setY(0).normalize();
    this.target.addScaledVector(forwardVector, forward * amount);
    this.target.addScaledVector(rightVector, right * amount);
    this.updateCamera();
  }

  focus(path: string) {
    const visual = this.buildingByPath.get(path);
    if (!visual || !this.sceneData) return;
    this.activeBoat = null;
    this.boatAiming = false;
    this.updateAimReticle();
    const building = visual.building;
    const targetX = building.x + building.width / 2 - this.sceneData.width / 2;
    const targetZ = building.y + building.depth / 2 - this.sceneData.depth / 2;
    if (this.mode === "walking") {
      this.walkPosition.copy(this.nearestRoadPoint(targetX, targetZ));
      this.walkYaw = Math.atan2(targetX - this.walkPosition.x, targetZ - this.walkPosition.z);
      this.updateWalkingCamera();
      return;
    }
    this.target.set(
      targetX,
      Math.min(18, building.height * 0.3),
      targetZ,
    );
    this.distance = clamp(Math.min(this.distance, Math.max(55, building.height * 4.5)), 34, 280);
    this.updateCamera();
  }

  pick(clientX: number, clientY: number): CityBuilding | null {
    if (!this.buildingMesh || this.width <= 0 || this.height <= 0) return null;
    this.pointer.set((clientX / this.width) * 2 - 1, -(clientY / this.height) * 2 + 1);
    this.raycaster.setFromCamera(this.pointer, this.camera);
    const intersection = this.raycaster.intersectObject(this.buildingMesh, false)[0];
    if (!intersection || intersection.instanceId === undefined) return null;
    return this.buildingByIndex[intersection.instanceId] ?? null;
  }

  pickDistrict(clientX: number, clientY: number): CityDistrict | null {
    if (!this.districtMesh || this.width <= 0 || this.height <= 0) return null;
    this.pointer.set((clientX / this.width) * 2 - 1, -(clientY / this.height) * 2 + 1);
    this.raycaster.setFromCamera(this.pointer, this.camera);
    const intersection = this.raycaster.intersectObject(this.districtMesh, false)[0];
    if (!intersection || intersection.instanceId === undefined) return null;
    return this.districtByIndex[intersection.instanceId] ?? null;
  }

  pickBoat(clientX: number, clientY: number): CityBoatInfo | null {
    if (this.boatPickTargets.length === 0 || this.width <= 0 || this.height <= 0) return null;
    this.pointer.set((clientX / this.width) * 2 - 1, -(clientY / this.height) * 2 + 1);
    this.raycaster.setFromCamera(this.pointer, this.camera);
    const intersection = this.raycaster.intersectObjects(this.boatPickTargets, false)[0];
    if (!intersection?.object) return null;
    const boat = this.boatByMesh.get(intersection.object as Mesh);
    return boat ? { id: boat.id, name: boat.name, kind: boat.kind } : null;
  }

  enterBoat(id: string): CityBoatInfo | null {
    const boat = this.boatById.get(id);
    if (!boat) return null;
    this.activeBoat = boat;
    this.mode = "boat";
    this.boatCameraYawOffset = Math.PI;
    this.boatCameraPitch = 0.34;
    this.boatCannonSide = 1;
    this.boatAiming = false;
    this.updateBoatCamera();
    return { id: boat.id, name: boat.name, kind: boat.kind };
  }

  setBoatCannonSide(side: "left" | "right") {
    if (!this.activeBoat) return;
    this.boatCannonSide = side === "right" ? 1 : -1;
    this.boatAiming = true;
    this.updateBoatCamera();
  }

  setBoatAiming(aiming: boolean) {
    this.boatAiming = aiming && !!this.activeBoat;
    if (this.activeBoat) this.updateBoatCamera();
    else {
      this.updateAimReticle();
      this.render();
    }
  }

  adjustBoatAim(deltaY: number) {
    if (!this.activeBoat) return;
    this.boatAiming = true;
    this.boatAimPitch = clamp(this.boatAimPitch - deltaY * 0.0045, 0.04, 0.68);
    this.updateBoatCamera();
  }

  fireBoatCannon(): boolean {
    if (!this.activeBoat) return false;
    const slot = this.cannonBalls.find((ball) => !ball.active);
    if (!slot) return false;
    const boat = this.activeBoat;
    const rightX = Math.cos(boat.yaw);
    const rightZ = -Math.sin(boat.yaw);
    const forwardX = Math.sin(boat.yaw);
    const forwardZ = Math.cos(boat.yaw);
    const side = this.boatCannonSide;
    const pitch = this.boatAimPitch;
    const speed = 58;
    const horizontal = Math.cos(pitch) * speed;
    slot.active = true;
    slot.x = boat.group.position.x + rightX * side * 4.65 + forwardX * 1.1;
    slot.y = boat.group.position.y + 2.2;
    slot.z = boat.group.position.z + rightZ * side * 4.65 + forwardZ * 1.1;
    slot.vx = rightX * side * horizontal + forwardX * 5.5;
    slot.vy = Math.sin(pitch) * speed + 2.2;
    slot.vz = rightZ * side * horizontal + forwardZ * 5.5;
    slot.age = 0;
    this.startCannonAnimation();
    return true;
  }

  private pilotBoat(forward: number, right: number, fast = false, frameScale = 1) {
    if (!this.sceneData || !this.activeBoat) return;
    const boat = this.activeBoat;
    if (right !== 0) {
      boat.yaw -= right * BOAT_TURN_STEP * (fast ? 1.35 : 1) * frameScale;
      boat.group.rotation.y = boat.yaw;
    }
    if (forward !== 0) {
      const amount = forward * BOAT_STEP * (fast ? 1.65 : 1) * frameScale;
      this.boatMove.set(Math.sin(boat.yaw) * amount, 0, Math.cos(boat.yaw) * amount);
      const nextX = boat.group.position.x + this.boatMove.x;
      const nextZ = boat.group.position.z + this.boatMove.z;
      if (this.isBoatOnWater(nextX, nextZ)) {
        boat.group.position.x = nextX;
        boat.group.position.z = nextZ;
      }
    }
    this.updateBoatCamera();
  }

  private updateAimReticle() {
    if (!this.aimReticle) return;
    if (!this.activeBoat || !this.boatAiming) {
      this.aimReticle.visible = false;
      return;
    }
    const boat = this.activeBoat;
    const rightX = Math.cos(boat.yaw);
    const rightZ = -Math.sin(boat.yaw);
    const forwardX = Math.sin(boat.yaw);
    const forwardZ = Math.cos(boat.yaw);
    const side = this.boatCannonSide;
    const distance = 48;
    this.aimReticle.visible = true;
    this.aimReticle.position.set(
      boat.group.position.x + rightX * side * distance + forwardX * 7,
      boat.group.position.y + 2.2 + Math.tan(this.boatAimPitch) * distance,
      boat.group.position.z + rightZ * side * distance + forwardZ * 7,
    );
    this.aimReticle.lookAt(this.camera.position);
  }

  private tryCannonHitBuilding(ball: CannonBall, previousX: number, previousY: number, previousZ: number): boolean {
    if (!this.sceneData || Math.max(previousY, ball.y) < -0.8 || Math.min(previousY, ball.y) > 142) return false;
    let nearest: { visual: BuildingVisual; time: number } | null = null;
    for (const visual of this.buildingByPath.values()) {
      const building = visual.building;
      if (this.collapsedBuildingPaths.has(building.file.path)) continue;
      const padding = 0.78;
      const hitTime = segmentBoxHit(
        previousX,
        previousY,
        previousZ,
        ball.x,
        ball.y,
        ball.z,
        building.x - this.sceneData.width / 2 - padding,
        -0.25,
        building.y - this.sceneData.depth / 2 - padding,
        building.x + building.width - this.sceneData.width / 2 + padding,
        building.height + padding,
        building.y + building.depth - this.sceneData.depth / 2 + padding,
      );
      if (hitTime !== null && (!nearest || hitTime < nearest.time)) nearest = { visual, time: hitTime };
    }
    if (!nearest) return false;
    ball.x = previousX + (ball.x - previousX) * nearest.time;
    ball.y = previousY + (ball.y - previousY) * nearest.time;
    ball.z = previousZ + (ball.z - previousZ) * nearest.time;
    this.collapseBuilding(nearest.visual, ball);
    return true;
  }

  private collapseBuilding(visual: BuildingVisual, impact: CannonBall) {
    if (!this.sceneData || !this.buildingMesh || !this.debrisMesh) return;
    const building = visual.building;
    this.collapsedBuildingPaths.add(building.file.path);
    this.debrisMatrix.compose(
      new Vector3(0, -140, 0),
      this.debrisQuaternion,
      new Vector3(0.001, 0.001, 0.001),
    );
    this.buildingMesh.setMatrixAt(visual.index, this.debrisMatrix);
    this.buildingMesh.instanceMatrix.needsUpdate = true;

    const baseColor = new Color("#111827").lerp(heatColor(building.file.heat, this.heatVisible), 0.72);
    const count = clamp(Math.ceil(building.height * 0.46 + (building.width * building.depth) / 10), 18, 58);
    const centerX = building.x + building.width / 2 - this.sceneData.width / 2;
    const centerZ = building.y + building.depth / 2 - this.sceneData.depth / 2;
    for (let index = 0; index < count; index += 1) {
      const slotIndex = this.debrisBlocks.findIndex((block) => !block.active);
      const block = slotIndex >= 0
        ? this.debrisBlocks[slotIndex]
        : this.debrisBlocks.reduce((oldest, item) => (item.age > oldest.age ? item : oldest), this.debrisBlocks[0]);
      const realIndex = slotIndex >= 0 ? slotIndex : this.debrisBlocks.indexOf(block);
      const seed = hashText(`${building.file.path}:debris:${index}:${Math.round(impact.x * 10)}`);
      const localX = (((seed >>> 5) % 1000) / 1000 - 0.5) * building.width;
      const localY = ((seed >>> 15) % 1000) / 1000 * Math.max(3, building.height);
      const localZ = (((seed >>> 25) % 1000) / 1000 - 0.5) * building.depth;
      const x = centerX + localX;
      const y = Math.max(0.8, localY);
      const z = centerZ + localZ;
      const awayX = x - impact.x;
      const awayZ = z - impact.z;
      const awayLength = Math.max(0.4, Math.hypot(awayX, awayZ));
      const blast = 5 + ((seed >>> 9) % 120) / 10;
      block.active = true;
      block.sleeping = false;
      block.x = x;
      block.y = y;
      block.z = z;
      block.vx = (awayX / awayLength) * blast * 0.48 + impact.vx * 0.16;
      block.vy = 5 + ((seed >>> 18) % 105) / 10 + Math.max(0, impact.vy) * 0.08;
      block.vz = (awayZ / awayLength) * blast * 0.48 + impact.vz * 0.16;
      block.rx = ((seed >>> 2) % 628) / 100;
      block.ry = ((seed >>> 8) % 628) / 100;
      block.rz = ((seed >>> 14) % 628) / 100;
      block.vrx = (((seed >>> 4) % 100) / 100 - 0.5) * 3.2;
      block.vry = (((seed >>> 12) % 100) / 100 - 0.5) * 4.4;
      block.vrz = (((seed >>> 20) % 100) / 100 - 0.5) * 3.2;
      block.sx = clamp(building.width * (0.1 + ((seed >>> 7) % 13) / 100), 0.7, 2.6);
      block.sy = clamp(building.height * (0.035 + ((seed >>> 13) % 9) / 100), 0.7, 3.8);
      block.sz = clamp(building.depth * (0.1 + ((seed >>> 19) % 12) / 100), 0.7, 2.6);
      block.age = 0;
      this.debrisMesh.setColorAt(realIndex, new Color(baseColor).lerp(new Color("#0f172a"), (seed % 35) / 100));
    }
    if (this.debrisMesh.instanceColor) this.debrisMesh.instanceColor.needsUpdate = true;
    this.spawnImpactBurst(impact.x, impact.y, impact.z, "building");
    this.startCannonAnimation();
  }

  private updateDebris(dt: number): boolean {
    if (!this.debrisMesh) return false;
    let hasActive = false;
    for (let index = 0; index < this.debrisBlocks.length; index += 1) {
      const block = this.debrisBlocks[index];
      if (block.active && !block.sleeping) {
        const drag = Math.exp(-0.62 * dt);
        block.vy -= CANNON_GRAVITY * 1.15 * dt;
        block.vx *= drag;
        block.vz *= drag;
        block.x += block.vx * dt;
        block.y += block.vy * dt;
        block.z += block.vz * dt;
        block.rx += block.vrx * dt;
        block.ry += block.vry * dt;
        block.rz += block.vrz * dt;
        block.age += dt;
        if (block.y <= block.sy / 2) {
          block.y = block.sy / 2;
          block.vx *= 0.82;
          block.vz *= 0.82;
          block.vrx *= 0.74;
          block.vry *= 0.78;
          block.vrz *= 0.74;
          if (Math.abs(block.vy) > 1.1) block.vy = Math.abs(block.vy) * 0.24;
          else block.vy = 0;
        }
        if (block.age > 9.5 || (block.age > 2.25 && Math.hypot(block.vx, block.vy, block.vz) < 0.32)) {
          block.sleeping = true;
          block.vx = 0;
          block.vy = 0;
          block.vz = 0;
          block.vrx = 0;
          block.vry = 0;
          block.vrz = 0;
        }
        if (!this.isCannonInWorld(block.x, block.z) || block.y < -8) {
          block.active = false;
          block.sleeping = false;
        }
      }

      if (block.active) {
        this.debrisPosition.set(block.x, block.y, block.z);
        this.debrisEuler.set(block.rx, block.ry, block.rz);
        this.debrisQuaternion.setFromEuler(this.debrisEuler);
        this.debrisScale.set(block.sx, block.sy, block.sz);
        hasActive = hasActive || !block.sleeping;
      } else {
        this.debrisPosition.set(0, -120, 0);
        this.debrisQuaternion.identity();
        this.debrisScale.set(0.001, 0.001, 0.001);
      }
      this.debrisMatrix.compose(this.debrisPosition, this.debrisQuaternion, this.debrisScale);
      this.debrisMesh.setMatrixAt(index, this.debrisMatrix);
    }
    this.debrisMesh.instanceMatrix.needsUpdate = true;
    return hasActive;
  }

  private spawnImpactBurst(x: number, y: number, z: number, kind: "water" | "ground" | "building") {
    if (!this.impactMesh) return;
    const count = kind === "building" ? 18 : kind === "water" ? 14 : 10;
    const baseSeed = hashText(`${kind}:${Math.round(x * 10)}:${Math.round(y * 10)}:${Math.round(z * 10)}:${performance.now() | 0}`);
    let colorsChanged = false;
    for (let index = 0; index < count; index += 1) {
      let slotIndex = this.impactParticles.findIndex((particle) => !particle.active);
      if (slotIndex < 0) {
        slotIndex = this.impactParticles.reduce(
          (oldestIndex, particle, candidateIndex, particles) => particle.age > particles[oldestIndex].age ? candidateIndex : oldestIndex,
          0,
        );
      }
      const particle = this.impactParticles[slotIndex];
      const seed = hashText(`${baseSeed}:${index}`);
      const angle = (index / count) * Math.PI * 2 + ((seed >>> 9) % 42) / 100;
      const outward = kind === "water" ? 2.8 + (seed % 58) / 10 : 2 + (seed % 76) / 10;
      particle.active = true;
      particle.x = x + Math.cos(angle) * 0.4;
      particle.y = Math.max(kind === "water" ? -1.35 : 0.15, y);
      particle.z = z + Math.sin(angle) * 0.4;
      particle.vx = Math.cos(angle) * outward;
      particle.vy = kind === "water" ? 7 + ((seed >>> 17) % 85) / 10 : 4 + ((seed >>> 17) % 72) / 10;
      particle.vz = Math.sin(angle) * outward;
      particle.scale = kind === "building" ? 0.34 + ((seed >>> 25) % 52) / 100 : 0.2 + ((seed >>> 25) % 36) / 100;
      particle.gravity = kind === "water" ? 15.5 : 20;
      particle.age = 0;
      particle.ttl = kind === "building" ? 1.6 + (seed % 50) / 100 : 0.85 + (seed % 54) / 100;
      const color = kind === "water"
        ? new Color(index % 4 === 0 ? "#e0f7ff" : "#69c7e5")
        : kind === "building"
          ? new Color(index % 3 === 0 ? "#94a3b8" : "#475569")
          : new Color(index % 3 === 0 ? "#7b6751" : "#536057");
      this.impactMesh.setColorAt(slotIndex, color);
      colorsChanged = true;
    }
    if (colorsChanged && this.impactMesh.instanceColor) this.impactMesh.instanceColor.needsUpdate = true;
  }

  private updateImpactParticles(dt: number): boolean {
    if (!this.impactMesh) return false;
    let moving = false;
    for (let index = 0; index < this.impactParticles.length; index += 1) {
      const particle = this.impactParticles[index];
      if (particle.active) {
        particle.vy -= particle.gravity * dt;
        particle.x += particle.vx * dt;
        particle.y += particle.vy * dt;
        particle.z += particle.vz * dt;
        particle.age += dt;
        const life = 1 - particle.age / particle.ttl;
        if (life <= 0 || particle.y < -3) particle.active = false;
        else {
          const scale = particle.scale * clamp(life * 1.35, 0.12, 1);
          this.impactPosition.set(particle.x, particle.y, particle.z);
          this.impactScale.set(scale, scale * (1 + Math.max(0, particle.vy) * 0.045), scale);
          moving = true;
        }
      }
      if (!particle.active) {
        this.impactPosition.set(0, -120, 0);
        this.impactScale.set(0.001, 0.001, 0.001);
      }
      this.impactMatrix.compose(this.impactPosition, this.cannonQuaternion, this.impactScale);
      this.impactMesh.setMatrixAt(index, this.impactMatrix);
    }
    this.impactMesh.instanceMatrix.needsUpdate = true;
    return moving;
  }

  private startCannonAnimation() {
    if (this.cannonAnimationFrame !== null) return;
    this.lastCannonAnimationAt = performance.now();
    const animate = (time: number) => {
      if (this.disposed || !this.cannonBallMesh) {
        this.cannonAnimationFrame = null;
        return;
      }
      const dt = clamp((time - this.lastCannonAnimationAt) / 1000, 0.001, 0.033);
      this.lastCannonAnimationAt = time;
      let hasActive = false;
      for (let index = 0; index < this.cannonBalls.length; index += 1) {
        const ball = this.cannonBalls[index];
        if (ball.active) {
          const previousX = ball.x;
          const previousY = ball.y;
          const previousZ = ball.z;
          ball.vy -= CANNON_GRAVITY * dt;
          const drag = Math.exp(-0.018 * dt);
          ball.vx *= drag;
          ball.vy *= drag;
          ball.vz *= drag;
          ball.x += ball.vx * dt;
          ball.y += ball.vy * dt;
          ball.z += ball.vz * dt;
          ball.age += dt;
          if (this.tryCannonHitBuilding(ball, previousX, previousY, previousZ)) {
            ball.active = false;
          } else {
            const overLand = this.isPointOverLand(ball.x, ball.z);
            const surfaceY = overLand ? 0 : -1.5;
            if (previousY > surfaceY && ball.y <= surfaceY) {
              ball.y = surfaceY;
              this.spawnImpactBurst(ball.x, ball.y, ball.z, overLand ? "ground" : "water");
              ball.active = false;
            }
          }
          if (ball.active && (ball.y < -3 || ball.age > 6.8 || !this.isCannonInWorld(ball.x, ball.z))) {
            ball.active = false;
          }
        }
        this.cannonPosition.set(ball.active ? ball.x : 0, ball.active ? ball.y : -100, ball.active ? ball.z : 0);
        const scale = ball.active ? 1 : 0.001;
        this.cannonScale.set(scale, scale, scale);
        this.cannonMatrix.compose(this.cannonPosition, this.cannonQuaternion, this.cannonScale);
        this.cannonBallMesh.setMatrixAt(index, this.cannonMatrix);
        hasActive = hasActive || ball.active;
      }
      this.cannonBallMesh.instanceMatrix.needsUpdate = true;
      hasActive = this.updateDebris(dt) || hasActive;
      hasActive = this.updateImpactParticles(dt) || hasActive;
      this.render();
      if (hasActive) this.cannonAnimationFrame = requestAnimationFrame(animate);
      else this.cannonAnimationFrame = null;
    };
    this.cannonAnimationFrame = requestAnimationFrame(animate);
  }

  private isCannonInWorld(centeredX: number, centeredZ: number): boolean {
    if (!this.sceneData) return false;
    const oceanMargin = Math.max(720, Math.max(this.sceneData.width, this.sceneData.depth) * 0.92);
    return Math.abs(centeredX) < this.sceneData.width / 2 + oceanMargin - 36
      && Math.abs(centeredZ) < this.sceneData.depth / 2 + oceanMargin - 36;
  }

  private isPointOverLand(centeredX: number, centeredZ: number): boolean {
    if (!this.sceneData) return false;
    return Math.abs(centeredX) <= this.sceneData.width / 2 + SHORE_BELT_WIDTH
      && Math.abs(centeredZ) <= this.sceneData.depth / 2 + SHORE_BELT_WIDTH;
  }

  private isBoatOnWater(centeredX: number, centeredZ: number): boolean {
    if (!this.sceneData) return false;
    const halfWidth = this.sceneData.width / 2;
    const halfDepth = this.sceneData.depth / 2;
    const oceanMargin = Math.max(720, Math.max(this.sceneData.width, this.sceneData.depth) * 0.92);
    const outsideLand = Math.abs(centeredX) > halfWidth + SHORE_BELT_WIDTH + 8
      || Math.abs(centeredZ) > halfDepth + SHORE_BELT_WIDTH + 8;
    const insideOcean = Math.abs(centeredX) < halfWidth + oceanMargin - 48
      && Math.abs(centeredZ) < halfDepth + oceanMargin - 48;
    return outsideLand && insideOcean;
  }

  private updateBoatCamera() {
    if (!this.activeBoat) return;
    const boat = this.activeBoat;
    if (this.boatAiming) {
      const side = this.boatCannonSide;
      const rightX = Math.cos(boat.yaw);
      const rightZ = -Math.sin(boat.yaw);
      const forwardX = Math.sin(boat.yaw);
      const forwardZ = Math.cos(boat.yaw);
      const targetDistance = 52;
      this.boatLookTarget.set(
        boat.group.position.x + rightX * side * targetDistance + forwardX * 7,
        boat.group.position.y + 2.4 + Math.tan(this.boatAimPitch) * targetDistance,
        boat.group.position.z + rightZ * side * targetDistance + forwardZ * 7,
      );
      this.camera.position.set(
        boat.group.position.x - rightX * side * 8 - forwardX * 8,
        boat.group.position.y + 7.2,
        boat.group.position.z - rightZ * side * 8 - forwardZ * 8,
      );
      this.camera.lookAt(this.boatLookTarget);
      this.camera.updateMatrixWorld();
      this.updateAimReticle();
      this.render();
      return;
    }
    const yaw = boat.yaw + this.boatCameraYawOffset;
    const distance = 32;
    const height = 8 + Math.sin(this.boatCameraPitch) * 15;
    this.camera.position.set(
      boat.group.position.x + Math.sin(yaw) * distance,
      boat.group.position.y + height,
      boat.group.position.z + Math.cos(yaw) * distance,
    );
    this.boatLookTarget.set(boat.group.position.x, boat.group.position.y + 2.4, boat.group.position.z);
    this.camera.lookAt(this.boatLookTarget);
    this.camera.updateMatrixWorld();
    this.updateAimReticle();
    this.render();
  }

  districtLabels(): CityScreenLabel[] {
    if (!this.sceneData) return [];
    const point = new Vector3();
    return this.sceneData.districts.map((district) => {
      point.set(
        district.x + district.width / 2 - this.sceneData!.width / 2,
        2,
        district.y + district.depth / 2 - this.sceneData!.depth / 2,
      ).project(this.camera);
      const x = (point.x * 0.5 + 0.5) * this.width;
      const y = (-point.y * 0.5 + 0.5) * this.height;
      return {
        id: district.id,
        name: district.name,
        fileCount: district.fileCount,
        x,
        y,
        visible: point.z > -1 && point.z < 1 && x > -80 && x < this.width + 80 && y > -30 && y < this.height + 30,
      };
    });
  }

  render() {
    if (this.disposed) return;
    this.renderer.render(this.threeScene, this.camera);
  }

  private startWaveAnimation() {
    if (this.waveAnimationFrame !== null) {
      cancelAnimationFrame(this.waveAnimationFrame);
      this.waveAnimationFrame = null;
    }
    if (!this.waveMesh || this.waveInstances.length === 0) return;

    const animate = (time: number) => {
      if (this.disposed || !this.waveMesh) {
        this.waveAnimationFrame = null;
        return;
      }
      this.waveAnimationFrame = requestAnimationFrame(animate);
      if (time - this.lastWaveAnimationAt < 66) return;
      this.lastWaveAnimationAt = time;

      const drift = time * 0.0011;
      if (this.oceanWaveTime) this.oceanWaveTime.value = drift;
      for (let index = 0; index < this.waveInstances.length; index += 1) {
        const wave = this.waveInstances[index];
        const phase = drift * wave.speed + wave.phase;
        const pulse = Math.sin(phase);
        const travel = (Math.sin(phase * 0.54) * 0.5 + 0.5) * wave.travel;
        this.waveQuaternion.setFromAxisAngle(this.waveAxis, wave.rotation);
        this.wavePosition.set(
          wave.x + wave.driftX * travel,
          -1.54 + pulse * wave.amplitude,
          wave.z + wave.driftZ * travel,
        );
        this.waveScale.set(
          wave.length * (1 + pulse * 0.07),
          0.035,
          wave.depth * (1 + Math.cos(phase * 0.8) * 0.16),
        );
        this.waveMatrix.compose(this.wavePosition, this.waveQuaternion, this.waveScale);
        this.waveMesh.setMatrixAt(index, this.waveMatrix);
      }
      this.waveMesh.instanceMatrix.needsUpdate = true;
      for (const boat of this.boatById.values()) {
        const boatPhase = drift * 0.72 + boat.bobPhase;
        boat.group.position.y = -1.02 + Math.sin(boatPhase) * 0.09;
        boat.group.rotation.x = Math.sin(boatPhase * 0.83) * 0.009;
        boat.group.rotation.z = Math.cos(boatPhase * 0.67) * 0.012;
      }
      if (this.mode === "boat" && this.activeBoat) this.updateBoatCamera();
      else this.render();
    };

    this.waveAnimationFrame = requestAnimationFrame(animate);
  }

  clear() {
    this.clearDynamicScene();
    this.sceneData = null;
    this.hoveredPath = "";
    this.target.set(0, 0, 0);
    this.walkPosition.set(0, 0, 0);
    this.render();
  }

  private updateCamera() {
    if (this.mode === "walking") {
      this.updateWalkingCamera();
      return;
    }
    const horizontal = Math.cos(this.pitch) * this.distance;
    this.camera.position.set(
      this.target.x + Math.sin(this.yaw) * horizontal,
      this.target.y + Math.sin(this.pitch) * this.distance,
      this.target.z + Math.cos(this.yaw) * horizontal,
    );
    this.camera.lookAt(this.target);
    this.camera.updateMatrixWorld();
    this.render();
  }

  private updateWalkingCamera() {
    const bob = Math.sin(this.walkPhase) * WALK_BOB_AMOUNT;
    this.camera.position.set(this.walkPosition.x, WALK_EYE_HEIGHT + bob, this.walkPosition.z);
    const lookDirection = new Vector3(
      Math.sin(this.walkYaw) * Math.cos(this.walkPitch),
      Math.sin(this.walkPitch),
      Math.cos(this.walkYaw) * Math.cos(this.walkPitch),
    );
    this.camera.lookAt(this.camera.position.clone().add(lookDirection));
    this.camera.updateMatrixWorld();
    this.render();
  }

  private walkBy(forward: number, right: number) {
    if (!this.sceneData) return;
    const forwardVector = new Vector3(Math.sin(this.walkYaw), 0, Math.cos(this.walkYaw));
    const rightVector = new Vector3(-Math.cos(this.walkYaw), 0, Math.sin(this.walkYaw));
    const move = forwardVector.multiplyScalar(forward * WALK_STEP)
      .add(rightVector.multiplyScalar(right * WALK_STEP));
    if (move.lengthSq() <= 0.0001) return;
    move.multiplyScalar(this.walkSpeed);
    const proposed = this.walkPosition.clone().add(move);
    if (this.isWalkableRoad(proposed.x, proposed.z)) {
      this.walkPosition.copy(proposed);
    } else {
      const forwardOnly = this.walkPosition.clone().add(new Vector3(move.x, 0, 0));
      const rightOnly = this.walkPosition.clone().add(new Vector3(0, 0, move.z));
      if (this.isWalkableRoad(forwardOnly.x, forwardOnly.z)) this.walkPosition.copy(forwardOnly);
      else if (this.isWalkableRoad(rightOnly.x, rightOnly.z)) this.walkPosition.copy(rightOnly);
    }
    this.walkPhase += move.length() * 0.95;
    this.updateWalkingCamera();
  }

  private isWalkableRoad(centeredX: number, centeredZ: number): boolean {
    if (!this.sceneData) return false;
    const x = centeredX + this.sceneData.width / 2;
    const y = centeredZ + this.sceneData.depth / 2;
    return this.sceneData.roads.some((road) =>
      x >= road.x - WALK_ROAD_PADDING
      && x <= road.x + road.width + WALK_ROAD_PADDING
      && y >= road.y - WALK_ROAD_PADDING
      && y <= road.y + road.depth + WALK_ROAD_PADDING,
    );
  }

  private nearestRoadPoint(centeredX: number, centeredZ: number): Vector3 {
    if (!this.sceneData || this.sceneData.roads.length === 0) return new Vector3(centeredX, 0, centeredZ);
    const x = centeredX + this.sceneData.width / 2;
    const y = centeredZ + this.sceneData.depth / 2;
    let bestX = this.sceneData.width / 2;
    let bestY = this.sceneData.depth / 2;
    let bestDistance = Number.POSITIVE_INFINITY;
    for (const road of this.sceneData.roads) {
      const clampedX = clamp(x, road.x, road.x + road.width);
      const clampedY = clamp(y, road.y, road.y + road.depth);
      const distance = (x - clampedX) ** 2 + (y - clampedY) ** 2;
      if (distance < bestDistance) {
        bestDistance = distance;
        bestX = clampedX;
        bestY = clampedY;
      }
    }
    return new Vector3(bestX - this.sceneData.width / 2, 0, bestY - this.sceneData.depth / 2);
  }

  private groundPointFromScreen(x: number, y: number): Vector3 | null {
    if (this.width <= 0 || this.height <= 0) return null;
    this.pointer.set((x / this.width) * 2 - 1, -(y / this.height) * 2 + 1);
    this.raycaster.setFromCamera(this.pointer, this.camera);
    const directionY = this.raycaster.ray.direction.y;
    if (Math.abs(directionY) < 0.0001) return null;
    const distance = -this.raycaster.ray.origin.y / directionY;
    if (distance < 0) return null;
    return this.raycaster.ray.origin.clone().addScaledVector(this.raycaster.ray.direction, distance);
  }

  private clearDynamicScene() {
    if (this.waveAnimationFrame !== null) {
      cancelAnimationFrame(this.waveAnimationFrame);
      this.waveAnimationFrame = null;
    }
    if (this.cannonAnimationFrame !== null) {
      cancelAnimationFrame(this.cannonAnimationFrame);
      this.cannonAnimationFrame = null;
    }
    const keep = new Set(this.threeScene.children.filter((child) => (child as Light).isLight));
    for (const child of [...this.threeScene.children]) {
      if (!keep.has(child)) this.threeScene.remove(child);
    }
    this.buildingMesh?.dispose();
    this.districtMesh?.dispose();
    this.roadMesh?.dispose();
    this.radarMesh?.dispose();
    this.waveMesh?.dispose();
    this.cannonBallMesh?.dispose();
    this.debrisMesh?.dispose();
    this.impactMesh?.dispose();
    this.roadMesh = null;
    this.buildingMesh = null;
    this.districtMesh = null;
    this.radarMesh = null;
    this.waveMesh = null;
    this.oceanWaveTime = null;
    this.cannonBallMesh = null;
    this.debrisMesh = null;
    this.impactMesh = null;
    this.aimReticle = null;
    this.buildingByPath.clear();
    this.buildingByIndex.length = 0;
    this.districtByIndex.length = 0;
    this.boatById.clear();
    this.boatPickTargets.length = 0;
    this.boatByMesh.clear();
    this.activeBoat = null;
    this.boatAiming = false;
    this.cannonBalls.length = 0;
    this.debrisBlocks.length = 0;
    this.impactParticles.length = 0;
    this.collapsedBuildingPaths.clear();
    this.lastCannonAnimationAt = 0;
    this.waveInstances.length = 0;
    this.lastWaveAnimationAt = 0;
    this.disposableGeometries.forEach((geometry) => geometry.dispose());
    this.disposableMaterials.forEach((material) => disposeMaterial(material));
    this.disposableGeometries.clear();
    this.disposableMaterials.clear();
  }

  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    this.clearDynamicScene();
    this.threeScene.clear();
    this.renderer.setAnimationLoop(null);
    this.renderer.renderLists.dispose();
    this.renderer.dispose();
    this.renderer.forceContextLoss();
    this.canvas.width = 1;
    this.canvas.height = 1;
    this.sceneData = null;
  }
}

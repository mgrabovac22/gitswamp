import {
  ACESFilmicToneMapping,
  AmbientLight,
  BoxGeometry,
  Color,
  ConeGeometry,
  CylinderGeometry,
  DirectionalLight,
  FogExp2,
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
  Vector2,
  Vector3,
  WebGLRenderer,
} from "three";
import type { BufferGeometry, Light, Material } from "three";
import type {
  CityBuilding,
  CityCameraMode,
  CityRadarMarker,
  CityRoad,
  CityScene,
} from "./repositoryCity.types";

const CLEAR_COLOR = 0x0a1320;
const MAX_PIXEL_RATIO = 1.25;
const MAX_TREES = 640;
const MAX_ROOF_MARKERS = 180;
const MAX_ROOF_DETAILS = 620;
const MAX_LANE_MARKS = 720;
const MAX_TRAFFIC_LIGHTS = 96;
const MAX_WAVE_LINES = 148;
const MAX_GRASS_PATCHES = 760;
const MAX_SIDEWALK_CORNERS = 560;
const VISUAL_ROAD_WIDTH = 12;
const WALK_EYE_HEIGHT = 4.8;
const WALK_STEP = 0.72;
const WALK_BOB_AMOUNT = 0.12;
const WALK_ROAD_PADDING = 5;

export interface CityScreenLabel {
  id: string;
  name: string;
  fileCount: number;
  x: number;
  y: number;
  visible: boolean;
}

interface BuildingVisual {
  building: CityBuilding;
  index: number;
  baseColor: Color;
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
  private readonly disposableGeometries = new Set<BufferGeometry>();
  private readonly disposableMaterials = new Set<Material>();
  private roadMesh: InstancedMesh | null = null;
  private buildingMesh: InstancedMesh | null = null;
  private radarMesh: InstancedMesh | null = null;
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
    this.addDistricts(scene);
    this.addParks(scene);
    this.addGrassDetails(scene);
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
    for (let index = 0; index < MAX_WAVE_LINES; index += 1) {
      const seed = hashText(`city-wave:${index}:${scene.width}:${scene.depth}`);
      const side = seed % 4;
      const progress = ((seed >>> 6) % 1000) / 1000;
      const offset = 30 + ((seed >>> 16) % Math.max(48, Math.floor(margin - 72)));
      const x = side < 2
        ? -scene.width / 2 + progress * scene.width
        : (side === 2 ? -scene.width / 2 - offset : scene.width / 2 + offset);
      const z = side >= 2
        ? -scene.depth / 2 + progress * scene.depth
        : (side === 0 ? -scene.depth / 2 - offset : scene.depth / 2 + offset);
      quaternion.setFromAxisAngle(new Vector3(0, 1, 0), ((seed >>> 24) % 100) / 100 * Math.PI);
      position.set(x, -1.55, z);
      scale.set(14 + ((seed >>> 12) % 58), 0.035, 0.24 + ((seed >>> 20) % 18) / 100);
      matrix.compose(position, quaternion, scale);
      waves.setMatrixAt(used, matrix);
      used += 1;
    }
    waves.count = used;
    waves.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(waves);
    this.disposableGeometries.add(waveGeometry);
    this.disposableMaterials.add(waveMaterial);
  }

  private addGround(scene: CityScene) {
    const margin = 220;
    const geometry = new PlaneGeometry(scene.width + margin, scene.depth + margin);
    const material = new MeshStandardMaterial({
      color: 0x123421,
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
      { x: 0, z: -scene.depth / 2 - 4, width: scene.width + margin, depth: 8 },
      { x: 0, z: scene.depth / 2 + 4, width: scene.width + margin, depth: 8 },
      { x: -scene.width / 2 - 4, z: 0, width: 8, depth: scene.depth + margin },
      { x: scene.width / 2 + 4, z: 0, width: 8, depth: scene.depth + margin },
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
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addParks(scene: CityScene) {
    if (scene.parks.length === 0) return;
    const geometry = new BoxGeometry(1, 1, 1);
    const material = new MeshStandardMaterial({
      color: 0x256d3d,
      emissive: 0x082515,
      emissiveIntensity: 0.22,
      roughness: 0.95,
      metalness: 0,
    });
    const mesh = new InstancedMesh(geometry, material, scene.parks.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    scene.parks.forEach((park, index) => {
      matrix.compose(
        new Vector3(
          park.x + park.width / 2 - scene.width / 2,
          0.08,
          park.y + park.depth / 2 - scene.depth / 2,
        ),
        quaternion,
        new Vector3(park.width, 0.12, park.depth),
      );
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addGrassDetails(scene: CityScene) {
    const patches: Array<{ x: number; z: number; scale: number }> = [];
    for (const park of scene.parks) {
      const count = clamp(Math.floor((park.width * park.depth) / 34), 4, 24);
      for (let index = 0; index < count && patches.length < MAX_GRASS_PATCHES; index += 1) {
        const seed = hashText(`${park.id}:grass:${index}`);
        patches.push({
          x: park.x + ((seed >>> 6) % 1000) / 1000 * park.width - scene.width / 2,
          z: park.y + ((seed >>> 16) % 1000) / 1000 * park.depth - scene.depth / 2,
          scale: 0.65 + ((seed >>> 26) % 38) / 100,
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
        });
      }
    }
    if (patches.length === 0) return;
    const geometry = new ConeGeometry(0.32, 1.1, 3);
    const material = new MeshStandardMaterial({
      color: 0x4d8f48,
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
        new Vector3(patch.x, 0.52, patch.z),
        quaternion,
        new Vector3(patch.scale, patch.scale, patch.scale),
      );
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
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
        new Color(
          road.kind === "connector"
            ? "#2d3035"
            : road.kind === "avenue"
              ? "#32363b"
              : "#292d32",
        ),
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
      color: 0x2f3338,
      roughness: 0.88,
      metalness: 0.02,
    });
    const mesh = new InstancedMesh(geometry, material, intersections.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    intersections.forEach((intersection, index) => {
      matrix.compose(
        new Vector3(
          intersection.x + intersection.width / 2 - scene.width / 2,
          0.535,
          intersection.y + intersection.depth / 2 - scene.depth / 2,
        ),
        quaternion,
        new Vector3(intersection.width, 0.055, intersection.depth),
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
    const horizontalRoads = scene.roads.filter((road) => roadOrientation(road) === "horizontal");
    const verticalRoads = scene.roads.filter((road) => roadOrientation(road) === "vertical");
    const sidewalkWidth = 2.2;
    for (const road of scene.roads) {
      const horizontal = roadOrientation(road) === "horizontal";
      if (horizontal) {
        const blocks = verticalRoads
          .filter((other) => roadIntersects(road, other))
          .map((other) => ({ start: other.x - sidewalkWidth, end: other.x + other.width + sidewalkWidth }));
        for (const segment of splitByBlocks(road.x, road.x + road.width, blocks)) {
          sidewalks.push(
            { x: segment.start, y: road.y - sidewalkWidth, width: segment.end - segment.start, depth: sidewalkWidth },
            { x: segment.start, y: road.y + road.depth, width: segment.end - segment.start, depth: sidewalkWidth },
          );
        }
      } else {
        const blocks = horizontalRoads
          .filter((other) => roadIntersects(road, other))
          .map((other) => ({ start: other.y - sidewalkWidth, end: other.y + other.depth + sidewalkWidth }));
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
      color: 0x535a63,
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
          0.42,
          sidewalk.y + sidewalk.depth / 2 - scene.depth / 2,
        ),
        quaternion,
        new Vector3(sidewalk.width, 0.08, sidewalk.depth),
      );
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.setUsage(StaticDrawUsage);
    this.threeScene.add(mesh);
    this.disposableGeometries.add(geometry);
    this.disposableMaterials.add(material);
  }

  private addSidewalkCorners(scene: CityScene) {
    const horizontal = scene.roads.filter((road) => roadOrientation(road) === "horizontal");
    const vertical = scene.roads.filter((road) => roadOrientation(road) === "vertical");
    const sidewalkWidth = 2.2;
    const corners: Array<{ x: number; y: number; scaleX: number; scaleZ: number }> = [];
    const seen = new Set<string>();

    const addCorner = (x: number, y: number, scaleX = sidewalkWidth * 1.15, scaleZ = sidewalkWidth * 1.15) => {
      if (corners.length >= MAX_SIDEWALK_CORNERS) return;
      const key = `${Math.round(x * 2)}:${Math.round(y * 2)}`;
      if (seen.has(key)) return;
      seen.add(key);
      corners.push({ x, y, scaleX, scaleZ });
    };

    for (const hRoad of horizontal) {
      for (const vRoad of vertical) {
        if (corners.length >= MAX_SIDEWALK_CORNERS) break;
        if (!roadIntersects(hRoad, vRoad)) continue;
        const left = Math.max(hRoad.x, vRoad.x);
        const right = Math.min(hRoad.x + hRoad.width, vRoad.x + vRoad.width);
        const top = Math.max(hRoad.y, vRoad.y);
        const bottom = Math.min(hRoad.y + hRoad.depth, vRoad.y + vRoad.depth);
        const centerX = (left + right) / 2;
        const centerY = (top + bottom) / 2;
        const offsetX = Math.max(VISUAL_ROAD_WIDTH / 2 + sidewalkWidth * 0.42, (right - left) / 2 + sidewalkWidth * 0.52);
        const offsetY = Math.max(VISUAL_ROAD_WIDTH / 2 + sidewalkWidth * 0.42, (bottom - top) / 2 + sidewalkWidth * 0.52);
        addCorner(centerX - offsetX, centerY - offsetY);
        addCorner(centerX + offsetX, centerY - offsetY);
        addCorner(centerX - offsetX, centerY + offsetY);
        addCorner(centerX + offsetX, centerY + offsetY);
      }
    }

    for (const road of scene.roads) {
      if (corners.length >= MAX_SIDEWALK_CORNERS) break;
      const horizontalRoad = roadOrientation(road) === "horizontal";
      if (horizontalRoad) {
        addCorner(road.x - sidewalkWidth * 0.5, road.y - sidewalkWidth * 0.5, sidewalkWidth, sidewalkWidth);
        addCorner(road.x - sidewalkWidth * 0.5, road.y + road.depth + sidewalkWidth * 0.5, sidewalkWidth, sidewalkWidth);
        addCorner(road.x + road.width + sidewalkWidth * 0.5, road.y - sidewalkWidth * 0.5, sidewalkWidth, sidewalkWidth);
        addCorner(road.x + road.width + sidewalkWidth * 0.5, road.y + road.depth + sidewalkWidth * 0.5, sidewalkWidth, sidewalkWidth);
      } else {
        addCorner(road.x - sidewalkWidth * 0.5, road.y - sidewalkWidth * 0.5, sidewalkWidth, sidewalkWidth);
        addCorner(road.x + road.width + sidewalkWidth * 0.5, road.y - sidewalkWidth * 0.5, sidewalkWidth, sidewalkWidth);
        addCorner(road.x - sidewalkWidth * 0.5, road.y + road.depth + sidewalkWidth * 0.5, sidewalkWidth, sidewalkWidth);
        addCorner(road.x + road.width + sidewalkWidth * 0.5, road.y + road.depth + sidewalkWidth * 0.5, sidewalkWidth, sidewalkWidth);
      }
    }

    if (corners.length === 0) return;
    const geometry = new CylinderGeometry(1, 1, 0.075, 14);
    const material = new MeshStandardMaterial({
      color: 0x535a63,
      roughness: 0.92,
      metalness: 0,
    });
    const mesh = new InstancedMesh(geometry, material, corners.length);
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    corners.forEach((corner, index) => {
      matrix.compose(
        new Vector3(corner.x - scene.width / 2, 0.423, corner.y - scene.depth / 2),
        quaternion,
        new Vector3(corner.scaleX, 1, corner.scaleZ),
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
      const horizontal = roadOrientation(road) === "horizontal";
      const length = horizontal ? road.width : road.depth;
      if (length < 34) continue;
      const dashCount = Math.min(42, Math.floor(length / 18));
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
    const horizontal = scene.roads.filter((road) => roadOrientation(road) === "horizontal");
    const vertical = scene.roads.filter((road) => roadOrientation(road) === "vertical");
    const intersections: Array<{ x: number; y: number }> = [];
    const seen = new Set<string>();
    for (const hRoad of horizontal) {
      for (const vRoad of vertical) {
        if (intersections.length >= MAX_TRAFFIC_LIGHTS) break;
        if (!roadIntersects(hRoad, vRoad)) continue;
        const left = Math.max(hRoad.x, vRoad.x);
        const right = Math.min(hRoad.x + hRoad.width, vRoad.x + vRoad.width);
        const top = Math.max(hRoad.y, vRoad.y);
        const bottom = Math.min(hRoad.y + hRoad.depth, vRoad.y + vRoad.depth);
        const x = (left + right) / 2;
        const y = (top + bottom) / 2;
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
      const lightX = intersection.x + 4.6 - scene.width / 2;
      const lightZ = intersection.y + 4.6 - scene.depth / 2;
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
    for (const park of scene.parks) {
      const count = clamp(Math.ceil(park.treeCount * 1.65), 4, 38);
      for (let index = 0; index < count && candidates.length < MAX_TREES; index += 1) {
        const seed = hashText(`${park.id}:tree:${index}`);
        const x = park.x + 2 + ((seed >>> 4) % 1000) / 1000 * Math.max(1, park.width - 4);
        const z = park.y + 2 + ((seed >>> 14) % 1000) / 1000 * Math.max(1, park.depth - 4);
        candidates.push({
          x: x - scene.width / 2,
          z: z - scene.depth / 2,
          scale: 0.68 + ((seed >>> 24) % 55) / 100,
          kind: seed % 3 === 0 ? "round" : "pine",
        });
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
    if (mode === "walking") {
      this.walkPosition.copy(this.nearestRoadPoint(this.target.x, this.target.z));
      this.updateWalkingCamera();
      return;
    }
    this.fit(mode);
  }

  orbit(deltaX: number, deltaY: number) {
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
    if (this.mode === "walking") return;
    const before = anchor ? this.groundPointFromScreen(anchor.x, anchor.y) : null;
    this.distance = clamp(this.distance * factor, 24, Math.max(5000, (this.sceneData?.width ?? 500) * 6));
    this.updateCamera();
    if (!before || !anchor) return;
    const after = this.groundPointFromScreen(anchor.x, anchor.y);
    if (!after) return;
    this.target.add(before.sub(after));
    this.updateCamera();
  }

  moveByKeyboard(forward: number, right: number, fast = false) {
    if (forward === 0 && right === 0) return;
    if (this.mode === "walking") {
      this.walkBy(forward * (fast ? 1.65 : 1), right * (fast ? 1.65 : 1));
      return;
    }
    const amount = this.distance * (fast ? 0.052 : 0.028);
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
    const keep = new Set(this.threeScene.children.filter((child) => (child as Light).isLight));
    for (const child of [...this.threeScene.children]) {
      if (!keep.has(child)) this.threeScene.remove(child);
    }
    this.buildingMesh?.dispose();
    this.roadMesh?.dispose();
    this.radarMesh?.dispose();
    this.roadMesh = null;
    this.buildingMesh = null;
    this.radarMesh = null;
    this.buildingByPath.clear();
    this.buildingByIndex.length = 0;
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

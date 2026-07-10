<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  shallowRef,
  watch,
} from "vue";
import {
  Building2,
  Crosshair,
  Flame,
  Focus,
  Footprints,
  LoaderCircle,
  Map,
  MapPinned,
  Minus,
  Plus,
  RefreshCw,
  Search,
  Ship,
  Tags,
  Users,
} from "lucide-vue-next";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import type { BranchInfo } from "@/types";
import {
  clearRepositoryCityCache,
  findLatestFileCommit,
  loadRepositoryCity,
} from "./repositoryCityData";
import {
  buildRepositoryCityScene,
} from "./repositoryCityScene";
import {
  RepositoryCityThreeRenderer,
  type CityBoatInfo,
  type CityScreenLabel,
} from "./repositoryCityThree";
import type {
  CityCameraMode,
  CityDistrict,
  CityZoomMode,
  CityRadarMarker,
  CityScene,
  RepositoryCityFile,
  RepositoryCitySnapshot,
} from "./repositoryCity.types";

const props = defineProps<{
  repoPath: string;
  branches: BranchInfo[];
  currentBranch: string;
  workingFilePaths?: string[];
}>();

const emit = defineEmits<{
  close: [];
  openFile: [payload: { path: string; sha: string | null }];
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const canvasHostRef = ref<HTMLElement | null>(null);
const selectedRef = ref("");
const historyDepth = ref(300);
const loading = ref(false);
const error = ref("");
const searchQuery = ref("");
const selectedPath = ref("");
const scopePath = ref("");
const hoveredPath = ref("");
const hoveredDistrict = shallowRef<CityDistrict | null>(null);
const hoveredBoat = shallowRef<CityBoatInfo | null>(null);
const activeBoat = shallowRef<CityBoatInfo | null>(null);
const tooltip = ref({ x: 0, y: 0 });
const showHeat = ref(true);
const showLabels = ref(true);
const showRadar = ref(true);
const showMiniMap = ref(true);
const snapshot = shallowRef<RepositoryCitySnapshot | null>(null);
const scene = shallowRef<CityScene | null>(null);
const cameraMode = ref<CityCameraMode>("bird");
const zoomMode = ref<CityZoomMode>("center");
const walkSpeed = ref(0.65);
const districtLabels = shallowRef<CityScreenLabel[]>([]);
let resizeObserver: ResizeObserver | null = null;
let threeRenderer: RepositoryCityThreeRenderer | null = null;
let canvasWidth = 1;
let canvasHeight = 1;
let drawFrame: number | null = null;
let loadToken = 0;
const pointerDown = ref(false);
let dragged = false;
let panGesture = false;
let boatAimGesture = false;
let pointerStartX = 0;
let pointerStartY = 0;
let previousPointerX = 0;
let previousPointerY = 0;
let keyboardFrame: number | null = null;
const pressedKeys = new Set<string>();
let lastRepoPath = "";

const branchOptions = computed(() => {
  const seen = new Set<string>();
  return props.branches
    .filter((branch) => {
      if (!branch.name || seen.has(branch.name)) return false;
      seen.add(branch.name);
      return true;
    })
    .sort((a, b) => {
      if (a.is_head !== b.is_head) return a.is_head ? -1 : 1;
      if (a.is_remote !== b.is_remote) return a.is_remote ? 1 : -1;
      return a.name.localeCompare(b.name);
    });
});

const hoveredFile = computed<RepositoryCityFile | null>(() => {
  if (!hoveredPath.value || !scene.value) return null;
  return scene.value.buildingByPath.get(hoveredPath.value)?.file ?? null;
});

const hotFileCount = computed(() =>
  snapshot.value?.files.filter((file) => file.heat >= 0.72).length ?? 0,
);

const miniMapDistricts = computed(() => {
  if (!scene.value) return [];
  return scene.value.districts.map((district) => ({
    ...district,
    left: (district.x / scene.value!.width) * 100,
    top: (district.y / scene.value!.depth) * 100,
    widthPercent: (district.width / scene.value!.width) * 100,
    depthPercent: (district.depth / scene.value!.depth) * 100,
  }));
});

const currentFolderLabel = computed(() => scopePath.value || "root");

const hoveredChildFolder = computed(() => {
  if (!hoveredFile.value) return "";
  return childFolderForFile(hoveredFile.value.path);
});

const miniMapMarkers = computed(() => {
  if (!scene.value) return [];
  return radarMarkers.value.flatMap((marker) => {
    const building = scene.value!.buildingByPath.get(marker.path);
    if (!building) return [];
    return [{
      ...marker,
      left: ((building.x + building.width / 2) / scene.value!.width) * 100,
      top: ((building.y + building.depth / 2) / scene.value!.depth) * 100,
    }];
  });
});

function branchRef(name: string): string {
  const branch = props.branches.find((item) => item.name === name);
  if (!branch) return name || "HEAD";
  if (branch.is_remote) {
    return name.startsWith("refs/remotes/") ? name : `refs/remotes/${name}`;
  }
  return name.startsWith("refs/heads/") ? name : `refs/heads/${name}`;
}

function preferredBranchName(): string {
  const options = branchOptions.value;
  const head = options.find((branch) => branch.is_head);
  if (head) return head.name;
  const current = options.find((branch) => branch.name === props.currentBranch);
  if (current) return current.name;
  const localPreferred = options.find((branch) =>
    !branch.is_remote && ["main", "master", "develop"].includes(branch.name.toLowerCase()),
  );
  if (localPreferred) return localPreferred.name;
  const local = options.find((branch) => !branch.is_remote);
  if (local) return local.name;
  const remotePreferred = options.find((branch) =>
    branch.is_remote && /(^|\/)(main|master|develop)$/i.test(branch.name),
  );
  return remotePreferred?.name ?? options[0]?.name ?? props.currentBranch ?? "HEAD";
}

function hasSelectedBranch(name: string): boolean {
  return name === "HEAD" || branchOptions.value.some((branch) => branch.name === name);
}

function markerColor(value: string): string {
  const colors = ["#22d3ee", "#a78bfa", "#fb7185", "#fbbf24", "#34d399", "#60a5fa", "#f472b6"];
  let hash = 0;
  for (let index = 0; index < value.length; index += 1) {
    hash = Math.imul(31, hash) + value.charCodeAt(index);
  }
  return colors[Math.abs(hash) % colors.length];
}

const radarMarkers = computed<CityRadarMarker[]>(() => {
  const value = snapshot.value;
  const currentScene = scene.value;
  if (!showRadar.value || !value || !currentScene) return [];

  const markers: CityRadarMarker[] = [];
  const visibleWorkingPaths = (props.workingFilePaths ?? [])
    .filter((path) => currentScene.buildingByPath.has(path))
    .slice(0, 3);
  visibleWorkingPaths.forEach((path, index) => {
    markers.push({
      id: `local-${index}-${path}`,
      name: value.userName || "You",
      email: value.userEmail,
      path,
      source: "local",
      activeAt: Math.floor(Date.now() / 1000),
      color: "#2dd4bf",
    });
  });

  for (const contributor of value.contributors.slice(0, 12)) {
    if (!contributor.primaryPath || !currentScene.buildingByPath.has(contributor.primaryPath)) continue;
    if (
      contributor.email
      && value.userEmail
      && contributor.email.toLowerCase() === value.userEmail.toLowerCase()
      && visibleWorkingPaths.length > 0
    ) {
      continue;
    }
    markers.push({
      id: contributor.id,
      name: contributor.name,
      email: contributor.email,
      path: contributor.primaryPath,
      source: "history",
      activeAt: contributor.lastActiveAt,
      color: markerColor(contributor.id),
    });
  }
  return markers;
});

function timeAgo(timestamp: number): string {
  if (!timestamp) return "No recent activity";
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - timestamp);
  if (seconds < 60) return "Just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

function requestDraw() {
  if (drawFrame !== null) return;
  drawFrame = requestAnimationFrame(() => {
    drawFrame = null;
    threeRenderer?.render();
    refreshDistrictLabels();
  });
}

function refreshDistrictLabels() {
  districtLabels.value = showLabels.value
    ? (threeRenderer?.districtLabels() ?? []).filter((label) => label.visible)
    : [];
}

function ensureThreeRenderer(): RepositoryCityThreeRenderer | null {
  const canvas = canvasRef.value;
  if (!canvas) return null;
  if (!threeRenderer) {
    threeRenderer = new RepositoryCityThreeRenderer(canvas);
    threeRenderer.resize(canvasWidth, canvasHeight);
  }
  return threeRenderer;
}

function resizeCanvas() {
  const host = canvasHostRef.value;
  if (!host) return;
  const bounds = host.getBoundingClientRect();
  canvasWidth = Math.max(1, Math.floor(bounds.width));
  canvasHeight = Math.max(1, Math.floor(bounds.height));
  threeRenderer?.resize(canvasWidth, canvasHeight);
  refreshDistrictLabels();
}

function fitCity() {
  if (cameraMode.value === "boat") {
    cameraMode.value = "bird";
    activeBoat.value = null;
    threeRenderer?.setBoatAiming(false);
    threeRenderer?.setMode("bird");
    requestDraw();
    return;
  }
  threeRenderer?.fit(cameraMode.value);
  requestDraw();
}

function setCameraMode(mode: CityCameraMode) {
  cameraMode.value = mode;
  if (mode !== "boat") activeBoat.value = null;
  if (mode !== "boat") threeRenderer?.setBoatAiming(false);
  threeRenderer?.setMode(mode);
  canvasRef.value?.focus({ preventScroll: true });
  requestDraw();
}

function zoomBy(factor: number, anchor?: { x: number; y: number }) {
  if (anchor && zoomMode.value === "cursor") {
    threeRenderer?.zoomAt(1 / factor, anchor);
  } else {
    threeRenderer?.zoomAt(1 / factor);
  }
  requestDraw();
}

function focusBuilding(path: string) {
  selectedPath.value = path;
  threeRenderer?.focus(path);
  requestDraw();
}

function hasNestedFolder(path: string): boolean {
  if (!snapshot.value || !path || path === scopePath.value) return false;
  const prefix = `${path}/`;
  return snapshot.value.files.some((file) => file.path.startsWith(prefix));
}

function canOpenDistrict(district: CityDistrict | null): district is CityDistrict {
  return !!district && hasNestedFolder(district.path);
}

function childFolderForFile(path: string): string {
  if (!snapshot.value || !path) return "";
  const normalizedScope = scopePath.value ? `${scopePath.value}/` : "";
  if (normalizedScope && !path.startsWith(normalizedScope)) return "";
  const relativePath = normalizedScope ? path.slice(normalizedScope.length) : path;
  const slashIndex = relativePath.indexOf("/");
  if (slashIndex < 0) return "";
  const child = relativePath.slice(0, slashIndex);
  return scopePath.value ? `${scopePath.value}/${child}` : child;
}

function parentFolder(path: string): string {
  const index = path.lastIndexOf("/");
  return index > 0 ? path.slice(0, index) : "";
}

async function renderCurrentScene() {
  if (!snapshot.value) return;
  let nextScene = buildRepositoryCityScene(snapshot.value.files, scopePath.value);
  if (scopePath.value && nextScene.buildings.length === 0) {
    scopePath.value = "";
    nextScene = buildRepositoryCityScene(snapshot.value.files);
  }
  scene.value = nextScene;
  hoveredPath.value = "";
  hoveredDistrict.value = null;
  hoveredBoat.value = null;
  activeBoat.value = null;
  selectedPath.value = "";
  await nextTick();
  resizeCanvas();
  const renderer = ensureThreeRenderer();
  if (!renderer || !scene.value) throw new Error("3D renderer is unavailable.");
  renderer.setScene(scene.value);
  renderer.setHeatVisible(showHeat.value);
  renderer.setWalkSpeed(walkSpeed.value);
  renderer.setRadarMarkers(radarMarkers.value);
  fitCity();
}

function openFolderPath(path: string): boolean {
  if (!path || !hasNestedFolder(path)) return false;
  scopePath.value = path;
  void renderCurrentScene();
  return true;
}

function openDistrict(district: CityDistrict | null) {
  if (!canOpenDistrict(district)) return;
  openFolderPath(district.path);
}

function openParentFolder() {
  scopePath.value = parentFolder(scopePath.value);
  void renderCurrentScene();
}

function resetFolderScope() {
  if (!scopePath.value) return;
  scopePath.value = "";
  void renderCurrentScene();
}

function searchFile() {
  const query = searchQuery.value.trim().toLowerCase();
  if (!query || !snapshot.value) return;
  const exact = snapshot.value.files.find((file) => file.path.toLowerCase() === query);
  const partial = snapshot.value.files.find((file) => file.path.toLowerCase().includes(query));
  const match = exact ?? partial;
  if (match) focusBuilding(match.path);
}

async function loadCity(force = false) {
  if (!props.repoPath || !selectedRef.value) return;
  const token = ++loadToken;
  loading.value = true;
  error.value = "";
  hoveredPath.value = "";
  selectedPath.value = "";
  try {
    await nextTick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    if (force) clearRepositoryCityCache(props.repoPath);
    const next = await loadRepositoryCity(
      props.repoPath,
      branchRef(selectedRef.value),
      historyDepth.value,
    );
    if (token !== loadToken) {
      clearRepositoryCityCache(props.repoPath);
      return;
    }
    const currentBranchSelected = selectedRef.value === props.currentBranch;
    const knownPaths = new Set(next.files.map((file) => file.path));
    const workingOnlyFiles: RepositoryCityFile[] = currentBranchSelected
      ? (props.workingFilePaths ?? [])
        .filter((path) => !knownPaths.has(path))
        .map((path) => ({
          path,
          folder: path.includes("/") ? path.split("/")[0] : "Repository root",
          size: 0,
          touches: 0,
          churn: 0,
          fixTouches: 0,
          heat: 0.48,
          lastAuthor: next.userName,
          lastAuthorEmail: next.userEmail,
          lastChangedAt: Math.floor(Date.now() / 1000),
          lastCommitSha: "",
        }))
      : [];
    const displayedSnapshot = workingOnlyFiles.length > 0
      ? { ...next, files: [...next.files, ...workingOnlyFiles] }
      : next;
    snapshot.value = displayedSnapshot;
    await renderCurrentScene();
  } catch (loadError) {
    if (token === loadToken) {
      error.value = String(loadError);
      snapshot.value = null;
      scene.value = null;
      districtLabels.value = [];
      hoveredDistrict.value = null;
      hoveredBoat.value = null;
      activeBoat.value = null;
      threeRenderer?.clear();
    }
  } finally {
    if (token === loadToken) loading.value = false;
  }
}

function pointerPosition(event: PointerEvent | WheelEvent): { x: number; y: number } {
  const bounds = canvasRef.value?.getBoundingClientRect();
  return {
    x: event.clientX - (bounds?.left ?? 0),
    y: event.clientY - (bounds?.top ?? 0),
  };
}

function onPointerDown(event: PointerEvent) {
  canvasRef.value?.focus({ preventScroll: true });
  boatAimGesture = cameraMode.value === "boat" && event.ctrlKey;
  if (boatAimGesture) {
    pressedKeys.add("control");
    threeRenderer?.setBoatAiming(true);
  }
  pointerDown.value = true;
  dragged = false;
  panGesture = event.shiftKey || event.button !== 0;
  pointerStartX = event.clientX;
  pointerStartY = event.clientY;
  previousPointerX = event.clientX;
  previousPointerY = event.clientY;
  canvasRef.value?.setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
  const position = pointerPosition(event);
  tooltip.value = { x: position.x + 14, y: position.y + 14 };
  if (pointerDown.value) {
    const dx = event.clientX - previousPointerX;
    const dy = event.clientY - previousPointerY;
    if (Math.hypot(event.clientX - pointerStartX, event.clientY - pointerStartY) > 3) dragged = true;
    if (cameraMode.value === "boat" && (boatAimGesture || pressedKeys.has("control") || event.ctrlKey)) {
      threeRenderer?.adjustBoatAim(dy);
    } else if (panGesture || event.shiftKey) {
      threeRenderer?.pan(dx, dy);
    } else {
      threeRenderer?.orbit(dx, dy);
    }
    previousPointerX = event.clientX;
    previousPointerY = event.clientY;
    hoveredPath.value = "";
    hoveredDistrict.value = null;
    hoveredBoat.value = null;
    requestDraw();
    return;
  }

  const hit = threeRenderer?.pick(position.x, position.y) ?? null;
  const nextPath = hit?.file.path ?? "";
  const boatHit = hit ? null : (threeRenderer?.pickBoat(position.x, position.y) ?? null);
  const districtHit = hit || boatHit ? null : (threeRenderer?.pickDistrict(position.x, position.y) ?? null);
  if (hoveredPath.value !== nextPath) {
    hoveredPath.value = nextPath;
    threeRenderer?.setHoveredPath(nextPath);
    refreshDistrictLabels();
  }
  hoveredBoat.value = boatHit;
  hoveredDistrict.value = canOpenDistrict(districtHit) ? districtHit : null;
}

async function onPointerUp(event: PointerEvent) {
  if (!pointerDown.value) return;
  pointerDown.value = false;
  canvasRef.value?.releasePointerCapture(event.pointerId);
  const wasBoatAimGesture = boatAimGesture;
  boatAimGesture = false;
  if (cameraMode.value === "boat" && (wasBoatAimGesture || pressedKeys.has("control") || event.ctrlKey)) {
    threeRenderer?.fireBoatCannon();
    if (!pressedKeys.has("control") && !event.ctrlKey) threeRenderer?.setBoatAiming(false);
    requestDraw();
    return;
  }
  if (dragged) return;
  const position = pointerPosition(event);
  const boatHit = threeRenderer?.pickBoat(position.x, position.y) ?? null;
  if (boatHit) {
    activeBoat.value = threeRenderer?.enterBoat(boatHit.id) ?? boatHit;
    hoveredBoat.value = null;
    cameraMode.value = "boat";
    canvasRef.value?.focus({ preventScroll: true });
    requestDraw();
    return;
  }
  const hit = threeRenderer?.pick(position.x, position.y) ?? null;
  if (!hit) {
    openDistrict(threeRenderer?.pickDistrict(position.x, position.y) ?? null);
    return;
  }
  const childFolder = childFolderForFile(hit.file.path);
  if (openFolderPath(childFolder)) return;
  if (!snapshot.value) return;
  selectedPath.value = hit.file.path;
  requestDraw();
  let sha = hit.file.lastCommitSha;
  if (!sha) {
    sha = await findLatestFileCommit(props.repoPath, snapshot.value.refName, hit.file.path).catch(() => "");
  }
  const isWorkingFile = (props.workingFilePaths ?? []).includes(hit.file.path)
    && selectedRef.value === props.currentBranch;
  if (sha || isWorkingFile) {
    emit("openFile", { path: hit.file.path, sha: sha || null });
  }
}

function onWheel(event: WheelEvent) {
  event.preventDefault();
  zoomBy(event.deltaY < 0 ? 1.1 : 0.9, pointerPosition(event));
}

function onPointerLeave() {
  hoveredPath.value = "";
  hoveredDistrict.value = null;
  hoveredBoat.value = null;
  threeRenderer?.setHoveredPath("");
}

function onPointerCancel() {
  pointerDown.value = false;
  dragged = false;
  boatAimGesture = false;
}

function isEditableKeyTarget(event: KeyboardEvent): boolean {
  const target = event.target as HTMLElement | null;
  if (!target) return false;
  const tag = target.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || tag === "select" || target.isContentEditable;
}

function keyboardDirection() {
  const forward = (pressedKeys.has("w") ? 1 : 0) - (pressedKeys.has("s") ? 1 : 0);
  const right = (pressedKeys.has("d") ? 1 : 0) - (pressedKeys.has("a") ? 1 : 0);
  return { forward, right };
}

function requestKeyboardNavigation() {
  if (keyboardFrame !== null) return;
  keyboardFrame = requestAnimationFrame(() => {
    keyboardFrame = null;
    const { forward, right } = keyboardDirection();
    if (forward !== 0 || right !== 0) {
      threeRenderer?.moveByKeyboard(forward, right, pressedKeys.has("shift"));
      requestDraw();
      requestKeyboardNavigation();
    }
  });
}

function onKeyDown(event: KeyboardEvent) {
  if (isEditableKeyTarget(event)) return;
  const key = event.key.toLowerCase();
  if (!["w", "a", "s", "d", "shift", "control"].includes(key)) return;
  event.preventDefault();
  pressedKeys.add(key);
  if (key === "control") {
    if (cameraMode.value === "boat") threeRenderer?.setBoatAiming(true);
    return;
  }
  requestKeyboardNavigation();
}

function onKeyUp(event: KeyboardEvent) {
  const key = event.key.toLowerCase();
  if (!["w", "a", "s", "d", "shift", "control"].includes(key)) return;
  pressedKeys.delete(key);
  if (key === "control" && !boatAimGesture) threeRenderer?.setBoatAiming(false);
}

function clearPressedKeys() {
  boatAimGesture = false;
  pressedKeys.clear();
  threeRenderer?.setBoatAiming(false);
}

watch(
  () => [props.repoPath, selectedRef.value, historyDepth.value],
  () => loadCity(),
);

watch(
  showHeat,
  (visible) => {
    threeRenderer?.setHeatVisible(visible);
    requestDraw();
  },
);

watch(
  showLabels,
  () => refreshDistrictLabels(),
);

watch(
  radarMarkers,
  (markers) => {
    threeRenderer?.setRadarMarkers(markers);
    requestDraw();
  },
);

watch(
  walkSpeed,
  (speed) => {
    threeRenderer?.setWalkSpeed(speed);
  },
);

watch(
  () => [
    props.repoPath,
    props.currentBranch,
    branchOptions.value.map((branch) => `${branch.name}:${branch.is_head}:${branch.is_remote}`).join("|"),
  ],
  () => {
    const repoChanged = props.repoPath !== lastRepoPath;
    lastRepoPath = props.repoPath;
    if (repoChanged) scopePath.value = "";
    if (repoChanged || !selectedRef.value || !hasSelectedBranch(selectedRef.value)) {
      selectedRef.value = preferredBranchName();
    }
  },
  { immediate: true },
);

onMounted(() => {
  resizeObserver = new ResizeObserver(resizeCanvas);
  if (canvasHostRef.value) resizeObserver.observe(canvasHostRef.value);
  globalThis.addEventListener("keydown", onKeyDown);
  globalThis.addEventListener("keyup", onKeyUp);
  globalThis.addEventListener("blur", clearPressedKeys);
  resizeCanvas();
});

onUnmounted(() => {
  loadToken += 1;
  resizeObserver?.disconnect();
  if (drawFrame !== null) cancelAnimationFrame(drawFrame);
  if (keyboardFrame !== null) cancelAnimationFrame(keyboardFrame);
  globalThis.removeEventListener("keydown", onKeyDown);
  globalThis.removeEventListener("keyup", onKeyUp);
  globalThis.removeEventListener("blur", clearPressedKeys);
  pressedKeys.clear();
  threeRenderer?.dispose();
  threeRenderer = null;
  clearRepositoryCityCache(props.repoPath);
  districtLabels.value = [];
  scene.value = null;
  snapshot.value = null;
  hoveredDistrict.value = null;
  hoveredBoat.value = null;
  activeBoat.value = null;
});
</script>

<template>
  <section class="flex h-full min-w-0 flex-1 flex-col overflow-hidden bg-[var(--background)]">
    <header class="flex min-h-14 flex-wrap items-center gap-2 border-b border-[var(--border)] px-3 py-2">
      <div class="flex min-w-0 items-center gap-2 pr-2">
        <Building2 class="h-4 w-4 shrink-0 text-[var(--primary)]" />
        <div class="min-w-0">
          <h2 class="truncate text-sm font-semibold text-[var(--foreground)]">Repository City</h2>
          <p class="truncate text-[10px] text-[var(--muted-foreground)]">
            {{ snapshot?.files.length || 0 }} buildings, {{ snapshot?.contributors.length || 0 }} contributors
          </p>
        </div>
      </div>

      <div class="h-7 w-px bg-[var(--border)]" />

      <select
        v-model="selectedRef"
        class="h-8 max-w-48 rounded-md border border-[var(--border)] bg-[var(--input)] px-2 text-xs text-[var(--foreground)] outline-none focus:border-[var(--primary)]"
        title="Branch city"
      >
        <option v-for="branch in branchOptions" :key="branch.name" :value="branch.name">
          {{ branch.is_head ? "Current, " : "" }}{{ branch.name }}
        </option>
      </select>

      <select
        v-model.number="historyDepth"
        class="h-8 rounded-md border border-[var(--border)] bg-[var(--input)] px-2 text-xs text-[var(--foreground)] outline-none focus:border-[var(--primary)]"
        title="Heat history depth"
      >
        <option :value="100">100 commits</option>
        <option :value="300">300 commits</option>
        <option :value="1000">1,000 commits</option>
        <option :value="0">All commits</option>
      </select>

      <form class="relative min-w-36 max-w-64 flex-1" @submit.prevent="searchFile">
        <Search class="pointer-events-none absolute left-2.5 top-2 h-3.5 w-3.5 text-[var(--muted-foreground)]" />
        <input
          v-model="searchQuery"
          type="search"
          placeholder="Find a building..."
          class="h-8 w-full rounded-md border border-[var(--border)] bg-[var(--input)] pl-8 pr-2 text-xs text-[var(--foreground)] outline-none placeholder:text-[var(--muted-foreground)] focus:border-[var(--primary)]"
        />
      </form>

      <div class="ml-auto flex items-center gap-1">
        <div class="flex h-8 items-center rounded-md border border-[var(--border)] bg-[var(--secondary)] p-0.5">
          <button
            type="button"
            class="flex h-6 items-center gap-1 rounded px-2 text-[10px] transition-colors"
            :class="cameraMode === 'bird' ? 'bg-[var(--primary)] text-[var(--primary-foreground)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
            title="Bird view"
            @click="setCameraMode('bird')"
          >
            <Map class="h-3 w-3" /> Bird
          </button>
          <button
            type="button"
            class="flex h-6 items-center gap-1 rounded px-2 text-[10px] transition-colors"
            :class="cameraMode === 'landscape' ? 'bg-[var(--primary)] text-[var(--primary-foreground)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
            title="Landscape view"
            @click="setCameraMode('landscape')"
          >
            <MapPinned class="h-3 w-3" /> Landscape
          </button>
          <button
            type="button"
            class="flex h-6 items-center gap-1 rounded px-2 text-[10px] transition-colors"
            :class="cameraMode === 'walking' ? 'bg-[var(--primary)] text-[var(--primary-foreground)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
            title="Walking mode"
            @click="setCameraMode('walking')"
          >
            <Footprints class="h-3 w-3" /> Walk
          </button>
        </div>

        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border transition-colors"
          :class="showHeat ? 'border-[var(--primary)]/50 bg-[var(--primary)]/15 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
          title="Heat layer"
          @click="showHeat = !showHeat"
        >
          <Flame class="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border transition-colors"
          :class="showLabels ? 'border-[var(--primary)]/50 bg-[var(--primary)]/15 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
          title="District labels"
          @click="showLabels = !showLabels"
        >
          <Tags class="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border transition-colors"
          :class="showRadar ? 'border-[var(--primary)]/50 bg-[var(--primary)]/15 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
          title="Team radar"
          @click="showRadar = !showRadar"
        >
          <Users class="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border transition-colors"
          :class="showMiniMap ? 'border-[var(--primary)]/50 bg-[var(--primary)]/15 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
          title="Mini map"
          @click="showMiniMap = !showMiniMap"
        >
          <Map class="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border transition-colors"
          :class="zoomMode === 'cursor' ? 'border-[var(--primary)]/50 bg-[var(--primary)]/15 text-[var(--primary)]' : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
          :title="zoomMode === 'cursor' ? 'Zoom follows cursor' : 'Zoom stays centered'"
          @click="zoomMode = zoomMode === 'cursor' ? 'center' : 'cursor'"
        >
          <Crosshair class="h-3.5 w-3.5" />
        </button>

        <button type="button" class="city-icon-button" title="Zoom out" @click="zoomBy(0.84)">
          <Minus class="h-3.5 w-3.5" />
        </button>
        <button type="button" class="city-icon-button" title="Fit city" @click="fitCity">
          <Focus class="h-3.5 w-3.5" />
        </button>
        <button type="button" class="city-icon-button" title="Zoom in" @click="zoomBy(1.18)">
          <Plus class="h-3.5 w-3.5" />
        </button>
        <button type="button" class="city-icon-button" title="Refresh city" :disabled="loading" @click="loadCity(true)">
          <RefreshCw class="h-3.5 w-3.5" :class="loading ? 'animate-spin' : ''" />
        </button>
        <label
          v-if="cameraMode === 'walking'"
          class="hidden h-8 items-center gap-2 rounded-md border border-[var(--border)] bg-[var(--secondary)] px-2 text-[10px] text-[var(--muted-foreground)] 2xl:flex"
          title="Walking speed"
        >
          <Footprints class="h-3 w-3 text-[var(--primary)]" />
          <input
            v-model.number="walkSpeed"
            type="range"
            min="0.35"
            max="1.25"
            step="0.05"
            class="h-1 w-20 accent-[var(--primary)]"
          />
          <span class="w-7 text-right text-[var(--foreground)]">{{ walkSpeed.toFixed(2) }}</span>
        </label>
        <CloseIconButton size="sm" subtle @click="emit('close')" />
      </div>
    </header>

    <div class="flex min-h-0 flex-1">
      <div ref="canvasHostRef" class="relative min-w-0 flex-1 overflow-hidden">
        <canvas
          ref="canvasRef"
          tabindex="0"
          aria-label="Repository city map"
          class="block h-full w-full touch-none"
          :class="cameraMode === 'walking' || cameraMode === 'boat' ? 'cursor-crosshair' : pointerDown ? 'cursor-grabbing' : hoveredPath || hoveredDistrict || hoveredBoat ? 'cursor-pointer' : 'cursor-grab'"
          @pointerdown="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="onPointerUp"
          @pointercancel="onPointerCancel"
          @pointerleave="onPointerLeave"
          @wheel="onWheel"
          @contextmenu.prevent
        />

        <div
          v-if="loading && !snapshot"
          class="absolute inset-0 flex items-center justify-center bg-[#0b1018]"
        >
          <div class="flex items-center gap-2 text-xs text-slate-300">
            <LoaderCircle class="h-4 w-4 animate-spin text-teal-400" />
            Surveying repository
          </div>
        </div>
        <div
          v-else-if="loading"
          class="pointer-events-none absolute left-1/2 top-4 z-20 flex -translate-x-1/2 items-center gap-2 rounded-md border border-teal-400/20 bg-slate-950/85 px-3 py-2 text-xs text-slate-200 shadow-lg"
        >
          <LoaderCircle class="h-3.5 w-3.5 animate-spin text-teal-400" />
          Updating city data
        </div>

        <div
          v-else-if="error"
          class="absolute inset-0 flex items-center justify-center bg-[#0b1018] p-8 text-center"
        >
          <div>
            <p class="text-sm font-semibold text-rose-300">City data could not be loaded</p>
            <p class="mt-1 max-w-xl text-xs text-slate-400">{{ error }}</p>
            <button class="mt-3 text-xs font-medium text-teal-300 hover:text-teal-200" @click="loadCity(true)">
              Try again
            </button>
          </div>
        </div>

        <div class="pointer-events-none absolute inset-0 overflow-hidden">
          <div
            v-for="label in districtLabels"
            :key="label.id"
            class="absolute -translate-x-1/2 -translate-y-1/2 rounded bg-slate-950/70 px-2 py-1 text-[9px] font-semibold text-slate-200 shadow-sm"
            :style="{ left: `${label.x}px`, top: `${label.y}px` }"
          >
            {{ label.name }}
            <span class="ml-1 font-normal text-slate-500">{{ label.fileCount }}</span>
          </div>
        </div>

        <div
          v-if="showMiniMap && scene"
          class="absolute bottom-3 left-3 h-32 w-44 overflow-hidden rounded-md border border-teal-400/20 bg-slate-950/90 p-2 shadow-lg"
        >
          <div class="mb-1 flex items-center justify-between gap-2 text-[8px] font-semibold uppercase text-slate-500">
            <button
              type="button"
              class="pointer-events-auto max-w-[7.5rem] truncate text-left text-teal-300 hover:text-teal-200"
              :title="`Current folder: ${currentFolderLabel}`"
              @click="resetFolderScope"
            >
              {{ currentFolderLabel }}
            </button>
            <button
              v-if="scopePath"
              type="button"
              class="pointer-events-auto rounded border border-teal-400/20 px-1.5 py-0.5 text-[8px] text-slate-300 hover:bg-teal-400/10 hover:text-teal-200"
              @click="openParentFolder"
            >
              Up
            </button>
            <span v-else>Branch map</span>
          </div>
          <div class="pointer-events-none relative h-[92px] w-full">
            <div
              v-for="district in miniMapDistricts"
              :key="district.id"
              class="absolute rounded-[2px] border border-sky-300/15"
              :class="district.averageHeat > 0.66 ? 'bg-rose-500/45' : 'bg-sky-400/20'"
              :style="{
                left: `${district.left}%`,
                top: `${district.top}%`,
                width: `${district.widthPercent}%`,
                height: `${district.depthPercent}%`,
              }"
            />
            <i
              v-for="marker in miniMapMarkers"
              :key="marker.id"
              class="absolute h-1.5 w-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full border border-white/60"
              :style="{ left: `${marker.left}%`, top: `${marker.top}%`, backgroundColor: marker.color }"
            />
          </div>
        </div>

        <div
          v-if="hoveredFile"
          class="pointer-events-none absolute z-20 w-64 rounded-md border border-slate-700 bg-slate-950/95 p-3 shadow-xl"
          :style="{ left: `${Math.max(8, Math.min(tooltip.x, canvasWidth - 270))}px`, top: `${Math.max(8, Math.min(tooltip.y, canvasHeight - 150))}px` }"
        >
          <p class="truncate text-xs font-semibold text-slate-100">{{ hoveredFile.path }}</p>
          <div class="mt-2 grid grid-cols-3 gap-2 text-[10px]">
            <div><span class="block text-slate-500">Size</span><span class="text-slate-200">{{ Math.max(1, Math.round(hoveredFile.size / 1024)) }} KB</span></div>
            <div><span class="block text-slate-500">Touches</span><span class="text-slate-200">{{ hoveredFile.touches }}</span></div>
            <div><span class="block text-slate-500">Fix signals</span><span class="text-slate-200">{{ hoveredFile.fixTouches }}</span></div>
          </div>
          <p class="mt-2 truncate text-[10px] text-slate-400">
            {{ hoveredFile.lastAuthor ? `${hoveredFile.lastAuthor}, ${timeAgo(hoveredFile.lastChangedAt)}` : "Outside sampled history" }}
          </p>
          <p v-if="hoveredChildFolder" class="mt-2 text-[10px] font-medium text-teal-300">
            Click to open {{ hoveredChildFolder }}
          </p>
        </div>
        <div
          v-else-if="hoveredDistrict"
          class="pointer-events-none absolute z-20 w-56 rounded-md border border-slate-700 bg-slate-950/95 p-3 shadow-xl"
          :style="{ left: `${Math.max(8, Math.min(tooltip.x, canvasWidth - 240))}px`, top: `${Math.max(8, Math.min(tooltip.y, canvasHeight - 120))}px` }"
        >
          <p class="truncate text-xs font-semibold text-slate-100">{{ hoveredDistrict.name }}</p>
          <p class="mt-1 truncate text-[10px] text-slate-400">{{ hoveredDistrict.path }}</p>
          <p class="mt-2 text-[10px] font-medium text-teal-300">Click to open folder</p>
        </div>
        <div
          v-else-if="hoveredBoat"
          class="pointer-events-none absolute z-20 w-56 rounded-md border border-cyan-400/25 bg-slate-950/95 p-3 shadow-xl"
          :style="{ left: `${Math.max(8, Math.min(tooltip.x, canvasWidth - 240))}px`, top: `${Math.max(8, Math.min(tooltip.y, canvasHeight - 120))}px` }"
        >
          <p class="flex items-center gap-2 truncate text-xs font-semibold text-slate-100">
            <Ship class="h-3.5 w-3.5 text-cyan-300" />
            {{ hoveredBoat.name }}
          </p>
          <p class="mt-1 text-[10px] capitalize text-slate-400">{{ hoveredBoat.kind }}</p>
          <p class="mt-2 text-[10px] font-medium text-cyan-300">Click to pilot, Ctrl to aim cannons</p>
        </div>

        <div class="pointer-events-none absolute bottom-3 right-3 flex items-center gap-3 rounded-md bg-slate-950/75 px-3 py-2 text-[9px] text-slate-300">
          <span class="flex items-center gap-1"><i class="h-2 w-2 rounded-sm bg-emerald-400" /> Calm</span>
          <span class="flex items-center gap-1"><i class="h-2 w-2 rounded-sm bg-sky-400" /> Active</span>
          <span class="flex items-center gap-1"><i class="h-2 w-2 rounded-sm bg-amber-400" /> Busy</span>
          <span class="flex items-center gap-1"><i class="h-2 w-2 rounded-sm bg-rose-500" /> Hotspot</span>
        </div>
      </div>

      <aside v-if="showRadar" class="hidden w-64 shrink-0 overflow-y-auto border-l border-[var(--border)] bg-[var(--card)] xl:block no-scrollbar">
        <div class="border-b border-[var(--border)] px-4 py-3">
          <div class="flex items-center justify-between">
            <h3 class="text-xs font-semibold text-[var(--foreground)]">Team Radar</h3>
            <span class="text-[9px] uppercase text-[var(--muted-foreground)]">Branch activity</span>
          </div>
          <div class="mt-3 grid grid-cols-3 gap-2 text-center">
            <div>
              <strong class="block text-sm text-[var(--foreground)]">{{ snapshot?.contributors.length || 0 }}</strong>
              <span class="text-[9px] text-[var(--muted-foreground)]">People</span>
            </div>
            <div>
              <strong class="block text-sm text-rose-400">{{ hotFileCount }}</strong>
              <span class="text-[9px] text-[var(--muted-foreground)]">Hotspots</span>
            </div>
            <div>
              <strong class="block text-sm text-[var(--foreground)]">{{ snapshot?.sampledCommits || 0 }}</strong>
              <span class="text-[9px] text-[var(--muted-foreground)]">Commits</span>
            </div>
          </div>
        </div>

        <button
          v-for="marker in radarMarkers"
          :key="marker.id"
          type="button"
          class="flex w-full items-center gap-3 border-b border-[var(--border)]/60 px-4 py-3 text-left transition-colors hover:bg-[var(--secondary)]"
          @click="focusBuilding(marker.path)"
        >
          <span
            class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-[10px] font-bold text-slate-950"
            :style="{ backgroundColor: marker.color }"
          >
            {{ marker.name.split(/\s+/).map((part) => part[0]).join("").slice(0, 2).toUpperCase() }}
          </span>
          <span class="min-w-0 flex-1">
            <span class="block truncate text-xs font-medium text-[var(--foreground)]">{{ marker.name }}</span>
            <span class="block truncate text-[9px] text-[var(--muted-foreground)]">{{ marker.path }}</span>
          </span>
          <span
            class="shrink-0 text-[9px]"
            :class="marker.source === 'local' ? 'text-teal-400' : 'text-[var(--muted-foreground)]'"
          >
            {{ marker.source === "local" ? "Now" : timeAgo(marker.activeAt) }}
          </span>
        </button>

        <div v-if="radarMarkers.length === 0" class="px-4 py-8 text-center text-xs text-[var(--muted-foreground)]">
          No activity in this history window.
        </div>
      </aside>
    </div>

    <footer class="flex h-7 shrink-0 items-center justify-between border-t border-[var(--border)] px-3 text-[9px] text-[var(--muted-foreground)]">
      <span>{{ selectedRef }} · {{ snapshot?.headSha.slice(0, 8) || "loading" }}</span>
      <span v-if="cameraMode === 'boat'">
        Piloting {{ activeBoat?.name || "boat" }} · WASD to drive · Ctrl hold to aim right cannons · mouse up/down to aim · release/click to fire
      </span>
      <span v-else>
        {{ snapshot?.omittedFiles ? `${snapshot.omittedFiles.toLocaleString()} quiet files aggregated for performance` : "Complete branch snapshot" }}
        · WASD to move · drag to look · Shift for faster walking
      </span>
    </footer>
  </section>
</template>

<style scoped>
.city-icon-button {
  display: inline-flex;
  height: 2rem;
  width: 2rem;
  align-items: center;
  justify-content: center;
  border-radius: 0.375rem;
  border: 1px solid var(--border);
  color: var(--muted-foreground);
  transition: color 120ms ease, background-color 120ms ease;
}

.city-icon-button:hover {
  background: var(--secondary);
  color: var(--foreground);
}

.city-icon-button:disabled {
  cursor: wait;
  opacity: 0.55;
}

.no-scrollbar {
  scrollbar-width: none;
}

.no-scrollbar::-webkit-scrollbar {
  display: none;
}
</style>

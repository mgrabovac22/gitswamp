import type {
  CityBuilding,
  CityCamera,
  CityDistrict,
  CityPark,
  CityRadarMarker,
  CityRoad,
  CityScene,
  ProjectedCityBuilding,
  RepositoryCityFile,
  ScreenPoint,
} from "./repositoryCity.types";

const DISTRICT_GAP = 36;
const ROAD_WIDTH = 12;
const BUILDING_GAP = 3;
const DISTRICT_ROAD_CLEARANCE = 7;
const CITY_EDGE_PADDING = ROAD_WIDTH + DISTRICT_ROAD_CLEARANCE + 16;

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

function createDistrictBuildings(
  files: RepositoryCityFile[],
  districtId: string,
  originX: number,
  originY: number,
): { buildings: CityBuilding[]; parks: CityPark[]; width: number; depth: number } {
  const count = files.length;
  const columns = Math.max(1, Math.ceil(Math.sqrt(count * 1.2)));
  const cell = count > 800 ? 8 : count > 300 ? 10 : count > 100 ? 13 : 17;
  const rows = Math.max(1, Math.ceil(count / columns));
  const width = columns * cell + ROAD_WIDTH * 2;
  const depth = rows * cell + ROAD_WIDTH * 2;
  const ordered = [...files].sort((a, b) => hashText(a.path) - hashText(b.path));
  const cells = Array.from({ length: rows * columns }, (_, index) => index)
    .sort((a, b) => hashText(`${districtId}:cell:${a}`) - hashText(`${districtId}:cell:${b}`));
  const usedCells = new Set<number>();
  const buildings = ordered.map((file, index) => {
    const cellIndex = cells[index] ?? index;
    usedCells.add(cellIndex);
    const column = cellIndex % columns;
    const row = Math.floor(cellIndex / columns);
    const footprint = Math.max(3.5, cell - BUILDING_GAP);
    const jitterSeed = hashText(`${districtId}:${file.path}:jitter`);
    const jitterRange = Math.max(0, (cell - footprint) * 0.42);
    const jitterX = (((jitterSeed & 255) / 255) - 0.5) * jitterRange;
    const jitterY = ((((jitterSeed >>> 8) & 255) / 255) - 0.5) * jitterRange;
    return {
      file,
      districtId,
      x: originX + ROAD_WIDTH + column * cell + jitterX,
      y: originY + ROAD_WIDTH + row * cell + jitterY,
      width: footprint,
      depth: footprint,
      height: clamp(8 + Math.log2(Math.max(1, file.size) + 1) * 3.7, 8, 74),
    };
  });

  const freeCells = cells.filter((cellIndex) => !usedCells.has(cellIndex));
  const parks: CityPark[] = [];
  if (freeCells.length > 0 && columns >= 2) {
    const parkSeed = hashText(`${districtId}:park`);
    const cellIndex = freeCells[parkSeed % freeCells.length];
    const column = cellIndex % columns;
    const row = Math.floor(cellIndex / columns);
    const parkColumns = Math.min(2, columns - column);
    const parkRows = Math.min(2, rows - row);
    parks.push({
      id: `${districtId}-park-0`,
      x: originX + ROAD_WIDTH + column * cell + 1,
      y: originY + ROAD_WIDTH + row * cell + 1,
      width: Math.max(4, parkColumns * cell - BUILDING_GAP),
      depth: Math.max(4, parkRows * cell - BUILDING_GAP),
      treeCount: clamp(Math.ceil((parkColumns * parkRows + freeCells.length) / 2), 2, 7),
    });
  }

  return { buildings, parks, width, depth };
}

function addRoad(
  roads: CityRoad[],
  id: string,
  x: number,
  y: number,
  width: number,
  depth: number,
  kind: CityRoad["kind"],
) {
  if (width < 2 || depth < 2) return;
  roads.push({ id, x, y, width, depth, kind });
}

function rectsOverlap(
  a: Pick<CityRoad, "x" | "y" | "width" | "depth">,
  b: Pick<CityRoad, "x" | "y" | "width" | "depth">,
  padding = 0,
): boolean {
  return a.x < b.x + b.width + padding
    && a.x + a.width > b.x - padding
    && a.y < b.y + b.depth + padding
    && a.y + a.depth > b.y - padding;
}

function roadHitsDistrictInterior(district: CityDistrict, road: Pick<CityRoad, "x" | "y" | "width" | "depth">): boolean {
  return rectsOverlap(road, district, -1.2);
}

function addPerimeterRoads(roads: CityRoad[], district: CityDistrict) {
  const outerX = district.x - DISTRICT_ROAD_CLEARANCE - ROAD_WIDTH;
  const outerY = district.y - DISTRICT_ROAD_CLEARANCE - ROAD_WIDTH;
  const outerWidth = district.width + (DISTRICT_ROAD_CLEARANCE + ROAD_WIDTH) * 2;
  const outerDepth = district.depth + (DISTRICT_ROAD_CLEARANCE + ROAD_WIDTH) * 2;
  addRoad(roads, `${district.id}-road-top`, outerX, outerY, outerWidth, ROAD_WIDTH, "district");
  addRoad(
    roads,
    `${district.id}-road-bottom`,
    outerX,
    district.y + district.depth + DISTRICT_ROAD_CLEARANCE,
    outerWidth,
    ROAD_WIDTH,
    "district",
  );
  addRoad(roads, `${district.id}-road-left`, outerX, outerY, ROAD_WIDTH, outerDepth, "district");
  addRoad(
    roads,
    `${district.id}-road-right`,
    district.x + district.width + DISTRICT_ROAD_CLEARANCE,
    outerY,
    ROAD_WIDTH,
    outerDepth,
    "district",
  );
}

function addConnectorRoads(
  roads: CityRoad[],
  districts: CityDistrict[],
  id: string,
  from: { x: number; y: number },
  to: { x: number; y: number },
) {
  const halfRoad = ROAD_WIDTH / 2;
  const makeHorizontal = (x1: number, x2: number, y: number): Pick<CityRoad, "x" | "y" | "width" | "depth"> => ({
    x: Math.min(x1, x2) - halfRoad,
    y: y - halfRoad,
    width: Math.abs(x1 - x2) + ROAD_WIDTH,
    depth: ROAD_WIDTH,
  });
  const makeVertical = (x: number, y1: number, y2: number): Pick<CityRoad, "x" | "y" | "width" | "depth"> => ({
    x: x - halfRoad,
    y: Math.min(y1, y2) - halfRoad,
    width: ROAD_WIDTH,
    depth: Math.abs(y1 - y2) + ROAD_WIDTH,
  });
  const outsideX = from.x < to.x
    ? Math.min(from.x, to.x) - ROAD_WIDTH * 2.5
    : Math.max(from.x, to.x) + ROAD_WIDTH * 2.5;
  const outsideY = from.y < to.y
    ? Math.min(from.y, to.y) - ROAD_WIDTH * 2.5
    : Math.max(from.y, to.y) + ROAD_WIDTH * 2.5;
  const horizontalFirst = Math.abs(from.x - to.x) >= Math.abs(from.y - to.y);
  const routes: Array<Array<Pick<CityRoad, "x" | "y" | "width" | "depth">>> = horizontalFirst
    ? [
      [makeHorizontal(from.x, to.x, from.y), makeVertical(to.x, from.y, to.y)],
      [makeVertical(from.x, from.y, to.y), makeHorizontal(from.x, to.x, to.y)],
      [makeVertical(from.x, from.y, outsideY), makeHorizontal(from.x, to.x, outsideY), makeVertical(to.x, outsideY, to.y)],
      [makeHorizontal(from.x, outsideX, from.y), makeVertical(outsideX, from.y, to.y), makeHorizontal(outsideX, to.x, to.y)],
    ]
    : [
      [makeVertical(from.x, from.y, to.y), makeHorizontal(from.x, to.x, to.y)],
      [makeHorizontal(from.x, to.x, from.y), makeVertical(to.x, from.y, to.y)],
      [makeHorizontal(from.x, outsideX, from.y), makeVertical(outsideX, from.y, to.y), makeHorizontal(outsideX, to.x, to.y)],
      [makeVertical(from.x, from.y, outsideY), makeHorizontal(from.x, to.x, outsideY), makeVertical(to.x, outsideY, to.y)],
    ];

  const chosen = routes.find((route) =>
    !districts.some((district) => route.some((segment) => roadHitsDistrictInterior(district, segment))),
  ) ?? routes[0];
  chosen.forEach((segment, index) => {
    addRoad(roads, `${id}-${index}`, segment.x, segment.y, segment.width, segment.depth, "connector");
  });
}

function connectionAnchor(district: CityDistrict, target: { x: number; y: number }): { x: number; y: number } {
  const centerX = district.x + district.width / 2;
  const centerY = district.y + district.depth / 2;
  const dx = target.x - centerX;
  const dy = target.y - centerY;
  const offset = DISTRICT_ROAD_CLEARANCE + ROAD_WIDTH / 2;
  const minX = district.x - offset;
  const maxX = district.x + district.width + offset;
  const minY = district.y - offset;
  const maxY = district.y + district.depth + offset;

  if (Math.abs(dx) >= Math.abs(dy)) {
    return {
      x: dx >= 0 ? maxX : minX,
      y: clamp(target.y, minY, maxY),
    };
  }

  return {
    x: clamp(target.x, minX, maxX),
    y: dy >= 0 ? maxY : minY,
  };
}

function buildDistrictRoads(districts: CityDistrict[]): CityRoad[] {
  const roads: CityRoad[] = [];
  districts.forEach((district) => addPerimeterRoads(roads, district));

  const centers = districts.map((district) => ({
    id: district.id,
    x: district.x + district.width / 2,
    y: district.y + district.depth / 2,
    district,
  }));
  const orderedCenters = centers.slice().sort((a, b) => hashText(a.id) - hashText(b.id));
  orderedCenters.forEach((center, index) => {
    if (index === 0) return;
    let target = orderedCenters[0];
    let bestDistance = Number.POSITIVE_INFINITY;
    for (let candidateIndex = 0; candidateIndex < index; candidateIndex += 1) {
      const candidate = orderedCenters[candidateIndex];
      const distance = Math.abs(center.x - candidate.x) + Math.abs(center.y - candidate.y);
      if (distance < bestDistance) {
        bestDistance = distance;
        target = candidate;
      }
    }
    const fromAnchor = connectionAnchor(center.district, target);
    const toAnchor = connectionAnchor(target.district, center);
    addConnectorRoads(roads, districts, `connector-${index}`, fromAnchor, toAnchor);
  });

  return roads;
}

function addOpenSpaceParks(
  parks: CityPark[],
  districts: CityDistrict[],
  roads: CityRoad[],
  worldWidth: number,
  worldDepth: number,
) {
  const targetCount = clamp(Math.ceil(districts.length * 1.45), 8, 54);
  const cellSize = clamp(Math.sqrt((worldWidth * worldDepth) / Math.max(8, targetCount)) * 0.72, 48, 96);
  const occupied = [
    ...districts.map((district) => ({
      x: district.x - 5,
      y: district.y - 5,
      width: district.width + 10,
      depth: district.depth + 10,
    })),
    ...roads.map((road) => ({
      x: road.x - 4,
      y: road.y - 4,
      width: road.width + 8,
      depth: road.depth + 8,
    })),
  ];
  let parkIndex = 0;
  for (let y = CITY_EDGE_PADDING; y < worldDepth - CITY_EDGE_PADDING && parkIndex < targetCount; y += cellSize) {
    for (let x = CITY_EDGE_PADDING; x < worldWidth - CITY_EDGE_PADDING && parkIndex < targetCount; x += cellSize) {
      const seed = hashText(`city-open-park:${parkIndex}:${Math.round(x)}:${Math.round(y)}`);
      if (seed % 5 === 0) continue;
      const width = clamp(cellSize * (0.34 + ((seed >>> 8) % 28) / 100), 22, 58);
      const depth = clamp(cellSize * (0.32 + ((seed >>> 16) % 30) / 100), 20, 56);
      const candidate = {
        id: `city-open-park-${parkIndex}`,
        x: clamp(x + ((seed >>> 22) % 18) - 9, CITY_EDGE_PADDING, Math.max(CITY_EDGE_PADDING, worldWidth - CITY_EDGE_PADDING - width)),
        y: clamp(y + ((seed >>> 27) % 18) - 9, CITY_EDGE_PADDING, Math.max(CITY_EDGE_PADDING, worldDepth - CITY_EDGE_PADDING - depth)),
        width,
        depth,
        treeCount: clamp(Math.ceil((width * depth) / 82), 8, 24),
      };
      if (occupied.some((rect) => rectsOverlap(candidate, rect, 3))) continue;
      parks.push(candidate);
      occupied.push(candidate);
      parkIndex += 1;
    }
  }
}

export function buildRepositoryCityScene(files: RepositoryCityFile[]): CityScene {
  const byFolder = new Map<string, RepositoryCityFile[]>();
  for (const file of files) {
    const rows = byFolder.get(file.folder) ?? [];
    rows.push(file);
    byFolder.set(file.folder, rows);
  }

  const groups = Array.from(byFolder.entries())
    .sort((a, b) => hashText(a[0]) - hashText(b[0]));
  const previews = groups.map(([name, districtFiles], index) => ({
    name,
    districtFiles,
    id: `district-${index}`,
    preview: createDistrictBuildings(districtFiles, `district-${index}`, 0, 0),
  }));
  const totalArea = previews.reduce(
    (sum, item) => sum + (item.preview.width + DISTRICT_GAP) * (item.preview.depth + DISTRICT_GAP),
    0,
  );
  const targetRowWidth = Math.max(420, Math.sqrt(Math.max(1, totalArea)) * 1.08);
  const rows: Array<{
    items: typeof previews;
    width: number;
    depth: number;
  }> = [];
  let activeRow: typeof previews = [];
  let activeWidth = 0;
  let activeDepth = 0;

  for (const item of previews) {
    const nextWidth = activeWidth === 0
      ? item.preview.width
      : activeWidth + DISTRICT_GAP + item.preview.width;
    if (activeRow.length > 0 && nextWidth > targetRowWidth) {
      rows.push({ items: activeRow, width: activeWidth, depth: activeDepth });
      activeRow = [];
      activeWidth = 0;
      activeDepth = 0;
    }
    activeRow.push(item);
    activeWidth = activeWidth === 0 ? item.preview.width : activeWidth + DISTRICT_GAP + item.preview.width;
    activeDepth = Math.max(activeDepth, item.preview.depth);
  }
  if (activeRow.length > 0) rows.push({ items: activeRow, width: activeWidth, depth: activeDepth });

  const widestRow = rows.reduce((value, row) => Math.max(value, row.width), 0);
  const districts: CityDistrict[] = [];
  const buildings: CityBuilding[] = [];
  const parks: CityPark[] = [];
  let cursorY = CITY_EDGE_PADDING;

  rows.forEach((row, rowIndex) => {
    const freeSpace = Math.max(0, widestRow - row.width);
    const rowJitter = (((hashText(`city-row:${rowIndex}`) % 1000) / 1000) - 0.5)
      * Math.min(18, freeSpace * 0.18);
    let cursorX = CITY_EDGE_PADDING + freeSpace / 2 + rowJitter;
    row.items.forEach((item) => {
      const placed = createDistrictBuildings(item.districtFiles, item.id, cursorX, cursorY);
      const averageHeat = item.districtFiles.reduce((sum, file) => sum + file.heat, 0)
        / Math.max(1, item.districtFiles.length);
      districts.push({
        id: item.id,
        name: item.name,
        x: cursorX,
        y: cursorY,
        width: placed.width,
        depth: placed.depth,
        fileCount: item.districtFiles.length,
        averageHeat,
      });
      buildings.push(...placed.buildings);
      parks.push(...placed.parks);
      cursorX += placed.width + DISTRICT_GAP;
    });
    cursorY += row.depth + DISTRICT_GAP;
  });

  const worldWidth = Math.max(1, widestRow + CITY_EDGE_PADDING * 2);
  const worldDepth = Math.max(1, cursorY - DISTRICT_GAP + CITY_EDGE_PADDING);
  const roads = buildDistrictRoads(districts);
  addOpenSpaceParks(parks, districts, roads, worldWidth, worldDepth);
  return {
    districts,
    buildings,
    roads,
    parks,
    buildingByPath: new Map(buildings.map((building) => [building.file.path, building])),
    width: worldWidth,
    depth: worldDepth,
  };
}

export function projectCityPoint(
  x: number,
  y: number,
  z: number,
  camera: CityCamera,
  canvasWidth: number,
  canvasHeight: number,
  worldWidth: number,
  worldDepth: number,
): ScreenPoint {
  const centeredX = x - worldWidth / 2;
  const centeredY = y - worldDepth / 2;
  const cos = Math.cos(camera.rotation);
  const sin = Math.sin(camera.rotation);
  const rotatedX = centeredX * cos - centeredY * sin;
  const rotatedY = centeredX * sin + centeredY * cos;
  const groundScale = camera.mode === "bird" ? 0.78 : 0.43;
  const heightScale = camera.mode === "bird" ? 0.38 : 0.9;
  return {
    x: canvasWidth / 2 + camera.panX + rotatedX * camera.zoom,
    y: canvasHeight / 2 + camera.panY + (rotatedY * groundScale - z * heightScale) * camera.zoom,
  };
}

function buildingProjection(
  building: CityBuilding,
  camera: CityCamera,
  canvasWidth: number,
  canvasHeight: number,
  scene: CityScene,
): ProjectedCityBuilding {
  const point = (x: number, y: number, z: number) =>
    projectCityPoint(x, y, z, camera, canvasWidth, canvasHeight, scene.width, scene.depth);
  const x1 = building.x;
  const y1 = building.y;
  const x2 = x1 + building.width;
  const y2 = y1 + building.depth;
  const z = building.height;
  const groundA = point(x1, y1, 0);
  const groundB = point(x2, y1, 0);
  const groundC = point(x2, y2, 0);
  const groundD = point(x1, y2, 0);
  const topA = point(x1, y1, z);
  const topB = point(x2, y1, z);
  const topC = point(x2, y2, z);
  const topD = point(x1, y2, z);
  return {
    building,
    top: [topA, topB, topC, topD],
    left: [groundD, groundA, topA, topD],
    right: [groundB, groundC, topC, topB],
    anchor: point(x1 + building.width / 2, y1 + building.depth / 2, z),
    depth: groundA.y + groundB.y + groundC.y + groundD.y,
  };
}

function colorForHeat(heat: number): { top: string; left: string; right: string; glow: string } {
  const value = clamp(heat, 0, 1);
  if (value >= 0.78) {
    return { top: "#f43f5e", left: "#8f1d3b", right: "#be2748", glow: "rgba(244, 63, 94, 0.28)" };
  }
  if (value >= 0.52) {
    return { top: "#f59e0b", left: "#8d5408", right: "#bd7008", glow: "rgba(245, 158, 11, 0.18)" };
  }
  if (value >= 0.24) {
    return { top: "#38bdf8", left: "#12638a", right: "#1887b7", glow: "rgba(56, 189, 248, 0.12)" };
  }
  return { top: "#34d399", left: "#176d55", right: "#209c76", glow: "rgba(52, 211, 153, 0.08)" };
}

function drawPolygon(
  context: CanvasRenderingContext2D,
  points: ScreenPoint[],
  fill: string,
  stroke = "rgba(255,255,255,0.08)",
) {
  context.beginPath();
  context.moveTo(points[0].x, points[0].y);
  for (let index = 1; index < points.length; index += 1) {
    context.lineTo(points[index].x, points[index].y);
  }
  context.closePath();
  context.fillStyle = fill;
  context.fill();
  context.strokeStyle = stroke;
  context.lineWidth = 0.65;
  context.stroke();
}

function drawFacadeDetails(
  context: CanvasRenderingContext2D,
  item: ProjectedCityBuilding,
  zoom: number,
) {
  if (zoom < 0.8 || item.building.height < 22) return;
  const floorCount = Math.min(5, Math.max(1, Math.floor(item.building.height / 14)));
  context.save();
  context.strokeStyle = "rgba(186, 230, 253, 0.25)";
  context.lineWidth = 0.6;
  for (let floor = 1; floor <= floorCount; floor += 1) {
    const ratio = floor / (floorCount + 1);
    const leftStart = {
      x: item.left[0].x + (item.left[3].x - item.left[0].x) * ratio,
      y: item.left[0].y + (item.left[3].y - item.left[0].y) * ratio,
    };
    const leftEnd = {
      x: item.left[1].x + (item.left[2].x - item.left[1].x) * ratio,
      y: item.left[1].y + (item.left[2].y - item.left[1].y) * ratio,
    };
    const rightStart = {
      x: item.right[0].x + (item.right[3].x - item.right[0].x) * ratio,
      y: item.right[0].y + (item.right[3].y - item.right[0].y) * ratio,
    };
    const rightEnd = {
      x: item.right[1].x + (item.right[2].x - item.right[1].x) * ratio,
      y: item.right[1].y + (item.right[2].y - item.right[1].y) * ratio,
    };
    context.beginPath();
    context.moveTo(leftStart.x, leftStart.y);
    context.lineTo(leftEnd.x, leftEnd.y);
    context.moveTo(rightStart.x, rightStart.y);
    context.lineTo(rightEnd.x, rightEnd.y);
    context.stroke();
  }
  context.restore();
}

function drawDistrict(
  context: CanvasRenderingContext2D,
  district: CityDistrict,
  scene: CityScene,
  camera: CityCamera,
  canvasWidth: number,
  canvasHeight: number,
  showLabels: boolean,
) {
  const corners = [
    projectCityPoint(district.x, district.y, 0, camera, canvasWidth, canvasHeight, scene.width, scene.depth),
    projectCityPoint(district.x + district.width, district.y, 0, camera, canvasWidth, canvasHeight, scene.width, scene.depth),
    projectCityPoint(district.x + district.width, district.y + district.depth, 0, camera, canvasWidth, canvasHeight, scene.width, scene.depth),
    projectCityPoint(district.x, district.y + district.depth, 0, camera, canvasWidth, canvasHeight, scene.width, scene.depth),
  ];
  drawPolygon(context, corners, "rgba(18, 27, 41, 0.92)", district.averageHeat > 0.65
    ? "rgba(244, 63, 94, 0.5)"
    : "rgba(45, 212, 191, 0.19)");

  if (!showLabels || camera.zoom < 0.52) return;
  const label = projectCityPoint(
    district.x + 6,
    district.y + 7,
    1,
    camera,
    canvasWidth,
    canvasHeight,
    scene.width,
    scene.depth,
  );
  context.font = "600 10px ui-sans-serif, system-ui, sans-serif";
  context.fillStyle = "rgba(226, 232, 240, 0.82)";
  context.fillText(`${district.name}  ${district.fileCount}`, label.x, label.y);
}

function pointInPolygon(point: ScreenPoint, polygon: ScreenPoint[]): boolean {
  let inside = false;
  for (let current = 0, previous = polygon.length - 1; current < polygon.length; previous = current++) {
    const a = polygon[current];
    const b = polygon[previous];
    const intersects = ((a.y > point.y) !== (b.y > point.y))
      && point.x < ((b.x - a.x) * (point.y - a.y)) / (b.y - a.y || 0.0001) + a.x;
    if (intersects) inside = !inside;
  }
  return inside;
}

export function hitTestCity(
  projected: ProjectedCityBuilding[],
  x: number,
  y: number,
): ProjectedCityBuilding | null {
  const point = { x, y };
  for (let index = projected.length - 1; index >= 0; index -= 1) {
    const item = projected[index];
    if (
      pointInPolygon(point, item.top)
      || pointInPolygon(point, item.left)
      || pointInPolygon(point, item.right)
    ) {
      return item;
    }
  }
  return null;
}

export function drawRepositoryCity(
  context: CanvasRenderingContext2D,
  canvasWidth: number,
  canvasHeight: number,
  scene: CityScene,
  camera: CityCamera,
  options: {
    selectedPath: string;
    hoveredPath: string;
    showLabels: boolean;
    showHeat: boolean;
    showMiniMap: boolean;
    radarMarkers: CityRadarMarker[];
  },
): ProjectedCityBuilding[] {
  context.clearRect(0, 0, canvasWidth, canvasHeight);
  context.fillStyle = "#0b1018";
  context.fillRect(0, 0, canvasWidth, canvasHeight);

  const gridSpacing = Math.max(28, 54 * camera.zoom);
  context.strokeStyle = "rgba(77, 99, 129, 0.08)";
  context.lineWidth = 1;
  for (let x = (camera.panX % gridSpacing); x < canvasWidth; x += gridSpacing) {
    context.beginPath();
    context.moveTo(x, 0);
    context.lineTo(x, canvasHeight);
    context.stroke();
  }
  for (let y = (camera.panY % gridSpacing); y < canvasHeight; y += gridSpacing) {
    context.beginPath();
    context.moveTo(0, y);
    context.lineTo(canvasWidth, y);
    context.stroke();
  }

  for (const district of scene.districts) {
    drawDistrict(context, district, scene, camera, canvasWidth, canvasHeight, options.showLabels);
  }

  const projected = scene.buildings
    .filter((building) => {
      const center = projectCityPoint(
        building.x + building.width / 2,
        building.y + building.depth / 2,
        building.height / 2,
        camera,
        canvasWidth,
        canvasHeight,
        scene.width,
        scene.depth,
      );
      const margin = 140 + building.height * camera.zoom;
      return center.x >= -margin
        && center.x <= canvasWidth + margin
        && center.y >= -margin
        && center.y <= canvasHeight + margin;
    })
    .map((building) => buildingProjection(building, camera, canvasWidth, canvasHeight, scene))
    .sort((a, b) => a.depth - b.depth);

  for (const item of projected) {
    const heat = options.showHeat ? item.building.file.heat : 0.34;
    const colors = colorForHeat(heat);
    if (heat >= 0.78 && camera.zoom >= 0.45) {
      context.save();
      context.shadowBlur = 13;
      context.shadowColor = colors.glow;
      drawPolygon(context, item.top, colors.top);
      context.restore();
    }
    drawPolygon(context, item.left, colors.left);
    drawPolygon(context, item.right, colors.right);
    drawPolygon(context, item.top, colors.top);
    drawFacadeDetails(context, item, camera.zoom);

    if (item.building.file.path === options.selectedPath || item.building.file.path === options.hoveredPath) {
      context.save();
      context.strokeStyle = item.building.file.path === options.selectedPath ? "#ffffff" : "#7dd3fc";
      context.lineWidth = 1.8;
      context.beginPath();
      context.moveTo(item.top[0].x, item.top[0].y);
      item.top.slice(1).forEach((point) => context.lineTo(point.x, point.y));
      context.closePath();
      context.stroke();
      context.restore();
    }
  }

  for (const marker of options.radarMarkers) {
    const building = scene.buildingByPath.get(marker.path);
    if (!building) continue;
    const anchor = projectCityPoint(
      building.x + building.width / 2,
      building.y + building.depth / 2,
      building.height + 9,
      camera,
      canvasWidth,
      canvasHeight,
      scene.width,
      scene.depth,
    );
    context.save();
    context.fillStyle = marker.color;
    context.strokeStyle = marker.source === "local" ? "#ffffff" : "rgba(255,255,255,0.55)";
    context.lineWidth = marker.source === "local" ? 2 : 1;
    context.beginPath();
    context.arc(anchor.x, anchor.y, marker.source === "local" ? 6 : 4.5, 0, Math.PI * 2);
    context.fill();
    context.stroke();
    if (camera.zoom >= 0.7) {
      context.font = "600 9px ui-sans-serif, system-ui, sans-serif";
      context.fillStyle = "rgba(241,245,249,0.9)";
      context.fillText(marker.name, anchor.x + 8, anchor.y + 3);
    }
    context.restore();
  }

  if (options.showMiniMap && canvasWidth >= 620 && canvasHeight >= 420) {
    const mapWidth = 156;
    const mapHeight = 102;
    const mapX = 14;
    const mapY = canvasHeight - mapHeight - 14;
    const scale = Math.min((mapWidth - 18) / scene.width, (mapHeight - 24) / scene.depth);
    context.save();
    context.fillStyle = "rgba(8, 13, 22, 0.9)";
    context.strokeStyle = "rgba(94, 234, 212, 0.25)";
    context.lineWidth = 1;
    context.beginPath();
    context.roundRect(mapX, mapY, mapWidth, mapHeight, 6);
    context.fill();
    context.stroke();
    context.font = "600 8px ui-sans-serif, system-ui, sans-serif";
    context.fillStyle = "rgba(148, 163, 184, 0.85)";
    context.fillText("CITY MAP", mapX + 9, mapY + 13);
    for (const district of scene.districts) {
      context.fillStyle = district.averageHeat > 0.66
        ? "rgba(244, 63, 94, 0.6)"
        : "rgba(56, 189, 248, 0.24)";
      context.fillRect(
        mapX + 9 + district.x * scale,
        mapY + 19 + district.y * scale,
        Math.max(2, district.width * scale),
        Math.max(2, district.depth * scale),
      );
    }
    for (const marker of options.radarMarkers) {
      const building = scene.buildingByPath.get(marker.path);
      if (!building) continue;
      context.fillStyle = marker.color;
      context.beginPath();
      context.arc(
        mapX + 9 + (building.x + building.width / 2) * scale,
        mapY + 19 + (building.y + building.depth / 2) * scale,
        marker.source === "local" ? 2.8 : 1.8,
        0,
        Math.PI * 2,
      );
      context.fill();
    }
    context.restore();
  }

  return projected;
}

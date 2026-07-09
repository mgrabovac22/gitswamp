export type CityCameraMode = "bird" | "landscape" | "walking";
export type CityZoomMode = "center" | "cursor";

export interface RepositoryCityFile {
  path: string;
  folder: string;
  size: number;
  touches: number;
  churn: number;
  fixTouches: number;
  heat: number;
  lastAuthor: string;
  lastAuthorEmail: string;
  lastChangedAt: number;
  lastCommitSha: string;
}

export interface RepositoryCityContributor {
  id: string;
  name: string;
  email: string;
  commits: number;
  touches: number;
  primaryPath: string;
  lastActiveAt: number;
}

export interface RepositoryCitySnapshot {
  repoPath: string;
  refName: string;
  headSha: string;
  files: RepositoryCityFile[];
  contributors: RepositoryCityContributor[];
  sampledCommits: number;
  omittedFiles: number;
  userName: string;
  userEmail: string;
}

export interface CityDistrict {
  id: string;
  name: string;
  path: string;
  x: number;
  y: number;
  width: number;
  depth: number;
  fileCount: number;
  averageHeat: number;
}

export interface CityBuilding {
  file: RepositoryCityFile;
  districtId: string;
  x: number;
  y: number;
  width: number;
  depth: number;
  height: number;
}

export interface CityRoad {
  id: string;
  x: number;
  y: number;
  width: number;
  depth: number;
  kind: "district" | "connector" | "avenue";
}

export interface CityPark {
  id: string;
  x: number;
  y: number;
  width: number;
  depth: number;
  treeCount: number;
}

export interface CityScene {
  districts: CityDistrict[];
  buildings: CityBuilding[];
  roads: CityRoad[];
  parks: CityPark[];
  buildingByPath: Map<string, CityBuilding>;
  width: number;
  depth: number;
}

export interface CityCamera {
  mode: CityCameraMode;
  zoom: number;
  panX: number;
  panY: number;
  rotation: number;
}

export interface ScreenPoint {
  x: number;
  y: number;
}

export interface ProjectedCityBuilding {
  building: CityBuilding;
  top: [ScreenPoint, ScreenPoint, ScreenPoint, ScreenPoint];
  left: [ScreenPoint, ScreenPoint, ScreenPoint, ScreenPoint];
  right: [ScreenPoint, ScreenPoint, ScreenPoint, ScreenPoint];
  anchor: ScreenPoint;
  depth: number;
}

export interface CityRadarMarker {
  id: string;
  name: string;
  email: string;
  path: string;
  source: "local" | "history";
  activeAt: number;
  color: string;
}

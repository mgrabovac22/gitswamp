export interface WorkspaceLocation {
  path: string;
  branch: string;
  isPrimary: boolean;
}

export interface WorkspaceRemovalRequest {
  sourcePath: string;
  location: WorkspaceLocation;
  finish: () => void;
}

export function sameWorkspacePath(left: string, right: string): boolean {
  const normalize = (value: string) => /^[a-z]:[/\\]|^\\\\/i.test(value)
    ? value.replace(/\\/g, "/").replace(/\/$/, "").toLowerCase()
    : value.replace(/\/$/, "");
  return !!left && !!right && normalize(left) === normalize(right);
}

export function isOriginalWorktree(sourcePath: string, location: WorkspaceLocation): boolean {
  return location.isPrimary || sameWorkspacePath(sourcePath, location.path);
}

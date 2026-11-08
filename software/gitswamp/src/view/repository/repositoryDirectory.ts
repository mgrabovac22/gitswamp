import { homeDir, join } from "@tauri-apps/api/path";
import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

const cloneDirectoryKey = "gitswamp-clone-path";

export async function defaultRepositoryDirectory(): Promise<string> {
  const lastClone = safeStorageGet(cloneDirectoryKey)?.trim();
  if (lastClone) return lastClone;
  try {
    return await join(await homeDir(), "Repozitoriji");
  } catch {
    return "";
  }
}

export function rememberCloneDirectory(path: string) {
  if (path.trim()) safeStorageSet(cloneDirectoryKey, path.trim());
}

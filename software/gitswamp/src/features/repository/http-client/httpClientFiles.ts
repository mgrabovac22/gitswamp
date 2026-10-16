import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

export async function saveJsonFile(defaultName: string, content: string): Promise<boolean> {
  const path = await saveDialog({
    defaultPath: defaultName.endsWith(".json") ? defaultName : `${defaultName}.json`,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return false;
  await invoke("write_text_file", { path, content });
  return true;
}

export async function openJsonFile(): Promise<string | null> {
  const path = await openDialog({
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path || typeof path !== "string") return null;
  return invoke<string>("read_text_file", { path });
}

export interface SaveTextFilter {
  name: string;
  extensions: string[];
}

export async function saveTextFile(
  defaultName: string,
  content: string,
  filters: SaveTextFilter[],
  extension: string,
): Promise<boolean> {
  const suffix = extension.startsWith(".") ? extension : `.${extension}`;
  const path = await saveDialog({
    defaultPath: defaultName.endsWith(suffix) ? defaultName : `${defaultName}${suffix}`,
    filters,
  });
  if (!path) return false;
  await invoke("write_text_file", { path, content });
  return true;
}

import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type WindowAction = "minimize" | "maximize" | "close";
export type ResizeDirection =
  | "North" | "NorthEast" | "East" | "SouthEast"
  | "South" | "SouthWest" | "West" | "NorthWest";

export const isDesktopRuntime = () => isTauri();

export async function chooseRecordingPaths(multiple: boolean, filterName: string): Promise<string[]> {
  const selected = await open({
    multiple,
    directory: false,
    filters: [{ name: filterName, extensions: ["ts", "m2ts", "tlv"] }],
  });
  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

export async function chooseDirectory(title: string, defaultPath?: string): Promise<string | null> {
  const selected = await open({ multiple: false, directory: true, title, defaultPath });
  return typeof selected === 'string' ? selected : null;
}

export async function performWindowAction(action: WindowAction) {
  const window = getCurrentWindow();
  if (action === "minimize") return window.minimize();
  if (action === "maximize") return window.toggleMaximize();
  return window.close();
}

export const beginWindowDrag = () => getCurrentWindow().startDragging();

export const beginWindowResize = (direction: ResizeDirection) =>
  getCurrentWindow().startResizeDragging(direction);

export async function subscribeRecordingDrops(onDrop: (path: string) => void) {
  return getCurrentWindow().onDragDropEvent((event) => {
    if (event.payload.type !== "drop") return;
    const source = event.payload.paths.find((path) => /\.(ts|m2ts|tlv)$/i.test(path));
    if (source) onDrop(source);
  });
}

export async function subscribeWindowMovement(onMove: () => void) {
  return getCurrentWindow().onMoved(onMove);
}

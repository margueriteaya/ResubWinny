import type { PreviewCommand } from "../backend";

type PlayerKey = Pick<KeyboardEvent, "code" | "shiftKey" | "repeat" | "altKey" | "ctrlKey" | "metaKey">;

export function playerShortcut(event: PlayerKey): PreviewCommand | null {
  if (event.altKey || event.ctrlKey || event.metaKey) return null;
  if (event.code === "Space") return event.shiftKey || event.repeat ? null : "toggle-pause";
  if (event.code === "ArrowLeft") return event.shiftKey ? "frame-back" : "seek-back";
  if (event.code === "ArrowRight") return event.shiftKey ? "frame-forward" : "seek-forward";
  return null;
}

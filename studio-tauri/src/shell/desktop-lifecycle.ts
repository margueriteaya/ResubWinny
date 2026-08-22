import {
  subscribeRecordingDrops,
  subscribeWindowMovement,
} from "./desktop";

export interface DesktopLifecycleOptions {
  playerRunning: () => boolean;
  onRecordingDrop: (path: string) => void;
  onPlayerCommand: (command: "toggle-pause" | "seek-back" | "seek-forward") => void;
  onSurfaceChange: () => void;
}

export function installDesktopLifecycle(options: DesktopLifecycleOptions): () => void {
  let disposed = false;
  let disposeDrop: (() => void) | undefined;
  let disposeMovement: (() => void) | undefined;
  let surfaceFrame = 0;
  const scheduleSurfaceChange = () => {
    if (surfaceFrame) return;
    surfaceFrame = requestAnimationFrame(() => {
      surfaceFrame = 0;
      options.onSurfaceChange();
    });
  };

  void subscribeRecordingDrops(options.onRecordingDrop).then((dispose) => {
    if (disposed) dispose();
    else disposeDrop = dispose;
  });
  void subscribeWindowMovement(scheduleSurfaceChange).then((dispose) => {
    if (disposed) dispose();
    else disposeMovement = dispose;
  });

  const handlePlayerKey = (event: KeyboardEvent) => {
    const target = event.target as HTMLElement | null;
    if (
      !options.playerRunning() ||
      target?.matches('input, select, textarea, [contenteditable="true"]')
    )
      return;
    const command =
      event.code === "Space"
        ? "toggle-pause"
        : event.code === "ArrowLeft"
          ? "seek-back"
          : event.code === "ArrowRight"
            ? "seek-forward"
            : null;
    if (!command) return;
    event.preventDefault();
    options.onPlayerCommand(command);
  };

  const layoutObserver = new ResizeObserver(scheduleSurfaceChange);
  layoutObserver.observe(document.documentElement);
  window.addEventListener("resize", scheduleSurfaceChange);
  window.addEventListener("scroll", scheduleSurfaceChange, true);
  window.addEventListener("keydown", handlePlayerKey);

  return () => {
    disposed = true;
    disposeDrop?.();
    disposeMovement?.();
    layoutObserver.disconnect();
    window.removeEventListener("resize", scheduleSurfaceChange);
    window.removeEventListener("scroll", scheduleSurfaceChange, true);
    window.removeEventListener("keydown", handlePlayerKey);
    if (surfaceFrame) cancelAnimationFrame(surfaceFrame);
  };
}

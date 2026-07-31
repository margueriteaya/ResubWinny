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

  void subscribeRecordingDrops(options.onRecordingDrop).then((dispose) => {
    if (disposed) dispose();
    else disposeDrop = dispose;
  });
  void subscribeWindowMovement(options.onSurfaceChange).then((dispose) => {
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

  const layoutObserver = new ResizeObserver(options.onSurfaceChange);
  layoutObserver.observe(document.documentElement);
  window.addEventListener("resize", options.onSurfaceChange);
  window.addEventListener("scroll", options.onSurfaceChange, true);
  window.addEventListener("keydown", handlePlayerKey);

  return () => {
    disposed = true;
    disposeDrop?.();
    disposeMovement?.();
    layoutObserver.disconnect();
    window.removeEventListener("resize", options.onSurfaceChange);
    window.removeEventListener("scroll", options.onSurfaceChange, true);
    window.removeEventListener("keydown", handlePlayerKey);
  };
}

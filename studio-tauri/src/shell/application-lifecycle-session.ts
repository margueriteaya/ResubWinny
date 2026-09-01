import type { PreviewCommand, TaskEvent } from "../backend";
import { installDesktopLifecycle } from "./desktop-lifecycle";

type LifecycleHooks = {
  desktopRuntime: boolean;
  subscribeTaskEvents: (handler: (payload: TaskEvent) => void) => Promise<() => void>;
  onTaskEvent: (payload: TaskEvent) => void;
  playerRunning: () => boolean;
  onRecordingDrop: (source: string) => void;
  onPlayerCommand: (command: PreviewCommand) => void;
  onSurfaceChange: () => void;
};

/** Owns desktop event subscriptions and their teardown as one application lifetime. */
export class ApplicationLifecycleSession {
  constructor(private readonly hooks: LifecycleHooks) {}

  async mount() {
    if (!this.hooks.desktopRuntime) return () => {};
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await this.hooks.subscribeTaskEvents(this.hooks.onTaskEvent);
    } catch {
      // Event subscription is best-effort; backend calls still report failures independently.
    }
    const disposeDesktopLifecycle = installDesktopLifecycle({
      playerRunning: this.hooks.playerRunning,
      onRecordingDrop: this.hooks.onRecordingDrop,
      onPlayerCommand: this.hooks.onPlayerCommand,
      onSurfaceChange: this.hooks.onSurfaceChange,
    });
    return () => {
      unlisten?.();
      disposeDesktopLifecycle();
    };
  }
}

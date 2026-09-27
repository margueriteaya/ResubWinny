import type { PreviewSession } from "./preview-session";
import type { ProjectTimeMs } from "./time-mapping";

type TaskTab = "preview" | "events" | "diagnostics";
type PreviewLifetime = Pick<PreviewSession,
  "beginPageTransition" | "isCurrentPageTransition" | "whenStopped" |
  "resumeTime" | "clearResumeTime" | "seekMedia" | "seekProject" |
  "currentIntent" | "isCurrentIntent">;

type Bindings = {
  desktopRuntime: () => boolean;
  tab: () => TaskTab;
  setTab: (tab: TaskTab) => void;
  tasksVisible: () => boolean;
  hasSource: () => boolean;
  running: () => boolean;
  layoutReady: () => Promise<void>;
  start: () => Promise<void>;
  stop: () => Promise<void>;
  onError: (reason: unknown) => void;
};

/** Coordinates host replacement and playback restoration; UI state stays in the shell. */
export class PreviewNavigationSession {
  private readonly preview: PreviewLifetime;
  private readonly bindings: Bindings;

  constructor(preview: PreviewLifetime, bindings: Bindings) {
    this.preview = preview;
    this.bindings = bindings;
  }

  switchTab(next: TaskTab) {
    const b = this.bindings;
    if (next === b.tab()) return;
    const generation = this.preview.beginPageTransition(next !== "preview");
    b.setTab(next);
    if (next !== "preview") {
      void b.stop();
    } else if (b.hasSource()) {
      void this.activate(() => this.preview.isCurrentPageTransition(generation));
    }
  }

  async activate(isCurrent: () => boolean) {
    const b = this.bindings;
    const visible = () => isCurrent() && b.tasksVisible() && b.tab() === "preview";
    await this.preview.whenStopped();
    await b.layoutReady();
    if (!visible()) return;
    const resumeAt = this.preview.resumeTime();
    await b.start();
    if (resumeAt != null && resumeAt > 0 && visible() && b.running()) {
      await this.preview.seekMedia(resumeAt, true, visible);
      if (visible()) this.preview.clearResumeTime();
    }
  }

  async seek(milliseconds: ProjectTimeMs, final = true, intent = this.preview.currentIntent()) {
    const b = this.bindings;
    if (!b.desktopRuntime()) return;
    let restarted = false;
    if (b.tab() !== "preview") {
      const generation = this.preview.beginPageTransition(false);
      b.setTab("preview");
      await b.layoutReady();
      const current = () => this.preview.isCurrentPageTransition(generation)
        && b.tasksVisible() && this.preview.isCurrentIntent(intent);
      if (!current()) return;
      await this.preview.whenStopped();
      if (!current()) return;
      await b.start();
      restarted = true;
    }
    if (!b.running() || !this.preview.isCurrentIntent(intent)) return;
    try {
      await this.preview.seekProject(milliseconds, restarted, final, intent);
    } catch (reason) {
      b.onError(reason);
    }
  }
}

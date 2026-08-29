import {
  backend,
  type PlaybackTimeMapping,
  type PreviewCommand,
  type PreviewRect,
} from "../../backend";
import {
  NativePreviewController,
  type PreviewCallbacks,
} from "./native-preview-controller";
import {
  mediaTimeMs,
  mediaToProjectTime,
  projectTimeMs,
  projectToMediaTime,
  type MediaTimeMs,
  type ProjectTimeMs,
} from "./time-mapping";

type PreviewSessionBindings = {
  desktopRuntime: () => boolean;
  running: () => boolean;
  paused: () => boolean;
  mediaTimeMs: () => MediaTimeMs | null;
  mapping: () => PlaybackTimeMapping;
  setMappings: (draft: PlaybackTimeMapping, applied: PlaybackTimeMapping) => void;
  setMappingBusy: (busy: boolean) => void;
  setTimes: (mediaTimeMs: MediaTimeMs, projectTimeMs: ProjectTimeMs) => void;
  setProjectTime: (projectTimeMs: ProjectTimeMs) => void;
  setRunning: (running: boolean) => void;
  setPaused: (paused: boolean) => void;
  onNotice: (code: string, parameters?: Record<string, unknown>) => void;
  onError: (reason: unknown) => void;
};

type PendingProjectSeek = {
  projectTimeMs: ProjectTimeMs;
  final: boolean;
  intent: number;
  execute: (projectTimeMs: ProjectTimeMs, final: boolean, intent: number) => Promise<void>;
  resolve: () => void;
  reject: (reason: unknown) => void;
};

function settleWithin(operation: Promise<unknown> | null, timeoutMs = 1_500) {
  if (!operation) return Promise.resolve();
  return new Promise<void>((resolve) => {
    const timeout = window.setTimeout(resolve, timeoutMs);
    void operation.then(
      () => { window.clearTimeout(timeout); resolve(); },
      () => { window.clearTimeout(timeout); resolve(); },
    );
  });
}

/**
 * Owns the page-facing native preview lifetime, including serialized stop,
 * latest-wins project seeking, and pause/restore scrub gestures. Rendered UI
 * values remain in Svelte and cross this boundary through explicit bindings.
 */
export class PreviewSession {
  readonly controller = new NativePreviewController();
  private stopPromise: Promise<void> = Promise.resolve();
  private scrubWasPaused: boolean | null = null;
  private scrubInitPromise: Promise<void> | null = null;
  private scrubGesture = 0;
  private seekIntent = 0;
  private seekRunning = false;
  private activeSeekPromise: Promise<void> | null = null;
  private pendingSeek: PendingProjectSeek | null = null;
  private resizeFrame = 0;
  private resizeInFlight = false;
  private resizePending = false;
  private pageGeneration = 0;
  private resumeMediaTimeMs: MediaTimeMs | null = null;

  constructor(
    private readonly host: () => HTMLDivElement | null,
    private readonly bindings: PreviewSessionBindings,
  ) {}

  isRunning() { return this.controller.isRunning(); }
  currentIntent() { return this.seekIntent; }
  isCurrentIntent(intent: number) { return intent === this.seekIntent; }
  whenStopped() { return this.stopPromise; }

  private rect(): PreviewRect {
    const element = this.host();
    if (!element) throw new Error("Native preview host is not mounted.");
    const bounds = element.getBoundingClientRect();
    const scale = window.devicePixelRatio;
    return {
      x: Math.round(bounds.left * scale),
      y: Math.round(bounds.top * scale),
      width: Math.round(bounds.width * scale),
      height: Math.round(bounds.height * scale),
    };
  }

  start(source: string, setMapping: (mapping: PlaybackTimeMapping) => void, callbacks: PreviewCallbacks) {
    return this.controller.start(source, this.rect(), setMapping, callbacks);
  }

  resize() { return this.controller.resize(this.rect()); }
  stop(callbacks: Pick<PreviewCallbacks, "onNotice">) { return this.controller.stop(callbacks); }
  dispose() {
    if (this.resizeFrame) cancelAnimationFrame(this.resizeFrame);
    this.resizeFrame = 0;
    this.resizePending = false;
    return this.controller.dispose();
  }

  queueResize() {
    this.resizePending = true;
    if (this.resizeFrame || this.resizeInFlight) return;
    this.resizeFrame = requestAnimationFrame(() => {
      this.resizeFrame = 0;
      void this.flushResize();
    });
  }

  private async flushResize() {
    if (
      !this.resizePending
      || !this.bindings.desktopRuntime()
      || !this.bindings.running()
      || !this.host()
    ) return;
    this.resizePending = false;
    this.resizeInFlight = true;
    try {
      await this.resize();
    } finally {
      this.resizeInFlight = false;
      if (this.resizePending) this.queueResize();
    }
  }

  beginPageTransition(keepResumeTime: boolean) {
    this.pageGeneration += 1;
    if (keepResumeTime) this.resumeMediaTimeMs = this.bindings.mediaTimeMs();
    return this.pageGeneration;
  }

  isCurrentPageTransition(generation: number) {
    return generation === this.pageGeneration;
  }

  resumeTime() { return this.resumeMediaTimeMs; }
  clearResumeTime() { this.resumeMediaTimeMs = null; }

  async startManaged(
    source: string,
    setMapping: (mapping: PlaybackTimeMapping) => void,
    callbacks: PreviewCallbacks,
  ) {
    if (
      !this.bindings.desktopRuntime()
      || this.bindings.running()
      || !this.host()
    ) return false;
    try {
      const started = await this.start(source, setMapping, callbacks);
      if (!started) return false;
      this.bindings.setRunning(true);
      this.bindings.setPaused(true);
      // start() resolves after the controller's authoritative first poll. If
      // mpv has not published a media sample, initialise only the project
      // cursor and preserve the distinction between "unknown" and media zero.
      if (this.bindings.mediaTimeMs() == null)
        this.bindings.setProjectTime(mediaToProjectTime(mediaTimeMs(0), this.bindings.mapping()));
      return true;
    } catch (reason) {
      this.bindings.onError(reason);
      return false;
    }
  }

  async stopManaged(callbacks: Pick<PreviewCallbacks, "onNotice">) {
    await this.settleGesturesBeforeStop();
    if (!this.bindings.running() && !this.isRunning()) return;
    try {
      if (this.bindings.desktopRuntime()) await this.stop(callbacks);
    } finally {
      // A failed IPC acknowledgement cannot leave the UI attached to the
      // previous page's native child surface.
      this.bindings.setRunning(false);
      this.bindings.setPaused(true);
    }
  }

  applyMapping(mapping: PlaybackTimeMapping) {
    const applied = { ...mapping };
    this.bindings.setMappings({ ...mapping }, applied);
    const media = this.bindings.mediaTimeMs();
    if (media != null)
      this.bindings.setTimes(media, mediaToProjectTime(media, applied));
  }

  async saveMapping(mapping: PlaybackTimeMapping) {
    if (!this.bindings.desktopRuntime()) return;
    this.bindings.setMappingBusy(true);
    try {
      await backend.updatePlaybackTimeMapping({ ...mapping });
      this.applyMapping(mapping);
      this.bindings.onNotice("preview.mappingApplied");
    } catch (reason) {
      this.bindings.onError(reason);
    } finally {
      this.bindings.setMappingBusy(false);
    }
  }

  async command(command: PreviewCommand, label: string) {
    if (!this.bindings.desktopRuntime() || !this.bindings.running()) return;
    try {
      await backend.previewCommand(command);
      this.bindings.onNotice("notice.playerCommand", { label });
    } catch (reason) {
      this.bindings.onError(reason);
    }
  }

  async setVolume(volume: number) {
    if (!this.bindings.desktopRuntime() || !this.bindings.running()) return;
    try {
      await this.controller.setVolume(volume);
    } catch (reason) {
      this.bindings.onError(reason);
    }
  }

  queueStop(stop: () => Promise<void>) {
    this.stopPromise = this.stopPromise
      .catch(() => {})
      .then(stop)
      .catch(this.bindings.onError);
    return this.stopPromise;
  }

  async settleGesturesBeforeStop() {
    const scrubInitialization = this.scrubInitPromise;
    const activeSeek = this.activeSeekPromise;
    this.cancelSeek();
    await Promise.all([settleWithin(scrubInitialization), settleWithin(activeSeek)]);
    this.scrubWasPaused = null;
  }

  cancelSeek() {
    this.seekIntent += 1;
    this.scrubGesture += 1;
    this.pendingSeek?.resolve();
    this.pendingSeek = null;
    this.scrubWasPaused = null;
    if (this.bindings.running()) this.controller.cancelSeek();
  }

  publishProjectTarget(projectTargetMs: ProjectTimeMs, final = false) {
    this.seekIntent += 1;
    const mediaTargetMs = mediaTimeMs(Math.max(0, projectToMediaTime(projectTargetMs, this.bindings.mapping())));
    this.bindings.setTimes(mediaTargetMs, projectTargetMs);
    if (this.bindings.running()) {
      if (final) this.controller.beginSeek();
      else this.controller.beginScrub();
    }
  }

  enqueueProjectSeek(
    projectTimeMs: ProjectTimeMs,
    final: boolean,
    execute: PendingProjectSeek["execute"],
  ) {
    const intent = ++this.seekIntent;
    const operation = new Promise<void>((resolve, reject) => {
      this.pendingSeek?.resolve();
      this.pendingSeek = { projectTimeMs, final, intent, execute, resolve, reject };
    });
    void this.pumpSeek();
    return operation;
  }

  private async pumpSeek() {
    if (this.seekRunning || !this.pendingSeek) return;
    const next = this.pendingSeek;
    this.pendingSeek = null;
    this.seekRunning = true;
    const execution = next.execute(next.projectTimeMs, next.final, next.intent);
    this.activeSeekPromise = execution;
    try {
      await execution;
      next.resolve();
    } catch (reason) {
      next.reject(reason);
    } finally {
      if (this.activeSeekPromise === execution) this.activeSeekPromise = null;
      this.seekRunning = false;
      if (this.pendingSeek) void this.pumpSeek();
    }
  }

  async seekMedia(
    mediaTimeMs: MediaTimeMs,
    waitForReady = false,
    isCurrent: () => boolean = () => true,
  ) {
    if (!this.bindings.desktopRuntime() || !this.bindings.running()) return;
    if (waitForReady) {
      for (let attempt = 0; attempt < 20; attempt += 1) {
        if (!isCurrent()) return;
        if (await backend.getPreviewDuration() != null) break;
        await new Promise((resolve) => window.setTimeout(resolve, 100));
      }
    }
    if (!isCurrent() || !this.bindings.running()) return;
    this.controller.beginSeek();
    this.bindings.setTimes(mediaTimeMs, mediaToProjectTime(mediaTimeMs, this.bindings.mapping()));
    try {
      await backend.seekPreviewAbsolute(mediaTimeMs / 1_000);
    } finally {
      this.controller.finishSeek(mediaTimeMs);
    }
  }

  async seekProject(projectTimeMs: ProjectTimeMs, waitForReady = false, final = true, intent = this.seekIntent) {
    if (!this.bindings.desktopRuntime() || !this.bindings.running() || intent !== this.seekIntent) return;
    if (waitForReady && final) {
      for (let attempt = 0; attempt < 20; attempt += 1) {
        if (intent !== this.seekIntent) return;
        if (await backend.getPreviewDuration() != null) break;
        await new Promise((resolve) => window.setTimeout(resolve, 100));
      }
    }
    if (this.scrubInitPromise) {
      await this.scrubInitPromise;
      if (intent !== this.seekIntent) return;
    }
    if (!final && this.scrubWasPaused === null) {
      await this.prepareScrub();
      if (intent !== this.seekIntent) return;
    } else if (final && this.scrubWasPaused === null) {
      this.controller.beginSeek();
    }
    let mappedMediaTimeMs = this.bindings.mediaTimeMs();
    try {
      const mapped = await backend.seekPreviewProject(projectTimeMs, final);
      mappedMediaTimeMs = mapped == null ? null : mediaTimeMs(mapped);
      if (intent === this.seekIntent && mappedMediaTimeMs != null)
        this.bindings.setTimes(
          mappedMediaTimeMs,
          mediaToProjectTime(mappedMediaTimeMs, this.bindings.mapping()),
        );
    } finally {
      if (final) {
        const wasPaused = this.scrubWasPaused;
        this.scrubWasPaused = null;
        if (wasPaused !== null) {
          try {
            if (!wasPaused) {
              await backend.setPreviewPaused(false);
              if (this.bindings.running()) this.bindings.setPaused(false);
            }
          } finally {
            this.controller.finishScrub(mappedMediaTimeMs);
          }
        } else {
          this.controller.finishSeek(mappedMediaTimeMs);
        }
      }
    }
  }

  private async prepareScrub() {
    if (this.scrubWasPaused !== null) {
      if (this.scrubInitPromise) await this.scrubInitPromise;
      return;
    }
    const gesture = ++this.scrubGesture;
    const wasPaused = this.bindings.paused();
    this.scrubWasPaused = wasPaused;
    this.controller.beginScrub();
    const initialization = (async () => {
      await backend.clearCaptionOverlay().catch(() => {});
      if (!wasPaused) await backend.setPreviewPaused(true);
      if (gesture === this.scrubGesture && this.scrubWasPaused !== null)
        this.bindings.setPaused(true);
    })();
    this.scrubInitPromise = initialization;
    try {
      await initialization;
    } catch (reason) {
      if (gesture === this.scrubGesture) {
        this.scrubWasPaused = null;
        this.controller.finishScrub(this.bindings.mediaTimeMs());
      }
      throw reason;
    } finally {
      if (this.scrubInitPromise === initialization) this.scrubInitPromise = null;
    }
  }
}

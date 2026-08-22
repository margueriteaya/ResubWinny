import {
  backend,
  type BroadcastMetadata,
  type PlaybackTimeMapping,
  type PreviewRect,
} from "../../backend";

type PreviewCallbacks = {
  archivePath: () => string;
  renderBusy: () => boolean;
  setRenderBusy: (value: boolean) => void;
  setProjectTime: (timeMs: number) => void;
  setMediaTime: (timeMs: number | null) => void;
  setDuration: (timeMs: number | null) => void;
  setPaused: (paused: boolean) => void;
  setBroadcastMetadata: (metadata: BroadcastMetadata) => void;
  selectedServiceId: () => number | undefined;
  onError: (reason: unknown) => void;
  onNotice: (code: string) => void;
};

/**
 * Owns the native libmpv session and its deliberately low-frequency caption
 * overlay sync. It never renders media in the WebView; all operations are
 * forwarded to the Tauri service through the typed backend facade.
 */
export class NativePreviewController {
  private running = false;
  private starting = false;
  private generation = 0;
  private syncTimer: ReturnType<typeof setInterval> | undefined;
  private playbackSyncing = false;
  private broadcastSyncing = false;
  private lastBroadcastSyncAt = 0;
  private source = "";
  private rect: PreviewRect | null = null;
  private lastTimeSeconds: number | null = null;
  private lastPaused = true;
  private volume = 100;
  private consecutiveSyncFailures = 0;
  private recovering = false;

  isRunning() {
    return this.running || this.starting;
  }

  async start(
    source: string,
    rect: PreviewRect,
    setMapping: (mapping: PlaybackTimeMapping) => void,
    callbacks: PreviewCallbacks,
  ): Promise<boolean> {
    if (this.running || this.starting) return this.running;
    const generation = ++this.generation;
    this.starting = true;
    try {
      const mapping = await backend.getPlaybackTimeMapping();
      if (generation !== this.generation) return false;
      await backend.startPreview(source, rect);
      if (generation !== this.generation) {
        await backend.stopPreview().catch(() => {});
        return false;
      }
      this.running = true;
      this.source = source;
      this.rect = { ...rect };
      this.lastTimeSeconds = null;
      this.lastPaused = true;
      this.consecutiveSyncFailures = 0;
      this.starting = false;
      this.lastBroadcastSyncAt = 0;
      setMapping(mapping);
      this.stopSync();
      this.syncTimer = setInterval(() => {
        this.scheduleSync(callbacks);
      }, 500);
      this.scheduleSync(callbacks);
      callbacks.onNotice("notice.previewStarted");
      return true;
    } finally {
      if (generation === this.generation && !this.running) this.starting = false;
    }
  }

  async resize(rect: PreviewRect) {
    if (this.running) {
      this.rect = { ...rect };
      await backend.resizePreview(rect);
    }
  }

  async setVolume(volume: number) {
    this.volume = Math.min(100, Math.max(0, volume));
    if (this.running) await backend.setPreviewVolume(this.volume);
  }

  async stop(callbacks: Pick<PreviewCallbacks, "onNotice">) {
    const hadSession = this.running || this.starting;
    ++this.generation;
    this.starting = false;
    if (!hadSession) return;
    // Mark stopped before IPC. A navigation action must not retain a stale
    // session when Windows has already destroyed its native surface.
    this.running = false;
    this.stopSync();
    try {
      await backend.clearCaptionOverlay();
    } catch {
      // The native player may already have closed. Stopping remains idempotent.
    }
    try {
      await backend.stopPreview();
    } finally {
      callbacks.onNotice("notice.previewStopped");
    }
  }

  async dispose() {
    ++this.generation;
    this.starting = false;
    this.stopSync();
    if (!this.running) return;
    try {
      await backend.clearCaptionOverlay();
      await backend.stopPreview();
    } catch {
      // The desktop host can be torn down before libmpv receives this command.
    } finally {
      this.running = false;
    }
  }

  private stopSync() {
    if (this.syncTimer) clearInterval(this.syncTimer);
    this.syncTimer = undefined;
    this.playbackSyncing = false;
    this.broadcastSyncing = false;
    this.lastBroadcastSyncAt = 0;
  }

  private scheduleSync(callbacks: PreviewCallbacks) {
    void this.syncPlaybackState(callbacks);
    void this.syncBroadcastMetadata(callbacks);
    void this.syncCaptionOverlay(callbacks);
  }

  private async syncPlaybackState(callbacks: PreviewCallbacks) {
    if (!this.running || this.playbackSyncing) return;
    this.playbackSyncing = true;
    try {
      const state = await backend.getPreviewPlaybackState();
      this.consecutiveSyncFailures = 0;
      this.lastTimeSeconds = state.timeSeconds ?? this.lastTimeSeconds;
      this.lastPaused = state.paused ?? this.lastPaused;
      callbacks.setMediaTime(state.timeSeconds == null ? null : Math.round(state.timeSeconds * 1000));
      callbacks.setDuration(state.durationSeconds == null ? null : Math.round(state.durationSeconds * 1000));
      if (state.paused != null) callbacks.setPaused(state.paused);
    } catch (reason) {
      this.consecutiveSyncFailures += 1;
      if (this.consecutiveSyncFailures >= 3) {
        await this.recover(callbacks, reason);
      } else {
        callbacks.onError(reason);
      }
    } finally {
      this.playbackSyncing = false;
    }
  }

  private async syncBroadcastMetadata(callbacks: PreviewCallbacks) {
    const now = Date.now();
    if (!this.running || this.broadcastSyncing || now - this.lastBroadcastSyncAt < 5_000) return;
    this.lastBroadcastSyncAt = now;
    this.broadcastSyncing = true;
    try {
      callbacks.setBroadcastMetadata(
        await backend.getPreviewBroadcastMetadata(callbacks.selectedServiceId()),
      );
    } catch (reason) {
      // SI tables can be absent from an individual bounded window. Keep the
      // last verified metadata while playback proceeds to the next one.
      callbacks.onError(reason);
    } finally {
      this.broadcastSyncing = false;
    }
  }

  private async syncCaptionOverlay(callbacks: PreviewCallbacks) {
    if (!this.running || callbacks.renderBusy()) return;
    const archivePath = callbacks.archivePath();
    if (!archivePath) return;
    callbacks.setRenderBusy(true);
    try {
      const result = await backend.syncPreviewOverlay(archivePath);
      if (result.projectTimeMs != null) callbacks.setProjectTime(result.projectTimeMs);
    } catch (reason) {
      callbacks.onError(reason);
    } finally {
      callbacks.setRenderBusy(false);
    }
  }

  private async recover(callbacks: PreviewCallbacks, originalReason: unknown) {
    if (this.recovering || !this.running || !this.rect || !this.source) return;
    this.recovering = true;
    try {
      await backend.recoverPreview(
        this.source,
        this.rect,
        this.lastTimeSeconds,
        this.lastPaused,
        this.volume,
      );
      this.consecutiveSyncFailures = 0;
      callbacks.onNotice("notice.previewRecovered");
    } catch (recoveryReason) {
      callbacks.onError(originalReason);
      callbacks.onError(recoveryReason);
    } finally {
      this.recovering = false;
    }
  }
}

import {
  backend,
  type BroadcastMetadata,
  type PlaybackTimeMapping,
  type PreviewRect,
} from "../../backend";
import { mediaTimeMs as asMediaTimeMs, type MediaTimeMs } from "./time-mapping";

export type PreviewCallbacks = {
  archivePath: () => string;
  renderBusy: () => boolean;
  setRenderBusy: (value: boolean) => void;
  setMediaTime: (timeMs: MediaTimeMs | null) => void;
  setDuration: (timeMs: MediaTimeMs | null) => void;
  setPaused: (paused: boolean) => void;
  setBroadcastMetadata: (metadata: BroadcastMetadata) => void;
  selectedServiceId: () => number | undefined;
  onError: (reason: unknown) => void;
  onNotice: (code: string) => void;
};

function settleWithin(operation: Promise<unknown> | null, timeoutMs = 1_500) {
  if (!operation) return Promise.resolve();
  return new Promise<void>((resolve) => {
    const timeout = setTimeout(resolve, timeoutMs);
    void operation.then(
      () => { clearTimeout(timeout); resolve(); },
      () => { clearTimeout(timeout); resolve(); },
    );
  });
}

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
  private overlaySyncing = false;
  // Async IPC calls can outlive stop()/start().  Each flag has an owner token
  // so a stale request cannot clear the state (or render-busy indicator) of a
  // newer preview session from its finally block.
  private playbackSyncToken = 0;
  private broadcastSyncToken = 0;
  private overlaySyncToken = 0;
  private lastBroadcastSyncAt = 0;
  private lastCaptionSyncAt = 0;
  private source = "";
  private rect: PreviewRect | null = null;
  private lastTimeSeconds: number | null = null;
  private lastPaused = true;
  private volume = 100;
  private consecutiveSyncFailures = 0;
  private recovering = false;
  private recoveryToken = 0;
  private recoveryPromise: Promise<void> | null = null;
  private syncRevision = 0;
  private scrubbing = false;
  private seeking = false;
  private callbacks: PreviewCallbacks | null = null;

  isRunning() {
    return this.running || this.starting;
  }

  beginSeek() {
    // An exact seek terminates a pointer scrub. Clearing this guard before
    // entering the seek state prevents a short click/release gesture (where
    // no approximate request reached mpv) from leaving polling disabled.
    this.scrubbing = false;
    this.seeking = true;
    this.syncRevision += 1;
    this.lastCaptionSyncAt = 0;
  }

  beginScrub() {
    if (this.scrubbing) {
      this.seeking = false;
      return;
    }
    this.scrubbing = true;
    this.seeking = false;
    this.syncRevision += 1;
    this.lastCaptionSyncAt = 0;
  }

  finishSeek(mediaTimeMs: MediaTimeMs | null) {
    if (mediaTimeMs != null && Number.isFinite(mediaTimeMs))
      this.lastTimeSeconds = Math.max(0, mediaTimeMs) / 1_000;
    this.syncRevision += 1;
    this.seeking = false;
    this.lastCaptionSyncAt = 0;
    this.requestSync();
  }

  finishScrub(mediaTimeMs: MediaTimeMs | null) {
    // Release the scrub guard before requesting the authoritative sample.
    // finishSeek() schedules an immediate poll; doing this in the opposite
    // order caused that poll to be rejected by `scrubbing` and left the UI
    // one sample behind until the next 100 ms interval.
    this.scrubbing = false;
    this.finishSeek(mediaTimeMs);
  }

  /** Abort a gesture without allowing a stale seek to keep polling paused. */
  cancelSeek() {
    this.scrubbing = false;
    this.seeking = false;
    this.syncRevision += 1;
    this.lastCaptionSyncAt = 0;
    this.requestSync();
  }

  requestSync() {
    if (this.callbacks) this.scheduleSync(this.callbacks, this.generation);
  }

  private isCurrent(callbacks: PreviewCallbacks, generation: number) {
    return this.running && this.generation === generation && this.callbacks === callbacks;
  }

  async start(
    source: string,
    rect: PreviewRect,
    setMapping: (mapping: PlaybackTimeMapping) => void,
    callbacks: PreviewCallbacks,
  ): Promise<boolean> {
    await settleWithin(this.recoveryPromise);
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
      this.scrubbing = false;
      this.seeking = false;
      this.syncRevision += 1;
      this.consecutiveSyncFailures = 0;
      this.starting = false;
      this.lastBroadcastSyncAt = 0;
      this.lastCaptionSyncAt = 0;
      this.callbacks = callbacks;
      setMapping(mapping);
      this.stopSync();
      this.syncTimer = setInterval(() => {
        this.scheduleSync(callbacks, generation);
      }, 100);
      // Fetch duration and the first authoritative timestamp before returning
      // to the page. This prevents the controls from briefly rendering a
      // provisional range and then jumping when the first poll arrives.
      const initialSample = await this.syncPlaybackState(callbacks, generation);
      if (initialSample && this.isCurrent(callbacks, generation)) {
        this.lastCaptionSyncAt = performance.now();
        void this.syncCaptionOverlay(
          callbacks,
          initialSample.mediaTimeMs,
          initialSample.revision,
          generation,
        );
      }
      if (!this.isCurrent(callbacks, generation)) return false;
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
    const pendingRecovery = this.recoveryPromise;
    this.recoveryToken += 1;
    const stopGeneration = ++this.generation;
    const sessionCallbacks = this.callbacks;
    this.starting = false;
    if (!hadSession) return;
    // Mark stopped before IPC. A navigation action must not retain a stale
    // session when Windows has already destroyed its native surface.
    this.running = false;
    this.scrubbing = false;
    this.seeking = false;
    this.syncRevision += 1;
    this.stopSync();
    // A stale overlay IPC may have set the UI busy flag and will no longer be
    // allowed to clear it after the session token is invalidated above.
    sessionCallbacks?.setRenderBusy(false);
    // recoverPreview may recreate the native session. Let an already-issued
    // recovery settle before the authoritative stop so it cannot resurrect a
    // player after navigation.
    await settleWithin(pendingRecovery);
    this.recoveryPromise = null;
    this.recovering = false;
    try {
      await backend.clearCaptionOverlay();
    } catch {
      // The native player may already have closed. Stopping remains idempotent.
    }
    try {
      await backend.stopPreview();
    } finally {
      // A new preview may have started while native stop was in flight. Do
      // not clear its callbacks or emit a stale stopped notice in that case.
      if (this.generation === stopGeneration) {
        callbacks.onNotice("notice.previewStopped");
        this.callbacks = null;
      }
    }
  }

  async dispose() {
    const pendingRecovery = this.recoveryPromise;
    this.recoveryToken += 1;
    const disposeGeneration = ++this.generation;
    const sessionCallbacks = this.callbacks;
    this.starting = false;
    this.stopSync();
    sessionCallbacks?.setRenderBusy(false);
    await settleWithin(pendingRecovery);
    this.recoveryPromise = null;
    this.recovering = false;
    if (!this.running) return;
    try {
      await backend.clearCaptionOverlay();
      await backend.stopPreview();
    } catch {
      // The desktop host can be torn down before libmpv receives this command.
    } finally {
      this.running = false;
      this.scrubbing = false;
      this.seeking = false;
      this.syncRevision += 1;
      if (this.generation === disposeGeneration) this.callbacks = null;
    }
  }

  private stopSync() {
    if (this.syncTimer) clearInterval(this.syncTimer);
    this.syncTimer = undefined;
    this.playbackSyncing = false;
    this.broadcastSyncing = false;
    this.overlaySyncing = false;
    this.playbackSyncToken += 1;
    this.broadcastSyncToken += 1;
    this.overlaySyncToken += 1;
    this.lastBroadcastSyncAt = 0;
    this.lastCaptionSyncAt = 0;
  }

  private scheduleSync(callbacks: PreviewCallbacks, generation = this.generation) {
    if (!this.isCurrent(callbacks, generation)) return;
    void this.syncPlaybackState(callbacks, generation).then((sample) => {
      if (!sample) return;
      const now = performance.now();
      if (now - this.lastCaptionSyncAt < 250) return;
      this.lastCaptionSyncAt = now;
      void this.syncCaptionOverlay(callbacks, sample.mediaTimeMs, sample.revision, generation);
    });
    void this.syncBroadcastMetadata(callbacks, generation);
  }

  private async syncPlaybackState(callbacks: PreviewCallbacks, generation = this.generation): Promise<{
    mediaTimeMs: MediaTimeMs;
    revision: number;
  } | null> {
    if (!this.isCurrent(callbacks, generation) || this.playbackSyncing || this.scrubbing || this.seeking) return null;
    const revision = this.syncRevision;
    const token = ++this.playbackSyncToken;
    this.playbackSyncing = true;
    try {
      const state = await backend.getPreviewPlaybackState();
      if (!this.isCurrent(callbacks, generation) || revision !== this.syncRevision) return null;
      this.consecutiveSyncFailures = 0;
      this.lastTimeSeconds = state.timeSeconds ?? this.lastTimeSeconds;
      this.lastPaused = state.paused ?? this.lastPaused;
      const mediaTimeMs = state.timeSeconds == null ? null : asMediaTimeMs(state.timeSeconds * 1000);
      callbacks.setMediaTime(mediaTimeMs);
      callbacks.setDuration(state.durationSeconds == null ? null : asMediaTimeMs(state.durationSeconds * 1000));
      if (state.paused != null) callbacks.setPaused(state.paused);
      return mediaTimeMs == null ? null : { mediaTimeMs, revision };
    } catch (reason) {
      if (!this.isCurrent(callbacks, generation) || revision !== this.syncRevision) return null;
      this.consecutiveSyncFailures += 1;
      if (this.consecutiveSyncFailures >= 3) {
        await this.recover(callbacks, reason, generation);
      } else {
        callbacks.onError(reason);
      }
      return null;
    } finally {
      if (token === this.playbackSyncToken) this.playbackSyncing = false;
    }
  }

  private async syncBroadcastMetadata(callbacks: PreviewCallbacks, generation = this.generation) {
    const now = Date.now();
    if (!this.isCurrent(callbacks, generation) || this.broadcastSyncing || now - this.lastBroadcastSyncAt < 5_000) return;
    this.lastBroadcastSyncAt = now;
    const token = ++this.broadcastSyncToken;
    this.broadcastSyncing = true;
    try {
      const metadata = await backend.getPreviewBroadcastMetadata(callbacks.selectedServiceId());
      if (!this.isCurrent(callbacks, generation)) return;
      callbacks.setBroadcastMetadata(metadata);
    } catch (reason) {
      // SI tables can be absent from an individual bounded window. Keep the
      // last verified metadata while playback proceeds to the next one.
      if (this.isCurrent(callbacks, generation)) callbacks.onError(reason);
    } finally {
      if (token === this.broadcastSyncToken) this.broadcastSyncing = false;
    }
  }

  private async syncCaptionOverlay(
    callbacks: PreviewCallbacks,
    mediaTimeMs: MediaTimeMs,
    revision: number,
    generation = this.generation,
  ) {
    if (
      !this.isCurrent(callbacks, generation)
      || callbacks.renderBusy()
      || this.scrubbing
      || this.seeking
      || this.overlaySyncing
      || revision !== this.syncRevision
    ) return;
    const archivePath = callbacks.archivePath();
    if (!archivePath) return;
    this.overlaySyncing = true;
    const token = ++this.overlaySyncToken;
    callbacks.setRenderBusy(true);
    try {
      // Use the exact playback sample that drove the UI update. The native
      // command accepts it explicitly, so overlay rendering cannot read a
      // second, slightly later player timestamp and land on a different
      // subtitle frame.
      await backend.syncPreviewOverlay(archivePath, mediaTimeMs);
      if (!this.isCurrent(callbacks, generation) || revision !== this.syncRevision) return;
    } catch (reason) {
      if (!this.isCurrent(callbacks, generation) || revision !== this.syncRevision) return;
      callbacks.onError(reason);
    } finally {
      if (token === this.overlaySyncToken) {
        callbacks.setRenderBusy(false);
        this.overlaySyncing = false;
      }
    }
  }

  private async recover(callbacks: PreviewCallbacks, originalReason: unknown, generation = this.generation) {
    if (this.recovering || !this.isCurrent(callbacks, generation) || !this.rect || !this.source) return;
    this.recovering = true;
    const token = ++this.recoveryToken;
    try {
      const recovery = backend.recoverPreview(
        this.source,
        this.rect,
        this.lastTimeSeconds,
        this.lastPaused,
        this.volume,
      );
      this.recoveryPromise = recovery;
      await recovery;
      if (!this.isCurrent(callbacks, generation)) return;
      this.consecutiveSyncFailures = 0;
      callbacks.onNotice("notice.previewRecovered");
    } catch (recoveryReason) {
      if (!this.isCurrent(callbacks, generation)) return;
      callbacks.onError(originalReason);
      callbacks.onError(recoveryReason);
    } finally {
      if (token === this.recoveryToken) {
        this.recovering = false;
        this.recoveryPromise = null;
      }
    }
  }
}

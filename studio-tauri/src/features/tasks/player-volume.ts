export function clampPreviewVolume(value: number) {
  return Number.isFinite(value) ? Math.max(0, Math.min(100, Math.round(value))) : 100;
}

export function toggledPreviewVolume(volume: number, lastNonZeroVolume: number) {
  return volume === 0 ? Math.max(1, clampPreviewVolume(lastNonZeroVolume)) : 0;
}

type VolumeQueueOptions = {
  send: (volume: number) => Promise<void>;
  onError: (reason: unknown) => void;
  schedule: (callback: () => void) => number;
  cancel: (handle: number) => void;
};

/** Sends at most one in-flight native command and keeps only the newest target. */
export class VolumeCommandQueue {
  private readonly options: VolumeQueueOptions;
  private pending: number | null = null;
  private frame = 0;
  private inFlight = false;
  private disposed = false;

  constructor(options: VolumeQueueOptions) {
    this.options = options;
  }

  enqueue(value: number) {
    if (this.disposed) return;
    this.pending = clampPreviewVolume(value);
    if (this.frame || this.inFlight) return;
    this.frame = this.options.schedule(() => {
      this.frame = 0;
      void this.flush();
    });
  }

  private async flush() {
    if (this.inFlight || this.disposed) return;
    this.inFlight = true;
    try {
      while (this.pending !== null && !this.disposed) {
        const volume = this.pending;
        this.pending = null;
        try {
          await this.options.send(volume);
        } catch (reason) {
          this.options.onError(reason);
        }
      }
    } finally {
      this.inFlight = false;
    }
  }

  dispose() {
    this.disposed = true;
    if (this.frame) this.options.cancel(this.frame);
    this.frame = 0;
    this.pending = null;
  }
}

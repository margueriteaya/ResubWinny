/** Coordinates mutually exclusive export/index requests and stale results. */
type ExportSessionHooks = {
  beginExport: () => void;
  setJob: (jobId: string) => void;
  completeExportStart: (jobId: string) => void;
  failExport: (reason: unknown) => void;
  beginIndex: () => void;
  completeIndex: (archivePath: string) => void;
  failIndex: (reason: unknown) => void;
};

export class ExportSession {
  private generation = 0;

  constructor(private readonly hooks: ExportSessionHooks) {}

  begin() {
    return ++this.generation;
  }
  isCurrent(generation: number) { return generation === this.generation; }
  invalidate() { ++this.generation; }

  /** Invalidates in-flight callbacks before asking the backend to stop work. */
  async cancel(operation: () => Promise<void>) {
    this.invalidate();
    await operation();
  }


  async startExport(
    operation: (onCreated: (jobId: string) => void) => Promise<string>,
    onCreated: (jobId: string) => void,
  ) {
    const generation = this.begin();
    let jobId: string;
    try {
      jobId = await operation((createdJobId) => {
        if (this.isCurrent(generation)) onCreated(createdJobId);
      });
    } catch (reason) {
      if (!this.isCurrent(generation)) return null;
      throw reason;
    }
    return this.isCurrent(generation) ? jobId : null;
  }

  async startPreviewIndex<T>(operation: () => Promise<T>, isCurrent: (value: T) => boolean, cancel: () => Promise<void>) {
    const generation = this.begin();
    let result: T;
    try {
      result = await operation();
    } catch (reason) {
      if (!this.isCurrent(generation)) return null;
      throw reason;
    }
    if (!this.isCurrent(generation) || !isCurrent(result)) {
      await cancel().catch(() => {});
      return null;
    }
    return result;
  }

  async runExport(
    operation: (onCreated: (jobId: string) => void) => Promise<string>,
  ) {
    this.hooks.beginExport();
    try {
      const jobId = await this.startExport(operation, this.hooks.setJob);
      if (jobId) this.hooks.completeExportStart(jobId);
    } catch (reason) {
      this.hooks.failExport(reason);
    }
  }

  async runPreviewIndex<T extends { archivePath: string }>(
    operation: () => Promise<T>,
    isCurrent: (value: T) => boolean,
    cancel: () => Promise<void>,
  ) {
    this.hooks.beginIndex();
    try {
      const result = await this.startPreviewIndex(operation, isCurrent, cancel);
      if (result) this.hooks.completeIndex(result.archivePath);
    } catch (reason) {
      this.hooks.failIndex(reason);
    }
  }
}

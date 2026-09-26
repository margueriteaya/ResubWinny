import type { BatchItem } from "./controller";
import type { SourceSession } from "../tasks/source-session";

type Bindings = {
  stopPreview: () => Promise<void>;
  apply: (item: BatchItem) => void;
  archive: (jobId: string) => Promise<string | undefined>;
  setArchive: (path: string) => void;
  layoutReady: () => Promise<void>;
  startPreview: () => Promise<void>;
  needsIndex: () => boolean;
  startIndex: (path: string) => Promise<void>;
};

/** Opens a queued task without allowing an old artifact lookup to replace a new task. */
export class BatchTaskSession {
  private readonly source: Pick<SourceSession, "begin" | "isCurrent">;
  private readonly bindings: Bindings;

  constructor(source: Pick<SourceSession, "begin" | "isCurrent">, bindings: Bindings) {
    this.source = source;
    this.bindings = bindings;
  }

  async open(item: BatchItem) {
    const b = this.bindings;
    const generation = this.source.begin();
    await b.stopPreview();
    if (!this.source.isCurrent(generation)) return;
    b.apply(item);
    if (item.jobId) {
      try {
        const archive = await b.archive(item.jobId);
        if (!this.source.isCurrent(generation)) return;
        if (archive) b.setArchive(archive);
      } catch {
        // Queued/running tasks may not have published an artifact yet.
      }
    }
    await b.layoutReady();
    if (!this.source.isCurrent(generation)) return;
    void b.startPreview();
    if (b.needsIndex()) void b.startIndex(item.inspection.path);
  }
}

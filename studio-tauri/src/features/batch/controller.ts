import { backend, type CheckpointRecord, type DrcsMapping, type ExportFormat, type ExportPreservation, type Inspection, type JobRecord, type Track } from "../../backend";
import { trackKey } from "../tracks";

export { trackKey } from "../tracks";

export type BatchStatus = "Queued" | "Inspecting" | "Processing" | "Completed" | "Warning";
export type BatchItem = {
  inspection: Inspection;
  status: BatchStatus;
  progress: number;
  warnings: number;
  jobId?: string;
  selectedTrackKey?: string;
};

function trackId(track: Track | undefined) {
  const match = track?.pid?.match(/0x([0-9a-f]+)/i);
  return match ? Number.parseInt(match[1], 16) : undefined;
}

export function addInspections(items: BatchItem[], inspections: Inspection[]): BatchItem[] {
  const known = new Set(items.map((item) => item.inspection.path));
  const additions = inspections
    .filter((inspection) => !known.has(inspection.path))
    .map((inspection) => ({
      inspection,
      status: "Queued" as const,
      progress: 0,
      warnings: 0,
      selectedTrackKey: inspection.tracks[0] ? trackKey(inspection.tracks[0]) : undefined,
    }));
  return [...items, ...additions];
}

export function jobStatus(state: JobRecord["state"]): BatchStatus {
  if (state === "Completed") return "Completed";
  if (["Failed", "Cancelled", "Interrupted"].includes(state)) return "Warning";
  if (["Running", "Starting", "Pausing", "Paused", "Resuming", "Cancelling"].includes(state)) return "Processing";
  return "Queued";
}

export function isActiveState(state: JobRecord["state"]): boolean {
  return ["Created", "Queued", "Starting", "Running", "Pausing", "Paused", "Resuming", "Cancelling"].includes(state);
}

export function synchroniseJobs(
  items: BatchItem[],
  jobs: JobRecord[],
  checkpoints = new Map<string, CheckpointRecord | null>(),
) {
  const byId = new Map(jobs.map((job) => [job.jobId, job]));
  const updated = items.map((item) => {
    const job = item.jobId ? byId.get(item.jobId) : undefined;
    if (!job) return item;
    const status = jobStatus(job.state);
    const checkpoint = checkpoints.get(job.jobId);
    const progress = status === "Completed"
      ? 100
      : checkpoint && item.inspection.size > 0
        ? Math.min(99.9, (checkpoint.bytesRead / item.inspection.size) * 100)
        : item.progress;
    return {
      ...item,
      status,
      progress,
      warnings: checkpoint?.warnings ?? item.warnings,
    };
  });
  const active = jobs.find((job) => updated.some((item) => item.jobId === job.jobId) && isActiveState(job.state));
  return {
    items: updated,
    activeJobId: active?.jobId,
    activeInspection: active ? updated.find((item) => item.jobId === active.jobId)?.inspection : undefined,
    queueActive: jobs.some((job) => updated.some((item) => item.jobId === job.jobId) && isActiveState(job.state)),
  };
}

export async function createQueuedJobs(
  items: BatchItem[],
  drcsMappings: DrcsMapping[],
  formats: ExportFormat[],
  preservation: ExportPreservation,
  outputDirectory?: string,
): Promise<{ items: BatchItem[]; jobIds: string[] }> {
  const queued = items.filter((item) => item.status === "Queued");
  const jobIds = await Promise.all(queued.map(async (item) => {
    const selected = item.inspection.tracks.find(
      (track) => trackKey(track) === item.selectedTrackKey,
    ) ?? item.inspection.tracks[0];
    const output = await backend.defaultOutputPath(item.inspection.path, outputDirectory);
    const job = await backend.createJob({
      source: item.inspection.path,
      output,
      archive: false,
      raw: false,
      drcsReport: false,
      drcsMappings,
      trackId: trackId(selected),
      formats,
      preservation,
    });
    return job.jobId;
  }));
  let index = 0;
  return {
    jobIds,
    items: items.map((item) => item.status === "Queued" ? { ...item, jobId: jobIds[index++], status: "Queued" } : item),
  };
}

type BatchQueueHooks = {
  desktopRuntime: boolean;
  items: () => BatchItem[];
  running: () => boolean;
  paused: () => boolean;
  updateItems: (items: BatchItem[]) => void;
  setRunning: (running: boolean) => void;
  setExporting: (exporting: boolean) => void;
  setPaused: (paused: boolean) => void;
  setActiveTask: (jobId: string, inspection: Inspection) => void;
  selectPaths: () => Promise<string[]>;
  inspect: (path: string) => Promise<Inspection>;
  mappings: () => DrcsMapping[];
  formats: () => ExportFormat[];
  preservation: () => ExportPreservation;
  outputDirectory: () => string;
  notice: (code: string, parameters?: Record<string, unknown>) => void;
  fail: (reason: unknown) => void;
};

/** Coordinates queue API calls while App.svelte only projects the resulting state. */
export class BatchQueueController {
  private editingPath: string | null = null;

  constructor(private readonly hooks: BatchQueueHooks) {}

  beginEditing(item: BatchItem) { this.editingPath = item.inspection.path; }
  endEditing() { this.editingPath = null; }

  selectEditingTrack(selectedTrackKey: string) {
    if (!this.editingPath) return;
    const editingPath = this.editingPath;
    this.hooks.updateItems(this.hooks.items().map((item) =>
      item.inspection.path === editingPath
        ? { ...item, selectedTrackKey }
        : item
    ));
  }

  async addFiles() {
    if (!this.hooks.desktopRuntime) return;
    const paths = await this.hooks.selectPaths();
    if (!paths.length) return;
    const discovered: Inspection[] = [];
    for (const path of paths) {
      try {
        discovered.push(await this.hooks.inspect(path));
      } catch (reason) {
        this.hooks.fail(reason);
      }
    }
    this.hooks.updateItems(addInspections(this.hooks.items(), discovered));
  }

  async refresh() {
    const items = this.hooks.items();
    if (!this.hooks.desktopRuntime || !items.length) return;
    try {
      const jobs = await backend.listJobs();
      const jobIds = items
        .map((item) => item.jobId)
        .filter((jobId): jobId is string => Boolean(jobId));
      const checkpoints = new Map(
        await Promise.all(
          jobIds.map(async (jobId) =>
            [jobId, await backend.getJobCheckpoint(jobId)] as const,
          ),
        ),
      );
      const sync = synchroniseJobs(items, jobs, checkpoints);
      this.hooks.updateItems(sync.items);
      if (sync.activeJobId && sync.activeInspection) {
        this.hooks.setActiveTask(sync.activeJobId, sync.activeInspection);
      }
      if (!sync.queueActive) {
        this.hooks.setRunning(false);
        this.hooks.setExporting(false);
      }
    } catch (reason) {
      this.hooks.fail(reason);
    }
  }

  async start() {
    const items = this.hooks.items();
    if (!this.hooks.desktopRuntime) return;
    if (this.hooks.paused()) {
      try {
        this.hooks.setRunning(true);
        await backend.resumeQueue();
        this.hooks.setPaused(false);
        await this.refresh();
      } catch (reason) {
        this.hooks.fail(reason);
      }
      return;
    }
    const queued = items.filter((item) => item.status === "Queued");
    if (!queued.length || this.hooks.running()) return;
    try {
      const existing = queued
        .map((item) => item.jobId)
        .filter((jobId): jobId is string => Boolean(jobId));
      if (existing.length === queued.length) {
        this.hooks.setRunning(true);
        await backend.resumeQueue();
        this.hooks.setPaused(false);
        await this.refresh();
        return;
      }
      const created = await createQueuedJobs(
        items,
        this.hooks.mappings(),
        this.hooks.formats(),
        this.hooks.preservation(),
        this.hooks.outputDirectory().trim() || undefined,
      );
      this.hooks.updateItems(created.items);
      this.hooks.setRunning(true);
      this.hooks.setExporting(true);
      this.hooks.setPaused(false);
      this.hooks.notice("notice.batchQueued", { count: created.jobIds.length });
      await backend.enqueueJobs(created.jobIds);
    } catch (reason) {
      this.hooks.setRunning(false);
      this.hooks.setExporting(false);
      this.hooks.fail(reason);
    }
  }

  async pause() {
    try {
      if (this.hooks.desktopRuntime) await backend.pauseQueue();
      this.hooks.setPaused(true);
      this.hooks.notice("notice.batchPaused");
      await this.refresh();
    } catch (reason) {
      this.hooks.fail(reason);
    }
  }

  async remove(predicate: (item: BatchItem) => boolean) {
    const items = this.hooks.items();
    try {
      const ids = items
        .filter(predicate)
        .map((item) => item.jobId)
        .filter((jobId): jobId is string => Boolean(jobId));
      for (const jobId of ids) await backend.removeJob(jobId);
      this.hooks.updateItems(items.filter((item) => !predicate(item)));
    } catch (reason) {
      this.hooks.fail(reason);
    }
  }

  clearAll() {
    return this.hooks.running() ? Promise.resolve() : this.remove(() => true);
  }

  clearCompleted() {
    return this.remove((item) => item.status === "Completed");
  }
}

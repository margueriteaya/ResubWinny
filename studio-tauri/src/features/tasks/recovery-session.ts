import type { CheckpointRecord, JobRecord } from "../../backend";

type RecoveryHooks = {
  desktopRuntime: boolean;
  getJob: (jobId: string) => Promise<JobRecord | null>;
  getCheckpoint: (jobId: string) => Promise<CheckpointRecord | null>;
  resumeJob: (jobId: string) => Promise<unknown>;
  setAvailable: (available: boolean) => void;
  setBusy: (busy: boolean) => void;
  setExporting: (exporting: boolean) => void;
  setPaused: (paused: boolean) => void;
  onError: (reason: unknown) => void;
  notice: (code: string) => void;
};

/** Owns checkpoint eligibility and replay lifecycle for the current task. */
export class RecoverySession {
  constructor(private readonly hooks: RecoveryHooks) {}

  async refresh(jobId: string) {
    if (!this.hooks.desktopRuntime || !jobId) {
      this.hooks.setAvailable(false);
      return;
    }
    try {
      const [job, checkpoint] = await Promise.all([
        this.hooks.getJob(jobId),
        this.hooks.getCheckpoint(jobId),
      ]);
      this.hooks.setAvailable(Boolean(
        checkpoint && job && ["Interrupted", "Failed", "Cancelled"].includes(job.state),
      ));
    } catch {
      this.hooks.setAvailable(false);
    }
  }

  async resume(jobId: string, available: boolean) {
    if (!this.hooks.desktopRuntime || !available || !jobId) return;
    this.hooks.setBusy(true);
    try {
      await this.hooks.resumeJob(jobId);
      this.hooks.setExporting(true);
      this.hooks.setPaused(false);
      this.hooks.setAvailable(false);
      this.hooks.notice("notice.checkpointReplayStarted");
    } catch (reason) {
      this.hooks.onError(reason);
      await this.refresh(jobId);
    } finally {
      this.hooks.setBusy(false);
    }
  }
}

import type { ArtifactRecord, CheckpointRecord, DiagnosticRecord, DrcsMapping, ExportFormat, ExportPreservation, JobRecord } from '../backend'
import { call } from './client'

type JobRequest = { source: string; output: string; archive: boolean; raw: boolean; trackId?: number; logicalTrack?: string; drcsReport: boolean; drcsMappings: DrcsMapping[]; formats?: ExportFormat[]; preservation?: ExportPreservation }

export const jobsApi = {
  defaultOutputPath: (source: string, outputDirectory?: string) => call<string>('default_output_path', { source, outputDirectory }),
  startPreviewIndex: (source: string, trackId?: number) => call<{ archivePath: string }>('start_preview_index', { source, trackId }),
  startExport: (request: JobRequest & { jobId?: string }) => call<void>('start_export', request),
  cancelExport: () => call<void>('cancel_export'),
  cancelExportAndWait: () => call<void>('cancel_export_and_wait'),
  pauseExport: () => call<void>('pause_export'),
  resumeExport: () => call<void>('resume_export'),
  createJob: (request: JobRequest) => call<JobRecord>('create_job', request),
  listJobs: () => call<JobRecord[]>('list_jobs'),
  listJobsWindow: (offset: number, limit: number) => call<JobRecord[]>('list_jobs_window', { offset, limit }),
  getJob: (jobId: string) => call<JobRecord | null>('get_job', { jobId }),
  getJobDiagnostics: (jobId: string) => call<DiagnosticRecord[]>('get_job_diagnostics', { jobId }),
  getJobDiagnosticsWindow: (jobId: string, offset: number, limit: number) => call<DiagnosticRecord[]>('get_job_diagnostics_window', { jobId, offset, limit }),
  getJobArtifacts: (jobId: string) => call<ArtifactRecord[]>('get_job_artifacts', { jobId }),
  getJobCheckpoint: (jobId: string) => call<CheckpointRecord | null>('get_job_checkpoint', { jobId }),
  removeJob: (jobId: string) => call<void>('remove_job', { jobId }),
  startJob: (jobId: string) => call<void>('start_job', { jobId }),
  enqueueJobs: (jobIds: string[]) => call<void>('enqueue_jobs', { jobIds }),
  pauseJob: () => call<void>('pause_job'),
  resumeJob: (jobId?: string) => call<void>('resume_job', { jobId }),
  cancelJob: () => call<void>('cancel_job'),
  pauseQueue: () => call<void>('pause_queue'),
  resumeQueue: () => call<void>('resume_queue'),
  queueIsPaused: () => call<boolean>('queue_is_paused'),
}

import {
  backend,
  type CaptionRenderSnapshot,
  type DrcsGlyph,
  type DrcsMapping,
  type ExportFormat,
  type ExportPreservation,
  type Inspection,
  type Track,
} from "../../backend";
import { trackKey } from "../tracks";

export type { ExportFormat, ExportPreservation } from "../../backend";
export type TaskExportPlan = {
  output: string;
  formats: ExportFormat[];
  preservation: ExportPreservation;
  trackId?: number;
};

export type SourceTaskSetup = {
  outputDirectory: string;
  selectedTrackKeys: Set<string>;
  selectedFormats: Set<ExportFormat> | null;
  logs: string[];
};

export const taskTrackKey = trackKey;

export function taskTrackId(track: Track | undefined) {
  const match = track?.pid?.match(/0x([0-9a-f]+)/i);
  return match ? Number.parseInt(match[1], 16) : undefined;
}

export function createExportPlan(
  inspection: Inspection,
  formats: Set<ExportFormat>,
  preservation: ExportPreservation,
  selectedTrackKeys: Set<string>,
  outputDirectory = inspection.path.replace(/[\\/][^\\/]+$/, ""),
): TaskExportPlan | null {
  const selectedTrack = inspection.tracks.find((track) =>
    selectedTrackKeys.has(taskTrackKey(track)),
  );
  if (inspection.tracks.length > 0 && !selectedTrack) return null;

  if (!formats.size || !outputDirectory.trim()) return null;
  const separator = outputDirectory.includes("\\") ? "\\" : "/";
  const name = `${inspection.name.replace(/\.[^.]+$/, "")}.ass`;
  return {
    output: `${outputDirectory.replace(/[\\/]+$/, "")}${separator}${name}`,
    formats: [...formats],
    preservation,
    trackId: taskTrackId(selectedTrack),
  };
}

export function inspectTaskSource(path: string) {
  return backend.inspectSource(path);
}

export function createSourceTaskSetup(
  inspection: Inspection,
  defaultFormat: string,
  message: (code: string, parameters: Record<string, unknown>) => string,
): SourceTaskSetup {
  const selectedTrack = inspection.tracks[0];
  const supportedDefault = ["ASS", "TTML", "JSON", "Raw Data"].includes(defaultFormat)
    ? defaultFormat as ExportFormat
    : null;
  return {
    outputDirectory: inspection.path.replace(/[\\/][^\\/]+$/, ""),
    selectedTrackKeys: selectedTrack
      ? new Set([taskTrackKey(selectedTrack)])
      : new Set(),
    selectedFormats: supportedDefault ? new Set([supportedDefault]) : null,
    logs: [
      message("notice.sourceSelected", { name: inspection.name }),
      message("notice.container", { container: inspection.container }),
      message("notice.captionTracks", { count: inspection.tracks.length }),
    ],
  };
}

export async function startTaskExport(
  inspection: Inspection,
  plan: TaskExportPlan,
  drcsMappings: DrcsMapping[],
  onCreated?: (jobId: string) => void,
) {
  const job = await backend.createJob({
    source: inspection.path,
    output: plan.output,
    archive: false,
    raw: false,
    trackId: plan.trackId,
    drcsReport: false,
    drcsMappings,
    formats: plan.formats,
    preservation: plan.preservation,
  });
  onCreated?.(job.jobId);
  await backend.startJob(job.jobId);
  return job.jobId;
}

export function renderTaskSnapshot(
  archivePath: string,
  mediaTimeMs: number,
  playerRunning: boolean,
): Promise<CaptionRenderSnapshot> {
  return playerRunning
    ? backend.renderPreviewAt(archivePath, mediaTimeMs)
    : backend.renderAt(archivePath, mediaTimeMs);
}

export function loadTaskDrcs(path: string): Promise<DrcsGlyph[]> {
  return backend.loadDrcsReport(path);
}

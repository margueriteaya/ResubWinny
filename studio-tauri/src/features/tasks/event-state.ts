import type { TaskEvent } from "../../backend";
import type { FeatureFact, FeatureKnowledge, FeatureKnowledgeState, RuntimeExportConflicts } from "./export-assessment";

export function featureCountSummary(fact: FeatureFact | undefined): { count: number; final: boolean } | null {
  if (fact?.state !== "present" || !fact.observedCount) return null;
  return { count: fact.observedCount, final: fact.complete };
}

export type TaskEventState = {
  archivePath: string;
  bytesRead: number;
  captions: number;
  isExporting: boolean;
  isPaused: boolean;
  lastLoggedProgressBucket: number;
  logs: string[];
  previewIndexing: boolean;
  progress: number;
  warnings: number;
  featureKnowledge: Record<string, FeatureKnowledge>;
  exportConflicts: Record<string, RuntimeExportConflicts>;
};

export function emptyTaskEventState(): TaskEventState {
  return {
    archivePath: "",
    bytesRead: 0,
    captions: 0,
    isExporting: false,
    isPaused: false,
    lastLoggedProgressBucket: -1,
    logs: [],
    previewIndexing: false,
    progress: 0,
    warnings: 0,
    featureKnowledge: {},
    exportConflicts: {},
  };
}

export function resetTaskEventState(overrides: Partial<TaskEventState> = {}): TaskEventState {
  return { ...emptyTaskEventState(), ...overrides };
}

export type TaskEventEffects = {
  addHistory: "Completed" | "Warning" | null;
  archiveCompleted: boolean;
  refreshBatch: boolean;
  refreshResume: boolean;
};

export function reduceTaskEvent(
  current: TaskEventState,
  event: TaskEvent,
  sourceSize: number,
  message: string,
  batchRunning: boolean,
  sourceIdentity = "",
): { state: TaskEventState; effects: TaskEventEffects } {
  const state = { ...current, logs: current.logs };
  const wasPreviewIndexing = current.previewIndexing;
  if (typeof event.bytesRead === "number" && Number.isFinite(event.bytesRead))
    state.bytesRead = Math.max(0, event.bytesRead);
  if (typeof event.captions === "number" && Number.isFinite(event.captions))
    state.captions = Math.max(0, event.captions);
  if (typeof event.warnings === "number" && Number.isFinite(event.warnings))
    state.warnings = Math.max(0, event.warnings);
  if (sourceSize > 0 && state.bytesRead > 0)
    state.progress = Math.min(100, (state.bytesRead / sourceSize) * 100);

  if (event.kind === "feature_observed" || event.kind === "feature_summary" || event.code === "export_conflict") {
    const logicalTrack = String(event.parameters?.logicalTrack ?? "");
    const feature = String(event.parameters?.feature ?? "") as keyof import("../../backend").ExportPreservation;
    if (logicalTrack && ["position", "color", "ruby", "drcs", "gaiji", "accessibility"].includes(feature)) {
      const key = `${sourceIdentity}::${logicalTrack}`;
      const track = { ...(state.featureKnowledge[key] ?? {}) };
      const previous = track[feature];
      const incoming = (event.kind === "feature_observed" || event.code === "export_conflict" ? "present" : event.parameters?.state) as FeatureKnowledgeState;
      const nextState = previous?.state === "present" ? "present" : incoming;
      if (nextState === "present" || (nextState === "absent" && event.parameters?.complete === true)) {
        track[feature] = {
          state: nextState,
          observedCount: Number(event.parameters?.observedCount ?? previous?.observedCount ?? 0),
          complete: event.parameters?.complete === true || previous?.complete === true,
          details: {
            ...previous?.details,
            ...(event.parameters?.details as Record<string, unknown> | undefined),
          },
        };
        state.featureKnowledge = { ...state.featureKnowledge, [key]: track };
        if (event.code === "export_conflict") {
          const conflicts = { ...(state.exportConflicts[key] ?? {}) };
          conflicts[feature] = {
            formats: Array.isArray(event.parameters?.formats) ? event.parameters.formats.map(String) as import("../../backend").ExportFormat[] : [],
            issueCode: String(event.parameters?.issueCode ?? "format_cannot_preserve_feature"),
            availableActions: Array.isArray(event.parameters?.availableActions) ? event.parameters.availableActions.map(String) : [],
          };
          state.exportConflicts = { ...state.exportConflicts, [key]: conflicts };
        }
      }
    }
  }

  const progressBucket = Math.floor(state.progress / 5);
  if (event.kind !== "progress" || progressBucket > state.lastLoggedProgressBucket) {
    state.logs = [...state.logs, message].slice(-1000);
    if (event.kind === "progress") state.lastLoggedProgressBucket = progressBucket;
  }

  const effects: TaskEventEffects = {
    addHistory: null,
    archiveCompleted: false,
    refreshBatch: batchRunning,
    refreshResume: false,
  };
  if (event.kind === "completed") {
    state.isExporting = false;
    state.isPaused = false;
    state.progress = 100;
    state.archivePath = state.archivePath.replace(/\.jsonl\.part$/i, ".jsonl");
    effects.archiveCompleted = Boolean(state.archivePath);
    effects.addHistory = wasPreviewIndexing
      ? null
      : state.warnings
        ? "Warning"
        : "Completed";
    state.previewIndexing = false;
  } else if (event.kind === "failed") {
    state.isExporting = false;
    state.isPaused = false;
    effects.addHistory = wasPreviewIndexing ? null : "Warning";
    effects.refreshResume = true;
    state.previewIndexing = false;
  } else if (event.kind === "paused") {
    state.isPaused = true;
  } else if (event.kind === "resumed") {
    state.isPaused = false;
  } else if (event.kind === "cancelled") {
    state.isExporting = false;
    state.isPaused = false;
    state.previewIndexing = false;
    effects.refreshResume = true;
  }
  return { state, effects };
}

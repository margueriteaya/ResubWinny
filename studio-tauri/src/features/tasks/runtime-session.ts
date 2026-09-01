import { resetTaskEventState, type TaskEventState } from "./event-state";
import { projectTimeMs, type MediaTimeMs, type ProjectTimeMs } from "./time-mapping";

export type RuntimeReset = TaskEventState & {
  mediaTimeMs: MediaTimeMs | null;
  projectTimeMs: ProjectTimeMs;
  previewDurationMs: MediaTimeMs | null;
};

/** Builds consistent runtime projections when a task source changes. */
export function resetTaskRuntime(
  patch: Partial<RuntimeReset> = {},
): RuntimeReset {
  return {
    ...resetTaskEventState(),
    archivePath: "",
    mediaTimeMs: null,
    projectTimeMs: projectTimeMs(0),
    previewDurationMs: null,
    ...patch,
  };
}

type RuntimeProjection = {
  setEventState: (state: TaskEventState) => void;
  setMediaTime: (value: MediaTimeMs | null) => void;
  setProjectTime: (value: ProjectTimeMs) => void;
  setDuration: (value: MediaTimeMs | null) => void;
};

/** Applies a consistent reset through a narrow projection instead of exposing state fields. */
export class TaskRuntimeSession {
  constructor(private readonly projection: RuntimeProjection) {}

  reset(patch: Partial<RuntimeReset> = {}) {
    const next = resetTaskRuntime(patch);
    this.projection.setEventState(next);
    this.projection.setMediaTime(next.mediaTimeMs);
    this.projection.setProjectTime(next.projectTimeMs);
    this.projection.setDuration(next.previewDurationMs);
  }
}

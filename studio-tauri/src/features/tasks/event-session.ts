import type { TaskEvent } from "../../backend";
import { formatMessage } from "../../i18n";
import {
  reduceTaskEvent,
  type TaskEventEffects,
  type TaskEventState,
} from "./event-state";

type EventSessionBindings = {
  currentJobId: () => string;
  previewIndexing: () => boolean;
  batchRunning: () => boolean;
  sourceSize: () => number;
  sourceIdentity: () => string;
  state: () => TaskEventState;
  setState: (state: TaskEventState) => void;
  onEffects: (effects: TaskEventEffects) => void;
  refreshBatch: () => void;
};

/** Owns Worker event subscription filtering and reducer projection. */
export class TaskEventSession {
  constructor(private readonly bindings: EventSessionBindings) {}

  handle(payload: TaskEvent) {
    const belongsToCurrentJob = Boolean(
      payload.jobId && this.bindings.currentJobId() &&
        payload.jobId === this.bindings.currentJobId(),
    );
    const belongsToPreviewIndex = !payload.jobId && this.bindings.previewIndexing();
    if (!belongsToCurrentJob && !belongsToPreviewIndex) {
      if (this.bindings.batchRunning()) this.bindings.refreshBatch();
      return;
    }
    const message = payload.code
      ? formatMessage(payload.code, payload.parameters, payload.message)
      : payload.message;
    const transition = reduceTaskEvent(
      this.bindings.state(),
      payload,
      this.bindings.sourceSize(),
      message,
      this.bindings.batchRunning(),
      this.bindings.sourceIdentity(),
    );
    this.bindings.setState(transition.state);
    this.bindings.onEffects(transition.effects);
  }
}

import type { TaskHistoryRecord } from "../../backend";
import { upsertHistory, type TaskRecord } from "./presentation";

type HistoryHooks = {
  desktopRuntime: boolean;
  load: () => Promise<TaskHistoryRecord[]>;
  save: (records: TaskHistoryRecord[]) => Promise<unknown>;
  onError: (reason: unknown) => void;
};

/** Owns the bounded task-history persistence policy. */
export class HistorySession {
  constructor(private readonly hooks: HistoryHooks) {}

  async load() {
    if (!this.hooks.desktopRuntime) return [] as TaskRecord[];
    try {
      return (await this.hooks.load()) as TaskRecord[];
    } catch (reason) {
      this.hooks.onError(reason);
      return [] as TaskRecord[];
    }
  }

  add(history: TaskRecord[], record: TaskRecord) {
    const next = upsertHistory(history, record);
    this.persist(next);
    return next;
  }

  persist(history: TaskRecord[]) {
    if (!this.hooks.desktopRuntime) return;
    const records: TaskHistoryRecord[] = history.slice(0, 25).map((item) => ({
      ...item,
      captions: item.captions ?? 0,
    }));
    void this.hooks.save(records).catch(this.hooks.onError);
  }
}

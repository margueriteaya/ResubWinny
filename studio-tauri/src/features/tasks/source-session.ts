import type { Inspection } from "../../backend";
import { createSourceTaskSetup, type SourceTaskSetup } from "./controller";

export type SourceInspectionResult<T> = {
  generation: number;
  value: T;
};

type SourceSessionHooks = {
  prepare: () => Promise<void>;
  inspect: (path: string) => Promise<Inspection>;
  preferredFormats: () => string[];
  message: (code: string, parameters: Record<string, unknown>) => string;
  apply: (inspection: Inspection, setup: SourceTaskSetup, jobId: string) => void;
  afterApply: () => Promise<void>;
  activate: (path: string) => void;
  setBusy: (busy: boolean) => void;
  fail: (reason: unknown) => void;
};

/** Serializes source preparation/inspection without owning page state. */
export class SourceSession {
  private generation = 0;

  constructor(private readonly hooks: SourceSessionHooks) {}

  begin() { return ++this.generation; }
  isCurrent(generation: number) { return generation === this.generation; }
  invalidate() { ++this.generation; }

  private async inspect<T>(
    prepare: () => Promise<void>,
    operation: () => Promise<T>,
    setBusy: (busy: boolean) => void,
  ): Promise<SourceInspectionResult<T> | null> {
    const generation = this.begin();
    setBusy(true);
    try {
      await prepare();
      if (!this.isCurrent(generation)) return null;
      const value = await operation();
      return this.isCurrent(generation) ? { generation, value } : null;
    } catch (reason) {
      if (!this.isCurrent(generation)) return null;
      throw reason;
    } finally {
      if (this.isCurrent(generation)) setBusy(false);
    }
  }

  async load(path: string, jobId = "") {
    try {
      const result = await this.inspect(
        this.hooks.prepare,
        () => this.hooks.inspect(path),
        this.hooks.setBusy,
      );
      if (!result) return;
      const setup = createSourceTaskSetup(
        result.value,
        this.hooks.preferredFormats(),
        this.hooks.message,
      );
      this.hooks.apply(result.value, setup, jobId);
      await this.hooks.afterApply();
      if (!this.isCurrent(result.generation)) return;
      this.hooks.activate(path);
    } catch (reason) {
      this.hooks.fail(reason);
    }
  }
}

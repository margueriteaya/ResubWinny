import type { Inspection } from "../../backend";
import type { TaskExportPlan } from "./controller";
import type { ExportSession } from "./export-session";

type ExportWorkflowBindings = {
  desktopRuntime: () => boolean;
  inspection: () => Inspection | null;
  sourceGeneration: () => number;
  exporting: () => boolean;
  pending: () => boolean;
  indexing: () => boolean;
  outputDirectory: () => string;
  plan: (inspection: Inspection) => TaskExportPlan | null;
  setPending: (pending: boolean) => void;
  setIndexing: (indexing: boolean) => void;
  error: (code: string) => void;
  clearError: () => void;
  started: (plan: TaskExportPlan) => void;
  fail: (reason: unknown) => void;
  cancelIndex: () => Promise<void>;
  start: (inspection: Inspection, plan: TaskExportPlan, onCreated: (id: string) => void) => Promise<string>;
  index: (inspection: Inspection) => Promise<{ archivePath: string }>;
};

/** Owns export validation and the transition from preview indexing to export. */
export class ExportWorkflow {
  private readonly session: ExportSession;
  private readonly bindings: ExportWorkflowBindings;

  constructor(session: ExportSession, bindings: ExportWorkflowBindings) {
    this.session = session;
    this.bindings = bindings;
  }

  async start() {
    const b = this.bindings;
    if (!b.desktopRuntime()) { b.error("error.desktopExport"); return; }
    const source = b.inspection();
    const generation = b.sourceGeneration();
    if (!source || b.exporting() || b.pending()) return;
    b.clearError();
    if (!b.outputDirectory().trim()) { b.error("workspace.outputDirectoryRequired"); return; }
    const plan = b.plan(source);
    if (!plan) { b.error("tracks.selectionRequired"); return; }
    if (b.indexing()) {
      b.setPending(true);
      try {
        await this.session.cancel(b.cancelIndex);
      } catch (reason) {
        if (b.sourceGeneration() === generation) b.fail(reason);
        return;
      } finally {
        b.setPending(false);
      }
      // Source loading can complete while the previous index is stopping.
      if (b.sourceGeneration() !== generation || b.inspection()?.path !== source.path) return;
      b.setIndexing(false);
    }
    b.started(plan);
    await this.session.runExport((onCreated) => b.start(source, plan, onCreated));
  }

  async index(expectedPath = this.bindings.inspection()?.path ?? "") {
    const b = this.bindings;
    const source = b.inspection();
    if (!b.desktopRuntime() || !source || b.exporting() || b.pending() || b.indexing()) return;
    if (!expectedPath || source.path !== expectedPath) return;
    await this.session.runPreviewIndex(
      () => b.index(source),
      () => b.inspection()?.path === source.path,
      b.cancelIndex,
    );
  }
}

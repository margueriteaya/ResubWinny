import type { ExportFormat, ExportPreservation } from "../../backend";

/** Encapsulates task output and track-selection transitions. */
export class TaskSelectionSession {
  toggleFormat(formats: Set<ExportFormat>, next: ExportFormat) {
    const updated = new Set(formats);
    if (updated.has(next)) updated.delete(next); else updated.add(next);
    return updated;
  }

  togglePreservation(preservation: ExportPreservation, feature: keyof ExportPreservation) {
    return { ...preservation, [feature]: !preservation[feature] };
  }

  singleTrack(trackKey: string) { return new Set([trackKey]); }
}

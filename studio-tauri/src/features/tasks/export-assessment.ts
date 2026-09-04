import type { ExportFormat, ExportPreservation } from "../../backend";
import { formatCapabilities, type CapabilityLevel } from "./format-capabilities";

export type FeatureKnowledgeState = "unknown" | "present" | "absent";
export type FeatureKnowledge = Partial<Record<keyof ExportPreservation, FeatureKnowledgeState>>;
export type AssessmentIssue = { code: string; feature: keyof ExportPreservation; severity: "warning" | "conflict"; parameters: Record<string, string> };
export type FormatAssessment = { preserved: string[]; approximated: string[]; dropped: string[]; conditional: AssessmentIssue[]; conflicts: AssessmentIssue[]; warnings: AssessmentIssue[] };
export type ExportAssessment = { formats: Partial<Record<ExportFormat, FormatAssessment>>; hasConflict: boolean };

const featureMap: Record<string, keyof ExportPreservation> = { position: "position", color: "color", ruby: "ruby", drcs: "drcs" };

export function assessExports(formats: Iterable<ExportFormat>, preservation: ExportPreservation, knowledge: FeatureKnowledge = {}): ExportAssessment {
  const result: ExportAssessment = { formats: {}, hasConflict: false };
  for (const format of formats) {
    const assessment: FormatAssessment = { preserved: [], approximated: [], dropped: [], conditional: [], conflicts: [], warnings: [] };
    for (const capability of formatCapabilities(format)) {
      const feature = featureMap[capability.feature];
      if (!feature || !preservation[feature]) {
        if (feature && knowledge[feature] === "present") assessment.dropped.push(feature);
        continue;
      }
      const state = knowledge[feature] ?? "unknown";
      if (state === "absent") continue;
      if (capability.level === "preserved") { if (state === "present") assessment.preserved.push(feature); continue; }
      if (capability.level === "approximated") { if (state === "present") assessment.approximated.push(feature); continue; }
      const issue: AssessmentIssue = { code: "format_cannot_preserve_feature", feature, severity: state === "present" ? "conflict" : "warning", parameters: { format, feature } };
      if (state === "present") { assessment.conflicts.push(issue); result.hasConflict = true; }
      else assessment.conditional.push(issue);
    }
    result.formats[format] = assessment;
  }
  return result;
}

export function capabilityMark(level: CapabilityLevel) { return level === "preserved" ? "✓" : level === "approximated" ? "△" : level === "conditional" ? "◇" : "×"; }

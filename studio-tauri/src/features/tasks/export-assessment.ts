import type { ExportFormat, ExportPreservation } from "../../backend";
import { formatCapabilities, type CapabilityLevel } from "./format-capabilities.ts";

export type FeatureKnowledgeState = "unknown" | "present" | "absent";
export type FeatureFact = { state: FeatureKnowledgeState; observedCount?: number; complete: boolean; details?: Record<string, unknown> };
export type FeatureKnowledge = Partial<Record<keyof ExportPreservation, FeatureFact>>;
export type RuntimeExportConflict = { formats: ExportFormat[]; issueCode: string; availableActions: string[] };
export type RuntimeExportConflicts = Partial<Record<keyof ExportPreservation, RuntimeExportConflict>>;
export type AssessmentIssue = { code: string; feature: keyof ExportPreservation; severity?: "warning" | "conflict"; parameters: Record<string, string>; actions: string[] };
export type FormatAssessment = { preserved: AssessmentIssue[]; approximated: AssessmentIssue[]; dropped: AssessmentIssue[]; conditional: AssessmentIssue[]; conflicts: AssessmentIssue[]; warnings: AssessmentIssue[] };
export type ExportAssessment = { formats: Partial<Record<ExportFormat, FormatAssessment>>; hasConflict: boolean };

const featureMap: Record<string, keyof ExportPreservation> = { position: "position", color: "color", ruby: "ruby", drcs: "drcs", gaiji: "gaiji", accessibility: "accessibility" };

export function assessExports(formats: Iterable<ExportFormat>, preservation: ExportPreservation, knowledge: FeatureKnowledge = {}, runtimeConflicts: RuntimeExportConflicts = {}): ExportAssessment {
  const result: ExportAssessment = { formats: {}, hasConflict: false };
  for (const format of formats) {
    const assessment: FormatAssessment = { preserved: [], approximated: [], dropped: [], conditional: [], conflicts: [], warnings: [] };
    for (const capability of formatCapabilities(format)) {
      const feature = featureMap[capability.feature];
      const entry = (code: string): AssessmentIssue => ({ code, feature, parameters: { format, feature }, actions: [] });
      if (!feature || !preservation[feature]) {
        if (feature && knowledge[feature]?.state === "present") assessment.dropped.push(entry("feature_dropped"));
        if (feature && (knowledge[feature]?.state ?? "unknown") === "unknown") assessment.conditional.push(entry("feature_will_be_dropped_if_present"));
        continue;
      }
      const state = knowledge[feature]?.state ?? "unknown";
      if (state === "absent") continue;
      if (capability.level === "preserved") { if (state === "present") assessment.preserved.push(entry("format_preserves_feature")); continue; }
      if (capability.level === "approximated") { if (state === "present") assessment.approximated.push(entry("format_approximates_feature")); continue; }
      if (capability.level === "conditional") {
        const runtimeConflict = runtimeConflicts[feature];
        if (state === "present" && runtimeConflict?.formats.includes(format)) {
          const issue: AssessmentIssue = {
            code: runtimeConflict.issueCode,
            feature,
            severity: "conflict",
            parameters: { format, feature },
            actions: runtimeConflict.availableActions,
          };
          assessment.conflicts.push(issue);
          result.hasConflict = true;
        } else if (state === "present") {
          assessment.approximated.push(entry("format_approximates_feature"));
        } else {
          assessment.conditional.push({ code: "format_conditionally_preserves_feature", feature, severity: "warning", parameters: { format, feature }, actions: [] });
        }
        continue;
      }
      const actions = [`disable_preservation:${feature}`, "remove_format"];
      if (format !== "ASS" && formatCapabilities("ASS").some((item) => item.feature === feature && item.level !== "unsupported")) actions.push("choose_compatible_format");
      const issue: AssessmentIssue = { code: "format_cannot_preserve_feature", feature, severity: state === "present" ? "conflict" : "warning", parameters: { format, feature }, actions };
      if (state === "present") { assessment.conflicts.push(issue); result.hasConflict = true; }
      else assessment.conditional.push(issue);
    }
    result.formats[format] = assessment;
  }
  return result;
}

export function capabilityMark(level: CapabilityLevel) { return level === "preserved" ? "✓" : level === "approximated" ? "△" : level === "conditional" ? "◇" : "×"; }

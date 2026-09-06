import type { ExportPreservation } from "../../backend";
import type { FeatureFact } from "./export-assessment";

export type FeatureDetailKey =
  | "multipleRegions"
  | "explicitGeometry"
  | "verticalWriting"
  | "explicitDirection"
  | "explicitAlignment"
  | "foreground"
  | "background"
  | "stroke"
  | "boundAnnotation"
  | "resourceBacked"
  | "aribAdditionalSymbol"
  | "textCue"
  | "leadingAnnotation"
  | "musicCue"
  | "narrationDelimiter";

const detailKeys: Record<keyof ExportPreservation, readonly FeatureDetailKey[]> = {
  position: ["multipleRegions", "explicitGeometry", "verticalWriting", "explicitDirection", "explicitAlignment"],
  color: ["foreground", "background", "stroke"],
  ruby: ["boundAnnotation"],
  drcs: ["resourceBacked"],
  gaiji: ["aribAdditionalSymbol"],
  accessibility: ["leadingAnnotation", "musicCue", "narrationDelimiter"],
};

export function featureDetailKeys(
  feature: keyof ExportPreservation,
  fact: FeatureFact | undefined,
): FeatureDetailKey[] {
  if (fact?.state !== "present" || !fact.details) return [];
  return detailKeys[feature].filter((key) => fact.details?.[key] === true);
}

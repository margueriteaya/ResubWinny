import type { ExportFormat } from "../../backend";
import canonical from "../../../../shared/format_capabilities.json" with { type: "json" };

export type CapabilityLevel = "preserved" | "approximated" | "unsupported" | "conditional";
export type FormatCapability = { feature: string; level: CapabilityLevel };

// Presentation consumes this single capability contract. Runtime assessment
// remains a worker concern; these entries describe target-format semantics.
const capabilities = Object.entries(canonical).reduce((result, [format, entries]) => {
  result[format as ExportFormat] = Object.entries(entries as Record<string, CapabilityLevel>).map(([feature, level]) => ({ feature, level }));
  return result;
}, {} as Record<ExportFormat, FormatCapability[]>);
export function formatCapabilities(format: ExportFormat) { return capabilities[format]; }
export function capabilitySummary(format: ExportFormat, featureLabel: (feature: string) => string) {
  return capabilities[format].map((item) => `${item.level === "preserved" ? "✓" : item.level === "approximated" ? "△" : item.level === "conditional" ? "◇" : "×"} ${featureLabel(item.feature)}`).join(" · ");
}

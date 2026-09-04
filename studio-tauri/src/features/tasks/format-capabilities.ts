import type { ExportFormat } from "../../backend";
import canonical from "../../../../shared/format_capabilities.json" with { type: "json" };

export type CapabilityLevel = "preserved" | "approximated" | "unsupported" | "conditional";
export type FormatCapability = { feature: string; level: CapabilityLevel; label: string };

// Presentation consumes this single capability contract. Runtime assessment
// remains a worker concern; these entries describe target-format semantics.
const labels: Record<string, string> = {
  position: "位置",
  color: "颜色",
  ruby: "Ruby",
  drcs: "DRCS",
  gaiji: "外字",
  accessibility: "无障碍信息",
};
const capabilities = Object.entries(canonical).reduce((result, [format, entries]) => {
  result[format as ExportFormat] = Object.entries(entries as Record<string, CapabilityLevel>).map(([feature, level]) => ({ feature, level, label: labels[feature] ?? feature }));
  return result;
}, {} as Record<ExportFormat, FormatCapability[]>);
export function formatCapabilities(format: ExportFormat) { return capabilities[format]; }
export function capabilitySummary(format: ExportFormat) {
  return capabilities[format].map((item) => `${item.level === "preserved" ? "✓" : item.level === "approximated" ? "△" : item.level === "conditional" ? "◇" : "×"} ${item.label}`).join(" · ");
}

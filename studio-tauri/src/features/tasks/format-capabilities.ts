import type { ExportFormat } from "../../backend";
import canonical from "../../../../shared/format_capabilities.json";

export type CapabilityLevel = "preserved" | "approximated" | "unsupported" | "conditional";
export type FormatCapability = { feature: string; level: CapabilityLevel; label: string };

// Presentation consumes this single capability contract. Runtime assessment
// remains a worker concern; these entries describe target-format semantics.
const labels: Record<string, string> = { position: "位置", color: "颜色", ruby: "Ruby", drcs: "DRCS" };
const capabilities = Object.entries(canonical).reduce((result, [format, entries]) => {
  result[format as ExportFormat] = Object.entries(entries as Record<string, CapabilityLevel>).map(([feature, level]) => ({ feature, level, label: labels[feature] ?? feature }));
  return result;
}, {} as Record<ExportFormat, FormatCapability[]>);
/*
const capabilities: Record<ExportFormat, FormatCapability[]> = {
  ASS: [
    { feature: "position", level: "preserved", label: "位置" },
    { feature: "color", level: "preserved", label: "颜色" },
    { feature: "ruby", level: "approximated", label: "Ruby（兼容近似）" },
    { feature: "drcs", level: "conditional", label: "DRCS（按视觉资源保留）" },
  ],
  TTML: [
    { feature: "position", level: "preserved", label: "位置与区域" },
    { feature: "color", level: "preserved", label: "颜色" },
    { feature: "ruby", level: "preserved", label: "Ruby" },
    { feature: "drcs", level: "conditional", label: "DRCS（取决于资源）" },
  ],
  SRT: [
    { feature: "position", level: "unsupported", label: "位置" },
    { feature: "color", level: "unsupported", label: "颜色" },
    { feature: "ruby", level: "unsupported", label: "Ruby" },
    { feature: "drcs", level: "conditional", label: "DRCS（仅限可映射字符）" },
  ],
  WebVTT: [
    { feature: "position", level: "approximated", label: "位置（有限支持）" },
    { feature: "color", level: "unsupported", label: "颜色" },
    { feature: "ruby", level: "unsupported", label: "Ruby" },
    { feature: "drcs", level: "conditional", label: "DRCS（仅限可映射字符）" },
  ],
  JSON: [
    { feature: "position", level: "preserved", label: "位置" },
    { feature: "color", level: "preserved", label: "颜色" },
    { feature: "ruby", level: "preserved", label: "Ruby" },
    { feature: "drcs", level: "preserved", label: "DRCS 资源" },
  ],
  "Raw Data": [
    { feature: "position", level: "preserved", label: "位置证据" },
    { feature: "color", level: "preserved", label: "颜色证据" },
    { feature: "ruby", level: "preserved", label: "Ruby 证据" },
    { feature: "drcs", level: "preserved", label: "DRCS 资源" },
  ],
}; */

export function formatCapabilities(format: ExportFormat) { return capabilities[format]; }
export function capabilitySummary(format: ExportFormat) {
  return capabilities[format].map((item) => `${item.level === "preserved" ? "✓" : item.level === "approximated" ? "△" : item.level === "conditional" ? "◇" : "×"} ${item.label}`).join(" · ");
}

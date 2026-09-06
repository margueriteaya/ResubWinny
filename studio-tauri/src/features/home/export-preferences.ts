import type { ExportFormat } from "../../backend";

export function togglePreferredFormat(
  formats: readonly ExportFormat[],
  format: ExportFormat,
): ExportFormat[] {
  if (formats.includes(format)) {
    return formats.length === 1 ? [...formats] : formats.filter((item) => item !== format);
  }
  return [...formats, format];
}

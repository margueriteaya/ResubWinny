import IconBinary from "@tabler/icons-svelte/icons/binary";
import IconBraces from "@tabler/icons-svelte/icons/braces";
import IconBrowser from "@tabler/icons-svelte/icons/browser";
import IconFileCode from "@tabler/icons-svelte/icons/file-code";
import IconListNumbers from "@tabler/icons-svelte/icons/list-numbers";
import IconSubtitles from "@tabler/icons-svelte/icons/subtitles";
import type { ExportFormat } from "./controller";

export const formatVisuals: Record<ExportFormat, { icon: any; extension: string; group: "subtitle" | "data" }> = {
  ASS: { icon: IconSubtitles, extension: ".ass", group: "subtitle" },
  TTML: { icon: IconFileCode, extension: ".ttml", group: "subtitle" },
  SRT: { icon: IconListNumbers, extension: ".srt", group: "subtitle" },
  WebVTT: { icon: IconBrowser, extension: ".vtt", group: "subtitle" },
  JSON: { icon: IconBraces, extension: ".json", group: "data" },
  "Raw Data": { icon: IconBinary, extension: ".bin", group: "data" },
};

export function formatOptions(message: (key: string) => string) {
  return [
    {
      name: "ASS" as ExportFormat,
      description: message("format.assDescription"),
      icon: formatVisuals.ASS.icon,
      color: "purple",
    },
    {
      name: "TTML" as ExportFormat,
      description: message("format.ttmlDescription"),
      icon: formatVisuals.TTML.icon,
      color: "green",
    },
    {
      name: "SRT" as ExportFormat,
      description: message("format.srtDescription"),
      icon: formatVisuals.SRT.icon,
      color: "blue",
    },
    {
      name: "WebVTT" as ExportFormat,
      description: message("format.webvttDescription"),
      icon: formatVisuals.WebVTT.icon,
      color: "green",
    },
    {
      name: "JSON" as ExportFormat,
      description: message("format.jsonDescription"),
      icon: formatVisuals.JSON.icon,
      color: "orange",
    },
    {
      name: "Raw Data" as ExportFormat,
      description: message("format.rawDescription"),
      icon: formatVisuals["Raw Data"].icon,
      color: "blue",
    },
  ];
}

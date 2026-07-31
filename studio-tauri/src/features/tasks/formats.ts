import { Braces, Captions, Database, FileCode2, Subtitles } from "@lucide/svelte";
import type { ExportFormat } from "./controller";

export function formatOptions(message: (key: string) => string) {
  return [
    {
      name: "ASS" as ExportFormat,
      description: message("format.assDescription"),
      icon: FileCode2,
      color: "purple",
    },
    {
      name: "TTML" as ExportFormat,
      description: message("format.ttmlDescription"),
      icon: Braces,
      color: "green",
    },
    {
      name: "SRT" as ExportFormat,
      description: message("format.srtDescription"),
      icon: Captions,
      color: "blue",
    },
    {
      name: "WebVTT" as ExportFormat,
      description: message("format.webvttDescription"),
      icon: Subtitles,
      color: "green",
    },
    {
      name: "JSON" as ExportFormat,
      description: message("format.jsonDescription"),
      icon: Braces,
      color: "orange",
    },
    {
      name: "Raw Data" as ExportFormat,
      description: message("format.rawDescription"),
      icon: Database,
      color: "blue",
    },
  ];
}

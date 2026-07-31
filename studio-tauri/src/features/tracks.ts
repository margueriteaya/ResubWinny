import type { Track } from "../backend";
import { t } from "../i18n";

export const trackKey = (track: Track) =>
  `${track.kind ?? "unknown"}:${track.pid ?? `${track.label}:${track.detail}`}`;

export function trackDisplayLabel(track: Track) {
  const ordinal = String(track.ordinal ?? 1);
  switch (track.kind) {
    case "b24_verified":
      return t("tracks.b24Verified").replace("{0}", ordinal);
    case "mpeg_ts_ttml_candidate":
      return t("tracks.mpegTsTtmlCandidate").replace("{0}", ordinal);
    case "m2ts_ttml":
      return t("tracks.m2tsTtml").replace("{0}", ordinal);
    default:
      return track.label;
  }
}

export function trackDisplayDetail(track: Track) {
  if (track.kind === "b24_verified") {
    return [
      t("tracks.b24Detail"),
      track.serviceId ? `Service ${track.serviceId}` : "",
      track.language ?? "",
      track.serviceName ?? "",
    ].filter(Boolean).join(" · ");
  }
  if (track.kind === "mpeg_ts_ttml_candidate") return t("tracks.mpegTsTtmlDetail");
  if (track.kind === "m2ts_ttml") return t("tracks.m2tsTtmlDetail");
  return track.detail;
}

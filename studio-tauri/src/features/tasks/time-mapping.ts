import type { PlaybackTimeMapping } from "../../backend";

/** Domain-labelled millisecond values. Both remain runtime numbers so the
 * existing Tauri wire contract stays unchanged while callers migrate away
 * from ambiguous `timeMs` names. */
export type MediaTimeMs = number & { readonly __timeDomain: "media-ms" };
export type ProjectTimeMs = number & { readonly __timeDomain: "project-ms" };

export const mediaTimeMs = (value: number): MediaTimeMs => Math.trunc(value) as MediaTimeMs;
export const projectTimeMs = (value: number): ProjectTimeMs => Math.trunc(value) as ProjectTimeMs;

function validRate(mapping: PlaybackTimeMapping) {
  return Number.isFinite(mapping.rateNumerator)
    && Number.isFinite(mapping.rateDenominator)
    && mapping.rateNumerator > 0
    && mapping.rateDenominator > 0;
}

/** Mirrors PlaybackTimeMapping::project_time_ms in the desktop backend. */
export function mediaToProjectTime(mediaTime: MediaTimeMs, mapping: PlaybackTimeMapping): ProjectTimeMs {
  if (!Number.isFinite(mediaTime) || !validRate(mapping)) return projectTimeMs(0);
  const delta = Math.trunc(mediaTime) - Math.trunc(mapping.mediaAnchorMs);
  return projectTimeMs(mapping.projectAnchorMs + Math.trunc(delta * mapping.rateNumerator / mapping.rateDenominator));
}

/** Mirrors PlaybackTimeMapping::media_time_ms in the desktop backend. */
export function projectToMediaTime(projectTime: ProjectTimeMs, mapping: PlaybackTimeMapping): MediaTimeMs {
  if (!Number.isFinite(projectTime) || !validRate(mapping)) return mediaTimeMs(0);
  const delta = Math.trunc(projectTime) - Math.trunc(mapping.projectAnchorMs);
  return mediaTimeMs(mapping.mediaAnchorMs + Math.trunc(delta * mapping.rateDenominator / mapping.rateNumerator));
}

export function projectRangeForMedia(durationMs: MediaTimeMs | null, mapping: PlaybackTimeMapping) {
  const startMs = projectTimeMs(Math.max(0, mediaToProjectTime(mediaTimeMs(0), mapping)));
  // libmpv exposes duration just after the native surface is created.  Keep a
  // stable, useful provisional range while that value is unavailable instead
  // of rendering a misleading 0–5 second ruler that jumps as soon as the
  // first playback poll completes.
  const provisionalDurationMs = mediaTimeMs(120_000);
  const mediaDurationMs = durationMs == null
    ? provisionalDurationMs
    : mediaTimeMs(Math.max(0, durationMs));
  const mappedEnd = mediaToProjectTime(mediaDurationMs, mapping);
  return {
    startMs,
    endMs: projectTimeMs(Math.max(startMs + 5_000, mappedEnd)),
  };
}

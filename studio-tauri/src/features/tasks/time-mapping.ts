import type { PlaybackTimeMapping } from "../../backend";

function validRate(mapping: PlaybackTimeMapping) {
  return Number.isFinite(mapping.rateNumerator)
    && Number.isFinite(mapping.rateDenominator)
    && mapping.rateNumerator > 0
    && mapping.rateDenominator > 0;
}

/** Mirrors PlaybackTimeMapping::project_time_ms in the desktop backend. */
export function mediaToProjectTime(mediaTimeMs: number, mapping: PlaybackTimeMapping) {
  if (!Number.isFinite(mediaTimeMs) || !validRate(mapping)) return 0;
  const delta = Math.trunc(mediaTimeMs) - Math.trunc(mapping.mediaAnchorMs);
  return Math.trunc(mapping.projectAnchorMs + Math.trunc(delta * mapping.rateNumerator / mapping.rateDenominator));
}

/** Mirrors PlaybackTimeMapping::media_time_ms in the desktop backend. */
export function projectToMediaTime(projectTimeMs: number, mapping: PlaybackTimeMapping) {
  if (!Number.isFinite(projectTimeMs) || !validRate(mapping)) return 0;
  const delta = Math.trunc(projectTimeMs) - Math.trunc(mapping.projectAnchorMs);
  return Math.trunc(mapping.mediaAnchorMs + Math.trunc(delta * mapping.rateDenominator / mapping.rateNumerator));
}

export function projectRangeForMedia(durationMs: number | null, mapping: PlaybackTimeMapping) {
  const startMs = Math.max(0, mediaToProjectTime(0, mapping));
  // libmpv exposes duration just after the native surface is created.  Keep a
  // stable, useful provisional range while that value is unavailable instead
  // of rendering a misleading 0–5 second ruler that jumps as soon as the
  // first playback poll completes.
  const provisionalDurationMs = 120_000;
  const mediaDurationMs = durationMs == null
    ? provisionalDurationMs
    : Math.max(0, durationMs);
  const mappedEnd = mediaToProjectTime(mediaDurationMs, mapping);
  return {
    startMs,
    endMs: Math.max(startMs + 5_000, mappedEnd),
  };
}

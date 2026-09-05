type CaptionTrack = { logicalTrack: string };

export function hasCaptionTrack(tracks: readonly CaptionTrack[]): boolean {
  return tracks.length > 0;
}

export function selectedCaptionTrack<T extends CaptionTrack>(
  tracks: readonly T[],
  selectedTrackKeys: ReadonlySet<string>,
): T | undefined {
  return tracks.find((track) => selectedTrackKeys.has(track.logicalTrack));
}

export function hasSelectedCaptionTrack(
  tracks: readonly CaptionTrack[],
  selectedTrackKeys: ReadonlySet<string>,
): boolean {
  return Boolean(selectedCaptionTrack(tracks, selectedTrackKeys));
}

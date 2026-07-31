import { backend, type TimelineFeature } from "../../backend";

export function getTimelineWindow(archive: string, offset: number, limit: number) {
  return backend.getTimelineWindow(archive, offset, limit);
}

export function getFilteredTimelineWindow(archive: string, offset: number, limit: number, features: TimelineFeature[]) {
  return backend.getTimelineWindowFiltered(archive, offset, limit, features);
}

export function getRecentTimelineWindow(archive: string, limit: number, features: TimelineFeature[]) {
  return backend.getTimelineRecentWindowFiltered(archive, limit, features);
}

export function getTimelineTimeWindow(archive: string, beginMs: number, endMs: number, limit: number) {
  return backend.getTimelineTimeWindow(archive, beginMs, endMs, limit);
}

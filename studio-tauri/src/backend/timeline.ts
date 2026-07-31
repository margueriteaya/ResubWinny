import type { TimelineFeature, TimelineWindow } from '../backend'
import { call } from './client'

export const timelineApi = {
  getTimelineWindow: (archive: string, offset: number, limit: number) =>
    call<TimelineWindow>('get_timeline_window', { archive, offset, limit }),
  getTimelineWindowFiltered: (archive: string, offset: number, limit: number, features: TimelineFeature[]) =>
    call<TimelineWindow>('get_timeline_window_filtered', { archive, offset, limit, features }),
  getTimelineRecentWindowFiltered: (archive: string, limit: number, features: TimelineFeature[]) =>
    call<TimelineWindow>('get_timeline_recent_window_filtered', { archive, limit, features }),
  getTimelineTimeWindow: (archive: string, beginMs: number, endMs: number, limit: number) =>
    call<TimelineWindow>('get_timeline_time_window', { archive, beginMs, endMs, limit }),
}

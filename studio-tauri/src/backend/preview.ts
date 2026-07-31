import type { BroadcastMetadata, CaptionRenderSnapshot, PlaybackTimeMapping, PreviewCapabilities, PreviewCommand, PreviewOverlaySyncResult, PreviewPlaybackState, PreviewRect, PreviewRenderDiagnostics, PreviewRuntime } from '../backend'
import { call } from './client'

export const previewApi = {
  startPreview: (source: string, rect: PreviewRect) => call<void>('start_preview', { source, rect }),
  recoverPreview: (source: string, rect: PreviewRect, timeSeconds: number | null, paused: boolean, volume: number) =>
    call<void>('recover_preview', { source, rect, timeSeconds, paused, volume }),
  resizePreview: (rect: PreviewRect) => call<void>('resize_preview', { rect }),
  stopPreview: () => call<void>('stop_preview'),
  previewCommand: (command: PreviewCommand) => call<void>('preview_command', { command }),
  setCaptionFont: (font: string) => call<void>('set_caption_font', { font }),
  getPreviewCapabilities: () => call<PreviewCapabilities>('get_preview_capabilities'),
  getPreviewRuntime: () => call<PreviewRuntime>('get_preview_runtime'),
  getPreviewRenderDiagnostics: () => call<PreviewRenderDiagnostics>('get_preview_render_diagnostics'),
  clearCaptionOverlay: () => call<void>('clear_caption_overlay'),
  getPreviewTime: () => call<number | null>('get_preview_time'),
  getPreviewDuration: () => call<number | null>('get_preview_duration'),
  getPreviewPlaybackState: () => call<PreviewPlaybackState>('get_preview_playback_state'),
  getPreviewBroadcastMetadata: (serviceId?: number) => call<BroadcastMetadata>('get_preview_broadcast_metadata', { serviceId }),
  seekPreviewAbsolute: (seconds: number) => call<void>('preview_command', { command: `seek-absolute:${Math.max(0, seconds)}` }),
  setPreviewVolume: (volume: number) => call<void>('preview_command', { command: `set-volume:${Math.min(100, Math.max(0, volume))}` }),
  getPlaybackTimeMapping: () => call<PlaybackTimeMapping>('get_playback_time_mapping'),
  updatePlaybackTimeMapping: (mapping: PlaybackTimeMapping) => call<void>('update_playback_time_mapping', { mapping }),
  renderAt: (archive: string, timeMs: number) => call<CaptionRenderSnapshot>('render_at', { archive, timeMs }),
  renderPreviewAt: (archive: string, timeMs: number) => call<CaptionRenderSnapshot>('render_preview_at', { archive, timeMs, x: 0, y: 0 }),
  syncPreviewOverlay: (archive: string) => call<PreviewOverlaySyncResult>('sync_preview_overlay', { archive }),
}

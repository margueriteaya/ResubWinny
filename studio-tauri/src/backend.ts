import { drcsApi } from './backend/drcs'
import { subscribeTaskEvents } from './backend/events'
import { inspectionApi } from './backend/inspection'
import { jobsApi } from './backend/jobs'
import { previewApi } from './backend/preview'
import { settingsApi } from './backend/settings'
import { timelineApi } from './backend/timeline'

export type Track = { label: string; detail: string; pid?: string; kind?: string; ordinal?: number; serviceId?: number; language?: string; serviceName?: string }
export type BroadcastMetadata = { networkName?: string | null; programmeName?: string | null; programmeDescription?: string | null; broadcastTimeUtc?: string | null }
export type PreviewPlaybackState = { timeSeconds: number | null; durationSeconds: number | null; paused: boolean | null }
export type Inspection = { path: string; name: string; size: number; container: string; packetSize?: number; routeCode?: string; route?: string; service: string; tracks: Track[]; broadcast: BroadcastMetadata }
export type DrcsGlyph = { id: string; width: number; height: number; alternativeText: string; image: string }
export type DrcsMapping = { id: string; text: string; action: 'image' | 'character' | 'font' }
export type ExportFormat = 'ASS' | 'TTML' | 'SRT' | 'WebVTT' | 'JSON' | 'Raw Data'
export type ExportPreservation = { position: boolean; color: boolean; ruby: boolean; drcs: boolean; gaiji: boolean; accessibility: boolean }
export type WorkspaceLayoutSettings = { sourceWidth: number; outputWidth: number; sourceCollapsed: boolean; outputCollapsed: boolean }
export type AppSettings = { uiFont: 'system' | 'cjk' | 'arib'; captionFont: 'arib' | 'system'; defaultFormat: 'ASS' | 'TTML' | 'JSON' | 'Raw Data'; locale: string; theme: 'system' | 'light' | 'dark'; workspaceLayout: WorkspaceLayoutSettings }
export type LanguagePack = { locale: string; name: string; messages: Record<string, string> }
export type AboutInfo = { productName: string; description: string; version: string; channel: string; platform: string; architecture: string; releaseTier: string; buildTag: string | null; buildCommit: string | null; signingDeclaration: 'development' | 'unsigned-alpha' | 'declared-signed' }
export type TaskHistoryRecord = { name: string; path: string; size: number; container: string; status: string; time: string; warnings: number; captions: number; jobId?: string }
export type PreviewCommand = 'toggle-pause' | 'seek-back' | 'seek-forward' | 'frame-back' | 'frame-forward'
export type PreviewRect = { x: number; y: number; width: number; height: number }
export type PreviewSurfaceCapability = { id: string; available: boolean; experimental: boolean; unavailableReasonCode: string | null }
export type PreviewCapabilities = { videoBackend: string; captionOverlayModes: PreviewSurfaceCapability[]; selectedCaptionOverlay: string; captionPlaneModes: string[]; availableCaptionPlaneModes: string[] }
export type PreviewRuntime = { backend: string; platform: string; libraryPath: string | null; available: boolean; renderApiAvailable: boolean; detail: string }
export type PreviewRenderDiagnostics = {
  route: string
  active: boolean
  framesPresented: number
  presentsPerSecond: number
  captionTextureUploads: number
  captionTextureClears: number
  videoAspect: number | null
  surfaceWidth: number | null
  surfaceHeight: number | null
  decoderMode: string | null
  fallbackReason: string | null
  lastError: string | null
}
export type JobState = 'Created' | 'Inspecting' | 'Ready' | 'Queued' | 'Starting' | 'Running' | 'Pausing' | 'Paused' | 'Resuming' | 'Cancelling' | 'Cancelled' | 'Completed' | 'Failed' | 'Interrupted'
export type JobRecord = { jobId: string; source: string; output: string; archive: boolean; raw: boolean; trackId?: number; drcsReport: boolean; drcsMappings: DrcsMapping[]; exportSelection: { formats: ExportFormat[]; preservation: ExportPreservation }; state: JobState; createdAt: number; updatedAt: number }
export type DiagnosticRecord = { timestamp: number; severity: string; code: string; parameters: Record<string, unknown>; message: string }
export type ArtifactRecord = { kind: string; path: string; temporaryPath: string; status: string; existedBeforeStart: boolean }
export type CheckpointRecord = { jobId: string; source: string; output: string; bytesRead: number; captions: number; warnings: number; strategy: string; updatedAt: number }
export type CaptionRenderProfile = { renderer: string; fontFamily: string; preserveCharacterCells: boolean; rubyScale: number; backgroundAlphaFromSource: boolean; strokeFromSource: boolean; drcsPolicy: string }
export type CaptionRenderSnapshot = { source: string; timeMs: number; intervals: Record<string, unknown>[]; resourcePreviews: Record<string, unknown>[]; planeWidth?: number; planeHeight?: number; composedPngBase64?: string; activeLayerCount: number; captionPlaneMode: string; missingGlyphCount: number; renderedRubyCount: number; renderProfile: CaptionRenderProfile }
export type PreviewOverlaySyncResult = { action: 'applied' | 'cleared' | 'unchanged' | 'superseded' | 'awaiting-player-time' | 'awaiting-caption-index'; mediaTimeMs: number | null; projectTimeMs: number | null; snapshot: CaptionRenderSnapshot | null }
export type PlaybackTimeMapping = { segmentId: string; mediaAnchorMs: number; projectAnchorMs: number; rateNumerator: number; rateDenominator: number }
export type TimelineFeature = 'position' | 'color' | 'ruby' | 'drcs' | 'gaiji' | 'accessibility'
export type TimelineHighlight = { start: number; end: number; feature: TimelineFeature }
export type TimelineColor = { role: 'text' | 'background'; value: string }
export type TimelineEvent = { index: number; kind: 'region_interval' | 'caption' | 'scene'; beginMs: number; endMs: number; text: string; regionX: number | null; regionY: number | null; trackId: string | null; features: TimelineFeature[]; highlights: TimelineHighlight[]; colors: TimelineColor[] }
export type TimelineWindow = { items: TimelineEvent[]; hasMore: boolean }
export type WorkerEvent = { protocolVersion?: number; jobId?: string; sequence?: number; type?: string; kind?: string; payload?: Record<string, unknown>; message?: string; bytesRead?: number; captions?: number; warnings?: number; output?: string; [key: string]: unknown }
export type TaskEvent = { jobId?: string; kind: string; code?: string; parameters?: Record<string, unknown>; message: string; bytesRead?: number; captions?: number; warnings?: number; output?: string }

// This is deliberately the only public browser-to-Rust boundary. Components
// consume domain operations, never Tauri command strings or transport payloads.
export const backend = {
  ...inspectionApi,
  ...drcsApi,
  ...settingsApi,
  ...previewApi,
  ...timelineApi,
  ...jobsApi,
  subscribeTaskEvents,
}

<script lang="ts">
  import { ChevronRight, CircleCheck, CirclePlay, ListVideo, LoaderCircle, SquarePlay, Stethoscope, TriangleAlert } from "@lucide/svelte";
  import type { PlaybackTimeMapping, PreviewCommand, UserMode } from "../../backend";
  import { t } from "../../i18n";
  import TaskDiagnostics from "./TaskDiagnostics.svelte";
  import TaskTimeline from "./TaskTimeline.svelte";
  import PlayerControls from "./PlayerControls.svelte";
  import MacSegmentedControl from "../../components/MacSegmentedControl.svelte";
  import { projectRangeForMedia, type MediaTimeMs, type ProjectTimeMs } from "./time-mapping";

  type TaskTab = "preview" | "events" | "diagnostics";
  let {
    taskTab = "preview",
    userMode = "normie",
    currentJobId = "",
    desktopRuntime = false,
    archivePath = "",
    logs = [],
    captions = 0,
    warnings = 0,
    selectedTrackCount = 0,
    diagnosticsCount = 0,
    bytesRead = 0,
    progress = 0,
    isExporting = false,
    previewIndexing = false,
    compactViewport = false,
    playerRunning = false,
    playerPaused = true,
    previewAvailable = null,
    previewVolume = 100,
    nativePreview = $bindable(null),
    playbackMapping = $bindable(),
    appliedPlaybackMapping,
    playbackMappingBusy = false,
    projectTimeMs = 0 as ProjectTimeMs,
    durationMs = null,
    trackLabel = "",
    trackName = "",
    trackDetail = "",
    onSelectTab = () => {},
    onPlayerCommand = () => {},
    onStartPreview = () => {},
    onStopPreview = () => {},
    onResizePreview = () => {},
    onSeekProject = () => {},
    onSeekTarget = () => {},
    onSetVolume = () => {},
    onToggleMute = () => {},
    onSaveMapping = () => {},
    onDiagnosticsCount = () => {},
    onError = () => {},
  }: {
    taskTab?: TaskTab;
    userMode?: UserMode;
    currentJobId?: string;
    desktopRuntime?: boolean;
    archivePath?: string;
    logs?: string[];
    captions?: number;
    warnings?: number;
    selectedTrackCount?: number;
    diagnosticsCount?: number;
    bytesRead?: number;
    progress?: number;
    isExporting?: boolean;
    previewIndexing?: boolean;
    compactViewport?: boolean;
    playerRunning?: boolean;
    playerPaused?: boolean;
    previewAvailable?: boolean | null;
    previewVolume?: number;
    nativePreview?: HTMLDivElement | null;
    playbackMapping: PlaybackTimeMapping;
    appliedPlaybackMapping: PlaybackTimeMapping;
    playbackMappingBusy?: boolean;
    projectTimeMs?: ProjectTimeMs;
    durationMs?: MediaTimeMs | null;
    trackLabel?: string;
    trackName?: string;
    trackDetail?: string;
    onSelectTab?: (tab: TaskTab) => void;
    onPlayerCommand?: (command: PreviewCommand) => void;
    onStartPreview?: () => void;
    onStopPreview?: () => void;
    onResizePreview?: () => void;
    onSeekProject?: (milliseconds: ProjectTimeMs, final?: boolean) => void | Promise<void>;
    onSeekTarget?: (milliseconds: ProjectTimeMs, final?: boolean) => void;
    onSetVolume?: (volume: number) => void;
    onToggleMute?: () => void;
    onSaveMapping?: () => void;
    onDiagnosticsCount?: (count: number) => void;
    onError?: (message: string) => void;
  } = $props();
  let playbackMappingDetails: HTMLDetailsElement | undefined = $state();

  // Pane drags change this rectangle without resizing the document or window.
  // Follow the actual surface so its native child HWND stays inside the player.
  $effect(() => {
    if (!desktopRuntime || !nativePreview) return;
    const observer = new ResizeObserver(() => onResizePreview());
    observer.observe(nativePreview);
    return () => observer.disconnect();
  });

  function openPlaybackMapping() {
    const details = playbackMappingDetails;
    if (!details) return;
    details.open = true;
    requestAnimationFrame(() => {
      const behavior = matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth";
      details.scrollIntoView({ block: "nearest", behavior });
      details.querySelector<HTMLElement>("summary")?.focus();
    });
  }

  const taskTabOptions = $derived([
    { value: "preview", label: t("workspace.captionPreview"), icon: SquarePlay },
    { value: "events", label: `${t("workspace.eventList")} · ${captions.toLocaleString()}`, icon: ListVideo },
    { value: "diagnostics", label: `${t("workspace.diagnostics")} · ${diagnosticsCount}`, icon: Stethoscope },
  ].filter((option) => option.value !== "diagnostics" || userMode === "nerd" || diagnosticsCount > 0 || warnings > 0));
  const projectRange = $derived(projectRangeForMedia(durationMs, appliedPlaybackMapping));
  const mappingIsAutomatic = $derived(
    playbackMapping.segmentId === "recording-origin"
      && playbackMapping.mediaAnchorMs === 0
      && playbackMapping.projectAnchorMs === 0
      && playbackMapping.rateNumerator === 1
      && playbackMapping.rateDenominator === 1,
  );
  const mappingStatus = $derived(
    mappingIsAutomatic ? t("preview.mappingAuto") : t("preview.mappingAdjusted"),
  );
  const workbenchStatus = $derived(
    isExporting
      ? t("task.statusExporting").replace("{0}", progress.toFixed(0))
      : previewIndexing
        ? t("task.statusIndexing").replace("{0}", progress.toFixed(0))
        : diagnosticsCount || warnings
          ? t("task.statusWarnings").replace("{0}", String(Math.max(diagnosticsCount, warnings)))
          : t("task.statusReady").replace("{0}", String(selectedTrackCount)),
  );
</script>

<section class="preview-panel">
  <div class="tabs">
    <span aria-hidden="true"></span>
    <MacSegmentedControl size="toolbar" iconOnly={compactViewport} ariaLabel={t("app.navigation")} value={taskTab} options={taskTabOptions} onChange={(value) => onSelectTab(value as TaskTab)} />
    <p class:warning={Boolean(diagnosticsCount || warnings) && !isExporting && !previewIndexing} class:active={isExporting || previewIndexing} class="workbench-status" role="status" aria-live="polite">
      {#if isExporting || previewIndexing}<LoaderCircle size={15} />{:else if diagnosticsCount || warnings}<TriangleAlert size={15} />{:else}<CircleCheck size={15} />{/if}
      <span>{workbenchStatus}</span>
    </p>
  </div>
  {#if taskTab === "preview"}
    <div class="player-shell">
      <div class="native-preview" data-liquid-ignore bind:this={nativePreview}><div class="native-notice"><CirclePlay size={30} /><b>{playerRunning ? t("workspace.nativePreviewActive") : t("workspace.nativePreview")}</b><p>{playerRunning ? t("workspace.nativePreviewActiveDescription") : t("workspace.nativePreviewDescription")}</p></div></div>
      <PlayerControls running={playerRunning} paused={playerPaused} available={previewAvailable} {projectTimeMs} rangeStartMs={projectRange.startMs} rangeEndMs={projectRange.endMs} durationKnown={durationMs !== null} volume={previewVolume} onStart={onStartPreview} onTogglePause={() => onPlayerCommand("toggle-pause")} onStop={onStopPreview} onFit={onResizePreview} onSkipBack={() => onPlayerCommand("seek-back")} onSkipForward={() => onPlayerCommand("seek-forward")} {onToggleMute} {onSetVolume} {onSeekProject} {onSeekTarget} {onError} />
    </div>
    <div class="preview-status"><span>{t("workspace.scanned").replace("{0}", (bytesRead / 1024 ** 3).toFixed(2))}</span><span>{t("workspace.decodedEvents").replace("{0}", captions.toLocaleString())}</span></div>
    <TaskTimeline {archivePath} {desktopRuntime} live={isExporting || previewIndexing} editor {trackLabel} {trackName} {trackDetail} projectTimeMs={projectTimeMs} rangeStartMs={projectRange.startMs} rangeEndMs={projectRange.endMs} playing={playerRunning && !playerPaused} expectedCount={captions} onSeek={onSeekProject} {onSeekTarget} onOpenMapping={openPlaybackMapping} {onError} />
    <details class="playback-mapping" bind:this={playbackMappingDetails}>
      <summary>
        <span class="mapping-summary"><b>{t("preview.mappingTitle")}</b><small>{mappingStatus}</small></span>
        <span class="mapping-adjust">{t("preview.mappingAdjust")}<ChevronRight size={16} /></span>
      </summary>
      <div class="mapping-controls">
        <p>{t("preview.mappingDescription")}</p>
        <label>{t("preview.mappingSegment")}<input bind:value={playbackMapping.segmentId} /></label>
        <label>{t("preview.mappingMediaAnchor")}<input type="number" step="1" bind:value={playbackMapping.mediaAnchorMs} /></label>
        <label>{t("preview.mappingProjectAnchor")}<input type="number" step="1" bind:value={playbackMapping.projectAnchorMs} /></label>
        <label>{t("preview.mappingRateNumerator")}<input type="number" min="1" step="1" bind:value={playbackMapping.rateNumerator} /></label>
        <label>{t("preview.mappingRateDenominator")}<input type="number" min="1" step="1" bind:value={playbackMapping.rateDenominator} /></label>
        <button class="quiet-button" onclick={onSaveMapping} disabled={playbackMappingBusy}>{playbackMappingBusy ? t("preview.mappingApplying") : t("preview.mappingApply")}</button>
      </div>
    </details>
    {#if previewAvailable === false}<p class="preview-unavailable">{t("preview.runtimeUnavailable")}</p>{/if}
  {:else if taskTab === "events"}
    <TaskTimeline {archivePath} {desktopRuntime} live={isExporting || previewIndexing} {trackLabel} {trackName} {trackDetail} expectedCount={captions} projectTimeMs={projectTimeMs} rangeStartMs={projectRange.startMs} rangeEndMs={projectRange.endMs} playing={playerRunning && !playerPaused} onSeek={onSeekProject} {onSeekTarget} {onError} />
  {:else}<TaskDiagnostics jobId={currentJobId} {desktopRuntime} {logs} onCountChange={onDiagnosticsCount} {onError} />{/if}
</section>

<style>
  .preview-panel { --rw-timeline-gutter: 92px; --rw-timeline-axis-inset: 10px; }
  .tabs { display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); column-gap: 12px; }
  .workbench-status { display: flex; justify-self: end; align-items: center; gap: 6px; margin: 0; min-width: 0; color: var(--rw-success); font-size: 12px; line-height: 16px; white-space: nowrap; }
  .workbench-status.warning { color: var(--rw-warning); }.workbench-status.active { color: var(--rw-accent); }.workbench-status.active :global(svg) { animation: status-spin 1.2s linear infinite; }
  .player-shell { container: player / inline-size; margin-top: 10px; overflow: hidden; border: 1px solid var(--rw-border); border-radius: 6px; background: var(--rw-content); }
  .native-preview { margin: 0; min-height: clamp(270px, 38vh, 380px); border-radius: 0; }
  .preview-status { display:flex; justify-content:space-between; padding:8px 1px 0; color:var(--rw-muted); font-size:12px; line-height:16px; }
  .playback-mapping { margin-top:10px; overflow:hidden; border:1px solid var(--rw-border-subtle); border-radius:7px; background:var(--rw-content); }
  .playback-mapping summary { display:flex; align-items:center; justify-content:space-between; min-height:48px; padding:7px 11px; cursor:pointer; list-style:none; }.playback-mapping summary::-webkit-details-marker { display:none; }
  .mapping-summary { display:grid; gap:2px; min-width:0; }.mapping-summary b { font-size:13px; line-height:17px; font-weight:680; }.mapping-summary small { overflow:hidden; color:var(--rw-text-secondary); font-size:12px; line-height:16px; text-overflow:ellipsis; white-space:nowrap; }
  .mapping-adjust { display:flex; align-items:center; gap:3px; flex:0 0 auto; color:var(--rw-accent); font-size:12px; font-weight:620; }.mapping-adjust :global(svg) { transition:transform var(--rw-motion-responsive) var(--rw-ease-out); }.playback-mapping[open] .mapping-adjust :global(svg) { transform:rotate(90deg); }
  .mapping-controls { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:10px; padding:12px; border-top:1px solid var(--rw-border-subtle); }.mapping-controls>p { grid-column:1/-1; margin:0 0 2px; color:var(--rw-text-secondary); font-size:12px; line-height:1.5; }.mapping-controls label { display:grid; gap:5px; min-width:0; color:var(--rw-text-secondary); font-size:12px; font-weight:620; }.mapping-controls input { min-width:0; width:100%; padding:7px 8px; font-size:12px; }.mapping-controls .quiet-button { grid-column:1/-1; justify-self:start; min-height:32px; border:1px solid var(--rw-border); border-radius:6px; color:var(--rw-text); background:var(--rw-surface-muted); font-size:12px; }
  @keyframes status-spin { to { transform:rotate(1turn); } }
  @container content (max-width: 700px) { .tabs { grid-template-columns:auto minmax(0,1fr); }.tabs>span[aria-hidden="true"] { display:none; }.workbench-status { overflow:hidden; }.workbench-status span { overflow:hidden; text-overflow:ellipsis; }.mapping-controls { grid-template-columns:1fr; } }
  @media (max-height:700px) { .native-preview { min-height:168px; } }
  @media (prefers-reduced-motion: reduce) { .workbench-status.active :global(svg) { animation:none; } }
</style>

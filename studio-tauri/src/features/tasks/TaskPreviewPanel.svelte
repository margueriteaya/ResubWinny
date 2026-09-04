<script lang="ts">
  import { onDestroy } from "svelte";
  import { ChevronRight, CircleCheck, CirclePlay, ListVideo, LoaderCircle, Maximize2, Pause, Play, Square, SquarePlay, Stethoscope, TriangleAlert, Volume2 } from "@lucide/svelte";
  import type { PlaybackTimeMapping, PreviewCommand, UserMode } from "../../backend";
  import { t } from "../../i18n";
  import TaskDiagnostics from "./TaskDiagnostics.svelte";
  import TaskTimeline from "./TaskTimeline.svelte";
  import MacSlider from "../../components/MacSlider.svelte";
  import MacSegmentedControl from "../../components/MacSegmentedControl.svelte";
  import { projectRangeForMedia, projectTimeMs as asProjectTimeMs, type MediaTimeMs, type ProjectTimeMs } from "./time-mapping";

  type TaskTab = "preview" | "events" | "diagnostics";
  export let taskTab: TaskTab = "preview";
  export let userMode: UserMode = "normie";
  export let currentJobId = "";
  export let desktopRuntime = false;
  export let archivePath = "";
  export let logs: string[] = [];
  export let captions = 0;
  export let warnings = 0;
  export let selectedTrackCount = 0;
  export let diagnosticsCount = 0;
  export let bytesRead = 0;
  export let progress = 0;
  export let isExporting = false;
  export let previewIndexing = false;
  export let compactViewport = false;
  export let playerRunning = false;
  export let playerPaused = true;
  export let previewAvailable: boolean | null = null;
  export let nativePreview: HTMLDivElement | null = null;
  export let playbackMapping: PlaybackTimeMapping;
  export let appliedPlaybackMapping: PlaybackTimeMapping;
  export let playbackMappingBusy = false;
  export let projectTimeMs: ProjectTimeMs = 0 as ProjectTimeMs;
  export let durationMs: MediaTimeMs | null = null;
  export let trackLabel = "";
  export let trackName = "";
  export let trackDetail = "";
  export let onSelectTab: (tab: TaskTab) => void = () => {};
  export let onPlayerCommand: (command: PreviewCommand) => void = () => {};
  export let onStartPreview: () => void = () => {};
  export let onStopPreview: () => void = () => {};
  export let onResizePreview: () => void = () => {};
  export let onSeekProject: (milliseconds: ProjectTimeMs, final?: boolean) => void | Promise<void> = () => {};
  export let onSeekTarget: (milliseconds: ProjectTimeMs, final?: boolean) => void = () => {};
  export let onSetVolume: (volume: number) => void = () => {};
  export let onSaveMapping: () => void = () => {};
  export let onDiagnosticsCount: (count: number) => void = () => {};
  export let onError: (message: string) => void = () => {};
  let playbackMappingDetails: HTMLDetailsElement;
  let scrubberActive = false;
  let scrubberTargetMs = 0;
  let scrubberFrame: number | undefined;
  let pendingScrubberTarget: number | null = null;

  function openPlaybackMapping() {
    playbackMappingDetails.open = true;
    requestAnimationFrame(() => {
      const behavior = matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth";
      playbackMappingDetails.scrollIntoView({ block: "nearest", behavior });
      playbackMappingDetails.querySelector<HTMLElement>("summary")?.focus();
    });
  }

  const formatTime = (milliseconds: number) => {
    const seconds = Math.max(0, Math.floor(milliseconds / 1000));
    const hours = Math.floor(seconds / 3_600);
    const minutes = Math.floor(seconds / 60) % 60;
    const body = `${String(minutes).padStart(hours ? 2 : 1, "0")}:${String(seconds % 60).padStart(2, "0")}`;
    return hours ? `${hours}:${body}` : body;
  };

  function queueScrubberSeek(timeMs: number, final: boolean) {
    scrubberTargetMs = Math.max(projectRange.startMs, Math.min(projectRange.endMs, Math.round(timeMs)));
    onSeekTarget(asProjectTimeMs(scrubberTargetMs), final);
    scrubberActive = !final;
    if (final) {
      pendingScrubberTarget = null;
      if (scrubberFrame !== undefined) {
        cancelAnimationFrame(scrubberFrame);
        scrubberFrame = undefined;
      }
      dispatchScrubberSeek(scrubberTargetMs, true);
      return;
    }
    pendingScrubberTarget = scrubberTargetMs;
    if (scrubberFrame === undefined)
      scrubberFrame = requestAnimationFrame(flushScrubberSeekFrame);
  }

  function dispatchScrubberSeek(timeMs: number, final: boolean) {
    try {
      const operation = onSeekProject(asProjectTimeMs(timeMs), final);
      if (operation && typeof (operation as Promise<void>).catch === "function")
        void Promise.resolve(operation).catch((reason) => onError(String(reason)));
    } catch (reason) {
      onError(String(reason));
    }
  }

  function flushScrubberSeekFrame() {
    scrubberFrame = undefined;
    if (pendingScrubberTarget === null) return;
    const target = pendingScrubberTarget;
    pendingScrubberTarget = null;
    dispatchScrubberSeek(target, false);
  }

  function cancelScrubber() {
    if (!scrubberActive) return;
    queueScrubberSeek(scrubberTargetMs, true);
  }
  $: taskTabOptions = [
    { value: "preview", label: t("workspace.captionPreview"), icon: SquarePlay },
    { value: "events", label: `${t("workspace.eventList")} · ${captions.toLocaleString()}`, icon: ListVideo },
    { value: "diagnostics", label: `${t("workspace.diagnostics")} · ${diagnosticsCount}`, icon: Stethoscope },
  ].filter((option) => option.value !== "diagnostics" || userMode === "nerd" || diagnosticsCount > 0 || warnings > 0);
  $: projectRange = projectRangeForMedia(durationMs, appliedPlaybackMapping);
  $: scrubberValueMs = scrubberActive ? scrubberTargetMs : projectTimeMs;
  $: mappingIsAutomatic = playbackMapping.segmentId === "recording-origin"
    && playbackMapping.mediaAnchorMs === 0
    && playbackMapping.projectAnchorMs === 0
    && playbackMapping.rateNumerator === 1
    && playbackMapping.rateDenominator === 1;
  $: mappingStatus = mappingIsAutomatic ? t("preview.mappingAuto") : t("preview.mappingAdjusted");
  $: workbenchStatus = isExporting
    ? t("task.statusExporting").replace("{0}", progress.toFixed(0))
    : previewIndexing
      ? t("task.statusIndexing").replace("{0}", progress.toFixed(0))
      : diagnosticsCount || warnings
        ? t("task.statusWarnings").replace("{0}", String(Math.max(diagnosticsCount, warnings)))
        : t("task.statusReady").replace("{0}", String(selectedTrackCount));
  onDestroy(() => {
    if (scrubberFrame !== undefined) cancelAnimationFrame(scrubberFrame);
    if (scrubberActive || pendingScrubberTarget !== null) {
      scrubberActive = false;
      onSeekTarget(asProjectTimeMs(scrubberTargetMs), true);
      dispatchScrubberSeek(scrubberTargetMs, true);
    }
  });
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
      <div class="player-controls">
        <div class="player-time"><span>{formatTime(scrubberValueMs)}</span><span>/ {durationMs ? formatTime(projectRange.endMs) : "--:--"}</span></div>
        <MacSlider className="player-scrubber" ariaLabel={t("preview.seekTimeline")} min={projectRange.startMs} max={projectRange.endMs} value={scrubberValueMs} disabled={!playerRunning || !durationMs} onInput={(value) => queueScrubberSeek(value, false)} onChange={(value) => queueScrubberSeek(value, true)} onCancel={cancelScrubber} />
        <div class="player-buttons"><button class:play-icon={!playerRunning || playerPaused} class="player-button primary" data-tooltip={playerRunning ? t("workspace.pauseResume") : t("common.startPreview")} aria-label={playerRunning ? t("workspace.pauseResume") : t("common.startPreview")} onclick={playerRunning ? () => onPlayerCommand("toggle-pause") : onStartPreview} disabled={!playerRunning && previewAvailable === false}>{#if playerRunning && !playerPaused}<Pause size={18} />{:else}<Play size={18} />{/if}</button><span class="volume"><Volume2 size={17} /><MacSlider ariaLabel={t("preview.volume")} min={0} max={100} value={100} disabled={!playerRunning} onChange={onSetVolume} /></span><button class="player-button" data-tooltip={t("workspace.fitPreview")} aria-label={t("workspace.fitPreview")} onclick={onResizePreview} disabled={!playerRunning}><Maximize2 size={16} /></button><button class="player-button stop" data-tooltip={t("common.stopPreview")} aria-label={t("common.stopPreview")} onclick={onStopPreview} disabled={!playerRunning}><Square size={15} /></button></div>
      </div>
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
  .player-shell { margin-top: 19px; overflow: hidden; border: 1px solid var(--rw-border); border-radius: 7px; background: #05080c; }
  .native-preview { margin: 0; min-height: 304px; border-radius: 0; }
  .player-controls { display: grid; grid-template-columns: var(--rw-timeline-gutter) minmax(120px, 1fr); column-gap: 0; row-gap: 10px; align-items: center; padding: 10px; color: #dfe8f3; background: #111923; }
  .player-time { display: flex; width: var(--rw-timeline-gutter); min-width: 0; padding-right: 11px; gap: 5px; font: 12px "Cascadia Mono", monospace; white-space: nowrap; }
  .player-time span + span { color: #92a3b5; }
  :global(.player-scrubber){width:100%}
  .player-buttons { grid-column: 1 / -1; display: flex; align-items: center; gap: 6px; }
  .player-button { display:grid; place-items:center; width:34px; height:32px; padding:0; color:#d9e4f0; border:1px solid #314152; border-radius:5px; background:#182330; }.player-button :global(svg){display:block;margin:0}.player-button.play-icon :global(svg){transform:translateX(1px)}
  .player-button:hover:not(:disabled) { background:#24354a; }.player-button.primary { color:#fff; background:#1766e7; border-color:#3680ec; }.player-button.stop { margin-left:auto; }.volume{display:flex;align-items:center;gap:7px;min-width:130px;margin-left:8px;color:#b6c7d8}.volume :global(.mac-slider){width:92px}
  .preview-status { display:flex; justify-content:space-between; padding:8px 1px 0; color:var(--rw-muted); font-size:12px; line-height:16px; }
  .playback-mapping { margin-top:10px; overflow:hidden; border:1px solid var(--rw-border-subtle); border-radius:7px; background:var(--rw-content); }
  .playback-mapping summary { display:flex; align-items:center; justify-content:space-between; min-height:48px; padding:7px 11px; cursor:pointer; list-style:none; }.playback-mapping summary::-webkit-details-marker { display:none; }
  .mapping-summary { display:grid; gap:2px; min-width:0; }.mapping-summary b { font-size:13px; line-height:17px; font-weight:680; }.mapping-summary small { overflow:hidden; color:var(--rw-text-secondary); font-size:12px; line-height:16px; text-overflow:ellipsis; white-space:nowrap; }
  .mapping-adjust { display:flex; align-items:center; gap:3px; flex:0 0 auto; color:var(--rw-accent); font-size:12px; font-weight:620; }.mapping-adjust :global(svg) { transition:transform var(--rw-motion-responsive) var(--rw-ease-out); }.playback-mapping[open] .mapping-adjust :global(svg) { transform:rotate(90deg); }
  .mapping-controls { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:10px; padding:12px; border-top:1px solid var(--rw-border-subtle); }.mapping-controls>p { grid-column:1/-1; margin:0 0 2px; color:var(--rw-text-secondary); font-size:12px; line-height:1.5; }.mapping-controls label { display:grid; gap:5px; min-width:0; color:var(--rw-text-secondary); font-size:12px; font-weight:620; }.mapping-controls input { min-width:0; width:100%; padding:7px 8px; font-size:12px; }.mapping-controls .quiet-button { grid-column:1/-1; justify-self:start; min-height:32px; border:1px solid var(--rw-border); border-radius:6px; color:var(--rw-text); background:var(--rw-surface-muted); font-size:12px; }
  @keyframes status-spin { to { transform:rotate(1turn); } }
  @container content (max-width: 700px) { .tabs { grid-template-columns:auto minmax(0,1fr); }.tabs>span[aria-hidden="true"] { display:none; }.workbench-status { overflow:hidden; }.workbench-status span { overflow:hidden; text-overflow:ellipsis; }.mapping-controls { grid-template-columns:1fr; } }
  @media (prefers-reduced-motion: reduce) { .workbench-status.active :global(svg) { animation:none; } }
</style>

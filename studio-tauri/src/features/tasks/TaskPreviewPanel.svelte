<script lang="ts">
  import { CirclePlay, FileText, Maximize2, Pause, Play, RotateCcw, SkipBack, SkipForward, Square, StepBack, StepForward, Volume2 } from "@lucide/svelte";
  import type { PlaybackTimeMapping, PreviewCommand } from "../../backend";
  import { t } from "../../i18n";
  import TaskDiagnostics from "./TaskDiagnostics.svelte";
  import TaskTimeline from "./TaskTimeline.svelte";

  type TaskTab = "preview" | "events" | "diagnostics";
  export let taskTab: TaskTab = "preview";
  export let currentJobId = "";
  export let desktopRuntime = false;
  export let archivePath = "";
  export let captions = 0;
  export let diagnosticsCount = 0;
  export let bytesRead = 0;
  export let progress = 0;
  export let isExporting = false;
  export let playerRunning = false;
  export let playerPaused = true;
  export let previewAvailable: boolean | null = null;
  export let nativePreview: HTMLDivElement | null = null;
  export let playbackMapping: PlaybackTimeMapping;
  export let playbackMappingBusy = false;
  export let mediaTimeMs: number | null = null;
  export let durationMs: number | null = null;
  export let trackLabel = "";
  export let onSelectTab: (tab: TaskTab) => void = () => {};
  export let onPlayerCommand: (command: PreviewCommand) => void = () => {};
  export let onStartPreview: () => void = () => {};
  export let onStopPreview: () => void = () => {};
  export let onResizePreview: () => void = () => {};
  export let onSeekAbsolute: (milliseconds: number) => void = () => {};
  export let onSetVolume: (volume: number) => void = () => {};
  export let onSaveMapping: () => void = () => {};
  export let onDiagnosticsCount: (count: number) => void = () => {};
  export let onError: (message: string) => void = () => {};

  const formatTime = (milliseconds: number) => {
    const seconds = Math.max(0, Math.floor(milliseconds / 1000));
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  };
</script>

<section class="preview-panel">
  <div class="tabs">
    <button class:selected={taskTab === "preview"} onclick={() => onSelectTab("preview")}>{t("workspace.captionPreview")}</button><button class:selected={taskTab === "events"} onclick={() => onSelectTab("events")}>{t("workspace.eventList")} <em>{captions.toLocaleString()}</em></button><button class:selected={taskTab === "diagnostics"} onclick={() => onSelectTab("diagnostics")}>{t("workspace.diagnostics")} <em>{diagnosticsCount}</em></button>
    <div class="preview-tools"><button class="icon-button" onclick={() => onSelectTab("events")} aria-label={t("workspace.openEventList")}><FileText size={18} /></button></div>
  </div>
  {#if taskTab === "preview"}
    <div class="player-shell">
      <div class="native-preview" bind:this={nativePreview}><div class="native-notice"><CirclePlay size={30} /><b>{playerRunning ? t("workspace.nativePreviewActive") : t("workspace.nativePreview")}</b><p>{playerRunning ? t("workspace.nativePreviewActiveDescription") : t("workspace.nativePreviewDescription")}</p></div></div>
      <div class="player-controls">
        <div class="player-time"><span>{formatTime(mediaTimeMs ?? 0)}</span><span>/ {durationMs ? formatTime(durationMs) : "--:--"}</span></div>
        <input class="player-scrubber" aria-label={t("preview.seekTimeline")} type="range" min="0" max={Math.max(1, durationMs ?? 1)} value={mediaTimeMs ?? 0} disabled={!playerRunning || !durationMs} onchange={(event) => onSeekAbsolute(Number(event.currentTarget.value))} />
        <div class="player-buttons"><button class="player-button" title={t("preview.command.frame-back")} onclick={() => onPlayerCommand("frame-back")} disabled={!playerRunning}><StepBack size={17} /></button><button class="player-button" title={t("preview.seekBackShort")} onclick={() => onPlayerCommand("seek-back")} disabled={!playerRunning}><SkipBack size={17} /></button><button class="player-button primary" title={t("workspace.pauseResume")} onclick={() => onPlayerCommand("toggle-pause")} disabled={!playerRunning}>{#if playerPaused}<Play size={18} />{:else}<Pause size={18} />{/if}</button><button class="player-button" title={t("preview.seekForwardShort")} onclick={() => onPlayerCommand("seek-forward")} disabled={!playerRunning}><SkipForward size={17} /></button><button class="player-button" title={t("preview.command.frame-forward")} onclick={() => onPlayerCommand("frame-forward")} disabled={!playerRunning}><StepForward size={17} /></button><span class="volume"><Volume2 size={17} /><input aria-label={t("preview.volume")} type="range" min="0" max="100" value="100" disabled={!playerRunning} onchange={(event) => onSetVolume(Number(event.currentTarget.value))} /></span><button class="player-button" title={t("workspace.fitPreview")} onclick={onResizePreview} disabled={!playerRunning}><Maximize2 size={16} /></button><button class="player-button stop" title={playerRunning ? t("common.stopPreview") : t("common.startPreview")} aria-label={playerRunning ? t("common.stopPreview") : t("common.startPreview")} onclick={playerRunning ? onStopPreview : onStartPreview} disabled={!playerRunning && previewAvailable === false}>{#if playerRunning}<Square size={15} />{:else}<RotateCcw size={16} />{/if}</button></div>
      </div>
    </div>
    <div class="preview-status"><span>{t("workspace.scanned").replace("{0}", (bytesRead / 1024 ** 3).toFixed(2))}</span><span>{progress.toFixed(1)}%</span></div>
    <TaskTimeline {archivePath} {desktopRuntime} live={isExporting} editor {trackLabel} currentTimeMs={mediaTimeMs ?? 0} durationMs={durationMs ?? 0} expectedCount={captions} onSeek={onSeekAbsolute} {onError} />
    <details class="playback-mapping"><summary>{t("preview.mappingTitle")}</summary><p>{t("preview.mappingDescription")}</p><label>{t("preview.mappingSegment")}<input bind:value={playbackMapping.segmentId} /></label><label>{t("preview.mappingMediaAnchor")}<input type="number" step="1" bind:value={playbackMapping.mediaAnchorMs} /></label><label>{t("preview.mappingProjectAnchor")}<input type="number" step="1" bind:value={playbackMapping.projectAnchorMs} /></label><label>{t("preview.mappingRateNumerator")}<input type="number" min="1" step="1" bind:value={playbackMapping.rateNumerator} /></label><label>{t("preview.mappingRateDenominator")}<input type="number" min="1" step="1" bind:value={playbackMapping.rateDenominator} /></label><button class="quiet-button" onclick={onSaveMapping} disabled={playbackMappingBusy}>{playbackMappingBusy ? t("preview.mappingApplying") : t("preview.mappingApply")}</button></details>
    {#if previewAvailable === false}<p class="preview-unavailable">{t("preview.runtimeUnavailable")}</p>{/if}
  {:else if taskTab === "events"}
    <TaskTimeline {archivePath} {desktopRuntime} live={isExporting} {trackLabel} expectedCount={captions} onSeek={onSeekAbsolute} {onError} />
  {:else}<TaskDiagnostics jobId={currentJobId} {desktopRuntime} onCountChange={onDiagnosticsCount} {onError} />{/if}
</section>

<style>
  .player-shell { margin-top: 19px; overflow: hidden; border: 1px solid var(--rw-border); border-radius: 7px; background: #05080c; }
  .native-preview { margin: 0; min-height: 304px; border-radius: 0; }
  .player-controls { display: grid; grid-template-columns: auto minmax(120px, 1fr); gap: 10px 14px; align-items: center; padding: 10px 12px; color: #dfe8f3; background: #111923; }
  .player-time { display: flex; gap: 5px; font: 12px "Cascadia Mono", monospace; white-space: nowrap; }
  .player-time span + span { color: #92a3b5; }
  .player-scrubber,.volume input { width: 100%; accent-color: #3d8dff; }
  .player-buttons { grid-column: 1 / -1; display: flex; align-items: center; gap: 6px; }
  .player-button { display:grid; place-items:center; width:34px; height:32px; color:#d9e4f0; border:1px solid #314152; border-radius:5px; background:#182330; }
  .player-button:hover:not(:disabled) { background:#24354a; }.player-button.primary { color:#fff; background:#1766e7; border-color:#3680ec; }.player-button.stop { margin-left:auto; }.volume{display:flex;align-items:center;gap:7px;min-width:130px;margin-left:8px;color:#b6c7d8}.volume input{width:92px}
</style>

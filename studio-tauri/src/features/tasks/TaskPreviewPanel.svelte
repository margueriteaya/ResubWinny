<script lang="ts">
  import { CirclePlay, ListVideo, Maximize2, Pause, Play, Square, SquarePlay, Stethoscope, Volume2 } from "@lucide/svelte";
  import type { PlaybackTimeMapping, PreviewCommand } from "../../backend";
  import { t } from "../../i18n";
  import TaskDiagnostics from "./TaskDiagnostics.svelte";
  import TaskTimeline from "./TaskTimeline.svelte";
  import MacSlider from "../../components/MacSlider.svelte";
  import MacSegmentedControl from "../../components/MacSegmentedControl.svelte";

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
  export let trackName = "";
  export let trackDetail = "";
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
  let playbackMappingDetails: HTMLDetailsElement;

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
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  };
  $: taskTabOptions = [
    { value: "preview", label: t("workspace.captionPreview"), icon: SquarePlay },
    { value: "events", label: `${t("workspace.eventList")} · ${captions.toLocaleString()}`, icon: ListVideo },
    { value: "diagnostics", label: `${t("workspace.diagnostics")} · ${diagnosticsCount}`, icon: Stethoscope },
  ];
</script>

<section class="preview-panel">
  <div class="tabs">
    <MacSegmentedControl size="toolbar" iconOnly ariaLabel={t("app.navigation")} value={taskTab} options={taskTabOptions} onChange={(value) => onSelectTab(value as TaskTab)} />
  </div>
  {#if taskTab === "preview"}
    <div class="player-shell">
      <div class="native-preview" data-liquid-ignore bind:this={nativePreview}><div class="native-notice"><CirclePlay size={30} /><b>{playerRunning ? t("workspace.nativePreviewActive") : t("workspace.nativePreview")}</b><p>{playerRunning ? t("workspace.nativePreviewActiveDescription") : t("workspace.nativePreviewDescription")}</p></div></div>
      <div class="player-controls">
        <div class="player-time"><span>{formatTime(mediaTimeMs ?? 0)}</span><span>/ {durationMs ? formatTime(durationMs) : "--:--"}</span></div>
        <MacSlider className="player-scrubber" ariaLabel={t("preview.seekTimeline")} min={0} max={Math.max(1, durationMs ?? 1)} value={mediaTimeMs ?? 0} disabled={!playerRunning || !durationMs} onChange={onSeekAbsolute} />
        <div class="player-buttons"><button class:play-icon={!playerRunning || playerPaused} class="player-button primary" data-tooltip={playerRunning ? t("workspace.pauseResume") : t("common.startPreview")} aria-label={playerRunning ? t("workspace.pauseResume") : t("common.startPreview")} onclick={playerRunning ? () => onPlayerCommand("toggle-pause") : onStartPreview} disabled={!playerRunning && previewAvailable === false}>{#if playerRunning && !playerPaused}<Pause size={18} />{:else}<Play size={18} />{/if}</button><span class="volume"><Volume2 size={17} /><MacSlider ariaLabel={t("preview.volume")} min={0} max={100} value={100} disabled={!playerRunning} onChange={onSetVolume} /></span><button class="player-button" data-tooltip={t("workspace.fitPreview")} aria-label={t("workspace.fitPreview")} onclick={onResizePreview} disabled={!playerRunning}><Maximize2 size={16} /></button><button class="player-button stop" data-tooltip={t("common.stopPreview")} aria-label={t("common.stopPreview")} onclick={onStopPreview} disabled={!playerRunning}><Square size={15} /></button></div>
      </div>
    </div>
    <div class="preview-status"><span>{t("workspace.scanned").replace("{0}", (bytesRead / 1024 ** 3).toFixed(2))}</span><span>{progress.toFixed(1)}%</span></div>
    <TaskTimeline {archivePath} {desktopRuntime} live={isExporting} editor {trackLabel} {trackName} {trackDetail} currentTimeMs={mediaTimeMs ?? 0} durationMs={durationMs ?? 0} expectedCount={captions} onSeek={onSeekAbsolute} onOpenMapping={openPlaybackMapping} {onError} />
    <details class="playback-mapping" bind:this={playbackMappingDetails}><summary tabindex="-1">{t("preview.mappingTitle")}</summary><p>{t("preview.mappingDescription")}</p><label>{t("preview.mappingSegment")}<input bind:value={playbackMapping.segmentId} /></label><label>{t("preview.mappingMediaAnchor")}<input type="number" step="1" bind:value={playbackMapping.mediaAnchorMs} /></label><label>{t("preview.mappingProjectAnchor")}<input type="number" step="1" bind:value={playbackMapping.projectAnchorMs} /></label><label>{t("preview.mappingRateNumerator")}<input type="number" min="1" step="1" bind:value={playbackMapping.rateNumerator} /></label><label>{t("preview.mappingRateDenominator")}<input type="number" min="1" step="1" bind:value={playbackMapping.rateDenominator} /></label><button class="quiet-button" onclick={onSaveMapping} disabled={playbackMappingBusy}>{playbackMappingBusy ? t("preview.mappingApplying") : t("preview.mappingApply")}</button></details>
    {#if previewAvailable === false}<p class="preview-unavailable">{t("preview.runtimeUnavailable")}</p>{/if}
  {:else if taskTab === "events"}
    <TaskTimeline {archivePath} {desktopRuntime} live={isExporting} {trackLabel} {trackName} {trackDetail} expectedCount={captions} onSeek={onSeekAbsolute} {onError} />
  {:else}<TaskDiagnostics jobId={currentJobId} {desktopRuntime} onCountChange={onDiagnosticsCount} {onError} />{/if}
</section>

<style>
  .player-shell { margin-top: 19px; overflow: hidden; border: 1px solid var(--rw-border); border-radius: 7px; background: #05080c; }
  .native-preview { margin: 0; min-height: 304px; border-radius: 0; }
  .player-controls { display: grid; grid-template-columns: auto minmax(120px, 1fr); gap: 10px 14px; align-items: center; padding: 10px 12px; color: #dfe8f3; background: #111923; }
  .player-time { display: flex; gap: 5px; font: 12px "Cascadia Mono", monospace; white-space: nowrap; }
  .player-time span + span { color: #92a3b5; }
  :global(.player-scrubber){width:100%}
  .player-buttons { grid-column: 1 / -1; display: flex; align-items: center; gap: 6px; }
  .player-button { display:grid; place-items:center; width:34px; height:32px; padding:0; color:#d9e4f0; border:1px solid #314152; border-radius:5px; background:#182330; }.player-button :global(svg){display:block;margin:0}.player-button.play-icon :global(svg){transform:translateX(1px)}
  .player-button:hover:not(:disabled) { background:#24354a; }.player-button.primary { color:#fff; background:#1766e7; border-color:#3680ec; }.player-button.stop { margin-left:auto; }.volume{display:flex;align-items:center;gap:7px;min-width:130px;margin-left:8px;color:#b6c7d8}.volume :global(.mac-slider){width:92px}
</style>

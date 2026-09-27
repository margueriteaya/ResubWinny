<script lang="ts">
  import { onDestroy } from "svelte";
  import { Maximize2, Pause, Play, RotateCcw, RotateCw, Square, Volume2, VolumeX } from "@lucide/svelte";
  import { t } from "../../i18n";
  import MacSlider from "../../components/MacSlider.svelte";
  import { projectTimeMs as asProjectTimeMs, type ProjectTimeMs } from "./time-mapping";

  let {
    running = false,
    paused = true,
    available = null,
    projectTimeMs = 0 as ProjectTimeMs,
    rangeStartMs = 0 as ProjectTimeMs,
    rangeEndMs = 120_000 as ProjectTimeMs,
    durationKnown = false,
    volume = 100,
    onStart = () => {},
    onTogglePause = () => {},
    onStop = () => {},
    onFit = () => {},
    onSkipBack = () => {},
    onSkipForward = () => {},
    onToggleMute = () => {},
    onSetVolume = () => {},
    onSeekProject = () => {},
    onSeekTarget = () => {},
    onError = () => {},
  }: {
    running?: boolean;
    paused?: boolean;
    available?: boolean | null;
    projectTimeMs?: ProjectTimeMs;
    rangeStartMs?: ProjectTimeMs;
    rangeEndMs?: ProjectTimeMs;
    durationKnown?: boolean;
    volume?: number;
    onStart?: () => void;
    onTogglePause?: () => void;
    onStop?: () => void;
    onFit?: () => void;
    onSkipBack?: () => void;
    onSkipForward?: () => void;
    onToggleMute?: () => void;
    onSetVolume?: (volume: number) => void;
    onSeekProject?: (milliseconds: ProjectTimeMs, final?: boolean) => void | Promise<void>;
    onSeekTarget?: (milliseconds: ProjectTimeMs, final?: boolean) => void;
    onError?: (message: string) => void;
  } = $props();

  let scrubberActive = $state(false);
  let scrubberTargetMs = $state(0);
  let scrubberFrame: number | undefined;
  let pendingScrubberTarget: number | null = null;
  const scrubberValueMs = $derived(scrubberActive ? scrubberTargetMs : projectTimeMs);

  function formatTime(milliseconds: number) {
    const seconds = Math.max(0, Math.floor(milliseconds / 1000));
    const hours = Math.floor(seconds / 3_600);
    const minutes = Math.floor(seconds / 60) % 60;
    const body = `${String(minutes).padStart(hours ? 2 : 1, "0")}:${String(seconds % 60).padStart(2, "0")}`;
    return hours ? `${hours}:${body}` : body;
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

  function queueScrubberSeek(timeMs: number, final: boolean) {
    scrubberTargetMs = Math.max(rangeStartMs, Math.min(rangeEndMs, Math.round(timeMs)));
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
    if (scrubberFrame === undefined) scrubberFrame = requestAnimationFrame(flushScrubberSeekFrame);
  }

  function cancelScrubber() {
    if (scrubberActive) queueScrubberSeek(scrubberTargetMs, true);
  }

  onDestroy(() => {
    if (scrubberFrame !== undefined) cancelAnimationFrame(scrubberFrame);
    if (scrubberActive || pendingScrubberTarget !== null) {
      onSeekTarget(asProjectTimeMs(scrubberTargetMs), true);
      dispatchScrubberSeek(scrubberTargetMs, true);
    }
  });
</script>

<div class="player-controls" role="group" aria-label={t("preview.controls")}>
  <div class="player-progress">
    <time>{formatTime(scrubberValueMs)}</time>
    <MacSlider className="player-scrubber" ariaLabel={t("preview.seekTimeline")} ariaValueText={formatTime(scrubberValueMs)} min={rangeStartMs} max={rangeEndMs} value={scrubberValueMs} disabled={!running || !durationKnown} onInput={(value) => queueScrubberSeek(value, false)} onChange={(value) => queueScrubberSeek(value, true)} onCancel={cancelScrubber} />
    <time class="duration">{durationKnown ? formatTime(rangeEndMs) : "--:--"}</time>
  </div>
  <div class="player-transport">
    <div class="transport-main">
      <button class="player-action skip" type="button" data-tooltip={t("preview.skipBack")} aria-label={t("preview.skipBack")} onclick={onSkipBack} disabled={!running}><RotateCcw size={22} /><small aria-hidden="true">5</small></button>
      <button class="player-button" class:play-icon={!running || paused} type="button" data-tooltip={running ? paused ? t("preview.resume") : t("preview.pause") : t("common.startPreview")} aria-label={running ? paused ? t("preview.resume") : t("preview.pause") : t("common.startPreview")} onclick={running ? onTogglePause : onStart} disabled={!running && available === false}>{#if running && !paused}<Pause size={19} />{:else}<Play size={19} />{/if}</button>
      <button class="player-action skip" type="button" data-tooltip={t("preview.skipForward")} aria-label={t("preview.skipForward")} onclick={onSkipForward} disabled={!running}><RotateCw size={22} /><small aria-hidden="true">5</small></button>
    </div>
    <div class="transport-options">
      <div class="volume-control">
        <button class="player-action mute" type="button" data-tooltip={volume === 0 ? t("preview.unmute") : t("preview.mute")} aria-label={volume === 0 ? t("preview.unmute") : t("preview.mute")} aria-pressed={volume === 0} onclick={onToggleMute} disabled={!running}>{#if volume === 0}<VolumeX size={17} />{:else}<Volume2 size={17} />{/if}</button>
        <MacSlider ariaLabel={t("preview.volume")} ariaValueText={`${volume}%`} min={0} max={100} value={volume} disabled={!running} onInput={onSetVolume} onChange={onSetVolume} />
      </div>
      <span class="options-divider" aria-hidden="true"></span>
      <button class="player-action" type="button" data-tooltip={t("workspace.fitPreview")} aria-label={t("workspace.fitPreview")} onclick={onFit} disabled={!running}><Maximize2 size={16} /></button>
      <button class="player-action stop" type="button" data-tooltip={t("common.stopPreview")} aria-label={t("common.stopPreview")} onclick={onStop} disabled={!running}><Square size={15} /></button>
    </div>
  </div>
</div>

<style>
  .player-controls{display:grid;gap:4px;padding:7px 10px 8px;color:var(--rw-text);background:var(--rw-surface-muted);border-top:1px solid var(--rw-border-subtle)}
  .player-progress{display:grid;grid-template-columns:minmax(48px,auto) minmax(0,1fr) minmax(48px,auto);align-items:center;gap:10px;min-height:28px}
  .player-progress time{font:12px/16px var(--rw-font-mono);font-variant-numeric:tabular-nums;white-space:nowrap}.player-progress .duration{text-align:right;color:var(--rw-text-secondary)}
  .player-progress :global(.player-scrubber){width:100%}
  .player-transport,.transport-main,.transport-options,.volume-control{display:flex;align-items:center}.player-transport{justify-content:space-between;gap:8px;min-width:0}.transport-main{gap:2px}.transport-options{gap:2px;min-width:0}.volume-control{gap:3px;margin-right:4px}.volume-control :global(.mac-slider){width:76px}.options-divider{width:1px;height:18px;margin:0 6px;background:var(--rw-border)}
  .player-action,.player-button{display:grid;place-items:center;flex:none;width:36px;height:36px;padding:0;border:0;border-radius:18px;color:var(--rw-text-secondary);background:transparent;cursor:pointer}.player-button{width:40px;height:40px;border:.5px solid var(--rw-glass-border);color:var(--rw-accent-text);background:var(--rw-glass-control);box-shadow:var(--rw-control-shadow)}.player-button.play-icon :global(svg){transform:translateX(1px)}.player-action.stop{color:var(--rw-text-secondary)}
  .player-action.skip{position:relative}.player-action.skip small{position:absolute;top:12px;left:0;width:100%;font-size:11px;line-height:12px;font-weight:700;text-align:center;pointer-events:none}
  .player-action:disabled,.player-button:disabled{opacity:.45;cursor:not-allowed}.player-action:focus-visible,.player-button:focus-visible{outline:2px solid var(--rw-accent);outline-offset:2px}
  @media(hover:hover) and (pointer:fine){.player-action:hover:not(:disabled){color:var(--rw-text);background:color-mix(in srgb,var(--rw-text) 8%,transparent)}.player-button:hover:not(:disabled){background:var(--rw-glass-control-hover)}}
  .player-action:active:not(:disabled){background:color-mix(in srgb,var(--rw-text) 14%,transparent)}
  @container player (max-width:610px){.player-transport{flex-wrap:wrap}.transport-options{margin-left:auto}.volume-control :global(.mac-slider){width:62px}}
  @container player (max-width:480px){.player-progress{gap:6px}.transport-options{width:100%;justify-content:flex-end}.volume-control{margin-right:auto}.volume-control :global(.mac-slider){width:86px}}
  @media(prefers-reduced-transparency:reduce){.player-button{background:var(--rw-surface-raised)}}
  @media(prefers-contrast:more){.player-button{border-color:var(--rw-text)}.player-action:focus-visible,.player-button:focus-visible{outline-width:3px}}
  @media(prefers-reduced-motion:reduce){.player-action,.player-button{transition:none}.player-controls :global(.liquid-control[data-liquid-pressed="true"]){transform:none}}
</style>

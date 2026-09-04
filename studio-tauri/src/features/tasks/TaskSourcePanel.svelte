<script lang="ts">
  import { ChevronRight } from "@lucide/svelte";
  import type { Inspection, Track, UserMode } from "../../backend";
  import { t } from "../../i18n";
  import { trackDisplayDetail, trackDisplayLabel, trackKey } from "../tracks";

  export let inspection: Inspection;
  export let userMode: UserMode = "normie";
  export let routeLabel = "";
  export let selectedTrackKeys: Set<string> = new Set();
  export let selectionDisabled = false;
  export let onSelectTrack: (track: Track) => void = () => {};

  const bytes = (value: number) => value ? `${(value / 1024 ** 3).toFixed(value > 100 * 1024 ** 3 ? 1 : 2)} GB` : "-";
  $: serviceName = inspection.tracks[0]?.serviceName;
  $: networkName = inspection.broadcast.networkName;
  $: programmeName = inspection.broadcast.programmeName;
  $: programmeDescription = inspection.broadcast.programmeDescription;
  $: broadcastTime = inspection.broadcast.broadcastTimeUtc;
  $: hasBroadcastInformation = Boolean(serviceName || networkName || programmeName || programmeDescription || broadcastTime);
</script>

<section class="source-panel">
  <div class="source-file">
    <span class="large-file">{inspection.container === "TLV" ? "TLV" : "TS"}</span>
    <div><b>{inspection.name}</b><small>{bytes(inspection.size)}</small>{#if userMode === "nerd"}<small>{inspection.size.toLocaleString()} bytes</small>{/if}</div>
  </div>
  <p class="mode-section">{t("mode.summary")}</p>
  <div class="source-stats"><span><small>{t("workspace.container")}</small><b>{inspection.container}</b></span>{#if userMode === "nerd"}<span><small>{t("workspace.route")}</small><b>{routeLabel}</b></span>{/if}<span><small>{t("workspace.tracks")}</small><b>{inspection.tracks.length}</b></span></div>
  {#if hasBroadcastInformation}
    <p class="eyebrow">{t("broadcast.information")}</p>
    <dl class="broadcast-info">
      {#if serviceName}<div class:wide={!networkName}><dt>{t("broadcast.service")}</dt><dd>{serviceName}</dd></div>{/if}
      {#if networkName}<div class:wide={!serviceName}><dt>{t("broadcast.network")}</dt><dd>{networkName}</dd></div>{/if}
      {#if programmeName}<div class="wide"><dt>{t("broadcast.programme")}</dt><dd>{programmeName}</dd></div>{/if}
      {#if programmeDescription}<div class="wide"><dt>{t("broadcast.description")}</dt><dd>{programmeDescription}</dd></div>{/if}
      {#if broadcastTime}<div class="wide"><dt>{t("broadcast.time")}</dt><dd>{broadcastTime}</dd></div>{/if}
    </dl>
  {/if}
  <p class="mode-section">{userMode === "nerd" ? t("mode.structure") : t("workspace.captionTracks")}</p>
  {#if inspection.tracks.length}
    <p class="muted">{t("tracks.selectOne")}</p>
    {#each inspection.tracks as track}
      <button class:selected={selectedTrackKeys.has(trackKey(track))} class="track" aria-pressed={selectedTrackKeys.has(trackKey(track))} disabled={selectionDisabled} onclick={() => onSelectTrack(track)}>
        <span class="check">{#if selectedTrackKeys.has(trackKey(track))}<svg class="source-checkmark" viewBox="0 0 10 10" aria-hidden="true"><path d="M1 5.15 3.65 8 9 1.55" /></svg>{/if}</span><span><b>{trackDisplayLabel(track)}</b><small>{trackDisplayDetail(track)}</small>{#if userMode === "nerd"}<small>{track.logicalTrack}</small>{/if}</span><ChevronRight size={19} />
      </button>
    {/each}
  {:else}<p class="muted">{t("tracks.none")}</p>{/if}
  {#if userMode === "nerd"}
    <p class="mode-section">{t("mode.evidence")}</p>
    <dl class="evidence-list">
      <div><dt>{t("workspace.route")}</dt><dd>{routeLabel}</dd></div>
      <div><dt>{t("workspace.container")}</dt><dd>{inspection.container}{inspection.packetSize ? ` · ${inspection.packetSize} B packets` : ""}</dd></div>
      {#each inspection.tracks as track}<div><dt>{trackDisplayLabel(track)}</dt><dd>{[track.pid, track.language, track.serviceId ? `Service ${track.serviceId}` : ""].filter(Boolean).join(" · ")}</dd></div>{/each}
    </dl>
  {/if}
</section>

<style>
  .source-checkmark{display:block;width:10px;height:10px;overflow:visible;filter:none}.source-checkmark path{fill:none;stroke:currentColor;stroke-width:1.9;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke}
  .mode-section{margin:18px 0 8px;color:var(--rw-text);font-size:12px;font-weight:750}.evidence-list{display:grid;gap:7px;margin:0}.evidence-list div{display:grid;gap:2px;padding:7px 8px;border:1px solid var(--rw-border-subtle);border-radius:6px;background:var(--rw-content)}.evidence-list dt{color:var(--rw-muted);font-size:10px}.evidence-list dd{margin:0;overflow-wrap:anywhere;color:var(--rw-text-secondary);font:11px/1.4 "Cascadia Mono",monospace}
</style>

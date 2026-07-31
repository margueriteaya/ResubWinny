<script lang="ts">
  import { ChevronRight } from "@lucide/svelte";
  import type { Inspection, Track } from "../../backend";
  import { t } from "../../i18n";
  import { trackDisplayDetail, trackDisplayLabel, trackKey } from "../tracks";

  export let inspection: Inspection;
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
  <p class="eyebrow">{t("workspace.sourceFile")}</p>
  <div class="source-file">
    <span class="large-file">{inspection.container === "TLV" ? "TLV" : "TS"}</span>
    <div><b>{inspection.name}</b><small>{bytes(inspection.size)} ({inspection.size.toLocaleString()} bytes)</small><small>{inspection.container}{inspection.packetSize ? ` · ${inspection.packetSize} B packets` : ""}</small></div>
  </div>
  <div class="source-stats"><span><small>{t("workspace.container")}</small><b>{inspection.container}</b></span><span><small>{t("workspace.route")}</small><b>{routeLabel}</b></span><span><small>{t("workspace.tracks")}</small><b>{inspection.tracks.length}</b></span></div>
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
  <p class="eyebrow">{t("workspace.captionTracks")}</p>
  {#if inspection.tracks.length}
    <p class="muted">{t("tracks.selectOne")}</p>
    {#each inspection.tracks as track}
      <button class:selected={selectedTrackKeys.has(trackKey(track))} class="track" aria-pressed={selectedTrackKeys.has(trackKey(track))} disabled={selectionDisabled} onclick={() => onSelectTrack(track)}>
        <span class="check">{selectedTrackKeys.has(trackKey(track)) ? "✓" : ""}</span><span><b>{trackDisplayLabel(track)}</b><small>{trackDisplayDetail(track)}</small><small>{track.pid}</small></span><ChevronRight size={19} />
      </button>
    {/each}
  {:else}<p class="muted">{t("tracks.none")}</p>{/if}
</section>

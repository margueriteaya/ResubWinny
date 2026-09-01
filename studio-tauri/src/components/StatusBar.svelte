<script lang="ts">
  import { CirclePause, CirclePlay, HardDrive, X } from "@lucide/svelte";
  import { t } from "../i18n";

  export let sourceSize = 0;
  export let container = "";
  export let trackCount = 0;
  export let warnings = 0;
  export let isExporting = false;
  export let previewIndexing = false;
  export let isPaused = false;
  export let progress = 0;
  export let onPause: () => void = () => {};
  export let onResume: () => void = () => {};
  export let onCancel: () => void = () => {};

  const bytes = (value: number) => value ? `${(value / 1024 ** 3).toFixed(value > 100 * 1024 ** 3 ? 1 : 2)} GB` : "-";
</script>

<footer class="status-bar">
  <span><HardDrive size={17} />{container ? `${bytes(sourceSize)} · ${container}` : t("common.ready")}</span>
  {#if isExporting}
    <div class="task-progress"><span>{isPaused ? t("task.pausedStatus") : t("task.processing")} ({progress.toFixed(1)}%)</span><i><b style={`width:${progress}%`}></b></i></div>
    <span>{progress.toFixed(0)}%</span>
    {#if isPaused}<button onclick={onResume}><CirclePlay size={17} /> {t("task.resume")}</button>{:else}<button onclick={onPause}><CirclePause size={17} /> {t("task.pause")}</button>{/if}
    <button onclick={onCancel}><X size={17} /> {t("task.cancel")}</button>
  {:else if previewIndexing}
    <span class="status-message">{t("task.indexingTimeline")} {progress.toFixed(0)}%</span>
  {:else}
    <span class="status-message">{container ? t("task.trackSummary").replace("{0}", String(trackCount)).replace("{1}", String(warnings)) : t("task.dropStart")}</span>
  {/if}
</footer>

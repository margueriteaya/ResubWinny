<script lang="ts">
  import { CirclePause, CirclePlay, HardDrive, X } from "@lucide/svelte";
  import { t } from "../i18n";

  let {
    sourceSize = 0,
    container = "",
    trackCount = 0,
    warnings = 0,
    isExporting = false,
    previewIndexing = false,
    isPaused = false,
    progress = 0,
    onPause = () => {},
    onResume = () => {},
    onCancel = () => {},
  }: {
    sourceSize?: number;
    container?: string;
    trackCount?: number;
    warnings?: number;
    isExporting?: boolean;
    previewIndexing?: boolean;
    isPaused?: boolean;
    progress?: number;
    onPause?: () => void;
    onResume?: () => void;
    onCancel?: () => void;
  } = $props();

  const bytes = (value: number) => value ? `${(value / 1024 ** 3).toFixed(value > 100 * 1024 ** 3 ? 1 : 2)} GB` : "-";
</script>

<footer class="status-bar">
  <span><HardDrive size={17} />{container ? `${bytes(sourceSize)} · ${container}` : t("common.ready")}</span>
  {#if isExporting}
    <div class="task-progress"><span>{isPaused ? t("task.pausedStatus") : t("task.processing")} ({progress.toFixed(1)}%)</span><i><b style={`transform:scaleX(${Math.max(0, Math.min(100, progress)) / 100})`}></b></i></div>
    <span>{progress.toFixed(0)}%</span>
    {#if isPaused}<button class="task-control" onclick={onResume}><CirclePlay size={17} /> {t("task.resume")}</button>{:else}<button class="task-control" onclick={onPause}><CirclePause size={17} /> {t("task.pause")}</button>{/if}
    <button class="task-control cancel-control" onclick={onCancel}><X size={17} /> {t("task.cancel")}</button>
  {:else if previewIndexing}
    <span class="status-message">{t("task.indexingTimeline")} {progress.toFixed(0)}%</span>
  {:else}
    <span class="status-message">{container ? t("task.trackSummary").replace("{0}", String(trackCount)).replace("{1}", String(warnings)) : t("task.dropStart")}</span>
  {/if}
</footer>

<style>
  .task-control{--rw-liquid-press-scale:.96}
  .cancel-control{color:#a83535}
</style>

<script lang="ts">
  import BatchQueue from "./BatchQueue.svelte";
  import { t } from "../../i18n";
  import type { ExportFormat, ExportPreservation } from "../../backend";
  import type { BatchItem } from "./controller";

  export let items: BatchItem[] = [];
  export let running = false;
  export let paused = false;
  export let onAddFiles: () => void = () => {};
  export let onClearQueue: () => void | Promise<void> = () => {};
  export let onClearCompleted: () => void | Promise<void> = () => {};
  export let onPauseQueue: () => void = () => {};
  export let onStartQueue: () => void = () => {};
  export let onOpenItem: (item: BatchItem) => void = () => {};
  export let outputDirectory = "";
  export let onChooseOutputDirectory: () => void = () => {};
  export let formats: { name: ExportFormat; description: string; icon?: any; color?: string }[] = [];
  export let selectedFormats = new Set<ExportFormat>(["ASS"]);
  export let preservation: ExportPreservation;
  export let onToggleFormat: (format: ExportFormat) => void = () => {};
  export let onTogglePreservation: (feature: keyof ExportPreservation) => void = () => {};
</script>

<header class="workspace-header">
  <div><h1>{t("batch.title")}</h1><p>{t("batch.description")}</p></div>
</header>
<section class="batch-page">
  <BatchQueue
    {items}
    {running}
    {paused}
    addFiles={onAddFiles}
    clearQueue={onClearQueue}
    clearCompleted={onClearCompleted}
    pauseQueue={onPauseQueue}
    startQueue={onStartQueue}
    openItem={onOpenItem}
    {outputDirectory}
    chooseOutputDirectory={onChooseOutputDirectory}
    {formats}
    {selectedFormats}
    {preservation}
    {onToggleFormat}
    {onTogglePreservation}
  />
</section>

<style>
  .batch-page{min-width:0;min-height:0;overflow:hidden}
  :global(main[data-page="batch"] .application){display:grid;grid-template-rows:auto minmax(0,1fr);overflow:hidden}
  @media(max-width:760px){:global(main[data-page="batch"] .application){overflow:auto}.batch-page{overflow:visible}}
</style>

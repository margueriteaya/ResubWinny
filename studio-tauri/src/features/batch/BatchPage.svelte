<script lang="ts">
  import BatchProcessing from "../../BatchProcessing.svelte";
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
  export let formats: { name: ExportFormat; description: string }[] = [];
  export let selectedFormats = new Set<ExportFormat>(["ASS"]);
  export let preservation: ExportPreservation;
  export let onToggleFormat: (format: ExportFormat) => void = () => {};
  export let onTogglePreservation: (feature: keyof ExportPreservation) => void = () => {};
</script>

<header class="workspace-header">
  <div><h1>{t("batch.title")}</h1><p>{t("batch.description")}</p></div>
</header>
<section class="batch-page">
  <BatchProcessing
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

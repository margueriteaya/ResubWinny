<script lang="ts">
  import BatchQueue from "./BatchQueue.svelte";
  import { t } from "../../i18n";
  import type { ExportFormat, ExportPreservation } from "../../backend";
  import type { BatchItem } from "./controller";

  let {
    items = [],
    running = false,
    paused = false,
    onAddFiles = () => {},
    onClearQueue = () => {},
    onClearCompleted = () => {},
    onPauseQueue = () => {},
    onStartQueue = () => {},
    onOpenItem = () => {},
    outputDirectory = "",
    onChooseOutputDirectory = () => {},
    formats = [],
    selectedFormats = new Set<ExportFormat>(["ASS"]),
    preservation,
    onToggleFormat = () => {},
    onTogglePreservation = () => {},
  }: {
    items?: BatchItem[];
    running?: boolean;
    paused?: boolean;
    onAddFiles?: () => void;
    onClearQueue?: () => void | Promise<void>;
    onClearCompleted?: () => void | Promise<void>;
    onPauseQueue?: () => void;
    onStartQueue?: () => void;
    onOpenItem?: (item: BatchItem) => void;
    outputDirectory?: string;
    onChooseOutputDirectory?: () => void;
    formats?: { name: ExportFormat; description: string; icon?: any; color?: string }[];
    selectedFormats?: Set<ExportFormat>;
    preservation: ExportPreservation;
    onToggleFormat?: (format: ExportFormat) => void;
    onTogglePreservation?: (feature: keyof ExportPreservation) => void;
  } = $props();
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

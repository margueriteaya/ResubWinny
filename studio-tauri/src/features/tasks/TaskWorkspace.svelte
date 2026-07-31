<script lang="ts">
  import { FolderOpen, FolderPlus, Upload } from "@lucide/svelte";
  import type { ExportFormat, ExportPreservation, Inspection, PlaybackTimeMapping, PreviewCommand, Track } from "../../backend";
  import { t } from "../../i18n";
  import TaskOutputPanel from "./TaskOutputPanel.svelte";
  import { trackKey } from "../tracks";
  import TaskPreviewPanel from "./TaskPreviewPanel.svelte";
  import TaskSourcePanel from "./TaskSourcePanel.svelte";

  type Format = { name: ExportFormat; description: string };
  type TaskTab = "preview" | "events" | "diagnostics";
  export let inspection: Inspection | null = null;
  export let isInspecting = false;
  export let previewIndexing = false;
  export let routeLabel = "";
  export let selectedTracks = new Set<string>();
  export let taskTab: TaskTab = "preview";
  export let currentJobId = "";
  export let archivePath = "";
  export let desktopRuntime = false;
  export let logs: string[] = [];
  export let captions = 0;
  export let diagnosticsCount = 0;
  export let bytesRead = 0;
  export let progress = 0;
  export let mediaTimeMs: number | null = null;
  export let durationMs: number | null = null;
  export let playerRunning = false;
  export let playerPaused = true;
  export let previewAvailable: boolean | null = null;
  export let nativePreview: HTMLDivElement | null = null;
  export let playbackMapping: PlaybackTimeMapping;
  export let playbackMappingBusy = false;
  export let formats: Format[] = [];
  export let selectedFormats = new Set<ExportFormat>(["ASS"]);
  export let preservation: ExportPreservation;
  export let error = "";
  export let isExporting = false;
  export let subtitle: string = t("task.selectRecording");
  export let onChooseSource: () => void = () => {};
  export let onSelectTrack: (track: Track) => void = () => {};
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
  export let onStartExport: () => void = () => {};
  export let outputDirectory = "";
  export let onChooseOutputDirectory: () => void = () => {};
  export let onToggleFormat: (format: ExportFormat) => void = () => {};
  export let onTogglePreservation: (feature: keyof ExportPreservation) => void = () => {};
  export let canResume = false;
  export let resumeBusy = false;
  export let onResume: () => void = () => {};
  $: selectedTrackLabel = inspection?.tracks.find((track) => selectedTracks.has(trackKey(track)))?.pid ?? "";
</script>

<header class="workspace-header">
  <div><h1>{inspection?.name ?? t("task.new")}</h1><p>{inspection ? subtitle : t("task.selectRecording")}</p></div>
  <div class="header-actions"><button class="outline" onclick={onChooseSource} disabled={isInspecting}><FolderPlus size={18} /> {isInspecting ? t("task.inspecting") : t("common.openFile")}</button></div>
</header>
{#if inspection}
  <div class="task-workspace">
    <TaskSourcePanel {inspection} {routeLabel} selectedTrackKeys={selectedTracks} selectionDisabled={isExporting && !previewIndexing} onSelectTrack={onSelectTrack} />
    <TaskPreviewPanel
      {taskTab} {currentJobId} {archivePath} {desktopRuntime} {captions} {diagnosticsCount} {bytesRead} {progress} {isExporting} {mediaTimeMs} {durationMs} {playerRunning} {playerPaused} {previewAvailable} trackLabel={selectedTrackLabel}
      bind:nativePreview bind:playbackMapping {playbackMappingBusy}
      onSelectTab={onSelectTab} onPlayerCommand={onPlayerCommand} onStartPreview={onStartPreview} onStopPreview={onStopPreview}
      onResizePreview={onResizePreview} {onSeekAbsolute} {onSetVolume} onSaveMapping={onSaveMapping}
      onDiagnosticsCount={onDiagnosticsCount} onError={onError}
    />
    <TaskOutputPanel {inspection} {formats} {selectedFormats} {preservation} {error} {isExporting} {previewIndexing} {logs} {canResume} {resumeBusy} {onToggleFormat} {onTogglePreservation} onStartExport={onStartExport} {onResume} bind:outputDirectory {onChooseOutputDirectory} />
  </div>
{:else}
  <section class="blank-task"><Upload size={42} /><h2>{t("task.chooseRecording")}</h2><p>{t("task.structureFirst")}</p><button class="primary-button plain" onclick={onChooseSource}><FolderOpen size={19} /> {t("task.selectFiles")}</button></section>
{/if}

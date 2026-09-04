<script lang="ts">
  import { FolderOpen, FolderPlus, PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen, Upload } from "@lucide/svelte";
  import type { ExportFormat, ExportPreservation, Inspection, PlaybackTimeMapping, PreviewCommand, Track, WorkspaceLayoutSettings } from "../../backend";
  import { t } from "../../i18n";
  import TaskOutputPanel from "./TaskOutputPanel.svelte";
  import { trackDisplayDetail, trackDisplayLabel, trackKey } from "../tracks";
  import TaskPreviewPanel from "./TaskPreviewPanel.svelte";
  import TaskSourcePanel from "./TaskSourcePanel.svelte";
  import type { MediaTimeMs, ProjectTimeMs } from "./time-mapping";
  import type { FeatureKnowledge } from "./export-assessment";

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
  export let warnings = 0;
  export let diagnosticsCount = 0;
  export let bytesRead = 0;
  export let progress = 0;
  export let projectTimeMs: ProjectTimeMs = 0 as ProjectTimeMs;
  export let durationMs: MediaTimeMs | null = null;
  export let playerRunning = false;
  export let playerPaused = true;
  export let previewAvailable: boolean | null = null;
  export let nativePreview: HTMLDivElement | null = null;
  export let playbackMapping: PlaybackTimeMapping;
  export let appliedPlaybackMapping: PlaybackTimeMapping;
  export let playbackMappingBusy = false;
  export let formats: Format[] = [];
  export let selectedFormats = new Set<ExportFormat>(["ASS"]);
  export let preservation: ExportPreservation;
  export let featureKnowledge: FeatureKnowledge = {};
  export let error = "";
  export let isExporting = false;
  export let exportPending = false;
  export let subtitle: string = t("task.selectRecording");
  export let onChooseSource: () => void = () => {};
  export let onSelectTrack: (track: Track) => void = () => {};
  export let onSelectTab: (tab: TaskTab) => void = () => {};
  export let onPlayerCommand: (command: PreviewCommand) => void = () => {};
  export let onStartPreview: () => void = () => {};
  export let onStopPreview: () => void = () => {};
  export let onResizePreview: () => void = () => {};
  export let onSeekProject: (milliseconds: ProjectTimeMs, final?: boolean) => void | Promise<void> = () => {};
  export let onSeekTarget: (milliseconds: ProjectTimeMs, final?: boolean) => void = () => {};
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
  export let workspaceLayout: WorkspaceLayoutSettings = { sourceWidth: 240, outputWidth: 300, sourceCollapsed: false, outputCollapsed: false };
  export let onWorkspaceLayoutChange: (layout: WorkspaceLayoutSettings) => void = () => {};
  export let compactViewport = false;
  export let compactSourceOpen = false;
  export let compactOutputOpen = false;
  export let onToggleCompactSource: () => void = () => {};
  export let onToggleCompactOutput: () => void = () => {};
  let sourceWidth = workspaceLayout.sourceWidth;
  let outputWidth = workspaceLayout.outputWidth;
  let sourceCollapsed = workspaceLayout.sourceCollapsed;
  let outputCollapsed = workspaceLayout.outputCollapsed;
  let lastWorkspaceLayout = workspaceLayout;
  let dragFrame = 0;
  let pendingWidth = 0;
  $: if (workspaceLayout !== lastWorkspaceLayout) {
    lastWorkspaceLayout = workspaceLayout;
    sourceWidth = workspaceLayout.sourceWidth;
    outputWidth = workspaceLayout.outputWidth;
    sourceCollapsed = workspaceLayout.sourceCollapsed;
    outputCollapsed = workspaceLayout.outputCollapsed;
  }
  $: selectedTrack = inspection?.tracks.find((track) => selectedTracks.has(trackKey(track)));
  $: selectedTrackLabel = selectedTrack?.pid ?? "";
  $: selectedTrackName = selectedTrack ? trackDisplayLabel(selectedTrack) : "";
  $: selectedTrackDetail = selectedTrack ? trackDisplayDetail(selectedTrack) : "";

  const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, Math.round(value)));
  function commitLayout() {
    onWorkspaceLayoutChange({ sourceWidth, outputWidth, sourceCollapsed, outputCollapsed });
  }
  $: sourceIsCollapsed = compactViewport ? !compactSourceOpen : sourceCollapsed;
  $: outputIsCollapsed = compactViewport ? !compactOutputOpen : outputCollapsed;
  function toggleSource() { if (compactViewport) onToggleCompactSource(); else { sourceCollapsed = !sourceCollapsed; commitLayout(); } }
  function toggleOutput() { if (compactViewport) onToggleCompactOutput(); else { outputCollapsed = !outputCollapsed; commitLayout(); } }
  function resizePane(side: "source" | "output", event: PointerEvent) {
    const separator = event.currentTarget as HTMLElement;
    const startX = event.clientX;
    const startWidth = side === "source" ? sourceWidth : outputWidth;
    const applyPendingWidth = () => {
      if (side === "source") sourceWidth = clamp(pendingWidth, 220, 320);
      else outputWidth = clamp(pendingWidth, 280, 380);
    };
    separator.setPointerCapture(event.pointerId);
    const move = (moveEvent: PointerEvent) => {
      pendingWidth = startWidth + (moveEvent.clientX - startX) * (side === "source" ? 1 : -1);
      if (dragFrame) return;
      dragFrame = requestAnimationFrame(() => {
        dragFrame = 0;
        applyPendingWidth();
      });
    };
    const end = (endEvent: PointerEvent) => {
      separator.removeEventListener("pointermove", move);
      separator.removeEventListener("pointerup", end);
      separator.removeEventListener("pointercancel", end);
      if (dragFrame) {
        cancelAnimationFrame(dragFrame);
        dragFrame = 0;
        applyPendingWidth();
      }
      if (separator.hasPointerCapture(endEvent.pointerId)) separator.releasePointerCapture(endEvent.pointerId);
      commitLayout();
    };
    separator.addEventListener("pointermove", move);
    separator.addEventListener("pointerup", end);
    separator.addEventListener("pointercancel", end);
  }
  function resizeFromKeyboard(side: "source" | "output", event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const delta = (event.key === "ArrowRight" ? 8 : -8) * (side === "source" ? 1 : -1);
    if (side === "source") sourceWidth = clamp(sourceWidth + delta, 220, 320);
    else outputWidth = clamp(outputWidth + delta, 280, 380);
    commitLayout();
  }
</script>

<header class="workspace-header">
  <div><h1>{inspection?.name ?? t("task.new")}</h1><p>{inspection ? subtitle : t("task.selectRecording")}</p></div>
  <div class="header-actions"><button class="outline" onclick={onChooseSource} disabled={isInspecting}><FolderPlus size={18} /> {isInspecting ? t("task.inspecting") : t("common.openFile")}</button></div>
</header>
{#if inspection}
  <div class:source-collapsed={sourceIsCollapsed} class:output-collapsed={outputIsCollapsed} class="task-workspace" style={`--source-width:${sourceWidth}px;--output-width:${outputWidth}px`}>
    <div class="workspace-pane source-pane">
      <header class="pane-header"><b>{t("workspace.sourceFile")}</b><button class="pane-toggle liquid-control" onclick={toggleSource} data-tooltip={sourceIsCollapsed ? t("app.showSidebar") : t("app.hideSidebar")} aria-label={sourceIsCollapsed ? t("app.showSidebar") : t("app.hideSidebar")}>{#if sourceIsCollapsed}<PanelLeftOpen size={16} />{:else}<PanelLeftClose size={16} />{/if}</button></header>
      {#if !sourceIsCollapsed}<TaskSourcePanel {inspection} {routeLabel} selectedTrackKeys={selectedTracks} selectionDisabled={isExporting && !previewIndexing} onSelectTrack={onSelectTrack} />{/if}
    </div>
    {#if !sourceIsCollapsed}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
      <div class="pane-separator source-separator" role="separator" aria-orientation="vertical" aria-label={t("workspace.resizeSource")} aria-valuemin="220" aria-valuemax="320" aria-valuenow={sourceWidth} tabindex="0" onpointerdown={(event) => resizePane("source", event)} onkeydown={(event) => resizeFromKeyboard("source", event)}></div>
    {/if}
    <TaskPreviewPanel
      {taskTab} {currentJobId} {archivePath} {desktopRuntime} {logs} {captions} {warnings} selectedTrackCount={selectedTracks.size} {diagnosticsCount} {bytesRead} {progress} {isExporting} {previewIndexing} {projectTimeMs} {durationMs} {playerRunning} {playerPaused} {previewAvailable} {compactViewport} trackLabel={selectedTrackLabel} trackName={selectedTrackName} trackDetail={selectedTrackDetail}
      bind:nativePreview bind:playbackMapping {appliedPlaybackMapping} {playbackMappingBusy}
      onSelectTab={onSelectTab} onPlayerCommand={onPlayerCommand} onStartPreview={onStartPreview} onStopPreview={onStopPreview}
      onResizePreview={onResizePreview} {onSeekProject} {onSeekTarget} {onSetVolume} onSaveMapping={onSaveMapping}
      onDiagnosticsCount={onDiagnosticsCount} onError={onError}
    />
    {#if !outputIsCollapsed}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
      <div class="pane-separator output-separator" role="separator" aria-orientation="vertical" aria-label={t("workspace.resizeOutput")} aria-valuemin="280" aria-valuemax="380" aria-valuenow={outputWidth} tabindex="0" onpointerdown={(event) => resizePane("output", event)} onkeydown={(event) => resizeFromKeyboard("output", event)}></div>
    {/if}
    <div class="workspace-pane output-pane">
      <header class="pane-header"><b>{t("workspace.outputSettings")}</b><button class="pane-toggle liquid-control" onclick={toggleOutput} data-tooltip={outputIsCollapsed ? t("workspace.showOutput") : t("workspace.hideOutput")} aria-label={outputIsCollapsed ? t("workspace.showOutput") : t("workspace.hideOutput")}>{#if outputIsCollapsed}<PanelRightOpen size={16} />{:else}<PanelRightClose size={16} />{/if}</button></header>
      {#if !outputIsCollapsed}<TaskOutputPanel {inspection} {formats} {selectedFormats} {preservation} {featureKnowledge} {error} {isExporting} {exportPending} {canResume} {resumeBusy} {onToggleFormat} {onTogglePreservation} onStartExport={onStartExport} {onResume} bind:outputDirectory {onChooseOutputDirectory} />{/if}
    </div>
  </div>
{:else}
  <section class="blank-task"><Upload size={42} /><h2>{t("task.chooseRecording")}</h2><p>{t("task.structureFirst")}</p><button class="primary-button plain" onclick={onChooseSource}><FolderOpen size={19} /> {t("task.selectFiles")}</button></section>
{/if}

<script lang="ts">
  import { FolderOpen, FolderPlus, PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen, Upload } from "@lucide/svelte";
  import type { ExportFormat, ExportPreservation, Inspection, PlaybackTimeMapping, PreviewCommand, Track, UserMode, WorkspaceLayoutSettings } from "../../backend";
  import { t } from "../../i18n";
  import TaskOutputPanel from "./TaskOutputPanel.svelte";
  import { trackDisplayDetail, trackDisplayLabel, trackKey } from "../tracks";
  import TaskPreviewPanel from "./TaskPreviewPanel.svelte";
  import TaskSourcePanel from "./TaskSourcePanel.svelte";
  import type { MediaTimeMs, ProjectTimeMs } from "./time-mapping";
  import type { FeatureKnowledge, RuntimeExportConflicts } from "./export-assessment";
  import { selectedCaptionTrack } from "./export-eligibility";

  type Format = { name: ExportFormat; description: string };
  type TaskTab = "preview" | "events" | "diagnostics";
  let {
    inspection = null,
    userMode = "normie",
    isInspecting = false,
    previewIndexing = false,
    routeLabel = "",
    selectedTracks = new Set<string>(),
    taskTab = "preview",
    currentJobId = "",
    archivePath = "",
    desktopRuntime = false,
    logs = [],
    captions = 0,
    warnings = 0,
    diagnosticsCount = 0,
    bytesRead = 0,
    progress = 0,
    projectTimeMs = 0 as ProjectTimeMs,
    durationMs = null,
    playerRunning = false,
    playerPaused = true,
    previewAvailable = null,
    nativePreview = null,
    playbackMapping,
    appliedPlaybackMapping,
    playbackMappingBusy = false,
    formats = [],
    selectedFormats = new Set<ExportFormat>(["ASS"]),
    preservation,
    featureKnowledge = {},
    runtimeConflicts = {},
    error = "",
    isExporting = false,
    exportPending = false,
    subtitle = t("task.selectRecording"),
    onChooseSource = () => {},
    onSelectTrack = () => {},
    onSelectTab = () => {},
    onPlayerCommand = () => {},
    onStartPreview = () => {},
    onStopPreview = () => {},
    onResizePreview = () => {},
    onSeekProject = () => {},
    onSeekTarget = () => {},
    onSetVolume = () => {},
    onSaveMapping = () => {},
    onDiagnosticsCount = () => {},
    onError = () => {},
    onStartExport = () => {},
    outputDirectory = "",
    onChooseOutputDirectory = () => {},
    onToggleFormat = () => {},
    onTogglePreservation = () => {},
    onOpenDrcsMapping = () => {},
    canResume = false,
    resumeBusy = false,
    onResume = () => {},
    workspaceLayout = { sourceWidth: 240, outputWidth: 300, sourceCollapsed: false, outputCollapsed: false },
    onWorkspaceLayoutChange = () => {},
    compactViewport = false,
    compactSourceOpen = false,
    compactOutputOpen = false,
    onToggleCompactSource = () => {},
    onToggleCompactOutput = () => {},
  }: {
    inspection?: Inspection | null;
    userMode?: UserMode;
    isInspecting?: boolean;
    previewIndexing?: boolean;
    routeLabel?: string;
    selectedTracks?: Set<string>;
    taskTab?: TaskTab;
    currentJobId?: string;
    archivePath?: string;
    desktopRuntime?: boolean;
    logs?: string[];
    captions?: number;
    warnings?: number;
    diagnosticsCount?: number;
    bytesRead?: number;
    progress?: number;
    projectTimeMs?: ProjectTimeMs;
    durationMs?: MediaTimeMs | null;
    playerRunning?: boolean;
    playerPaused?: boolean;
    previewAvailable?: boolean | null;
    nativePreview?: HTMLDivElement | null;
    playbackMapping: PlaybackTimeMapping;
    appliedPlaybackMapping: PlaybackTimeMapping;
    playbackMappingBusy?: boolean;
    formats?: Format[];
    selectedFormats?: Set<ExportFormat>;
    preservation: ExportPreservation;
    featureKnowledge?: FeatureKnowledge;
    runtimeConflicts?: RuntimeExportConflicts;
    error?: string;
    isExporting?: boolean;
    exportPending?: boolean;
    subtitle?: string;
    onChooseSource?: () => void;
    onSelectTrack?: (track: Track) => void;
    onSelectTab?: (tab: TaskTab) => void;
    onPlayerCommand?: (command: PreviewCommand) => void;
    onStartPreview?: () => void;
    onStopPreview?: () => void;
    onResizePreview?: () => void;
    onSeekProject?: (milliseconds: ProjectTimeMs, final?: boolean) => void | Promise<void>;
    onSeekTarget?: (milliseconds: ProjectTimeMs, final?: boolean) => void;
    onSetVolume?: (volume: number) => void;
    onSaveMapping?: () => void;
    onDiagnosticsCount?: (count: number) => void;
    onError?: (message: string) => void;
    onStartExport?: () => void;
    outputDirectory?: string;
    onChooseOutputDirectory?: () => void;
    onToggleFormat?: (format: ExportFormat) => void;
    onTogglePreservation?: (feature: keyof ExportPreservation) => void;
    onOpenDrcsMapping?: () => void;
    canResume?: boolean;
    resumeBusy?: boolean;
    onResume?: () => void;
    workspaceLayout?: WorkspaceLayoutSettings;
    onWorkspaceLayoutChange?: (layout: WorkspaceLayoutSettings) => void;
    compactViewport?: boolean;
    compactSourceOpen?: boolean;
    compactOutputOpen?: boolean;
    onToggleCompactSource?: () => void;
    onToggleCompactOutput?: () => void;
  } = $props();
  // Seeded from the incoming layout so the first paint is already correct; the
  // effect below adopts every later change. Capturing only the initial value
  // here is the intent, not an oversight.
  /* svelte-ignore state_referenced_locally */
  let sourceWidth = $state(workspaceLayout.sourceWidth);
  /* svelte-ignore state_referenced_locally */
  let outputWidth = $state(workspaceLayout.outputWidth);
  /* svelte-ignore state_referenced_locally */
  let sourceCollapsed = $state(workspaceLayout.sourceCollapsed);
  /* svelte-ignore state_referenced_locally */
  let outputCollapsed = $state(workspaceLayout.outputCollapsed);
  let dragFrame = 0;
  let pendingWidth = 0;
  // Adopt a layout that arrives from settings. This effect depends only on the
  // prop, so unlike the legacy reactive statement it needs no manual guard
  // against unrelated invalidations.
  $effect(() => {
    sourceWidth = workspaceLayout.sourceWidth;
    outputWidth = workspaceLayout.outputWidth;
    sourceCollapsed = workspaceLayout.sourceCollapsed;
    outputCollapsed = workspaceLayout.outputCollapsed;
  });
  const selectedTrack = $derived(selectedCaptionTrack(inspection?.tracks ?? [], selectedTracks));
  const selectedTrackLabel = $derived(selectedTrack?.pid ?? "");
  const selectedTrackName = $derived(selectedTrack ? trackDisplayLabel(selectedTrack) : "");
  const selectedTrackDetail = $derived(selectedTrack ? trackDisplayDetail(selectedTrack) : "");

  const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, Math.round(value)));
  function commitLayout() {
    onWorkspaceLayoutChange({ sourceWidth, outputWidth, sourceCollapsed, outputCollapsed });
  }
  const sourceIsCollapsed = $derived(compactViewport ? !compactSourceOpen : sourceCollapsed);
  const outputIsCollapsed = $derived(compactViewport ? !compactOutputOpen : outputCollapsed);
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
      {#if !sourceIsCollapsed}<TaskSourcePanel {inspection} {routeLabel} {userMode} selectedTrackKeys={selectedTracks} selectionDisabled={isExporting && !previewIndexing} onSelectTrack={onSelectTrack} />{/if}
    </div>
    {#if !sourceIsCollapsed}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="pane-separator source-separator" role="separator" aria-orientation="vertical" aria-label={t("workspace.resizeSource")} aria-valuemin="220" aria-valuemax="320" aria-valuenow={sourceWidth} tabindex="0" onpointerdown={(event) => resizePane("source", event)} onkeydown={(event) => resizeFromKeyboard("source", event)}></div>
    {/if}
    <TaskPreviewPanel
      {taskTab} {currentJobId} {archivePath} {desktopRuntime} {logs} {captions} {warnings} selectedTrackCount={selectedTracks.size} {diagnosticsCount} {bytesRead} {progress} {isExporting} {previewIndexing} {projectTimeMs} {durationMs} {playerRunning} {playerPaused} {previewAvailable} {compactViewport} {userMode} trackLabel={userMode === "nerd" ? selectedTrackLabel : ""} trackName={selectedTrackName} trackDetail={selectedTrackDetail}
      bind:nativePreview bind:playbackMapping {appliedPlaybackMapping} {playbackMappingBusy}
      onSelectTab={onSelectTab} onPlayerCommand={onPlayerCommand} onStartPreview={onStartPreview} onStopPreview={onStopPreview}
      onResizePreview={onResizePreview} {onSeekProject} {onSeekTarget} {onSetVolume} onSaveMapping={onSaveMapping}
      onDiagnosticsCount={onDiagnosticsCount} onError={onError}
    />
    {#if !outputIsCollapsed}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="pane-separator output-separator" role="separator" aria-orientation="vertical" aria-label={t("workspace.resizeOutput")} aria-valuemin="280" aria-valuemax="380" aria-valuenow={outputWidth} tabindex="0" onpointerdown={(event) => resizePane("output", event)} onkeydown={(event) => resizeFromKeyboard("output", event)}></div>
    {/if}
    <div class="workspace-pane output-pane">
      <header class="pane-header"><b>{t("workspace.outputSettings")}</b><button class="pane-toggle liquid-control" onclick={toggleOutput} data-tooltip={outputIsCollapsed ? t("workspace.showOutput") : t("workspace.hideOutput")} aria-label={outputIsCollapsed ? t("workspace.showOutput") : t("workspace.hideOutput")}>{#if outputIsCollapsed}<PanelRightOpen size={16} />{:else}<PanelRightClose size={16} />{/if}</button></header>
      {#if !outputIsCollapsed}<TaskOutputPanel {inspection} {userMode} {formats} {selectedFormats} {preservation} {featureKnowledge} {runtimeConflicts} {error} {isExporting} {exportPending} hasSelectedTrack={Boolean(selectedTrack)} {canResume} {resumeBusy} {onToggleFormat} {onTogglePreservation} {onOpenDrcsMapping} onStartExport={onStartExport} {onResume} bind:outputDirectory {onChooseOutputDirectory} />{/if}
    </div>
  </div>
{:else}
  <section class="blank-task"><Upload size={42} /><h2>{t("task.chooseRecording")}</h2><p>{t("task.structureFirst")}</p><button class="primary-button plain" onclick={onChooseSource}><FolderOpen size={19} /> {t("task.selectFiles")}</button></section>
{/if}

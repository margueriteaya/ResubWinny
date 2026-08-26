<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import HomePage from "./features/home/HomePage.svelte";
  import { NativePreviewController } from "./features/tasks/native-preview-controller";
  import { reduceTaskEvent } from "./features/tasks/event-state";
  import {
    createExportPlan,
    inspectTaskSource,
    startTaskExport,
    taskTrackId,
    taskTrackKey,
    type ExportFormat,
    type ExportPreservation,
  } from "./features/tasks/controller";
  import AppSidebar from "./components/AppSidebar.svelte";
  import type { Page } from "./components/navigation";
  import {
    beginWindowDrag,
    beginWindowResize,
    chooseDirectory,
    chooseRecordingPaths,
    isDesktopRuntime,
    performWindowAction,
    type ResizeDirection,
    type WindowAction,
  } from "./shell/desktop";
  import { installDesktopLifecycle } from "./shell/desktop-lifecycle";
  import StatusBar from "./components/StatusBar.svelte";
  import WindowChrome from "./components/WindowChrome.svelte";
  import {
    BatchQueueController,
    type BatchItem,
  } from "./features/batch/controller";
  import { formatMessage, locale, localeRevision, registerLanguagePacks, setLocale, t } from "./i18n";
  import {
    backend,
    type AppSettings,
    type DrcsGlyph,
    type Inspection,
    type PlaybackTimeMapping,
    type PreviewCommand,
    type TaskHistoryRecord,
    type Track,
  } from "./backend";
  import { formatOptions } from "./features/tasks/formats";
  import {
    basename,
    formatBytes,
    routeLabel,
    upsertHistory,
    type TaskRecord,
  } from "./features/tasks/presentation";
  import {
    DrcsDictionaryController,
    serialiseDrcsMappings,
    type SavedDrcsMapping,
  } from "./features/drcs/controller";
  import {
    applyTheme,
    resolveLocale,
    restoreCachedTheme,
  } from "./features/settings/preferences";


  let page: Page = "home";
  let TaskWorkspaceComponent: any = null;
  let BatchPageComponent: any = null;
  let DrcsPageComponent: any = null;
  let SettingsPageComponent: any = null;
  const sidebarCompactQuery = "(max-width: 1250px)";
  let sidebarCollapsed = typeof window !== "undefined" && window.matchMedia(sidebarCompactQuery).matches;
  let sidebarAutoCollapsed = sidebarCollapsed;
  let compactTaskViewport = false;
  let compactSourceOpen = false;
  let compactOutputOpen = false;
  let inspection: Inspection | null = null;
  let error = "";
  let isInspecting = false;
  let sourceLoadGeneration = 0;
  let isExporting = false;
  let previewIndexing = false;
  let isPaused = false;
  let logs: string[] = [];
  let lastLoggedProgressBucket = -1;
  let progress = 0;
  let bytesRead = 0;
  let warnings = 0;
  let captions = 0;
  let selectedFormats = new Set<ExportFormat>(["ASS"]);
  let preservation: ExportPreservation = {
    position: true,
    color: true,
    ruby: true,
    drcs: true,
    gaiji: true,
    accessibility: true,
  };
  let appSettings: AppSettings = {
    uiFont: "system",
    captionFont: "arib",
    defaultFormat: "ASS",
    defaultTimeline: "Auto (Gap Merge + Overlap Resolve)",
    locale: "system",
    theme: "system",
    workspaceLayout: {
      sourceWidth: 240,
      outputWidth: 300,
      sourceCollapsed: false,
      outputCollapsed: false,
    },
  };
  let settingsPanel: "appearance" | "typography" | "output" | "player" = "typography";
  let outputDirectory = "";
  let taskTab: "preview" | "events" | "diagnostics" = "preview";
  let currentJobId = "";
  let canResumeCurrentJob = false;
  let resumeBusy = false;
  let diagnosticsCount = 0;
  let selectedTracks = new Set<string>();
  let history: TaskRecord[] = [];
  let savedDrcsMappings: Record<string, SavedDrcsMapping> = {};
  // This node exists only while the Tasks preview tab is mounted.  Keeping a
  // nullable reference prevents a destroyed page's host from being reused by
  // a later native-preview start.
  let nativePreview: HTMLDivElement | null = null;
  let playerRunning = false;
  let playerPaused = true;
  let previewAvailable: boolean | null = null;
  const nativePreviewController = new NativePreviewController();
  let renderTimeMs = 0;
  let renderBusy = false;
  let archivePath = "";
  let playbackMapping: PlaybackTimeMapping = {
    segmentId: "recording-origin",
    mediaAnchorMs: 0,
    projectAnchorMs: 0,
    rateNumerator: 1,
    rateDenominator: 1,
  };
  let playbackMappingBusy = false;
  let mediaTimeMs: number | null = null;
  let previewDurationMs: number | null = null;
  let previewResizeFrame = 0;
  let previewResizeInFlight = false;
  let previewResizePending = false;

  $: if (page === "batch" && !BatchPageComponent)
    void import("./features/batch/BatchPage.svelte").then((module) => BatchPageComponent = module.default);
  $: if (page === "tasks" && !TaskWorkspaceComponent)
    void import("./features/tasks/TaskWorkspace.svelte").then((module) => TaskWorkspaceComponent = module.default);
  $: if (page === "drcs" && !DrcsPageComponent)
    void import("./features/drcs/DrcsPage.svelte").then((module) => DrcsPageComponent = module.default);
  $: if (page === "settings" && !SettingsPageComponent)
    void import("./features/settings/SettingsPage.svelte").then((module) => SettingsPageComponent = module.default);
  // mpv owns a native surface. Once it starts, remove the WebView placeholder
  // so the instructional layer cannot be mistaken for video state.
  $: if (nativePreview)
    nativePreview.classList.toggle("native-preview-active", playerRunning);
  // A language switch deliberately remounts the WebView page tree so every
  // legacy translation call refreshes.  Rebind the native child HWND to the
  // replacement placeholder immediately instead of leaving a stale rectangle.
  $: if (playerRunning && nativePreview) void resizePreview();
  let batchInputs: BatchItem[] = [];
  let batchEditingPath: string | null = null;
  let batchRunning = false;
  let multiTaskOutputDirectory = "";
  let drcsGlyphs: DrcsGlyph[] = [];
  let drcsMessage = t("drcs.selectTask");
  // `isTauri()` is the supported runtime probe.  Inspecting private
  // `__TAURI_INTERNALS__.metadata` is not stable across Tauri/WebView2
  // releases and can incorrectly disable every real desktop action.
  const desktopRuntime = isDesktopRuntime();
  restoreCachedTheme();

  async function applyPreferences(settings: AppSettings, refreshLanguagePacks = false) {
    if (desktopRuntime && refreshLanguagePacks)
      registerLanguagePacks(await backend.listLanguagePacks());
    const selected = resolveLocale(settings.locale);
    if (locale() !== selected) setLocale(selected);
    applyTheme(settings.theme);
    appSettings = { ...settings };
  }

  let supportedFormats = formatOptions(t);
  $: {
    $localeRevision;
    supportedFormats = formatOptions(t);
  }

  const bytes = formatBytes;
  $: routeDisplayLabel = routeLabel(inspection?.routeCode, t);
  function savedPreferences(): AppSettings {
    return appSettings;
  }

  function updateWorkspaceLayout(workspaceLayout: AppSettings["workspaceLayout"]) {
    appSettings = { ...appSettings, workspaceLayout };
    if (desktopRuntime) void backend.updateSettings(appSettings).catch(reportBackendFailure);
  }

  $: sourceInspectorCollapsed = compactTaskViewport ? !compactSourceOpen : appSettings.workspaceLayout.sourceCollapsed;
  $: outputInspectorCollapsed = compactTaskViewport ? !compactOutputOpen : appSettings.workspaceLayout.outputCollapsed;

  function toggleSourceInspector() {
    if (compactTaskViewport) {
      const opening = !compactSourceOpen;
      compactSourceOpen = opening;
      if (opening) compactOutputOpen = false;
      void tick().then(resizePreview);
      return;
    }
    updateWorkspaceLayout({ ...appSettings.workspaceLayout, sourceCollapsed: !appSettings.workspaceLayout.sourceCollapsed });
  }

  function toggleOutputInspector() {
    if (compactTaskViewport) {
      const opening = !compactOutputOpen;
      compactOutputOpen = opening;
      if (opening) compactSourceOpen = false;
      void tick().then(resizePreview);
      return;
    }
    updateWorkspaceLayout({ ...appSettings.workspaceLayout, outputCollapsed: !appSettings.workspaceLayout.outputCollapsed });
  }

  function setSidebarCollapsed(collapsed: boolean, automatic = false) {
    if (sidebarCollapsed === collapsed) {
      sidebarAutoCollapsed = automatic && collapsed;
      return;
    }
    sidebarCollapsed = collapsed;
    sidebarAutoCollapsed = automatic && collapsed;
    void tick().then(resizePreview);
  }

  function toggleSidebar() {
    setSidebarCollapsed(!sidebarCollapsed);
  }

  onMount(() => {
    const query = window.matchMedia("(max-width: 980px)");
    const update = () => {
      compactTaskViewport = query.matches;
      compactSourceOpen = false;
      compactOutputOpen = false;
    };
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  });

  onMount(() => {
    const query = window.matchMedia(sidebarCompactQuery);
    const update = () => {
      if (query.matches && !sidebarCollapsed) {
        setSidebarCollapsed(true, true);
      } else if (!query.matches && sidebarAutoCollapsed) {
        setSidebarCollapsed(false);
      }
    };
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  });

  function reportBackendFailure(reason: unknown) {
    const message = reason instanceof Error ? reason.message : String(reason);
    error = formatMessage("error.backend", { message });
  }

  function appendNotice(code: string, parameters: Record<string, unknown> = {}) {
    logs = [...logs, formatMessage(code, parameters)].slice(-1000);
  }

  function persistHistory() {
    const records: TaskHistoryRecord[] = history
      .slice(0, 25)
      .map((item) => ({ ...item, captions: item.captions ?? 0 }));
    if (desktopRuntime) void backend.saveTaskHistory(records);
  }

  async function selectTrack(track: Track) {
    const nextKey = taskTrackKey(track);
    if (selectedTracks.has(nextKey) || (isExporting && !previewIndexing)) return;
    if (previewIndexing) {
      try {
        await backend.cancelExportAndWait();
      } catch (reason) {
        reportBackendFailure(reason);
        return;
      }
      isExporting = false;
      previewIndexing = false;
    }
    selectedTracks = new Set([nextKey]);
    archivePath = "";
    captions = 0;
    bytesRead = 0;
    progress = 0;
    if (batchEditingPath) {
      const selectedTrackKey = taskTrackKey(track);
      batchInputs = batchInputs.map((item) =>
        item.inspection.path === batchEditingPath
          ? { ...item, selectedTrackKey }
          : item,
      );
    }
    if (inspection) await startPreviewIndex(inspection.path);
  }
  function addHistory(status: TaskRecord["status"]) {
    if (!inspection) return;
    const record: TaskRecord = {
      name: inspection.name,
      path: inspection.path,
      size: inspection.size,
      container: inspection.container,
      status,
      time: new Date().toLocaleString(),
      warnings,
      captions,
      jobId: currentJobId || undefined,
    };
    history = upsertHistory(history, record);
    persistHistory();
  }

  async function loadSource(path: string, jobId = "") {
    if (!desktopRuntime) {
      error = t("error.desktopInspect");
      return;
    }
    const generation = ++sourceLoadGeneration;
    isInspecting = true;
    error = "";
    try {
      await stopPreview();
      if (isExporting || previewIndexing) await backend.cancelExportAndWait();
      if (generation !== sourceLoadGeneration) return;
      isExporting = false;
      previewIndexing = false;
      isPaused = false;
      const discovered = await inspectTaskSource(path);
      if (generation !== sourceLoadGeneration) return;
      inspection = discovered;
      outputDirectory = discovered.path.replace(/[\\/][^\\/]+$/, "");
      batchEditingPath = null;
      currentJobId = jobId;
      canResumeCurrentJob = false;
      selectedTracks = inspection.tracks[0]
        ? new Set([taskTrackKey(inspection.tracks[0])])
        : new Set();
      const preferences = savedPreferences();
      if (
        preferences.defaultFormat &&
        ["ASS", "TTML", "JSON", "Raw Data"].includes(preferences.defaultFormat)
      )
        selectedFormats = new Set([preferences.defaultFormat as ExportFormat]);
      page = "tasks";
      taskTab = "preview";
      logs = [
        formatMessage("notice.sourceSelected", { name: inspection.name }),
        formatMessage("notice.container", { container: inspection.container }),
        formatMessage("notice.captionTracks", { count: inspection.tracks.length }),
      ];
      lastLoggedProgressBucket = -1;
      progress = 0;
      warnings = 0;
      captions = 0;
      archivePath = "";
      renderTimeMs = 0;
      mediaTimeMs = null;
      previewDurationMs = null;
      // Start the native player and the bounded caption index independently.
      // The first frame, stream metadata and the initial timeline window can
      // arrive in parallel; neither waits for the other to finish.
      await tick();
      if (generation !== sourceLoadGeneration) return;
      void startPreview();
      void startPreviewIndex(path);
    } catch (reason) {
      if (generation === sourceLoadGeneration) reportBackendFailure(reason);
    } finally {
      if (generation === sourceLoadGeneration) isInspecting = false;
    }
  }

  async function chooseSource() {
    if (!desktopRuntime) {
      error = t("error.desktopSelect");
      return;
    }
    const [selected] = await chooseRecordingPaths(false, t("dialog.broadcastRecordings"));
    if (selected) await loadSource(selected);
  }

  async function openHistory(record: TaskRecord) {
    // A history row is only a saved pointer to a source.  Re-inspect it instead
    // of reconstructing a partial task object, otherwise the task workspace
    // would show an empty, misleading track list.
    await loadSource(record.path, record.jobId ?? "");
    await refreshResumeAvailability();
  }

  async function startExport() {
    if (!desktopRuntime) {
      error = t("error.desktopExport");
      return;
    }
    if (!inspection || isExporting) return;
    error = "";
    isExporting = true;
    previewIndexing = false;
    isPaused = false;
    bytesRead = 0;
    progress = 0;
    if (!outputDirectory.trim()) {
      isExporting = false;
      error = t("workspace.outputDirectoryRequired");
      return;
    }
    const plan = createExportPlan(inspection, selectedFormats, preservation, selectedTracks, outputDirectory);
    logs = [
      ...logs,
      formatMessage("notice.exportStarted", { format: plan?.formats.join(", ") ?? "" }),
      formatMessage("notice.exportOptions"),
    ];
    lastLoggedProgressBucket = -1;
    if (!plan) {
      isExporting = false;
      error = t("tracks.selectionRequired");
      return;
    }
    try {
      currentJobId = await startTaskExport(
        inspection,
        plan,
        exportMappings(),
        (jobId) => (currentJobId = jobId),
      );
      diagnosticsCount = 0;
    } catch (reason) {
      isExporting = false;
      previewIndexing = false;
      reportBackendFailure(reason);
    }
  }

  async function chooseOutputDirectory() {
    if (!desktopRuntime || !inspection) return;
    const selected = await chooseDirectory(t("workspace.chooseOutputDirectory"), outputDirectory);
    if (selected) outputDirectory = selected;
  }

  async function startPreviewIndex(expectedPath = inspection?.path ?? "") {
    if (!desktopRuntime || !inspection || isExporting) return;
    if (!expectedPath || inspection.path !== expectedPath) return;
    const selected = inspection.tracks.find((track) => selectedTracks.has(taskTrackKey(track)));
    isExporting = true;
    previewIndexing = true;
    try {
      const index = await backend.startPreviewIndex(inspection.path, taskTrackId(selected));
      if (inspection?.path !== expectedPath) {
        await backend.cancelExportAndWait().catch(() => {});
        return;
      }
      archivePath = index.archivePath.replace(/\.jsonl$/i, ".jsonl.part");
      appendNotice("notice.previewIndexStarted");
    } catch (reason) {
      isExporting = false;
      reportBackendFailure(reason);
    }
  }

  async function cancelExport() {
    if (desktopRuntime) await backend.cancelExport();
  }
  async function pauseExport() {
    if (desktopRuntime) await backend.pauseExport();
  }
  async function resumeExport() {
    if (desktopRuntime) await backend.resumeExport();
  }
  async function refreshResumeAvailability() {
    if (!desktopRuntime || !currentJobId) {
      canResumeCurrentJob = false;
      return;
    }
    try {
      const [job, checkpoint] = await Promise.all([backend.getJob(currentJobId), backend.getJobCheckpoint(currentJobId)]);
      canResumeCurrentJob = Boolean(checkpoint && job && ["Interrupted", "Failed", "Cancelled"].includes(job.state));
    } catch {
      canResumeCurrentJob = false;
    }
  }
  async function resumeCheckpoint() {
    if (!desktopRuntime || !canResumeCurrentJob || !currentJobId) return;
    resumeBusy = true;
    error = "";
    try {
      await backend.resumeJob(currentJobId);
      isExporting = true;
      isPaused = false;
      canResumeCurrentJob = false;
      appendNotice("notice.checkpointReplayStarted");
    } catch (reason) {
      reportBackendFailure(reason);
      await refreshResumeAvailability();
    } finally {
      resumeBusy = false;
    }
  }

  function previewRect() {
    if (!nativePreview) throw new Error("Native preview host is not mounted.");
    const bounds = nativePreview.getBoundingClientRect();
    const scale = window.devicePixelRatio;
    return {
      x: Math.round(bounds.left * scale),
      y: Math.round(bounds.top * scale),
      width: Math.round(bounds.width * scale),
      height: Math.round(bounds.height * scale),
    };
  }

  async function startPreview() {
    if (!desktopRuntime) {
      error = t("error.desktopPreview");
      return;
    }
    if (!inspection || playerRunning || !nativePreview) return;
    error = "";
    try {
      const started = await nativePreviewController.start(
        inspection.path,
        previewRect(),
        (mapping) => (playbackMapping = mapping),
        {
          archivePath: () => archivePath,
          renderBusy: () => renderBusy,
          setRenderBusy: (value) => (renderBusy = value),
          setProjectTime: (timeMs) => (renderTimeMs = timeMs),
          setMediaTime: (timeMs) => (mediaTimeMs = timeMs),
          setDuration: (timeMs) => (previewDurationMs = timeMs),
          setPaused: (paused) => (playerPaused = paused),
          setBroadcastMetadata: (metadata) => {
            if (!inspection) return;
            inspection = {
              ...inspection,
              broadcast: {
                networkName: metadata.networkName ?? inspection.broadcast.networkName,
                programmeName: metadata.programmeName ?? inspection.broadcast.programmeName,
                programmeDescription:
                  metadata.programmeDescription ?? inspection.broadcast.programmeDescription,
                broadcastTimeUtc:
                  metadata.broadcastTimeUtc ?? inspection.broadcast.broadcastTimeUtc,
              },
            };
          },
          selectedServiceId: () =>
            inspection?.tracks.find((track) =>
              selectedTracks.has(taskTrackKey(track)),
            )?.serviceId,
          onError: reportBackendFailure,
          onNotice: appendNotice,
        },
      );
      if (!started || page !== "tasks" || taskTab !== "preview") return;
      playerRunning = true;
      playerPaused = true;
      renderTimeMs = 0;
    } catch (reason) {
      reportBackendFailure(reason);
    }
  }

  function resizePreview() {
    previewResizePending = true;
    if (previewResizeFrame || previewResizeInFlight) return;
    previewResizeFrame = requestAnimationFrame(() => {
      previewResizeFrame = 0;
      void flushPreviewResize();
    });
  }

  async function flushPreviewResize() {
    if (!previewResizePending || !desktopRuntime || !playerRunning || !nativePreview) return;
    previewResizePending = false;
    previewResizeInFlight = true;
    try {
      await nativePreviewController.resize(previewRect());
    } finally {
      previewResizeInFlight = false;
      if (previewResizePending) resizePreview();
    }
  }

  async function stopPreview() {
    if (!playerRunning && !nativePreviewController.isRunning()) return;
    try {
      if (desktopRuntime) await nativePreviewController.stop({ onNotice: appendNotice });
    } finally {
      // A failed IPC acknowledgement cannot leave the UI believing that a
      // previous page's HWND still owns the current preview surface.
      playerRunning = false;
      playerPaused = true;
    }
  }

  let previewGeneration = 0;
  let previewStopPromise: Promise<void> = Promise.resolve();
  let previewResumeTimeMs: number | null = null;

  function queuePreviewStop() {
    previewStopPromise = previewStopPromise
      .catch(() => {})
      .then(() => stopPreview())
      .catch((reason) => reportBackendFailure(reason));
    return previewStopPromise;
  }

  async function seekRunningPreview(milliseconds: number, waitForReady = false) {
    if (!desktopRuntime || !playerRunning) return;
    if (waitForReady) {
      for (let attempt = 0; attempt < 20; attempt += 1) {
        if (await backend.getPreviewDuration() != null) break;
        await new Promise((resolve) => window.setTimeout(resolve, 100));
      }
    }
    await backend.seekPreviewAbsolute(milliseconds / 1000);
    mediaTimeMs = milliseconds;
  }

  function switchTaskTab(next: typeof taskTab) {
    if (next === taskTab) return;
    const generation = ++previewGeneration;
    taskTab = next;
    if (next !== "preview") {
      previewResumeTimeMs = mediaTimeMs;
      void queuePreviewStop();
      return;
    }
    if (inspection) void activateTabPreview(generation);
  }

  async function activateTabPreview(generation: number) {
    await previewStopPromise;
    await tick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    if (generation !== previewGeneration || taskTab !== "preview" || page !== "tasks") return;
    const resumeAt = previewResumeTimeMs;
    await startPreview();
    if (resumeAt != null && resumeAt > 0 && generation === previewGeneration && playerRunning) {
      await seekRunningPreview(resumeAt, true);
      previewResumeTimeMs = null;
    }
  }

  async function playerCommand(command: PreviewCommand) {
    if (!desktopRuntime || !playerRunning) return;
    try {
      await backend.previewCommand(command);
      const label = t(`preview.command.${command}`);
      appendNotice("notice.playerCommand", { label });
    } catch (reason) {
      reportBackendFailure(reason);
    }
  }
  async function seekPreviewAbsolute(milliseconds: number) {
    if (!desktopRuntime) return;
    let restarted = false;
    if (taskTab !== "preview") {
      const generation = ++previewGeneration;
      taskTab = "preview";
      await tick();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      if (generation !== previewGeneration || page !== "tasks") return;
      await previewStopPromise;
      await startPreview();
      restarted = true;
    }
    if (!playerRunning) return;
    try {
      // `loadfile` is asynchronous in libmpv. A newly recreated preview can
      // accept commands before its demuxer exposes a seekable duration, which
      // returns MPV_ERROR_COMMAND even for a valid absolute seek.
      await seekRunningPreview(milliseconds, restarted);
    } catch (reason) {
      reportBackendFailure(reason);
    }
  }
  async function setPreviewVolume(volume: number) {
    if (!desktopRuntime || !playerRunning) return;
    try {
      await nativePreviewController.setVolume(volume);
    } catch (reason) {
      reportBackendFailure(reason);
    }
  }

  async function savePlaybackMapping() {
    if (!desktopRuntime) return;
    playbackMappingBusy = true;
    try {
      await backend.updatePlaybackTimeMapping({ ...playbackMapping });
      logs = [...logs, t("preview.mappingApplied")];
    } catch (reason) {
      reportBackendFailure(reason);
    } finally {
      playbackMappingBusy = false;
    }
  }

  async function saveCaptionFont(font: string) {
    if (!desktopRuntime) return;
    try {
      await backend.setCaptionFont(font);
    } catch (reason) {
      reportBackendFailure(reason);
    }
  }

  async function windowAction(action: WindowAction) {
    // Window controls must remain usable even when the optional runtime
    // capability probe is unavailable (for example in a bundled WebView).
    // The Tauri API itself is the source of truth; surface failures instead
    // of silently ignoring a click.
    try {
      await performWindowAction(action);
    } catch (reason) {
      error = formatMessage("error.windowAction", { message: String(reason) });
    }
  }

  async function beginDrag() {
    try {
      await beginWindowDrag();
    } catch (reason) {
      error = formatMessage("error.windowDrag", { message: String(reason) });
    }
  }

  async function beginResize(direction: ResizeDirection) {
    try {
      await beginWindowResize(direction);
    } catch (reason) {
      error = formatMessage("error.windowResize", { message: String(reason) });
    }
  }

  const batchController = new BatchQueueController({
    desktopRuntime,
    items: () => batchInputs,
    running: () => batchRunning,
    paused: () => isPaused,
    updateItems: (items) => (batchInputs = items),
    setRunning: (running) => (batchRunning = running),
    setExporting: (exporting) => (isExporting = exporting),
    setPaused: (paused) => (isPaused = paused),
    setActiveTask: (jobId, activeInspection) => {
      if (inspection?.path === activeInspection.path) currentJobId = jobId;
    },
    selectPaths: () => chooseRecordingPaths(true, t("dialog.broadcastRecordings")),
    inspect: inspectTaskSource,
    mappings: () => serialiseDrcsMappings(savedDrcsMappings),
    formats: () => [...selectedFormats],
    preservation: () => preservation,
    outputDirectory: () => multiTaskOutputDirectory,
    notice: appendNotice,
    fail: reportBackendFailure,
  });

  async function addBatchFiles() {
    if (!desktopRuntime) {
      error = t("error.desktopBatchSelect");
      return;
    }
    await batchController.addFiles();
  }

  const refreshBatchJobs = () => batchController.refresh();
  const startBatchQueue = () => batchController.start();
  const pauseBatchQueue = () => batchController.pause();
  const clearBatchQueue = () =>
    batchRunning ? Promise.resolve() : batchController.remove(() => true);
  const clearCompletedBatchJobs = () =>
    batchController.remove((item) => item.status === "Completed");

  async function chooseMultiTaskOutputDirectory() {
    if (!desktopRuntime) return;
    const selected = await chooseDirectory(
      t("batch.chooseOutputDirectory"),
      multiTaskOutputDirectory,
    );
    if (selected) multiTaskOutputDirectory = selected;
  }

  async function openMultiTaskItem(item: BatchItem) {
    const generation = ++sourceLoadGeneration;
    await stopPreview();
    if (generation !== sourceLoadGeneration) return;
    inspection = item.inspection;
    currentJobId = item.jobId ?? "";
    batchEditingPath = item.inspection.path;
    selectedTracks = item.inspection.tracks[0]
      ? new Set([item.selectedTrackKey ?? taskTrackKey(item.inspection.tracks[0])])
      : new Set();
    taskTab = "preview";
    page = "tasks";
    logs = [];
    lastLoggedProgressBucket = -1;
    progress = item.progress;
    bytesRead = Math.round((item.progress / 100) * item.inspection.size);
    warnings = item.warnings;
    captions = 0;
    archivePath = "";
    renderTimeMs = 0;
    mediaTimeMs = null;
    previewDurationMs = null;
    previewIndexing = false;
    isPaused = false;
    isExporting = item.status === "Processing";
    if (item.jobId) {
      try {
        const artifacts = await backend.getJobArtifacts(item.jobId);
        const archive = artifacts.find(
          (artifact) => artifact.kind === "archive" && artifact.status === "completed",
        );
        if (archive) archivePath = archive.path;
      } catch {
        // A queued or running task may not have published an artifact yet.
      }
    }
    await tick();
    if (generation !== sourceLoadGeneration) return;
    void startPreview();
    if (!batchRunning && !archivePath) void startPreviewIndex(item.inspection.path);
  }
  const drcsController = new DrcsDictionaryController({
    desktopRuntime,
    sourcePath: () => inspection?.path,
    mappings: () => savedDrcsMappings,
    updateMappings: (mappings) => (savedDrcsMappings = mappings),
    updateGlyphs: (glyphs) => (drcsGlyphs = glyphs),
    updateMessage: (message) => (drcsMessage = message),
    message: formatMessage,
  });

  const loadDrcs = () => drcsController.load();
  const mappings = () => savedDrcsMappings;
  const exportMappings = () => drcsController.export();
  const saveGlyphMapping = (
    id: string,
    text: string,
    action: SavedDrcsMapping["action"],
  ) => drcsController.save(id, text, action);
  let navigationGeneration = 0;

  function selectView(target: typeof page) {
    if (target === page) return;
    const generation = ++navigationGeneration;
    ++previewGeneration;
    const leavingTasks = page === "tasks";

    // Navigation is intentionally synchronous.  A native player is an
    // optional native surface, so its asynchronous teardown must never decide
    // whether the WebView may change route.
    page = target;

    if (leavingTasks) {
      previewResumeTimeMs = mediaTimeMs;
      void queuePreviewStop();
    }
    if (target !== "tasks") {
      if (target === "drcs") void loadDrcs();
      return;
    }

    if (!inspection) {
      void chooseSource();
      return;
    }
    if (taskTab !== "preview") return;
    // A page change creates a fresh host.  Start only after it has been laid
    // out, and abandon the work if the user has navigated again meanwhile.
    void activateTaskPreview(generation);
  }

  async function activateTaskPreview(generation: number) {
    await previewStopPromise;
    await tick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    if (generation !== navigationGeneration || page !== "tasks" || taskTab !== "preview") return;
    const resumeAt = previewResumeTimeMs;
    await startPreview();
    if (resumeAt != null && resumeAt > 0 && generation === navigationGeneration && playerRunning) {
      await seekRunningPreview(resumeAt, true);
      previewResumeTimeMs = null;
    }
  }

  onMount(() => {
    if (desktopRuntime) {
      void backend
        .getSettings()
        .then(async (settings) => {
          await applyPreferences(settings, true);
          selectedFormats = new Set([settings.defaultFormat as ExportFormat]);
          void saveCaptionFont(settings.captionFont);
        })
        .catch((reason) => {
          reportBackendFailure(reason);
        });
      void backend
        .getPlaybackTimeMapping()
        .then((mapping) => {
          playbackMapping = mapping;
        })
        .catch((reason) => {
          reportBackendFailure(reason);
        });
      void backend
        .getPreviewRuntime()
        .then((runtime) => {
          previewAvailable = runtime.available;
        })
        .catch((reason) => {
          previewAvailable = false;
          reportBackendFailure(reason);
        });
      void backend
        .loadTaskHistory()
        .then((records) => {
          history = records as TaskRecord[];
        })
        .catch((reason) => {
          reportBackendFailure(reason);
        });
      void backend
        .loadDrcsMappings()
        .then((records) => {
          savedDrcsMappings = records.reduce<Record<string, SavedDrcsMapping>>(
            (result, record) => {
              result[record.id] = { text: record.text, action: record.action };
              return result;
            },
            {},
          );
        })
        .catch((reason) => {
          reportBackendFailure(reason);
        });
    }
    if (!desktopRuntime) return;
    let unlisten: (() => void) | undefined;
    backend
      .subscribeTaskEvents((payload) => {
        const belongsToCurrentJob = Boolean(
          payload.jobId && currentJobId && payload.jobId === currentJobId,
        );
        const belongsToPreviewIndex = !payload.jobId && previewIndexing;
        if (!belongsToCurrentJob && !belongsToPreviewIndex) {
          if (batchRunning) void refreshBatchJobs();
          return;
        }
        const message = payload.code
          ? formatMessage(payload.code, payload.parameters, payload.message)
          : payload.message;
        const transition = reduceTaskEvent(
          {
            archivePath,
            bytesRead,
            captions,
            isExporting,
            isPaused,
            lastLoggedProgressBucket,
            logs,
            previewIndexing,
            progress,
            warnings,
          },
          payload,
          inspection?.size ?? 0,
          message,
          batchRunning,
        );
        ({
          archivePath,
          bytesRead,
          captions,
          isExporting,
          isPaused,
          lastLoggedProgressBucket,
          logs,
          previewIndexing,
          progress,
          warnings,
        } = transition.state);
        if (transition.effects.addHistory) addHistory(transition.effects.addHistory);
        if (transition.effects.refreshResume) void refreshResumeAvailability();
        if (transition.effects.refreshBatch) void refreshBatchJobs();
      })
      .then((dispose) => {
        unlisten = dispose;
      });
    const disposeDesktopLifecycle = installDesktopLifecycle({
      playerRunning: () => playerRunning,
      onRecordingDrop: (source) => void loadSource(source),
      onPlayerCommand: (command) => void playerCommand(command),
      onSurfaceChange: () => void resizePreview(),
    });
    return () => {
      unlisten?.();
      disposeDesktopLifecycle();
      void stopPreview();
    };
  });

  onDestroy(() => {
    if (previewResizeFrame) cancelAnimationFrame(previewResizeFrame);
    void nativePreviewController.dispose();
  });
</script>

<svelte:head><meta name="color-scheme" content="light dark" /></svelte:head>

<main class:dark-workspace={page !== "home"} class:sidebar-collapsed={sidebarCollapsed} data-page={page} data-preview-active={playerRunning}>
  <div class="shell-glass" aria-hidden="true"></div>
  {#key `shell:${$localeRevision}`}
    <WindowChrome
      {sidebarCollapsed}
      {page}
      workspaceLayout={appSettings.workspaceLayout}
      {sourceInspectorCollapsed}
      {outputInspectorCollapsed}
      onWindowAction={(action) => void windowAction(action)}
      onBeginDrag={() => void beginDrag()}
      onBeginResize={(direction) => void beginResize(direction as ResizeDirection)}
      onToggleSidebar={toggleSidebar}
      onToggleSourceInspector={toggleSourceInspector}
      onToggleOutputInspector={toggleOutputInspector}
      onChooseSource={() => void chooseSource()}
    />
    <AppSidebar {page} collapsed={sidebarCollapsed} hasTask={Boolean(inspection)} taskName={inspection?.name ?? ""} busy={isInspecting || isExporting || batchRunning} onNavigate={selectView} />
  {/key}

  <section class="application">
    {#key $localeRevision}
    {#if page === "home"}
      <HomePage
        formats={supportedFormats}
        {history}
        {isInspecting}
        onChooseSource={chooseSource}
        onOpenTask={() => selectView("tasks")}
        onOpenHistory={(item) => void openHistory(item)}
        onNavigate={(target) => selectView(target)}
        onChooseFormat={(name) => {
          selectedFormats = new Set([name as ExportFormat]);
          selectView("tasks");
        }}
      />
    {:else if page === "tasks"}
      {#if TaskWorkspaceComponent}
      <svelte:component this={TaskWorkspaceComponent}
        {inspection}
        {isInspecting}
        {previewIndexing}
        routeLabel={routeDisplayLabel}
        selectedTracks={selectedTracks}
        {taskTab}
        {currentJobId}
        {archivePath}
        {desktopRuntime}
        {logs}
        {captions}
        {diagnosticsCount}
        {bytesRead}
        {progress}
        {mediaTimeMs}
        durationMs={previewDurationMs}
        {playerRunning}
        {playerPaused}
        {previewAvailable}
        bind:nativePreview
        bind:playbackMapping
        {playbackMappingBusy}
        formats={supportedFormats}
        {selectedFormats}
        {preservation}
        {error}
        {isExporting}
        bind:outputDirectory
        canResume={canResumeCurrentJob}
        {resumeBusy}
        workspaceLayout={appSettings.workspaceLayout}
        compactViewport={compactTaskViewport}
        {compactSourceOpen}
        {compactOutputOpen}
        onToggleCompactSource={toggleSourceInspector}
        onToggleCompactOutput={toggleOutputInspector}
        onWorkspaceLayoutChange={updateWorkspaceLayout}
        subtitle={inspection ? `${inspection.container} · ${routeDisplayLabel}` : t("task.selectRecording")}
        onChooseSource={chooseSource}
        onChooseOutputDirectory={chooseOutputDirectory}
        onSelectTrack={selectTrack}
        onSelectTab={switchTaskTab}
        onPlayerCommand={playerCommand}
        onStartPreview={startPreview}
        onStopPreview={stopPreview}
        onResizePreview={resizePreview}
        onSeekAbsolute={seekPreviewAbsolute}
        onSetVolume={setPreviewVolume}
        onSaveMapping={savePlaybackMapping}
        onDiagnosticsCount={(count: number) => (diagnosticsCount = count)}
        onError={(message: string) => (error = formatMessage("error.backend", { message }))}
        onStartExport={startExport}
        onToggleFormat={(next: ExportFormat) => {
          const updated = new Set(selectedFormats);
          if (updated.has(next)) updated.delete(next); else updated.add(next);
          selectedFormats = updated;
        }}
        onTogglePreservation={(feature: keyof ExportPreservation) => (preservation = { ...preservation, [feature]: !preservation[feature] })}
        onResume={resumeCheckpoint}
      />
      {:else}<div class="route-loading" role="status" aria-label={t("workspace.loading")}><span></span></div>{/if}
    {:else if page === "batch"}
      {#if BatchPageComponent}
      <svelte:component this={BatchPageComponent}
        items={batchInputs}
        running={batchRunning}
        paused={isPaused}
        onAddFiles={addBatchFiles}
        onClearQueue={clearBatchQueue}
        onClearCompleted={clearCompletedBatchJobs}
        onPauseQueue={pauseBatchQueue}
        onStartQueue={startBatchQueue}
        onOpenItem={(item: BatchItem) => void openMultiTaskItem(item)}
        outputDirectory={multiTaskOutputDirectory}
        onChooseOutputDirectory={chooseMultiTaskOutputDirectory}
        formats={supportedFormats}
        {selectedFormats}
        {preservation}
        onToggleFormat={(next: ExportFormat) => {
          const updated = new Set(selectedFormats);
          if (updated.has(next)) updated.delete(next); else updated.add(next);
          selectedFormats = updated;
        }}
        onTogglePreservation={(feature: keyof ExportPreservation) => (preservation = { ...preservation, [feature]: !preservation[feature] })}
      />
      {:else}<div class="route-loading" role="status" aria-label={t("workspace.loading")}><span></span></div>{/if}
    {:else if page === "drcs"}
      {#if DrcsPageComponent}
      <svelte:component this={DrcsPageComponent}
        glyphs={drcsGlyphs}
        message={drcsMessage}
        canRefresh={Boolean(inspection)}
        onRefresh={loadDrcs}
        getMapping={(id: string) => mappings()[id]}
        onSaveMapping={saveGlyphMapping}
      />
      {:else}<div class="route-loading" role="status" aria-label={t("workspace.loading")}><span></span></div>{/if}
    {:else}
      <header class="workspace-header">
        <div>
          <h1>{t("settings.title")}</h1>
          <p>{t("settings.description")}</p>
        </div>
      </header>
      {#if SettingsPageComponent}
      <svelte:component this={SettingsPageComponent}
        bind:panel={settingsPanel}
        {saveCaptionFont}
        onSettingsSaved={applyPreferences}
        onSettingsPreview={applyPreferences}
      />
      {:else}<div class="route-loading" role="status" aria-label={t("workspace.loading")}><span></span></div>{/if}
    {/if}
    {/key}
  </section>
  {#key `status:${$localeRevision}`}
    <StatusBar sourceSize={inspection?.size ?? 0} container={inspection?.container ?? ""} trackCount={inspection?.tracks.length ?? 0} {warnings} {isExporting} {previewIndexing} {isPaused} {progress} onPause={pauseExport} onResume={resumeExport} onCancel={cancelExport} />
  {/key}
</main>

<style>
  .route-loading{display:grid;place-items:center;min-height:240px}.route-loading span{width:16px;height:16px;border:2px solid color-mix(in srgb,var(--rw-text) 16%,transparent);border-top-color:var(--rw-accent);border-radius:50%;animation:route-spin 700ms linear infinite}@keyframes route-spin{to{transform:rotate(1turn)}}
  @media(prefers-reduced-motion:reduce){.route-loading span{animation:none;border-top-color:inherit}}
</style>

<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { TriangleAlert, X } from "@lucide/svelte";
  import HomePage from "./features/home/HomePage.svelte";
  import { PreviewSession } from "./features/tasks/preview-session";
  import { SourceSession } from "./features/tasks/source-session";
  import { ExportSession } from "./features/tasks/export-session";
  import { TaskEventSession } from "./features/tasks/event-session";
  import {
    mediaTimeMs as asMediaTimeMs,
    mediaToProjectTime,
    projectTimeMs as asProjectTimeMs,
    type MediaTimeMs,
    type ProjectTimeMs,
  } from "./features/tasks/time-mapping";
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
    chooseDirectory,
    chooseRecordingPaths,
    isDesktopRuntime,
    type ResizeDirection,
  } from "./shell/desktop";
  import { WindowSession } from "./shell/window-session";
  import { ApplicationLifecycleSession } from "./shell/application-lifecycle-session";
  import { LayoutSession } from "./shell/layout-session";
  import { NavigationSession } from "./shell/navigation-session";
  import { FeedbackSession } from "./shell/feedback-session";
  import { BootstrapSession } from "./shell/bootstrap-session";
  import { TaskSelectionSession } from "./features/tasks/selection-session";
  import { RecoverySession } from "./features/tasks/recovery-session";
  import { TaskRuntimeSession, resetTaskRuntime, type RuntimeReset } from "./features/tasks/runtime-session";
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
    type Track,
  } from "./backend";
  import { formatOptions } from "./features/tasks/formats";
  import {
    basename,
    formatBytes,
    routeLabel,
    type TaskRecord,
  } from "./features/tasks/presentation";
  import { HistorySession } from "./features/tasks/history-session";
  import {
    DrcsDictionaryController,
    serialiseDrcsMappings,
    type SavedDrcsMapping,
  } from "./features/drcs/controller";
  import { restoreCachedTheme } from "./features/settings/preferences";
  import { PreferencesSession } from "./features/settings/session";

  let page: Page = "home";
  const navigationSession = new NavigationSession(page);
  let TaskWorkspaceComponent: any = null;
  let BatchPageComponent: any = null;
  let DrcsPageComponent: any = null;
  let SettingsPageComponent: any = null;
  const sidebarCompactQuery = "(max-width: 1250px)";
  const layoutSession = new LayoutSession(typeof window !== "undefined" && window.matchMedia(sidebarCompactQuery).matches);
  let { sidebarCollapsed, sidebarAutoCollapsed, compactTaskViewport, compactSourceOpen, compactOutputOpen } = layoutSession.state;
  function syncLayout() {
    ({ sidebarCollapsed, sidebarAutoCollapsed, compactTaskViewport, compactSourceOpen, compactOutputOpen } = layoutSession.state);
  }
  let inspection: Inspection | null = null;
  let error = "";
  let isInspecting = false;
  const exportSession = new ExportSession({
    beginExport: () => {
      isExporting = true;
      previewIndexing = false;
      isPaused = false;
      bytesRead = 0;
      progress = 0;
    },
    setJob: (jobId) => (currentJobId = jobId),
    completeExportStart: (jobId) => {
      currentJobId = jobId;
      diagnosticsCount = 0;
    },
    failExport: (reason) => {
      isExporting = false;
      exportPending = false;
      reportBackendFailure(reason);
    },
    beginIndex: () => {
      previewIndexing = true;
    },
    completeIndex: (path) => {
      archivePath = path.replace(/\.jsonl$/i, ".jsonl.part");
      appendNotice("notice.previewIndexStarted");
    },
    failIndex: (reason) => {
      previewIndexing = false;
      reportBackendFailure(reason);
    },
  });
  const sourceSession = new SourceSession({
    prepare: async () => {
      exportSession.invalidate();
      await stopPreview();
      if (isExporting || previewIndexing) await backend.cancelExportAndWait();
      applyRuntimeReset(resetTaskRuntime());
    },
    inspect: inspectTaskSource,
    defaultFormat: () => savedPreferences().defaultFormat,
    message: formatMessage,
    apply: (discovered, setup, jobId) => {
      inspection = discovered;
      outputDirectory = setup.outputDirectory;
      batchController.endEditing();
      currentJobId = jobId;
      canResumeCurrentJob = false;
      selectedTracks = setup.selectedTrackKeys;
      if (setup.selectedFormats) selectedFormats = setup.selectedFormats;
      page = "tasks";
      taskTab = "preview";
      logs = setup.logs;
      applyRuntimeReset(resetTaskRuntime({ logs: setup.logs }));
    },
    afterApply: tick,
    activate: (path) => {
      void startPreview();
      void startPreviewIndex(path);
    },
    setBusy: (busy) => (isInspecting = busy),
    fail: reportBackendFailure,
  });
  const taskEventSession = new TaskEventSession({
    currentJobId: () => currentJobId,
    previewIndexing: () => previewIndexing,
    batchRunning: () => batchRunning,
    sourceSize: () => inspection?.size ?? 0,
    state: () => ({ archivePath, bytesRead, captions, isExporting, isPaused, lastLoggedProgressBucket, logs, previewIndexing, progress, warnings }),
    setState: (state) => ({ archivePath, bytesRead, captions, isExporting, isPaused, lastLoggedProgressBucket, logs, previewIndexing, progress, warnings } = state),
    onEffects: (effects) => {
      if (effects.addHistory) addHistory(effects.addHistory);
      if (effects.refreshResume) void refreshResumeAvailability();
      if (effects.refreshBatch) void refreshBatchJobs();
    },
    refreshBatch: () => void refreshBatchJobs(),
  });
  let isExporting = false;
  let previewIndexing = false;
  let exportPending = false;
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
  let projectCursorMs: ProjectTimeMs = asProjectTimeMs(0);
  let renderBusy = false;
  let archivePath = "";
  let playbackMapping: PlaybackTimeMapping = {
    segmentId: "recording-origin",
    mediaAnchorMs: 0,
    projectAnchorMs: 0,
    rateNumerator: 1,
    rateDenominator: 1,
  };
  let appliedPlaybackMapping: PlaybackTimeMapping = { ...playbackMapping };
  let playbackMappingBusy = false;
  let mediaTimeMs: MediaTimeMs | null = null;
  let previewDurationMs: MediaTimeMs | null = null;
  const runtimeSession = new TaskRuntimeSession({
    setEventState: (state) => ({ archivePath, bytesRead, captions, isExporting, isPaused, lastLoggedProgressBucket, logs, previewIndexing, progress, warnings } = state),
    setMediaTime: (value) => (mediaTimeMs = value),
    setProjectTime: (value) => (projectCursorMs = value),
    setDuration: (value) => (previewDurationMs = value),
  });
  const previewSession = new PreviewSession(
    () => nativePreview,
    {
      desktopRuntime: () => desktopRuntime,
      running: () => playerRunning,
      paused: () => playerPaused,
      mediaTimeMs: () => mediaTimeMs,
      mapping: () => appliedPlaybackMapping,
      setMappings: (draft, applied) => {
        playbackMapping = draft;
        appliedPlaybackMapping = applied;
      },
      setMappingBusy: (busy) => (playbackMappingBusy = busy),
      setTimes: (media, project) => {
        mediaTimeMs = media;
        projectCursorMs = project;
      },
      setProjectTime: (project) => (projectCursorMs = project),
      setRunning: (running) => (playerRunning = running),
      setPaused: (paused) => (playerPaused = paused),
      onNotice: (code, parameters) => appendNotice(code, parameters),
      onError: reportBackendFailure,
    },
  );
  function setAppliedPlaybackMapping(mapping: PlaybackTimeMapping) {
    previewSession.applyMapping(mapping);
  }

  function applyRuntimeReset(next: RuntimeReset) {
    runtimeSession.reset({
      ...next,
      projectTimeMs: asProjectTimeMs(next.projectTimeMs),
      mediaTimeMs: next.mediaTimeMs == null ? null : asMediaTimeMs(next.mediaTimeMs),
      previewDurationMs: next.previewDurationMs == null ? null : asMediaTimeMs(next.previewDurationMs),
    });
  }

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
  let batchRunning = false;
  let multiTaskOutputDirectory = "";
  let drcsGlyphs: DrcsGlyph[] = [];
  let drcsMessage = t("drcs.selectTask");
  // `isTauri()` is the supported runtime probe.  Inspecting private
  // `__TAURI_INTERNALS__.metadata` is not stable across Tauri/WebView2
  // releases and can incorrectly disable every real desktop action.
  const desktopRuntime = isDesktopRuntime();
  restoreCachedTheme();
  const feedbackSession = new FeedbackSession();
  const selectionSession = new TaskSelectionSession();
  const historySession = new HistorySession({
    desktopRuntime,
    load: () => backend.loadTaskHistory(),
    save: (records) => backend.saveTaskHistory(records),
    onError: reportBackendFailure,
  });
  const preferencesSession = new PreferencesSession({
    desktopRuntime,
    getSettings: () => backend.getSettings(),
    updateSettings: (settings) => backend.updateSettings(settings),
    setCaptionFont: (font) => backend.setCaptionFont(font),
    listLanguagePacks: () => backend.listLanguagePacks(),
    registerLanguagePacks,
    locale,
    setLocale,
    onError: reportBackendFailure,
  });
  const bootstrapSession = new BootstrapSession({
    desktopRuntime,
    getPlaybackTimeMapping: () => backend.getPlaybackTimeMapping(),
    getPreviewRuntime: () => backend.getPreviewRuntime(),
    loadTaskHistory: () => backend.loadTaskHistory(),
    loadDrcsMappings: () => backend.loadDrcsMappings(),
    onError: reportBackendFailure,
  });
  const recoverySession = new RecoverySession({
    desktopRuntime,
    getJob: (jobId) => backend.getJob(jobId),
    getCheckpoint: (jobId) => backend.getJobCheckpoint(jobId),
    resumeJob: (jobId) => backend.resumeJob(jobId),
    setAvailable: (available) => (canResumeCurrentJob = available),
    setBusy: (busy) => (resumeBusy = busy),
    setExporting: (exporting) => (isExporting = exporting),
    setPaused: (paused) => (isPaused = paused),
    onError: reportBackendFailure,
    notice: appendNotice,
  });
  const lifecycleSession = new ApplicationLifecycleSession({
    desktopRuntime,
    subscribeTaskEvents: (handler) => backend.subscribeTaskEvents(handler),
    onTaskEvent: (payload) => taskEventSession.handle(payload),
    playerRunning: () => playerRunning,
    onRecordingDrop: (source) => void loadSource(source),
    onPlayerCommand: (command) => void playerCommand(command),
    onSurfaceChange: () => void resizePreview(),
  });

  async function applyPreferences(settings: AppSettings, refreshLanguagePacks = false) {
    await preferencesSession.apply(settings, refreshLanguagePacks);
    appSettings = { ...settings };
  }

  const saveCaptionFont = (font: string) => preferencesSession.saveCaptionFont(font);

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
    preferencesSession.persist(appSettings);
  }

  $: sourceInspectorCollapsed = compactTaskViewport ? !compactSourceOpen : appSettings.workspaceLayout.sourceCollapsed;
  $: outputInspectorCollapsed = compactTaskViewport ? !compactOutputOpen : appSettings.workspaceLayout.outputCollapsed;

  function toggleSourceInspector() {
    if (compactTaskViewport) {
      layoutSession.toggleInspector("source");
      syncLayout();
      void tick().then(resizePreview);
      return;
    }
    updateWorkspaceLayout({ ...appSettings.workspaceLayout, sourceCollapsed: !appSettings.workspaceLayout.sourceCollapsed });
  }

  function toggleOutputInspector() {
    if (compactTaskViewport) {
      layoutSession.toggleInspector("output");
      syncLayout();
      void tick().then(resizePreview);
      return;
    }
    updateWorkspaceLayout({ ...appSettings.workspaceLayout, outputCollapsed: !appSettings.workspaceLayout.outputCollapsed });
  }

  function setSidebarCollapsed(collapsed: boolean, automatic = false) {
    if (sidebarCollapsed === collapsed && sidebarAutoCollapsed === automatic && !automatic) return;
    layoutSession.setSidebarCollapsed(collapsed, automatic);
    syncLayout();
    void tick().then(resizePreview);
  }

  function toggleSidebar() {
    layoutSession.toggleSidebar();
    syncLayout();
  }

  onMount(() => {
    const query = window.matchMedia("(max-width: 980px)");
    const update = () => {
      layoutSession.setCompactViewport(query.matches);
      syncLayout();
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
    error = feedbackSession.error(reason);
  }

  function appendNotice(code: string, parameters: Record<string, unknown> = {}) {
    logs = feedbackSession.append(logs, code, parameters);
  }

  async function selectTrack(track: Track) {
    const nextKey = taskTrackKey(track);
    if (selectedTracks.has(nextKey) || (isExporting && !previewIndexing)) return;
    if (previewIndexing) {
      try {
        await exportSession.cancel(() => backend.cancelExportAndWait());
      } catch (reason) {
        reportBackendFailure(reason);
        return;
      }
      isExporting = false;
      previewIndexing = false;
    }
    selectedTracks = selectionSession.singleTrack(nextKey);
    archivePath = "";
    captions = 0;
    bytesRead = 0;
    progress = 0;
    batchController.selectEditingTrack(taskTrackKey(track));
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
    history = historySession.add(history, record);
  }

  async function loadSource(path: string, jobId = "") {
    if (!desktopRuntime) {
      error = t("error.desktopInspect");
      return;
    }
    error = "";
    await sourceSession.load(path, jobId);
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
    if (!inspection || isExporting || exportPending) return;
    const activeInspection = inspection;
    error = "";
    if (!outputDirectory.trim()) {
      error = t("workspace.outputDirectoryRequired");
      return;
    }
    const plan = createExportPlan(activeInspection, selectedFormats, preservation, selectedTracks, outputDirectory);
    logs = [
      ...logs,
      formatMessage("notice.exportStarted", { format: plan?.formats.join(", ") ?? "" }),
      formatMessage("notice.exportOptions"),
    ];
    lastLoggedProgressBucket = -1;
    if (!plan) {
      error = t("tracks.selectionRequired");
      return;
    }
    if (previewIndexing) {
      exportPending = true;
      try {
        await exportSession.cancel(() => backend.cancelExportAndWait());
        previewIndexing = false;
      } catch (reason) {
        exportPending = false;
        reportBackendFailure(reason);
        return;
      }
    }
    exportPending = false;
    await exportSession.runExport(
      (onCreated) => startTaskExport(
          activeInspection,
          plan,
          exportMappings(),
          onCreated,
      ),
    );
  }

  async function chooseOutputDirectory() {
    if (!desktopRuntime || !inspection) return;
    const selected = await chooseDirectory(t("workspace.chooseOutputDirectory"), outputDirectory);
    if (selected) outputDirectory = selected;
  }

  async function startPreviewIndex(expectedPath = inspection?.path ?? "") {
    if (!desktopRuntime || !inspection || isExporting || exportPending || previewIndexing) return;
    if (!expectedPath || inspection.path !== expectedPath) return;
    const sourcePath = inspection.path;
    const selected = inspection.tracks.find((track) => selectedTracks.has(taskTrackKey(track)));
    await exportSession.runPreviewIndex(
        () => backend.startPreviewIndex(sourcePath, taskTrackId(selected)),
        () => inspection?.path === sourcePath,
        () => backend.cancelExportAndWait(),
    );
  }

  async function cancelExport() {
    if (!desktopRuntime) return;
    await exportSession.cancel(() => backend.cancelExport());
  }
  async function pauseExport() {
    if (desktopRuntime) await backend.pauseExport();
  }
  async function resumeExport() {
    if (desktopRuntime) await backend.resumeExport();
  }
  async function refreshResumeAvailability() {
    await recoverySession.refresh(currentJobId);
  }
  async function resumeCheckpoint() {
    error = "";
    await recoverySession.resume(currentJobId, canResumeCurrentJob);
  }

  async function startPreview() {
    if (!desktopRuntime) {
      error = t("error.desktopPreview");
      return;
    }
    if (!inspection) return;
    error = "";
    const started = await previewSession.startManaged(
      inspection.path,
      setAppliedPlaybackMapping,
      {
          archivePath: () => archivePath,
          renderBusy: () => renderBusy,
          setRenderBusy: (value) => (renderBusy = value),
          setMediaTime: (timeMs) => {
            mediaTimeMs = timeMs == null ? null : asMediaTimeMs(timeMs);
            if (timeMs != null)
              projectCursorMs = mediaToProjectTime(asMediaTimeMs(timeMs), appliedPlaybackMapping);
          },
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
  }

  function resizePreview() {
    previewSession.queueResize();
  }

  async function stopPreview() {
    await previewSession.stopManaged({ onNotice: appendNotice });
  }

  function queuePreviewStop() {
    return previewSession.queueStop(stopPreview);
  }

  async function seekRunningPreview(
    milliseconds: MediaTimeMs,
    waitForReady = false,
    isCurrent: () => boolean = () => true,
  ) {
    await previewSession.seekMedia(milliseconds, waitForReady, isCurrent);
  }

  async function seekRunningPreviewProject(
    milliseconds: ProjectTimeMs,
    waitForReady = false,
    final = true,
    intent = previewSession.currentIntent(),
  ) {
    await previewSession.seekProject(milliseconds, waitForReady, final, intent);
  }

  function switchTaskTab(next: typeof taskTab) {
    if (next === taskTab) return;
    const generation = previewSession.beginPageTransition(next !== "preview");
    taskTab = next;
    if (next !== "preview") {
      void queuePreviewStop();
      return;
    }
    if (inspection) void activateTabPreview(generation);
  }

  async function activateTabPreview(generation: number) {
    await previewSession.whenStopped();
    await tick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    if (!previewSession.isCurrentPageTransition(generation) || taskTab !== "preview" || page !== "tasks") return;
    const resumeAt = previewSession.resumeTime();
    await startPreview();
    if (resumeAt != null && resumeAt > 0 && previewSession.isCurrentPageTransition(generation) && playerRunning) {
      await seekRunningPreview(
        resumeAt,
        true,
        () => previewSession.isCurrentPageTransition(generation) && taskTab === "preview" && page === "tasks",
      );
      previewSession.clearResumeTime();
    }
  }

  async function playerCommand(command: PreviewCommand) {
    await previewSession.command(command, t(`preview.command.${command}`));
  }
  // Seek requests are latest-wins. Pointer scrubbing can produce targets
  // faster than mpv/IPC can acknowledge them; serializing every intermediate
  // request creates a long queue and makes the playhead visibly lag behind
  // the pointer. Keep only the newest pending target and resolve superseded
  // callers immediately.
  function seekPreviewProject(milliseconds: ProjectTimeMs, final = true) {
    return previewSession.enqueueProjectSeek(milliseconds, final, performSeekPreviewProject);
  }

  function setPreviewSeekTarget(milliseconds: ProjectTimeMs, final = false) {
    // Publish the target synchronously, before the native IPC round trip. The
    // same range value is then visible in both controls immediately, while
    // the controller revision prevents an old playback/overlay sample from
    // snapping the playhead back during the gesture.
    // Invalidate an in-flight native result now, rather than waiting for the
    // next animation-frame broker dispatch. This prevents a one-frame snap
    // back between two fast pointer samples.
    previewSession.publishProjectTarget(milliseconds, final);
  }

  async function performSeekPreviewProject(milliseconds: ProjectTimeMs, final = true, intent = previewSession.currentIntent()) {
    if (!desktopRuntime) return;
    let restarted = false;
    if (taskTab !== "preview") {
      const generation = previewSession.beginPageTransition(false);
      taskTab = "preview";
      await tick();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      if (!previewSession.isCurrentPageTransition(generation) || page !== "tasks" || !previewSession.isCurrentIntent(intent)) return;
      await previewSession.whenStopped();
      await startPreview();
      restarted = true;
    }
    if (!playerRunning || !previewSession.isCurrentIntent(intent)) return;
    try {
      await seekRunningPreviewProject(milliseconds, restarted, final, intent);
    } catch (reason) {
      reportBackendFailure(reason);
    }
  }
  async function setPreviewVolume(volume: number) {
    await previewSession.setVolume(volume);
  }

  async function savePlaybackMapping() {
    await previewSession.saveMapping(playbackMapping);
  }

  const windowSession = new WindowSession({
    onError: (message) => (error = message),
    formatError: (kind, message) => formatMessage(`error.window${kind[0].toUpperCase()}${kind.slice(1)}`, { message }),
  });

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
  const clearBatchQueue = () => batchController.clearAll();
  const clearCompletedBatchJobs = () => batchController.clearCompleted();

  async function chooseMultiTaskOutputDirectory() {
    if (!desktopRuntime) return;
    const selected = await chooseDirectory(
      t("batch.chooseOutputDirectory"),
      multiTaskOutputDirectory,
    );
    if (selected) multiTaskOutputDirectory = selected;
  }

  async function openMultiTaskItem(item: BatchItem) {
    const generation = sourceSession.begin();
    await stopPreview();
    if (!sourceSession.isCurrent(generation)) return;
    inspection = item.inspection;
    currentJobId = item.jobId ?? "";
    batchController.beginEditing(item);
    selectedTracks = item.inspection.tracks[0]
      ? selectionSession.singleTrack(item.selectedTrackKey ?? taskTrackKey(item.inspection.tracks[0]))
      : new Set();
    taskTab = "preview";
    page = "tasks";
    applyRuntimeReset(resetTaskRuntime({
      progress: item.progress,
      bytesRead: Math.round((item.progress / 100) * item.inspection.size),
      warnings: item.warnings,
      isExporting: item.status === "Processing",
    }));
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
    if (!sourceSession.isCurrent(generation)) return;
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
  function selectView(target: typeof page) {
    const generation = navigationSession.navigate(target);
    if (generation == null) return;
    const leavingTasks = page === "tasks";
    previewSession.beginPageTransition(leavingTasks);

    // Navigation is intentionally synchronous.  A native player is an
    // optional native surface, so its asynchronous teardown must never decide
    // whether the WebView may change route.
    page = target;

    if (leavingTasks) {
      void queuePreviewStop();
    }
    if (target !== "tasks") {
      if (target === "drcs") void loadDrcs();
      return;
    }

    if (!inspection) return;
    if (taskTab !== "preview") return;
    // A page change creates a fresh host.  Start only after it has been laid
    // out, and abandon the work if the user has navigated again meanwhile.
    void activateTaskPreview(generation);
  }

  async function activateTaskPreview(generation: number) {
    await previewSession.whenStopped();
    await tick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    if (!navigationSession.isCurrent(generation, "tasks") || page !== "tasks" || taskTab !== "preview") return;
    const resumeAt = previewSession.resumeTime();
    await startPreview();
    if (resumeAt != null && resumeAt > 0 && navigationSession.isCurrent(generation, "tasks") && playerRunning) {
      await seekRunningPreview(
        resumeAt,
        true,
        () => navigationSession.isCurrent(generation, "tasks") && page === "tasks" && taskTab === "preview",
      );
      previewSession.clearResumeTime();
    }
  }

  onMount(() => {
    let disposeLifecycle = () => {};
    void lifecycleSession.mount().then((dispose) => {
      disposeLifecycle = dispose;
    });
    if (desktopRuntime) {
      void preferencesSession.load(true).then((settings) => {
        if (settings) {
          selectedFormats = new Set([settings.defaultFormat as ExportFormat]);
          void preferencesSession.saveCaptionFont(settings.captionFont);
        }
      });
      void bootstrapSession.load().then((state) => {
        if (state.playbackMapping) setAppliedPlaybackMapping(state.playbackMapping);
        previewAvailable = state.previewAvailable ?? false;
        history = state.history as TaskRecord[];
        savedDrcsMappings = state.drcsMappings.reduce<Record<string, SavedDrcsMapping>>(
          (result, record) => ({ ...result, [record.id]: { text: record.text, action: record.action } }),
          {},
        );
      });
    }
    if (!desktopRuntime) return;
    return () => {
      disposeLifecycle();
      void stopPreview();
    };
  });

  onDestroy(() => {
    void previewSession.dispose();
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
      onWindowAction={(action) => void windowSession.action(action)}
      onBeginDrag={() => void windowSession.beginDrag()}
      onBeginResize={(direction) => void windowSession.beginResize(direction as ResizeDirection)}
      onToggleSidebar={toggleSidebar}
      onToggleSourceInspector={toggleSourceInspector}
      onToggleOutputInspector={toggleOutputInspector}
      onChooseSource={() => void chooseSource()}
    />
    <AppSidebar {page} collapsed={sidebarCollapsed} hasTask={Boolean(inspection)} taskName={inspection?.name ?? ""} busy={isInspecting || isExporting || batchRunning} onNavigate={selectView} />
  {/key}

  {#if error}
    <div class="global-error" role="alert">
      <TriangleAlert class="global-error-icon" size={17} aria-hidden="true" />
      <span>{error}</span>
      <button type="button" aria-label={t("common.dismiss")} onclick={() => error = ""}><X size={16} /></button>
    </div>
  {/if}

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
        projectTimeMs={projectCursorMs}
        durationMs={previewDurationMs}
        {playerRunning}
        {playerPaused}
        {previewAvailable}
        bind:nativePreview
        bind:playbackMapping
        {appliedPlaybackMapping}
        {playbackMappingBusy}
        formats={supportedFormats}
        {selectedFormats}
        {preservation}
        {error}
        {isExporting}
        {exportPending}
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
        onSeekProject={seekPreviewProject}
        onSeekTarget={setPreviewSeekTarget}
        onSetVolume={setPreviewVolume}
        onSaveMapping={savePlaybackMapping}
        onDiagnosticsCount={(count: number) => (diagnosticsCount = count)}
        onError={(message: string) => (error = formatMessage("error.backend", { message }))}
        onStartExport={startExport}
        onToggleFormat={(next: ExportFormat) => {
          selectedFormats = selectionSession.toggleFormat(selectedFormats, next);
        }}
        onTogglePreservation={(feature: keyof ExportPreservation) => (preservation = selectionSession.togglePreservation(preservation, feature))}
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
          selectedFormats = selectionSession.toggleFormat(selectedFormats, next);
        }}
        onTogglePreservation={(feature: keyof ExportPreservation) => (preservation = selectionSession.togglePreservation(preservation, feature))}
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
  .global-error{position:fixed;z-index:50;right:18px;bottom:42px;left:calc(var(--rw-sidebar-width, 220px) + 18px);display:grid;grid-template-columns:auto minmax(0,1fr) auto;align-items:center;gap:9px;min-height:42px;padding:8px 10px;border:1px solid color-mix(in srgb,#bb3d3d 65%,var(--rw-border));border-radius:8px;color:var(--rw-text);background:color-mix(in srgb,#bb3d3d 10%,var(--rw-surface-raised));box-shadow:0 8px 28px rgba(0,0,0,.2);backdrop-filter:blur(18px)}:global(.global-error-icon){color:#bb3d3d}.global-error span{font-size:12px;line-height:1.4}.global-error button{display:grid;place-items:center;width:28px;height:28px;padding:0;border:0;border-radius:50%;color:var(--rw-text-secondary);background:transparent}.global-error button:hover{background:color-mix(in srgb,var(--rw-text) 8%,transparent)}.sidebar-collapsed .global-error{left:76px}@media(max-width:700px){.global-error{right:10px;bottom:38px;left:10px}.sidebar-collapsed .global-error{left:10px}}
  .route-loading{display:grid;place-items:center;min-height:240px}.route-loading span{width:16px;height:16px;border:2px solid color-mix(in srgb,var(--rw-text) 16%,transparent);border-top-color:var(--rw-accent);border-radius:50%;animation:route-spin 700ms linear infinite}@keyframes route-spin{to{transform:rotate(1turn)}}
  @media(prefers-reduced-motion:reduce){.route-loading span{animation:none;border-top-color:inherit}}
</style>

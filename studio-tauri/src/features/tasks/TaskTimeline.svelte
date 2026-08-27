<script lang="ts">
  import { ChevronLeft, ChevronRight, Clock3, FileText, GitCompareArrows, LocateFixed, RefreshCw, ScanLine } from "@lucide/svelte";
  import { onDestroy } from "svelte";
  import type { TimelineEvent, TimelineFeature } from "../../backend";
  import { t } from "../../i18n";
  import accessibilityIcon from "../../assets/arib/accessibility.svg";
  import colorIcon from "../../assets/arib/color.svg";
  import drcsIcon from "../../assets/arib/drcs.svg";
  import gaijiIcon from "../../assets/arib/gaiji.svg";
  import rubyIcon from "../../assets/arib/ruby.svg";
  import MacCheckbox from "../../components/MacCheckbox.svelte";
  import MacSlider from "../../components/MacSlider.svelte";
  import { getFilteredTimelineWindow, getRecentTimelineWindow, getTimelineTimeWindow } from "./timeline-controller";

  export let archivePath = "";
  export let desktopRuntime = false;
  export let live = false;
  export let editor = false;
  export let currentTimeMs = 0;
  export let rangeStartMs = 0;
  export let rangeEndMs = 120_000;
  export let playing = false;
  export let trackLabel = "";
  export let trackName = "";
  export let trackDetail = "";
  export let expectedCount = 0;
  export let onSeek: (milliseconds: number, final: boolean) => void | Promise<void> = () => {};
  export let onSeekTarget: (milliseconds: number, final?: boolean) => void = () => {};
  export let onOpenMapping: () => void = () => {};
  export let onError: (message: string) => void = () => {};

  const pageSize = 100;
  const featureOptions: TimelineFeature[] = ["color", "ruby", "drcs", "gaiji", "accessibility"];
  let records: TimelineEvent[] = [];
  let filters = new Set<TimelineFeature>();
  let loadedArchive = "";
  let loading = false;
  let exhausted = false;
  let refreshTimer: ReturnType<typeof setInterval> | undefined;
  let loadedEditorStartMs = Number.NaN;
  let loadedEditorEndMs = Number.NaN;
  let loadedEditorSpanMs = 0;
  let loadedEditorHasMore = false;
  let editorRequestKey = "";
  let loadedFilterKey = "";
  let loadedLive = false;
  // Event rows are re-rendered whenever the playhead moves. Keep the rich
  // text segmentation out of that hot path: highlights are immutable for an
  // indexed archive event, so recomputing them on every 500 ms playback tick
  // only burns CPU (especially for Japanese captions with many characters).
  const textSegmentCache = new Map<number, {
    signature: string;
    segments: { text: string; features: TimelineFeature[] }[];
  }>();
  // A 30 second default window keeps typical ARIB caption events readable;
  // users can still zoom out to the full programme or into frame-scale work.
  let zoom = 4;
  let scrubbing = false;
  let dragStartX = 0;
  let dragStartTimeMs = 0;
  let dragWindowStartMs = 0;
  let dragWindowSpanMs = 0;
  let dragWidth = 1;
  let dragMoved = false;
  let scrubStartedOnEvent = false;
  let dragTargetTimeMs = 0;
  // Pointer capture allows a drag that starts on an event bar to continue
  // across the whole track. Suppress the synthetic click emitted after such
  // a drag so it cannot jump back to the event's begin time.
  let suppressEventClickUntil = 0;
  let seekFrame: number | undefined;
  let pendingSeekTarget: number | null = null;
  let resumeFollowAfterScrub = false;
  let scrollStartMs = 0;
  let followPlayhead = true;
  let scrollbar: HTMLDivElement;
  let scrollbarThumb: HTMLSpanElement;
  let scrollbarDragging = false;
  let scrollbarPointerId = -1;
  let scrollbarGrabOffset = 0;

  const featureMeta: Record<Exclude<TimelineFeature, "position">, { color: string; icon: string }> = {
    color: { color: "#d47b00", icon: colorIcon },
    ruby: { color: "#008b95", icon: rubyIcon },
    drcs: { color: "#7b4db5", icon: drcsIcon },
    gaiji: { color: "#316fa5", icon: gaijiIcon },
    accessibility: { color: "#168247", icon: accessibilityIcon },
  };

  const timestamp = (timeMs: number) => {
    const whole = Math.max(0, Math.floor(timeMs));
    const seconds = Math.floor(whole / 1000);
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const remainder = seconds % 60;
    return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}.${String(whole % 1000).padStart(3, "0")}`;
  };
  const rulerTime = (timeMs: number, precision: number) => {
    const unitMs = 10 ** (3 - precision);
    const value = timestamp(Math.round(timeMs / unitMs) * unitMs);
    if (precision === 0) return value.slice(0, -4);
    if (precision === 3) return value;
    return value.slice(0, -(3 - precision));
  };
  const eventEndTimestamp = (timeMs: number) => timeMs >= Number.MAX_SAFE_INTEGER ? "—" : timestamp(timeMs);
  const kind = (value: TimelineEvent["kind"]) => t(`timeline.kind.${value}`, value);
  // The parent supplies this exact mapped range to both the player slider and
  // the timeline. Do not let whichever events arrived first expand the ruler:
  // that makes the same timestamp occupy different percentages in each
  // control and is the source of the apparent ruler/player drift.
  $: timelineTimeMs = scrubbing ? dragTargetTimeMs : currentTimeMs;
  $: timelineStartMs = Math.max(0, Math.min(rangeStartMs, rangeEndMs));
  $: timelineEndMs = Math.max(timelineStartMs + 5_000, rangeEndMs);
  $: timelineSpanMs = timelineEndMs - timelineStartMs;
  $: minimumZoom = Math.min(.25, 120_000 / timelineSpanMs);
  $: viewSpanMs = Math.min(timelineSpanMs, Math.max(5_000, 120_000 / zoom));
  $: scrollMaximumMs = Math.max(0, timelineSpanMs - viewSpanMs);
  // Follow at the edges instead of recentering on every playback sample.
  // Re-centering every 100 ms invalidates every event bar's percentage and
  // forces a dense archive to repaint continuously. Keeping a safety band
  // makes the playhead feel anchored while still bringing it back into view.
  $: if (followPlayhead) {
    const maximumStart = timelineStartMs + scrollMaximumMs;
    const boundedStart = Math.max(timelineStartMs, Math.min(maximumStart, scrollStartMs));
    if (Math.abs(boundedStart - scrollStartMs) > 1) scrollStartMs = boundedStart;
    const lowerEdge = boundedStart + viewSpanMs * .2;
    const upperEdge = boundedStart + viewSpanMs * .8;
    if (timelineTimeMs < lowerEdge || timelineTimeMs > upperEdge) {
      const nextStart = Math.max(timelineStartMs, Math.min(maximumStart, timelineTimeMs - viewSpanMs / 2));
      if (Math.abs(nextStart - scrollStartMs) > 1) scrollStartMs = nextStart;
    }
  }
  $: viewStartMs = Math.max(timelineStartMs, Math.min(timelineStartMs + scrollMaximumMs, scrollStartMs));
  $: visibleEndMs = viewStartMs + viewSpanMs;
  $: editorWindowNeedsLoad = loadedArchive !== archivePath
    || !Number.isFinite(loadedEditorStartMs)
    || viewStartMs < loadedEditorStartMs
    || visibleEndMs > loadedEditorEndMs
    || (loadedEditorHasMore && loadedEditorSpanMs > 0 && viewSpanMs < loadedEditorSpanMs / 3);
  $: visibleRecords = records.filter((item) => item.endMs > viewStartMs && item.beginMs < viewStartMs + viewSpanMs && (!item.trackId || !trackLabel || item.trackId === trackLabel));
  $: ticks = Array.from({ length: 5 }, (_, index) => ({ percent: index * 25, time: viewStartMs + viewSpanMs * index / 4 }));
  $: tickStepMs = viewSpanMs / 4;
  $: tickPrecision = tickStepMs % 1_000 === 0 ? 0 : tickStepMs % 100 === 0 ? 1 : tickStepMs % 10 === 0 ? 2 : 3;
  $: zoomPercent = Math.max(1, Math.round(zoom * 50));
  $: scrollbarThumbPercent = Math.max(7, Math.min(100, viewSpanMs / timelineSpanMs * 100));
  $: scrollbarThumbLeft = scrollMaximumMs <= 0 ? 0 : (viewStartMs - timelineStartMs) / scrollMaximumMs * (100 - scrollbarThumbPercent);
  $: cursorPositionPercent = Math.max(0, Math.min(100, ((timelineTimeMs - viewStartMs) / viewSpanMs) * 100));
  $: playheadVisible = timelineTimeMs >= viewStartMs && timelineTimeMs <= visibleEndMs;
  $: activeRecordIndex = buildActiveRecordIndex(visibleRecords);
  $: activeRecord = findActiveRecord(activeRecordIndex, timelineTimeMs);
  const eventNodes = new Map<number, HTMLButtonElement>();
  let highlightedEventIndex: number | undefined;
  function registerEventNode(node: HTMLButtonElement, index: number) {
    eventNodes.set(index, node);
    if (index === highlightedEventIndex) node.classList.add("current");
    return {
      destroy() {
        if (eventNodes.get(index) === node) eventNodes.delete(index);
      },
    };
  }
  function highlightActiveEvent(index: number | undefined) {
    if (index === highlightedEventIndex) return;
    if (highlightedEventIndex !== undefined)
      eventNodes.get(highlightedEventIndex)?.classList.remove("current");
    highlightedEventIndex = index;
    if (index !== undefined) eventNodes.get(index)?.classList.add("current");
  }
  $: highlightActiveEvent(activeRecord?.index);
  const visibleFeatures = (item: TimelineEvent) => item.features.filter((feature): feature is Exclude<TimelineFeature, "position"> => feature !== "position");
  const eventColor = (item: TimelineEvent) => featureMeta[visibleFeatures(item)[0]]?.color ?? "var(--rw-accent)";
  const sameTimelineRecords = (left: TimelineEvent[], right: TimelineEvent[]) =>
    left.length === right.length
    && left.every((item, index) => {
      const other = right[index];
      return other !== undefined
        && item.index === other.index
        && item.kind === other.kind
        && item.beginMs === other.beginMs
        && item.endMs === other.endMs;
    });
  function buildActiveRecordIndex(items: TimelineEvent[]) {
    let maximumEnd = Number.NEGATIVE_INFINITY;
    return {
      items,
      maximumEnds: items.map((item) => {
        maximumEnd = Math.max(maximumEnd, item.endMs);
        return maximumEnd;
      }),
    };
  }
  function findActiveRecord(index: ReturnType<typeof buildActiveRecordIndex>, timeMs: number) {
    const { items, maximumEnds } = index;
    let low = 0;
    let high = items.length;
    while (low < high) {
      const middle = low + Math.floor((high - low) / 2);
      if (items[middle].beginMs <= timeMs) low = middle + 1;
      else high = middle;
    }
    // The archive is ordered by begin time, but overlapping regions are
    // valid. A short later interval may already have ended while an earlier
    // long interval is still active, so `item.end <= time` is not a valid
    // stopping condition. The prefix maximum lets us stop safely without
    // degrading the 100 ms playback hot path into a full scan.
    let fallback: TimelineEvent | null = null;
    for (let position = low - 1; position >= 0; position -= 1) {
      if (maximumEnds[position] <= timeMs) break;
      const item = items[position];
      if (item.endMs <= timeMs) continue;
      if (!fallback) fallback = item;
      if (item.kind === "caption") return item;
    }
    return fallback;
  }
  const barStyle = (item: TimelineEvent, windowStartMs: number, windowSpanMs: number) => {
    const left = Math.max(0, Math.min(100, ((item.beginMs - windowStartMs) / windowSpanMs) * 100));
    const right = Math.max(left + 0.2, Math.min(100, ((item.endMs - windowStartMs) / windowSpanMs) * 100));
    return `left:${left}%;width:${right - left}%;`;
  };
  // Position, feature badges and colours are functions of the visible window,
  // not of playback time. Keep their strings stable between samples so a
  // 100 Hz playhead update does not force every event button through another
  // style calculation.
  $: positionedRecords = visibleRecords.map((item) => ({
    item,
    features: visibleFeatures(item),
    style: `${barStyle(item, viewStartMs, viewSpanMs)}--event-color:${eventColor(item)};`,
  }));
  // The event-list view is also mounted while playback is active. Precompute
  // rich-text segments only when records change so a playhead update does not
  // repeatedly walk every caption's Unicode/highlight ranges.
  $: eventListItems = records.map((item) => ({ item, segments: textSegments(item) }));

  async function loadPage(reset: boolean) {
    if (!desktopRuntime || !archivePath || loading || (!reset && exhausted && !live)) return;
    const requestedArchive = archivePath;
    loading = true;
    if (reset) {
      loadedArchive = archivePath;
      records = [];
      exhausted = false;
      textSegmentCache.clear();
    }
    try {
      const next = await getFilteredTimelineWindow(archivePath, records.length, pageSize, [...filters]);
      const known = new Set(records.map((item) => item.index));
      records = [...records, ...next.items.filter((item) => !known.has(item.index))];
      exhausted = !next.hasMore;
    } catch (reason) {
      if (!live) onError(String(reason));
    } finally {
      loading = false;
      if (archivePath && archivePath !== requestedArchive)
        queueMicrotask(() => void loadPage(true));
    }
  }

  function editorWindowCovered() {
    return !editorWindowNeedsLoad;
  }

  async function loadEditorWindow(force = false) {
    if (!desktopRuntime || !archivePath || loading || (!force && editorWindowCovered())) return;
    const requestedArchive = archivePath;
    const bufferMs = Math.max(5_000, Math.round(viewSpanMs * .75));
    const requestStartMs = Math.max(timelineStartMs, Math.floor(viewStartMs - bufferMs));
    const requestEndMs = Math.min(timelineEndMs, Math.ceil(viewStartMs + viewSpanMs + bufferMs));
    const requestKey = `${requestedArchive}:${requestStartMs}:${requestEndMs}`;
    if (!force && requestKey === editorRequestKey) return;
    editorRequestKey = requestKey;
    loading = true;
    try {
      const next = await getTimelineTimeWindow(
        requestedArchive,
        requestStartMs,
        requestEndMs,
        500,
      );
      if (archivePath === requestedArchive) {
        if (!sameTimelineRecords(records, next.items)) records = next.items;
        loadedArchive = requestedArchive;
        loadedEditorStartMs = requestStartMs;
        loadedEditorEndMs = requestEndMs;
        loadedEditorSpanMs = requestEndMs - requestStartMs;
        loadedEditorHasMore = next.hasMore;
        exhausted = !next.hasMore;
      }
    } catch (reason) {
      if (!live) onError(String(reason));
    } finally {
      loading = false;
      editorRequestKey = "";
      if (archivePath && !editorWindowCovered())
        queueMicrotask(() => void loadEditorWindow());
    }
  }

  async function loadRecentPage() {
    if (!desktopRuntime || !archivePath || loading) return;
    const requestedArchive = archivePath;
    loading = true;
    try {
      const next = await getRecentTimelineWindow(archivePath, pageSize, [...filters]);
      if (!sameTimelineRecords(records, next.items)) records = next.items;
      loadedArchive = archivePath;
      exhausted = !next.hasMore;
    } catch (reason) {
      if (!live) onError(String(reason));
    } finally {
      loading = false;
      if (archivePath && archivePath !== requestedArchive)
        queueMicrotask(() => void loadRecentPage());
    }
  }

  function toggleFilter(feature: TimelineFeature) {
    const next = new Set(filters);
    if (next.has(feature)) next.delete(feature); else next.add(feature);
    filters = next;
  }

  function pointerTime(event: PointerEvent, windowStart = viewStartMs, windowSpan = viewSpanMs, source?: HTMLElement) {
    const element = source ?? event.currentTarget as HTMLElement;
    const bounds = element.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (event.clientX - bounds.left) / Math.max(1, bounds.width)));
    return Math.max(timelineStartMs, Math.min(timelineEndMs, Math.round(windowStart + ratio * windowSpan)));
  }

  function beginScrub(event: PointerEvent, startedOnEvent = false) {
    const source = (event.currentTarget as HTMLElement).closest<HTMLElement>(".timeline-track")
      ?? event.currentTarget as HTMLElement;
    resumeFollowAfterScrub = followPlayhead;
    followPlayhead = false;
    scrubbing = true;
    dragStartX = event.clientX;
    dragStartTimeMs = pointerTime(event, viewStartMs, viewSpanMs, source);
    dragWindowStartMs = viewStartMs;
    dragWindowSpanMs = viewSpanMs;
    dragWidth = Math.max(1, source.getBoundingClientRect().width);
    dragMoved = false;
    scrubStartedOnEvent = startedOnEvent;
    dragTargetTimeMs = dragStartTimeMs;
    source.setPointerCapture(event.pointerId);
  }

  // Event bars live inside the track and otherwise bubble pointerdown to the
  // track, starting the scrub gesture twice (with two competing captures).
  // Stop propagation while retaining the same coordinate space used by the
  // track so a drag that starts on a caption remains continuous.
  function beginEventScrub(event: PointerEvent) {
    event.stopPropagation();
    beginScrub(event, true);
  }

  function dispatchSeek(timeMs: number, final: boolean) {
    try {
      const operation = onSeek(timeMs, final);
      if (operation && typeof (operation as Promise<void>).catch === "function")
        void Promise.resolve(operation).catch((reason) => onError(String(reason)));
    } catch (reason) {
      onError(String(reason));
    }
  }

  function flushSeekFrame() {
    seekFrame = undefined;
    if (pendingSeekTarget === null) return;
    const target = pendingSeekTarget;
    pendingSeekTarget = null;
    dispatchSeek(target, false);
  }

  function scheduleSeek(timeMs: number, final = false) {
    dragTargetTimeMs = Math.max(timelineStartMs, Math.min(timelineEndMs, Math.round(timeMs)));
    onSeekTarget(dragTargetTimeMs, final);
    if (final) {
      pendingSeekTarget = null;
      if (seekFrame !== undefined) {
        cancelAnimationFrame(seekFrame);
        seekFrame = undefined;
      }
      dispatchSeek(dragTargetTimeMs, true);
      return;
    }
    // Pointer events can arrive faster than the native IPC round trip. Keep
    // the visual target immediate, but send at most one approximate seek per
    // animation frame; the app-level broker coalesces anything still in
    // flight.
    pendingSeekTarget = dragTargetTimeMs;
    if (seekFrame === undefined) seekFrame = requestAnimationFrame(flushSeekFrame);
  }

  function moveScrub(event: PointerEvent) {
    if (!scrubbing) return;
    const delta = event.clientX - dragStartX;
    if (Math.abs(delta) >= 3) dragMoved = true;
    if (!dragMoved) return;
    const next = dragStartTimeMs + delta / dragWidth * dragWindowSpanMs;
    scheduleSeek(next);
  }

  function endScrub(event: PointerEvent) {
    const finalTime = dragMoved
      ? dragTargetTimeMs
      : pointerTime(event, dragWindowStartMs, dragWindowSpanMs);
    scrubbing = false;
    if (dragMoved || !scrubStartedOnEvent) scheduleSeek(finalTime, true);
    if (resumeFollowAfterScrub) {
      resumeFollowAfterScrub = false;
      followPlayhead = true;
    }
    const element = event.currentTarget as HTMLElement;
    if (dragMoved) suppressEventClickUntil = performance.now() + 250;
    scrubStartedOnEvent = false;
    if (element.hasPointerCapture(event.pointerId)) element.releasePointerCapture(event.pointerId);
  }

  function cancelScrub(event: PointerEvent) {
    scrubbing = false;
    if (dragMoved || pendingSeekTarget !== null) {
      scheduleSeek(dragTargetTimeMs, true);
    }
    if (resumeFollowAfterScrub) {
      resumeFollowAfterScrub = false;
      followPlayhead = true;
    }
    scrubStartedOnEvent = false;
    const element = event.currentTarget as HTMLElement;
    if (element.hasPointerCapture(event.pointerId)) element.releasePointerCapture(event.pointerId);
  }

  function setZoom(nextZoom: number) {
    const center = viewStartMs + viewSpanMs / 2;
    const clampedZoom = Math.max(minimumZoom, Math.min(24, nextZoom));
    const nextViewSpan = Math.min(timelineSpanMs, Math.max(5_000, 120_000 / clampedZoom));
    zoom = clampedZoom;
    followPlayhead = false;
    scrollStartMs = Math.max(timelineStartMs, Math.min(timelineEndMs - nextViewSpan, center - nextViewSpan / 2));
  }

  function panView(ratio: number) {
    followPlayhead = false;
    scrollStartMs = Math.max(timelineStartMs, Math.min(timelineStartMs + scrollMaximumMs, Math.round(viewStartMs + viewSpanMs * ratio)));
  }

  function toggleFollowPlayhead() {
    followPlayhead = !followPlayhead;
  }

  function setViewStart(value: number) {
    followPlayhead = false;
    scrollStartMs = Math.max(timelineStartMs, Math.min(timelineStartMs + scrollMaximumMs, value));
  }

  function scrollbarValue(event: PointerEvent) {
    const trackBounds = scrollbar.getBoundingClientRect();
    const thumbWidth = scrollbarThumb.getBoundingClientRect().width;
    const available = Math.max(1, trackBounds.width - thumbWidth);
    const ratio = Math.max(0, Math.min(1, (event.clientX - trackBounds.left - scrollbarGrabOffset) / available));
    return Math.round(timelineStartMs + ratio * scrollMaximumMs);
  }

  function beginScrollbar(event: PointerEvent) {
    if (scrollMaximumMs <= 0) return;
    const thumbBounds = scrollbarThumb.getBoundingClientRect();
    scrollbarGrabOffset = event.target === scrollbarThumb
      ? event.clientX - thumbBounds.left
      : thumbBounds.width / 2;
    scrollbarDragging = true;
    scrollbarPointerId = event.pointerId;
    scrollbar.setPointerCapture(event.pointerId);
    setViewStart(scrollbarValue(event));
  }

  function moveScrollbar(event: PointerEvent) {
    if (!scrollbarDragging || event.pointerId !== scrollbarPointerId) return;
    setViewStart(scrollbarValue(event));
  }

  function endScrollbar(event: PointerEvent) {
    if (!scrollbarDragging || event.pointerId !== scrollbarPointerId) return;
    scrollbarDragging = false;
    if (scrollbar.hasPointerCapture(event.pointerId)) scrollbar.releasePointerCapture(event.pointerId);
    scrollbarPointerId = -1;
  }

  function scrollbarKeydown(event: KeyboardEvent) {
    const increment = Math.max(1_000, Math.round(viewSpanMs * .08));
    if (event.key === "ArrowLeft" || event.key === "ArrowDown") setViewStart(viewStartMs - increment);
    else if (event.key === "ArrowRight" || event.key === "ArrowUp") setViewStart(viewStartMs + increment);
    else if (event.key === "PageUp") setViewStart(viewStartMs - viewSpanMs * .8);
    else if (event.key === "PageDown") setViewStart(viewStartMs + viewSpanMs * .8);
    else if (event.key === "Home") setViewStart(timelineStartMs);
    else if (event.key === "End") setViewStart(timelineStartMs + scrollMaximumMs);
    else return;
    event.preventDefault();
  }

  function zoomFromWheel(event: WheelEvent) {
    if (!event.ctrlKey && !event.metaKey) return;
    event.preventDefault();
    setZoom(zoom * (event.deltaY < 0 ? 1.25 : .8));
  }

  function timelineKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowLeft") scheduleSeek(Math.max(timelineStartMs, timelineTimeMs - 1000), true);
    else if (event.key === "ArrowRight") scheduleSeek(Math.min(timelineEndMs, timelineTimeMs + 1000), true);
    else if (event.key === "PageUp") panView(-.8);
    else if (event.key === "PageDown") panView(.8);
    else if (event.key === "Home") scheduleSeek(timelineStartMs, true);
    else if (event.key === "End") scheduleSeek(timelineEndMs, true);
    else return;
    event.preventDefault();
  }

  function textSegments(item: TimelineEvent) {
    const signature = `${item.text}\u0000${item.highlights.map((range) => `${range.start}:${range.end}:${range.feature}`).join(";")}`;
    const cached = textSegmentCache.get(item.index);
    if (cached?.signature === signature) return cached.segments;
    const chars = [...item.text];
    // Build a sweep-line index once per event instead of testing every
    // highlight range against every character. Caption lines can contain
    // thousands of code points; the previous O(characters × ranges) loop
    // caused a noticeable synchronous pause when the event list mounted.
    const starts = new Map<number, TimelineFeature[]>();
    const ends = new Map<number, TimelineFeature[]>();
    for (const range of item.highlights) {
      const start = Math.max(0, Math.min(chars.length, range.start));
      const end = Math.max(start, Math.min(chars.length, range.end));
      if (end <= start) continue;
      const started = starts.get(start);
      if (started) started.push(range.feature); else starts.set(start, [range.feature]);
      const ended = ends.get(end);
      if (ended) ended.push(range.feature); else ends.set(end, [range.feature]);
    }
    const active = new Map<TimelineFeature, number>();
    const segments: { text: string; features: TimelineFeature[] }[] = [];
    for (let index = 0; index < chars.length; index += 1) {
      // End markers are processed before starts so ranges stay half-open:
      // [begin, end). Adjacent highlights therefore do not overlap at a
      // shared boundary.
      for (const feature of ends.get(index) ?? []) {
        const count = active.get(feature) ?? 0;
        if (count <= 1) active.delete(feature); else active.set(feature, count - 1);
      }
      for (const feature of starts.get(index) ?? []) active.set(feature, (active.get(feature) ?? 0) + 1);
      const activeFeatures = featureOptions.filter((feature) => active.has(feature));
      const key = activeFeatures.join("|");
      const previous = segments[segments.length - 1];
      if (previous && previous.features.join("|") === key) previous.text += chars[index];
      else segments.push({ text: chars[index], features: activeFeatures });
    }
    textSegmentCache.set(item.index, { signature, segments });
    // A live archive can run indefinitely. Retain only a small working set so
    // the optimization never turns into an unbounded UI-side cache.
    if (textSegmentCache.size > 2_000) {
      const first = textSegmentCache.keys().next().value;
      if (typeof first === "number") textSegmentCache.delete(first);
    }
    return segments;
  }

  $: filterKey = [...filters].sort().join(",");
  $: if (!editor && desktopRuntime && archivePath && (archivePath !== loadedArchive || filterKey !== loadedFilterKey || loadedLive !== live)) {
    loadedFilterKey = filterKey;
    loadedLive = live;
    void (live ? loadRecentPage() : loadPage(true));
  }
  $: if (editor && desktopRuntime && archivePath && !loading && editorWindowNeedsLoad) void loadEditorWindow();
  $: if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = undefined; }
  $: if (desktopRuntime && live && archivePath) refreshTimer = setInterval(() => void (editor ? loadEditorWindow(true) : loadRecentPage()), 1_500);
  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
    if (seekFrame !== undefined) cancelAnimationFrame(seekFrame);
    if (scrubbing || pendingSeekTarget !== null) {
      scrubbing = false;
      onSeekTarget(dragTargetTimeMs, true);
      dispatchSeek(dragTargetTimeMs, true);
    }
  });
</script>

{#if editor}
  <section class="caption-timeline" class:dragging={scrubbing} aria-label={t("timeline.editorLabel")} onwheel={zoomFromWheel}>
    <header class="timeline-toolbar">
      <b>{t("timeline.editorTitle")}</b>
      <div class="timeline-tools">
        <button class="timeline-step" data-tooltip={t("timeline.previousWindow")} aria-label={t("timeline.previousWindow")} onclick={() => panView(-.8)}><ChevronLeft size={14} /></button>
        <span class="zoom-label">{t("timeline.zoom")}</span>
        <MacSlider className="timeline-zoom-slider" ariaLabel={t("timeline.zoom")} min={minimumZoom} max={24} step={0.05} value={zoom} onInput={setZoom} />
        <output class="zoom-value" aria-live="polite">{zoomPercent}%</output>
        <button class="timeline-step" data-tooltip={t("timeline.nextWindow")} aria-label={t("timeline.nextWindow")} onclick={() => panView(.8)}><ChevronRight size={14} /></button>
        <button class="timeline-follow" class:active={followPlayhead} data-tooltip={t("timeline.followPlayhead")} aria-label={t("timeline.followPlayhead")} aria-pressed={followPlayhead} onclick={toggleFollowPlayhead}><LocateFixed size={14} /><span>{t("timeline.followPlayhead")}</span></button>
        <button class="timeline-fit" data-tooltip={t("timeline.fit")} aria-label={t("timeline.fit")} onclick={() => setZoom(120_000 / timelineSpanMs)}><ScanLine size={14} /><span>{t("timeline.fit")}</span></button>
      </div>
    </header>
    <div class="timeline-ruler-row">
      <span class="timeline-ruler-gutter" aria-hidden="true"></span>
      <div class="timeline-ruler" class:playing role="slider" tabindex="0" aria-label={t("preview.seekTimeline")} aria-valuemin={timelineStartMs} aria-valuemax={timelineEndMs} aria-valuenow={timelineTimeMs} onkeydown={timelineKeydown} onpointerdown={beginScrub} onpointermove={moveScrub} onpointerup={endScrub} onpointercancel={cancelScrub}>
        {#each ticks as tick, index}{#if tick.time >= timelineStartMs && tick.time <= timelineEndMs}<span class:first-tick={index === 0} class:middle-tick={index === 2} class:last-tick={index === ticks.length - 1} style={`left:${tick.percent}%`}><i></i><small>{rulerTime(tick.time, tickPrecision)}</small></span>{/if}{/each}
        {#if playheadVisible}<em style={`left:${cursorPositionPercent}%`}></em>{/if}
      </div>
    </div>
    <div class="timeline-lanes" id="timeline-visible-content">
      <div class="timeline-lane">
        <div class="timeline-lane-label"><b>{trackName || t("timeline.selectedTrack")}</b>{#if trackDetail || trackLabel}<small>{trackDetail || trackLabel}</small>{/if}</div>
        <div class="timeline-track" class:playing role="slider" tabindex="0" aria-label={t("preview.seekTimeline")} aria-valuemin={timelineStartMs} aria-valuemax={timelineEndMs} aria-valuenow={timelineTimeMs} onkeydown={timelineKeydown} onpointerdown={beginScrub} onpointermove={moveScrub} onpointerup={endScrub} onpointercancel={cancelScrub}>
          {#each positionedRecords as positioned (positioned.item.index)}
            {@const item = positioned.item}
            {@const features = positioned.features}
            <button use:registerEventNode={item.index} class:ttml={item.kind === "caption"} class:scene={item.kind === "scene"} data-tooltip={`${timestamp(item.beginMs)} · ${item.text || kind(item.kind)}`} aria-label={`${timestamp(item.beginMs)} · ${item.text || kind(item.kind)}`} style={positioned.style} onpointerdown={beginEventScrub} onclick={(event) => { event.stopPropagation(); if (performance.now() < suppressEventClickUntil) return; scheduleSeek(item.beginMs, true); }}>
              <span class="timeline-event-features">
                {#each features.slice(0, 2) as feature}<span class="timeline-event-feature" style={`--feature-color:${featureMeta[feature].color}`}><img src={featureMeta[feature].icon} alt="" /><span>{t(`feature.${feature}`)}</span></span>{/each}
                {#if features.length > 2}<span class="timeline-feature-overflow">+{features.length - 2}</span>{/if}
              </span>
              <span class="timeline-event-text">{item.text || kind(item.kind)}</span>
            </button>
          {/each}
          {#if playheadVisible}<i class="timeline-playhead" style={`left:${cursorPositionPercent}%`}></i>{/if}
        </div>
      </div>
    </div>
    <div class="timeline-scrollbar-row">
      <span class="timeline-scrollbar-gutter" aria-hidden="true"></span>
      <div class="timeline-scrollbar" class:dragging={scrollbarDragging} bind:this={scrollbar} role="scrollbar" tabindex="0" aria-controls="timeline-visible-content" aria-label={t("timeline.visibleRange")} aria-orientation="horizontal" aria-valuemin={timelineStartMs} aria-valuemax={timelineStartMs + scrollMaximumMs} aria-valuenow={Math.round(viewStartMs)} onkeydown={scrollbarKeydown} onpointerdown={beginScrollbar} onpointermove={moveScrollbar} onpointerup={endScrollbar} onpointercancel={endScrollbar}>
        <span class="timeline-scroll-thumb" bind:this={scrollbarThumb} style={`left:${scrollbarThumbLeft}%;width:${scrollbarThumbPercent}%`}></span>
      </div>
    </div>
    <div class="timeline-now">
      <time>{timestamp(activeRecord?.beginMs ?? timelineTimeMs)}</time>
      <strong>{activeRecord?.text || t("timeline.noText")}</strong>
      <button class="timeline-mapping outline" onclick={onOpenMapping}><GitCompareArrows size={14} />{t("preview.mappingTitle")}<ChevronRight size={14} /></button>
    </div>
    {#if !records.length}<p>{live ? t("workspace.eventsEmpty") : t("timeline.empty")}</p>{/if}
  </section>
{:else}
  <section class="event-list timeline-list">
    <header><b>{t("workspace.eventsTitle")}</b><span>{expectedCount > records.length ? t("timeline.loadedCount").replace("{0}", String(records.length)).replace("{1}", expectedCount.toLocaleString()) : t("workspace.eventsCount").replace("{0}", String(records.length || expectedCount))}</span></header>
    <div class="event-filters" aria-label={t("timeline.filters")}>
      {#each featureOptions as feature}<span class:active={filters.has(feature)}><MacCheckbox checked={filters.has(feature)} label={t(`feature.${feature}`)} onChange={() => toggleFilter(feature)} /></span>{/each}
    </div>
    {#if !archivePath}<div class="event-empty"><FileText size={30} /><p>{live ? t("workspace.eventsEmpty") : t("timeline.archiveRequired")}</p></div>
    {:else if records.length}<ol>{#each eventListItems as entry (entry.item.index)}{@const item = entry.item}<li><time>{timestamp(item.beginMs)}<small>{eventEndTimestamp(item.endMs)}</small></time><button class="event-content" onclick={() => scheduleSeek(item.beginMs, true)}><strong>{kind(item.kind)}{#if item.regionX !== null && item.regionY !== null}<small>{t("timeline.region").replace("{0}", String(item.regionX)).replace("{1}", String(item.regionY))}</small>{/if}</strong><span class="feature-badges">{#each item.features as feature}{#if feature === "color" && item.colors?.length}{#each item.colors as color}<i class="feature-color" data-tooltip={t(`feature.color.${color.role}`)}><span class="color-swatch" style={`background:${color.value}`}></span>{color.value}</i>{/each}{:else}<i class={`feature-${feature}`}>{t(`feature.${feature}`)}</i>{/if}{/each}</span><small>{#each entry.segments as segment}{#if segment.features.length}<mark class={segment.features.map((feature) => `feature-${feature}`).join(" ")} data-tooltip={segment.features.map((feature) => t(`feature.${feature}`)).join(" · ")}>{segment.text}</mark>{:else}{segment.text}{/if}{/each}</small></button></li>{/each}</ol>{#if !exhausted}<button class="load-more" onclick={() => loadPage(false)} disabled={loading}><RefreshCw size={15} /> {loading ? t("workspace.loading") : t("workspace.loadMore")}</button>{/if}
    {:else}<div class="event-empty"><Clock3 size={30} /><p>{loading ? t("workspace.loading") : live ? t("workspace.eventsEmpty") : t("timeline.empty")}</p></div>{/if}
  </section>
{/if}

<style>
  .event-list { min-height: 360px; color: var(--rw-text); }
  header { display: flex; justify-content: space-between; gap: 12px; padding: 13px 15px; border-bottom: 1px solid var(--rw-border); }
  header span { color: var(--rw-muted); font-size: 12px; }
  .event-filters { display: flex; flex-wrap: wrap; gap: 6px; padding: 9px 15px; border-bottom: 1px solid var(--rw-border-subtle); }
  ol { margin: 0; padding: 0; list-style: none; }
  li { display: grid; grid-template-columns: 144px minmax(0, 1fr); gap: 18px; padding: 13px 15px; border-bottom: 1px solid var(--rw-border-subtle); content-visibility: auto; contain-intrinsic-size: auto 76px; }
  time { color: var(--rw-accent); font: 12px/1.4 var(--rw-font-mono); }
  strong, small { display: block; }
  strong { color: var(--rw-text); font: 12px/1.4 var(--rw-font-mono); }
  time small, strong small { margin-top: 4px; color: var(--rw-muted); font: 11px var(--rw-font-mono); }
  .event-content { min-width: 0; padding: 0; color: inherit; background: transparent; text-align: left; }
  .event-content > small { margin-top: 7px; color: var(--rw-text-secondary); font-size: 13px; line-height: 1.6; white-space: pre-wrap; }
  .feature-badges { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 6px; }
  .feature-badges i { padding: 2px 5px; border-radius: 3px; background: var(--rw-surface-raised); color: var(--rw-muted); font-size: 9px; font-style: normal; }
  mark { padding: 1px 0; color: inherit; border-radius: 2px; background: #6e5b163f; }
  .feature-ruby { border-bottom: 1px solid #56a8ff !important; }
  .feature-drcs { background: #a74c6f45 !important; }
  .feature-gaiji { background: #7358b94d !important; }
  .feature-accessibility { background: #26795c4d !important; }
  .feature-color { background: #8f6b244d !important; }
  .event-empty { display: grid; place-items: center; gap: 10px; min-height: 220px; padding: 20px; color: var(--rw-muted); text-align: center; }
  .load-more { display: flex; align-items: center; gap: 7px; margin: 14px auto; padding: 8px 12px; color: var(--rw-text); border: 1px solid var(--rw-border); border-radius: 5px; background: var(--rw-surface-raised); }
  .caption-timeline {
    --timeline-gutter: var(--rw-timeline-gutter, 92px);
    --timeline-axis-inset: var(--rw-timeline-axis-inset, 10px);
    --timeline-thumb-inset: 8px;
    margin: 8px 0 0;
    overflow: hidden;
    border: 1px solid var(--rw-border-subtle);
    border-radius: 6px;
    color: var(--rw-text);
    background: var(--rw-content);
  }
  .timeline-toolbar {
    display: flex;
    align-items: center;
    min-height: 40px;
    padding: 4px 7px;
    border-bottom: 1px solid var(--rw-border-subtle);
    background: var(--rw-content);
  }
  .timeline-toolbar > b { flex: 0 0 auto; font-size: 11px; line-height: 15px; font-weight: 650; }
  .timeline-tools { display: flex; align-items: center; justify-content: flex-end; min-width: 0; margin-left: auto; gap: 5px; }
  .timeline-tools .timeline-step {
    display: grid;
    place-items: center;
    width: 28px !important;
    height: 28px !important;
    min-height: 28px !important;
    flex: 0 0 28px;
    padding: 0;
    border-radius: 50% !important;
    color: var(--rw-text-secondary) !important;
  }
  .timeline-tools :global(svg), .timeline-mapping :global(svg) { display: block; margin: 0; }
  .timeline-tools .zoom-label { min-width: auto; color: var(--rw-muted); font: 10px/14px var(--rw-font-ui); white-space: nowrap; }
  .timeline-tools :global(.timeline-zoom-slider) { width: 92px; flex: 0 1 92px; }
  .timeline-tools .zoom-value { width: 34px; color: var(--rw-text-secondary); font: 9px/14px var(--rw-font-mono); text-align: right; white-space: nowrap; }
  .timeline-tools .timeline-fit, .timeline-tools .timeline-follow {
    display: flex;
    align-items: center;
    justify-content: center;
    width: auto !important;
    height: 28px !important;
    min-height: 28px !important;
    padding: 0 9px;
    gap: 5px;
    border-radius: 7px !important;
    font-size: 10px;
    line-height: 14px;
    white-space: nowrap;
  }
  .timeline-tools .timeline-follow.active { color: var(--rw-accent) !important; border-color: color-mix(in srgb, var(--rw-accent) 45%, var(--rw-border)); background: color-mix(in srgb, var(--rw-accent) 9%, transparent); }
  .timeline-ruler-row { display: grid; grid-template-columns: var(--timeline-gutter) minmax(0, 1fr); height: 30px; padding-inline: var(--timeline-axis-inset); border-bottom: 1px solid var(--rw-border-subtle); background: var(--rw-surface-muted); }
  .timeline-ruler-gutter { border-right: 1px solid var(--rw-border-subtle); }
  .timeline-ruler { position: relative; min-width: 0; height: 30px; margin-inline: var(--timeline-thumb-inset); cursor: grab; touch-action: none; user-select: none; }
  .timeline-ruler > span { position: absolute; top: 0; bottom: 0; }
  .timeline-ruler > span > i { display: block; width: 1px; height: 8px; background: color-mix(in srgb, var(--rw-text) 24%, transparent); }
  .timeline-ruler > span > small { position: absolute; top: 11px; left: 0; color: var(--rw-muted); font: 8px/11px var(--rw-font-mono); white-space: nowrap; transform: translateX(-50%); }
  .timeline-ruler > span.first-tick > small { left: 4px; transform: none; }
  .timeline-ruler > span.last-tick > small { right: 4px; left: auto; transform: none; }
  .timeline-ruler > span.middle-tick > i { height: 11px; background: color-mix(in srgb, var(--rw-text) 38%, transparent); }
  .timeline-ruler > span.middle-tick > small { color: var(--rw-text-secondary); font-weight: 600; }
  .timeline-ruler > em, .timeline-playhead { position: absolute; z-index: 6; top: 0; bottom: 0; width: 2px; margin-left: -1px; background: var(--rw-accent); pointer-events: none; }
  .timeline-ruler.playing > em, .timeline-track.playing > .timeline-playhead { transition: left 100ms linear; }
  .caption-timeline.dragging .timeline-ruler > em,
  .caption-timeline.dragging .timeline-track > .timeline-playhead { transition: none; }
  .timeline-ruler > em::before { position: absolute; top: 0; left: -4px; border-top: 6px solid var(--rw-accent); border-right: 5px solid transparent; border-left: 5px solid transparent; content: ""; }
  .timeline-lanes { min-width: 0; }
  .timeline-lane { display: grid; grid-template-columns: var(--timeline-gutter) minmax(0, 1fr); min-height: 72px; padding-inline: var(--timeline-axis-inset); background: var(--rw-content); }
  .timeline-lane-label { display: flex; flex-direction: column; justify-content: center; min-width: 0; padding: 7px 8px; border-right: 1px solid var(--rw-border-subtle); background: color-mix(in srgb, var(--rw-surface-muted) 70%, var(--rw-content)); }
  .timeline-lane-label b { overflow: hidden; color: var(--rw-text-secondary); font-size: 9px; line-height: 12px; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }
  .timeline-lane-label small { display: -webkit-box; overflow: hidden; margin-top: 2px; color: var(--rw-muted); font-size: 8px; line-height: 11px; -webkit-box-orient: vertical; -webkit-line-clamp: 3; line-clamp: 3; }
  .timeline-track {
    position: relative;
    min-width: 0;
    height: 72px;
    margin-inline: var(--timeline-thumb-inset);
    overflow: hidden;
    background: repeating-linear-gradient(90deg, transparent 0, transparent calc(25% - 1px), color-mix(in srgb, var(--rw-border-subtle) 76%, transparent) calc(25% - 1px), color-mix(in srgb, var(--rw-border-subtle) 76%, transparent) 25%);
    cursor: grab;
    touch-action: none;
    user-select: none;
  }
  .timeline-track button {
    position: absolute;
    z-index: 2;
    top: 10px;
    height: 52px;
    min-width: 2px;
    overflow: hidden;
    padding: 5px 6px 4px;
    border: 1px solid color-mix(in srgb, var(--event-color, var(--rw-accent)) 42%, var(--rw-border));
    border-left: 3px solid var(--event-color, var(--rw-accent));
    border-radius: 5px;
    color: var(--rw-text);
    background: color-mix(in srgb, var(--event-color, var(--rw-accent)) 7%, var(--rw-content));
    box-shadow: 0 1px 2px rgba(0, 0, 0, .05), inset 0 .5px rgba(255, 255, 255, .62);
    container-type: inline-size;
    text-align: left;
  }
  .timeline-track button:hover { background: color-mix(in srgb, var(--event-color, var(--rw-accent)) 12%, var(--rw-content)); }
  .timeline-track button:global(.current) { border-color: color-mix(in srgb, var(--rw-accent) 82%, var(--rw-border)); box-shadow: 0 0 0 1px color-mix(in srgb, var(--rw-accent) 42%, transparent), 0 2px 5px rgba(0, 68, 150, .12); }
  .timeline-event-features { display: flex; align-items: center; height: 15px; overflow: hidden; gap: 4px; white-space: nowrap; }
  .timeline-event-feature { display: inline-flex; align-items: center; min-width: 0; gap: 2px; color: var(--feature-color); font-size: 8px; line-height: 12px; font-weight: 650; }
  .timeline-event-feature img { display: block; width: 13px; height: 13px; flex: 0 0 13px; }
  .timeline-feature-overflow { flex: 0 0 auto; color: var(--rw-muted); font: 8px/12px var(--rw-font-mono); }
  .timeline-event-text { display: block; overflow: hidden; margin-top: 4px; color: var(--rw-text); font-size: 10px; line-height: 13px; font-weight: 560; text-overflow: ellipsis; white-space: nowrap; }
  .timeline-event-features:empty + .timeline-event-text { margin-top: 12px; }
  .timeline-scrollbar-row { display: grid; grid-template-columns: var(--timeline-gutter) minmax(0, 1fr); height: 20px; padding-inline: var(--timeline-axis-inset); border-top: 1px solid var(--rw-border-subtle); border-bottom: 1px solid var(--rw-border-subtle); background: var(--rw-surface-muted); }
  .timeline-scrollbar-gutter { border-right: 1px solid var(--rw-border-subtle); }
  .timeline-scrollbar { position: relative; min-width: 0; height: 20px; margin-inline: var(--timeline-thumb-inset); outline-offset: -2px; cursor: default; touch-action: none; user-select: none; }
  .timeline-scrollbar::before { position: absolute; top: 7px; right: 4px; left: 4px; height: 5px; border-radius: 3px; background: color-mix(in srgb, var(--rw-text) 10%, transparent); box-shadow: inset 0 .5px rgba(0, 0, 0, .08); content: ""; }
  .timeline-scroll-thumb { position: absolute; z-index: 1; top: 6px; height: 7px; min-width: 22px; border: .5px solid color-mix(in srgb, var(--rw-text) 20%, transparent); border-radius: 4px; background: color-mix(in srgb, var(--rw-text) 40%, var(--rw-content)); box-shadow: 0 .5px 1px rgba(0, 0, 0, .16), inset 0 .5px rgba(255, 255, 255, .44); cursor: grab; }
  .timeline-scrollbar.dragging .timeline-scroll-thumb { background: color-mix(in srgb, var(--rw-text) 52%, var(--rw-content)); cursor: grabbing; }
  .timeline-now { display: grid; grid-template-columns: 88px minmax(0, 1fr) auto; align-items: center; min-height: 40px; padding: 4px 7px; gap: 8px; background: color-mix(in srgb, var(--rw-accent) 4%, var(--rw-content)); }
  .timeline-now time { overflow: hidden; color: var(--rw-muted); font: 9px/13px var(--rw-font-mono); text-overflow: ellipsis; white-space: nowrap; }
  .timeline-now strong { overflow: hidden; color: var(--rw-text-secondary); font: 10px/14px var(--rw-font-ui); font-weight: 560; text-overflow: ellipsis; white-space: nowrap; }
  .timeline-mapping { display: flex; align-items: center; justify-content: center; height: 28px !important; min-height: 28px !important; padding: 0 8px !important; gap: 5px; border-radius: 6px !important; font-size: 10px; line-height: 14px; white-space: nowrap; }
  .caption-timeline > p { margin: 0; padding: 15px; border-top: 1px solid var(--rw-border-subtle); color: var(--rw-muted); font-size: 11px; text-align: center; }
  .caption-timeline.dragging .timeline-ruler, .caption-timeline.dragging .timeline-track { cursor: grabbing; }
  @media(max-width: 900px) {
    li { grid-template-columns: 1fr; gap: 6px; }
    .caption-timeline { --rw-timeline-gutter: 82px; }
    .timeline-tools .zoom-label { display: none; }
    .timeline-tools :global(.timeline-zoom-slider) { width: 64px; flex-basis: 64px; }
    .timeline-tools .timeline-fit span { display: none; }
    .timeline-tools .timeline-fit, .timeline-tools .timeline-follow { width: 28px !important; padding: 0; border-radius: 50% !important; }
    .timeline-tools .timeline-follow span { display: none; }
    .timeline-now { grid-template-columns: 80px minmax(0, 1fr) auto; }
  }
  .feature-badges i.feature-color{display:inline-flex;align-items:center;gap:4px;background:var(--rw-surface-raised)!important}.color-swatch{width:9px;height:9px;border:1px solid color-mix(in srgb,var(--rw-text) 32%,transparent);border-radius:2px;box-shadow:inset 0 0 0 1px #ffffff26}.event-content mark{background:transparent!important}.event-content mark.feature-accessibility{background:#26795c4d!important}.event-content mark.feature-ruby{border-top:1px solid #56a8ff!important;border-bottom:0!important}.event-content mark.feature-drcs{box-shadow:inset 0 -2px #d46b9a}.event-content mark.feature-gaiji{border-bottom:1px dashed #9d82e8}.event-content mark.feature-color{background:transparent!important}.timeline-ruler,.timeline-track{cursor:grab}.caption-timeline.dragging .timeline-ruler,.caption-timeline.dragging .timeline-track{cursor:grabbing}
  .event-filters>span{display:flex;align-items:center;padding:2px 7px;border:1px solid var(--rw-border);border-radius:6px;color:var(--rw-muted)}.event-filters>span.active{border-color:color-mix(in srgb,var(--rw-accent) 58%,var(--rw-border));color:var(--rw-text);background:color-mix(in srgb,var(--rw-accent) 9%,transparent)}.event-filters :global(.mac-checkbox){font-size:11px}
  @container (max-width: 84px){.timeline-event-feature span{display:none}.timeline-event-features{gap:2px}.timeline-event-text{margin-top:3px!important;font-size:9px!important}}
  @container (max-width: 36px){.timeline-track button{padding-inline:3px}.timeline-event-features{justify-content:center}.timeline-event-feature:not(:first-child),.timeline-event-text{display:none!important}}
</style>

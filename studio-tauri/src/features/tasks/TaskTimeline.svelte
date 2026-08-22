<script lang="ts">
  import { ChevronLeft, ChevronRight, Clock3, FileText, GitCompareArrows, RefreshCw, ScanLine } from "@lucide/svelte";
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
  export let durationMs = 0;
  export let trackLabel = "";
  export let trackName = "";
  export let trackDetail = "";
  export let expectedCount = 0;
  export let onSeek: (milliseconds: number) => void = () => {};
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
  let loadedEditorWindow = "";
  let loadedFilterKey = "";
  let loadedLive = false;
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
  let dragTargetTimeMs = 0;
  let seekFrame: number | undefined;
  let scrollStartMs = 0;
  let followPlayhead = true;
  let followedTimeMs = -1;
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
  const rulerTime = (timeMs: number) => timestamp(timeMs).replace(/\.\d{3}$/, "");
  const kind = (value: TimelineEvent["kind"]) => t(`timeline.kind.${value}`, value);
  $: timelineTimeMs = scrubbing ? dragTargetTimeMs : currentTimeMs;
  $: maximumSpanMs = Math.max(5_000, durationMs || 120_000);
  $: minimumZoom = Math.min(.25, 120_000 / maximumSpanMs);
  $: viewSpanMs = Math.min(maximumSpanMs, Math.max(5_000, 120_000 / zoom));
  $: scrollMaximumMs = Math.max(0, maximumSpanMs - viewSpanMs);
  $: if (followPlayhead && timelineTimeMs !== followedTimeMs) {
    followedTimeMs = timelineTimeMs;
    scrollStartMs = Math.max(0, Math.min(scrollMaximumMs, timelineTimeMs - viewSpanMs / 2));
  }
  $: viewStartMs = Math.max(0, Math.min(scrollMaximumMs, scrollStartMs));
  $: visibleRecords = records.filter((item) => item.endMs >= viewStartMs && item.beginMs <= viewStartMs + viewSpanMs && (!item.trackId || !trackLabel || item.trackId === trackLabel));
  $: ticks = Array.from({ length: 5 }, (_, index) => ({ percent: index * 25, time: viewStartMs + viewSpanMs * index / 4 }));
  $: zoomPercent = Math.max(1, Math.round(zoom * 50));
  $: scrollbarThumbPercent = Math.max(7, Math.min(100, viewSpanMs / maximumSpanMs * 100));
  $: scrollbarThumbLeft = scrollMaximumMs <= 0 ? 0 : viewStartMs / scrollMaximumMs * (100 - scrollbarThumbPercent);
  $: activeRecord = visibleRecords.find((item) => item.kind === "caption" && timelineTimeMs >= item.beginMs && timelineTimeMs <= item.endMs)
    ?? visibleRecords.find((item) => timelineTimeMs >= item.beginMs && timelineTimeMs <= item.endMs)
    ?? null;
  const cursorPercent = () => Math.max(0, Math.min(100, ((timelineTimeMs - viewStartMs) / viewSpanMs) * 100));
  const visibleFeatures = (item: TimelineEvent) => item.features.filter((feature): feature is Exclude<TimelineFeature, "position"> => feature !== "position");
  const eventColor = (item: TimelineEvent) => featureMeta[visibleFeatures(item)[0]]?.color ?? "var(--rw-accent)";
  const barStyle = (item: TimelineEvent) => {
    const left = Math.max(0, Math.min(100, ((item.beginMs - viewStartMs) / viewSpanMs) * 100));
    const right = Math.max(left + 0.2, Math.min(100, ((item.endMs - viewStartMs) / viewSpanMs) * 100));
    return `left:${left}%;width:${right - left}%;`;
  };

  async function loadPage(reset: boolean) {
    if (!desktopRuntime || !archivePath || loading || (!reset && exhausted && !live)) return;
    const requestedArchive = archivePath;
    loading = true;
    if (reset) { loadedArchive = archivePath; records = []; exhausted = false; }
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

  async function loadEditorWindow() {
    if (!desktopRuntime || !archivePath || loading) return;
    const requestedArchive = archivePath;
    const windowKey = `${archivePath}:${Math.floor(viewStartMs / 2_000)}:${Math.round(viewSpanMs)}`;
    loading = true;
    try {
      const next = await getTimelineTimeWindow(
        archivePath,
        Math.max(0, Math.floor(viewStartMs)),
        Math.min(maximumSpanMs, Math.ceil(viewStartMs + viewSpanMs)),
        200,
      );
      records = next.items;
      loadedArchive = archivePath;
      loadedEditorWindow = windowKey;
      exhausted = !next.hasMore;
    } catch (reason) {
      if (!live) onError(String(reason));
    } finally {
      loading = false;
      const currentKey = `${archivePath}:${Math.floor(viewStartMs / 2_000)}:${Math.round(viewSpanMs)}`;
      if (archivePath && (archivePath !== requestedArchive || currentKey !== windowKey))
        queueMicrotask(() => void loadEditorWindow());
    }
  }

  async function loadRecentPage() {
    if (!desktopRuntime || !archivePath || loading) return;
    const requestedArchive = archivePath;
    loading = true;
    try {
      const next = await getRecentTimelineWindow(archivePath, pageSize, [...filters]);
      records = next.items;
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

  function pointerTime(event: PointerEvent, windowStart = viewStartMs, windowSpan = viewSpanMs) {
    const element = event.currentTarget as HTMLElement;
    const bounds = element.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (event.clientX - bounds.left) / Math.max(1, bounds.width)));
    return Math.max(0, Math.min(maximumSpanMs, Math.round(windowStart + ratio * windowSpan)));
  }

  function beginScrub(event: PointerEvent) {
    scrubbing = true;
    dragStartX = event.clientX;
    dragStartTimeMs = pointerTime(event);
    dragWindowStartMs = viewStartMs;
    dragWindowSpanMs = viewSpanMs;
    dragWidth = Math.max(1, (event.currentTarget as HTMLElement).getBoundingClientRect().width);
    dragMoved = false;
    dragTargetTimeMs = dragStartTimeMs;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function scheduleSeek(timeMs: number) {
    dragTargetTimeMs = Math.max(0, Math.min(maximumSpanMs, Math.round(timeMs)));
    if (seekFrame !== undefined) return;
    seekFrame = requestAnimationFrame(() => {
      seekFrame = undefined;
      onSeek(dragTargetTimeMs);
    });
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
    if (seekFrame !== undefined) cancelAnimationFrame(seekFrame);
    seekFrame = undefined;
    onSeek(finalTime);
    scrubbing = false;
    const element = event.currentTarget as HTMLElement;
    if (element.hasPointerCapture(event.pointerId)) element.releasePointerCapture(event.pointerId);
  }

  function cancelScrub(event: PointerEvent) {
    if (seekFrame !== undefined) cancelAnimationFrame(seekFrame);
    seekFrame = undefined;
    scrubbing = false;
    const element = event.currentTarget as HTMLElement;
    if (element.hasPointerCapture(event.pointerId)) element.releasePointerCapture(event.pointerId);
  }

  function setZoom(nextZoom: number) {
    const center = viewStartMs + viewSpanMs / 2;
    const clampedZoom = Math.max(minimumZoom, Math.min(24, nextZoom));
    const nextViewSpan = Math.min(maximumSpanMs, Math.max(5_000, 120_000 / clampedZoom));
    zoom = clampedZoom;
    followPlayhead = false;
    scrollStartMs = Math.max(0, Math.min(Math.max(0, maximumSpanMs - nextViewSpan), center - nextViewSpan / 2));
  }

  function panView(ratio: number) {
    followPlayhead = false;
    scrollStartMs = Math.max(0, Math.min(scrollMaximumMs, Math.round(viewStartMs + viewSpanMs * ratio)));
  }

  function setViewStart(value: number) {
    followPlayhead = false;
    scrollStartMs = Math.max(0, Math.min(scrollMaximumMs, value));
  }

  function scrollbarValue(event: PointerEvent) {
    const trackBounds = scrollbar.getBoundingClientRect();
    const thumbWidth = scrollbarThumb.getBoundingClientRect().width;
    const available = Math.max(1, trackBounds.width - thumbWidth);
    const ratio = Math.max(0, Math.min(1, (event.clientX - trackBounds.left - scrollbarGrabOffset) / available));
    return Math.round(ratio * scrollMaximumMs);
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
    else if (event.key === "Home") setViewStart(0);
    else if (event.key === "End") setViewStart(scrollMaximumMs);
    else return;
    event.preventDefault();
  }

  function zoomFromWheel(event: WheelEvent) {
    if (!event.ctrlKey && !event.metaKey) return;
    event.preventDefault();
    setZoom(zoom * (event.deltaY < 0 ? 1.25 : .8));
  }

  function timelineKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowLeft") onSeek(Math.max(0, currentTimeMs - 1000));
    else if (event.key === "ArrowRight") onSeek(Math.min(maximumSpanMs, currentTimeMs + 1000));
    else if (event.key === "PageUp") panView(-.8);
    else if (event.key === "PageDown") panView(.8);
    else if (event.key === "Home") onSeek(0);
    else if (event.key === "End") onSeek(maximumSpanMs);
    else return;
    event.preventDefault();
  }

  function textSegments(item: TimelineEvent) {
    const chars = [...item.text];
    const segments: { text: string; features: TimelineFeature[] }[] = [];
    for (let index = 0; index < chars.length; index += 1) {
      const active = item.highlights.filter((range) => index >= range.start && index < range.end).map((range) => range.feature);
      const key = active.join("|");
      const previous = segments[segments.length - 1];
      if (previous && previous.features.join("|") === key) previous.text += chars[index];
      else segments.push({ text: chars[index], features: active });
    }
    return segments;
  }

  $: filterKey = [...filters].sort().join(",");
  $: if (!editor && desktopRuntime && archivePath && (archivePath !== loadedArchive || filterKey !== loadedFilterKey || loadedLive !== live)) {
    loadedFilterKey = filterKey;
    loadedLive = live;
    void (live ? loadRecentPage() : loadPage(true));
  }
  $: editorWindowKey = `${archivePath}:${Math.floor(viewStartMs / 2_000)}:${Math.round(viewSpanMs)}`;
  $: if (editor && desktopRuntime && archivePath && editorWindowKey !== loadedEditorWindow) void loadEditorWindow();
  $: if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = undefined; }
  $: if (desktopRuntime && live && archivePath) refreshTimer = setInterval(() => void (editor ? loadEditorWindow() : loadRecentPage()), 700);
  onDestroy(() => { if (refreshTimer) clearInterval(refreshTimer); if (seekFrame !== undefined) cancelAnimationFrame(seekFrame); });
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
        <button class="timeline-fit" data-tooltip={t("timeline.fit")} aria-label={t("timeline.fit")} onclick={() => setZoom(120_000 / maximumSpanMs)}><ScanLine size={14} /><span>{t("timeline.fit")}</span></button>
      </div>
    </header>
    <div class="timeline-ruler-row">
      <span class="timeline-ruler-gutter" aria-hidden="true"></span>
      <div class="timeline-ruler" role="slider" tabindex="0" aria-label={t("preview.seekTimeline")} aria-valuemin="0" aria-valuemax={maximumSpanMs} aria-valuenow={timelineTimeMs} onkeydown={timelineKeydown} onpointerdown={beginScrub} onpointermove={moveScrub} onpointerup={endScrub} onpointercancel={cancelScrub}>
        {#each ticks as tick, index}{#if tick.time >= 0 && tick.time <= maximumSpanMs}<span class:last-tick={index === ticks.length - 1} style={`left:${tick.percent}%`}><i></i><small>{rulerTime(tick.time)}</small></span>{/if}{/each}
        <em style={`left:${cursorPercent()}%`}></em>
      </div>
    </div>
    <div class="timeline-lanes" id="timeline-visible-content">
      <div class="timeline-lane">
        <div class="timeline-lane-label"><b>{trackName || t("timeline.selectedTrack")}</b>{#if trackDetail || trackLabel}<small>{trackDetail || trackLabel}</small>{/if}</div>
        <div class="timeline-track" role="slider" tabindex="0" aria-label={t("preview.seekTimeline")} aria-valuemin="0" aria-valuemax={maximumSpanMs} aria-valuenow={timelineTimeMs} onkeydown={timelineKeydown} onpointerdown={beginScrub} onpointermove={moveScrub} onpointerup={endScrub} onpointercancel={cancelScrub}>
          {#each visibleRecords as item (item.index)}
            {@const features = visibleFeatures(item)}
            <button class:ttml={item.kind === "caption"} class:scene={item.kind === "scene"} data-tooltip={`${timestamp(item.beginMs)} · ${item.text || kind(item.kind)}`} aria-label={`${timestamp(item.beginMs)} · ${item.text || kind(item.kind)}`} class:current={timelineTimeMs >= item.beginMs && timelineTimeMs <= item.endMs} style={`${barStyle(item)}--event-color:${eventColor(item)};`} onpointerdown={(event) => event.stopPropagation()} onclick={(event) => { event.stopPropagation(); onSeek(item.beginMs); }}>
              <span class="timeline-event-features">
                {#each features.slice(0, 2) as feature}<span class="timeline-event-feature" style={`--feature-color:${featureMeta[feature].color}`}><img src={featureMeta[feature].icon} alt="" /><span>{t(`feature.${feature}`)}</span></span>{/each}
                {#if features.length > 2}<span class="timeline-feature-overflow">+{features.length - 2}</span>{/if}
              </span>
              <span class="timeline-event-text">{item.text || kind(item.kind)}</span>
            </button>
          {/each}
          <i class="timeline-playhead" style={`left:${cursorPercent()}%`}></i>
        </div>
      </div>
    </div>
    <div class="timeline-scrollbar-row">
      <span class="timeline-scrollbar-gutter" aria-hidden="true"></span>
      <div class="timeline-scrollbar" class:dragging={scrollbarDragging} bind:this={scrollbar} role="scrollbar" tabindex="0" aria-controls="timeline-visible-content" aria-label={t("timeline.visibleRange")} aria-orientation="horizontal" aria-valuemin="0" aria-valuemax={scrollMaximumMs} aria-valuenow={Math.round(viewStartMs)} onkeydown={scrollbarKeydown} onpointerdown={beginScrollbar} onpointermove={moveScrollbar} onpointerup={endScrollbar} onpointercancel={endScrollbar}>
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
    {:else if records.length}<ol>{#each records as item (item.index)}<li><time>{timestamp(item.beginMs)}<small>{timestamp(item.endMs)}</small></time><button class="event-content" onclick={() => onSeek(item.beginMs)}><strong>{kind(item.kind)}{#if item.regionX !== null && item.regionY !== null}<small>{t("timeline.region").replace("{0}", String(item.regionX)).replace("{1}", String(item.regionY))}</small>{/if}</strong><span class="feature-badges">{#each item.features as feature}{#if feature === "color" && item.colors?.length}{#each item.colors as color}<i class="feature-color" data-tooltip={t(`feature.color.${color.role}`)}><span class="color-swatch" style={`background:${color.value}`}></span>{color.value}</i>{/each}{:else}<i class={`feature-${feature}`}>{t(`feature.${feature}`)}</i>{/if}{/each}</span><small>{#each textSegments(item) as segment}{#if segment.features.length}<mark class={segment.features.map((feature) => `feature-${feature}`).join(" ")} data-tooltip={segment.features.map((feature) => t(`feature.${feature}`)).join(" · ")}>{segment.text}</mark>{:else}{segment.text}{/if}{/each}</small></button></li>{/each}</ol>{#if !exhausted}<button class="load-more" onclick={() => loadPage(false)} disabled={loading}><RefreshCw size={15} /> {loading ? t("workspace.loading") : t("workspace.loadMore")}</button>{/if}
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
    --timeline-gutter: 92px;
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
  .timeline-tools .timeline-fit {
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
  .timeline-ruler-row { display: grid; grid-template-columns: var(--timeline-gutter) minmax(0, 1fr); height: 30px; border-bottom: 1px solid var(--rw-border-subtle); background: var(--rw-surface-muted); }
  .timeline-ruler-gutter { border-right: 1px solid var(--rw-border-subtle); }
  .timeline-ruler { position: relative; min-width: 0; height: 30px; cursor: grab; touch-action: none; }
  .timeline-ruler > span { position: absolute; top: 0; bottom: 0; }
  .timeline-ruler > span > i { display: block; width: 1px; height: 8px; background: color-mix(in srgb, var(--rw-text) 24%, transparent); }
  .timeline-ruler > span > small { position: absolute; top: 11px; left: 4px; color: var(--rw-muted); font: 8px/11px var(--rw-font-mono); white-space: nowrap; }
  .timeline-ruler > span.last-tick > small { right: 4px; left: auto; }
  .timeline-ruler > em, .timeline-playhead { position: absolute; z-index: 6; top: 0; bottom: 0; width: 2px; margin-left: -1px; background: var(--rw-accent); pointer-events: none; }
  .timeline-ruler > em::before { position: absolute; top: 0; left: -4px; border-top: 6px solid var(--rw-accent); border-right: 5px solid transparent; border-left: 5px solid transparent; content: ""; }
  .timeline-lanes { min-width: 0; }
  .timeline-lane { display: grid; grid-template-columns: var(--timeline-gutter) minmax(0, 1fr); min-height: 72px; background: var(--rw-content); }
  .timeline-lane-label { display: flex; flex-direction: column; justify-content: center; min-width: 0; padding: 7px 8px; border-right: 1px solid var(--rw-border-subtle); background: color-mix(in srgb, var(--rw-surface-muted) 70%, var(--rw-content)); }
  .timeline-lane-label b { overflow: hidden; color: var(--rw-text-secondary); font-size: 9px; line-height: 12px; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }
  .timeline-lane-label small { display: -webkit-box; overflow: hidden; margin-top: 2px; color: var(--rw-muted); font-size: 8px; line-height: 11px; -webkit-box-orient: vertical; -webkit-line-clamp: 3; line-clamp: 3; }
  .timeline-track {
    position: relative;
    min-width: 0;
    height: 72px;
    overflow: hidden;
    background: repeating-linear-gradient(90deg, transparent 0, transparent calc(25% - 1px), color-mix(in srgb, var(--rw-border-subtle) 76%, transparent) calc(25% - 1px), color-mix(in srgb, var(--rw-border-subtle) 76%, transparent) 25%);
    cursor: grab;
    touch-action: none;
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
  .timeline-track button.current { border-color: color-mix(in srgb, var(--rw-accent) 82%, var(--rw-border)); box-shadow: 0 0 0 1px color-mix(in srgb, var(--rw-accent) 42%, transparent), 0 2px 5px rgba(0, 68, 150, .12); }
  .timeline-event-features { display: flex; align-items: center; height: 15px; overflow: hidden; gap: 4px; white-space: nowrap; }
  .timeline-event-feature { display: inline-flex; align-items: center; min-width: 0; gap: 2px; color: var(--feature-color); font-size: 8px; line-height: 12px; font-weight: 650; }
  .timeline-event-feature img { display: block; width: 13px; height: 13px; flex: 0 0 13px; }
  .timeline-feature-overflow { flex: 0 0 auto; color: var(--rw-muted); font: 8px/12px var(--rw-font-mono); }
  .timeline-event-text { display: block; overflow: hidden; margin-top: 4px; color: var(--rw-text); font-size: 10px; line-height: 13px; font-weight: 560; text-overflow: ellipsis; white-space: nowrap; }
  .timeline-event-features:empty + .timeline-event-text { margin-top: 12px; }
  .timeline-scrollbar-row { display: grid; grid-template-columns: var(--timeline-gutter) minmax(0, 1fr); height: 20px; border-top: 1px solid var(--rw-border-subtle); border-bottom: 1px solid var(--rw-border-subtle); background: var(--rw-surface-muted); }
  .timeline-scrollbar-gutter { border-right: 1px solid var(--rw-border-subtle); }
  .timeline-scrollbar { position: relative; min-width: 0; height: 20px; outline-offset: -2px; cursor: default; touch-action: none; }
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
    .caption-timeline { --timeline-gutter: 82px; }
    .timeline-tools .zoom-label { display: none; }
    .timeline-tools :global(.timeline-zoom-slider) { width: 64px; flex-basis: 64px; }
    .timeline-tools .timeline-fit span { display: none; }
    .timeline-tools .timeline-fit { width: 28px !important; padding: 0; border-radius: 50% !important; }
    .timeline-now { grid-template-columns: 80px minmax(0, 1fr) auto; }
  }
  .feature-badges i.feature-color{display:inline-flex;align-items:center;gap:4px;background:var(--rw-surface-raised)!important}.color-swatch{width:9px;height:9px;border:1px solid color-mix(in srgb,var(--rw-text) 32%,transparent);border-radius:2px;box-shadow:inset 0 0 0 1px #ffffff26}.event-content mark{background:transparent!important}.event-content mark.feature-accessibility{background:#26795c4d!important}.event-content mark.feature-ruby{border-top:1px solid #56a8ff!important;border-bottom:0!important}.event-content mark.feature-drcs{box-shadow:inset 0 -2px #d46b9a}.event-content mark.feature-gaiji{border-bottom:1px dashed #9d82e8}.event-content mark.feature-color{background:transparent!important}.timeline-ruler,.timeline-track{cursor:grab}.caption-timeline.dragging .timeline-ruler,.caption-timeline.dragging .timeline-track{cursor:grabbing}
  .event-filters>span{display:flex;align-items:center;padding:2px 7px;border:1px solid var(--rw-border);border-radius:6px;color:var(--rw-muted)}.event-filters>span.active{border-color:color-mix(in srgb,var(--rw-accent) 58%,var(--rw-border));color:var(--rw-text);background:color-mix(in srgb,var(--rw-accent) 9%,transparent)}.event-filters :global(.mac-checkbox){font-size:11px}
  @container (max-width: 84px){.timeline-event-feature span{display:none}.timeline-event-features{gap:2px}.timeline-event-text{margin-top:3px!important;font-size:9px!important}}
  @container (max-width: 36px){.timeline-track button{padding-inline:3px}.timeline-event-features{justify-content:center}.timeline-event-feature:not(:first-child),.timeline-event-text{display:none!important}}
</style>

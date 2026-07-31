<script lang="ts">
  import { ChevronLeft, ChevronRight, Clock3, FileText, RefreshCw, ScanLine, ZoomIn, ZoomOut } from "@lucide/svelte";
  import { onDestroy } from "svelte";
  import type { TimelineEvent, TimelineFeature } from "../../backend";
  import { t } from "../../i18n";
  import { getFilteredTimelineWindow, getRecentTimelineWindow, getTimelineTimeWindow } from "./timeline-controller";

  export let archivePath = "";
  export let desktopRuntime = false;
  export let live = false;
  export let editor = false;
  export let currentTimeMs = 0;
  export let durationMs = 0;
  export let trackLabel = "";
  export let expectedCount = 0;
  export let onSeek: (milliseconds: number) => void = () => {};
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
  let zoom = 1;
  let scrubbing = false;
  let dragStartX = 0;
  let dragStartTimeMs = 0;
  let dragWindowStartMs = 0;
  let dragWindowSpanMs = 0;
  let dragWidth = 1;
  let dragMoved = false;
  let dragTargetTimeMs = 0;
  let seekFrame: number | undefined;

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
  $: viewSpanMs = Math.min(maximumSpanMs, Math.max(5_000, 120_000 / zoom));
  $: viewStartMs = timelineTimeMs - viewSpanMs / 2;
  $: ticks = Array.from({ length: 7 }, (_, index) => ({ percent: index * 100 / 6, time: viewStartMs + viewSpanMs * index / 6 }));
  const cursorPercent = () => 50;
  const barStyle = (item: TimelineEvent) => {
    const left = Math.max(0, Math.min(100, ((item.beginMs - viewStartMs) / viewSpanMs) * 100));
    const right = Math.max(left + 0.35, Math.min(100, ((item.endMs - viewStartMs) / viewSpanMs) * 100));
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
    dragStartTimeMs = currentTimeMs;
    dragWindowStartMs = viewStartMs;
    dragWindowSpanMs = viewSpanMs;
    dragWidth = Math.max(1, (event.currentTarget as HTMLElement).getBoundingClientRect().width);
    dragMoved = false;
    dragTargetTimeMs = currentTimeMs;
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
    const next = dragStartTimeMs - delta / dragWidth * dragWindowSpanMs;
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
    zoom = Math.max(.25, Math.min(24, nextZoom));
  }

  function panView(ratio: number) {
    onSeek(Math.max(0, Math.min(maximumSpanMs, Math.round(currentTimeMs + viewSpanMs * ratio))));
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
    <header>
      <b>{t("timeline.editorTitle")}</b>
      <div class="timeline-tools"><span>{timestamp(timelineTimeMs)}</span><button title={t("timeline.previousWindow")} onclick={() => panView(-.8)}><ChevronLeft size={15} /></button><button title={t("timeline.zoomOut")} onclick={() => setZoom(zoom / 1.5)}><ZoomOut size={15} /></button><input aria-label={t("timeline.zoom")} type="range" min="0.25" max="24" step="0.25" value={zoom} oninput={(event) => setZoom(Number(event.currentTarget.value))} /><button title={t("timeline.zoomIn")} onclick={() => setZoom(zoom * 1.5)}><ZoomIn size={15} /></button><button title={t("timeline.nextWindow")} onclick={() => panView(.8)}><ChevronRight size={15} /></button><button title={t("timeline.fit")} onclick={() => setZoom(Math.max(.25, 120_000 / maximumSpanMs))}><ScanLine size={15} /></button></div>
    </header>
    <div class="timeline-ruler" role="slider" tabindex="0" aria-label={t("preview.seekTimeline")} aria-valuemin="0" aria-valuemax={maximumSpanMs} aria-valuenow={timelineTimeMs} onkeydown={timelineKeydown} onpointerdown={beginScrub} onpointermove={moveScrub} onpointerup={endScrub} onpointercancel={cancelScrub}>
      {#each ticks as tick}{#if tick.time >= 0 && tick.time <= maximumSpanMs}<span style={`left:${tick.percent}%`}><i></i><small>{rulerTime(tick.time)}</small></span>{/if}{/each}
      <em style={`left:${cursorPercent()}%`}></em>
    </div>
    <div class="timeline-lanes">
      <div class="timeline-lane"><b>{trackLabel || t("timeline.selectedTrack")}</b><div class="timeline-track" role="slider" tabindex="0" aria-label={t("preview.seekTimeline")} aria-valuemin="0" aria-valuemax={maximumSpanMs} aria-valuenow={timelineTimeMs} onkeydown={timelineKeydown} onpointerdown={beginScrub} onpointermove={moveScrub} onpointerup={endScrub} onpointercancel={cancelScrub}>{#each records.filter((item) => item.endMs >= viewStartMs && item.beginMs <= viewStartMs + viewSpanMs && (!item.trackId || !trackLabel || item.trackId === trackLabel)) as item}<button class:ttml={item.kind === "caption"} class:scene={item.kind === "scene"} title={item.text || kind(item.kind)} class:current={timelineTimeMs >= item.beginMs && timelineTimeMs <= item.endMs} style={barStyle(item)} onpointerdown={(event) => event.stopPropagation()} onclick={(event) => { event.stopPropagation(); onSeek(item.beginMs); }}>{item.text || kind(item.kind)}</button>{/each}<i style={`left:${cursorPercent()}%`}></i></div></div>
    </div>
    {#if !records.length}<p>{live ? t("workspace.eventsEmpty") : t("timeline.empty")}</p>{/if}
  </section>
{:else}
  <section class="event-list timeline-list">
    <header><b>{t("workspace.eventsTitle")}</b><span>{expectedCount > records.length ? t("timeline.loadedCount").replace("{0}", String(records.length)).replace("{1}", expectedCount.toLocaleString()) : t("workspace.eventsCount").replace("{0}", String(records.length || expectedCount))}</span></header>
    <div class="event-filters" aria-label={t("timeline.filters")}>
      {#each featureOptions as feature}<label class:active={filters.has(feature)}><input type="checkbox" checked={filters.has(feature)} onchange={() => toggleFilter(feature)} />{t(`feature.${feature}`)}</label>{/each}
    </div>
    {#if !archivePath}<div class="event-empty"><FileText size={30} /><p>{live ? t("workspace.eventsEmpty") : t("timeline.archiveRequired")}</p></div>
    {:else if records.length}<ol>{#each records as item}<li><time>{timestamp(item.beginMs)}<small>{timestamp(item.endMs)}</small></time><button class="event-content" onclick={() => onSeek(item.beginMs)}><strong>{kind(item.kind)}{#if item.regionX !== null && item.regionY !== null}<small>{t("timeline.region").replace("{0}", String(item.regionX)).replace("{1}", String(item.regionY))}</small>{/if}</strong><span class="feature-badges">{#each item.features as feature}{#if feature === "color" && item.colors?.length}{#each item.colors as color}<i class="feature-color" title={t(`feature.color.${color.role}`)}><span class="color-swatch" style={`background:${color.value}`}></span>{color.value}</i>{/each}{:else}<i class={`feature-${feature}`}>{t(`feature.${feature}`)}</i>{/if}{/each}</span><small>{#each textSegments(item) as segment}{#if segment.features.length}<mark class={segment.features.map((feature) => `feature-${feature}`).join(" ")} title={segment.features.map((feature) => t(`feature.${feature}`)).join(" · ")}>{segment.text}</mark>{:else}{segment.text}{/if}{/each}</small></button></li>{/each}</ol>{#if !exhausted}<button class="load-more" onclick={() => loadPage(false)} disabled={loading}><RefreshCw size={15} /> {loading ? t("workspace.loading") : t("workspace.loadMore")}</button>{/if}
    {:else}<div class="event-empty"><Clock3 size={30} /><p>{loading ? t("workspace.loading") : live ? t("workspace.eventsEmpty") : t("timeline.empty")}</p></div>{/if}
  </section>
{/if}

<style>
  .event-list{min-height:360px;color:var(--rw-text)}header{display:flex;justify-content:space-between;gap:12px;padding:13px 15px;border-bottom:1px solid var(--rw-border)}header span{color:var(--rw-muted);font-size:12px}.event-filters{display:flex;flex-wrap:wrap;gap:6px;padding:9px 15px;border-bottom:1px solid var(--rw-border-subtle)}.event-filters label{display:flex;align-items:center;gap:5px;padding:5px 7px;border:1px solid var(--rw-border);border-radius:4px;color:var(--rw-muted);font-size:11px;cursor:pointer}.event-filters label.active{border-color:var(--rw-accent);color:var(--rw-text);background:color-mix(in srgb,var(--rw-accent) 10%,transparent)}.event-filters input{accent-color:var(--rw-accent)}ol{margin:0;padding:0;list-style:none}li{display:grid;grid-template-columns:144px minmax(0,1fr);gap:18px;padding:13px 15px;border-bottom:1px solid var(--rw-border-subtle)}time{color:var(--rw-accent);font:12px/1.4 "Cascadia Mono",monospace}strong,small{display:block}strong{color:var(--rw-text);font:12px/1.4 "Cascadia Mono",monospace}time small,strong small{margin-top:4px;color:var(--rw-muted);font:11px "Cascadia Mono",monospace}.event-content>small{margin-top:7px;color:var(--rw-text-secondary);font-size:13px;line-height:1.6;white-space:pre-wrap}.event-content{min-width:0;padding:0;color:inherit;background:transparent;text-align:left}.feature-badges{display:flex;flex-wrap:wrap;gap:4px;margin-top:6px}.feature-badges i{padding:2px 5px;border-radius:3px;background:var(--rw-surface-raised);color:var(--rw-muted);font-size:9px;font-style:normal}mark{padding:1px 0;color:inherit;border-radius:2px;background:#6e5b163f}.feature-ruby{border-bottom:1px solid #56a8ff!important}.feature-drcs{background:#a74c6f45!important}.feature-gaiji{background:#7358b94d!important}.feature-accessibility{background:#26795c4d!important}.feature-color{background:#8f6b244d!important}.event-empty{display:grid;place-items:center;gap:10px;min-height:220px;padding:20px;color:var(--rw-muted);text-align:center}.load-more{display:flex;align-items:center;gap:7px;margin:14px auto;padding:8px 12px;color:var(--rw-text);border:1px solid var(--rw-border);border-radius:5px;background:var(--rw-surface-raised)}.caption-timeline{margin:14px 0;overflow:hidden;border:1px solid var(--rw-border);border-radius:7px;background:var(--rw-surface-muted);color:var(--rw-text)}.caption-timeline header{align-items:center;padding:8px 11px}.timeline-tools{display:flex;align-items:center;gap:5px}.timeline-tools>span{min-width:88px;color:var(--rw-text);font:11px "Cascadia Mono",monospace}.timeline-tools button{display:grid;place-items:center;width:26px;height:25px;color:var(--rw-text-secondary);border:1px solid var(--rw-border);border-radius:4px;background:var(--rw-surface-raised)}.timeline-tools input{width:82px;accent-color:var(--rw-accent)}.timeline-ruler{position:relative;height:31px;margin-left:108px;border-bottom:1px solid var(--rw-border);cursor:ew-resize;touch-action:none}.timeline-ruler>span{position:absolute;top:0;bottom:0}.timeline-ruler>span>i{display:block;width:1px;height:9px;background:var(--rw-border)}.timeline-ruler>span>small{position:absolute;top:11px;left:4px;color:var(--rw-muted);font:9px "Cascadia Mono",monospace;white-space:nowrap}.timeline-ruler>em,.timeline-track>i{position:absolute;top:0;bottom:0;width:1px;background:#4d9cff;z-index:5}.timeline-ruler>em:before{content:"";position:absolute;top:0;left:-4px;border-left:4px solid transparent;border-right:4px solid transparent;border-top:6px solid #4d9cff}.timeline-lane{display:grid;grid-template-columns:108px minmax(0,1fr);min-height:48px}.timeline-lane>b{display:flex;align-items:center;padding:0 10px;color:var(--rw-text-secondary);border-right:1px solid var(--rw-border);font-size:11px}.timeline-track{position:relative;overflow:hidden;background:repeating-linear-gradient(90deg,transparent 0,transparent calc(16.666% - 1px),color-mix(in srgb,var(--rw-border) 60%,transparent) calc(16.666% - 1px),color-mix(in srgb,var(--rw-border) 60%,transparent) 16.666%);cursor:ew-resize;touch-action:none}.timeline-track button{position:absolute;top:7px;bottom:7px;overflow:hidden;padding:0 7px;color:#dce9fb;border:1px solid #3179dd;border-radius:3px;background:#194b86;text-overflow:ellipsis;white-space:nowrap;font-size:11px;text-align:left;z-index:2}.timeline-track button.ttml{border-color:#31815e;background:#164c35}.timeline-track button.scene{border-color:#9d7621;background:#5a4212}.timeline-track button.current{box-shadow:0 0 0 1px #a9d0ff}.caption-timeline>p{margin:0;padding:15px;color:var(--rw-muted);font-size:12px;text-align:center}@media(max-width:900px){li{grid-template-columns:1fr;gap:6px}.timeline-tools input{width:52px}}
  .feature-badges i.feature-color{display:inline-flex;align-items:center;gap:4px;background:var(--rw-surface-raised)!important}.color-swatch{width:9px;height:9px;border:1px solid color-mix(in srgb,var(--rw-text) 32%,transparent);border-radius:2px;box-shadow:inset 0 0 0 1px #ffffff26}.event-content mark{background:transparent!important}.event-content mark.feature-accessibility{background:#26795c4d!important}.event-content mark.feature-ruby{border-top:1px solid #56a8ff!important;border-bottom:0!important}.event-content mark.feature-drcs{box-shadow:inset 0 -2px #d46b9a}.event-content mark.feature-gaiji{border-bottom:1px dashed #9d82e8}.event-content mark.feature-color{background:transparent!important}.timeline-ruler,.timeline-track{cursor:grab}.caption-timeline.dragging .timeline-ruler,.caption-timeline.dragging .timeline-track{cursor:grabbing}
</style>

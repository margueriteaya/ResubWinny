<script lang="ts">
  import {
    BookOpen, ChevronRight, File, FileCode2, FileText, FolderOpen,
    Layers3, Lightbulb, ListTodo, X,
  } from "@lucide/svelte";
  import { t } from "../../i18n";

  type Format = { name: string; description: string; color: string; icon: any };
  type HistoryItem = {
    name: string; path: string; size: number; container: string; status: "Completed" | "Warning" | "In Progress";
    time: string; warnings: number; captions?: number; jobId?: string;
  };

  export let formats: Format[] = [];
  export let history: HistoryItem[] = [];
  export let isInspecting = false;
  export let onChooseSource: () => void = () => {};
  export let onOpenTask: () => void = () => {};
  export let onOpenHistory: (item: HistoryItem) => void = () => {};
  export let onNavigate: (target: "batch" | "drcs") => void = () => {};
  export let onChooseFormat: (name: string) => void = () => {};

  let tipVisible = true;
  const bytes = (value: number) => value ? `${(value / 1024 ** 3).toFixed(value > 100 * 1024 ** 3 ? 1 : 2)} GB` : "-";
  const historyStatus = (status: HistoryItem["status"]) =>
    status === "In Progress"
      ? t("batch.status.running")
      : status === "Warning"
        ? t("batch.status.warning")
        : t("batch.status.completed");
</script>

<header class="home-header">
  <div><h1>{t("home.title")}</h1><p>{t("home.subtitle")}</p></div>
  <div class="header-actions"><button class="outline" onclick={onOpenTask}><FolderOpen size={19} /> {t("home.openTask")}</button></div>
</header>
<div class="home-grid">
  <section class="drop-card card">
    <button class="drop-zone" onclick={onChooseSource} disabled={isInspecting} aria-label={t("home.selectRecording")}>
      <span class="file-symbol"><File size={52} strokeWidth={1.6} /><b>TS</b></span>
      <strong>{isInspecting ? t("home.inspecting") : t("home.drop")}</strong>
      <span>{t("home.extract")}</span><small>{t("home.supportedInputs")}</small>
      <div class="or"><i></i><span>{t("home.or")}</span><i></i></div>
      <span class="primary-button"><FolderOpen size={20} /> {t("home.select")}</span>
      <small>{t("home.dragHint")}</small>
    </button>
  </section>
  <section class="formats card">
    <h2><FileCode2 size={24} /> {t("home.formats")}</h2>
    {#each formats as item}
      <button class="format-row" onclick={() => onChooseFormat(item.name)}>
        <span class={`format-icon ${item.color}`}><svelte:component this={item.icon} size={21} /></span>
        <span><b>{item.name}</b><small>{item.description}</small></span>
      </button>
    {/each}
    <button class="inline-link" onclick={onOpenTask}>{t("home.viewFormats")} <ChevronRight size={18} /></button>
  </section>
  <section class="recent card">
    <div class="card-title"><h2>{t("home.recent")}</h2>{#if history.length}<button class="inline-link" onclick={onOpenTask}>{t("home.viewAll")}</button>{/if}</div>
    {#if history.length}
      <div class="task-list">
        {#each history.slice(0, 5) as item}
          <button class="recent-row" onclick={() => onOpenHistory(item)} disabled={isInspecting}>
            <span class="record-icon">{item.container === "TLV" ? "TLV" : "TS"}</span>
            <span class="task-name"><b>{item.name}</b><small>{bytes(item.size)} · {item.captions ?? 0} {t("home.captions")} · {item.warnings} {t("home.warnings")}</small></span>
            <span class:warning={item.status === "Warning"} class="task-status">{historyStatus(item.status)}</span><small class="task-time">{item.time}</small><ChevronRight size={19} />
          </button>
        {/each}
      </div>
      <p class="showing">{t("home.showingTasks").replace("{0}", String(Math.min(history.length, 5))).replace("{1}", String(history.length))}</p>
    {:else}
      <div class="empty-history"><ListTodo size={30} /><p>{t("home.noCompleted")}</p><button class="inline-link" onclick={onChooseSource}>{t("home.startNew")}</button></div>
    {/if}
  </section>
  <section class="right-column">
    <section class="quick-start card">
      <h2>{t("home.quickStart")}</h2>
      <button onclick={onChooseSource}><span class="quick-icon blue"><FileText size={28} /></span><span><b>{t("home.newExtraction")}</b><small>{t("home.extractionDescription")}</small></span><ChevronRight size={22} /></button>
      <button onclick={() => onNavigate("batch")}><span class="quick-icon purple"><Layers3 size={28} /></span><span><b>{t("home.batch")}</b><small>{t("home.batchDescription")}</small></span><ChevronRight size={22} /></button>
      <button onclick={() => onNavigate("drcs")}><span class="quick-icon green"><BookOpen size={28} /></span><span><b>{t("home.openDrcs")}</b><small>{t("home.drcsDescription")}</small></span><ChevronRight size={22} /></button>
    </section>
    {#if tipVisible}<section class="tip card"><button class="close-tip" aria-label={t("home.closeTip")} onclick={() => tipVisible = false}><X size={17} /></button><Lightbulb size={29} /><div><b>{t("home.tipTitle")}</b><p>{t("home.tipBody")}</p></div></section>{/if}
  </section>
</div>

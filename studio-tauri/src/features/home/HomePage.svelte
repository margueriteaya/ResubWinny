<script lang="ts">
  import {
    Captions,
    ChevronRight,
    FileOutput,
    FileUp,
    FileVideo2,
    FolderOpen,
    FolderPlus,
    Layers3,
    Lightbulb,
    ScanText,
    X,
  } from "@lucide/svelte";
  import { t } from "../../i18n";
  import drcsIcon from "../../assets/arib/drcs.svg";

  type Format = { name: string; description: string; color: string; icon: any };
  type HistoryItem = {
    name: string;
    path: string;
    size: number;
    container: string;
    status: "Completed" | "Warning" | "In Progress";
    time: string;
    warnings: number;
    captions?: number;
    jobId?: string;
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
  const bytes = (value: number) => value
    ? `${(value / 1024 ** 3).toFixed(value > 100 * 1024 ** 3 ? 1 : 2)} GB`
    : "-";
  const historyStatus = (status: HistoryItem["status"]) =>
    status === "In Progress"
      ? t("batch.status.running")
      : status === "Warning"
        ? t("batch.status.warning")
        : t("batch.status.completed");
</script>

<div class="approved-home-page">
  <header class="approved-page-title">
    <div><h1>{t("home.title")}</h1><p>{t("home.subtitle")}</p></div>
    <button class="approved-secondary" onclick={onOpenTask}><FolderOpen size={15} />{t("home.openTask")}</button>
  </header>

  <div class="approved-home-grid">
    <div class="approved-home-primary">
      <button class="approved-dropzone" onclick={onChooseSource} disabled={isInspecting} aria-label={t("home.selectRecording")}>
        <FileUp size={28} />
        <h2>{isInspecting ? t("home.inspecting") : t("home.drop")}</h2>
        <p>{t("home.supportedInputs")}</p>
        <span class="approved-primary"><FolderPlus size={15} />{t("home.select")}</span>
      </button>

      <section class="approved-section recent-section">
        <header class="approved-section-head"><h2>{t("home.recent")}</h2></header>
        {#if history.length}
          <div class="recent-table-wrap">
            <table class="recent-table">
              <thead><tr><th>{t("batch.file")}</th><th>{t("batch.status")}</th><th>{t("home.captions")}</th><th>{t("home.warnings")}</th><th>{t("home.lastOpened")}</th></tr></thead>
              <tbody>
                {#each history.slice(0, 5) as item (item.path)}
                  <tr role="button" tabindex={isInspecting ? -1 : 0} aria-disabled={isInspecting} onclick={() => { if (!isInspecting) onOpenHistory(item); }} onkeydown={(event) => { if (!isInspecting && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); onOpenHistory(item); } }}>
                    <td><div class="recent-file"><span class="recent-file-icon"><FileVideo2 size={16} /><small>{item.container}</small></span><span><b>{item.name}</b><small>{bytes(item.size)} · {item.container}</small></span></div></td>
                    <td><span class:warning={item.status === "Warning"} class:progress={item.status === "In Progress"} class="state-dot"></span>{historyStatus(item.status)}</td>
                    <td>{item.captions ?? 0}</td><td>{item.warnings}</td><td>{item.time}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <div class="empty-recent"><FileVideo2 size={25} /><p>{t("home.noCompleted")}</p><button onclick={onChooseSource}>{t("home.startNew")}</button></div>
        {/if}
      </section>
    </div>

    <aside class="approved-home-aside">
      <section class="approved-section">
        <header class="approved-section-head"><h2>{t("home.quickStart")}</h2></header>
        <div class="quick-grid">
          <button onclick={onChooseSource}><FileOutput size={17} /><b>{t("home.newExtraction")}</b><small>{t("home.extractionDescription")}</small></button>
          <button onclick={() => onNavigate("batch")}><Layers3 size={17} /><b>{t("home.batch")}</b><small>{t("home.batchDescription")}</small></button>
          <button onclick={() => onNavigate("drcs")}><ScanText size={17} /><b>DRCS</b><small>{t("home.drcsDescription")}</small></button>
        </div>
      </section>

      <section class="approved-section format-section">
        <header class="approved-section-head"><h2>{t("home.formats")}</h2></header>
        <div class="format-list">
          {#each formats as item}
            <button onclick={() => onChooseFormat(item.name)} data-tooltip={item.description}><span class={`format-symbol ${item.color}`}><svelte:component this={item.icon} size={15} /></span><span><b>{item.name}</b><small>{item.description}</small></span><ChevronRight size={13} /></button>
          {/each}
        </div>
      </section>

      <section class="approved-section decoded-section">
        <header class="approved-section-head"><h2>{t("home.decodedTypes")}</h2></header>
        <div class="decoded-types"><span><ScanText size={15} />{t("home.decodedB24")}</span><span><Captions size={15} />{t("home.decodedTtml")}</span><span><img src={drcsIcon} alt="" />{t("home.decodedDrcs")}</span></div>
      </section>

      {#if tipVisible}
        <section class="approved-tip"><Lightbulb size={17} /><p>{t("home.tipBody")}</p><button aria-label={t("home.closeTip")} onclick={() => tipVisible = false}><X size={14} /></button></section>
      {/if}
    </aside>
  </div>
</div>

<style>
  .approved-home-page{width:100%;min-width:0;min-height:100%;color:var(--rw-text)}
  .approved-page-title{display:flex;align-items:center;margin-bottom:18px}.approved-page-title h1{margin:0;font-size:20px;line-height:25px;font-weight:680}.approved-page-title p{margin:3px 0 0;color:var(--rw-text-secondary);font-size:11px;line-height:15px}.approved-page-title>.approved-secondary{margin-left:auto}
  .approved-secondary{display:flex;align-items:center;justify-content:center;gap:6px;height:32px;padding:0 10px;border:.5px solid var(--rw-glass-border);border-radius:8px;color:var(--rw-text);background:transparent;box-shadow:var(--rw-control-shadow);backdrop-filter:blur(16px) saturate(1.18);-webkit-backdrop-filter:blur(16px) saturate(1.18);font-size:11px;font-weight:550}
  .approved-home-grid{display:grid;width:100%;max-width:100%;grid-template-columns:minmax(0,1.25fr) minmax(330px,.75fr);gap:18px}.approved-home-primary,.approved-home-aside{min-width:0}
  .approved-dropzone{display:grid;place-items:center;width:100%;min-height:220px;padding:22px;border:1px dashed color-mix(in srgb,var(--rw-accent) 48%,var(--rw-border));border-radius:8px;color:var(--rw-text);background:color-mix(in srgb,var(--rw-accent) 3%,var(--rw-surface-muted));text-align:center}.approved-dropzone :global(> svg){color:var(--rw-accent)}.approved-dropzone h2{margin:10px 0 3px;font-size:16px;line-height:20px}.approved-dropzone p{margin:0;color:var(--rw-text-secondary);font-size:11px}.approved-primary{display:flex;align-items:center;justify-content:center;gap:6px;height:32px;margin-top:10px;padding:0 14px;border:.5px solid color-mix(in srgb,var(--rw-accent) 76%,var(--rw-glass-border));border-radius:7px;color:#fff;background:color-mix(in srgb,var(--rw-accent) 76%,var(--rw-glass-control));font-size:11px;font-weight:650;box-shadow:0 .5px 1px rgba(0,0,0,.18),inset 0 .5px rgba(255,255,255,.2)}
  .approved-section{margin-top:18px}.approved-section:first-child{margin-top:0}.approved-section-head{display:flex;align-items:center;height:34px;border-bottom:1px solid var(--rw-border)}.approved-section-head h2{margin:0;font-size:13px;line-height:17px;font-weight:650}
  .recent-table-wrap{overflow:auto}.recent-table{width:100%;border-collapse:collapse;table-layout:fixed}.recent-table th{height:29px;padding:0 9px;color:var(--rw-muted);background:var(--rw-surface-muted);border-bottom:1px solid var(--rw-border);font-size:9px;font-weight:650;text-align:left}.recent-table th:first-child{width:44%}.recent-table th:nth-child(2){width:16%}.recent-table th:nth-child(3),.recent-table th:nth-child(4){width:10%}.recent-table th:last-child{width:20%}.recent-table td{height:56px;padding:0 9px;border-bottom:1px solid var(--rw-border);font-size:10px;white-space:nowrap}.recent-table tr[role="button"]{cursor:pointer}.recent-table tr[role="button"]:hover td,.recent-table tr[role="button"]:focus-visible td{background:color-mix(in srgb,var(--rw-accent) 7%,var(--rw-content))}.recent-table tr[aria-disabled="true"]{cursor:default;opacity:.65}.recent-file{display:flex;align-items:center;gap:8px;width:100%;padding:0;color:var(--rw-text);background:transparent;text-align:left}.recent-file>span:last-child{min-width:0}.recent-file b,.recent-file small{display:block;overflow:hidden;text-overflow:ellipsis}.recent-file b{font-size:10px}.recent-file small{margin-top:2px;color:var(--rw-muted);font-size:8px}.recent-file-icon{position:relative;display:grid;place-items:center;width:28px;height:32px;flex:0 0 28px;border:1px solid var(--rw-border);border-radius:6px;color:var(--rw-accent);background:var(--rw-content)}.recent-file-icon small{position:absolute;right:-4px;bottom:-3px;padding:1px 2px;border:1px solid var(--rw-content);border-radius:3px;background:var(--rw-surface-muted);font-size:6px;line-height:8px;font-weight:700}.state-dot{display:inline-block;width:7px;height:7px;margin-right:5px;border-radius:50%;background:#30b85a}.state-dot.warning{background:#e59a19}.state-dot.progress{background:var(--rw-accent)}.empty-recent{display:grid;place-items:center;min-height:154px;color:var(--rw-muted);text-align:center}.empty-recent p{margin:8px 0 7px;font-size:12px}.empty-recent button{padding:4px 7px;color:var(--rw-accent);background:transparent;font-size:10px}
  .quick-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:7px;padding-top:8px}.quick-grid button{display:block;min-width:0;min-height:72px;padding:10px;border:1px solid var(--rw-border);border-radius:7px;color:var(--rw-text);background:var(--rw-surface-muted);text-align:left}.quick-grid button:hover{background:color-mix(in srgb,var(--rw-accent) 6%,var(--rw-surface-muted))}.quick-grid button :global(svg){color:var(--rw-accent)}.quick-grid b,.quick-grid small{display:block}.quick-grid b{margin-top:6px;font-size:11px;line-height:14px}.quick-grid small{margin-top:2px;overflow:hidden;color:var(--rw-text-secondary);font-size:9px;line-height:12px}
  .format-list{display:grid;gap:1px;padding-top:4px}.format-list button{display:grid;grid-template-columns:26px minmax(0,1fr) 13px;align-items:center;gap:7px;width:100%;min-height:35px;padding:3px 5px;border-radius:6px;color:var(--rw-text);background:transparent;text-align:left}.format-list button:hover{background:color-mix(in srgb,var(--rw-accent) 7%,transparent)}.format-list button>span:nth-child(2){min-width:0}.format-list b,.format-list small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.format-list b{font-size:10px;line-height:13px}.format-list small{margin-top:1px;color:var(--rw-muted);font-size:8px;line-height:10px}.format-list button>:global(svg:last-child){color:var(--rw-muted)}.format-symbol{display:grid;place-items:center;width:26px;height:26px;border-radius:6px}.format-symbol.purple{color:#7b4db5;background:#7b4db512}.format-symbol.green{color:#168247;background:#16824712}.format-symbol.blue{color:#1766b3;background:#1766b312}.format-symbol.orange{color:#b86400;background:#b8640012}
  .decoded-types{display:grid;gap:2px;padding-top:5px}.decoded-types>span{display:flex;align-items:center;gap:7px;min-width:0;height:27px;color:var(--rw-text-secondary);font-size:9px}.decoded-types :global(svg),.decoded-types img{width:15px;height:15px;flex:0 0 15px;color:var(--rw-accent)}
  .approved-tip{display:flex;align-items:flex-start;gap:9px;margin-top:14px;padding:10px;border:1px solid var(--rw-border);border-radius:7px;background:var(--rw-surface-muted)}.approved-tip :global(> svg){flex:0 0 17px;color:var(--rw-text-secondary)}.approved-tip p{min-width:0;flex:1;margin:0;color:var(--rw-text-secondary);font-size:10px;line-height:14px}.approved-tip button{display:grid;place-items:center;width:24px;height:24px;flex:0 0 24px;margin:-5px -5px 0 0;padding:0;border-radius:50%;color:var(--rw-text-secondary);background:transparent}
  @media(max-width:980px){.approved-home-grid{grid-template-columns:minmax(0,1fr)}.approved-home-aside{display:grid;grid-template-columns:1fr 1fr;gap:14px}.approved-home-aside>.approved-section{margin-top:0}.approved-tip{grid-column:1/-1;margin-top:0}}
  @container content (max-width:900px){.approved-home-grid{grid-template-columns:minmax(0,1fr)}.approved-home-aside{display:grid;grid-template-columns:1fr 1fr;gap:14px}.approved-home-aside>.approved-section{margin-top:0}.approved-tip{grid-column:1/-1;margin-top:0}}
  @media(max-width:640px){.approved-page-title{align-items:flex-start;gap:10px}.approved-page-title p{max-width:320px}.approved-secondary{width:32px;padding:0;justify-content:center;font-size:0}.approved-home-aside{grid-template-columns:1fr}.approved-tip{grid-column:auto}.quick-grid{grid-template-columns:repeat(3,minmax(0,1fr))}.recent-table th:nth-child(3),.recent-table th:nth-child(4),.recent-table td:nth-child(3),.recent-table td:nth-child(4){display:none}.recent-table th:first-child{width:58%}.recent-table th:nth-child(2){width:20%}.recent-table th:last-child{width:22%}}
  @container content (max-width:640px){.approved-page-title{align-items:flex-start;gap:10px}.approved-page-title p{max-width:320px}.approved-secondary{width:32px;padding:0;justify-content:center;font-size:0}.approved-home-aside{grid-template-columns:1fr}.approved-tip{grid-column:auto}.quick-grid{grid-template-columns:repeat(3,minmax(0,1fr))}.recent-table th:nth-child(3),.recent-table th:nth-child(4),.recent-table td:nth-child(3),.recent-table td:nth-child(4){display:none}.recent-table th:first-child{width:58%}.recent-table th:nth-child(2){width:20%}.recent-table th:last-child{width:22%}}
</style>

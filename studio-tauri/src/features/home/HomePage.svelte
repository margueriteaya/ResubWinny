<script lang="ts">
  import { ChevronDown, FileUp, FileVideo2, FolderPlus, Layers3, ScanText, TriangleAlert } from "@lucide/svelte";
  import { t } from "../../i18n";
  import type { AppSettings, ExportFormat, ExportPreservation } from "../../backend";
  import { capabilitySummary } from "../tasks/format-capabilities";

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

  export let history: HistoryItem[] = [];
  export let isInspecting = false;
  export let onChooseSource: () => void = () => {};
  export let onOpenHistory: (item: HistoryItem) => void = () => {};
  export let onNavigate: (target: "batch" | "drcs") => void = () => {};
  export let settings: AppSettings;
  export let onSettingsChange: (settings: AppSettings) => void = () => {};
  const formats: ExportFormat[] = ["ASS", "TTML", "SRT", "WebVTT", "JSON", "Raw Data"];
  const preservationKeys: (keyof ExportPreservation)[] = ["position", "color", "ruby", "gaiji", "drcs", "accessibility"];
  function updatePreferences(next: Partial<AppSettings['exportPreferences']>) {
    onSettingsChange({ ...settings, exportPreferences: { ...settings.exportPreferences, ...next } });
  }
  function toggleFormat(format: ExportFormat) {
    const next = new Set(settings.exportPreferences.formats);
    next.has(format) ? next.delete(format) : next.add(format);
    if (!next.size) next.add("ASS");
    updatePreferences({ formats: [...next] });
  }
  function togglePreservation(key: keyof ExportPreservation) {
    updatePreferences({ preservation: { ...settings.exportPreferences.preservation, [key]: !settings.exportPreferences.preservation[key] } });
  }
  let noticeExpanded = false;

  const historyStatus = (item: HistoryItem) => {
    if (item.warnings) return `${item.warnings} ${t("home.warnings")}`;
    if (item.status === "In Progress") return t("batch.status.running");
    return t("batch.status.completed");
  };
</script>

<div class="workbench-home">
  <header class="workbench-home-title">
    <h1>{t("home.title")}</h1>
    <p>{t("home.subtitle")}</p>
  </header>

  <button class="recording-dropzone" onclick={onChooseSource} disabled={isInspecting} aria-label={t("home.selectRecording")}>
    <span class="caption-outline" class:inspecting={isInspecting} aria-hidden="true"><i></i><i></i><i></i><b>字</b></span>
    <FileUp size={32} />
    <span class="dropzone-copy">
      <strong>{isInspecting ? t("home.inspecting") : t("home.drop")}</strong>
      <small>{t("home.supportedInputs")}</small>
    </span>
    <span class="home-primary-action"><FolderPlus size={16} />{t("home.select")}</span>
  </button>
  <section class="home-output-preferences" aria-label="输出偏好">
    <h2>输出格式</h2>
    <div class="format-picker">{#each formats as format}<button type="button" class:selected={settings.exportPreferences.formats.includes(format)} onclick={() => toggleFormat(format)}>{format}</button>{/each}</div>
    <div class="format-notes">{#each settings.exportPreferences.formats as format}<p class="format-note"><b>{format}</b><br /><small>{capabilitySummary(format)}</small></p>{/each}</div>
    <h2>保留内容</h2>
    <div class="preservation-picker">{#each preservationKeys as key}<label><input type="checkbox" checked={settings.exportPreferences.preservation[key]} onchange={() => togglePreservation(key)} />{key === "gaiji" ? "外字" : key === "drcs" ? "DRCS" : key === "accessibility" ? "无障碍信息" : key === "ruby" ? "Ruby" : key === "position" ? "位置" : "颜色"}</label>{/each}</div>
  </section>
  <aside class="source-rights-notice">
    <TriangleAlert size={14}/><span>{t("home.rightsNotice")}</span>
    <button type="button" aria-expanded={noticeExpanded} aria-controls="source-rights-detail" onclick={() => noticeExpanded = !noticeExpanded}>{t("home.rightsWhy")}<ChevronDown size={13}/></button>
    {#if noticeExpanded}<p id="source-rights-detail">{t("onboarding.noticeBody")}</p>{/if}
  </aside>

  <section class="recent-workbench-section">
    <header><h2>{t("home.recent")}</h2></header>
    {#if history.length}
      <ol class="recent-task-list">
        {#each history.slice(0, 5) as item (item.path)}
          <li>
            <button disabled={isInspecting} onclick={() => onOpenHistory(item)}>
              <span class="recent-recording-icon"><FileVideo2 size={18} /></span>
              <span class="recent-recording-copy"><b>{item.name}</b><small>{item.time}</small></span>
              <span class:warning={item.warnings > 0 || item.status === "Warning"} class:progress={item.status === "In Progress"} class="state-dot"></span>
              <span class="recent-status">{historyStatus(item)}</span>
            </button>
          </li>
        {/each}
      </ol>
    {:else}
      <div class="empty-recent"><FileVideo2 size={25} /><p>{t("home.noCompleted")}</p></div>
    {/if}
  </section>

  <nav class="home-secondary-actions" aria-label={t("home.quickStart")}>
    <button onclick={() => onNavigate("batch")}><Layers3 size={17} /><span><b>{t("home.batch")}</b><small>{t("home.batchDescription")}</small></span></button>
    <button onclick={() => onNavigate("drcs")}><ScanText size={17} /><span><b>DRCS</b><small>{t("home.drcsDescription")}</small></span></button>
  </nav>
</div>

<style>
  .workbench-home { width: min(820px, 100%); min-width: 0; margin: clamp(8px, 4vh, 36px) auto 0; color: var(--rw-text); }
  .workbench-home-title { margin-bottom: 20px; text-align: center; }
  .workbench-home-title h1 { margin: 0; font-size: 24px; line-height: 30px; font-weight: 720; letter-spacing: -.02em; }
  .workbench-home-title p { margin: 6px 0 0; color: var(--rw-text-secondary); font-size: 13px; line-height: 19px; }
  .recording-dropzone { position:relative;display: grid; place-items: center; width: 100%; min-height: clamp(230px, 31vh, 290px); padding: 30px; overflow:hidden;border: 1px dashed color-mix(in srgb, var(--rw-accent) 58%, var(--rw-border)); border-radius: 12px; color: var(--rw-text); background: var(--rw-surface-muted); text-align: center; }
  .caption-outline{position:absolute;inset:18px;display:grid;align-content:center;gap:9px;padding:22px;border:1px solid color-mix(in srgb,var(--rw-accent) 14%,var(--rw-border-subtle));border-radius:8px;opacity:.5;pointer-events:none}.caption-outline i{display:block;width:34%;height:5px;margin-left:auto;margin-right:auto;border-radius:3px;background:color-mix(in srgb,var(--rw-accent) 12%,var(--rw-border))}.caption-outline i:nth-child(2){width:47%}.caption-outline b{position:absolute;right:16px;bottom:14px;color:color-mix(in srgb,var(--rw-accent) 22%,transparent);font-size:26px}.recording-dropzone> :global(svg),.dropzone-copy,.home-primary-action{position:relative;z-index:1}.caption-outline.inspecting i{animation:caption-scan 900ms var(--rw-ease-fluid) infinite alternate}.caption-outline.inspecting i:nth-child(2){animation-delay:120ms}.caption-outline.inspecting i:nth-child(3){animation-delay:240ms}
  .source-rights-notice{display:grid;grid-template-columns:14px minmax(0,1fr) auto;align-items:center;gap:7px;margin-top:8px;padding:8px 10px;border:1px solid color-mix(in srgb,#c73c3c 55%,var(--rw-border));border-left:3px solid #c73c3c;border-radius:8px;background:color-mix(in srgb,#d83e3e 9%,var(--rw-content));color:color-mix(in srgb,#9f2020 62%,var(--rw-text));font-size:11px;line-height:16px}.source-rights-notice :global(svg){color:#c93636}.source-rights-notice button{display:inline-flex;align-items:center;gap:3px;padding:2px 4px;border:0;color:#b52d2d;background:transparent;font-size:11px;font-weight:620}.source-rights-notice button :global(svg){transition:transform var(--rw-motion-fluid) var(--rw-ease-spring)}.source-rights-notice button[aria-expanded="true"] :global(svg){transform:rotate(180deg)}.source-rights-notice p{grid-column:2/-1;margin:1px 0 2px;color:color-mix(in srgb,#972626 48%,var(--rw-text-secondary))}:global([data-theme="dark"]) .source-rights-notice{color:#ffaaaa;background:color-mix(in srgb,#8d1717 28%,var(--rw-content));border-color:rgba(255,105,105,.48);border-left-color:#ff6868}:global([data-theme="dark"]) .source-rights-notice p{color:#efb6b6}:global([data-theme="dark"]) .source-rights-notice button{color:#ff9696}
  .recording-dropzone:hover:not(:disabled), .recording-dropzone:focus-visible { border-color: var(--rw-accent); background: color-mix(in srgb, var(--rw-accent) 5%, var(--rw-surface-muted)); }
  .recording-dropzone :global(> svg) { color: var(--rw-accent); }
  .dropzone-copy { display: grid; gap: 6px; margin-top: 14px; }
  .dropzone-copy strong { font-size: 17px; line-height: 22px; font-weight: 680; }
  .dropzone-copy small { color: var(--rw-text-secondary); font-size: 12px; line-height: 17px; }
  .home-primary-action { display: inline-flex; align-items: center; justify-content: center; gap: 7px; min-height: 36px; margin-top: 18px; padding: 0 16px; border: 1px solid color-mix(in srgb, var(--rw-accent) 80%, var(--rw-border)); border-radius: 7px; color: #fff; background: var(--rw-accent); font-size: 13px; font-weight: 680; }
  .recent-workbench-section { margin-top: 22px; }
  .home-output-preferences{margin-top:14px;padding:14px 16px;border:1px solid var(--rw-border-subtle);border-radius:9px;background:var(--rw-surface-muted)}.home-output-preferences h2{margin:0 0 8px;font-size:13px}.home-output-preferences h2:not(:first-child){margin-top:14px}.format-picker,.preservation-picker{display:flex;flex-wrap:wrap;gap:7px}.format-picker button{padding:6px 11px;border:1px solid var(--rw-border);border-radius:6px;color:var(--rw-text-secondary);background:var(--rw-content);font-size:12px}.format-picker button.selected{border-color:var(--rw-accent);color:var(--rw-text);background:color-mix(in srgb,var(--rw-accent) 12%,var(--rw-content))}.format-note{margin:8px 0 0;color:var(--rw-text-secondary);font-size:11px}.preservation-picker label{display:inline-flex;align-items:center;gap:5px;color:var(--rw-text-secondary);font-size:11px}
  .recent-workbench-section > header { display: flex; align-items: center; min-height: 36px; border-bottom: 1px solid var(--rw-border); }
  .recent-workbench-section h2 { margin: 0; font-size: 14px; line-height: 20px; font-weight: 680; }
  .recent-task-list { margin: 0; padding: 0; list-style: none; }
  .recent-task-list li { border-bottom: 1px solid var(--rw-border-subtle); }
  .recent-task-list button { display: grid; grid-template-columns: 34px minmax(0, 1fr) auto auto; align-items: center; width: 100%; min-height: 58px; gap: 10px; padding: 8px 7px; color: var(--rw-text); background: transparent; text-align: left; }
  .recent-task-list button:hover:not(:disabled), .recent-task-list button:focus-visible { background: color-mix(in srgb, var(--rw-accent) 6%, transparent); }
  .recent-recording-icon { display: grid; place-items: center; width: 30px; height: 34px; border: 1px solid var(--rw-border); border-radius: 6px; color: var(--rw-accent); background: var(--rw-content); }
  .recent-recording-copy { min-width: 0; }
  .recent-recording-copy b, .recent-recording-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .recent-recording-copy b { font-size: 13px; line-height: 18px; font-weight: 620; }
  .recent-recording-copy small { margin-top: 2px; color: var(--rw-muted); font-size: 11px; line-height: 15px; }
  .state-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--rw-success); }
  .state-dot.warning { background: var(--rw-warning); }.state-dot.progress { background: var(--rw-accent); }
  .recent-status { color: var(--rw-text-secondary); font-size: 12px; line-height: 17px; white-space: nowrap; }
  .empty-recent { display: grid; place-items: center; min-height: 100px; color: var(--rw-muted); text-align: center; }
  .empty-recent p { margin: 8px 0 0; font-size: 13px; }
  .home-secondary-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }
  .home-secondary-actions button { display: inline-flex; align-items: center; gap: 8px; min-width: 0; min-height: 42px; padding: 7px 10px; border: 1px solid var(--rw-border); border-radius: 7px; color: var(--rw-text); background: var(--rw-content); text-align: left; }
  .home-secondary-actions button:hover { border-color: color-mix(in srgb, var(--rw-accent) 55%, var(--rw-border)); background: color-mix(in srgb, var(--rw-accent) 5%, var(--rw-content)); }
  .home-secondary-actions button :global(svg) { flex: 0 0 auto; color: var(--rw-accent); }
  .home-secondary-actions span { min-width: 0; }.home-secondary-actions b, .home-secondary-actions small { display: block; }
  .home-secondary-actions b { font-size: 12px; line-height: 16px; font-weight: 650; }.home-secondary-actions small { overflow: hidden; max-width: 190px; color: var(--rw-muted); font-size: 11px; line-height: 14px; text-overflow: ellipsis; white-space: nowrap; }
  @container content (max-width: 620px) { .workbench-home { margin-top: 10px; }.workbench-home-title { text-align: left; }.recent-task-list button { grid-template-columns: 34px minmax(0, 1fr) auto; }.recent-status { display: none; }.home-secondary-actions { justify-content: stretch; }.home-secondary-actions button { flex: 1; }.home-secondary-actions small { max-width: 120px; } }
  @keyframes caption-scan{from{opacity:.2;transform:translateY(2px)}to{opacity:.95;transform:none}}
  @media(prefers-reduced-motion:reduce){.caption-outline.inspecting i{animation:none}.source-rights-notice button :global(svg){transition:none}}
  @media(forced-colors:active){.caption-outline,.source-rights-notice{border:1px solid CanvasText}}
</style>

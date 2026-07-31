<script lang="ts">
  import { FileOutput, FolderOpen, RotateCcw, TriangleAlert } from "@lucide/svelte";
  import { tick } from "svelte";
  import type { ExportFormat, ExportPreservation, Inspection } from "../../backend";
  import { t } from "../../i18n";

  type Format = { name: ExportFormat; description: string };
  type Feature = keyof ExportPreservation;
  export let inspection: Inspection;
  export let formats: Format[] = [];
  export let selectedFormats = new Set<ExportFormat>(["ASS"]);
  export let preservation: ExportPreservation;
  export let error = "";
  export let isExporting = false;
  export let previewIndexing = false;
  export let logs: string[] = [];
  export let onToggleFormat: (format: ExportFormat) => void = () => {};
  export let onTogglePreservation: (feature: Feature) => void = () => {};
  export let onStartExport: () => void = () => {};
  export let outputDirectory = "";
  export let onChooseOutputDirectory: () => void = () => {};
  export let canResume = false;
  export let resumeBusy = false;
  export let onResume: () => void = () => {};
  let logView: HTMLPreElement | null = null;
  let renderedLogCount = -1;
  const features: Feature[] = ["position", "color", "ruby", "drcs"];
  const lossyFormats = new Set<ExportFormat>(["SRT", "WebVTT"]);
  $: limitations = [...selectedFormats].filter((format) => lossyFormats.has(format));

  async function scrollLogs() {
    renderedLogCount = logs.length;
    await tick();
    if (logView) logView.scrollTop = logView.scrollHeight;
  }
  $: if (logView && logs.length !== renderedLogCount) void scrollLogs();
  function toggleAccessibilityAndGaiji() {
    const enabled = preservation.gaiji && preservation.accessibility;
    if (preservation.gaiji === enabled) onTogglePreservation("gaiji");
    if (preservation.accessibility === enabled) onTogglePreservation("accessibility");
  }
</script>

<section class="output-panel">
  <p class="eyebrow">{t("workspace.outputSettings")}</p>
  <fieldset class="format-fieldset">
    <legend>{t("workspace.outputFormat")}</legend>
    <div class="format-grid">
      {#each formats as item}
        <label class:checked={selectedFormats.has(item.name)}>
          <input type="checkbox" checked={selectedFormats.has(item.name)} onchange={() => onToggleFormat(item.name)} />
          <span><b>{item.name}</b><small>{item.description}</small></span>
        </label>
      {/each}
    </div>
  </fieldset>

  <fieldset class="preserve-fieldset">
    <legend>{t("workspace.preserveFeatures")}</legend>
    <div class="feature-grid">
      {#each features as feature}
        <label><input type="checkbox" checked={preservation[feature]} onchange={() => onTogglePreservation(feature)} /><span>{t(`feature.${feature}`)}</span></label>
      {/each}
      <label><input type="checkbox" checked={preservation.gaiji && preservation.accessibility} onchange={toggleAccessibilityAndGaiji} /><span>{t("feature.accessibilityAndGaiji")}</span></label>
    </div>
    <p>{t("workspace.preserveHint")}</p>
  </fieldset>

  {#if limitations.length}
    <section class="format-limitations">
      <b><TriangleAlert size={16} />{t("limits.selectedTitle")}</b>
      {#each limitations as selected}<p><strong>{selected}</strong> · {t(`limits.format.${selected.toLowerCase()}`)}</p>{/each}
    </section>
  {/if}

  <div class="output-directory"><span>{t("workspace.outputDirectory")}</span><div class="path-row"><input bind:value={outputDirectory} aria-label={t("workspace.outputDirectory")} /><button class="path-button" title={t("workspace.chooseOutputDirectory")} aria-label={t("workspace.chooseOutputDirectory")} onclick={onChooseOutputDirectory}><FolderOpen size={17} /></button></div></div>
  {#if inspection.container === "TLV"}<section class="constraint-card"><b>{t("limits.title")}</b><p>{t("limits.tlv")}</p></section>{/if}
  {#if error}<p class="error"><TriangleAlert size={16} />{error}</p>{/if}
  <div class="output-actions">
    <button class="export-button" onclick={onStartExport} disabled={isExporting || !selectedFormats.size}><FileOutput size={17} />{previewIndexing ? t("task.indexingTimeline") : isExporting ? t("common.exporting") : t("common.startExport")}</button>
    {#if canResume}<button class="resume-button" onclick={onResume} disabled={resumeBusy || isExporting}><RotateCcw size={16} /> {resumeBusy ? t("task.resumingCheckpoint") : t("task.resumeCheckpoint")}</button>{/if}
  </div>
  <section class="log-card"><div><b>{t("common.liveLog")}</b><span>{t("workspace.logEntries").replace("{0}", String(logs.length))}</span></div><pre bind:this={logView}>{logs.join("\n") || t("workspace.logEmpty")}</pre></section>
</section>

<style>
  fieldset{min-width:0;margin:0 0 15px;padding:0;border:0}legend{margin-bottom:8px;color:var(--rw-text);font-size:12px;font-weight:700}.format-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}.format-grid label{display:grid;grid-template-columns:auto minmax(0,1fr);gap:8px;align-items:start;padding:9px;border:1px solid var(--rw-border);border-radius:6px;background:var(--rw-surface-muted);cursor:pointer}.format-grid label.checked{border-color:var(--rw-accent);background:color-mix(in srgb,var(--rw-accent) 9%,var(--rw-surface-muted))}.format-grid input,.feature-grid input{margin-top:2px;accent-color:var(--rw-accent)}.format-grid b,.format-grid small{display:block}.format-grid b{font-size:12px}.format-grid small{margin-top:2px;color:var(--rw-muted);font-size:10px;line-height:1.25}.feature-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}.feature-grid label{display:flex;align-items:center;gap:7px;color:var(--rw-text-secondary);font-size:12px}.preserve-fieldset>p{margin:9px 0 0;color:var(--rw-muted);font-size:11px;line-height:1.45}.format-limitations{margin:0 0 15px;padding:10px 11px;border:1px solid color-mix(in srgb,var(--rw-warning) 65%,var(--rw-border));border-radius:6px;background:color-mix(in srgb,var(--rw-warning) 9%,var(--rw-surface));color:var(--rw-text-secondary)}.format-limitations>b{display:flex;align-items:center;gap:6px;color:var(--rw-warning);font-size:12px}.format-limitations p{margin:7px 0 0;font-size:11px;line-height:1.4}.output-directory{display:block}.path-row{display:grid;grid-template-columns:minmax(0,1fr) 38px;gap:6px;margin-top:6px}.path-row input{min-width:0;margin:0}.path-button{display:grid;place-items:center;width:38px;height:38px;color:var(--rw-text-secondary);border:1px solid var(--rw-border);border-radius:6px;background:var(--rw-surface-raised)}.path-button:hover{color:var(--rw-accent);border-color:var(--rw-accent)}.output-actions{display:flex;gap:10px;align-items:center}.export-button{display:flex;align-items:center;justify-content:center;gap:7px}.resume-button{display:inline-flex;align-items:center;gap:7px;padding:10px 13px;border:1px solid #a9c7ed;border-radius:7px;color:#176be6;background:#f4f9ff}.resume-button:disabled{opacity:.6;cursor:not-allowed}@media(max-width:1050px){.format-grid,.feature-grid{grid-template-columns:1fr}}
  .output-directory{color:var(--rw-text);font-size:12px;font-weight:700}
</style>

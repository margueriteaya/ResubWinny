<script lang="ts">
  import { FileOutput, FolderOpen, RotateCcw, TriangleAlert } from "@lucide/svelte";
  import { tick } from "svelte";
  import type { ExportFormat, ExportPreservation, Inspection } from "../../backend";
  import MacCheckbox from "../../components/MacCheckbox.svelte";
  import { t } from "../../i18n";

  type Format = { name: ExportFormat; description: string; icon?: any; color?: string };
  type Feature = keyof ExportPreservation;
  export let inspection: Inspection;
  export let formats: Format[] = [];
  export let selectedFormats = new Set<ExportFormat>(["ASS"]);
  export let preservation: ExportPreservation;
  export let error = "";
  export let isExporting = false;
  export let exportPending = false;
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
  <fieldset class="format-fieldset">
    <legend>{t("workspace.outputFormat")}</legend>
    <div class="format-grid">
      {#each formats as item}
        <div class="format-option" class:checked={selectedFormats.has(item.name)}>
          <MacCheckbox checked={selectedFormats.has(item.name)} label={item.name} onChange={() => onToggleFormat(item.name)} />
          <span class={`format-option-icon ${item.color ?? "blue"}`}>{#if item.icon}<svelte:component this={item.icon} size={15} />{/if}</span>
          <span class="format-option-copy"><b>{item.name}</b><small>{item.description}</small></span>
        </div>
      {/each}
    </div>
  </fieldset>

  <fieldset class="preserve-fieldset">
    <legend>{t("workspace.preserveFeatures")}</legend>
    <div class="feature-grid">
      {#each features as feature}
        <MacCheckbox checked={preservation[feature]} label={t(`feature.${feature}`)} onChange={() => onTogglePreservation(feature)} />
      {/each}
      <MacCheckbox checked={preservation.gaiji && preservation.accessibility} label={t("feature.accessibilityAndGaiji")} onChange={toggleAccessibilityAndGaiji} />
    </div>
    <p>{t("workspace.preserveHint")}</p>
  </fieldset>

  {#if limitations.length}
    <section class="format-limitations">
      <b><TriangleAlert size={16} />{t("limits.selectedTitle")}</b>
      {#each limitations as selected}<p><strong>{selected}</strong> · {t(`limits.format.${selected.toLowerCase()}`)}</p>{/each}
    </section>
  {/if}

  <div class="output-directory"><span>{t("workspace.outputDirectory")}</span><div class="path-row"><input bind:value={outputDirectory} aria-label={t("workspace.outputDirectory")} /><button class="path-button" data-tooltip={t("workspace.chooseOutputDirectory")} aria-label={t("workspace.chooseOutputDirectory")} onclick={onChooseOutputDirectory}><FolderOpen size={17} /></button></div></div>
  {#if inspection.container === "TLV"}<section class="constraint-card"><b>{t("limits.title")}</b><p>{t("limits.tlv")}</p></section>{/if}
  {#if error}<p class="error"><TriangleAlert size={16} />{error}</p>{/if}
  <div class="output-actions">
    <button class="export-button" onclick={onStartExport} disabled={isExporting || exportPending || !selectedFormats.size}><FileOutput size={17} />{exportPending ? t("task.exportWaitingForPreview") : isExporting ? t("common.exporting") : t("common.startExport")}</button>
    {#if canResume}<button class="resume-button" onclick={onResume} disabled={resumeBusy || isExporting}><RotateCcw size={16} /> {resumeBusy ? t("task.resumingCheckpoint") : t("task.resumeCheckpoint")}</button>{/if}
  </div>
  <section class="log-card"><div><b>{t("common.liveLog")}</b><span>{t("workspace.logEntries").replace("{0}", String(logs.length))}</span></div><pre bind:this={logView}>{logs.join("\n") || t("workspace.logEmpty")}</pre></section>
</section>

<style>
  fieldset{min-width:0;margin:0 0 15px;padding:0;border:0}legend{margin-bottom:8px;color:var(--rw-text);font-size:12px;font-weight:700}.format-grid{display:grid;grid-template-columns:1fr;gap:5px}.format-grid b,.format-grid small{display:block}.format-grid b{font-size:11px}.format-grid small{margin-top:1px;overflow:hidden;color:var(--rw-muted);font-size:9px;line-height:12px;text-overflow:ellipsis;white-space:nowrap}.feature-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}.preserve-fieldset>p{margin:9px 0 0;color:var(--rw-muted);font-size:11px;line-height:1.45}.format-limitations{margin:0 0 15px;padding:10px 11px;border:1px solid color-mix(in srgb,var(--rw-warning) 65%,var(--rw-border));border-radius:6px;background:color-mix(in srgb,var(--rw-warning) 9%,var(--rw-surface));color:var(--rw-text-secondary)}.format-limitations>b{display:flex;align-items:center;gap:6px;color:var(--rw-warning);font-size:12px}.format-limitations p{margin:7px 0 0;font-size:11px;line-height:1.4}.output-directory{display:block}.path-row{display:grid;grid-template-columns:minmax(0,1fr) 38px;gap:6px;margin-top:6px}.path-row input{min-width:0;margin:0}.path-button{display:grid;place-items:center;width:38px;height:38px;color:var(--rw-text-secondary);border:1px solid var(--rw-border);border-radius:6px;background:var(--rw-surface-raised)}.path-button:hover{color:var(--rw-accent);border-color:var(--rw-accent)}.output-actions{display:flex;gap:10px;align-items:center}.export-button{display:flex;align-items:center;justify-content:center;gap:7px}.resume-button{display:inline-flex;align-items:center;gap:7px;padding:10px 13px;border:1px solid #a9c7ed;border-radius:7px;color:#176be6;background:#f4f9ff}.resume-button:disabled{opacity:.6;cursor:not-allowed}@media(max-width:1050px){.feature-grid{grid-template-columns:1fr}}
  .output-directory{color:var(--rw-text);font-size:12px;font-weight:700}
  .format-option{position:relative;display:grid;grid-template-columns:16px 24px minmax(0,1fr);gap:7px;align-items:center;min-height:42px;padding:6px 8px;border:1px solid var(--rw-border);border-radius:6px;background:var(--rw-surface-muted)}.format-option.checked{border-color:color-mix(in srgb,var(--rw-accent) 60%,var(--rw-border));background:color-mix(in srgb,var(--rw-accent) 9%,var(--rw-surface-muted))}.format-option :global(.mac-checkbox){position:absolute;z-index:1;inset:0;align-items:center;width:100%;min-height:0;padding:6px 8px}.format-option :global(.checkbox-label){display:none}.format-option-icon{display:grid;place-items:center;width:24px;height:24px;grid-column:2;border-radius:5px}.format-option-icon.purple{color:#7b4db5;background:#7b4db512}.format-option-icon.green{color:#168247;background:#16824712}.format-option-icon.blue{color:#1766b3;background:#1766b312}.format-option-icon.orange{color:#b86400;background:#b8640012}.format-option-copy{min-width:0;grid-column:3}.feature-grid :global(.mac-checkbox){color:var(--rw-text-secondary);font-size:11px}
</style>

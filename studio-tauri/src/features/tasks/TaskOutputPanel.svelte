<script lang="ts">
  import { ChevronRight, FileOutput, FolderOpen, RotateCcw, TriangleAlert } from "@lucide/svelte";
  import type { ExportFormat, ExportPreservation, Inspection } from "../../backend";
  import MacCheckbox from "../../components/MacCheckbox.svelte";
  import { formatMessage, t } from "../../i18n";
  import { assessExports, type FeatureKnowledge, type RuntimeExportConflicts } from "./export-assessment";
  import { featureCountSummary } from "./event-state";
  import { formatCapabilities } from "./format-capabilities";

  type Format = { name: ExportFormat; description: string; icon?: any; color?: string };
  type Feature = keyof ExportPreservation;
  export let inspection: Inspection;
  export let formats: Format[] = [];
  export let selectedFormats = new Set<ExportFormat>(["ASS"]);
  export let preservation: ExportPreservation;
  export let featureKnowledge: FeatureKnowledge = {};
  export let runtimeConflicts: RuntimeExportConflicts = {};
  export let error = "";
  export let isExporting = false;
  export let exportPending = false;
  export let onToggleFormat: (format: ExportFormat) => void = () => {};
  export let onTogglePreservation: (feature: Feature) => void = () => {};
  export let onOpenDrcsMapping: () => void = () => {};
  export let onStartExport: () => void = () => {};
  export let outputDirectory = "";
  export let onChooseOutputDirectory: () => void = () => {};
  export let canResume = false;
  export let resumeBusy = false;
  export let onResume: () => void = () => {};
  const features: Feature[] = ["position", "color", "ruby", "gaiji", "drcs", "accessibility"];
  $: limitations = [...selectedFormats].filter((format) => formatCapabilities(format).some((item) => item.level === "unsupported"));
  $: assessment = assessExports(selectedFormats, preservation, featureKnowledge, runtimeConflicts);
  $: assessmentRows = [...selectedFormats].map((format) => ({ format, result: assessment.formats[format] })).filter((item) => item.result);
  $: observedFeatures = features.map((feature) => ({ feature, summary: featureCountSummary(featureKnowledge[feature]) })).filter((item) => item.summary);

  function chooseAss(format: ExportFormat) {
    if (!selectedFormats.has("ASS")) onToggleFormat("ASS");
    if (format !== "ASS" && selectedFormats.has(format)) onToggleFormat(format);
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
          <span class="format-option-copy"><b>{item.name}</b></span>
        </div>
      {/each}
    </div>
  </fieldset>

  <div class="output-directory"><span>{t("workspace.outputDirectory")}</span><div class="path-row"><input bind:value={outputDirectory} aria-label={t("workspace.outputDirectory")} /><button class="path-button" data-tooltip={t("workspace.chooseOutputDirectory")} aria-label={t("workspace.chooseOutputDirectory")} onclick={onChooseOutputDirectory}><FolderOpen size={17} /></button></div></div>
  {#if error}<p class="error"><TriangleAlert size={16} />{error}</p>{/if}
  {#if observedFeatures.length}<section class="feature-summary" aria-live="polite"><b>{t("assessment.sourceFeatures")}</b>{#each observedFeatures as item}<span>{formatMessage(item.summary!.final ? "assessment.featureCountFinal" : "assessment.featureCountObserved", { feature: t(`feature.${item.feature}`), count: item.summary!.count })}</span>{/each}</section>{/if}
  {#if assessmentRows.length}<section class:has-conflict={assessment.hasConflict} class="assessment" aria-live="polite"><b>{assessment.hasConflict ? t("assessment.needsSettings") : t("assessment.title")}</b>{#each assessmentRows as row}<div class="assessment-row"><strong>{row.format}</strong><div>{#each row.result?.approximated ?? [] as item}<span class="approx">△ {item.feature === "ruby" ? t("assessment.rubyApproximation") : item.feature === "gaiji" ? t("assessment.gaijiApproximation") : formatMessage("assessment.approximated", { features: t(`feature.${item.feature}`) })}</span>{/each}{#each row.result?.conditional ?? [] as item}<span class="conditional">◇ {formatMessage(item.code === "feature_will_be_dropped_if_present" ? "assessment.conditionalDrop" : item.code === "format_cannot_preserve_feature" ? "assessment.conditionalUnsupported" : "assessment.conditionalResource", { format: row.format, feature: t(`feature.${item.feature}`) })}</span>{/each}{#each row.result?.conflicts ?? [] as conflict}<span class="conflict">× {formatMessage("assessment.conflict", { feature: t(`feature.${conflict.feature}`) })}</span><div class="conflict-actions">{#if conflict.actions?.includes("open_drcs_mapping")}<button type="button" onclick={onOpenDrcsMapping}>{t("assessment.openDrcsMapping")}</button>{/if}{#if conflict.actions?.includes(`disable_preservation:${conflict.feature}`)}<button type="button" onclick={() => onTogglePreservation(conflict.feature)}>{formatMessage("assessment.disablePreservation", { feature: t(`feature.${conflict.feature}`) })}</button>{/if}{#if conflict.actions?.includes("remove_format")}<button type="button" onclick={() => onToggleFormat(row.format)}>{formatMessage("assessment.removeFormat", { format: row.format })}</button>{/if}{#if conflict.actions?.includes("choose_compatible_format") && row.format !== "ASS"}<button type="button" onclick={() => chooseAss(row.format)}>{t("assessment.useAss")}</button>{/if}</div>{/each}{#each row.result?.dropped ?? [] as item}{@const count = featureCountSummary(featureKnowledge[item.feature])}<span class="dropped">{formatMessage(count ? count.final ? "assessment.droppedFinal" : "assessment.droppedObserved" : "assessment.droppedFeature", { feature: t(`feature.${item.feature}`), count: count?.count })}</span>{/each}</div></div>{/each}</section>{/if}
  <div class="output-actions">
    <button class="export-button" onclick={onStartExport} disabled={isExporting || exportPending || !selectedFormats.size || assessment.hasConflict}><FileOutput size={17} />{exportPending ? t("task.exportWaitingForPreview") : isExporting ? t("common.exporting") : t("common.startExport")}</button>
    {#if canResume}<button class="resume-button" onclick={onResume} disabled={resumeBusy || isExporting}><RotateCcw size={16} /> {resumeBusy ? t("task.resumingCheckpoint") : t("task.resumeCheckpoint")}</button>{/if}
  </div>

  <details class:has-limitations={Boolean(limitations.length)} class="advanced-output">
    <summary><span><ChevronRight size={16} />{t("workspace.advancedOutput")}</span>{#if limitations.length}<TriangleAlert size={15} />{/if}</summary>
    <div class="advanced-output-content">
      <fieldset class="preserve-fieldset">
        <legend>{t("workspace.preserveFeatures")}</legend>
        <div class="feature-grid">
          {#each features as feature}
            <MacCheckbox checked={preservation[feature]} label={t(`feature.${feature}`)} onChange={() => onTogglePreservation(feature)} />
          {/each}
        </div>
        <p>{t("workspace.preserveHint")}</p>
      </fieldset>

      {#if limitations.length}
        <section class="format-limitations">
          <b><TriangleAlert size={16} />{t("limits.selectedTitle")}</b>
          {#each limitations as selected}<p><strong>{selected}</strong> · {t(`limits.format.${selected.toLowerCase()}`)}</p>{/each}
        </section>
      {/if}
      {#if inspection.container === "TLV"}<section class="constraint-card"><b>{t("limits.title")}</b><p>{t("limits.tlv")}</p></section>{/if}
    </div>
  </details>
</section>

<style>
  fieldset{min-width:0;margin:0;padding:0;border:0}legend{margin-bottom:8px;color:var(--rw-text);font-size:13px;font-weight:700}.format-grid{display:grid;grid-template-columns:1fr;gap:5px}.format-grid b{display:block;font-size:13px;line-height:17px}.feature-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}.preserve-fieldset>p{margin:9px 0 0;color:var(--rw-muted);font-size:12px;line-height:1.45}.format-limitations{margin:14px 0 0;padding:10px 11px;border:1px solid color-mix(in srgb,var(--rw-warning) 65%,var(--rw-border));border-radius:6px;background:color-mix(in srgb,var(--rw-warning) 9%,var(--rw-surface));color:var(--rw-text-secondary)}.format-limitations>b{display:flex;align-items:center;gap:6px;color:var(--rw-warning);font-size:12px}.format-limitations p{margin:7px 0 0;font-size:12px;line-height:1.4}.output-directory{display:block;margin-top:18px}.path-row{display:grid;grid-template-columns:minmax(0,1fr) 38px;gap:6px;margin-top:6px}.path-row input{min-width:0;margin:0}.path-button{display:grid;place-items:center;width:38px;height:38px;color:var(--rw-text-secondary);border:1px solid var(--rw-border);border-radius:6px;background:var(--rw-surface-raised)}.path-button:hover{color:var(--rw-accent);border-color:var(--rw-accent)}.output-actions{display:flex;gap:10px;align-items:center;margin-top:14px}.export-button{display:flex;align-items:center;justify-content:center;gap:7px}.resume-button{display:inline-flex;align-items:center;gap:7px;padding:10px 13px;border:1px solid #a9c7ed;border-radius:7px;color:#176be6;background:#f4f9ff}.resume-button:disabled{opacity:.6;cursor:not-allowed}@media(max-width:1050px){.feature-grid{grid-template-columns:1fr}}
  .output-directory{color:var(--rw-text);font-size:12px;font-weight:700}
  .feature-summary,.assessment{display:grid;gap:6px;margin-top:14px;padding:10px 11px;border:1px solid var(--rw-border);border-radius:7px;background:var(--rw-content);font-size:11px}.feature-summary>b,.assessment>b{font-size:12px}.feature-summary span{color:var(--rw-text-secondary);line-height:15px}.assessment.has-conflict{border-color:color-mix(in srgb,#c24848 55%,var(--rw-border));background:color-mix(in srgb,#c24848 5%,var(--rw-content))}.assessment.has-conflict>b{color:#c24848}.assessment-row{display:grid;grid-template-columns:72px minmax(0,1fr);gap:7px;align-items:start}.assessment-row strong{color:var(--rw-text)}.assessment-row span{display:block;line-height:15px}.approx{color:var(--rw-warning)}.conditional{color:var(--rw-text-secondary)}.conflict{color:#c24848}.dropped{color:var(--rw-muted)}.conflict-actions{display:flex;flex-wrap:wrap;gap:5px;margin:5px 0 7px}.conflict-actions button{padding:4px 7px;border:1px solid var(--rw-border);border-radius:5px;color:var(--rw-text-secondary);background:var(--rw-surface-raised);font-size:10px}.conflict-actions button:hover{color:var(--rw-accent);border-color:var(--rw-accent)}
  .format-option{position:relative;display:grid;grid-template-columns:16px 24px minmax(0,1fr);gap:7px;align-items:center;min-height:38px;padding:6px 8px;border:1px solid var(--rw-border);border-radius:6px;background:var(--rw-surface-muted)}.format-option.checked{border-color:color-mix(in srgb,var(--rw-accent) 60%,var(--rw-border));background:color-mix(in srgb,var(--rw-accent) 9%,var(--rw-surface-muted))}.format-option :global(.mac-checkbox){position:absolute;z-index:1;inset:0;align-items:center;width:100%;min-height:0;padding:6px 8px}.format-option :global(.checkbox-label){display:none}.format-option-icon{display:grid;place-items:center;width:24px;height:24px;grid-column:2;border-radius:5px}.format-option-icon.purple{color:#7b4db5;background:#7b4db512}.format-option-icon.green{color:#168247;background:#16824712}.format-option-icon.blue{color:#1766b3;background:#1766b312}.format-option-icon.orange{color:#b86400;background:#b8640012}.format-option-copy{min-width:0;grid-column:3}.feature-grid :global(.mac-checkbox){color:var(--rw-text-secondary);font-size:12px}
  .advanced-output{margin-top:18px;border-top:1px solid var(--rw-border-subtle)}.advanced-output summary{display:flex;align-items:center;justify-content:space-between;min-height:42px;cursor:pointer;color:var(--rw-text-secondary);font-size:12px;font-weight:650;list-style:none}.advanced-output summary::-webkit-details-marker{display:none}.advanced-output summary>span{display:flex;align-items:center;gap:5px}.advanced-output summary :global(svg:first-child){transition:transform var(--rw-motion-responsive) var(--rw-ease-out)}.advanced-output[open] summary :global(svg:first-child){transform:rotate(90deg)}.advanced-output.has-limitations summary>:global(svg:last-child){color:var(--rw-warning)}.advanced-output-content{padding:2px 0 2px}.advanced-output[open] .advanced-output-content{animation:rw-disclosure-in var(--rw-motion-fluid) var(--rw-ease-fluid) both}
</style>

<script lang="ts">
  import {
    ChevronDown,
    ChevronRight,
    CirclePause,
    CirclePlay,
    FileVideo2,
    FolderOutput,
    FolderPlus,
    FolderOpen,
    Gauge,
    ListRestart,
    Play,
    RadioTower,
    Trash2,
    TriangleAlert,
  } from "@lucide/svelte";
  import type { BatchItem } from "./controller";
  import type { ExportFormat, ExportPreservation } from "../../backend";
  import { trackDisplayLabel, trackKey } from "../tracks";
  import { t } from "../../i18n";
  import MacCheckbox from "../../components/MacCheckbox.svelte";
  import PopupButton from "../../components/PopupButton.svelte";

  export let items: BatchItem[] = [];
  export let running = false;
  export let paused = false;
  export let addFiles: () => void;
  export let clearQueue: () => void | Promise<void>;
  export let startQueue: () => void;
  export let pauseQueue: () => void;
  export let clearCompleted: () => void | Promise<void>;
  export let openItem: (item: BatchItem) => void;
  export let outputDirectory = "";
  export let chooseOutputDirectory: () => void;
  export let formats: { name: ExportFormat; description: string; icon?: any; color?: string }[] = [];
  export let selectedFormats = new Set<ExportFormat>(["ASS"]);
  export let preservation: ExportPreservation;
  export let onToggleFormat: (format: ExportFormat) => void = () => {};
  export let onTogglePreservation: (feature: keyof ExportPreservation) => void = () => {};
  const preservationKeys: (keyof ExportPreservation)[] = ["position", "color", "ruby", "drcs"];

  function toggleAccessibilityAndGaiji() {
    const enabled = preservation.gaiji && preservation.accessibility;
    if (preservation.gaiji === enabled) onTogglePreservation("gaiji");
    if (preservation.accessibility === enabled) onTogglePreservation("accessibility");
  }

  const bytes = (value: number) =>
    value
      ? `${(value / 1024 ** 3).toFixed(value > 100 * 1024 ** 3 ? 1 : 2)} GB`
      : "—";
  const selectedTrackLabel = (item: BatchItem) =>
    item.inspection.tracks.find(
      (track) => trackKey(track) === item.selectedTrackKey,
    );
  const selectedTrackDisplay = (item: BatchItem) => {
    const track = selectedTrackLabel(item);
    return track ? trackDisplayLabel(track) : t("batch.firstDetectedTrack");
  };
  const statusCode = (status: string) => {
    const normalized = status.trim().toLowerCase().replace(/[ _-]+/g, "");
    const aliases: Record<string, string> = {
      processing: "running", running: "running", complete: "completed",
      completed: "completed", warning: "warning", queued: "queued", ready: "ready",
      created: "created", inspecting: "inspecting", starting: "starting",
      pausing: "pausing", paused: "paused", resuming: "resuming",
      cancelling: "cancelling", cancelled: "cancelled", failed: "failed",
      interrupted: "interrupted",
    };
    return aliases[normalized] ?? normalized;
  };
  const isStatus = (item: BatchItem, code: string) => statusCode(item.status) === code;
  const statusLabel = (status: string) => t(`batch.status.${statusCode(status)}`, status);
  let preset = "custom";
  $: selectedFormatOptions = formats.filter((format) => selectedFormats.has(format.name));
  $: destinationLabel = outputDirectory.trim() || t("batch.sameFolder");
  $: queueSummary = items.reduce(
    (summary, item) => {
      const status = statusCode(item.status);
      if (status === "running") summary.running += 1;
      else if (status === "queued") summary.queued += 1;
      else if (status === "completed") summary.completed += 1;
      return summary;
    },
    { running: 0, queued: 0, completed: 0 },
  );
</script>

<section class="batch-shell">
  <div class="batch-actions">
    <button class="add" onclick={addFiles}
      ><FolderPlus size={20} /> {t("batch.add")}</button
    ><button
      class="secondary output-directory"
      onclick={chooseOutputDirectory}
      data-tooltip={outputDirectory || t("batch.sameFolder")}
      ><FolderOpen size={17} /><span>{outputDirectory || t("batch.chooseOutputDirectory")}</span></button
    ><button
      class="secondary"
      onclick={startQueue}
      disabled={(running && !paused) || queueSummary.queued === 0}
      ><Play size={18} />
      {paused ? t("batch.resumeQueue") : running ? t("batch.queueRunning") : t("batch.startQueue")}</button
    ><button
      class="secondary"
      onclick={clearQueue}
      disabled={running || !items.length}
      ><Trash2 size={17} /> {t("batch.clearQueue")}</button
    ><span></span>
  </div>
  <div class="batch-grid">
    <section class="queue-area">
      {#if items.length}
        <div class="queue-table">
          <div class="queue-heading">
            <span>{t("batch.file")}</span><span>{t("batch.service")}</span><span
              >{t("batch.outputFormat")}</span
            ><span>{t("batch.status")}</span><span>{t("batch.progress")}</span
            ><span>{t("batch.warnings")}</span><span
              >{t("batch.destination")}</span
            >
          </div>
          {#each items as item (item.inspection.path)}
            <button class="queue-row" onclick={() => openItem(item)}
              ><span class="file"
                ><span class="file-kind" aria-hidden="true">{#if item.inspection.container === "TLV"}<RadioTower size={18} />{:else}<FileVideo2 size={18} />{/if}<small>{item.inspection.container === "TLV" ? "TLV" : "TS"}</small></span><i
                  ><strong>{item.inspection.name}</strong><small
                    >{bytes(item.inspection.size)} · {item.inspection.tracks
                      .length}
                    {t("batch.captionTracks")}</small
                  ><small>{t("batch.track")}: {selectedTrackDisplay(item)}</small
                  ></i
                ></span
              ><span
                ><b>{item.inspection.service}</b><small
                  >{item.inspection.container}</small
                ></span
              ><span class="queue-formats">{#if selectedFormatOptions.length}<span class="format-icons">{#each selectedFormatOptions as format (format.name)}<span class={`format-icon ${format.color ?? "blue"}`} data-tooltip={format.description}>{#if format.icon}<svelte:component this={format.icon} size={13} />{/if}</span>{/each}</span><b>{selectedFormatOptions.map((format) => format.name).join(" · ")}</b>{:else}<b>—</b>{/if}<small>{t("batch.faithfulLayout")}</small></span
              ><span
                class:finished={isStatus(item, "completed")}
                class:issue={isStatus(item, "warning")}
                class="job-status"><i></i>{statusLabel(item.status)}</span
              ><span
                >{#if isStatus(item, "running")}<i class="job-progress"
                    ><b style={`width:${item.progress}%`}></b></i
                  ><small>{item.progress.toFixed(0)}%</small>{:else}<small
                    >{isStatus(item, "completed") ? "100%" : "0%"}</small
                  >{/if}</span
              ><span
                >{#if item.warnings}<span class="warnings"
                    ><TriangleAlert size={16} /> {item.warnings}</span
                  >{:else}—{/if}</span
              ><span class="destination" data-tooltip={destinationLabel}
                ><FolderOutput size={15} /><span>{destinationLabel}</span><ChevronRight size={16} /></span
              ></button
            >
          {/each}
        </div>
        <footer class="queue-footer"><span>{items.length} {t("batch.jobs")}</span></footer>
        <div class="batch-summary">
          <section>
            <h2>{t("batch.jobSummary")}</h2>
            <div>
              <span><b>{items.length}</b><small>{t("batch.total")}</small></span
              ><span
                ><b
                  >{queueSummary.running}</b
                ><small>{t("batch.processing")}</small></span
              ><span
                ><b>{queueSummary.queued}</b
                ><small>{t("batch.queued")}</small></span
              ><span
                ><b
                  >{queueSummary.completed}</b
                ><small>{t("batch.completed")}</small></span
              >
            </div>
          </section>
          <section>
            <h2>{t("batch.throughput")}</h2>
            <div class="metric">
              <Gauge size={23} /><b
                >{running ? t("batch.streaming") : t("batch.ready")}</b
              ><small>{t("batch.boundedIo")}</small>
            </div>
          </section>
          <section>
            <h2>{t("batch.cpuIo")}</h2>
            <div class="metric">
              <ListRestart size={23} /><b>{t("batch.onDemand")}</b><small
                >{t("batch.videoNotDecoded")}</small
              >
            </div>
          </section>
        </div>
      {:else}
        <div class="queue-empty">
          <FolderPlus size={40} />
          <h2>{t("batch.emptyTitle")}</h2>
          <p>{t("batch.emptyDescription")}</p>
          <button class="add" onclick={addFiles}
            ><FolderPlus size={19} /> {t("batch.add")}</button
          >
        </div>
      {/if}
    </section>
    <aside class="preset-panel">
      <p>{t("batch.preset")}</p>
      <PopupButton label={t("batch.preset")} value={preset} options={[{ value: "custom", label: t("batch.customOptions") }]} onChange={(value) => preset = value} />
      <section class="batch-options">
        <h2>{t("workspace.outputFormat")}</h2>
        <div class="batch-format-list">
          {#each formats as item}<div class="batch-format-option" class:checked={selectedFormats.has(item.name)}><MacCheckbox checked={selectedFormats.has(item.name)} label={item.name} onChange={() => onToggleFormat(item.name)} /><span class={`format-icon ${item.color ?? "blue"}`}>{#if item.icon}<svelte:component this={item.icon} size={13} />{/if}</span><span class="format-copy"><b>{item.name}</b><small>{item.description}</small></span></div>{/each}
        </div>
        <h2>{t("workspace.preserveFeatures")}</h2>
        <div class="batch-preserve-list">
          {#each preservationKeys as feature}<MacCheckbox checked={preservation[feature]} label={t(`feature.${feature}`)} onChange={() => onTogglePreservation(feature)} />{/each}
          <MacCheckbox checked={preservation.gaiji && preservation.accessibility} label={t("feature.accessibilityAndGaiji")} onChange={toggleAccessibilityAndGaiji} />
        </div>
      </section>
      <section>
        <h2>{t("batch.presetDetails")}</h2>
        <dl>
          <div>
            <dt>{t("batch.outputFormat")}</dt>
            <dd>{t("batch.advancedAss")}</dd>
          </div>
          <div>
            <dt>{t("batch.service")}</dt>
            <dd>{t("batch.configuredTrack")}</dd>
          </div>
          <div>
            <dt>{t("batch.drcsHandling")}</dt>
            <dd>{t("batch.drcsHandling")}</dd>
          </div>
        </dl>
      </section>
      <section>
        <h2>{t("batch.quickActions")}</h2>
        <button class="quick-action" onclick={clearCompleted}
          ><Trash2 size={17} /> {t("batch.clearCompleted")}</button
        ><button class="quick-action" onclick={pauseQueue} disabled={!running || paused}
          ><CirclePause size={17} /> {t("batch.pauseQueue")}</button
        ><button
          class="quick-action"
          onclick={startQueue}
          disabled={!paused}
          ><CirclePlay size={17} /> {t("batch.resumeQueue")}</button
        >
      </section>
    </aside>
  </div>
</section>

<style>
  .batch-shell {
    min-height: 660px;
  }
  .batch-actions {
    display: flex;
    gap: 11px;
    align-items: center;
    padding: 22px 20px;
  }
  .batch-actions > span {
    flex: 1;
  }
  .add,
  .secondary {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 11px 15px;
    color: #eaf3fc;
    border: 1px solid #34414d;
    border-radius: 6px;
    background: #1b252f;
  }
  .add {
    color: #fff;
    border-color: #1974e8;
    background: #146de5;
  }
  .secondary:disabled,
  .quick-action:disabled {
    cursor: not-allowed;
    opacity: 0.48;
  }
  .batch-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 330px;
    min-height: 600px;
    border-top: 1px solid #2b3743;
  }
  .queue-area {
    padding: 0 18px;
  }
  .queue-table {
    overflow: hidden;
    margin-top: 0;
    border: 1px solid #33414e;
    border-radius: 7px;
  }
  .queue-heading,
  .queue-row {
    display: grid;
    grid-template-columns: 1.6fr 1fr 0.9fr 0.8fr 1fr 0.55fr 1.15fr;
    gap: 12px;
    align-items: center;
  }
  .queue-heading {
    min-height: 50px;
    padding: 0 14px;
    color: #adb9c6;
    border-bottom: 1px solid #33414e;
    font-size: 12px;
  }
  .queue-row {
    width: 100%;
    min-height: 111px;
    padding: 12px 14px;
    color: #dfe8f1;
    border-bottom: 1px solid #2c3946;
    background: #151d25;
    text-align: left;
  }
  .queue-row:last-child {
    border: 0;
  }
  .queue-row:hover {
    background: #192735;
  }
  .queue-row b,
  .queue-row small {
    display: block;
  }
  .queue-row small {
    margin-top: 6px;
    color: #8f9eae;
    font-size: 11px;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .file-kind {
    display: grid;
    place-items: center;
    width: 38px;
    height: 49px;
    color: #ccd7e2;
    border: 1px solid #4a5663;
    border-radius: 4px;
    background: #e9edf0;
    font-size: 11px;
  }
  .file i {
    min-width: 0;
    font-style: normal;
  }
  .file strong {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .job-status {
    position: relative;
    padding-left: 13px;
    color: #5eabff;
    font-size: 13px;
  }
  .job-status i {
    position: absolute;
    left: 0;
    top: 4px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #2686f2;
  }
  .job-status.finished {
    color: #55ce7d;
  }
  .job-status.finished i {
    background: #55ce7d;
  }
  .job-status.issue {
    color: #f3b44c;
  }
  .job-status.issue i {
    background: #f3b44c;
  }
  .job-progress {
    display: block;
    overflow: hidden;
    width: 100px;
    height: 6px;
    border-radius: 5px;
    background: #34414e;
  }
  .job-progress b {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: #227cf0;
  }
  .warnings {
    display: flex;
    align-items: center;
    gap: 4px;
    color: #efad3a;
  }
  .destination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: #b8c4d1;
    font-size: 12px;
  }
  .queue-footer {
    display: flex;
    justify-content: space-between;
    padding: 13px 3px;
    color: #9ba9b8;
    font-size: 12px;
  }
  .batch-summary {
    display: grid;
    grid-template-columns: 1.2fr 1fr 1fr;
    gap: 12px;
    margin-top: 22px;
  }
  .batch-summary section,
  .preset-panel section {
    padding: 16px;
    border: 1px solid #303e4b;
    border-radius: 7px;
    background: #151d25;
  }
  .batch-options{margin-top:20px!important;padding:0!important;border:0!important;background:transparent!important}
  .batch-format-list,.batch-preserve-list{display:grid;gap:7px}
  .batch-format-list b,.batch-format-list small{display:block}.batch-format-list small{margin-top:3px;color:#91a0b0;font-size:10px;line-height:1.3}
  .batch-preserve-list{grid-template-columns:1fr 1fr}
  .batch-summary h2,
  .preset-panel h2 {
    margin: 0 0 17px;
    font-size: 14px;
  }
  .batch-summary section > div {
    display: flex;
    gap: 18px;
  }
  .batch-summary span b,
  .batch-summary span small {
    display: block;
  }
  .batch-summary span b {
    font-size: 24px;
  }
  .batch-summary span small,
  .metric small {
    margin-top: 5px;
    color: #91a0b0;
    font-size: 11px;
  }
  .metric {
    display: grid !important;
    grid-template-columns: 27px 1fr;
    align-items: center;
  }
  .metric small {
    grid-column: 2;
  }
  .queue-empty {
    display: grid;
    place-items: center;
    gap: 12px;
    min-height: 480px;
    color: #a7b4c1;
    border: 1px dashed #3c5c7e;
    border-radius: 8px;
    text-align: center;
  }
  .queue-empty h2,
  .queue-empty p {
    margin: 0;
  }
  .queue-empty h2 {
    color: #e6edf5;
  }
  .queue-empty p {
    max-width: 460px;
    line-height: 1.5;
  }
  .preset-panel {
    padding: 23px 20px;
    border-left: 1px solid #2b3743;
    background: #111820;
  }
  .preset-panel > p {
    margin: 0 0 10px;
    color: #a7b4c1;
    font-size: 11px;
    font-weight: 700;
  }
  .preset-panel section {
    margin-top: 20px;
  }
  .preset-panel dl {
    margin: 0;
  }
  .preset-panel dl div {
    margin: 0 0 16px;
  }
  .preset-panel dt {
    color: #95a4b4;
    font-size: 11px;
  }
  .preset-panel dd {
    margin: 5px 0 0;
    color: #d8e2eb;
    font-size: 12px;
    line-height: 1.4;
  }
  .quick-action {
    display: flex;
    gap: 9px;
    align-items: center;
    width: 100%;
    padding: 11px 0;
    color: #d7e1ec;
    border-bottom: 1px solid #2e3b47;
    background: transparent;
    text-align: left;
  }
  .quick-action:last-child {
    border: 0;
  }
  @media (prefers-color-scheme: light) {
    .batch-grid {
      border-color: #dfe6ee;
    }
    .add,
    .secondary {
      color: #334254;
      border-color: #d9e2ec;
      background: #fff;
    }
    .add {
      color: #fff;
      background: #176ce7;
    }
    .queue-table,
    .queue-heading,
    .queue-row {
      border-color: #dfe6ee;
    }
    .queue-row,
    .batch-summary section,
    .preset-panel section {
      color: #2d3a4a;
      background: #fff;
    }
    .queue-row:hover {
      background: #f3f8ff;
    }
    .queue-heading,
    .queue-row small,
    .queue-footer,
    .batch-summary span small,
    .metric small,
    .preset-panel > p,
    .preset-panel dt {
      color: #718093;
    }
    .preset-panel {
      border-color: #dfe6ee;
      background: #fafcff;
    }
    .preset-panel dd,
    .quick-action {
      color: #394758;
    }
    .quick-action {
      border-color: #e3e9ef;
    }
    .queue-empty {
      color: #728095;
      border-color: #bfd3ea;
    }
    .queue-empty h2 {
      color: #2a3849;
    }
    .file-kind {
      color: #4d5968;
      background: #f2f4f5;
    }
  }

  /* Theme selection is explicit application state. These token overrides sit
     after OS media rules so a user-selected dark theme also works on a light
     operating system, and vice versa. */
  .batch-shell{color:var(--rw-text)}
  .secondary{color:var(--rw-text);border-color:var(--rw-border);background:var(--rw-surface-raised)}
  .batch-grid{border-color:var(--rw-border)}
  .queue-area{overflow-x:auto}
  .queue-table{min-width:920px;border-color:var(--rw-border)}
  .queue-heading{color:var(--rw-muted);border-color:var(--rw-border)}
  .queue-row{color:var(--rw-text);border-color:var(--rw-border-subtle);background:var(--rw-surface-raised)}
  .queue-row:hover{background:color-mix(in srgb,var(--rw-accent) 7%,var(--rw-surface-raised))}
  .queue-row small,.queue-footer,.batch-summary span small,.metric small,.preset-panel>p,.preset-panel dt{color:var(--rw-muted)}
  .file-kind{color:var(--rw-text-secondary);border-color:var(--rw-border);background:var(--rw-surface-muted)}
  .job-status{color:var(--rw-accent)}.job-status.finished{color:var(--rw-success)}.job-status.finished i{background:var(--rw-success)}.job-status.issue,.warnings{color:var(--rw-warning)}.job-status.issue i{background:var(--rw-warning)}
  .job-progress{background:var(--rw-border)}.destination{color:var(--rw-text-secondary)}
  .batch-summary section,.preset-panel section{color:var(--rw-text);border-color:var(--rw-border);background:var(--rw-surface-raised)}
  .batch-format-list small{color:var(--rw-muted)}
  .queue-empty{color:var(--rw-muted);border-color:color-mix(in srgb,var(--rw-accent) 55%,var(--rw-border))}.queue-empty h2{color:var(--rw-text)}
  .preset-panel{border-color:var(--rw-border);background:var(--rw-surface-muted)}
  .preset-panel dd,.quick-action{color:var(--rw-text-secondary)}.quick-action{border-color:var(--rw-border)}
  .batch-format-option{position:relative;display:grid;grid-template-columns:16px minmax(0,1fr);gap:8px;align-items:start;padding:8px;border:1px solid var(--rw-border);border-radius:6px;background:var(--rw-surface-raised)}.batch-format-option.checked{border-color:color-mix(in srgb,var(--rw-accent) 60%,var(--rw-border));background:color-mix(in srgb,var(--rw-accent) 9%,var(--rw-surface-raised))}.batch-format-option>span{grid-column:2}.batch-format-option :global(.mac-checkbox){position:absolute;z-index:1;inset:0;align-items:flex-start;width:100%;min-height:0;padding:8px}.batch-format-option :global(.checkbox-label){display:none}.batch-preserve-list :global(.mac-checkbox){color:var(--rw-text-secondary);font-size:11px}
  @media (max-width:1180px){.batch-grid{grid-template-columns:minmax(0,1fr)}.preset-panel{border-top:1px solid var(--rw-border);border-left:0}.queue-area{padding-bottom:18px}}
  @media (max-width:760px){.batch-actions{flex-wrap:wrap;padding:14px 12px}.batch-actions>span{display:none}.queue-area{padding-inline:10px}.batch-summary{grid-template-columns:1fr}.batch-preserve-list{grid-template-columns:1fr}}

  /* macOS 27 content geometry. The rules above preserve the existing state
     surface while this block supplies the approved visual hierarchy. */
  .batch-shell{min-height:0;overflow:hidden;border:1px solid var(--rw-border-subtle);border-radius:8px;background:var(--rw-content)}
  .batch-actions{gap:7px;min-height:48px;padding:7px 10px;border-bottom:1px solid var(--rw-border-subtle);background:var(--rw-surface-muted)}
  .add,.secondary{height:32px;min-height:32px;padding:0 10px;border:.5px solid var(--rw-glass-border);border-radius:8px;font-size:11px;box-shadow:var(--rw-control-shadow);backdrop-filter:blur(16px) saturate(1.2)}
  .add :global(svg),.secondary :global(svg){width:15px;height:15px;flex:0 0 15px}.output-directory{max-width:260px;min-width:0}.output-directory>span{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .batch-grid{grid-template-columns:minmax(0,1fr) 292px;min-height:520px;border-top:0}.queue-area{padding:10px;overflow:auto}.queue-table{min-width:820px;margin:0;border:1px solid var(--rw-border-subtle);border-radius:7px}
  .queue-heading,.queue-row{grid-template-columns:minmax(190px,1.5fr) minmax(90px,.72fr) minmax(100px,.72fr) 86px 88px 60px minmax(100px,.8fr);gap:8px}.queue-heading{min-height:32px;padding:0 10px;color:var(--rw-muted);border-color:var(--rw-border-subtle);background:var(--rw-surface-muted);font-size:9px;font-weight:650}.queue-row{min-height:76px;padding:8px 10px;color:var(--rw-text);border-color:var(--rw-border-subtle);background:transparent;contain-intrinsic-size:auto 76px}.queue-row:hover{background:color-mix(in srgb,var(--rw-accent) 6%,transparent)}
  .file{gap:8px}.file-kind{position:relative;display:grid;place-items:center;width:30px;height:38px;flex:0 0 30px;border:1px solid var(--rw-border);border-radius:5px}.file-kind>small{position:absolute;right:-4px;bottom:-3px;margin:0;padding:0 2px;border:1px solid var(--rw-content);border-radius:3px;background:var(--rw-surface-muted);font-size:6px;line-height:9px;font-weight:700}.file strong{font-size:11px;line-height:14px}.queue-row b{font-size:10px;line-height:13px}.queue-row small{margin-top:2px;font-size:8px;line-height:11px}.job-status{padding-left:11px;font-size:10px}.job-status i{top:3px;width:6px;height:6px}.job-progress{width:70px;height:4px}.warnings,.destination{font-size:9px}
  .queue-footer{height:28px;padding:0 3px;align-items:center;font-size:9px}.batch-summary{grid-template-columns:1.3fr 1fr 1fr;gap:8px;margin-top:10px}.batch-summary section{padding:10px;border-color:var(--rw-border-subtle);border-radius:7px;background:var(--rw-surface-muted)}.batch-summary h2{margin:0 0 9px;font-size:10px}.batch-summary section>div{gap:14px}.batch-summary span b{font-size:17px;line-height:20px}.batch-summary span small,.metric small{margin-top:2px;font-size:8px}.metric{grid-template-columns:21px 1fr!important}.metric :global(svg){width:18px;height:18px}.queue-empty{min-height:400px;gap:8px;border-color:color-mix(in srgb,var(--rw-accent) 48%,var(--rw-border));border-radius:8px;font-size:11px}.queue-empty h2{font-size:15px}.queue-empty p{font-size:11px}
  .preset-panel{padding:12px;border-color:var(--rw-border-subtle);background:var(--rw-surface-muted)}.preset-panel>p{margin:0 0 5px;font-size:9px}.preset-panel>:global(.popup-button){margin-top:0}.preset-panel section{margin-top:14px;padding:10px;border-color:var(--rw-border-subtle);border-radius:7px;background:var(--rw-content)}.preset-panel h2{margin:0 0 9px;font-size:10px}.batch-options{margin-top:14px!important}.batch-format-list,.batch-preserve-list{gap:5px}.batch-format-option{padding:6px;border-color:var(--rw-border-subtle);border-radius:6px}.batch-format-option :global(.mac-checkbox){padding:6px}.batch-format-list b{font-size:10px}.batch-format-list small{margin-top:1px;font-size:8px}.batch-preserve-list :global(.mac-checkbox){font-size:9px;gap:6px}.preset-panel dl div{margin-bottom:10px}.preset-panel dt{font-size:8px}.preset-panel dd{margin-top:2px;font-size:9px}.quick-action{gap:7px;height:30px;padding:0;font-size:9px}.quick-action :global(svg){width:14px;height:14px}
  @media(max-width:1180px){.batch-grid{grid-template-columns:minmax(0,1fr)}.preset-panel{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:10px;border-top:1px solid var(--rw-border-subtle);border-left:0}.preset-panel>p,.preset-panel>:global(.popup-button){grid-column:1/-1}.preset-panel section{margin-top:0}.batch-options{margin-top:0!important}}
  @media(max-width:760px){.batch-actions{padding:7px;flex-wrap:wrap}.batch-actions>span{display:none}.queue-area{padding:7px}.batch-summary,.preset-panel{grid-template-columns:1fr}.batch-preserve-list{grid-template-columns:1fr}.preset-panel>p,.preset-panel>:global(.popup-button){grid-column:auto}}

  /* The toolbar and inspector are control layers; the queue is content. Keep
     each layer visually and mechanically independent instead of wrapping the
     whole workflow in a web-style card. */
  .batch-shell{display:grid;grid-template-rows:48px minmax(0,1fr);width:100%;height:100%;min-height:0;border:0;border-radius:0;background:transparent}
  .batch-actions{justify-self:stretch;width:100%;min-width:0;padding:7px 0;border:0;border-bottom:1px solid var(--rw-border-subtle);background:transparent}
  .batch-grid{justify-self:stretch;width:100%;height:100%;min-height:0;overflow:hidden}
  .queue-area{min-width:0;min-height:0;padding:10px 12px 10px 0;overflow:auto}
  .queue-table{margin:0;border-color:var(--rw-border-subtle);background:var(--rw-content)}
  .queue-empty{min-height:100%;border:0;background:transparent}
  .preset-panel{min-width:0;min-height:0;padding:10px 0 12px 12px;overflow:auto;border-left:1px solid var(--rw-border-subtle);background:var(--rw-surface-muted);backdrop-filter:none;-webkit-backdrop-filter:none}
  .preset-panel section{padding:0;border:0;border-radius:0;background:transparent}
  .batch-format-option{background:color-mix(in srgb,var(--rw-content) 72%,transparent)}
  .queue-formats{min-width:0}.format-icons{display:flex;align-items:center;gap:2px;margin-bottom:3px}.format-icon{display:grid;place-items:center;width:22px;height:22px;flex:0 0 22px;border-radius:5px}.format-icon.purple{color:#7b4db5;background:#7b4db516}.format-icon.green{color:#168247;background:#16824716}.format-icon.blue{color:#1766b3;background:#1766b316}.format-icon.orange{color:#b86400;background:#b8640016}.queue-formats>b{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.destination{display:grid;grid-template-columns:15px minmax(0,1fr) 16px;gap:4px}.destination>span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.batch-format-option{grid-template-columns:16px 24px minmax(0,1fr);align-items:center}.batch-format-option>.format-icon{grid-column:2}.batch-format-option>.format-copy{min-width:0;grid-column:3}.format-copy b,.format-copy small{overflow:hidden;text-overflow:ellipsis}.batch-format-option :global(.mac-checkbox){align-items:center}
  .batch-summary section{border-color:var(--rw-border-subtle);background:var(--rw-surface-muted)}
  .add,.secondary{background:transparent}
  .add{background:color-mix(in srgb,var(--rw-accent) 76%,var(--rw-glass-control))}
  @media(max-width:1180px){.batch-shell{height:auto}.batch-grid{overflow:visible}.preset-panel{overflow:visible;padding:12px 0 0;border-top:1px solid var(--rw-border-subtle);border-left:0;background:transparent;backdrop-filter:none;-webkit-backdrop-filter:none}}
</style>

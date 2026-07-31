<script lang="ts">
  import {
    ChevronDown,
    ChevronRight,
    CirclePause,
    CirclePlay,
    FolderPlus,
    FolderOpen,
    Gauge,
    ListRestart,
    Play,
    Trash2,
    TriangleAlert,
  } from "@lucide/svelte";
  import type { BatchItem } from "./features/batch/controller";
  import type { ExportFormat, ExportPreservation } from "./backend";
  import { trackDisplayLabel, trackKey } from "./features/tracks";
  import { t } from "./i18n";

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
  export let formats: { name: ExportFormat; description: string }[] = [];
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
</script>

<section class="batch-shell">
  <div class="batch-actions">
    <button class="add" onclick={addFiles}
      ><FolderPlus size={20} /> {t("batch.add")}</button
    ><button
      class="secondary output-directory"
      onclick={chooseOutputDirectory}
      title={outputDirectory || t("batch.sameFolder")}
      ><FolderOpen size={17} /> {outputDirectory || t("batch.chooseOutputDirectory")}</button
    ><button
      class="secondary"
      onclick={startQueue}
      disabled={(running && !paused) || !items.some((item) => isStatus(item, "queued"))}
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
          {#each items as item}
            <button class="queue-row" onclick={() => openItem(item)}
              ><span class="file"
                ><b>{item.inspection.container === "TLV" ? "TLV" : "TS"}</b><i
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
              ><span><b>{[...selectedFormats].join(" · ") || "—"}</b><small>{t("batch.faithfulLayout")}</small></span
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
              ><span class="destination"
                >{t("batch.sameFolder")} <ChevronRight size={16} /></span
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
                  >{items.filter((item) => isStatus(item, "running"))
                    .length}</b
                ><small>{t("batch.processing")}</small></span
              ><span
                ><b>{items.filter((item) => isStatus(item, "queued")).length}</b
                ><small>{t("batch.queued")}</small></span
              ><span
                ><b
                  >{items.filter((item) => isStatus(item, "completed"))
                    .length}</b
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
      <div class="preset-select">{t("batch.customOptions")}</div>
      <section class="batch-options">
        <h2>{t("workspace.outputFormat")}</h2>
        <div class="batch-format-list">
          {#each formats as item}<label class:checked={selectedFormats.has(item.name)}><input type="checkbox" checked={selectedFormats.has(item.name)} onchange={() => onToggleFormat(item.name)} /><span><b>{item.name}</b><small>{item.description}</small></span></label>{/each}
        </div>
        <h2>{t("workspace.preserveFeatures")}</h2>
        <div class="batch-preserve-list">
          {#each preservationKeys as feature}<label><input type="checkbox" checked={preservation[feature]} onchange={() => onTogglePreservation(feature)} /><span>{t(`feature.${feature}`)}</span></label>{/each}
          <label><input type="checkbox" checked={preservation.gaiji && preservation.accessibility} onchange={toggleAccessibilityAndGaiji} /><span>{t("feature.accessibilityAndGaiji")}</span></label>
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
  .file > b {
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
  .batch-format-list label{display:grid;grid-template-columns:auto minmax(0,1fr);gap:8px;padding:8px;border:1px solid #303e4b;border-radius:5px;background:#151d25;cursor:pointer}
  .batch-format-list label.checked{border-color:#227cf0;background:#172b42}
  .batch-format-list b,.batch-format-list small{display:block}.batch-format-list small{margin-top:3px;color:#91a0b0;font-size:10px;line-height:1.3}
  .batch-preserve-list{grid-template-columns:1fr 1fr}.batch-preserve-list label{display:flex;align-items:center;gap:6px;color:#b8c4d1;font-size:11px}
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
  .preset-select {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 12px;
    color: #e4edf6;
    border: 1px solid #354350;
    border-radius: 6px;
    background: #1b252e;
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
    .preset-select {
      color: #344254;
      border-color: #d9e2ec;
      background: #fff;
    }
    .preset-panel dd,
    .quick-action {
      color: #394758;
    }
    .batch-format-list label{border-color:#dfe6ee;background:#fff}.batch-format-list label.checked{border-color:#176ce7;background:#f3f8ff}.batch-preserve-list label{color:#394758}
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
    .file > b {
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
  .file>b{color:var(--rw-text-secondary);border-color:var(--rw-border);background:var(--rw-surface-muted)}
  .job-status{color:var(--rw-accent)}.job-status.finished{color:var(--rw-success)}.job-status.finished i{background:var(--rw-success)}.job-status.issue,.warnings{color:var(--rw-warning)}.job-status.issue i{background:var(--rw-warning)}
  .job-progress{background:var(--rw-border)}.destination{color:var(--rw-text-secondary)}
  .batch-summary section,.preset-panel section{color:var(--rw-text);border-color:var(--rw-border);background:var(--rw-surface-raised)}
  .batch-format-list label{color:var(--rw-text);border-color:var(--rw-border);background:var(--rw-surface-raised)}
  .batch-format-list label.checked{border-color:var(--rw-accent);background:color-mix(in srgb,var(--rw-accent) 9%,var(--rw-surface-raised))}
  .batch-format-list small{color:var(--rw-muted)}.batch-preserve-list label{color:var(--rw-text-secondary)}
  .batch-format-list input,.batch-preserve-list input{accent-color:var(--rw-accent)}
  .queue-empty{color:var(--rw-muted);border-color:color-mix(in srgb,var(--rw-accent) 55%,var(--rw-border))}.queue-empty h2{color:var(--rw-text)}
  .preset-panel{border-color:var(--rw-border);background:var(--rw-surface-muted)}
  .preset-select{color:var(--rw-text);border-color:var(--rw-border);background:var(--rw-surface-raised)}
  .preset-panel dd,.quick-action{color:var(--rw-text-secondary)}.quick-action{border-color:var(--rw-border)}
  @media (max-width:1180px){.batch-grid{grid-template-columns:minmax(0,1fr)}.preset-panel{border-top:1px solid var(--rw-border);border-left:0}.queue-area{padding-bottom:18px}}
  @media (max-width:760px){.batch-actions{flex-wrap:wrap;padding:14px 12px}.batch-actions>span{display:none}.queue-area{padding-inline:10px}.batch-summary{grid-template-columns:1fr}.batch-preserve-list{grid-template-columns:1fr}}
</style>

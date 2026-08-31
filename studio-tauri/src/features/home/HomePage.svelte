<script lang="ts">
  import { FileUp, FileVideo2, FolderPlus, Layers3, ScanText } from "@lucide/svelte";
  import { t } from "../../i18n";

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
    <FileUp size={32} />
    <span class="dropzone-copy">
      <strong>{isInspecting ? t("home.inspecting") : t("home.drop")}</strong>
      <small>{t("home.supportedInputs")}</small>
    </span>
    <span class="home-primary-action"><FolderPlus size={16} />{t("home.select")}</span>
  </button>

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
  .recording-dropzone { display: grid; place-items: center; width: 100%; min-height: clamp(230px, 31vh, 290px); padding: 30px; border: 1px dashed color-mix(in srgb, var(--rw-accent) 58%, var(--rw-border)); border-radius: 12px; color: var(--rw-text); background: var(--rw-surface-muted); text-align: center; }
  .recording-dropzone:hover:not(:disabled), .recording-dropzone:focus-visible { border-color: var(--rw-accent); background: color-mix(in srgb, var(--rw-accent) 5%, var(--rw-surface-muted)); }
  .recording-dropzone :global(> svg) { color: var(--rw-accent); }
  .dropzone-copy { display: grid; gap: 6px; margin-top: 14px; }
  .dropzone-copy strong { font-size: 17px; line-height: 22px; font-weight: 680; }
  .dropzone-copy small { color: var(--rw-text-secondary); font-size: 12px; line-height: 17px; }
  .home-primary-action { display: inline-flex; align-items: center; justify-content: center; gap: 7px; min-height: 36px; margin-top: 18px; padding: 0 16px; border: 1px solid color-mix(in srgb, var(--rw-accent) 80%, var(--rw-border)); border-radius: 7px; color: #fff; background: var(--rw-accent); font-size: 13px; font-weight: 680; }
  .recent-workbench-section { margin-top: 22px; }
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
</style>

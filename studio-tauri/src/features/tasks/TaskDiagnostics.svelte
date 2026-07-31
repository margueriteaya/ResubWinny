<script lang="ts">
  import { TriangleAlert, RefreshCw } from "@lucide/svelte";
  import { backend, type DiagnosticRecord } from "../../backend";
  import { formatMessage, t } from "../../i18n";

  export let jobId = "";
  export let desktopRuntime = false;
  export let onCountChange: (count: number) => void = () => {};
  export let onError: (message: string) => void = () => {};

  const pageSize = 100;
  let records: DiagnosticRecord[] = [];
  let loadedJobId = "";
  let loading = false;
  let exhausted = false;

  async function loadPage(reset: boolean) {
    if (!desktopRuntime || !jobId || loading || (!reset && exhausted)) return;
    loading = true;
    if (reset) {
      loadedJobId = jobId;
      records = [];
      exhausted = false;
    }
    try {
      const next = await backend.getJobDiagnosticsWindow(jobId, records.length, pageSize);
      records = [...records, ...next];
      exhausted = next.length < pageSize;
      onCountChange(records.length);
    } catch (reason) {
      onError(String(reason));
    } finally {
      loading = false;
    }
  }

  $: if (desktopRuntime && jobId && jobId !== loadedJobId) void loadPage(true);
</script>

<section class="event-list diagnostics-list">
  <header>
    <b>{t("workspace.diagnosticsTitle")}</b>
    <span>{t("workspace.recordsCount").replace("{0}", String(records.length))}</span>
  </header>
  {#if records.length}
    <ol>
      {#each records.slice().reverse() as item}
        <li>
          <time>{new Date(item.timestamp * 1000).toLocaleTimeString()}</time>
          <span><strong>{item.code}</strong><small>{formatMessage(item.code, item.parameters, item.message)}</small></span>
        </li>
      {/each}
    </ol>
    {#if !exhausted}
      <button class="load-more" onclick={() => loadPage(false)} disabled={loading}>
        <RefreshCw size={15} /> {loading ? t("workspace.loading") : t("workspace.loadMore")}
      </button>
    {/if}
  {:else}
    <div class="event-empty">
      <TriangleAlert size={30} />
      <p>{loading ? t("workspace.loading") : t("workspace.diagnosticsEmpty")}</p>
    </div>
  {/if}
</section>

<style>
  .event-list { min-height: 360px; color: var(--rw-text); }
  header { display: flex; justify-content: space-between; gap: 12px; padding: 15px 17px; border-bottom: 1px solid var(--rw-border); }
  header span { color: var(--rw-muted); font-size: 12px; }
  ol { margin: 0; padding: 0; list-style: none; }
  li { display: grid; grid-template-columns: 88px minmax(0, 1fr); gap: 12px; padding: 12px 17px; border-bottom: 1px solid var(--rw-border-subtle); }
  time { color: var(--rw-accent); font: 12px "Cascadia Mono", monospace; }
  strong, small { display: block; }
  strong { color: var(--rw-text); font: 12px "Cascadia Mono", monospace; }
  small { margin-top: 4px; color: var(--rw-text-secondary); line-height: 1.4; }
  .event-empty { display: grid; place-items: center; gap: 10px; min-height: 240px; padding: 20px; color: var(--rw-muted); text-align: center; }
  .event-empty p { max-width: 360px; margin: 0; line-height: 1.5; }
  .load-more { display: flex; align-items: center; gap: 7px; margin: 14px auto; padding: 8px 12px; color: var(--rw-text); border: 1px solid var(--rw-border); border-radius: 5px; background: var(--rw-surface-raised); }
  .load-more:disabled { opacity: .55; }
</style>

<script lang="ts">
  import { FileText, FolderCog, History, ScanText, Settings2 } from "@lucide/svelte";
  import packageMetadata from "../../package.json";
  import { t } from "../i18n";
  import type { Page } from "./navigation";

  export let page: Page = "home";
  export let hasTask = false;
  export let taskName = "";
  export let busy = false;
  export let onNavigate: (page: Page) => void = () => {};

  const displayVersion = `v${packageMetadata.version.replace(/-alpha(?:\.\d+)?$/, "alpha")}`;
</script>

<aside class="sidebar" data-liquid-region>
  <div class="sidebar-identity" data-liquid-ignore><strong>ResubWinny</strong><small>{t("app.tagline")} · {displayVersion}</small></div>
  <p class="sidebar-section-label" data-liquid-ignore>{t("nav.workspace")}</p>
  <nav class="sidebar-navigation" data-liquid-ignore aria-label={t("app.navigation")}>
    <button aria-label={t("nav.home")} class:active={page === "home"} onclick={() => onNavigate("home")}><History size={16} /><span>{t("nav.home")}</span></button>
    <button aria-label={t("nav.tasks")} class:active={page === "tasks"} onclick={() => onNavigate("tasks")}><FileText size={16} /><span>{t("nav.tasks")}</span>{#if hasTask}<em>1</em>{/if}</button>
    <button aria-label={t("nav.batch")} class:active={page === "batch"} onclick={() => onNavigate("batch")}><FolderCog size={16} /><span>{t("nav.batch")}</span></button>
    <button aria-label={t("nav.drcs")} class:active={page === "drcs"} onclick={() => onNavigate("drcs")}><ScanText size={16} /><span>{t("nav.drcs")}</span></button>
    <button aria-label={t("nav.settings")} class:active={page === "settings"} onclick={() => onNavigate("settings")}><Settings2 size={16} /><span>{t("nav.settings")}</span></button>
  </nav>
  {#if hasTask}
    <p class="sidebar-section-label current-label" data-liquid-ignore>{t("nav.currentTask")}</p>
    <button class="sidebar-current-task" data-liquid-ignore class:busy onclick={() => onNavigate("tasks")}><span>{busy ? t("task.processing") : t("nav.editing")}</span><b>{taskName}</b></button>
  {/if}
</aside>

<style>
  .sidebar-identity{height:44px;padding:4px 14px 6px}.sidebar-identity strong{display:block;font-size:15px;line-height:20px;font-weight:650}.sidebar-identity small{display:block;overflow:hidden;color:var(--rw-muted);font-size:9px;line-height:13px;text-overflow:ellipsis;white-space:nowrap}.sidebar-section-label{height:20px;margin:10px 0 0;padding:0 14px;color:var(--rw-text-secondary);font-size:10px;line-height:20px;font-weight:560}.sidebar-navigation{display:grid;gap:2px;padding:2px 8px}.sidebar-navigation button{position:relative;display:grid;grid-template-columns:20px minmax(0,1fr) auto;align-items:center;gap:8px;width:100%;height:32px;min-height:32px;padding:0 10px;border:0;border-radius:8px;color:var(--rw-text);background:transparent;font-size:12px;text-align:left}.sidebar-navigation button:hover{background:color-mix(in srgb,var(--rw-text) 7%,transparent)}.sidebar-navigation button.active{color:var(--rw-text);background:color-mix(in srgb,var(--rw-accent) 12%,transparent)}.sidebar-navigation :global(svg){width:16px;height:16px;color:currentColor;stroke-width:1.8}.sidebar-navigation button.active :global(svg){color:var(--rw-accent)}.sidebar-navigation em{min-width:18px;height:18px;padding:0 5px;border-radius:9px;color:#fff;background:var(--rw-accent);font-size:9px;line-height:18px;font-style:normal;text-align:center}.current-label{margin-top:9px}.sidebar-current-task{display:block;width:calc(100% - 28px);margin:2px 14px 0;padding:8px 2px;border:0;border-top:1px solid var(--rw-border);border-radius:0;color:var(--rw-text);background:transparent;text-align:left}.sidebar-current-task span,.sidebar-current-task b{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.sidebar-current-task span{color:var(--rw-muted);font-size:9px;line-height:13px}.sidebar-current-task b{margin-top:3px;font-size:11px;line-height:14px;font-weight:600}.sidebar-current-task.busy span{color:var(--rw-warning)}
  :global(main.sidebar-collapsed) .sidebar-identity,:global(main.sidebar-collapsed) .sidebar-section-label,:global(main.sidebar-collapsed) .sidebar-current-task,:global(main.sidebar-collapsed) .sidebar-navigation span{display:none}:global(main.sidebar-collapsed) .sidebar-navigation{padding:8px 10px}:global(main.sidebar-collapsed) .sidebar-navigation button{grid-template-columns:1fr;place-items:center;width:48px;height:36px;padding:0}:global(main.sidebar-collapsed) .sidebar-navigation button:hover span,:global(main.sidebar-collapsed) .sidebar-navigation button:focus-visible span{position:absolute;z-index:60;left:54px;display:block;width:max-content;max-width:180px;padding:5px 8px;border:.5px solid var(--rw-glass-border);border-radius:6px;color:var(--rw-text);background:color-mix(in srgb,var(--rw-glass-shell) 90%,var(--rw-content));box-shadow:0 6px 20px rgba(0,0,0,.18),inset 0 .5px rgba(255,255,255,.62);backdrop-filter:blur(24px) saturate(1.2);-webkit-backdrop-filter:blur(24px) saturate(1.2);font-size:11px;line-height:15px;white-space:nowrap}:global(main.sidebar-collapsed) .sidebar-navigation em{position:absolute;top:2px;right:2px}
  @media(max-width:1250px){.sidebar-identity,.sidebar-section-label,.sidebar-current-task,.sidebar-navigation span{display:none}.sidebar-navigation{padding:8px 10px}.sidebar-navigation button{grid-template-columns:1fr;place-items:center;width:48px;height:36px;padding:0}.sidebar-navigation button:hover span,.sidebar-navigation button:focus-visible span{position:absolute;z-index:60;left:54px;display:block;width:max-content;max-width:180px;padding:5px 8px;border:.5px solid var(--rw-glass-border);border-radius:6px;color:var(--rw-text);background:color-mix(in srgb,var(--rw-glass-shell) 90%,var(--rw-content));box-shadow:0 6px 20px rgba(0,0,0,.18);backdrop-filter:blur(24px) saturate(1.2);-webkit-backdrop-filter:blur(24px) saturate(1.2);font-size:11px;line-height:15px;white-space:nowrap}.sidebar-navigation em{position:absolute;top:2px;right:2px}}
</style>

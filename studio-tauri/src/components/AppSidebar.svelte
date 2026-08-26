<script lang="ts">
  import { FileText, FolderCog, House, ScanText, Settings2 } from "@lucide/svelte";
  import packageMetadata from "../../package.json";
  import { t } from "../i18n";
  import type { Page } from "./navigation";

  export let page: Page = "home";
  export let collapsed = false;
  export let hasTask = false;
  export let taskName = "";
  export let busy = false;
  export let onNavigate: (page: Page) => void = () => {};

  const displayVersion = `v${packageMetadata.version.replace(/-alpha(?:\.\d+)?$/, "alpha")}`;
  const pageOrder: Page[] = ["home", "tasks", "batch", "drcs", "settings"];
  $: activeIndex = Math.max(0, pageOrder.indexOf(page));
</script>

<aside id="app-sidebar" class="sidebar" class:collapsed data-liquid-region>
  <div class="sidebar-identity" data-liquid-ignore><strong>ResubWinny</strong><small>{t("app.tagline")} · {displayVersion}</small></div>
  <p class="sidebar-section-label" data-liquid-ignore>{t("nav.workspace")}</p>
  <nav class="sidebar-navigation" data-liquid-ignore aria-label={t("app.navigation")} style={`--sidebar-selection-index:${activeIndex}`}>
    <span class="sidebar-selection" aria-hidden="true"></span>
    <button class="liquid-control" type="button" aria-label={t("nav.home")} aria-current={page === "home" ? "page" : undefined} data-tooltip={collapsed ? t("nav.home") : undefined} class:active={page === "home"} onclick={() => onNavigate("home")}><House size={16} /><span>{t("nav.home")}</span></button>
    <button class="liquid-control" type="button" aria-label={t("nav.tasks")} aria-current={page === "tasks" ? "page" : undefined} data-tooltip={collapsed ? t("nav.tasks") : undefined} class:active={page === "tasks"} onclick={() => onNavigate("tasks")}><FileText size={16} /><span>{t("nav.tasks")}</span>{#if hasTask}<em>1</em>{/if}</button>
    <button class="liquid-control" type="button" aria-label={t("nav.batch")} aria-current={page === "batch" ? "page" : undefined} data-tooltip={collapsed ? t("nav.batch") : undefined} class:active={page === "batch"} onclick={() => onNavigate("batch")}><FolderCog size={16} /><span>{t("nav.batch")}</span></button>
    <button class="liquid-control" type="button" aria-label={t("nav.drcs")} aria-current={page === "drcs" ? "page" : undefined} data-tooltip={collapsed ? t("nav.drcs") : undefined} class:active={page === "drcs"} onclick={() => onNavigate("drcs")}><ScanText size={16} /><span>{t("nav.drcs")}</span></button>
    <button class="liquid-control" type="button" aria-label={t("nav.settings")} aria-current={page === "settings" ? "page" : undefined} data-tooltip={collapsed ? t("nav.settings") : undefined} class:active={page === "settings"} onclick={() => onNavigate("settings")}><Settings2 size={16} /><span>{t("nav.settings")}</span></button>
  </nav>
  {#if hasTask}
    <p class="sidebar-section-label current-label" data-liquid-ignore>{t("nav.currentTask")}</p>
    <button class="sidebar-current-task" data-liquid-ignore class:busy onclick={() => onNavigate("tasks")}><span>{busy ? t("task.processing") : t("nav.editing")}</span><b>{taskName}</b></button>
  {/if}
</aside>

<style>
  .sidebar-identity {
    height: 44px;
    padding: 4px 14px 6px;
    overflow: hidden;
    opacity: 1;
    transform: translateX(0);
    transition: height var(--rw-motion-fluid) var(--rw-ease-fluid), padding var(--rw-motion-fluid) var(--rw-ease-fluid), opacity var(--rw-motion-responsive) var(--rw-ease-out), transform var(--rw-motion-fluid) var(--rw-ease-fluid), visibility 0s linear;
  }
  .sidebar-identity strong { display: block; font-size: 15px; line-height: 20px; font-weight: 650; white-space: nowrap; }
  .sidebar-identity small { display: block; overflow: hidden; color: var(--rw-muted); font-size: 9px; line-height: 13px; text-overflow: ellipsis; white-space: nowrap; }
  .sidebar-section-label {
    height: 20px;
    margin: 10px 0 0;
    padding: 0 14px;
    overflow: hidden;
    color: var(--rw-text-secondary);
    font-size: 10px;
    line-height: 20px;
    font-weight: 560;
    white-space: nowrap;
    opacity: 1;
    transform: translateX(0);
    transition: height var(--rw-motion-fluid) var(--rw-ease-fluid), margin var(--rw-motion-fluid) var(--rw-ease-fluid), opacity var(--rw-motion-responsive) var(--rw-ease-out), transform var(--rw-motion-fluid) var(--rw-ease-fluid), visibility 0s linear;
  }
  .sidebar-navigation {
    --sidebar-nav-inset: 8px;
    --sidebar-icon-left: 10px;
    position: relative;
    display: grid;
    gap: 2px;
    padding: 2px var(--sidebar-nav-inset);
    transition: padding var(--rw-motion-fluid) var(--rw-ease-fluid);
  }
  .sidebar-selection {
    position: absolute;
    z-index: 0;
    top: 2px;
    left: var(--sidebar-nav-inset);
    right: var(--sidebar-nav-inset);
    height: 36px;
    border: .5px solid color-mix(in srgb, var(--rw-glass-border) 88%, var(--rw-accent) 12%);
    border-radius: 10px;
    background: color-mix(in srgb, var(--rw-accent) 12%, rgba(255,255,255,.30));
    box-shadow: 0 4px 12px rgba(25,34,42,.08), inset 0 .75px rgba(255,255,255,.64), inset 0 -.5px rgba(41,56,69,.10);
    backdrop-filter: blur(15px) saturate(1.24) brightness(1.025);
    -webkit-backdrop-filter: blur(15px) saturate(1.24) brightness(1.025);
    transform: translate3d(0, calc(var(--sidebar-selection-index) * 38px), 0);
    transition: transform var(--rw-motion-fluid) var(--rw-ease-spring), left var(--rw-motion-fluid) var(--rw-ease-fluid), right var(--rw-motion-fluid) var(--rw-ease-fluid), border-radius var(--rw-motion-fluid) var(--rw-ease-fluid);
    pointer-events: none;
  }
  .sidebar-selection::after { position: absolute; inset: .5px; border: .5px solid rgba(255,255,255,.28); border-radius: inherit; opacity: .78; content: ""; }
  .sidebar-navigation button {
    position: relative;
    z-index: 1;
    display: block;
    width: 100%;
    height: 36px;
    min-height: 36px;
    padding: 0;
    overflow: hidden;
    border: 0;
    border-radius: 10px;
    color: var(--rw-text);
    background: transparent !important;
    font-size: 12px;
    text-align: left;
    transition: color var(--rw-motion-responsive) var(--rw-ease-out);
  }
  .sidebar-navigation button::before { position: absolute; z-index: -1; inset: 0; border-radius: inherit; background: color-mix(in srgb,var(--rw-text) 7%,transparent); opacity: 0; transition: opacity var(--rw-motion-fast) var(--rw-ease-out); content: ""; }
  .sidebar-navigation button::after { position: absolute; z-index: 0; inset: 0; border-radius: inherit; background: radial-gradient(circle at var(--rw-liquid-pointer-x,50%) var(--rw-liquid-pointer-y,0%),rgba(255,255,255,.22),transparent 58%); opacity: 0; transition: opacity var(--rw-motion-responsive) var(--rw-ease-out); content: ""; }
  .sidebar-navigation button:hover:not(.active)::before,
  .sidebar-navigation button:focus-visible:not(.active)::before,
  .sidebar-navigation button:hover::after,
  .sidebar-navigation button:focus-visible::after { opacity: 1; }
  .sidebar-navigation button > span {
    position: absolute;
    z-index: 1;
    top: 0;
    left: 38px;
    width: calc(100% - 70px);
    height: 36px;
    overflow: hidden;
    line-height: 36px;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 1;
    transform: translateX(0);
    transition: width var(--rw-motion-fluid) var(--rw-ease-fluid), opacity var(--rw-motion-responsive) var(--rw-ease-out), transform var(--rw-motion-fluid) var(--rw-ease-fluid), visibility 0s linear;
  }
  .sidebar-navigation :global(svg) {
    position: absolute;
    z-index: 1;
    top: 10px;
    left: var(--sidebar-icon-left);
    width: 16px;
    height: 16px;
    color: currentColor;
    stroke-width: 1.8;
    transition: left var(--rw-motion-fluid) var(--rw-ease-fluid), color var(--rw-motion-responsive) var(--rw-ease-out), transform var(--rw-motion-fluid) var(--rw-ease-spring);
  }
  .sidebar-navigation button.active :global(svg) { color: var(--rw-accent); transform: scale(1.04); }
  .sidebar-navigation em {
    position: absolute;
    z-index: 1;
    top: 9px;
    right: 10px;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    color: #fff;
    background: var(--rw-accent);
    font-size: 9px;
    line-height: 18px;
    font-style: normal;
    text-align: center;
    transition: top var(--rw-motion-fluid) var(--rw-ease-fluid), right var(--rw-motion-fluid) var(--rw-ease-fluid), transform var(--rw-motion-fluid) var(--rw-ease-spring), opacity var(--rw-motion-responsive) var(--rw-ease-out);
  }
  .current-label { margin-top: 9px; }
  .sidebar-current-task {
    display: block;
    max-height: 54px;
    width: calc(100% - 28px);
    margin: 2px 14px 0;
    padding: 8px 2px;
    overflow: hidden;
    border: 0;
    border-top: 1px solid var(--rw-border);
    border-radius: 0;
    color: var(--rw-text);
    background: transparent;
    text-align: left;
    opacity: 1;
    transform: translateX(0);
    transition: max-height var(--rw-motion-fluid) var(--rw-ease-fluid), margin var(--rw-motion-fluid) var(--rw-ease-fluid), padding var(--rw-motion-fluid) var(--rw-ease-fluid), border-color var(--rw-motion-responsive) var(--rw-ease-out), opacity var(--rw-motion-responsive) var(--rw-ease-out), transform var(--rw-motion-fluid) var(--rw-ease-fluid), visibility 0s linear;
  }
  .sidebar-current-task span,
  .sidebar-current-task b { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sidebar-current-task span { color: var(--rw-muted); font-size: 9px; line-height: 13px; }
  .sidebar-current-task b { margin-top: 3px; font-size: 11px; line-height: 14px; font-weight: 600; }
  .sidebar-current-task.busy span { color: var(--rw-warning); }

  .sidebar.collapsed .sidebar-identity,
  .sidebar.collapsed .sidebar-section-label,
  .sidebar.collapsed .sidebar-current-task { visibility: hidden; opacity: 0; transform: translateX(-8px); }
  .sidebar.collapsed .sidebar-identity,
  .sidebar.collapsed .sidebar-section-label { transition-delay: 0s,0s,0s,0s,var(--rw-motion-responsive); }
  .sidebar.collapsed .sidebar-current-task { transition-delay: 0s,0s,0s,0s,0s,0s,var(--rw-motion-responsive); }
  .sidebar.collapsed .sidebar-identity { height: 0; padding-top: 0; padding-bottom: 0; }
  .sidebar.collapsed .sidebar-section-label { height: 0; margin-top: 0; }
  .sidebar.collapsed .sidebar-current-task { max-height: 0; margin-top: 0; margin-bottom: 0; padding-top: 0; padding-bottom: 0; border-color: transparent; }
  .sidebar.collapsed .sidebar-navigation { --sidebar-nav-inset: 0px; --sidebar-icon-left: 18px; }
  .sidebar.collapsed .sidebar-navigation button > span { width: 0; visibility: hidden; opacity: 0; transform: translateX(-8px); transition-delay: 0s,0s,0s,var(--rw-motion-responsive); }
  .sidebar.collapsed .sidebar-navigation em { top: 1px; right: 2px; min-width: 16px; height: 16px; padding: 0 4px; line-height: 16px; transform: scale(.86); }

  @media (prefers-reduced-motion: reduce) {
    .sidebar-identity,
    .sidebar-section-label,
    .sidebar-current-task,
    .sidebar-navigation,
    .sidebar-selection,
    .sidebar-navigation button,
    .sidebar-navigation button > span,
    .sidebar-navigation :global(svg),
    .sidebar-navigation em { transition: none !important; }
  }
</style>

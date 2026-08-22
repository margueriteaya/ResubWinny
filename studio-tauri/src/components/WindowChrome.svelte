<script lang="ts">
  import { FolderOpen, PanelLeftClose, PanelLeftOpen } from "@lucide/svelte";
  import { PanelRightClose, PanelRightOpen } from "@lucide/svelte";
  import type { WorkspaceLayoutSettings } from "../backend";
  import {
    closeActive,
    closeDefault,
    closeHover,
    maximizeActive,
    maximizeDefault,
    maximizeHover,
    minimizeActive,
    minimizeDefault,
    minimizeHover,
  } from "macos-traffic-lights";
  import { t } from "../i18n";

  export let sidebarCollapsed = false;
  export let page: "home" | "tasks" | "batch" | "drcs" | "settings" = "home";
  export let workspaceLayout: WorkspaceLayoutSettings = { sourceWidth: 240, outputWidth: 300, sourceCollapsed: false, outputCollapsed: false };
  export let sourceInspectorCollapsed = workspaceLayout.sourceCollapsed;
  export let outputInspectorCollapsed = workspaceLayout.outputCollapsed;
  export let onWindowAction: (action: "minimize" | "maximize" | "close") => void = () => {};
  export let onBeginDrag: () => void = () => {};
  export let onBeginResize: (direction: string) => void = () => {};
  export let onToggleSidebar: () => void = () => {};
  export let onChooseSource: () => void = () => {};
  export let onToggleSourceInspector: () => void = () => {};
  export let onToggleOutputInspector: () => void = () => {};

  const resizeDirections = [
    ["n", "North"], ["ne", "NorthEast"], ["e", "East"], ["se", "SouthEast"],
    ["s", "South"], ["sw", "SouthWest"], ["w", "West"], ["nw", "NorthWest"],
  ] as const;

  const lights = [
    { action: "close", label: () => t("app.closeWindow"), normal: closeDefault, hover: closeHover, active: closeActive },
    { action: "minimize", label: () => t("app.minimizeWindow"), normal: minimizeDefault, hover: minimizeHover, active: minimizeActive },
    { action: "maximize", label: () => t("app.maximizeWindow"), normal: maximizeDefault, hover: maximizeHover, active: maximizeActive },
  ] as const;
</script>

{#each resizeDirections as [edge, direction]}
  <div role="presentation" class={`resize-handle resize-${edge}`} onmousedown={() => onBeginResize(direction)}></div>
{/each}

<header class="window-titlebar" data-liquid-region aria-label={t("app.titleBar")}>
  <div class="traffic-lights" data-liquid-ignore aria-label={t("app.windowControls")}>
    {#each lights as light}
      <button class={`traffic-light traffic-${light.action}`} aria-label={light.label()} onclick={(event) => { event.stopPropagation(); onWindowAction(light.action); }}>
        <img class="traffic-normal" src={light.normal} alt="" />
        <img class="traffic-hover" src={light.hover} alt="" />
        <img class="traffic-active" src={light.active} alt="" />
      </button>
    {/each}
  </div>
  <button class="titlebar-icon liquid-control sidebar-toggle" aria-label={sidebarCollapsed ? t("app.showSidebar") : t("app.hideSidebar")} data-tooltip={sidebarCollapsed ? t("app.showSidebar") : t("app.hideSidebar")} onclick={onToggleSidebar}>
    {#if sidebarCollapsed}<PanelLeftOpen size={16} />{:else}<PanelLeftClose size={16} />{/if}
  </button>
  <div class="title-drag-region" role="presentation" onmousedown={onBeginDrag}></div>
  <div class="titlebar-tools">
    {#if page === "tasks"}
      <div class="titlebar-button-group" aria-label={t("workspace.panelControls")}>
        <button class="titlebar-icon liquid-control" aria-label={sourceInspectorCollapsed ? t("app.showSidebar") : t("app.hideSidebar")} data-tooltip={sourceInspectorCollapsed ? t("app.showSidebar") : t("app.hideSidebar")} onclick={onToggleSourceInspector}>{#if sourceInspectorCollapsed}<PanelLeftOpen size={16} />{:else}<PanelLeftClose size={16} />{/if}</button>
        <button class="titlebar-icon liquid-control" aria-label={outputInspectorCollapsed ? t("workspace.showOutput") : t("workspace.hideOutput")} data-tooltip={outputInspectorCollapsed ? t("workspace.showOutput") : t("workspace.hideOutput")} onclick={onToggleOutputInspector}>{#if outputInspectorCollapsed}<PanelRightOpen size={16} />{:else}<PanelRightClose size={16} />{/if}</button>
      </div>
    {/if}
    <button class="titlebar-icon liquid-control" aria-label={t("common.openFile")} data-tooltip={t("common.openFile")} onclick={onChooseSource}><FolderOpen size={16} /></button>
  </div>
</header>

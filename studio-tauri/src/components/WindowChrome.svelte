<script lang="ts">
  import { Minus, Square, X } from "@lucide/svelte";
  import { t } from "../i18n";

  export let onWindowAction: (action: "minimize" | "maximize" | "close") => void = () => {};
  export let onBeginDrag: () => void = () => {};
  export let onBeginResize: (direction: string) => void = () => {};

  const resizeDirections = [
    ["n", "North"], ["ne", "NorthEast"], ["e", "East"], ["se", "SouthEast"],
    ["s", "South"], ["sw", "SouthWest"], ["w", "West"], ["nw", "NorthWest"],
  ] as const;
</script>

{#each resizeDirections as [edge, direction]}
  <div role="presentation" class={`resize-handle resize-${edge}`} onmousedown={() => onBeginResize(direction)}></div>
{/each}
<div class="title-drag-region" role="presentation" onmousedown={onBeginDrag}></div>
<div class="window-controls">
  <button aria-label={t("app.minimizeWindow")} onclick={(event) => { event.stopPropagation(); onWindowAction("minimize"); }}><Minus size={17} /></button>
  <button aria-label={t("app.maximizeWindow")} onclick={(event) => { event.stopPropagation(); onWindowAction("maximize"); }}><Square size={14} /></button>
  <button class="close-window" aria-label={t("app.closeWindow")} onclick={(event) => { event.stopPropagation(); onWindowAction("close"); }}><X size={17} /></button>
</div>

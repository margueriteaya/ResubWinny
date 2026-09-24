<script lang="ts">
  import type { ExportPreservation } from "../../backend";
  import { featureVisuals } from "./feature-visuals";

  export let feature: keyof ExportPreservation;
  export let size = 14;
  export let stroke = 1.8;

  $: visual = featureVisuals[feature];
</script>

<span class="feature-glyph" style:width={`${size}px`} style:height={`${size}px`} aria-hidden="true">
  {#if visual.kind === "asset"}
    <span class="feature-glyph-mask" style={`--feature-glyph: url("${visual.asset}")`}></span>
  {:else}
    <visual.icon {size} {stroke} />
  {/if}
</span>

<style>
  .feature-glyph,
  .feature-glyph-mask {
    display: block;
    flex: 0 0 auto;
  }

  .feature-glyph-mask {
    width: 100%;
    height: 100%;
    background: currentColor;
    -webkit-mask: var(--feature-glyph) center / contain no-repeat;
    mask: var(--feature-glyph) center / contain no-repeat;
  }
</style>

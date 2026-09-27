<script lang="ts">
  import type { ExportPreservation } from "../../backend";
  import { featureVisuals } from "./feature-visuals";

  let {
    feature,
    size = 14,
    stroke = 1.8,
  }: {
    feature: keyof ExportPreservation;
    size?: number;
    stroke?: number;
  } = $props();

  const visual = $derived(featureVisuals[feature]);
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

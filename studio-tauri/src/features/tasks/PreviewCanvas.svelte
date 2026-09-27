<script lang="ts">
  import { CirclePlay } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { PreviewFrameRenderer } from "./preview-frame-renderer";
  let { running, title, description, onError }: {
    running: boolean;
    title: string;
    description: string;
    onError: (message: string) => void;
  } = $props();
  let canvas: HTMLCanvasElement;
  let hasFrame = $state(false);

  onMount(() => {
    let renderer: PreviewFrameRenderer | null = null;
    const report = (reason: unknown) =>
      onError(reason instanceof Error ? reason.message : String(reason));
    try {
      renderer = new PreviewFrameRenderer(canvas, () => { hasFrame = true; }, report);
    } catch (reason) {
      report(reason);
    }
    return () => renderer?.dispose();
  });

  $effect(() => {
    if (!running) hasFrame = false;
  });
</script>
<div class="preview-canvas" class:has-frame={hasFrame}>
  <canvas bind:this={canvas} aria-label={title}></canvas>
  <div class="preview-notice" aria-hidden={hasFrame}>
    <CirclePlay size={30} />
    <b>{title}</b>
    <p>{description}</p>
  </div>
</div>
<style>
  .preview-canvas { position: relative; display: grid; min-height: clamp(270px, 38vh, 380px); overflow: hidden; background: var(--rw-content); }
  canvas, .preview-notice { grid-area: 1 / 1; width: 100%; height: 100%; }
  canvas { display: block; min-width: 0; min-height: 0; opacity: 0; background: var(--rw-video-canvas); }
  .has-frame canvas { opacity: 1; }
  .preview-notice { z-index: 1; display: grid; place-content: center; justify-items: center; box-sizing: border-box; padding: 24px; color: var(--rw-text-secondary); text-align: center; transition: opacity var(--rw-motion-responsive) var(--rw-ease-out); pointer-events: none; }
  .preview-notice b { margin-top: 10px; color: var(--rw-text); font-size: 13px; }
  .preview-notice p { max-width: 360px; margin: 5px 0 0; font-size: 12px; line-height: 17px; }
  .has-frame .preview-notice { opacity: 0; }
  @media (max-height: 700px) { .preview-canvas { min-height: 168px; } }
  @media (prefers-reduced-motion: reduce) { .preview-notice { transition: none; } }
</style>

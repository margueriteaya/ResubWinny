<script lang="ts">
  export let value = 0;
  export let min = 0;
  export let max = 100;
  export let step = 1;
  export let disabled = false;
  export let ariaLabel = "";
  export let role = "slider";
  export let ariaControls = "";
  export let ariaOrientation: "horizontal" | "vertical" = "horizontal";
  export let className = "";
  export let onInput: (value: number) => void = () => {};
  export let onChange: (value: number) => void = () => {};

  function input(event: Event, commit = false) {
    value = Number((event.currentTarget as HTMLInputElement).value);
    (commit ? onChange : onInput)(value);
  }
  $: progress = max === min ? 0 : Math.max(0, Math.min(100, ((value - min) / (max - min)) * 100));
</script>

<span class={`mac-slider ${className}`} class:disabled style={`--slider-progress:${progress}%;--slider-thumb-offset:${progress * 0.16}px`}>
  <input type="range" {min} {max} {step} {value} {disabled} {role} aria-label={ariaLabel} aria-controls={ariaControls || undefined} aria-orientation={ariaOrientation} aria-valuemin={min} aria-valuemax={max} aria-valuenow={value} oninput={(event) => input(event)} onchange={(event) => input(event, true)} />
  <span class="slider-thumb liquid-control" aria-hidden="true"></span>
</span>

<style>
  .mac-slider{position:relative;display:inline-flex;align-items:center;min-width:0;min-height:24px;vertical-align:middle}.mac-slider input{position:relative;z-index:3;width:100%;height:24px;margin:0;appearance:none!important;background:transparent!important;border:0!important;outline:none!important;cursor:pointer}
  .mac-slider input::-webkit-slider-runnable-track{height:4px;border-radius:2px;background:linear-gradient(to right,var(--rw-accent) 0%,var(--rw-accent) var(--slider-progress,0%),color-mix(in srgb,var(--rw-text) 18%,transparent) var(--slider-progress,0%),color-mix(in srgb,var(--rw-text) 18%,transparent) 100%);box-shadow:inset 0 .5px rgba(0,0,0,.16),0 .5px rgba(255,255,255,.45)}
  .mac-slider input::-webkit-slider-thumb{width:16px;height:16px;margin-top:-6px;appearance:none;border:0;border-radius:50%;background:transparent;box-shadow:none}
  .slider-thumb{position:absolute;z-index:2;top:4px;left:calc(var(--slider-progress) - var(--slider-thumb-offset));display:block;width:16px;height:16px;overflow:hidden;border:.5px solid var(--rw-glass-border);border-radius:50%;background:transparent;box-shadow:0 1px 3px rgba(0,0,0,.22),inset 0 1px rgba(255,255,255,.82),inset 0 -1px var(--rw-glass-dark-edge);pointer-events:none;transition:box-shadow var(--rw-motion-fast) var(--rw-ease-out)}
  .mac-slider:hover .slider-thumb{box-shadow:0 1px 4px rgba(0,0,0,.26),inset 0 1px rgba(255,255,255,.9),inset 0 -1px var(--rw-glass-dark-edge)}
  .mac-slider input:focus-visible + .slider-thumb{outline:2px solid color-mix(in srgb,var(--rw-accent) 52%,transparent);outline-offset:2px}
  .mac-slider input::-moz-range-track{height:4px;border-radius:2px;background:color-mix(in srgb,var(--rw-text) 18%,transparent)}.mac-slider input::-moz-range-progress{height:4px;border-radius:2px;background:var(--rw-accent)}.mac-slider input::-moz-range-thumb{width:12px;height:12px;border:1px solid var(--rw-glass-border);border-radius:50%;background:var(--rw-glass-control);box-shadow:0 1px 3px rgba(0,0,0,.22)}
  .mac-slider.disabled{opacity:.48}.mac-slider.disabled input{cursor:not-allowed}@media(prefers-reduced-motion:reduce){.slider-thumb{transition:none}}
</style>

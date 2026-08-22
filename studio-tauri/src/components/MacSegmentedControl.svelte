<script lang="ts">
  export let value = "";
  export let options: { value: string; label: string; icon?: any }[] = [];
  export let ariaLabel = "";
  export let size: "regular" | "toolbar" = "regular";
  export let iconOnly = false;
  export let onChange: (value: string) => void = () => {};
  $: selectedIndex = Math.max(0, options.findIndex((option) => option.value === value));
</script>

<div class:toolbar={size === "toolbar"} class:icon-only={iconOnly} class:liquid-control={size !== "toolbar"} class="mac-segmented" role="radiogroup" aria-label={ariaLabel} style={`--segment-count:${Math.max(1, options.length)};--segment-offset:${selectedIndex * 100}%`}>
  {#if size !== "toolbar"}<span class="segment-indicator" aria-hidden="true"></span>{/if}
  {#each options as option (option.value)}
    <button class:liquid-control={size === "toolbar"} class:selected={value === option.value} data-tooltip={iconOnly ? option.label : undefined} aria-label={iconOnly ? option.label : undefined} type="button" role="radio" aria-checked={value === option.value} onclick={() => { value = option.value; onChange(value); }}>
      {#if option.icon}<svelte:component this={option.icon} size={14} strokeWidth={1.8} />{/if}{#if !iconOnly}<span>{option.label}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .mac-segmented{position:relative;display:inline-flex;align-items:center;height:24px;padding:0;gap:0;overflow:hidden;border:.5px solid var(--rw-glass-border);border-radius:7px;background:transparent;box-shadow:var(--rw-control-shadow);backdrop-filter:blur(18px) saturate(1.28) brightness(1.03);-webkit-backdrop-filter:blur(18px) saturate(1.28) brightness(1.03)}
  .segment-indicator{position:absolute;z-index:1;inset:1px auto 1px 0;width:calc(100% / var(--segment-count));overflow:hidden;border:.5px solid var(--rw-glass-border);border-radius:5.5px;background:color-mix(in srgb,var(--rw-content) 42%,transparent);box-shadow:0 .5px 2px rgba(0,0,0,.14),inset 0 .5px rgba(255,255,255,.72),inset 0 -1px var(--rw-glass-dark-edge);transform:translateX(var(--segment-offset));transition:transform 180ms cubic-bezier(.22,1,.36,1),width 180ms cubic-bezier(.22,1,.36,1);pointer-events:none}
  .mac-segmented button{position:relative;z-index:2;display:inline-flex;align-items:center;justify-content:center;gap:5px;min-width:56px;height:24px;padding:0 9px;border:0;border-right:.5px solid color-mix(in srgb,var(--rw-text) 14%,transparent);border-radius:0;color:var(--rw-text-secondary);background:transparent;font-size:11px;line-height:15px;white-space:nowrap;transition:color 160ms ease}.mac-segmented button:last-child,.mac-segmented button.selected,.mac-segmented button:has(+ button.selected){border-right-color:transparent}.mac-segmented button:hover,.mac-segmented button.selected{color:var(--rw-text)}.mac-segmented button:focus-visible{z-index:3;outline:2px solid color-mix(in srgb,var(--rw-accent) 52%,transparent);outline-offset:-2px}.mac-segmented.toolbar{height:36px;gap:6px;padding:0;overflow:visible;border:0;border-radius:0;background:transparent;box-shadow:none;backdrop-filter:none;-webkit-backdrop-filter:none}.mac-segmented.toolbar button{min-width:36px;height:36px;padding:0 10px;overflow:hidden;border:.5px solid var(--rw-glass-border);border-radius:18px;background:transparent;box-shadow:var(--rw-control-shadow);backdrop-filter:blur(18px) saturate(1.24) brightness(1.03);-webkit-backdrop-filter:blur(18px) saturate(1.24) brightness(1.03)}.mac-segmented.toolbar button.selected{color:var(--rw-accent);background:color-mix(in srgb,var(--rw-accent) 10%,var(--rw-glass-control));box-shadow:var(--rw-control-shadow),inset 0 0 0 .5px color-mix(in srgb,var(--rw-accent) 38%,transparent)}.mac-segmented.icon-only button{width:36px;min-width:36px;padding:0}
  @media(prefers-reduced-motion:reduce){.mac-segmented button,.segment-indicator{transition:none}}
</style>

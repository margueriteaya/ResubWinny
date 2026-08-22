<script lang="ts">
  export let checked = false;
  export let disabled = false;
  export let label = "";
  export let mixed = false;
  export let onChange: (checked: boolean) => void = () => {};

  function update(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    checked = input.checked;
    mixed = false;
    onChange(checked);
  }
</script>

<label class="mac-checkbox" class:disabled data-tooltip={label || undefined}>
  <input type="checkbox" {checked} {disabled} aria-checked={mixed ? "mixed" : checked} onchange={update} />
  <span class="checkbox-box" class:mixed aria-hidden="true">{#if checked && !mixed}<svg class="checkmark" viewBox="0 0 10 10"><path d="M1 5.15 3.65 8 9 1.55" /></svg>{/if}</span>
  {#if label}<span class="checkbox-label">{label}</span>{/if}
</label>

<style>
  .mac-checkbox{position:relative;display:inline-flex;align-items:center;gap:8px;min-height:24px;color:var(--rw-text);font-size:12px;font-weight:450;cursor:pointer;user-select:none}
  .mac-checkbox input{position:absolute;width:1px!important;height:1px!important;margin:-1px!important;padding:0!important;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0!important;opacity:0}
  .checkbox-box{display:grid;place-items:center;width:16px;height:16px;flex:0 0 16px;border:1px solid color-mix(in srgb,var(--rw-text) 28%,transparent);border-radius:5px;background:color-mix(in srgb,var(--rw-content) 86%,var(--rw-text) 4%);box-shadow:inset 0 .5px rgba(255,255,255,.75),0 .5px 1px rgba(0,0,0,.08);transition:background-color 140ms ease,border-color 140ms ease,box-shadow 140ms ease}
  .mac-checkbox input:checked + .checkbox-box{border-color:var(--rw-accent);background:var(--rw-accent);box-shadow:inset 0 1px rgba(255,255,255,.3),0 .5px 1px rgba(0,0,0,.14)}
  .mac-checkbox input:checked + .checkbox-box :global(.liquid-visual-layer),.checkbox-box.mixed :global(.liquid-visual-layer){display:none}
  .mac-checkbox input:focus-visible + .checkbox-box{outline:2px solid color-mix(in srgb,var(--rw-accent) 52%,transparent);outline-offset:2px}
  .mac-checkbox input:disabled + .checkbox-box{opacity:.46}
  .checkbox-box.mixed{border-color:var(--rw-accent);background:var(--rw-accent)}
  .checkbox-box.mixed::after{width:8px;height:2px;border-radius:1px;background:#fff;content:""}
  .checkmark{display:block;width:10px;height:10px;overflow:visible;filter:none}.checkmark path{fill:none;stroke:#fff;stroke-width:1.9;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke}
  .checkbox-label{line-height:16px}
  .mac-checkbox.disabled{cursor:not-allowed;opacity:.62}
  @media(prefers-reduced-motion:reduce){.checkbox-box{transition:none}}
</style>

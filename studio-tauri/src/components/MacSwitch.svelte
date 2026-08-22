<script lang="ts">
  export let checked = false;
  export let disabled = false;
  export let label = "";
  export let onChange: (checked: boolean) => void = () => {};

  function update(event: Event) {
    checked = (event.currentTarget as HTMLInputElement).checked;
    onChange(checked);
  }
</script>

<label class="mac-switch" class:disabled>
  <input type="checkbox" role="switch" {checked} {disabled} aria-label={label || undefined} onchange={update} />
  <span class="switch-track" aria-hidden="true"><span class="switch-thumb liquid-control"></span></span>
  {#if label}<span>{label}</span>{/if}
</label>

<style>
  .mac-switch{display:inline-flex;align-items:center;gap:8px;min-height:24px;color:var(--rw-text);font-size:12px;cursor:pointer;user-select:none}
  .mac-switch input{position:absolute;width:1px!important;height:1px!important;overflow:hidden;clip:rect(0 0 0 0);opacity:0}
  .switch-track{position:relative;display:block;width:38px;height:22px;flex:0 0 38px;border:1px solid color-mix(in srgb,var(--rw-text) 16%,transparent);border-radius:11px;background:color-mix(in srgb,var(--rw-text) 18%,transparent);box-shadow:inset 0 .5px rgba(0,0,0,.12),inset 0 1px rgba(255,255,255,.54);transition:background-color 160ms ease,border-color 160ms ease}
  .switch-thumb{position:absolute;z-index:2;top:1px;left:1px;width:18px;height:18px;overflow:hidden;border:.5px solid var(--rw-glass-border);border-radius:50%;background:transparent;box-shadow:0 1px 3px rgba(0,0,0,.25),inset 0 .5px rgba(255,255,255,.92),inset 0 -1px var(--rw-glass-dark-edge);transition:transform 180ms cubic-bezier(.2,.78,.2,1.12)}
  .mac-switch input:checked + .switch-track{border-color:color-mix(in srgb,var(--rw-accent) 80%,transparent);background:var(--rw-accent)}
  .mac-switch input:checked + .switch-track .switch-thumb{transform:translateX(16px)}
  .mac-switch input:focus-visible + .switch-track{outline:2px solid color-mix(in srgb,var(--rw-accent) 52%,transparent);outline-offset:2px}
  .mac-switch input:disabled + .switch-track{opacity:.5}.mac-switch.disabled{cursor:not-allowed;opacity:.62}
  @media(prefers-reduced-motion:reduce){.switch-track,.switch-thumb{transition:none}}
</style>

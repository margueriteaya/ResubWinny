<script lang="ts">
  import { ChevronsUpDown } from "@lucide/svelte";
  import { onMount } from "svelte";

  export let value = "";
  export let options: { value: string; label: string }[] = [];
  export let label = "";
  export let disabled = false;
  export let onChange: (value: string) => void = () => {};
  export let onOpen: () => void | Promise<void> = () => {};

  let root: HTMLDivElement;
  let trigger: HTMLButtonElement;
  let open = false;
  let itemButtons: HTMLButtonElement[] = [];
  $: selectedLabel = options.find((option) => option.value === value)?.label ?? value;

  function close(restoreFocus = false) {
    open = false;
    if (restoreFocus) requestAnimationFrame(() => trigger?.focus());
  }

  function choose(next: string) {
    value = next;
    onChange(next);
    close(true);
  }

  async function openMenu() {
    if (disabled || open) return;
    await onOpen();
    open = true;
  }

  function focusSelected(direction: "first" | "last" | "selected" = "selected") {
    requestAnimationFrame(() => {
      const index = direction === "first"
        ? 0
        : direction === "last"
          ? options.length - 1
          : Math.max(0, options.findIndex((option) => option.value === value));
      itemButtons[index]?.focus();
    });
  }

  function toggleMenu() {
    if (open) close(true);
    else void openMenu();
  }

  function triggerKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      void openMenu().then(() => focusSelected(event.key === "ArrowUp" ? "last" : "selected"));
    } else if (event.key === "Escape") close();
  }

  function menuKeydown(event: KeyboardEvent) {
    const index = itemButtons.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const direction = event.key === "ArrowDown" ? 1 : -1;
      itemButtons[(index + direction + itemButtons.length) % itemButtons.length]?.focus();
    } else if (event.key === "Home") {
      event.preventDefault(); itemButtons[0]?.focus();
    } else if (event.key === "End") {
      event.preventDefault(); itemButtons[itemButtons.length - 1]?.focus();
    } else if (event.key === "Escape") {
      event.preventDefault(); close(true);
    }
  }

  onMount(() => {
    const dismiss = (event: PointerEvent) => {
      if (open && !root.contains(event.target as Node)) close();
    };
    document.addEventListener("pointerdown", dismiss, { capture: true });
    return () => document.removeEventListener("pointerdown", dismiss, { capture: true });
  });
</script>

<div class:open class="popup-button" bind:this={root}>
  <button class="popup-trigger liquid-control" bind:this={trigger} type="button" aria-label={label} aria-haspopup="listbox" aria-expanded={open} {disabled} onclick={toggleMenu} onkeydown={triggerKeydown}>
    <span class="popup-label">{selectedLabel}</span><ChevronsUpDown size={13} strokeWidth={1.8} />
  </button>
  {#if open}
    <div class="popup-menu" role="listbox" aria-label={label} tabindex="-1" onkeydown={menuKeydown}>
      {#each options as option, index (option.value)}
        <button bind:this={itemButtons[index]} type="button" role="option" aria-selected={option.value === value} class:selected={option.value === value} onclick={() => choose(option.value)}><span class="popup-check" aria-hidden="true">{#if option.value === value}<svg viewBox="0 0 10 10"><path d="M1 5.15 3.65 8 9 1.55" /></svg>{/if}</span><span>{option.label}</span></button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .popup-button{position:relative;width:100%}.popup-trigger{display:flex;align-items:center;justify-content:space-between;gap:8px;width:100%;height:36px;padding:0 10px 0 12px;overflow:hidden;border:.5px solid var(--rw-glass-border);border-radius:18px;color:var(--rw-text);background:transparent;box-shadow:var(--rw-control-shadow);backdrop-filter:blur(18px) saturate(1.28) brightness(1.03);-webkit-backdrop-filter:blur(18px) saturate(1.28) brightness(1.03);font-size:12px;line-height:16px;text-align:left;transition:background-color 160ms ease,box-shadow 160ms ease}.popup-trigger:hover:not(:disabled){background:var(--rw-glass-control-hover)}.popup-label{position:relative;z-index:2;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.popup-trigger :global(svg){position:relative;z-index:2;display:block;flex:0 0 13px}.popup-trigger:focus:not(:focus-visible){outline:0;box-shadow:var(--rw-control-shadow)!important}.popup-trigger:focus-visible{outline:2px solid color-mix(in srgb,var(--rw-accent) 52%,transparent);outline-offset:2px}.popup-menu{position:absolute;z-index:50;top:42px;right:0;display:grid;gap:2px;min-width:max(100%,180px);padding:5px;overflow:hidden;border:.5px solid var(--rw-glass-border);border-radius:12px;background:color-mix(in srgb,var(--rw-content) 78%,var(--rw-glass-shell));box-shadow:0 10px 36px rgba(0,0,0,.24),inset 0 1px rgba(255,255,255,.62),inset 0 -1px var(--rw-glass-dark-edge);backdrop-filter:blur(40px) saturate(1.34) brightness(1.03);-webkit-backdrop-filter:blur(40px) saturate(1.34) brightness(1.03);transform-origin:top right;animation:popup-open 190ms cubic-bezier(.22,1,.36,1)}.popup-menu button{position:relative;z-index:2;display:grid;grid-template-columns:16px minmax(0,1fr);align-items:center;gap:7px;width:100%;height:28px;padding:0 7px;border:0;border-radius:6px;color:var(--rw-text);background:transparent;font-size:12px;line-height:16px;text-align:left}.popup-menu button:hover,.popup-menu button:focus-visible{color:#fff;background:var(--rw-accent)}.popup-menu button:focus-visible{outline:none}.popup-check{display:grid;place-items:center;width:16px;height:16px}.popup-check svg{display:block;width:10px;height:10px;overflow:visible}.popup-check path{fill:none;stroke:currentColor;stroke-width:1.9;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke}@keyframes popup-open{from{opacity:0;transform:translateY(-5px) scale(.96)}to{opacity:1;transform:none}}@media(prefers-reduced-motion:reduce){.popup-menu{animation:none}}
</style>

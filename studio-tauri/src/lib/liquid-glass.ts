const surfaceSelector = [
  ".titlebar-icon",
  ".icon-button",
  ".outline",
  ".secondary",
  ".reset",
  ".resume-button",
  ".path-button",
  ".primary-button",
  ".export-button",
  ".save",
  ".save-mapping",
  ".add",
  ".player-button",
  ".timeline-tools button",
  ".language-trigger",
  ".select",
  ".mac-segmented:not(.toolbar)",
  ".mac-segmented.toolbar button",
  ".mac-slider .slider-thumb",
  ".popup-trigger",
  ".zoom-row button",
  ".mac-switch .switch-track",
  ".mac-switch .switch-thumb",
  ".mac-checkbox .checkbox-box",
  ".popup-menu",
  ".approved-secondary",
  ".approved-icon",
].join(",");

const refractiveSurfaceSelector = [
  ".titlebar-icon",
  ".icon-button",
  ".outline",
  ".secondary",
  ".reset",
  ".resume-button",
  ".path-button",
  ".quiet-button",
  ".tool-button",
  ".player-button",
  ".timeline-tools button",
  ".pane-toggle",
  ".approved-secondary",
  ".approved-icon",
  // The regular segmented shell already owns the backdrop blur. Applying an
  // SVG backdrop filter to both it and its indicator creates a nested WebView2
  // compositor layer that can wash out the entire content plane.
  ".mac-segmented.toolbar button",
  ".mac-segmented .segment-indicator",
  ".popup-trigger",
  ".zoom-row button",
  ".mac-switch .switch-track",
  ".mac-switch .switch-thumb",
  ".mac-checkbox .checkbox-box",
  ".popup-menu",
].join(",");

const interactiveSurfaceSelector = [
  ".titlebar-icon",
  ".icon-button",
  ".outline",
  ".secondary",
  ".reset",
  ".resume-button",
  ".path-button",
  ".primary-button",
  ".export-button",
  ".save",
  ".save-mapping",
  ".add",
  ".quiet-button",
  ".tool-button",
  ".player-button",
  ".timeline-tools button",
  ".pane-toggle",
  ".sidebar-navigation button",
  ".mac-segmented.toolbar button",
  ".popup-trigger",
  ".zoom-row button",
  ".mac-switch .switch-track",
  ".mac-checkbox .checkbox-box",
  ".approved-secondary",
  ".approved-icon",
].join(",");

declare global {
  interface Window {
    __liquidGlassInstalled__?: boolean;
  }
}

function prepareSurfaces(
  filters: LiquidDisplacementController,
  root: ParentNode = document,
) {
  root.querySelectorAll<HTMLElement>(surfaceSelector).forEach((element) => {
    if (element.closest(".native-preview")) return;
    element.classList.add("liquid-control");
  });
  root.querySelectorAll<HTMLElement>(interactiveSurfaceSelector).forEach((element) => {
    if (element.closest(".native-preview")) return;
    element.dataset.liquidInteractive = "true";
  });
  root.querySelectorAll<HTMLElement>(refractiveSurfaceSelector).forEach((element) => {
    prepareRefractiveSurface(filters, element);
  });
}

function prepareRefractiveSurface(
  filters: LiquidDisplacementController,
  element: HTMLElement,
) {
  if (element.dataset.liquidLensHost || element.closest(".native-preview")) return;
  if (element.closest(".player-shell") && !element.closest(".player-controls, .tabs")) return;
  // Refractive-only selectors such as tool buttons still need to establish the
  // containing block used by the absolutely positioned visual layer.
  element.classList.add("liquid-control");
  element.dataset.liquidLensHost = "true";
  const lens = document.createElement("span");
  lens.className = "liquid-visual-layer";
  lens.setAttribute("aria-hidden", "true");
  element.prepend(lens);
  filters.observe(lens);
}

export function installLiquidGlass() {
  if (window.__liquidGlassInstalled__) return () => {};
  window.__liquidGlassInstalled__ = true;
  const filters = createLiquidDisplacementController();
  const mount = document.getElementById("app");
  let bootFrame = 0;
  const boot = () => {
    if (!mount?.firstElementChild) {
      bootFrame = requestAnimationFrame(boot);
      return;
    }
    prepareSurfaces(filters);
  };
  boot();

  const tooltip = document.createElement("div");
  tooltip.className = "rw-tooltip";
  tooltip.setAttribute("role", "tooltip");
  tooltip.hidden = true;
  document.body.append(tooltip);
  let tooltipHost: HTMLElement | null = null;
  let tooltipTimer = 0;
  let pointerFrame = 0;
  let pointerHost: HTMLElement | null = null;
  let pointerX = 50;
  let pointerY = 50;

  const hideTooltip = () => {
    if (tooltipTimer) window.clearTimeout(tooltipTimer);
    tooltipTimer = 0;
    tooltipHost = null;
    tooltip.hidden = true;
  };
  const showTooltip = (host: HTMLElement) => {
    const label = host.dataset.tooltip?.trim();
    if (!label || !host.isConnected) return;
    tooltipHost = host;
    tooltip.textContent = label;
    tooltip.hidden = false;
    const hostBounds = host.getBoundingClientRect();
    const tooltipBounds = tooltip.getBoundingClientRect();
    const gap = 8;
    const left = Math.min(
      window.innerWidth - tooltipBounds.width - gap,
      Math.max(gap, hostBounds.left + (hostBounds.width - tooltipBounds.width) / 2),
    );
    const below = hostBounds.bottom + gap;
    const top = below + tooltipBounds.height <= window.innerHeight - gap
      ? below
      : Math.max(gap, hostBounds.top - tooltipBounds.height - gap);
    tooltip.style.left = `${Math.round(left)}px`;
    tooltip.style.top = `${Math.round(top)}px`;
  };
  const scheduleTooltip = (host: HTMLElement, delay: number) => {
    if (tooltipHost === host && !tooltip.hidden) return;
    hideTooltip();
    tooltipHost = host;
    tooltipTimer = window.setTimeout(() => showTooltip(host), delay);
  };

  const clearPointerHighlight = (host: HTMLElement | null = pointerHost) => {
    if (!host) return;
    host.style.removeProperty("--rw-liquid-pointer-x");
    host.style.removeProperty("--rw-liquid-pointer-y");
    host.querySelector<HTMLElement>(".liquid-visual-layer")?.style.removeProperty("--rw-liquid-pointer-x");
    host.querySelector<HTMLElement>(".liquid-visual-layer")?.style.removeProperty("--rw-liquid-pointer-y");
    if (pointerHost === host) pointerHost = null;
  };
  const applyPointerHighlight = () => {
    pointerFrame = 0;
    if (!pointerHost?.isConnected) return;
    pointerHost.style.setProperty("--rw-liquid-pointer-x", `${pointerX}%`);
    pointerHost.style.setProperty("--rw-liquid-pointer-y", `${pointerY}%`);
    const layer = pointerHost.querySelector<HTMLElement>(".liquid-visual-layer");
    layer?.style.setProperty("--rw-liquid-pointer-x", `${pointerX}%`);
    layer?.style.setProperty("--rw-liquid-pointer-y", `${pointerY}%`);
  };
  const pointerMove = (event: PointerEvent) => {
    if (event.pointerType === "touch" || document.documentElement.dataset.glassStatic === "true") return;
    const host = (event.target as Element | null)?.closest<HTMLElement>(
      '[data-liquid-interactive="true"].liquid-control',
    );
    if (!host || surfaceDisabled(host)) {
      clearPointerHighlight();
      return;
    }
    const bounds = host.getBoundingClientRect();
    if (bounds.width <= 0 || bounds.height <= 0) return;
    if (pointerHost && pointerHost !== host) clearPointerHighlight(pointerHost);
    pointerHost = host;
    pointerX = Math.max(0, Math.min(100, ((event.clientX - bounds.left) / bounds.width) * 100));
    pointerY = Math.max(0, Math.min(100, ((event.clientY - bounds.top) / bounds.height) * 100));
    if (!pointerFrame) pointerFrame = requestAnimationFrame(applyPointerHighlight);
  };
  const pointerOut = (event: PointerEvent) => {
    const host = (event.target as Element | null)?.closest<HTMLElement>(
      '[data-liquid-interactive="true"].liquid-control',
    );
    if (host && !host.contains(event.relatedTarget as Node | null)) clearPointerHighlight(host);
  };

  const observer = new MutationObserver((records) => {
    for (const record of records) {
      for (const node of record.addedNodes) {
        if (node instanceof Element) {
          if (node.matches(surfaceSelector)) node.classList.add("liquid-control");
          if (node instanceof HTMLElement && node.matches(interactiveSurfaceSelector)) {
            node.dataset.liquidInteractive = "true";
          }
          if (node instanceof HTMLElement && node.matches(refractiveSurfaceSelector)) {
            prepareRefractiveSurface(filters, node);
          }
          prepareSurfaces(filters, node);
        }
      }
      for (const node of record.removedNodes) {
        if (node instanceof Element) filters.detach(node);
      }
    }
  });
  observer.observe(document.body, { childList: true, subtree: true });

  const accessibilityMedia = [
    window.matchMedia("(prefers-reduced-motion: reduce)"),
    window.matchMedia("(forced-colors: active)"),
  ];
  const updateMaterialMode = () => {
    const supportsBackdrop = typeof CSS !== "undefined" && CSS.supports("backdrop-filter", "blur(1px)");
    const staticMode = !supportsBackdrop || accessibilityMedia.some((query) => query.matches);
    document.documentElement.dataset.glassStatic = staticMode ? "true" : "false";
    document.documentElement.dataset.glassRefraction = filters.supported ? "svg" : "fallback";
    filters.setEnabled(!staticMode);
    if (staticMode) clearPointerHighlight();
  };
  accessibilityMedia.forEach((query) => query.addEventListener("change", updateMaterialMode));
  updateMaterialMode();

  const tooltipPointerOver = (event: PointerEvent) => {
    const host = (event.target as Element | null)?.closest<HTMLElement>("[data-tooltip]");
    if (host && !host.matches(":disabled")) scheduleTooltip(host, 480);
  };
  const tooltipPointerOut = (event: PointerEvent) => {
    const host = (event.target as Element | null)?.closest<HTMLElement>("[data-tooltip]");
    if (host && !host.contains(event.relatedTarget as Node | null)) hideTooltip();
  };
  const tooltipFocusIn = (event: FocusEvent) => {
    const host = (event.target as Element | null)?.closest<HTMLElement>("[data-tooltip]");
    if (host && !host.matches(":disabled")) scheduleTooltip(host, 120);
  };
  const surfaceDisabled = (surface: HTMLElement) => {
    if (surface.matches(":disabled")) return true;
    const composite = surface.closest<HTMLElement>(".mac-slider, .mac-switch, .mac-checkbox");
    return Boolean(composite?.querySelector(":disabled"));
  };
  const liquidSurface = (target: EventTarget | null) => {
    const element = target as Element | null;
    const sliderThumb = element?.closest(".mac-slider")?.querySelector<HTMLElement>(".slider-thumb");
    const switchTrack = element?.closest(".mac-switch")?.querySelector<HTMLElement>(".switch-track");
    const checkbox = element?.closest(".mac-checkbox")?.querySelector<HTMLElement>(".checkbox-box");
    return sliderThumb
      ?? switchTrack
      ?? checkbox
      ?? element?.closest<HTMLElement>('[data-liquid-interactive="true"].liquid-control')
      ?? null;
  };
  const pointerDown = (event: PointerEvent) => {
    if (event.button !== 0) return;
    const surface = liquidSurface(event.target);
    if (!surface || surfaceDisabled(surface)) return;
    delete surface.dataset.liquidReleasing;
    surface.dataset.liquidPressed = "true";
  };
  const releasePressedSurfaces = () => {
    document.querySelectorAll<HTMLElement>(".liquid-control[data-liquid-pressed]").forEach((surface) => {
      delete surface.dataset.liquidPressed;
      surface.dataset.liquidReleasing = "true";
    });
  };
  const keyDown = (event: KeyboardEvent) => {
    if (event.repeat || (event.key !== " " && event.key !== "Enter")) return;
    const surface = liquidSurface(event.target);
    if (!surface || surfaceDisabled(surface)) return;
    delete surface.dataset.liquidReleasing;
    surface.dataset.liquidPressed = "true";
  };
  const keyUp = (event: KeyboardEvent) => {
    if (event.key === " " || event.key === "Enter") releasePressedSurfaces();
  };
  const animationEnd = (event: AnimationEvent) => {
    const surface = (event.target as Element | null)?.closest<HTMLElement>('.liquid-control[data-liquid-releasing="true"]');
    if (surface && event.animationName === "rw-liquid-release") delete surface.dataset.liquidReleasing;
  };

  document.addEventListener("pointerover", tooltipPointerOver, { passive: true });
  document.addEventListener("pointerout", tooltipPointerOut, { passive: true });
  document.addEventListener("pointermove", pointerMove, { passive: true });
  document.addEventListener("pointerout", pointerOut, { passive: true });
  document.addEventListener("pointerdown", pointerDown, { passive: true });
  document.addEventListener("keydown", keyDown);
  document.addEventListener("keyup", keyUp);
  document.addEventListener("animationend", animationEnd);
  document.addEventListener("focusin", tooltipFocusIn);
  document.addEventListener("focusout", hideTooltip);
  document.addEventListener("scroll", hideTooltip, { capture: true, passive: true });
  window.addEventListener("pointerup", releasePressedSurfaces, { passive: true });
  window.addEventListener("pointercancel", releasePressedSurfaces, { passive: true });

  return () => {
    observer.disconnect();
    filters.destroy();
    accessibilityMedia.forEach((query) => query.removeEventListener("change", updateMaterialMode));
    document.removeEventListener("pointerover", tooltipPointerOver);
    document.removeEventListener("pointerout", tooltipPointerOut);
    document.removeEventListener("pointermove", pointerMove);
    document.removeEventListener("pointerout", pointerOut);
    document.removeEventListener("pointerdown", pointerDown);
    document.removeEventListener("keydown", keyDown);
    document.removeEventListener("keyup", keyUp);
    document.removeEventListener("animationend", animationEnd);
    document.removeEventListener("focusin", tooltipFocusIn);
    document.removeEventListener("focusout", hideTooltip);
    document.removeEventListener("scroll", hideTooltip, { capture: true });
    window.removeEventListener("pointerup", releasePressedSurfaces);
    window.removeEventListener("pointercancel", releasePressedSurfaces);
    if (bootFrame) cancelAnimationFrame(bootFrame);
    if (pointerFrame) cancelAnimationFrame(pointerFrame);
    clearPointerHighlight();
    hideTooltip();
    tooltip.remove();
    window.__liquidGlassInstalled__ = false;
  };
}
import {
  createLiquidDisplacementController,
  type LiquidDisplacementController,
} from "./liquid-displacement";

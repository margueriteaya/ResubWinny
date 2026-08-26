import type { TransitionConfig } from "svelte/transition";

function reducedMotion() {
  return typeof window !== "undefined"
    && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function fluidEase(value: number) {
  return 1 - Math.pow(1 - value, 3);
}

export function liquidPopover(_node: Element): TransitionConfig {
  return {
    duration: reducedMotion() ? 0 : 200,
    easing: fluidEase,
    css: (progress, inverse) => `
      opacity: ${progress};
      transform: translate3d(0, ${(-5 * inverse).toFixed(3)}px, 0)
        scale(${(0.965 + 0.035 * progress).toFixed(4)});
    `,
  };
}

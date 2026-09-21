import type { TransitionConfig } from "svelte/transition";

function reducedMotion() {
  return typeof window !== "undefined"
    && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function fluidEase(value: number) {
  return 1 - Math.pow(1 - value, 3);
}

export function liquidPopover(_node: Element): TransitionConfig {
  const keyboardOpen = Boolean(_node.closest('[data-keyboard-open="true"]'));
  const reduce = reducedMotion();
  return {
    duration: keyboardOpen ? 0 : reduce ? 120 : 200,
    easing: fluidEase,
    css: (progress, inverse) => `
      opacity: ${progress};
      transform: ${reduce
        ? "none"
        : `translate3d(0, ${(-5 * inverse).toFixed(3)}px, 0) scale(${(0.965 + 0.035 * progress).toFixed(4)})`};
    `,
  };
}

export function liquidDisclosure(_node: Element): TransitionConfig {
  const reduce = reducedMotion();
  return {
    duration: reduce ? 120 : 170,
    easing: fluidEase,
    css: (progress, inverse) => `
      opacity: ${progress};
      transform: ${reduce ? "none" : `translate3d(0, ${(-4 * inverse).toFixed(3)}px, 0)`};
    `,
  };
}

export function noticeIn(_node: Element): TransitionConfig {
  const reduce = reducedMotion();
  return {
    duration: reduce ? 120 : 240,
    easing: fluidEase,
    css: (progress, inverse) => `
      opacity: ${progress};
      transform: ${reduce ? "none" : `translate3d(0, ${(8 * inverse).toFixed(3)}px, 0) scale(${(0.985 + 0.015 * progress).toFixed(4)})`};
    `,
  };
}

export function noticeOut(_node: Element): TransitionConfig {
  const reduce = reducedMotion();
  return {
    duration: reduce ? 120 : 170,
    easing: fluidEase,
    css: (progress, inverse) => `
      opacity: ${progress};
      transform: ${reduce ? "none" : `translate3d(0, ${(8 * inverse).toFixed(3)}px, 0) scale(${(0.985 + 0.015 * progress).toFixed(4)})`};
    `,
  };
}

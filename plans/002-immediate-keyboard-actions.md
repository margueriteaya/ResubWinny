# 002 — Keep keyboard actions immediate

- **Status**: DONE
- **Commit**: 1249362
- **Severity**: HIGH
- **Category**: Purpose and frequency
- **Estimated scope**: 3 files, small

## Problem

Arrow, Home, and End keys in `studio-tauri/src/components/MacSegmentedControl.svelte:20-29` trigger the same 240ms indicator transition as pointer input. Arrow keys in `studio-tauri/src/components/PopupButton.svelte:52-56` open a menu through `liquidPopover`, which currently animates for 200ms in `studio-tauri/src/lib/motion.ts:12-20`.

## Target

Pointer-opened menus retain a 200ms `translate3d(0,-5px,0) scale(.965)` entrance. Keyboard-opened menus use duration `0`. Segmented controls set a one-frame keyboard-navigation state that disables the indicator transform transition for the key-driven change.

## Repo conventions to follow

Keep the existing Svelte transition helper in `studio-tauri/src/lib/motion.ts` and the component-local CSS pattern.

## Steps

1. Mark PopupButton opens by pointer or keyboard and expose the source on the menu ancestor.
2. Return a zero-duration config from `liquidPopover` for keyboard opens.
3. Add a transient keyboard-navigation class around segmented keyboard selection and disable only the indicator transform transition for that frame.

## Boundaries

- Keep pointer animations and focus behavior.
- Do not change selection semantics.

## Verification

- Run `npm run build` in `studio-tauri`.
- Open a popup with ArrowDown and confirm it appears immediately with focus on the selected item.
- Click the same popup and confirm the anchored 200ms entrance remains.
- Done when keyboard-triggered changes show no positional animation.

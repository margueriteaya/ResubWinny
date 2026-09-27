# 001 — Make press feedback interruptible

- **Status**: DONE
- **Commit**: 1249362
- **Severity**: HIGH
- **Category**: Interruptibility
- **Estimated scope**: 2 files, small

## Problem

`studio-tauri/src/macos27.css:224-227` releases every liquid control with `rw-liquid-release`, a fixed 300ms keyframe. `studio-tauri/src/lib/liquid-glass.ts:292-317` deletes and recreates the release state on repeated input, so the motion restarts instead of continuing from the current visual value.

```css
.liquid-control[data-liquid-releasing="true"] { animation:rw-liquid-release var(--rw-motion-spring) var(--rw-ease-spring) both; }
```

## Target

Use an 80ms press transition and a 170ms `cubic-bezier(.2,.8,.2,1)` release transition on `transform`. Removing the pressed attribute must retarget from the current transform. Remove the release keyframe and its animation-end bookkeeping.

## Repo conventions to follow

Motion tokens live in `studio-tauri/src/macos27.css:34-41`. Use `--rw-motion-press`, `--rw-motion-responsive`, and `--rw-ease-out`.

## Steps

1. Add `transform 170ms var(--rw-ease-out)` to the base liquid-control transition.
2. Keep the pressed-state transform at `80ms`.
3. Remove `data-liquid-releasing`, `rw-liquid-release`, and the animation-end listener.

## Boundaries

- Do not change the liquid glass rendering layers.
- Do not add dependencies.

## Verification

- Run `npm run build` in `studio-tauri`.
- Rapidly press a toolbar button and confirm every reversal starts from its current scale.
- Enable reduced motion and confirm press feedback remains immediate without rebound.
- Done when no `rw-liquid-release` reference remains.

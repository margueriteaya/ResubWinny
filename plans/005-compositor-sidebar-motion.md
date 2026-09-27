# 005 — Keep sidebar and inspector motion off layout properties

- **Status**: DONE
- **Commit**: 1249362
- **Severity**: HIGH
- **Category**: Performance
- **Estimated scope**: 2 files, medium

## Problem

`studio-tauri/src/macos27.css` and `studio-tauri/src/components/AppSidebar.svelte` animate grid columns, width, height, padding, margin, and positional offsets when sidebars collapse. These properties recalculate layout throughout the workbench.

## Target

The main sidebar changes width immediately and limits its internal feedback to opacity and transform. Narrow task inspectors keep a stable width and slide with `transform` for 240ms using `cubic-bezier(.22,1,.36,1)`.

## Repo conventions to follow

Use `--rw-motion-fluid`, `--rw-motion-responsive`, `--rw-motion-fast`, `--rw-ease-fluid`, and `--rw-ease-out` from `studio-tauri/src/macos27.css`.

## Steps

1. Remove the animated sidebar custom grid property and shell clip-path transition.
2. Remove width, height, padding, margin, and positional transitions from sidebar descendants.
3. Keep narrow task inspector widths stable and express collapsed state with translateX.

## Boundaries

- Preserve the 48px collapsed affordance.
- Keep native preview ancestors free of transforms at desktop sizes.

## Verification

- Run `npm run build` in `studio-tauri`.
- Toggle the main sidebar and confirm content reflows immediately without a frame-by-frame layout animation.
- At a viewport below 980px, toggle both inspectors and confirm they slide while leaving a 48px affordance.
- Done when no collapse transition animates a layout property.

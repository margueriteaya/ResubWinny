# 004 — Respect motion, transparency, and contrast preferences

- **Status**: DONE
- **Commit**: 1249362
- **Severity**: MEDIUM
- **Category**: Accessibility and materials
- **Estimated scope**: 2 files, medium

## Problem

`studio-tauri/src/macos27.css:328-343` has no `prefers-reduced-transparency` or `prefers-contrast` treatment. Its reduced-motion rule collapses every transition to `.01ms`, including useful color and opacity feedback. `--rw-muted:#818187` is also too faint for 10–12px text over the light surfaces.

## Target

- Raise the light muted token to `#696970` and dark muted token to `#aaaab2`.
- Reduced motion removes positional transforms and long-running motion while retaining 120ms opacity and color feedback.
- Reduced transparency makes shell, status, popup, and control surfaces near-solid and disables blur.
- Increased contrast strengthens borders and surface opacity.
- `studio-tauri/src/lib/liquid-glass.ts` treats reduced transparency as static material mode.

## Repo conventions to follow

Keep all material tokens in `studio-tauri/src/macos27.css`; keep runtime media-query handling beside the existing accessibility queries in `studio-tauri/src/lib/liquid-glass.ts:249-262`.

## Steps

1. Update muted color tokens.
2. Replace the global `.01ms` rule with targeted reduced-motion rules that preserve color and opacity feedback.
3. Add reduced-transparency and increased-contrast media queries.
4. Add reduced transparency to the runtime static-material query list.

## Boundaries

- Preserve the existing macOS and Liquid Glass design system.
- Keep forced-colors behavior.

## Verification

- Run `npm run build` in `studio-tauri`.
- Emulate reduced motion, reduced transparency, increased contrast, and dark mode in turn.
- Confirm navigation state remains legible and large movement stops under reduced motion.
- Done when all three user preferences have visible, readable outcomes.

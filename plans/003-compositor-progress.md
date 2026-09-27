# 003 — Move progress animation to the compositor

- **Status**: DONE
- **Commit**: 1249362
- **Severity**: HIGH
- **Category**: Performance
- **Estimated scope**: 2 files, small

## Problem

`studio-tauri/src/components/StatusBar.svelte:23` writes progress into inline `width`, and `studio-tauri/src/macos27.css:301` transitions that width on every update.

```svelte
<i><b style={`width:${progress}%`}></b></i>
```

## Target

Keep the bar at `width:100%`, set `transform-origin:left center`, and write progress as `transform:scaleX(progress / 100)`. Transition transform for 170ms with `cubic-bezier(.2,.8,.2,1)`.

## Repo conventions to follow

Use `--rw-motion-responsive` and `--rw-ease-out` from `studio-tauri/src/macos27.css:34-41`.

## Steps

1. Replace the inline width with a clamped scaleX transform.
2. Replace the width transition with a transform transition and set the origin.

## Boundaries

- Keep labels and progress calculation unchanged.
- Do not animate the numeric text.

## Verification

- Run `npm run build` in `studio-tauri`.
- Inspect the bar during export and confirm its computed width stays constant while transform changes.
- Done when no progress bar width transition remains.

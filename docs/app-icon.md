# ResubWinny app icon

The canonical Apple-platform source is layered artwork for Icon Composer. It
must not contain a platform mask, simulated Liquid Glass, bevels, glows,
blurred edges, specular highlights, or inter-layer shadows.

## Source layers

All source artwork is a 1024 x 1024 square and lives in
`studio-tauri/src-tauri/icons/source/apple/`:

- `01-background.svg`: full-bleed opaque background.
- `02-broadcast-plane.svg`: the 16:9 broadcast plane foreground layer.
- `03-captions.svg`: caption regions, ruby cells, and the independent side
  region.
- `04-composite-preview.svg`: unmasked preview only; do not import it as a
  layered Icon Composer source.

Import the first three layers into Icon Composer in that order. Keep the two
foreground SVGs fully opaque and tune translucency, refraction, specular
highlights, and shadows in Icon Composer. The system supplies platform masks
and dynamic effects for Default, Dark, Clear, and Tinted appearances.

The flat Tauri fallback is `icons/source/flat-app-icon.svg`. Its rounded
background is intentional for platforms that do not apply an Apple system
mask. It is not an Icon Composer source layer.

## Geometry

- Canvas: 1024 x 1024.
- Broadcast plane: 620 x 348.75, an exact 16:9 ratio, centered on the canvas.
- Primary content remains within the circular watchOS crop.
- Foreground edges are solid and unfeathered.
- No text, platform hardware, screenshots, or replicated application UI is
  included.

## Regeneration

From `studio-tauri/`, regenerate the cross-platform Tauri fallback icons:

```powershell
npm run tauri -- icon src-tauri/icons/source/flat-app-icon.svg
```

## References

- Apple Human Interface Guidelines: App icons
  <https://developer.apple.com/design/human-interface-guidelines/app-icons/>
- Apple Icon Composer
  <https://developer.apple.com/icon-composer/>
- Apple Design Resources license
  <https://developer.apple.com/apple-design-resources-license/>

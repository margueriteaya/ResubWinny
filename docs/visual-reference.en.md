# Visual reference baseline

[简体中文](visual-reference.md) · [繁體中文](visual-reference.zh-TW.md) · [日本語](visual-reference.ja.md) · [English](visual-reference.en.md)

> **Normative notice:** The Simplified Chinese version is the sole authoritative source. The other language versions are synchronized translations; if wording is ambiguous or conflicts, the Simplified Chinese version prevails.

ResubWinny uses libaribcaption's published caption screenshot as the primary
viewer-facing reference for the shared B24/B62 preview profile:

- Source: <https://github.com/xqq/libaribcaption/raw/master/screenshots/screenshot0.png>
- Vendored reference: `third_party/libaribcaption/screenshots/screenshot0.png`
- Dimensions: `1920×1080`
- SHA-256: `3115B9B125AFA7CDF6F41D3D0155476CD18134021CDD05A55C8C65E749A403F6`

It establishes the intended television-facing result: a 1920×1080 logical
caption plane, ARIB-capable font selection, independently positioned text
regions, source foreground/background/stroke colours, and ruby that remains
visually tied to its base text. It is not a B62 transport fixture and does not
license a guess at B62 features absent from that image.

## Implementation contract

The B24 route is authoritative: the project-owned C ABI asks libaribcaption to
produce RGBA directly with `Rounded M+ 1m for ARIB`, ruby and background
enabled, DRCS replacement disabled, merged regions, and a renderer stroke
width of `2.0`. ResubWinny preserves that image through its archive and native
preview compositor; neither the Svelte UI nor a browser text engine redraws
it.

The B62/ARIB-TTML native renderer must target the same viewer-facing
relationships, not a different visual language. Its 2K/4K/8K source coordinates
must map proportionally from their display plane into the actual video-content
viewport; the current 1920×1080 logical plane is only an intermediate texture. It has visual goldens for
horizontal ruby, vertical ruby, vertical punctuation, and the Rounded M+
receiver-baseline black stroke used when a broadcast omits a repeated direct
outline declaration. An explicit `tts:textOutline="none"` remains authoritative.
Pixel-for-pixel
comparison with this B24 screenshot is intentionally not an acceptance test:
the screenshot has no corresponding B62 source TTML, timing, region metadata,
or style payload. New B62 semantics require a lawful source sample plus a
reference capture before being marked verified.

## Review rule

When changing B24 bridge settings, caption-plane composition, font resources,
or B62 text/ruby/stroke layout, review this image alongside the affected PNG
goldens. Do not compensate with WebView CSS, substitute a generic font, or
claim visual parity from a synthetic example alone.

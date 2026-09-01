[简体中文](preview.md) · [繁體中文](preview.zh-TW.md) · [日本語](preview.ja.md) · [English](preview.en.md)

> This is a translation. The Simplified Chinese version is the sole authoritative source.

# Preview contract

Native preview is owned by the Tauri backend and in-process libmpv. The
WebView supplies commands and displays bounded state; it never submits caption
bitmaps or performs caption layout.

`render_at` and `sync_preview_overlay` use the explicit project-time mapping
and return both media and project times. The backend composes the caption plane
from the archive, reports the selected overlay route and capability metadata,
and keeps unsupported B62 features declarative rather than approximating them
with CSS. See [`backend-contract.md`](../backend-contract.md) for the detailed
render profile and route limitations.

For ARIB-TTML with `source_layout`, the backend first produces a bounded intermediate caption texture from the source display-plane
ratios, then maps the complete texture into the actual video-content viewport reported by libmpv. Letterbox or pillarbox bars,
window size, DPI, and fullscreen change only the final transform, not the caption position or area relative to video content.
Old archives without `source_layout` continue through the logical 1920×1080 compatibility path.

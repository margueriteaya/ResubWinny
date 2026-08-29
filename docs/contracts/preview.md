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

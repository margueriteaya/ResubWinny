[简体中文](preview-composition.md) · [繁體中文](preview-composition.zh-TW.md) · [日本語](preview-composition.ja.md) · [English](preview-composition.en.md)

> **Normative note:** Simplified Chinese is the only authoritative version. The other languages are synchronized translations. If wording is ambiguous or conflicts, the Simplified Chinese version controls.

# Preview composition decision

Windows preview uses in-process `libmpv-render`. ResubWinny does not start `mpv.exe` or use a JSON named pipe. macOS and Linux share the caption model, but their native preview backends are not implemented yet.

libmpv video and Rust captions are composed in an offscreen FBO on a dedicated render thread. A parentless, offscreen WGL host that is never shown exists only to provide an OpenGL device context. It does not display video or participate in application window stacking, movement, or resizing. The active path creates no visible or child HWND and has no `wid`, `overlay-add`, or per-source fallback.

Composited RGBA frames are read back through rotating PBO allocations. Each frame is mapped immediately after submission so video does not remain two frames behind libmpv audio. Three WebView2 SharedBuffer pixel slots deliver frames to a WebGL Canvas inside the player component; a control slot carries only sequence, size, and slot state. The Canvas claims the newest ready frame and releases uploaded or stale slots, so per-frame pixels do not pass through Tauri commands, events, or COM copies.

Window movement, pane collapse, and DPI changes affect only the Canvas DOM layout. A ResizeObserver sends the component's physical pixel size to the backend, which resizes only the offscreen FBO. Video and player controls stay in the same WebView content layer and no native window coordinates are tracked. The shared surface is bounded to 1920×1080; libmpv scales larger sources to the component's actual size.

`get_preview_capabilities` reports only the current `libmpv-render` route. `get_preview_render_diagnostics` reports rendered, presented, and dropped frame counts, surface size and cadence, caption texture operations, video aspect ratio, decoder policy, and the latest error. `render_at(time)` remains the bounded archive snapshot operation. `sync_preview_overlay` still queries project time from libmpv media time and updates the Rust caption texture; the WebView does not implement another playback clock or caption layout engine.

The render thread requests `hwdec=auto-safe` and reports libmpv's selected `hwdec-current` when available. This permits compatible copy-back hardware decoding and does not claim zero-copy D3D/ANGLE interoperation. Preview stop and application exit stop and join the render thread, release SharedBuffer objects on the WebView UI STA, and then destroy the hidden WGL host.

`scripts/validate-preview.ps1` uses legally held local recordings supplied through `ARIB_FIXTURE_DIR` for the Windows smoke gate. It covers the first decoded frame, seek, pause and resume, caption texture composition, resize, capture, and bounded shutdown. Private broadcast material is never committed; public regression coverage uses synthetic and protocol tests.

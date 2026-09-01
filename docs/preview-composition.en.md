# Preview composition decision

[简体中文](preview-composition.md) · [繁體中文](preview-composition.zh-TW.md) · [日本語](preview-composition.ja.md) · [English](preview-composition.en.md)

> **Normative notice:** The Simplified Chinese version is the sole authoritative source. The other language versions are synchronized translations; if wording is ambiguous or conflicts, the Simplified Chinese version prevails.

The external `mpv.exe` sidecar is not a product requirement. ResubWinny is
migrating to its own dynamic `libmpv` client ABI. The Windows x86_64 runtime is
bundled as a replaceable LGPL dependency; macOS and Linux use the same ABI
contract with their platform runtime.

| Backend | Platform | Caption fidelity | Main trade-off |
| --- | --- | --- | --- |
| `libmpv-render` | Windows/macOS/Linux | Highest; compose video and libaribcaption RGBA in one surface | Requires libmpv integration and a native texture/surface loop |
| mpv `overlay-add` | mpv platforms | High for low-frequency RGBA frames | Raw pixel upload and mpv-version/VO constraints |
| native window overlay | Windows first | High | HWND/Z-order, resize, DPI and platform-specific code |
| WebView image layer | All | Good for archive inspection | Cannot reliably cover an embedded native child window |
| ASS subtitle track | All | Approximate | Loses exact ARIB character-cell, ruby, DRCS and background semantics |

The selected Windows route is `libmpv-render`. It keeps video and captions in
the same project-owned WGL surface and avoids WebView/native-window stacking
problems. A `wid` plus `overlay-add` client remains only as a per-source
compatibility fallback when the render API or WGL startup fails. ResubWinny
does not start `mpv.exe` or use a JSON named pipe. macOS and Linux will use the
same caption model, but their native render surfaces are not implemented yet.

The typed `get_preview_capabilities` API exposes this choice without making
the UI depend on Windows-specific details. In both the render route and its
client-overlay fallback, the Tauri backend samples mpv time through the typed
`sync_preview_overlay` operation and compares the returned native caption plane
with the previous plane before uploading it. The WebView may schedule that
operation, but does not calculate media time, compose captions, or implement a
fallback clock.
An unchanged caption plane therefore does not result in another raw-pixel IPC
transfer; this keeps long, sparse recordings bounded without preloading their
caption timeline. Changed RGBA planes are written to a per-preview temporary
BGRA file and passed to mpv with its documented `overlay-add` file/offset
arguments; the temporary file is removed when the preview stops. The Tauri
archive reader keeps a forward-only cursor and at most 128 active intervals;
seeking backwards resets that bounded cursor, so it never loads a multi-hour
timeline into memory. mpv time queries have a short timeout; when IPC is not
ready the backend returns `awaiting-player-time` and leaves the last plane in
place rather than inventing a local-clock timeline.

`get_preview_runtime` reports the loaded runtime path and whether that runtime
exports the libmpv render API. On Windows, a complete render API selects the
native render route by default; if a particular source cannot initialise that
route, that preview safely falls back to the client-overlay route. A dedicated
native thread owns the project-owned WGL context, `vo=libmpv` instance and
render context. It creates
the render context before `loadfile`, processes bounded control/resize/time
messages, pumps pending frames at most once per 16 ms, presents on the default
framebuffer, and frees the context while that same WGL context remains current.
This keeps the fragile ABI/lifetime and thread ordering inside the backend.

The same render thread accepts only backend-composed BGRA caption planes. It
uploads a bounded OpenGL texture when a plane changes and blends that texture
after the mpv video frame; the WebView never receives a video frame or performs
caption layout. It asks libmpv for its display-correct video aspect and maps
the caption plane to the resulting letterbox/pillarbox viewport; if the media
parameters are not ready, it safely falls back to the full surface and retries.
The Windows route is available through the typed capability contract once the
runtime exposes the complete render API. It requests `hwdec=auto-safe`, then
reports libmpv's actual `hwdec-current` selection when available. This may use
compatible copy-back acceleration but is not a zero-copy D3D/ANGLE
hardware-decode claim. The real-corpus Windows 4K long gate described below is
complete. Independent 2K/8K gates, reference screenshot comparison, and DPI
review remain release-quality work rather than hidden fallbacks.

`scripts/validate-preview.ps1` is the Windows native smoke gate. It uses a
legal local recording supplied through `ARIB_FIXTURE_DIR`, creates a real WGL
host, loads the bundled libmpv runtime, starts the dedicated render worker, and
requires a non-uniform decoded video framebuffer plus one backend RGBA
caption-texture upload and clear without a worker error. It captures the WGL
back buffer on the same render thread and asserts that the uploaded opaque
green test texture is present in captured RGBA pixels. The `bs4k_test_2.ts`
smoke passed on 2026-07-29. This verifies lifecycle, TS opening, decoded video,
native present, texture blend, and bounded framebuffer readback. The smoke exercises a 3840×2160 HEVC source,
captures the blended plane at 1920×1080, then resizes the native surface to
3840×2160 and requires retained dimensions plus measurable presentation cadence.
This is a Windows route regression gate, not a claim of 8K throughput or
zero-copy decode. Reference screenshot capture, application-DPI review, and
image-difference approval remain separate gates.

Passing `-Long` adds a thresholded 120-second 4K playback gate. It runs the
same in-process native route, keeps a 3840x2160 surface active, performs three
complete 1920x1080 caption-plane replacements, and exercises pause, resume,
exact seek, and bounded shutdown. The default gate thresholds are at least
20 presented frames/s, at most 10 s startup, 1 s control/caption-update
latency, 3 s shutdown, 2048 MiB absolute working set, and 512 MiB growth after
the warmed 4K baseline. Results are written as schema-versioned JSON below
`build/validation/`.

The 2026-07-30 run against the real 3840x2160 HEVC `bs4k_test_2.ts` sustained
34.74 presented frames/s for 120 seconds with `d3d11va-copy`, started in 1.69
s, peaked at 1526.9 MiB, grew 111.9 MiB after warm-up, uploaded a complete
caption plane in at most 34.6 ms, completed controls in at most 14.7 ms, and
stopped in 337 ms without a render error. These are regression measurements
for this Windows machine, not universal hardware requirements beyond the
explicit pass/fail thresholds. The harness uses Cargo's test profile while the
bundled libmpv binary, WGL surface, decoder and composition route are the same
native runtime used by the application; a packaged-release rerun remains a
separate artifact-acceptance step.

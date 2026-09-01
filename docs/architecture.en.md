# Architecture Baseline (English Translation)

[简体中文（唯一权威）](architecture.zh-CN.md) | [English](architecture.en.md) | [日本語](architecture.ja.md) | [繁體中文（台灣）](architecture.zh-TW.md)

> This is a translation of `architecture.zh-CN.md`. The Simplified Chinese document is the sole authoritative architecture specification; the English, Japanese, and Traditional Chinese (Taiwan) versions are translations only. Any ambiguity or conflict is resolved exclusively by the Simplified Chinese document.

> The third-stage core is implemented. Windows is the native-preview release
> platform for the current Alpha. Native macOS/Linux preview backends are
> explicitly deferred and are not part of the current acceptance scope. The
> remaining renderer work is quality convergence: standard B62 stroke
> behaviour, complete resource rendering, and independent 2K/8K, DPI, and
> screenshot-difference gates. Zero-copy WGL/D3D interop is not a current
> product claim.

## Convergence boundary (2026-08-29)

This phase freezes the frontend stack and Rust crate layout: Svelte, Tauri, and
the existing `arib-caption-worker` remain in place. Frontend feature sessions
are the preferred way to consolidate central state. The Worker remains
responsible for input, probe/demux, decode, Caption IR, export, archive, and
evidence; Tauri remains responsible for task history, queue, settings, window
lifetime, and native preview. Splitting a `resubwinny-core` crate is deferred
until the Caption IR, time model, or transport API is stable and has multiple
consumers.

The same convergence phase explicitly defers BD/DVD bitmap-subtitle OCR, a
plugin system, AI translation, and native macOS/Linux preview. DRCS work is
limited to the local hash-to-Unicode mapping path rather than a general OCR
system.

## Scope

An open-source, cross-platform extractor, converter, archive, and diagnostic tool for captions in Japanese ISDB recordings. The transport layer distinguishes conventional MPEG-2 TS from native BS4K/8K TLV/MMT; `.ts`, `.m2ts`, `.tlv`, and `.mmts` are filename hints only and are never treated as transport evidence. Current release fixtures include conventional TS and 192-byte MPEG-TS/TTML recordings. Native TLV/MMT remains the normative BS4K/8K route with an experimental implementation until lawful real captures are available. The tool preserves ARIB semantics, layout, special characters, and provenance where the selected route supports them. It is not a recorder manager, player, video/audio decoder, EPG browser, CAS tool, generic MMT framework, or live receiver. Legacy tools, `bs4kass.exe`, and Caption2Ass are research/comparison references only and must never ship.

The ResubWinny Worker, Tauri service, and Svelte frontend are licensed under
MPL-2.0. Third-party libraries, binaries, fonts, and corpus material retain
their own licenses and provenance requirements.

## Required architecture

The worker `main.rs` now only calls the `run()` entry exported by `lib.rs`.
Module registration, shared exports, and the test entry live in the library so
the conversion core can be reused independently of the process launcher.

```text
Tauri 2 + Svelte 5 desktop GUI (WebView presentation only)
  -> background work, low-frequency progress, cancellation, diagnostics
Shared Rust conversion core (same implementation for GUI/CLI)
  -> bounded sequential I/O, parsing, timeline, atomic commits
Project caption model and exporters
  -> small stable C ABI
libaribcaption
```

The GUI is never the sole entry point and its UI thread never reads recording bytes, receives per-packet traffic, stores the complete timeline, demuxes, or owns final layout. The conversion core remains callable from the CLI. The current GUI runs that same core in a background thread with cooperative cancellation, progress, and atomic output; add a sidecar only when cross-process crash isolation is needed, not at the cost of the single-EXE delivery. Local large files use blocking buffered Rust I/O by default; do not create an async task or channel message per 188-byte TS packet.

## Streaming and recovery invariants

- File size must not determine normal memory use; 1 GB and 200 GB inputs have comparable working memory.
- Never load, demux, index, or retain the complete recording/timeline in memory; never process broadcast data in frontend JavaScript.
- Input, resync, PES, MPU, and active-scene buffers have hard limits; untrusted lengths never allocate without bounds.
- Stream only target service/caption PID or asset after probing. Do not decode video/audio or fully reconstruct their PES.
- Prefer borrowed slices; copy only when data crosses packets/fragments or must outlive the input window. Deduplicate DRCS by hash.
- Default to normal sequential reads, not whole-file mmap. Future inputs may include files, stdin/pipes, split files, and growing recordings.
- Checkpoints identify the file (size, mtime, first/last block hashes) and save byte offset, continuity, unwrapped PTS, B24 management/DRCS state, and safe output position. Resume from a reliable sync point and replay a short range when full decoder-state restoration is unsafe.
- Write `.part`, temporary event bodies, DRCS assets, and checkpoints; atomically publish only on success. Preserve an explicit incomplete state on cancellation/failure. Keep-awake is manual and off by default.

## Input routes

```text
MPEG-2 TS -> PAT/PMT -> subtitle PES -> ARIB STD-B24 data groups
TLV -> IPv6/compressed IP -> UDP -> MMTP -> signalling -> caption asset -> MPU
```

The conventional route retains service, PID, language, caption/superimpose type, PCR/PTS/DTS, source offset, discontinuities, and warnings. The first BS4K/8K scope is recorded files only: locate MMT package and caption asset, reassemble relevant MPU, recover timestamps, then feed the shared core. It excludes HEVC/audio decoding, complete SI/EPG, CAS, live reception, and a general MMT stack. Probe by content, not filename extension, and distinguish TS, TLV, MMTP, damaged, and partial input.

### Signal-to-ARIB specification map

This is a standards-layer map, not permission to identify a route from a filename. Listed versions were current in the ARIB public catalogue when checked in July 2026; in-stream signalling, descriptors, and payload are authoritative.

| Signal | Physical/transport layer | Service/track discovery | Caption coding/presentation | Demux entry |
| --- | --- | --- | --- | --- |
| Terrestrial 2K (ISDB-T) | ARIB STD-B31; recordings normally contain MPEG-2 TS | MPEG-2 PSI plus ARIB STD-B10 SI | ARIB STD-B24 caption/superimpose data; B24 data groups arrive in subtitle PES | PAT/PMT -> subtitle PES -> B24 data group |
| BS/wideband CS 2K | ARIB STD-B20; recordings normally contain MPEG-2 TS | MPEG-2 PSI plus ARIB STD-B10 SI | ARIB STD-B24; neither a single `stream_type` nor a component-tag heuristic is the whole rule | PAT/PMT -> subtitle PES -> B24 data group |
| BS4K/8K (advanced wideband satellite/ISDB-S3) | ARIB STD-B44 provides ISDB-S3 including TLV; ARIB STD-B60 specifies MMT media transport | MMT signalling, package/asset, descriptors | ARIB STD-B62 Volume 1 Part 3 specifies second-generation caption/superimpose coding, including the ARIB-TTML family | TLV -> IP/UDP -> MMTP -> signalling -> caption asset/MPU -> descriptor-identified caption format |

Important correction: do not assume all BS4K/8K payload is ARIB-TTML merely from its resolution or delivery. Later STD-B60 material says the caption data format is identified by the caption-description method. Read the actual signalling/descriptors; route ARIB-TTML, any B24-compatible/other indicated format, and unknown formats separately and preserve/report the latter. A 192-byte `*.m2ts` packetisation is a recorder-file representation, not a substitute for TS/TLV/MMT content probing.

STD-B24 is the conventional digital-broadcast data coding/transmission specification. STD-B10 complements MPEG-2 PSI with service information; it is not a glyph/layout specification. STD-B62 applies to advanced wideband satellite broadcasting and its Volume 1 Part 3 covers caption/superimpose coding, while STD-B60 covers MMT transport. Implement and test physical/transport, service signalling, and caption coding as separate layers.

Standards entry points (numbers, scope, and links only; do not reproduce copyrighted standard text): [STD-B31](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b31.html), [STD-B20](https://www.arib.or.jp/english/std_tr/broadcasting/std-b20.html), [STD-B10](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b10.html), [STD-B24](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b24.html), [STD-B44](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b44.html), [STD-B60](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b60.html), and [STD-B62](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b62.html).

## Caption truth model and timing

ASS and a single start/end/text cue are not internal truth. Keep a faithful timeline of `TimedCaptionOperation` (clear, cursor, style, text, DRCS, ruby, definitions, etc.) and apply it to `CaptionPlaneState`, producing closed `RegionInterval`/scene objects. Independent regions may overlap, update, and disappear at different times; never merge their lifetimes into one cue.

Retain raw/unwrapped/normalised PTS/DTS/PCR, source offset, management data, language/TCS, clear/repeat/roll-up behaviour, plane geometry, regions, character styling, writing direction, ruby, enclosures, DRCS, unsupported controls, optional raw payload, and warnings. Handle trimming, PCR jumps, discontinuities, wrap-around, loss, missing clears, multiplexed services, reset PTS, and missing end events. Offer explicit strict/repair/zero-point/manual-offset/end-inference policies; never unconditionally use the next subtitle start as prior end.

Close and export a region only when overwritten, cleared, or ended; close remaining regions at EOF. This provides bounded-memory incremental ASS/TTML export. Temporary output bodies are allowed when headers depend on later styles/DRCS; rereading broadcast input or retaining its timeline is not.

## Formats and DRCS

Faithful targets are ASS, TTML, ARIB-TTML, and the project archive. ASS is a compatible visual approximation, not lossless; document limits for vertical text, ruby, flashing, decorations, and complex DRCS. Distinguish full internal TTML, IMSC compatibility, and ARIB-TTML compatibility; never silently delete unsupported structure for validation.

SRT, ordinary WebVTT, TXT, and CSV are lossy/text-extraction outputs only, not formal subtitle conversion or default choices. The GUI must state region merging, timeline splitting, and styling loss.

Archive output stores operation/scene JSON, original data groups/PES/MMT caption assets, DRCS PNG/SVG, PID/asset ID/PTS, and diagnostics; it is the only long-term, best-effort reversible exchange format.

DRCS policy: use proven Unicode mapping; use a user-approved conventional substitute only while recording its mapping; otherwise export and reference the glyph visually; optionally use temporary fonts/vector/bitmap placement in ASS. Never discard, guess, or emit `[外:<hash>]`. Provide a local DRCS dictionary/inspector with glyph, replacement, count, first time, and user choice.

## libaribcaption, previews, IPC, and parser safety

Do not rewrite B24 first. libaribcaption supplies decoding/control/DRCS/region-style behaviour and optional rendering, but not demux, project model, full timing, exporting, checkpoints, or archive. Rust depends on a small project-owned C ABI, not a broad C++ bindgen surface; audit lifetime, pointers, UTF-8, exception containment, allocators, build portability, and ABI drift.

HTML/CSS is for structural previews. Faithful previews use native RGBA/PNG/WebP snapshots requested by time or caption-state change, never video-rate transfers to WebView. IPC is bounded and low-frequency; initial line-delimited JSON is sufficient for progress/warning/track messages.

The main UI prioritises file drop, service/caption-track selection, output format and mode, task control, and preview. Modern design means a simple default path with inspectable internals. The inspector shows at least container type, service ID, PID/asset ID, language, PTS range, DRCS count, CRC errors, packet loss, discontinuities, and unsupported commands.

Handwrite small bounded TS 188/192/204 and PAT/PMT parsers where simpler; TLV/MMTP may use winnow, nom, or a bounded cursor. Every parser must not panic, overrun, loop indefinitely, allocate from untrusted length, and must report offset and recover sync after corruption.

## Validation, phases, and status

The worker entry point remains intentionally small: configuration constants
live in `config.rs`, 188-byte MPEG-TS parsing in `transport/mpeg_ts.rs`,
192-byte M2TS route selection in `transport/m2ts.rs`, and caption document
semantics in `caption/ttml.rs`. The Tauri preview layer exposes a backend
catalog where Windows `libmpv-render` is preferred and
`libmpv-client-overlay` is the per-source fallback. Each route returns availability, experimental
status and a structured unavailable reason. The WebView cannot send caption
pixels: backend `render_preview_at` / `sync_preview_overlay` compose and apply
the native plane. macOS/Linux therefore report an explicit unsupported backend,
not a simulated preview capability.

The archive preview path includes a bounded native caption-plane compositor
for B24 RGBA evidence. It returns one composed PNG plus plane dimensions and
active-layer count. Text-only TTML/B62 archives with supported horizontal or
basic vertical text fields are rasterised by the backend using the bundled
ARIB font; ordinary text surrounding span/ruby tags, direct span styling, and
the narrow direct `tts:textOutline` form are retained. This is not a claim of
resource-complete B62 rendering: complex wrapped vertical ruby, full glyph orientation
and punctuation rules, `arib-tt:border`, and external font/image resources
remain structural metadata or evidence.

For the restricted TLV route, same-MPU resources are now persisted as
lossless `resource_evidence` archive records. The desktop reader retains at
most 64 records and exposes only structurally verified small PNG resources to
active matching captions; fonts and non-PNG resources remain raw evidence.

Build a golden corpus covering terrestrial, BS2K, caption/superimpose, vertical text, ruby, DRCS, colour, position changes, bilingual tracks, damaged TS, and BS4K/8K. Preserve legal originals/generated samples, trusted screenshots, expected event JSON, expected faithful/lossy output, and known issues. Differentially compare legacy tools, FFmpeg/libaribcaption, this tool, and when needed broadcast screenshots for text, timing, clears, position, colour, DRCS, and management changes. Fuzz TS sync, PSI, PES, B24, TLV, MMTP, signalling, and MPU assembly; validate on Windows/macOS/Linux CI.

Implementation order: (1) Rust core, CLI/API, caption model, B24 C ABI, conventional corpus; (2) ASS/TTML/archive plus DRCS visual assets; (3) limited BS4K/8K route; (4) Tauri 2/Svelte 5 task/track/log/inspector/multi-task UI and native mpv preview. Phase 3 is in progress: B62 semantics, bounded resource evidence, responsibility-based worker modules, archive-time preview, B24 native RGBA composition, horizontal and vertical TTML ruby, conservative B62 glyph orientation/punctuation, Windows `libmpv-render`, its thresholded real-corpus 4K long-performance gate, overlay composition tests, fuzz targets, and the cross-platform build matrix are implemented. Resource-complete preview, reference validation of standard B62 stroke rendering, independent 2K/8K and DPI/image-difference gates, and macOS/Linux native preview backends remain.

Native preview synchronization increment (2026-07-25): `sync_preview_overlay` keeps mpv playback time, archive lookup, native RGBA composition, overlay apply/clear, and caption-plane deduplication inside the Tauri backend. The Svelte UI only invokes this low-frequency typed operation and displays its result; it neither estimates media time nor lays out captions. A missing mpv time produces the explicit `awaiting-player-time` result rather than a local-clock guess.

Playback timeline increment (2026-07-25): native preview now carries a validated `PlaybackTimeMapping` with segment identifier, media/project anchors and rational rate. libmpv supplies media time only; archive rendering uses the mapped project time, so PTS repair, programme boundaries and user offsets cannot silently become WebView logic.

libmpv runtime increment (updated 2026-07-29): Windows loads project-bundled `libmpv` in-process; no `mpv.exe` sidecar or JSON named pipe remains. `preview_surface.rs` declares the preferred `libmpv-render` route and the per-source `libmpv-client-overlay` fallback. The WGL render thread owns the OpenGL context, libmpv render loop, resize messages and backend BGRA caption texture blend. Capability and diagnostic APIs report both availability and the actually selected route. Native macOS/Linux preview backends are deferred and are outside the current Alpha acceptance scope.

The Go/Wails prototype is evidence only: the 18.6 GB terrestrial fixture yielded 13,653 caption PES and 2,230 libaribcaption caption objects. It is neither final architecture nor ASS/DRCS/BS4K delivery. Local fixtures are selected with `ARIB_FIXTURE_DIR`; see `docs/corpus.md` for reproducible, opt-in regression checks.

The Rust workspace now contains `crates/arib-caption-worker`, split into `cli.rs`, `inspection.rs`, `jobs.rs`, `preview.rs`, `archive.rs`, `transport/`, `caption/`, `timeline.rs`, `drcs.rs`, and `exporters/`; `main.rs` is only the process entry point. Its bounded `inspect` command recognises 188-byte MPEG-TS, 192-byte M2TS, raw TLV, and unknown input. The worker also exposes `render-at` for bounded archive snapshots, so a desktop UI is not the only way to inspect a timeline. Conventional B24 uses libaribcaption through a narrow project-owned C ABI. The bridge copies plane, regions, Unicode/PUA characters, placement, colours, styles, DRCS code, alternatives, and raw pixels into a Rust scene snapshot before releasing the native object. Unknown DRCS is retained as raw-pixel/metadata assets in a matching `.drcs` directory and rendered as ASS `\p1` vector-drawing events; it never emits `[外:<hash>]`. A complete terrestrial conversion produced 13,653 PES packets, 2,230 caption objects, 2,736 regions, 29,892 characters, 61 DRCS glyphs, and zero decoder errors. The M2TS route discovers private data PIDs, reassembles bounded PES payloads, extracts UTF-8 ARIB-TTML documents, and writes their inherited `div` timing and `region` positions to ASS. The supplied 11.5 GB BS4K fixture completes with 422 TTML caption events, 5,051 characters, and zero parser warnings. The constrained TLV route also converts a complete `stpp` payload only when it is self-contained UTF-8 TTML and carries matching MPT NTP metadata; all other assets retain their raw evidence route. Optional raw export records each selected PES or accepted TLV payload during that same scan, with source offset and lossless hexadecimal bytes. Tauri/Svelte is presentation-only: it forwards typed requests and low-frequency events while the worker prepares all parsing, export, diagnostics, and preview data. B62 styles, ruby, writing mode, resource scope, and bounded PNG/font evidence are retained in the model. The backend natively rasterises supported TTML text fields, horizontal and wrapped vertical ruby, conservative orientation/punctuation, direct opacity, and a narrow direct `tts:textOutline` mapping. Windows `libmpv-render` and native overlay composition are connected. Resource-complete preview, complete B62 orientation/punctuation/stroke semantics, generic TLV/MMTP extraction, and macOS/Linux native preview backends remain Phase 3 work.

Current model delivery: every B24 scene is split into `RegionInterval` values. The bounded active-region map closes a region only when that region changes or disappears, so overlapping labels and dialogue retain independent lifetimes. The same closed interval is emitted to faithful ASS, optional TTML, and JSONL archive records. TTML keeps per-region timing, origin, extent, font size, colour, and a namespaced unresolved-DRCS reference; ASS remains the visual fallback with vector DRCS glyphs. Completed-task timeline and diagnostic windows stream JSONL and retain only the requested page. The live event view keeps only a bounded recent backend window, while the editor timeline uses a bounded prefetched time range and an append cursor instead of rereading the complete archive or sending complete history to WebView. Desktop and multi-task work can pause cooperatively at streaming parser boundaries, resume, or cancel. A `.checkpoint.json` records source size, mtime, a bounded head/tail fingerprint, selected track, and observed progress; recovery rejects a replaced or truncated recording. Because native B24 and partial-artifact state are not serializable yet, a later launch deliberately performs a full replay from the trusted recording origin rather than falsely claiming byte-exact resume.

Raw TLV delivery is content-probed through repeated 4-byte `0x7F/type/length` headers with bounded payload lengths. It now performs a bounded diagnostic/raw-evidence MMTP pass: direct IPv6/UDP, HCfB contexts `0x60`/`0x61`, MMTP packet IDs/payload types, sequential signalling-fragment reassembly (at most 16 streams and 1 MiB per stream), plus MPT asset types and descriptor tags are reported, including an observed `stpp` asset. An MPT MPU timestamp descriptor is retained as the exact 64-bit NTP value keyed by packet ID and MPU sequence, without claiming a normalised caption PTS. For known `stpp` packet IDs it validates MPU/MFU envelopes and bounded MFU reassembly (at most 8 MPU sequences and 4 MiB per sequence). The narrow semantic route accepts only a complete `stpp` payload that is self-contained UTF-8 XML TTML and has matching MPT NTP metadata; it maps NTP deltas from the first valid MPU to the existing TTML caption model. A sequence gap, invalid aggregation, cap breach, missing timestamp, or other payload format remains raw evidence and is not guessed into captions. This is not a claim of generic MMTP caption support. The desktop DRCS dictionary persists user mappings in the platform configuration directory; only the explicit mapping mode substitutes text, while the default continues to preserve unresolved glyph assets.
When an archive is requested, the same bounded pass also emits `asset_evidence` records for discovered MPT assets (packet ID, type, descriptor tags, and exact advertised NTP values). `resource_reference` records retain the originating `packet_id + mpu_sequence_number` scope; `subsampleNumber=0` is the TTML payload and bounded `1..lastSubsampleNumber` units form its same-MPU resource evidence. A numeric `subt://` index is never treated as a global packet ID; absent or incomplete evidence remains explicitly unresolved.

`dump-tlv` is the first raw-extraction route for this layer. It performs one sequential pass and emits JSONL records only after a complete closed-caption payload from a discovered `stpp` asset is available. Each record keeps TLV source offset, MMTP packet/sequence, MPU sequence, timed-MFU flag and lossless hexadecimal data. When its MPT MPU timestamp descriptor is present, `presentation_ntp` retains the exact source NTP value; `pts_ms` remains explicitly `null` until the shared timeline policy is implemented. Raw evidence must not invent a timeline.
The same route now emits complete, bounded non-`stpp` MPU/MFU payloads as `mmt_asset_payload` records with asset type, source offset, a deterministic MPU scope key and lossless bytes. Resource records may include bounded header validation, PNG dimensions, and a capped preview data URI for small structurally complete PNGs, but this remains extraction evidence only and does not claim general-purpose decoding.

Implementation correction (2026-07-23): the M2TS EOF flush regression is fixed. The supplied BS4K fixture now completes with 422 TTML caption events, 5,051 characters, zero parser warnings, and 330 captured PES records when raw export is enabled. The desktop client is Tauri 2 + Svelte 5, not the earlier Slint/eframe prototype. The Home task list persists the latest 20 local task summaries atomically in the platform configuration directory; it does not retain broadcast payloads.

Local-corpus correction (2026-07-23): the 18.58 GB terrestrial and 11.52 GB BS4K fixtures are now opt-in tests selected by `ARIB_FIXTURE_DIR`, with exact streamed byte/count baselines. The M2TS private PES envelope is not assumed to be UTF-8: the bounded extractor locates a complete `<tt>…</tt>` byte slice and validates that XML slice only. This restores 422 captions/5,051 characters from the BS4K fixture while retaining raw PES evidence unchanged.

DRCS-report delivery (2026-07-23): optional `--drcs-report` emits `<name>.drcs.json` only when conventional B24 conversion encountered glyphs. It indexes code, dimensions, colour-independent glyph metadata, alternatives, and paths to the preserved `.drcs` assets without duplicating raw pixel bytes. The native UI exposes the same option; the project archive remains a separate complete caption timeline.

TTML inheritance correction (2026-07-23): the constrained M2TS/TLV TTML parser now walks every still-open `div` before each caption instead of taking the nearest textual `<div>` match. Nested `begin`/`end`/`dur` containers therefore accumulate from the correct parent time base; inherited `style` and `region` apply in document order; and a closed sibling cannot leak timing, writing mode, colour, or placement into a later caption. This improves the shared TTML/archive model and faithful TTML output. ASS remains an approximation for writing modes and ruby.

TTML-style delivery (2026-07-23): the shared caption style now preserves inherited foreground/background colour, family, size, weight, style, writing mode, text/display alignment, outline, line height, letter spacing and opacity through both archive and TTML interchange. ASS applies only its defined equivalents (font, bold/italic, spacing and foreground colour) and deliberately does not pretend to represent unsupported TTML layout or background semantics.

ARIB-TTML span-style correction (2026-07-23): broadcast payloads commonly put their effective styling on `span style="…"` rather than on `p`. The parser now resolves those references, including two-axis font sizes, `arib-tt:letter-spacing`, and TTML eight-digit RGBA colours. Interchange output expands safe span references into self-contained inline TTML attributes, so it does not emit references to source-only style identifiers. The real BS4K sample now verifies `丸ゴシック`, `144px 144px`, foreground/background colour and 16px spacing in archive/TTML, with the defined ASS approximations.

Character-encoding correction (2026-07-23): ARIB STD-B24 character-coded captions remain decoded by libaribcaption rather than being treated as UTF-8 text. For the ARIB-TTML routes, the extractor first isolates XML from the PES/MMTP envelope, then honours its BOM/XML declaration and strictly decodes UTF-8, UTF-16LE/BE, Shift_JIS, EUC-JP, or ISO-2022-JP. A malformed or unsupported XML document is retained in raw evidence and reported; it is never repaired with replacement characters, and invalid framing bytes cannot discard a valid following document.

## Change control

Update all three language documents in the same change. State affected route/model invariant, fixture and validation, and compatibility impact on ASS, archive data, and DRCS mappings. Do not claim support from a proposal, prototype, or one fixture alone.

Evidence priority: the current release gates are 188-byte MPEG-TS/B24 and
192-byte MPEG-TS packetisation with private PES/ARIB-TTML, both backed by
local long fixtures and streamed count baselines. Native BS4K/8K is the
normative `TLV -> IP/UDP -> MMTP -> MPT/MPU` route, but it currently has only
constructed/unit evidence and a narrow `stpp` path; until a lawful real
capture is available, `tlv_mmtp_experimental` remains evidence-first and is
not general support.

For MPEG-TS, B24 caption PIDs remain the verified first choice. When PSI/PMT
only exposes private data PIDs, the worker may scan bounded PES assemblies for
a complete ARIB-TTML XML document. It enters the TTML model only after strict
document-boundary and declared-encoding validation; a private PID alone is
never evidence of captions. The 188-byte private-PES route has a constructed
end-to-end regression and still needs a lawful real-recording fixture.

Dynamic MPEG-TS PMT correction (2026-08-02): a B24 logical track is identified
by `service_id + component_tag`; the PID found at the beginning of a recording
is not treated as permanent. `inspect` samples the head plus a fixed number of
bounded 1 MiB windows across the file, while sequential decoding continuously
tracks current PAT/PMT and flushes the old PES before a PID transition.
Component tags `0x30..=0x37` are captions and `0x38..=0x3f` are superimpose;
the latter never enters the ordinary caption or TTML-candidate route. A
21,609,477,452-byte real recording whose later PMT introduced PID `0x1201`
completed with 18,722 PES, 3,825 scenes, 70,853 characters, and zero decoder
errors. ASS/archive/DRCS semantics are unchanged, and raw evidence records the
actual PID of each PES.

Route codes follow the same evidence boundary: `mpeg_ts_b24_verified` is
descriptor-verified; `mpeg_ts_ttml_candidate` denotes private PES PIDs in
either 188-byte TS or 192-byte M2TS and is not a caption claim;
`mpeg_ts_192_ttml_verified` is reserved for a successfully strict-validated
192-byte M2TS/TTML conversion; `tlv_mmtp_experimental` remains evidence-first;
and `unknown_unsupported` has no supported caption route. None is inferred
from the filename extension.
# Current implementation note

The desktop implementation is Tauri 2 + Svelte 5. The worker is split by responsibility (`cli.rs`, `inspection.rs`, `jobs.rs`, `preview.rs`, `archive.rs`, `protocol.rs`, `resource.rs`, `transport/`, `caption/`, `timeline.rs`, `drcs.rs`, and `exporters/`); `main.rs` is only the process entry point and tests. The `render-at` CLI command returns a bounded archive snapshot at a requested time. Historical Slint references do not describe the current architecture. Initial cargo-fuzz targets now cover content probing, strict TTML envelopes, and MMTP envelopes, and the CI matrix builds core/desktop on Windows, macOS, and Linux. Complete resource-to-preview composition, general TLV/MMT caption conversion, deeper PSI/PES/B24/signalling/MPU fuzz coverage, and macOS/Linux preview backends remain unfinished.

Display-plane correction (2026-07-25): B62/ARIB-TTML viewer geometry is normalised onto the native renderer's logical 1920×1080 plane when the root `<tt>` declares a valid pixel display extent. When it does not, logical 2K remains the default; only complete pixel `origin`/`extent` geometry that exceeds logical 2K on at least one axis and fits a canonical 3840×2160 or 7680×4320 plane may establish that source plane. Region geometry is scaled per axis while pixel font size, line height, letter spacing, and direct outline width use a bounded uniform scale. Equivalent 2K, 4K, and 8K source layouts therefore retain the same screen-relative caption size; ambiguous or invalid data never silently becomes 4K. Raw PES/MMTP evidence remains unmodified.

Vertical punctuation increment (2026-07-25): the native B62 preview maps only Unicode-defined vertical presentation punctuation and uses it only when the bundled ARIB font supplies that glyph. It otherwise preserves the source character. A deterministic archive-to-`render_at` PNG golden covers this path. This does not claim Latin rotation, tate-chu-yoko, complete orientation/punctuation rules, or standard B62 stroke behaviour.

Visual-reference correction (2026-07-25): the bundled libaribcaption
`screenshot0.png` is the project’s television-facing reference image. B24
continues to use libaribcaption-produced RGBA with its configured ARIB font,
ruby, background and stroke settings. B62 targets the same viewer-facing
relationships, but is never declared pixel-identical without a matching B62
source payload and lawful reference capture; see `docs/visual-reference.md`.

Horizontal layout increment (2026-07-25): the native B62 path now preserves
explicit line breaks and applies `textAlign`, `displayAlign`, and `lineHeight`
inside a bounded TTML region, including direction-aware `start`/`end` layout.
An archive-to-`render_at` PNG golden covers multiline centred bottom alignment.

Reference-implementation audit (2026-07-25): `makeding/aribb62.js` is useful
behavioural research and declares MIT in package metadata at reviewed commit
`74304d40a5b8556be1148e123ae70d60f937ecf5`, but has no standalone LICENSE
file or GitHub license endpoint. Its semantics may be independently ported to
the Rust renderer; source is not vendored until redistributable license text
and copyright notice are available. The first such port adds native named
TTML colours, including `transparent`, without browser CSS.

Vertical-ruby increment (2026-07-25): the backend now keeps an explicitly
associated ruby run visible when its vertical base text automatically crosses
columns. Ruby glyphs are distributed across the recorded base-cell reading
path and placed on the writing-mode-specific side at 0.5 scale. This bounded
continuation has archive-to-`render_at` PNG golden coverage; it is not a claim
of general B62 ruby grouping, source-specific placement, tate-chu-yoko, or
complete glyph orientation.

Desktop persistence correction (2026-07-26): settings, job records, task
history, artifact manifests, checkpoints, and DRCS mappings now use one
same-directory atomic publisher. It synchronises a complete `.part` file,
retains the existing metadata until the replacement is installed, and restores
that metadata if replacement fails. This fixes Windows replacement semantics;
it does not alter caption payloads, archive semantics, or any transport route.

## B62 convergence increment (2026-07-26)

The native TTML/B62 preview now treats consecutive `tts:ruby="base"` spans as one base group and places one `tts:ruby="text"` annotation across that group; `arib-tt:ruby` remains associated by `xml:id`. The backend retains supported annotation colour, font size, letter spacing, opacity, and direct restricted `tts:textOutline`; an unspecified annotation keeps the 0.5 base-font default. The same bounded model covers horizontal, vertical, and automatically wrapped vertical ruby.

For vertical layout, punctuation uses a bundled Unicode vertical-presentation glyph when available, CJK/full-width glyphs remain upright, and ASCII/Latin glyphs use a backend clockwise bitmap rotation. Explicit one/two-digit `textCombine` stays horizontal inside one vertical cell. Worker normalisation maps equivalent 2K, 4K, and 8K authored geometry onto one logical `1920×1080` plane, preserving viewer-relative caption area.

This records tested backend behaviour, not broadcaster-specific B62 validation. Lawful source payloads and reference captures remain the acceptance evidence for non-contiguous ruby, additional Unicode orientation classes, and standard stroke semantics.

## Windows native preview convergence increment (2026-07-26)

When Windows discovers a complete `libmpv` render API, it selects `libmpv-render` by default. The backend owns the WGL context, libmpv render loop, resize path, video viewport, backend BGRA caption texture, and blend. If a particular source cannot initialise the render worker, that preview falls back to `libmpv-client-overlay`; backend diagnostics report the actual route, fallback reason, surface dimensions and presentation cadence. A real 3840×2160 HEVC `bs4k_test_2.ts` smoke validates startup, video-frame present, 1920×1080 texture blend/readback, and 3840×2160 resize/present. The WebView receives neither video frames nor caption textures.

The WGL route requests libmpv's `hwdec=auto-safe` policy, which may use compatible copy-back acceleration but is not zero-copy ANGLE/D3D interoperability. `scripts/validate-preview.ps1 -Long` now enforces a 120-second real 4K gate with explicit startup, cadence, full caption-plane upload, control, working-set and shutdown thresholds. The 2026-07-30 `bs4k_test_2.ts` run sustained 34.74 presented frames/s with `d3d11va-copy`, peaked at 1526.9 MiB, and grew 111.9 MiB after the warmed 4K baseline. Independent 2K/8K profiling, DPI review, and reference screenshot differencing remain incomplete. macOS/Linux still return `preview.platform_not_implemented`.

## ASS fidelity correction (2026-07-29)

The B24 ASS exporter now normalises the decoded source plane to the ASS
1920x1080 play resolution, emits each visible character at that scaled position,
and scales its size, horizontal ratio, stroke, and DRCS geometry while retaining
per-character colour, bold, italic, and underline state. Ruby stays on layer 1
at its scaled broadcast character-cell coordinates. The ARIB-TTML path retains
safe inline span styles and places explicitly associated ruby separately; an
annotation without an explicit size uses half the base size. Following TTML
text-layout semantics and the audited reference implementation, the second
component of a two-axis B62 font size supplies ASS font height, while letter
spacing is emitted once through ASS's native spacing command. The exporter does
not horizontally stretch fonts or replace libass shaping with a project-owned
character-grid renderer. A bounded standalone ruby region is matched to an
adjacent containing base region by its source geometry. ASS centres the ruby on
the complete covered base range with `an8+pos`; one-character and multi-character
ranges therefore use the rendered glyph-range midpoint. The base remains one
unchanged Dialogue event so font shaping and spacing around the annotated text
cannot move. Only the ruby anchor is corrected from source character-cell
geometry to the bundled font's libass-compatible advance and ink bounds. Both
above-base and below-base adjacency are accepted; for multi-line captions the
nearest adjacent source row is selected before the horizontal range is mapped.
Real FFmpeg/libass pixel tests cover a single base glyph and multi-character
ruby below the lower row, fail when rendered horizontal centres differ by more
than 3 px, and compare the base raster with a no-ruby reference. Same-time captions are grouped only until their timing
changes, preserving the streaming-memory invariant. ASS defaults to the bundled
`Rounded M+ 1m for ARIB` family, and the broadcast `丸ゴシック` family maps to
that same measured font so ruby width calculations and player rendering use
consistent metrics. Other explicit source families remain unchanged.

The 18.58 GB terrestrial and 11.52 GB M2TS fixtures completed with zero decode
errors. FFmpeg/libass frames at the terrestrial `いかり`/`碇` event and the
M2TS `ささ` annotation centred over `捧` verified position, foreground colour, font size, and
2 px black outline. Arbitrary TTML translucent background rectangles remain
outside the ASS compatibility target and stay available in TTML/archive data.

## Ruby binding and export box layout (2026-07-30)

Ruby association is now a caption-model operation rather than an ASS-exporter
heuristic. B24 `RubyBinding` records the base region/index range, base text and
cell boxes, source ruby box, placement, writing mode and provenance before a
`RegionInterval` reaches any exporter. ARIB-TTML records the equivalent base
caption/run/grapheme range. Same-time standalone B62 ruby regions are associated
while that bounded caption group is complete, before archive, TTML and ASS are
written. The real M2TS corpus currently yields 31 such structured bindings,
including `ささ` to `捧`; unsupported or ambiguous regions remain unbound rather
than being guessed.

ASS alone uses the export-only box layout. It measures the bundled Rounded M+
font behind a replaceable glyph-metrics interface, divides the rendered base
ink range into exact slots, applies integer font-size fallback when glyph ink
would overlap, and performs one bounded integer recentering of the visible ruby
ink. Base text remains one shaped Dialogue event; only ruby glyphs may be
positioned separately. Explicit `rubyPosition` above/below is retained, and the
vertical data path is an axis transpose pending real vertical-corpus validation.
FFmpeg/libass pixel tests are the runtime compatibility gate because libmpv's
internal libass does not expose glyph metrics. This path does not enter or alter
the native preview chain (`libaribcaption -> native RGBA -> libmpv surface`).

## Sequential ARIB-TTML documents and private-PES tracks (2026-08-02)

Namespace-conformant TTML is now read through a read-only XML tree by local
name and ancestry, without requiring literal `<p>` spelling. Some 192-byte
recordings carry ARIB-TTML documents without `begin`, `end`, or `dur`; the next
complete document on the same PID closes the previous document, and an empty
`<tt>` clears it. Zero-filled private-PES timestamps are rejected when their
MPEG marker/prefix bits are invalid, and the 192-byte route instead uses the
M2TS arrival timestamp with 30-bit wrap handling. Document state is isolated
per PID.

PMT `component_tag` ranges `0x30..0x37` and `0x38..0x3f` classify caption and
superimpose components, but do not alone prove B24 or TTML. B24 still requires
`data_component_id 0x0008`; TTML still requires complete XML and strict
encoding validation. Default preview/export selects declared caption tracks,
while superimpose remains an independent explicitly selectable track.
Unclassified streams remain candidates and are never inferred from a PID,
filename, or programme name.

# ResubWinny — Japanese Broadcast Caption Toolkit

The supported desktop application is the Tauri 2 + Svelte frontend in
`studio-tauri/`. The Rust `arib-caption-worker` is a separate streaming backend;
the frontend never parses broadcast bytes or performs conversion itself.

## Build

Use the pinned Rust toolchain from `rust-toolchain.toml` and Node.js 22 LTS.
Contributor setup, architecture rules, and the complete quality gate are in
[`CONTRIBUTING.md`](CONTRIBUTING.md) and
[`docs/toolchain-policy.md`](docs/toolchain-policy.md).

```powershell
npm ci --prefix studio-tauri
./scripts/setup-libmpv.ps1
./scripts/check.ps1
npm run tauri build --prefix studio-tauri -- --no-bundle
```

`setup-libmpv.ps1` is an explicit Windows development step. It downloads the
pinned upstream archive, verifies the archive and extracted hashes, and places
the replaceable runtime under `third_party/`; ResubWinny never downloads or
updates playback components while the application is running. 7-Zip is
required for this setup step.

All generated files are kept under the repository-level `build/` directory.
The release executable is written to
`build/cargo/release/resubwinny-studio.exe`; installers are written below
`build/cargo/release/bundle/`.

The source package version is `0.1.0-alpha.1` and the UI displays
`v0.1.0α`. Windows MSI metadata uses the equivalent numeric prerelease
identifier `0.1.0-1`, because MSI rejects textual prerelease identifiers.

The worker can be built and tested independently from the workspace root:

```text
cargo test -p arib-caption-worker
cargo build -p arib-caption-worker --release
```

Direct debug `cargo check`, `cargo test`, and `cargo clippy` of
`studio-tauri/src-tauri` do not require a release Worker. Tauri bundle resource
validation remains enabled for release builds, and `npm run tauri build`
handles the Worker prerequisite through `build:bundle`.

The old Wails/Slint desktop shell and legacy Caption2Ass binaries are not part
of the supported project and are intentionally not kept in this tree.

Windows native preview has a real-recording smoke and an opt-in 120-second 4K
performance gate. With `ARIB_FIXTURE_DIR` pointing at a legal corpus containing
`bs4k_test_2.ts`, run `scripts/validate-preview.ps1 -Long`. The gate checks
startup, presentation cadence, full backend caption-plane updates,
pause/resume/seek latency, bounded working-set growth, and shutdown; it writes
a JSON report below `build/validation/`.

Run `resubwinny-studio.exe` from `build/cargo/release/` after building. No Python or browser runtime is required. Use `scripts/clean.ps1` to remove generated files, add `-Dependencies` to remove `node_modules`, or add `-DownloadedRuntimes` to remove the explicitly installed libmpv development files.

1. Choose a terrestrial or BS/CS 2K `.ts` recording.
2. Confirm the proposed `.ass` destination, or choose another empty destination.
3. Select **Convert to ASS**. The monitor reports bytes read, captions, characters, DRCS glyphs, and decoder errors while the source is scanned.

The converter writes the ASS through a temporary `.part` file and publishes it only after a successful scan. If unresolved DRCS glyphs occur, their original pixels and metadata are retained in an adjacent `<name>.drcs` directory; they are never emitted as legacy `[外:<hash>]` text.

The DRCS Dictionary accepts a hexadecimal code such as `0x2A7F` and a Unicode replacement. It persists the mapping in the user configuration directory and applies it to both ASS and TTML only when **Replace with Character** is selected; **Keep as Image** remains the default visual-preservation route. **Create DRCS report** writes an optional `<name>.drcs.json` index of glyph codes, dimensions, alternatives, and paths to the preserved pixel assets; it never duplicates raw pixel bytes into the report.

The desktop output panel can also write `<name>.ttml`, `<name>.caption.jsonl`, and a route-specific raw JSONL artifact. TTML preserves independently timed regions, pixel or percentage origin/extent geometry, every open nested time-container offset, colour/background, font family/size/weight/style, writing mode, alignment, outline, line-height, letter-spacing and opacity information, safe inline `span`/`ruby`/`rt` markup, and explicit TTML style references from inherited `div`, `region`, and caption elements, plus a namespaced reference for unresolved DRCS. A declared source display extent is normalised onto the logical 1920×1080 caption plane so equivalent 2K/4K/8K layouts retain their viewer-relative size. When that root extent is absent, the logical 2K plane remains the default; a canonical 4K/8K plane is inferred only from complete pixel region geometry that exceeds 1920×1080 on at least one axis and stays within that canonical plane. Original PES/MMTP payload remains lossless raw evidence. A closed sibling container cannot leak its time, style, or region into the following caption. ASS applies only its defined approximations for font, weight/style, spacing and colour. Complete `<tt>…</tt>` documents are accepted with or without an XML declaration. The streaming project archive contains decoded scenes, closed region intervals (or ARIB-TTML captions), structured Ruby-to-base ranges, source route, and final summary; TLV-derived TTML captions also retain their TLV offset, MMTP/MPU sequence and original NTP provenance. Raw export records each selected source PES or complete TLV payload during the same scan with lossless hexadecimal bytes.

SRT and WebVTT are available only as clearly labelled compatibility copies. They are lossy: neither can faithfully represent independent overlapping regions, broadcast layout, DRCS drawings, ruby layout, or all ARIB timing behaviour. ASS style tags are removed and a drawing-only DRCS event becomes `[DRCS glyph]`, rather than leaking ASS vector commands into the text output. They are plain-text delivery formats, not faithful ARIB conversion targets.

The release-gated conversion routes are ARIB STD-B24 in terrestrial and BS/CS 2K MPEG-TS, and recorder-style 192-byte BS4K/8K M2TS files after their private PES has yielded strictly validated ARIB-TTML. Raw ISDB-S3 TLV/MMTP is an **experimental, evidence-first** route: conversion is deliberately limited to a discovered `stpp` asset whose complete payload is self-contained XML TTML and whose MPU has exact MPT timestamp-descriptor metadata. TTML is decoded strictly from its XML declaration/BOM as UTF-8, UTF-16LE/BE, Shift_JIS, EUC-JP, or ISO-2022-JP; invalid bytes are reported by refusing that document rather than silently replacing or discarding surrounding valid transport data. TLV inspection remains bounded: it observes direct IPv6/UDP and supported HCfB contexts, MMTP packet IDs/payload types, sequential MMT signalling fragments, MPT assets/descriptor tags, raw NTP values, and reassembled closed-caption MFUs under explicit limits. Other TLV/MMTP assets remain raw evidence only; their format and timeline are never guessed.

`dump-tlv input.tlv output.caption.mmtp.jsonl` performs the raw TLV route as a single streaming pass. It writes only complete closed-caption payloads from discovered `stpp` assets, preserving the TLV byte offset, MMTP packet ID and sequence number, MPU sequence number, timed-MFU flag, and lossless hexadecimal bytes. When an MPT MPU timestamp descriptor identifies that MPU sequence, the record also retains its raw 64-bit `presentation_ntp`; `pts_ms` remains `null` until an explicit shared timeline policy is applied. Use `--overwrite` only when replacing an existing JSONL artifact.

Inspection reports evidence-based route codes: `mpeg_ts_b24_verified`,
`mpeg_ts_ttml_candidate`, `tlv_mmtp_experimental`, or
`unknown_unsupported`. `mpeg_ts_192_ttml_verified` is reserved for the
successfully validated 192-byte M2TS/TTML conversion route; a private PES PID
or a `.m2ts` filename alone never earns that status.

Light and dark are one system-following interface, not separate page designs. The Home page retains the latest 20 local task summaries atomically in the platform configuration directory; it stores no broadcast payload. Timeline and diagnostic windows are read directly from JSONL in bounded pages rather than retained as a complete desktop-memory cache; the live editor keeps only a bounded prefetched time window and tails newly completed records. Long-running desktop and multi-task operations can pause and resume cooperatively while the Worker remains alive, or cancel normally. `<name>.checkpoint.json` records source size, modification time, a bounded head/tail fingerprint, selected track and observed progress. Recovery after process/application interruption refuses a replaced or truncated recording, but currently restarts safely from the trusted origin because native B24 decoder and partial-artifact state are not yet serializable.

For automation, the same binary supports:

```text
arib-caption-worker.exe inspect recording.ts
arib-caption-worker.exe convert recording.ts output.ass --ttml --archive --raw --drcs-report
arib-caption-worker.exe convert-b24 recording.ts output.ass --webvtt --overwrite
arib-caption-worker.exe render-at output.caption.jsonl 90000
```

## License

ResubWinny source code is licensed under the Mozilla Public License 2.0. See
[`LICENSE`](LICENSE). Third-party libraries, binaries, fonts, and test corpus
materials remain subject to their own licenses and provenance requirements.
See [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) for pinned versions and
[`docs/dependency-updates.md`](docs/dependency-updates.md) for the update policy.

Security reports must follow [`SECURITY.md`](SECURITY.md). The Windows Alpha
candidate workflow produces private-test artifacts only; it deliberately does
not publish a public release while the exact bundled LGPL libmpv corresponding
source and code-signing chain remain unfinished.

The separate source and Windows-binary publication gates are listed in
[`docs/release-checklist.md`](docs/release-checklist.md). A clean tagged source
archive is produced with `scripts/package-source.ps1`.

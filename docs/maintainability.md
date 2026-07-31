# Maintainability review

This document records the current engineering boundaries and the remaining
work required before the repository is suitable for a public source release.

## Established boundaries

- The Svelte application presents state and calls the typed Tauri gateway. It
  does not parse transport streams, decode captions, or render video frames.
- The Tauri service owns desktop lifecycle, persistence, native preview, and
  Worker supervision. Media parsing remains in `arib-caption-worker`.
- The Worker keeps `CaptionPlane -> RegionInterval -> exporters` as its single
  semantic route and uses libaribcaption only through the narrow C bridge.
- Generated files are isolated under `build/`. Source builds do not depend on
  pre-existing `target/`, `dist/`, or checked-in logs.
- Tauri bundle creation builds its release Worker resource explicitly. Direct
  desktop `cargo check/test/clippy` skips bundle-only resource validation, so
  contributors do not need a stale release Worker merely to check Rust code.
  Release builds still fail when the Worker or another bundled resource is
  missing.
- ResubWinny source is licensed under MPL-2.0. Rust and frontend package
  metadata declare the same SPDX identifier, and desktop bundles include the
  canonical root license.

## Completed decomposition

| Former hotspot | Current structure |
| --- | --- |
| Desktop caption renderer | `caption_renderer.rs` now coordinates composition; `layout`, `rich_text`, `style`, `glyph`, `bitmap`, and `tests` own focused concerns. |
| Desktop preview | Platform-neutral orchestration remains in `preview.rs`; Windows host, unsupported-platform stubs, and tests are separate modules. |
| Worker TTML | B62 semantics, strict XML document decoding, and TS/PES scanning are separate `ttml`, `document`, and `scan` modules. |
| Experimental TLV/MMTP | Base packet/MPU handling, signalling/MPT, evidence writing, and the constrained route are separate modules. |
| Worker tests | Corpus, TS/M2TS, B24/timeline, TTML, and TLV suites own their fixtures in separate files; the full baseline is 129 tests. |
| libmpv | Dynamic client ABI/playback and the Windows render worker are separate; render tests are isolated. |
| Desktop timeline | Public paging/presentation stays in `timeline.rs`; the bounded live-window and append-cursor state is isolated in `timeline/cache.rs`. |
| Svelte application | Theme/locale preferences, multi-task coordination, DRCS dictionary state, task presentation, and output-format metadata moved into feature controllers. |

The largest production files are now Worker `exporters/mod.rs` (about 1,689
lines), `caption/ruby.rs` (about 1,080), `App.svelte` (about 1,043), desktop
`jobs.rs` (about 852), desktop `preview.rs` (about 842), and Worker
`caption/ttml.rs` (about 764). These are the next maintainability hotspots.
Splits must follow exporter format, ruby association/layout, application
lifecycle, job supervision, and preview orchestration boundaries rather than
arbitrary line-count thresholds.

Several renderer hot-path functions still pass explicit geometry to avoid
allocating transient context objects. The compatibility `start_export`,
`create_job`, Worker event helper, and libmpv thread entry points also have wide
signatures. Their lint exceptions are local and reasoned; new APIs should use
typed request/state objects. Existing Tauri parameter names must change only
with a coordinated frontend contract migration.

## Build and quality gate

- Cargo output for the Worker, desktop crate, and fuzz crate is unified under
  `build/cargo/`; Vite output is under `build/frontend/`.
- `scripts/clean.ps1` removes current output plus obsolete root, fuzz, Vite,
  and Tauri output locations. `-Dependencies` also removes `node_modules`.
- Worker and desktop Clippy run with `-D warnings` in CI.
- The current verified baseline is 129 Worker tests and 86 passing desktop
  tests. Two real OpenGL/4K environment tests remain opt-in because they need a
  Windows desktop session and a legal recording path.
- The frontend contract check currently covers 57 typed commands, 40 source
  files, and four complete built-in locale files; Svelte builds with no
  diagnostics.
- `scripts/check.ps1` is the single local entry point for formatting, Worker
  and desktop tests/lints, the frontend build, fuzz compilation, and the
  generated dependency-license inventory.
- `scripts/verify-repository.ps1` rejects generated/downloaded artifacts,
  nested repositories, oversized tracked files, and release-version drift.
  `scripts/package-source.ps1` creates a hash-addressed source archive from a
  clean Git revision; both paths have been exercised in a temporary repository.
- GitHub issue and pull-request templates capture legal sample boundaries,
  affected transport routes, model invariants, and validation evidence.

## Public-release blockers

- Keep `THIRD_PARTY_NOTICES.md` and `third_party/versions.json` synchronized
  with every dependency update. Exact libaribcaption/libmpv revisions, hashes,
  licenses, source locations, and dynamic replacement instructions are now
  recorded.
- Keep the large Windows libmpv binary out of Git. Its pinned archive and
  extracted hashes are verified by `scripts/setup-libmpv.ps1`; Windows CI and
  packaging invoke that explicit setup step.
- Keep the vendored libaribcaption commit and source-snapshot hash synchronized.
  Its nested Git metadata has been removed; future updates must pass
  `scripts/prepare-vendored-source.ps1` before entering the root repository.
- Mirror a durable complete-corresponding-source archive and build scripts for
  the exact bundled Windows libmpv build. The applicable LGPL text, build
  provenance, hashes, and replacement mechanism are now recorded, but upstream
  URLs alone are not treated as the final release artifact.
- Ensure the Rounded M+ 1m for ARIB provenance/license file beside the font is
  included in every installer and binary archive. The bundled binary has been
  matched to its recorded upstream by SHA-256.
- `CONTRIBUTING.md`, `SECURITY.md`, and the supported toolchain policy are now
  present. A Windows Alpha candidate workflow runs the full package gate and
  writes installer hashes without creating a public release.
- Record the code-of-conduct decision and configure a protected signing
  identity before public binary publication.
- Remove claims in architecture documents that no longer match the actual
  implementation and ensure all three language versions describe the same
  verified and experimental capability boundaries.

## Recommended order

1. Keep architecture, backend contract, README, and locale capability wording
   synchronized with the implemented lossy SRT/WebVTT and Windows-only native
   preview boundary.
2. Split the remaining production hotspots by behaviour and add focused tests
   around every moved boundary.
3. Run packaged Windows end-to-end acceptance for source selection, native
   preview, dynamic broadcast metadata, multi-task control, language packs,
   output planning, and artifact publication. Source selection, paused native
   video, dynamic metadata, 118-event indexing, and final-archive timeline
   recovery are verified with `bs4k_test_2.ts`; the remaining workflows still
   need packaged acceptance.
4. Produce a fixed, complete corresponding-source package for the exact
   bundled LGPL libmpv build; the current development DLL blocks public binary
   distribution until this is done.
5. Keep Cargo/npm dependency auditing active and convert the private Alpha
   candidate workflow into a signed public-release workflow only after the
   libmpv corresponding-source artifact and signing identity are available.

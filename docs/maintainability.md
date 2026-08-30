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
| Desktop preview | `preview.rs` owns capability discovery and the stable native command wrappers; archive paging, overlay synchronization, Windows native playback, unsupported-platform stubs, and tests are separate modules. |
| Desktop jobs | `jobs.rs` owns the public task model; JSON/JSONL persistence and the queue supervisor are isolated in `jobs/repository.rs` and `jobs/supervisor.rs`. |
| Worker exporters | The public exporter boundary remains in `exporters/mod.rs`; ASS, TTML, text formats, B24 orchestration, evidence, and Ruby layout live in format-focused modules. |
| Worker TTML | B62 semantics, strict XML document decoding, and TS/PES scanning are separate `ttml`, `document`, and `scan` modules. |
| Experimental TLV/MMTP | Base packet/MPU handling, signalling/MPT, evidence writing, and the constrained route are separate modules. |
| Worker tests | Corpus, TS/M2TS, B24/timeline, TTML, TLV, archive, and synthetic protocol suites own their fixtures in separate files; the full baseline is 146 tests. |
| libmpv | Dynamic client ABI/playback and the Windows render worker are separate; render tests are isolated. |
| Desktop timeline | Public paging/presentation stays in `timeline.rs`; the bounded live-window and append-cursor state is isolated in `timeline/cache.rs`. |
| Svelte application | Theme/locale preferences, multi-task coordination, DRCS dictionary state, task presentation, and output-format metadata moved into feature controllers. Multi-task, DRCS, and settings views now live under their owning feature directories rather than the source root. |

The remaining application shell is an explicit composition root. `SourceSession`
owns source preparation, inspection generations, busy lifetime, and suppression
of stale results/errors, then applies the task setup and activates preview/index
only for the current source. `ExportSession` owns export/index request
validity including stale job-created callbacks, stale failures, and preview-index
cancellation, together with their begin/success/failure state projections.
`PreviewSession` owns native
preview geometry, lifetime, and
managed start/stop transitions, seek/scrub coordination, and the distinction
between an unknown first media sample and an actual zero timestamp. It also
owns resize coalescing and preview-page
generation/resume state, so stale WebView hosts and untyped resume timestamps
do not leak into the application shell. Playback mapping persistence and the
explicit media-to-project cursor remap also remain inside this preview domain,
as do player-command and volume IPC error/notice handling.
Successful inspection defaults are produced by a pure task setup transition;
the shell no longer reconstructs output paths, initial track/format selection,
or source notices field by field. The batch controller owns queue lifecycle and
editing-item track projection, while cross-feature task activation remains in
the composition root.
`HistorySession` owns bounded task-history
persistence, and `LayoutSession` owns responsive shell transitions.
`runtime-session.ts` centralises task runtime resets; `feedback-session.ts`
centralises bounded notices and backend error messages; `selection-session.ts`
centralises output-format, preservation, and track selection transitions;
`bootstrap-session.ts` loads independent desktop startup resources;
`application-lifecycle-session.ts` owns desktop event subscriptions and
teardown; and `recovery-session.ts` owns checkpoint eligibility and replay.
These sessions project results into Svelte values but do not become a second
global store.

The largest production files are now Worker `exporters/ass.rs` (about 1,185
lines), `caption/ruby.rs` (about 1,080), `App.svelte` (about 1,100), Worker
`caption/ttml.rs` (about 764), desktop `jobs/repository.rs` (about 720), and
frontend `features/batch/BatchQueue.svelte` (about 632). The exporter, job, and
preview entry modules are now small ownership boundaries rather than
implementation buckets. Further splits should follow ASS event construction,
ruby association/layout, application session lifecycle, repository concerns,
and multi-task table/preset concerns rather than arbitrary line-count
thresholds.

Time domains are explicit at their ownership boundaries. The frontend and
desktop mapping layers distinguish media and project milliseconds, while the
Worker represents the 33-bit MPEG PES clock as `Pts90k` and converts it to
milliseconds only when entering Caption IR, evidence, or timeline handling.
MMT presentation NTP remains a separate transport concept.

Caption IR convergence happens after parsing rather than in the transport
models. A closed, zero-copy `CaptionCueRef` exposes shared timing, region,
route, plain-text, ruby-count, and DRCS-presence semantics for B24
`RegionInterval` and ARIB-TTML `TtmlCaption`, while retaining their complete
route-specific DRCS, ruby, style, and provenance payloads. The archive writer
consumes this common boundary but preserves the schema-v1 `region_interval` and
`caption` record shapes.

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
- The current verified baseline is 146 Worker tests and 106 passing desktop
  tests. Four real-recording/archive environment and performance tests remain
  opt-in because they need a Windows desktop session, a legal recording or
  archive path, and route-specific performance thresholds.
- The frontend contract check currently covers 58 typed commands, 64 source
  files, and four complete built-in locale files; Svelte builds with no
  diagnostics.
- `scripts/check.ps1` is the single local entry point for formatting, Worker
  and desktop tests/lints, the frontend build, fuzz compilation, and the
  generated dependency-license inventory.
- `scripts/build.ps1` is the single packaging entry point. Its Windows default
  is the bundled profile, which explicitly installs and verifies the pinned
  runtime; `-Libmpv External` produces a package without libmpv and expects a
  compatible runtime to be supplied by the user. The base Tauri configuration
  itself does not silently bundle a runtime.
- The ordinary CI path has four focused jobs: one shared static-quality gate,
  a three-platform Rust test matrix, fuzz-target compilation, and dependency
  auditing. A scheduled weekly workflow executes each fuzz target for a bounded
  30-second run; pull requests retain compile-only fuzz coverage. `cargo-deny`
  enforces the checked-in license/source policy for Worker, desktop, and fuzz
  manifests. The long LGPL libmpv build is manual and isolated from pull-request
  CI. It runs directly on the GitHub Ubuntu runner and records its complete
  tool/package environment beside the corresponding-source archive.
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
- Record the code-of-conduct decision. A protected signing identity is required
  for Signed Stable releases, but not for an explicitly disclosed Unsigned
  Windows Alpha that satisfies the source, hash, provenance, and license gates.
- Remove claims in architecture documents that no longer match the actual
  implementation and ensure all three language versions describe the same
  verified and experimental capability boundaries.

## Recommended order

1. Produce an auditable Unsigned Windows Alpha pipeline that publishes the
   exact tag and commit, complete artifact hashes, unsigned-build warning,
   notices, and the bundled libmpv corresponding-source receipt.
2. Run packaged Windows end-to-end acceptance for source selection, native
   preview, dynamic broadcast metadata, multi-task control, language packs,
   output planning, and artifact publication. Source selection, paused native
   video, dynamic metadata, 118-event indexing, and final-archive timeline
   recovery are verified with `bs4k_test_2.ts`; the remaining workflows still
   need packaged acceptance.
3. Maintain a private real-broadcast compatibility matrix and publish only its
   results. Do not add synthetic broadcast generation to replace legally held
   recordings, and keep TLV/MMTP explicitly experimental.
4. Add focused tests for pure frontend behavior and generated Rust-to-TypeScript
   DTO types without introducing a frontend test framework or RPC framework.
5. Produce a fixed, complete corresponding-source package for the exact
   bundled LGPL libmpv build; the current development DLL blocks public binary
   distribution until this is done.
6. Keep Cargo/npm dependency auditing active. Publish an unsigned Alpha only
   after the libmpv corresponding-source gate passes; add signing as a separate
   requirement when promoting a build to Signed Stable.

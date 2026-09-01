# Contributing to ResubWinny

> Translation. The [Simplified Chinese version](CONTRIBUTING.md) is the sole authoritative source. Other languages: [繁體中文](CONTRIBUTING.zh-TW.md) · [日本語](CONTRIBUTING.ja.md)

ResubWinny welcomes focused fixes and features that preserve its backend-first architecture. Before starting a large transport, caption-model, renderer, or desktop-workflow change, open a design discussion describing the input route, model invariant, expected artifacts, samples, and known compatibility limits.

## Architecture rules

- Svelte displays backend state and forwards typed requests. It does not parse media, calculate caption layout, decode video, or own subtitle timing.
- Tauri owns desktop lifecycle, persistence, native preview, and Worker supervision. Media and caption processing belongs in `arib-caption-worker`.
- A GUI operation needs an equivalent Worker/CLI or backend API unless it is purely transient interface state.
- `CaptionPlane -> RegionInterval -> exporters` is the only caption-semantic path. libaribcaption remains behind the project-owned narrow C ABI.
- Input type is detected from bounded content evidence, never trusted from a filename extension.
- TLV/MMTP is experimental and evidence-first. Do not describe it as verified or infer unknown assets as captions.

## Local setup

Use the pinned Rust toolchain in `rust-toolchain.toml`, Node.js 22 LTS, and `npm ci` in `studio-tauri`. Generated files belong below `build/` and are not source changes.

Windows native-preview development also requires 7-Zip and the explicitly installed, hash-verified libmpv runtime:

```powershell
./scripts/setup-libmpv.ps1
```

The application never downloads or updates this runtime on its own.

After installing dependencies, run the complete local quality gate:

```powershell
./scripts/check.ps1
```

`-SkipFrontend` and `-SkipFuzz` are available for a focused Rust-only pass; they do not replace the complete gate before a pull request.

```text
cargo test -p arib-caption-worker
cargo build -p arib-caption-worker --release
cargo test --manifest-path studio-tauri/src-tauri/Cargo.toml
npm ci --prefix studio-tauri
npm run build --prefix studio-tauri
cargo check --manifest-path fuzz/Cargo.toml
cargo fmt --check
cargo fmt --manifest-path studio-tauri/src-tauri/Cargo.toml --check
```

Run Clippy with warnings denied before submitting Rust changes. Transport, timeline, model, renderer, or exporter changes also need focused regression tests. Legal long recordings remain local; contribute constructed or trimmed fixtures only when redistribution is permitted.

## Change requirements

- Keep public user text in locale files. Built-in `en`, `ja`, `zh-CN`, and `zh-TW` files must contain the same keys.
- Keep Worker JSONL and typed Tauri contracts versioned and backward-aware.
- Keep parsing buffers and output dimensions bounded; use 64-bit source offsets.
- Preserve unsupported source data as explicit evidence or reject it with a stable code. Do not guess.
- Update README, backend contract, architecture documents, corpus expectations, and export limitations when their contract changes.
- Do not commit recordings, task output, logs, build products, generated dependency trees, credentials, or signing material.

## Dependencies and licenses

ResubWinny source is MPL-2.0. New dependencies must have a compatible license, documented purpose, pinned provenance where bundled, and an updated license inventory. Follow [the dependency-update policy](docs/dependency-updates.md); libaribcaption, libmpv, the Rounded M+ ARIB font, and reference-only aribb62.js each have distinct update and attribution requirements.

Vendored source directories must not contain nested `.git` metadata. After reviewing a clean, pinned libaribcaption update, run `scripts/prepare-vendored-source.ps1` to convert it into the source snapshot that belongs in this repository.

By contributing, you agree that your contribution is provided under MPL-2.0.

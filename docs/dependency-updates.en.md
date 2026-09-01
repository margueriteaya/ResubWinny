# Third-party dependency update policy

[简体中文](dependency-updates.md) · [繁體中文](dependency-updates.zh-TW.md) · [日本語](dependency-updates.ja.md) · [English](dependency-updates.en.md)

> **Normative notice:** The Simplified Chinese version is the sole authoritative source. The other language versions are synchronized translations; if wording is ambiguous or conflicts, the Simplified Chinese version prevails.

ResubWinny uses pinned, reviewable dependency updates. It never downloads or
replaces parser, renderer, font, or playback components at application runtime.
Update automation may open a proposal, but must not merge or publish it.

## Dependency classes

| Class | Examples | Pinning and update rule |
| --- | --- | --- |
| Vendored source | libaribcaption | Pin an upstream tag, full commit, and deterministic source-snapshot hash in `third_party/versions.json`; review the source diff and license, then remove nested Git metadata with `scripts/prepare-vendored-source.ps1`. |
| Downloaded binary runtime | Windows libmpv | Pin the release tag commit, workflow recipe commit/run, toolchain commit, upstream mpv commit, asset name, archive hash, and extracted hashes. `scripts/setup-libmpv.ps1` installs it explicitly for development and packaging; the application never downloads it. Never replace only the DLL without its headers, notices, and corresponding-source plan. |
| Reference-only source | aribb62.js | Pin the reviewed commit. Upstream changes are research input, not executable dependencies and not automatically ported. |
| Package-managed source | Cargo and npm packages | Lockfiles are authoritative. Dependabot may propose updates; maintainers review and test them. |
| Visual asset | Rounded M+ 1m for ARIB | Pin binary hash, provenance, and license. Replacement requires glyph coverage and visual-golden comparison. |

## Required update record

Every dependency update must record:

1. old and new version, commit, artifact hash, and upstream URL;
2. upstream release notes and the reviewed source/ABI diff;
3. license, copyright, build-option, and transitive-dependency changes;
4. affected ResubWinny route and model invariants;
5. tests and corpus evidence run for the update;
6. output compatibility impact for archive, ASS, TTML, DRCS, and preview;
7. rollback commit or previous artifact identity.

## Validation gates

All updates run the normal project gate:

```text
cargo test -p arib-caption-worker
cargo check --manifest-path studio-tauri/src-tauri/Cargo.toml
npm run build --prefix studio-tauri
cargo check --manifest-path fuzz/Cargo.toml
cargo fmt --check
```

Additional gates depend on the component:

- **libaribcaption:** bridge ABI compile, B24 decoding corpus, DRCS mapping,
  RegionInterval timing, and B24 visual golden comparisons. Character mapping,
  control-code, default option, or renderer changes are semantic changes even
  when the C ABI is unchanged.
- **libmpv:** exported-symbol check, replaceability check, native preview smoke,
  seek/pause/resume, overlay clock synchronisation, resize/DPI, and 2K/4K/8K
  performance samples. Verify that the artifact is still an LGPL build and
  package its exact source cache using `scripts/package-libmpv-source.ps1`.
- **aribb62.js:** inspect upstream changes manually. Port only independently
  understood behaviour supported by ARIB documents or corpus evidence. Never
  copy newly added code until its redistributable license is unambiguous.
- **font:** glyph coverage, missing-glyph diagnostics, horizontal/vertical ruby,
  punctuation orientation, outline/background, and logical 2K/4K/8K visual
  equivalence.

Parser or renderer updates require long-sample regression before release. A
change that alters expected output must update golden data and explain why the
new result is more correct; silently accepting new output is prohibited.

## Security updates

A high-severity security update may use an expedited review, but still requires
license verification, focused tests for the affected boundary, and an explicit
rollback artifact. It may skip unrelated long-running tests only when the
release note records that exception and schedules the omitted gate.

## Checking upstreams

Run `scripts/check-upstreams.ps1 -Online` to verify local hashes and compare
pinned commits with current upstream heads. Add `-FailOnUpdate` in scheduled CI
when an available upstream update should create a failing maintenance signal.
An available update is not permission to merge it.

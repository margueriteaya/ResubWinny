# ResubWinny third-party notices

ResubWinny source is licensed under MPL-2.0. The following components and
assets retain their own licenses. This inventory describes the versions
currently present in the source tree; `third_party/versions.json` is the
machine-readable provenance record.

## libaribcaption

- Upstream: <https://github.com/xqq/libaribcaption>
- Version: `v1.1.2`
- Commit: `c64c23b8905ba514b87c9789269e9f66f949ffe0`
- Source snapshot SHA-256:
  `E71F007E91A0D417384E6CDC12FAD38EF5D4BC2A5FEE14425EE6404559B069E8`
- License: MIT
- Copyright: Copyright (c) 2022 magicxqq
- Local license: `third_party/libaribcaption/LICENSE`

The complete upstream source is vendored without nested Git metadata and
linked statically through
ResubWinny's separately maintained narrow C ABI bridge. ResubWinny carries no
patches inside the vendored tree at this revision.

## libmpv for Windows x86_64

- Build upstream: <https://github.com/zhongfly/mpv-winbuild>
- Build tag: `2026-07-24-0fb136f685`
- Release tag commit: `9b6ccd6abbfcd6bb2dcad8946d445f670b0555ef`
- Build recipe commit: `b4b1088c30e8821e012fd20052de4c2d3a8eaad4`
- GitHub Actions run: `30091573485`
- Toolchain repository: <https://github.com/shinchiro/mpv-winbuild-cmake>
- Toolchain commit: `04283f7e911149809c46bc236a834cf7134ba133`
- FFmpeg commit: `2f209337fc66b58bf0495265880bb37580c3f981`
- Asset: `mpv-dev-lgpl-x86_64-20260724-git-0fb136f685.7z`
- mpv commit: `0fb136f685c21ec10943f682bec1c90220d2d90f`
- License: LGPL-2.1-or-later build
- Local license: `third_party/libmpv/LICENSE.LGPL`
- Upstream copyright and file exceptions: `third_party/libmpv/COPYRIGHT.mpv`
- Bundled DLL SHA-256:
  `2FDF6BF2AD4354F26A191C12EBA02492DC5A2F024AAC018494C85192DAE84E80`

The DLL is loaded dynamically and is replaceable. A compatible library can be
selected with `RESUBWINNY_LIBMPV`; the application does not copy libmpv code
into its executable. ResubWinny does not modify the DLL. The large development
binary is not stored in the source repository: `scripts/setup-libmpv.ps1`
downloads the pinned upstream archive only when explicitly run, verifies the
archive and extracted file hashes, and installs it under `third_party/`.

For every public binary release, the release page must provide a durable copy
of the complete corresponding source and build scripts for the exact bundled
LGPL build, or an LGPL-compliant written offer. A link to a mutable upstream
branch is not sufficient. This corresponding-source package has not yet been
mirrored in the current development workspace. The upstream build also used
moving dependency branches without publishing its complete source cache, so
the current DLL is for development/private testing only. Public binary
distribution remains blocked until ResubWinny produces a fixed build and the
archive required by `docs/libmpv-source-compliance.md`.

## Rounded M+ 1m for ARIB

- Binary source: <https://github.com/5ym/arib-font>
- License: M+ FONT LICENSE with redistributable WadaLabMaruGo2004ARIB glyphs
- SHA-256:
  `417D40CEA344A42A422AF35AF9460891456FB794D9F7BBAFE632549C4457EBDA`
- Full provenance and grants:
  `third_party/rounded-mplus-1m-arib/LICENSE.txt`

Both source grants permit modification, commercial use, embedding, and
redistribution. The local provenance notice is included in desktop bundles.

## aribb62.js

`makeding/aribb62.js` is not copied, linked, installed, or distributed by
ResubWinny. It is a reference used to compare publicly documented B62
behaviour at commit `74304d40a5b8556be1148e123ae70d60f937ecf5`. Its package metadata declares
MIT, but the reviewed repository has no standalone license file. Semantics are
implemented independently in Rust and accepted only with project tests and
ARIB/reference evidence. No third-party source is vendored from this project.

## Package-managed dependencies

Rust and npm dependencies are pinned by `Cargo.lock` and
`studio-tauri/package-lock.json`. Their individual copyright and license
notices remain applicable. The generated transitive package inventory is
maintained in `docs/dependency-licenses.md` and checked against the Cargo and
npm lock data in CI. This inventory does not replace the dependencies'
authoritative license texts, which must also be archived with public binary
releases where required.

## Broadcast corpus

Private recordings and local corpus files are not part of ResubWinny and are
not licensed by MPL-2.0. Public fixtures must be constructed, independently
licensed, or distributed only as hashes and expected metadata.

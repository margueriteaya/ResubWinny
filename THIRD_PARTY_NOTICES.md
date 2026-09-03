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

## libaribtlv

- Upstream: <https://github.com/makeding/libaribtlv>
- Version: `0.6.1`
- Commit: `a84e5b62bf9230d3fcea21c66e62f7cc5d50a3c2`
- License: MIT
- Copyright: Copyright (c) 2026 huggy
- Local license: `third_party/libaribtlv/LICENSE`

The complete source snapshot is linked statically only when the optional
`libaribtlv` Worker feature is enabled. ResubWinny exposes its B62 subtitle
events through a separately maintained narrow C ABI; callback-lifetime data is
copied before returning to the library. Player, MSE and `tlvdemux` code is not
included.

## Zlib

- Upstream: <https://github.com/madler/zlib>
- Version: `1.3.2`
- Commit: `da607da739fa6047df13e66a2af6b8bec7c2a498`
- License: Zlib License
- Copyright: Copyright (c) 1995-2026 Jean-loup Gailly and Mark Adler
- Local license: `third_party/zlib/LICENSE`

Zlib is built from the pinned source snapshot as a private static dependency
of libaribtlv. Shared/system Zlib discovery is not used by that build route.

## libmpv for Windows x86_64

- Build upstream: <https://github.com/zhongfly/mpv-winbuild>
- Build tag: `2026-08-29-e8673660ab`
- Release tag commit: `9b6ccd6abbfcd6bb2dcad8946d445f670b0555ef`
- Build recipe commit: `b4b1088c30e8821e012fd20052de4c2d3a8eaad4`
- GitHub Actions run: `30091573485`
- Toolchain repository: <https://github.com/shinchiro/mpv-winbuild-cmake>
- Toolchain commit: `04283f7e911149809c46bc236a834cf7134ba133`
- FFmpeg commit: `2f209337fc66b58bf0495265880bb37580c3f981`
- Asset: `mpv-dev-lgpl-x86_64-20260829-git-e8673660ab.7z`
- Asset SHA-256:
  `78260166265FBC09B3BEE75EE3464EB0F6BBAA8ECD172786E33C22BBF8A3CB47`
- mpv commit: `0fb136f685c21ec10943f682bec1c90220d2d90f`
- License: LGPL-2.1-or-later build
- Local license: `third_party/libmpv/LICENSE.LGPL`
- Upstream copyright and file exceptions: `third_party/libmpv/COPYRIGHT.mpv`
- Bundled DLL SHA-256:
  `9D3F661F510FDF660D80B663241D6C4A2933B083EC26AF3CCFD1FB4164F0708C`
- Import library SHA-256:
  `BEF1B89F534BC86B33135E1F04FA2D5064B9D48B5DE8BC9866665BBF43DEF793`

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

## shuding/liquid-glass

- Upstream: <https://github.com/shuding/liquid-glass>
- License: MIT
- Copyright: Copyright (c) 2025 Shu Ding
- Local license: `third_party/shuding-liquid-glass/LICENSE`

The Windows frontend adapts the upstream rounded-rectangle signed-distance
displacement-map approach. ResubWinny generates and caches maps only when a
production control first appears or changes size; WebView2 performs live
backdrop sampling through its SVG filter compositor. The demo shell, dragging
code, and continuous mouse-driven map generation are not included.

## PlayStation-3-XMB

- Upstream: <https://github.com/linkev/PlayStation-3-XMB>
- Reviewed commit: `1ec453a9dddec5448d615116ff428349f42d454e`
- License: MIT
- Copyright: Copyright (c) 2025 Mart
- Local license: `third_party/playstation-3-xmb/LICENSE`

The onboarding hero directly integrates the upstream default spline renderer,
reverse-engineered displacement pipeline, settings, background pass, and
particle renderer. ResubWinny supplies its own Svelte lifecycle and right-to-left
reveal, transparent background-pass integration, and onboarding colour field;
the upstream wave displacement and particle geometry remain unchanged. The
upstream controls, logos, and optional gradient presets are excluded.

## Package-managed dependencies

Rust and npm dependencies are pinned by `Cargo.lock` and
`studio-tauri/package-lock.json`. Their individual copyright and license
notices remain applicable. The generated transitive package inventory is
maintained in `docs/dependency-licenses.md` and checked against the Cargo and
npm lock data in CI. This inventory does not replace the dependencies'
authoritative license texts, which must also be archived with public binary
releases where required.

The Worker uses `roxmltree` for namespace-aware, read-only TTML structure
parsing. It is an unmodified package-managed dependency distributed under
MIT or Apache-2.0; its pinned version and authoritative package metadata are
recorded in `Cargo.lock` and `docs/dependency-licenses.md`.

## Broadcast corpus

Private recordings and local corpus files are not part of ResubWinny and are
not licensed by MPL-2.0. Public fixtures must be constructed, independently
licensed, or distributed only as hashes and expected metadata.

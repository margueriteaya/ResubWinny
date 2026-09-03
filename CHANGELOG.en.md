# Changelog

> Translation. The [Simplified Chinese version](CHANGELOG.md) is the sole authoritative source. Other languages: [繁體中文](CHANGELOG.zh-TW.md) · [日本語](CHANGELOG.ja.md)

This project remains in early Alpha. Releases may contain breaking changes.

## [0.2.3-alpha.1] - 2026-09-03

### Workspace and onboarding

- Reworked the main workspace so recording entry, preview, and common controls take priority. The primary home-page workflow remains visible at common window heights, and desktop-workflow text alignment was refined.
- Added Settings pages for About, build provenance, and offline license browsing. Segmented controls now have complete keyboard support, preferences save automatically, and the unused timeline preference was removed.
- Added an ARIB-inspired onboarding experience that introduces the workflow through caption overlays, ruby text, DRCS, and XMB waves. It uses less animation work and preserves the 16:9 XMB scene's aspect ratio at different window sizes.

### B62 / TLV caption handling

- Integrated a native B62 TLV backend for direct use by the ARIB-TTML caption workflow.
- Preserved B62 source-layout semantics: regions and inline backgrounds are handled separately, and captions map to the video-content viewport independently of resolution. This avoids treating a region's capacity as the display-plane boundary.

### Engineering, documentation, and release

- Added developer documentation in Simplified Chinese, Traditional Chinese, Japanese, and English, with Simplified Chinese explicitly designated as the sole authoritative source.
- Updated Rust dependencies, Vite, and the Svelte Vite plugin, and refreshed the frontend dependency-license inventory.
- Hardened the libmpv build and cache paths, fixed the stable-Cargo build and graphics-dependency setup, and upgraded the Actions artifact and cache actions.

### Windows Alpha release

- This is the first release to include installable, unsigned Windows x86_64 Alpha binaries, complete corresponding source, license materials, build receipts, and SHA-256 checksums.
- Windows may display an “unknown publisher” warning. This is expected for an unsigned Alpha and does not imply code-signing verification.

### Known limitations

- This remains a preview release; Windows is the primary acceptance platform for native video preview.
- Native video preview is not yet available on macOS or Linux.
- Raw TLV/MMTP support remains experimental and must not be considered general BS4K/8K support. Real-broadcast compatibility of B62 is validated only with private, non-redistributable material.
- The Windows package is unsigned. Private broadcast recordings, captions derived from them, and screenshots are not distributed with this release.

## [0.2.2-alpha.1] - 2026-08-30

### Windows Alpha release

- Public releases are now explicitly divided into Source Release, Unsigned Windows Alpha, and Signed Stable; code signing no longer blocks a publicly disclosed Alpha.
- Unsigned Windows Alpha packages now include a risk notice and a dependency-license inventory, and generate a Release manifest containing the exact Git tag, commit, file sizes, and SHA-256 values.
- A Windows candidate must use the same DLL, import library, complete corresponding source, and `SOURCE-RECEIPT.json` produced by the specified compliant libmpv build. Hashes, pinned provenance, and the complete source-package set are cross-checked during assembly.
- Added a private real-broadcast compatibility matrix. It requires validating the complete subtitle workflow using the installed application, while publishing results only—not recordings, subtitles, or programme metadata.

### Known limitations

- The currently pinned upstream libmpv development DLL cannot yet be publicly distributed. A new compliant workflow must first generate and durably publish matching binaries, complete corresponding source, and build receipts.
- Windows installer candidates must still pass clean-system installation, a complete real-recording workflow, and uninstall acceptance before a public Unsigned Alpha Release can be created.

## [0.2.1-alpha.1] - 2026-08-30

### UX and state presentation

- Split background preview indexing and export into two user-understandable states. Export can still be configured and started during indexing; when the backend must serialize work, it explains the waiting relationship.
- Added a global, dismissible persistent error banner, so operation failures remain visible even if the output panel is collapsed.
- Opening the Tasks page no longer automatically opens the file picker; it now shows the existing blank task page.
- Preview, Events, and Diagnostics show text labels at ordinary widths and use icons alone only in compact viewports.
- Fixed the false selected state and click target of Recent on the home page. The entire row now opens with mouse or keyboard, and the “View all” action without a matching history page was removed.

### Engineering and release

- Strengthened cross-platform CI, the Cargo dependency policy, fuzz checks, Windows native dependencies, and linting.
- Fixed cross-platform consistency of source-snapshot hashes and continued to pin and verify libmpv runtime provenance.
- Improved source releases, dependency licenses, and repository-integrity checks.

### Known limitations

- This remains a preview release; Windows is the primary acceptance platform for native video preview.
- Raw TLV/MMTP support remains experimental and must not be considered general BS4K/8K support.
- This Release includes no public Windows binary; signing and the libmpv corresponding-source release requirements must still be satisfied separately.

[0.2.2-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.2-alpha.1
[0.2.1-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.1-alpha.1
[0.2.3-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.3-alpha.1

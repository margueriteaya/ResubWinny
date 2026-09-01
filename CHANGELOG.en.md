# Changelog

> Translation. The [Simplified Chinese version](CHANGELOG.md) is the sole authoritative source. Other languages: [繁體中文](CHANGELOG.zh-TW.md) · [日本語](CHANGELOG.ja.md)

This project remains in early Alpha. Releases may contain breaking changes.

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

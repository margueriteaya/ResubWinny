# Release checklist

[简体中文](release-checklist.md) · [繁體中文](release-checklist.zh-TW.md) · [日本語](release-checklist.ja.md) · [English](release-checklist.en.md)

> **Normative notice:** The Simplified Chinese version is the sole authoritative source. The other language versions are synchronized translations; if wording is ambiguous or conflicts, the Simplified Chinese version prevails.

ResubWinny has three public release tiers. Passing one tier does not imply that
the next tier is ready:

1. **Source Release** publishes a tagged source archive only.
2. **Unsigned Windows Alpha** publishes an explicitly unsigned Windows build
   for early public testing, with complete hashes, provenance, corresponding
   source, and license materials.
3. **Signed Stable** adds protected code signing and the stricter installation
   and upgrade guarantees expected from a stable Windows release.

The absence of redistributable broadcast recordings is not a release defect.
Real recordings are legally held and tested only in the private validation
environment; public CI uses constructed protocol fixtures and publishes test
results without recording bytes, captions derived from those recordings, or
screenshots. A release may proceed with a skipped private-corpus gate when
the release notes identify the skipped gate and the public synthetic checks
pass.

## Source Release

- [ ] The tagged revision passes `scripts/check.ps1`.
- [ ] `scripts/verify-repository.ps1` reports no generated, downloaded,
  private-corpus, nested-repository, or oversized tracked files.
- [ ] All package versions and the UI version label describe the same release.
- [ ] `THIRD_PARTY_NOTICES.md`, `third_party/versions.json`, and
  `docs/dependency-licenses.md` are current.
- [ ] Architecture and backend-contract claims distinguish verified routes
  from experimental TLV/MMTP behaviour.
- [ ] `scripts/package-source.ps1` produces the source archive and checksum
  from a clean tag.
- [ ] Release notes identify known limitations and any skipped corpus gate.

## Unsigned Windows Alpha

Complete every Source Release item, then also require:

- [ ] Worker, frontend, and desktop binaries are built by the pinned workflow.
- [ ] The exact bundled libmpv binary and complete corresponding-source
  archive are produced by the same reviewed workflow and published together.
- [ ] `SOURCE-RECEIPT.json`, DLL hashes, installer hashes, and notices match.
- [ ] The installer includes MPL-2.0, libaribcaption, libmpv, and Rounded M+
  notices and leaves libmpv replaceable.
- [ ] The release title and notes say **Unsigned Windows Alpha**, explain that
  Windows may show an unknown-publisher warning, and do not imply authenticity
  through code signing.
- [ ] The release includes SHA-256 checksums for every downloadable archive,
  executable, and installer, plus the exact Git tag and commit used to build
  them.
- [ ] Native preview smoke, seek/pause/resume, overlay timing, resize/DPI, and
  long 4K performance gates pass on the packaged executable.
- [ ] Installation, uninstall, and clean-machine startup are tested; release
  notes identify any Alpha limitations and skipped private-corpus gates. Record
  packaged workflow results using `windows-alpha-acceptance.md`.

Code signing is not a prerequisite for this tier. Corresponding source and
license compliance for the bundled libmpv build remain mandatory and cannot be
waived by labeling a build Alpha or unsigned.

## Signed Stable

Complete every Unsigned Windows Alpha item, then also require:

- [ ] Release executables and installers are signed by the protected project
  signing identity.
- [ ] Signatures and certificate identity are verified after downloading the
  final public artifacts.
- [ ] Installation, upgrade, uninstall, rollback, and clean-machine startup are
  tested for every supported installer path.
- [ ] Stable release notes document the supported Windows versions and upgrade
  compatibility policy.

Until every item for a selected tier is complete, automation must not publish
that tier. Workflows may still upload private-test artifacts. In particular, no
Windows binary may be published without the exact bundled libmpv corresponding
source, receipt, hashes, and notices, whether or not signing is configured.

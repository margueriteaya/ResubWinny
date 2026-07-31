# Release checklist

ResubWinny distinguishes a public source release from a public Windows binary
release. Source publication does not imply that the current private-test
installer is ready for redistribution.

## Public source release

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

## Public Windows binary release

Complete every source-release item, then also require:

- [ ] Worker, frontend, and desktop binaries are built by the pinned workflow.
- [ ] The exact bundled libmpv binary and complete corresponding-source
  archive are produced by the same reviewed workflow and published together.
- [ ] `SOURCE-RECEIPT.json`, DLL hashes, installer hashes, and notices match.
- [ ] The installer includes MPL-2.0, libaribcaption, libmpv, and Rounded M+
  notices and leaves libmpv replaceable.
- [ ] Native preview smoke, seek/pause/resume, overlay timing, resize/DPI, and
  long 4K performance gates pass on the packaged executable.
- [ ] The release executables and installers are signed by the protected
  project signing identity.
- [ ] Installation, upgrade, uninstall, and clean-machine startup are tested.

Until every binary item is complete, workflows may upload private-test
artifacts but must not create a public GitHub Release.

# libmpv runtime

ResubWinny uses the libmpv client API as its native video playback backend. The
Windows x86_64 runtime is the LGPL build from `zhongfly/mpv-winbuild`, release
`2026-07-24-0fb136f685`, asset `mpv-dev-lgpl-x86_64-20260724-git-0fb136f685.7z`.
The build recipe commit is `9b6ccd6abbfcd6bb2dcad8946d445f670b0555ef`
and its mpv source commit is `0fb136f685c21ec10943f682bec1c90220d2d90f`.

`libmpv-2.dll` is dynamically loaded and must remain replaceable in distributed
packages. Release packaging must include the applicable LGPL notices and the
corresponding-source offer required by the selected upstream build. macOS and
Linux package their native libmpv runtime separately under this same contract.

The full LGPL text and upstream copyright/file-license inventory are stored as
`LICENSE.LGPL` and `COPYRIGHT.mpv`. See the repository root
`THIRD_PARTY_NOTICES.md` for hashes, replacement instructions, and the release
corresponding-source gate.

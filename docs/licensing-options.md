# ResubWinny licensing options

This is an engineering compatibility review, not legal advice. The repository
owner selected MPL-2.0 for ResubWinny on 2026-07-27. The alternatives below are
retained as decision history rather than active licensing choices.

## Selected: MPL-2.0

MPL-2.0 applies copyleft at the source-file level. A distributor may combine
ResubWinny with a larger proprietary product, but modifications to MPL-covered
files must remain available under MPL-2.0. This fits the project's stated goal:
prevent improvements to the caption core from returning only as opaque
binaries without making every program that invokes the Worker adopt the same
license.

Use one license for the Rust Worker, Tauri service, and Svelte application.
Keep the versioned Worker protocol and CLI usable by separately licensed tools.
The canonical root `LICENSE` text and `license = "MPL-2.0"` package metadata
are now present. Desktop bundles also include the root license.

## Stronger alternative: GPL-3.0-or-later

GPL-3.0-or-later requires distributed derivative applications to provide their
corresponding source under the GPL. It offers stronger protection against
closed forks, but makes proprietary desktop integration and some store or
appliance distribution significantly harder. Choose it only if that exclusion
is intentional.

## Split alternative: LGPL-2.1-or-later core, MPL-2.0 desktop

The Worker/core can use LGPL-2.1-or-later while the Tauri/Svelte application
uses MPL-2.0. This is useful if binary linking to a future shared core library
is a primary product goal. Today the narrow boundary is a process protocol, not
a public shared library, so the split adds contributor and release complexity
without a concrete benefit. It is not the recommended v0.1 arrangement.

## Permissive alternative: Apache-2.0 OR MIT

This is the easiest option for embedding, packaging and corporate adoption.
It permits closed modified forks and therefore does not meet the project's
original reciprocity goal. Apache-2.0 adds an express patent grant; MIT is
shorter. Dual `Apache-2.0 OR MIT` is conventional in the Rust ecosystem.

## Not recommended: Anti 996 License 1.0

Anti 996 License 1.0 is not an appropriate operative license for a project
that intends to be distributed as open-source software. The reviewed upstream
text labels itself a draft and conditions use on compliance with the strictest
applicable labour rules across several jurisdictions. Those conditions are an
ethical-use restriction, not ordinary copyright reciprocity. They conflict
with the Open Source Definition's non-discrimination principles and the
license is not present on the OSI-approved license list.

It should not be combined with MPL-2.0:

- `MPL-2.0 OR Anti-996-1.0` lets every recipient choose MPL-2.0 and therefore
  does not enforce the Anti-996 conditions.
- `MPL-2.0 AND Anti-996-1.0` adds non-standard restrictions to MPL-covered
  files, creates uncertain compatibility and enforcement, and is likely to be
  rejected by package repositories and downstream distributors.

ResubWinny may state support for fair labour practices in its README, code of
conduct, or project governance without turning that position into a software
use restriction. Such a statement must be identified as non-binding and must
not be presented as part of the MPL-2.0 grant.

## Third-party and asset boundaries

The project license does not replace dependency obligations:

- `libaribcaption` is MIT and its copyright/license notice must be retained.
- The bundled Windows `libmpv-2.dll` is an LGPL build. It must remain
  replaceable, ship the applicable LGPL notices, and have a valid
  corresponding-source mechanism for that exact build.
- Rounded M+ 1m for ARIB is redistributed under the M+ FONT LICENSE. Its
  additional WadaLabMaruGo2004ARIB glyph source permits modification,
  commercial use, bundling, and redistribution and is published as public
  domain/Unlicense by its current upstream. The exact bundled binary matches
  `5ym/arib-font` by SHA-256. Its provenance and both grants are recorded next
  to the font in `third_party/rounded-mplus-1m-arib/` and must ship with bundles.
- `makeding/aribb62.js` declares MIT in package metadata but had no standalone
  license file at the reviewed commit. Behavioural references are documented;
  copied source must not be vendored without a redistributable notice.
- Broadcast corpus files are not covered by the source license. Public tests
  must use redistributable constructed fixtures, hashes, or independently
  licensed excerpts.

Before public distribution, add `THIRD_PARTY_NOTICES.md` and include the
relevant third-party notices in every installer.

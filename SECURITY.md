# Security policy

## Supported versions

ResubWinny is currently an Alpha project. Security fixes are applied to the
latest source revision and the newest `0.1.x` development package only. Older
development binaries are not maintained.

Native preview is release-tested on Windows. macOS and Linux desktop preview
backends are deferred and are not currently supported security surfaces. The
cross-platform Worker and build checks do not imply native preview support.

## Reporting a vulnerability

Do not publish an issue containing an exploitable broadcast sample, private
recording, credential, or working proof of concept. Use the repository host's
private vulnerability-reporting feature when available. Include:

- affected revision and platform;
- input route and the smallest legally shareable reproducer;
- expected and observed behaviour;
- whether the issue crosses a length, memory, filesystem, IPC, native-library,
  or archive boundary; and
- crash diagnostics with personal paths and programme data removed.

Until a public repository has a private reporting channel, contact the project
owner privately and request one before sending sensitive material. The project
will acknowledge a complete report, assess affected versions, and coordinate a
fix and disclosure window. No response-time guarantee is made during Alpha.

## Security boundaries

Recordings, TTML/XML, DRCS, archives, language packs, and dependency artifacts
are untrusted input. Parsers must enforce length and allocation limits. The
WebView does not receive video frames and does not parse media or calculate
caption layout. Runtime components must not download or silently replace
libmpv, libaribcaption, fonts, or language packs.


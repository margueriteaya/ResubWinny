[简体中文](corpus.md) | [English](corpus.en.md) | [日本語](corpus.ja.md) | [繁體中文](corpus.zh-TW.md)

> This is a translation. The Simplified Chinese version is the sole authoritative source.

# Local broadcast regression corpus

Broadcast recordings are deliberately not committed or redistributed. Put legal
local samples in any directory and set `ARIB_FIXTURE_DIR` to that directory.
The tests deliberately do not fall back to an implicit developer path, so a
long-fixture run always names the corpus it is about to read.

```powershell
$env:ARIB_FIXTURE_DIR = 'C:\tvrecords_testfile'
$env:ARIB_LONG_FIXTURE = '1'
cargo test -p arib-caption-worker decodes_ -- --nocapture
```

The opt-in checks stream the complete inputs and assert the following current
baseline. They do not publish source bytes, captions, or screenshots.

The corpus deliberately prioritizes content-verified recordings users can
realistically obtain: the terrestrial MPEG-TS sample and the 192-byte
MPEG-TS/TTML sample are the release gates. The latter is a packetised MPEG-TS
recording and must not be used as evidence that native BS4K TLV was captured.
TLV/MMTP has no equivalent local release fixture at present; its parser,
signalling limits, and raw-evidence contract are covered by bounded
constructed tests until a lawful real capture becomes available.

Public protocol fixtures are available from the worker's `synthetic` module:
`make_ts_packet`, `make_pat`, `make_pmt`, `make_pes`, `make_b24_data_group`, and
`make_mmtp_packet` construct deterministic
packet and section boundaries for parser tests without embedding broadcast
recordings or claiming broadcaster-specific semantics.

For a release-artifact smoke check without a full scan, run:

```powershell
$env:ARIB_FIXTURE_DIR = 'C:\tvrecords_testfile'
.\scripts\validate-corpus.ps1
```

Add `-Long` to run both complete conversions into a temporary validation
directory. The script never writes outputs into the corpus directory.

| Fixture | Route | Release status / required evidence |
| --- | --- | --- |
| `chijo_digital_test.ts` | ISDB-T MPEG-TS / ARIB STD-B24 | **Release gate.** 18,579,078,944 input bytes; 13,653 PES; 2,230 scenes; 2,736 regions; 29,892 characters; 61 DRCS glyphs; 0 decoder errors. NIT network name, current EIT programme metadata and TDT/TOT broadcast time must all be present. |
| `bs4k_test.m2ts` | 192-byte recorder M2TS / private PES / ARIB-TTML | **Release gate.** 11,517,020,160 input bytes; 330 PES; 422 TTML captions; 5,051 characters; 0 parser errors. Same-time region association currently records 31 structured Ruby bindings before archive/ASS output, including `ささ` to the single base grapheme `捧`. |
| `bs4k_test_2.ts` | 188-byte recorder MPEG-TS / ARIB STD-B24 | **Release gate.** 3,089,047,552 input bytes; service 101 decodes from ARIB SI as `NHK　BSP4K`; NIT network name, current EIT programme metadata and TDT/TOT broadcast time must all be present; PID 0x0130 has 2,038 PES, 118 captions, 157 regions, 1,661 characters and 0 decoder errors; separately advertised PID 0x0138 has no caption event and must remain an empty result rather than a fabricated second track. |
| Local 38.07 GB Paris recording (not redistributed) | 192-byte M2TS / private PES / sequential ARIB-TTML | **General-route regression.** Content probing finds service 101, PMT `0x0100`, caption PID `0x1C00` (`component_tag 0x30`) and independent superimpose PID `0x1C01` (`0x38`). The XML has complete TTML namespaces but omits element timing; invalid zero-filled PES PTS is rejected and the wrap-aware M2TS arrival clock closes each document at the next same-PID document. Complete default-caption conversion reads 38,065,729,536 bytes and must retain 2,715 caption regions, 28,618 characters and 0 decoder errors with monotonically ordered output through 03:11:48. It must not use the filename, service ID, programme name or fixed PID values as a routing exception. |
| Local 20.12 GiB BS recording (not redistributed) | 188-byte MPEG-TS / PMT version and caption-PID transition | **Dynamic-PMT regression.** The initial PMT exposes only superimpose PID `0x1C12` (`component_tag 0x38`); a later current PMT adds caption PID `0x1201` (`component_tag 0x30`). Inspection must report only `0x1201`. Complete conversion reads 21,609,477,452 bytes and yields 18,722 selected PES, 3,825 scenes, 6,679 regions, 70,853 characters, 7 DRCS glyphs and 0 decoder errors. Raw evidence must contain only PID `0x1201`. |
| Constructed PMT-version transition TS | MPEG-TS / B24 caption versus superimpose | Fixed-size discovery windows must find a later caption component after an initial superimpose-only PMT; sequential decode must route only the selected logical `service_id + component_tag` and reject the superimpose PES. |
| Constructed 188-byte private-PES TS | MPEG-TS / PMT private PID / strict ARIB-TTML | B24 discovery remains empty; private PID is discovered; conversion, ASS, TTML, archive, raw PES evidence, and bounded preview all yield one validated TTML caption. |
| `testdata/golden/b62-layout.xml` | Constructed ARIB-TTML semantic fixture | Stable JSON summary verifies nested timing, percentage regions, horizontal ruby evidence, vertical writing mode, font size, and colour without redistributing broadcast material. Unit regression also verifies that equivalent declared 1920×1080, 3840×2160, and 7680×4320 pixel layouts normalise to identical logical viewer geometry and text lengths. |
| Constructed TLV/MMTP `stpp` fixtures | ISDB-S3 TLV → MMTP → MPT/MPU | **Experimental only.** Verifies bounds, fragment loss, provenance and evidence-first `stpp` routing; it cannot promote TLV/MMTP to a release-gated route. |

Parser fuzzing is kept outside the release workspace in `fuzz/`. The initial
targets cover content-based TS/TLV probing, strict TTML envelope decoding,
bounded ARIB SI service-name text decoding, 188/192-byte TS PSI/PES metadata
parsing, and MMTP/TLV payload
envelopes. `cargo check --manifest-path fuzz/Cargo.toml`
provides a stable-toolchain compile check; CI additionally builds all targets
with `cargo-fuzz` on Linux nightly. PSI/PES/B24 state-machine and deeper
signalling/MPU semantic fuzz targets remain future corpus work; the weekly
workflow runs every declared target for a bounded interval.

For visual or format changes, create outputs in an ignored validation directory
and compare the project archive, ASS, TTML, raw PES evidence, and unresolved
DRCS asset directory. The complete commands are:

```powershell
.\build\cargo\release\arib-caption-worker.exe convert `
  "$env:ARIB_FIXTURE_DIR\chijo_digital_test.ts" `
  artifacts\validation\chijo_digital_test.ass --ttml --archive --raw

.\build\cargo\release\arib-caption-worker.exe convert `
  "$env:ARIB_FIXTURE_DIR\bs4k_test.m2ts" `
  artifacts\validation\bs4k_test.ass --ttml --archive --raw
```

The M2TS sample is particularly important: its private PES envelope has
non-UTF-8 bytes before a valid TTML document. A regression must not reject the
entire PES merely because its transport framing is not UTF-8; XML text itself is
decoded strictly from its declared encoding or BOM.

The Windows native preview smoke gate uses the B24 `bs4k_test_2.ts` sample and
does not distribute the recording:

```powershell
$env:ARIB_FIXTURE_DIR = 'C:\tvrecords_testfile'
.\scripts\validate-preview.ps1 -FixtureDirectory $env:ARIB_FIXTURE_DIR
```

It validates WGL host creation, in-process libmpv loading, render-worker
startup, recording opening and clean shutdown. It is deliberately separate from
visual screenshot acceptance and must not be interpreted as a hardware-decoding
or pixel-fidelity claim.

Add `-Long` for the thresholded 120-second Windows 4K gate:

```powershell
.\scripts\validate-preview.ps1 `
  -FixtureDirectory $env:ARIB_FIXTURE_DIR `
  -Long
```

The gate keeps a 3840x2160 native surface active, replaces a complete
1920x1080 backend caption plane three times, and exercises pause, resume, exact
seek and shutdown. It fails below 20 presents/s, above 10 s startup, 1 s
control or caption upload, 3 s shutdown, 2048 MiB working set, or 512 MiB
working-set growth after 4K warm-up. The schema-versioned result is written to
`build/validation/preview-performance-windows-4k.json`.

The 2026-07-30 real-corpus baseline sustained 34.74 presents/s for 120 seconds
using `d3d11va-copy`; peak working set was 1526.9 MiB and post-warm-up growth
was 111.9 MiB. This completes the Windows 4K long gate only. It is not an 8K,
cross-platform, DPI, or visual-fidelity acceptance result.
The harness itself uses Cargo's test profile while loading the same bundled
libmpv DLL and native WGL route as the application; packaged-release acceptance
is tracked separately.

Packaged Windows acceptance on 2026-07-31 used the final release executable
with `bs4k_test_2.ts`. Content probing selected 188-byte MPEG-TS/B24, native
libmpv presented video while initially paused, EIT/NIT/TOT-derived channel,
network, programme, description, and broadcast time were visible, and PID
`0x0130` produced 118 decoded events. The task timeline repopulated with real
caption bars after the streaming archive changed from `.jsonl.part` to its
published `.jsonl` path, with no archive-not-found diagnostic. This is a
packaged regression result for that route, not acceptance of every desktop
workflow or a macOS/Linux preview claim.

## Streaming memory release gate

Bounded parser constants are necessary but not sufficient evidence for large
recordings. A release candidate must also complete at least one 1 GiB-or-larger
TS/M2TS conversion with a Worker peak working set no greater than **384 MiB**:

```powershell
.\scripts\validate-memory.ps1 `
  -Source "$env:ARIB_FIXTURE_DIR\chijo_digital_test.ts" `
  -TrackId 276
```

The script reports the absolute peak and peak MiB per input GiB. The absolute
gate catches accidental whole-timeline/PES retention; comparing the ratio over
the 3 GiB, 11 GiB and 18 GiB fixtures checks that memory does not grow linearly
with recording duration. Generated outputs stay in an isolated temporary
directory and are removed after the measurement.

Measured Windows x86-64 release baselines on 2026-07-27:

| Fixture | Input | Peak working set | Peak/input ratio |
| --- | ---: | ---: | ---: |
| `bs4k_test_2.ts`, PID 0x0130 | 2.877 GiB | 22.5 MiB | 7.83 MiB/GiB |
| `chijo_digital_test.ts`, PID 0x0114 | 17.303 GiB | 35.7 MiB | 2.06 MiB/GiB |

The sixfold input-size increase raised the absolute peak by only 13.2 MiB,
which is consistent with bounded streaming rather than whole-recording
retention. These numbers are a regression baseline for this machine, not a
promise that every decoder/runtime build has identical allocator overhead.

# libaribtlv B62 extraction backend

[简体中文](libaribtlv-poc.md) · [繁體中文](libaribtlv-poc.zh-TW.md) · [日本語](libaribtlv-poc.ja.md) · [English](libaribtlv-poc.en.md)

> **Normative note:** Simplified Chinese is the sole authoritative version. If a translation conflicts with it, the Simplified Chinese document prevails.

The optional Worker `libaribtlv` feature provides a bounded native TLV/MMTP demux path for ARIB STD-B62 subtitles. It is an implementation increment for the experimental, evidence-first TLV route; it is not a general BS4K/8K support claim and includes no player or MSE integration.

The reviewed dependencies are `makeding/libaribtlv` 0.6.1 (C API version 6, commit `a84e5b62bf9230d3fcea21c66e62f7cc5d50a3c2`) and Zlib 1.3.2 (commit `da607da739fa6047df13e66a2af6b8bec7c2a498`). Their complete source snapshots are vendored under `third_party/`, pinned by `third_party/versions.json`, and recorded in `THIRD_PARTY_NOTICES.md`. Neither runtime nor feature builds download them.

## Build and test

The project-owned bridge statically builds libaribtlv and its private Zlib from the vendored snapshots. No `CMAKE_PREFIX_PATH`, external checkout, or system Zlib is required:

```powershell
cargo test -p arib-caption-worker --features libaribtlv
```

The narrow C ABI exposes only subtitle tracks, access units, same-MPU subtitle resources, normalized timestamps, random-access/discontinuity metadata, and parser errors. Rust copies all callback-lifetime strings and byte views before returning. ARIB-HTML5 application resources and audio/video access units are not collected.

## Routing and evidence rules

When enabled, the native backend takes over TLV-to-B62 TTML scanning with bounded streaming chunks. The archive separately retains packet/track identity, available MPU/MMTP sequences, normalized rational PTS, its time origin, discontinuities, and actual MPT presentation NTP. Missing values remain absent; PTS is never labelled as NTP and NTP is never guessed as PTS.

Only compression type 0 enters the existing strict, self-contained XML TTML decoder. Compression types 1/2 (EXI), unknown compression/format/data types, non-self-contained XML, malformed documents, and incomplete resources retain raw evidence and diagnostics only. Same-MPU resources are complete only when the demuxer supplies an MPU scope.

General BS4K/8K support must not be claimed until lawful real-stream corpus tests and trusted reference captures pass. Public tests use constructed protocol fixtures; private broadcast recordings are never redistributed.

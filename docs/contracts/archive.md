# Caption archive contract

The caption archive is a UTF-8 JSON Lines (`.caption.jsonl`) format. It is the
project's durable intermediate representation; ASS, TTML, and preview output
may be derived from it without treating those presentation formats as
lossless.

## Header and schema version

The first complete line is an archive header:

```json
{"type":"arib_caption_studio_archive","schemaVersion":1,"version":1,"source":"recording.ts","route":"arib_std_b24","format":"jsonl"}
```

`schemaVersion` is the authoritative archive compatibility version. Version 1
also writes the original `version` field as a compatibility alias; the two
values must agree. New writers must not silently change the meaning or shape
of existing records without incrementing `schemaVersion`.

Readers that only need bounded timeline or preview records may ignore unknown
record types. A reader that needs complete semantic fidelity must reject an
unsupported `schemaVersion` instead of guessing. Files produced before the
explicit `schemaVersion` field used `version: 1` and remain version 1 archives.

## Records

Every following complete line is an independent JSON object with a stable
`type`. Caption payload records use an envelope shaped as
`{"type":"caption","value":{...}}`; other current types include
`region_interval`, `scene`, `resource_reference`, `resource_evidence`,
`asset_evidence`, and `summary`.

The writer flushes complete caption records while conversion is running so
the desktop can tail the file. Readers must ignore an incomplete final line
until a later append completes it. Transport-specific B24 and B62 evidence
remains distinct; common semantics are represented in caption records rather
than by pretending the transports share a decoder model.

Inside the Worker, both routes cross the closed, zero-copy `CaptionCueRef`
semantic boundary before archive publication. It standardises timing, region,
route identity, plain text, ruby count, and DRCS presence while retaining each
route's faithful payload. Style, glyph pixels, and TTML resource evidence stay
route-specific. Schema v1
therefore continues to publish B24 as `region_interval` and ARIB-TTML as
`caption`; the shared internal boundary does not relabel or duplicate records.

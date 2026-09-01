[简体中文（权威）](b62-compatibility.md) | [English](b62-compatibility.en.md) | [日本語](b62-compatibility.ja.md) | [繁體中文](b62-compatibility.zh-TW.md)

> This is a translation. The Simplified Chinese version is the sole authoritative source.

# ARIB STD-B62 / ARIB-TTML compatibility

ResubWinny treats ARIB-TTML as a caption-data format, not as browser CSS. The
transport and XML decoder remain independent from the renderer, and unknown
assets stay raw evidence rather than being guessed into captions.

The viewer-facing visual baseline is
[`libaribcaption` screenshot0](visual-reference.md): B24 remains
libaribcaption-rendered RGBA, while B62 work must converge on the same logical
plane, font/ruby/background/stroke relationships without using browser layout.

The project reviews `makeding/aribb62.js` as a public behaviour reference.
The reviewed `74304d40a5b8556be1148e123ae70d60f937ecf5` package metadata
declares MIT, but the repository and GitHub license endpoint currently provide
no standalone `LICENSE` file. ResubWinny therefore ports independently verified
semantics into the Rust backend and does not vendor its source until an
redistributable copyright notice and license text are available. In particular,
its browser-oriented stroke rendering is not considered a normative ARIB
implementation and must not be silently promoted to the archive model.

## Current semantic mapping

| ARIB-TTML concern | ResubWinny behaviour |
| --- | --- |
| `lrtb`, `rltb` | canonicalised to TTML `horizontal-tb` and retain derived `ltr`/`rtl` direction unless a source `tts:direction` explicitly overrides it; the native preview uses bounded character-cell RTL placement, not general Unicode bidirectional shaping |
| `tblr` | canonicalised to `vertical-lr` |
| `tbrl` | canonicalised to `vertical-rl` |
| `arib-tt:ruby` / `ruby` / `rt` | preserved in safe inline TTML bodies and archive records; the basic horizontal native preview resolves an `arib-tt:ruby` annotation span to its `xml:id` base span and removes the annotation from inline body rendering |
| inherited `div` timing and styles | resolved before caption intervals are emitted |
| standard named TTML colours | `black`, `white`, `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, and `transparent` are parsed natively, case-insensitively, in addition to existing `#RRGGBB[AA]` support; no browser CSS colour parser is used |
| horizontal `br`/newline, `textAlign`, `displayAlign`, `lineHeight` | the native preview keeps explicit line breaks, lays out each bounded line using `start`/`end`/`left`/`right`/`center`, and positions the line block using `before`/`center`/`after`. `start` and `end` observe the resolved LTR/RTL direction. This is native RGBA layout, not a browser fallback |
| declared or evidenced display plane | a valid root `tts:extent` is authoritative and defines the source coordinate space. Without it, logical 2K remains the default; the parser infers only canonical 3840×2160 or 7680×4320 when complete pixel `origin`/`extent` geometry exceeds logical 2K on at least one axis and remains within that plane. Region origin/extent use independent horizontal/vertical ratios; pixel font size, line height, letter spacing, and direct outline widths use the bounded uniform ratio. The current `1920×1080` RGBA plane is only an intermediate backend texture. Acceptance is based on proportional mapping into the video-content viewport excluding black bars, preserving the viewer-relative area across window size, DPI, and fullscreen changes; ambiguous input is never guessed |
| `subt://` images/fonts and `smpte:image` | numeric `subt://<index>` references are resolved only against the same `packet_id + mpu_sequence_number` resource state. When a bounded `subsampleNumber` resource is present, the archive writes a lossless `resource_evidence` record keyed by that scope plus subsample number, preserving data type, byte length, bounded format validation and base64 payload. The archive preview reader exposes only matching small structurally complete PNGs as low-frequency resource previews; font and non-PNG resources remain evidence, not rendered text. Missing or incomplete maps remain explicitly `unresolved`. Discovered MPT assets are emitted as bounded `asset_evidence` records, and complete non-`stpp` MPU/MFU payloads can be extracted by `dump-tlv` as `mmt_asset_payload` raw evidence with a matching scope key |
| horizontal text with explicit `origin`/`extent` | the backend can rasterise it into a bounded 1920×1080 RGBA plane with the bundled Rounded M+ 1m for ARIB font, using source foreground/background RGBA. Missing bundled-font glyphs are counted and left blank rather than replaced with tofu or a generic glyph. This is an initial native preview path, not a full B62 renderer |
| `vertical-lr` / `vertical-rl` | the backend has a bounded native vertical mode: it advances character cells vertically, opens a new column on region overflow, and observes left/right column direction. It maps punctuation with an explicit Unicode vertical-presentation form when that form exists in the bundled ARIB font. CJK/full-width glyphs remain upright; ASCII and Latin glyphs use a native clockwise bitmap rotation, while unclassified scripts remain upright rather than being guessed. An explicitly associated ruby annotation is rasterised beside its base cells, including a bounded continuation across automatically wrapped columns (`ttml-vertical-ruby-basic-native`). The annotation defaults to half the base font size, but its explicit `tts:color`, `tts:fontSize`, `tts:letterSpacing`, direct opacity, and supported direct `tts:textOutline` are retained. A direct `tts:textCombine="all"` or `digits` span containing one or two ASCII digits is rasterised horizontally within one vertical cell; longer runs remain vertical. Complete B62 orientation tables and source-specific ruby placement remain pending lawful corpus comparison. |
| safe `rich_body` span style | bounded token extraction retains ordinary body text between tags and applies each source span's explicit foreground colour, font size, letter spacing, and direct opacity to the native text preview. Explicitly associated ruby text (`tts:ruby="text"` or `arib-tt:ruby`) remains structural instead of being inlined, and carries its own supported annotation presentation properties. |
| horizontal `ruby` base/text pairs | the native preview associates a `tts:ruby="text"` span with the immediately preceding contiguous `tts:ruby="base"` group, or an `arib-tt:ruby` annotation span with its `xml:id` base span; one annotation is centred across the entire resolved base group. Annotation font size defaults to 0.5 of the base font size, while explicit supported annotation colour, font size, letter spacing, opacity, and direct outline take precedence. The snapshot reports `ttml-horizontal-ruby-basic-native` plus a rendered-ruby count. Non-contiguous/overlapping source-specific B62 ruby placement remains metadata until corpus comparison proves a placement rule. |
| direct TTML `tts:textOutline` | a conservative native preview mapping accepts direct TTML named colours or `#RRGGBB`/`#RRGGBBAA` plus a `px` width, accepts `none`, clamps the radius to 1–4 pixels, and applies inherited opacity. Rounded M+/`丸ゴシック` captions without a repeated outline declaration use the receiver-baseline 2 px black stroke and are protected by a native PNG golden; explicit `none` disables it. Unsupported syntax remains metadata rather than becoming an invented outline |
| `arib-tt:border` and browser stroke CSS | not converted to `tts:textOutline` automatically; this avoids claiming non-standard outline equivalence |
| unknown writing modes or extensions | retained as source style metadata and reported through the diagnostic/raw route |

ASS remains an approximation. It can preserve position, colour, font size,
and selected text styling, but it is not a lossless representation of B62
writing, ruby, animation, bitmap resources, or broadcast stroke semantics.

## Planned increments

1. Compare the implemented bounded ruby grouping and conservative vertical-orientation path against lawful B62 captures; extend only rules demonstrated by the corpus.
2. Compare the implemented receiver-baseline stroke golden with user-validated
   ARIB captures before extending it to any additional font family or syntax;
   never infer those extensions from browser `text-shadow` or
   `-webkit-text-stroke`.
3. Preserve native visual goldens for the current B24 RGBA compositor and
   basic horizontal-ruby TTML plane; add B62 fixtures with nested timing,
   vertical ruby, resource URLs, and unsupported extensions only when they
   can be compared with lawful reference captures.

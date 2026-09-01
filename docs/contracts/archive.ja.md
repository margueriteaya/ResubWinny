[简体中文](archive.md) · [繁體中文](archive.zh-TW.md) · [日本語](archive.ja.md) · [English](archive.en.md)

> 本ページは翻訳です。簡体字中国語版のみを正式な情報源とします。

# 字幕 archive 契約

字幕 archive は UTF-8 JSON Lines（`.caption.jsonl`）形式で、プロジェクトの永続的な中間表現です。
ASS、TTML、preview の出力はここから派生できますが、それらの表示形式を lossless とは扱いません。

## ヘッダーと schema version

最初の完全な行は archive header です。

```json
{"type":"arib_caption_studio_archive","schemaVersion":1,"version":1,"source":"recording.ts","route":"arib_std_b24","format":"jsonl"}
```

`schemaVersion` が archive 互換性の正式なバージョンです。Version 1 は互換用の別名として従来の
`version` フィールドも書き込み、両者は一致しなければなりません。新しい writer は
`schemaVersion` を増やさずに既存レコードの意味や形を暗黙に変更してはなりません。

上限のある timeline または preview レコードだけを必要とする reader は未知の record type を
無視できます。完全な意味忠実度が必要な reader は、推測せず未対応の `schemaVersion` を拒否します。
明示的な `schemaVersion` フィールドの導入前に生成され、`version: 1` を使用していたファイルも
version 1 archive のままです。

## レコード

以降の完全な各行は、安定した `type` を持つ独立した JSON object です。字幕 payload record は
`{"type":"caption","value":{...}}` という envelope を使用します。現在のその他の type には
`region_interval`、`scene`、`resource_reference`、`resource_evidence`、`asset_evidence`、`summary` があります。

変換中、writer は完全な字幕 record を flush し、デスクトップがファイルを追尾できるようにします。
reader は、後の追記で完成するまで未完了の最終行を無視しなければなりません。B24 と B62 の
transport 固有 evidence は分離したままにし、共通 semantics は caption record で表現します。
両 transport が同じ decoder model を共有するかのように扱ってはなりません。

Worker 内では両ルートとも、archive に公開する前に閉じた zero-copy の `CaptionCueRef` semantic boundary を
通過します。各ルートに忠実な payload を保持しつつ、timing、region、route identity、plain text、
ruby count、DRCS presence を標準化します。Style、glyph pixel、TTML resource evidence はルート固有のままです。
したがって schema v1 は引き続き B24 を `region_interval`、ARIB-TTML を `caption` として公開し、
共有内部境界によって record のラベル変更や複製は行いません。

ARIB-TTML の `caption.value` には optional `source_layout` を含めることができます。source display plane の寸法と根拠
（`declared`、`inferred`、`legacy_logical2k`）、source region geometry、未 scale の style、安全な inline TTML を保持します。
既存の `x`、`y`、`width`、`height`、`style`、`rich_body` は logical 1920×1080 compatibility view として維持されます。
新しい reader は実際の video-content viewport への mapping で `source_layout` を優先し、これが無い旧 archive は
`LegacyLogical1920x1080` として扱います。schema-v1 reader はこの optional field を無視できるため、旧 file の migration は不要です。
source semantics を保存せず誤って scale された archive は確実に逆算できず、適法な source recording から再抽出する必要があります。

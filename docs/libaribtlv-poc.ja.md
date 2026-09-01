# libaribtlv B62 抽出バックエンド

[简体中文](libaribtlv-poc.md) · [繁體中文](libaribtlv-poc.zh-TW.md) · [日本語](libaribtlv-poc.ja.md) · [English](libaribtlv-poc.en.md)

> **規範上の注記：**簡体字中国語版だけが正式版です。翻訳と矛盾する場合は簡体字中国語版を優先します。

任意の Worker `libaribtlv` feature は、ARIB STD-B62 字幕向けの有界な native TLV/MMTP demux 経路を提供します。これは実験的かつ evidence-first な TLV route の実装追加であり、汎用 BS4K/8K 対応の主張ではなく、player/MSE integration も含みません。

レビュー済み依存関係は `makeding/libaribtlv` 0.6.1（C API version 6、commit `a84e5b62bf9230d3fcea21c66e62f7cc5d50a3c2`）と Zlib 1.3.2（commit `da607da739fa6047df13e66a2af6b8bec7c2a498`）です。完全な source snapshot は `third_party/` に同梱し、`third_party/versions.json` で固定して `THIRD_PARTY_NOTICES.md` に記録します。runtime と feature build はこれらを download しません。

## Build と test

project 所有 bridge は vendored snapshot から libaribtlv と private Zlib を static build します。`CMAKE_PREFIX_PATH`、外部 checkout、system Zlib は不要です。

```powershell
cargo test -p arib-caption-worker --features libaribtlv
```

狭い C ABI は subtitle track、access unit、同一 MPU の字幕 resource、normalized timestamp、random-access/discontinuity metadata、parser error だけを公開します。Rust は callback から戻る前に短寿命の string と byte view をすべて copy します。ARIB-HTML5 application resource と audio/video access unit は収集しません。

## Routing と evidence 規則

feature 有効時は native backend が TLV→B62 TTML scan を引き継ぎ、有界 chunk で stream 処理します。archive は packet/track identity、利用可能な MPU/MMTP sequence、normalized rational PTS とその time origin、discontinuity、実際の MPT presentation NTP を分離して保持します。欠損値は欠損のままとし、PTS を NTP と記録せず、NTP から PTS を推測しません。

既存の strict self-contained XML TTML decoder に渡すのは compression type 0 だけです。compression type 1/2（EXI）、未知の compression/format/data type、非 self-contained XML、malformed document、不完全な resource は raw evidence と diagnostic のみ保持します。同一 MPU resource は demuxer が MPU scope を提供した場合だけ complete とします。

合法な実 stream corpus と信頼できる reference capture が合格するまで、汎用 BS4K/8K 対応を主張してはなりません。公開 test は構築した protocol fixture を使い、非公開の放送録画を再配布しません。

# アーキテクチャ基準（日本語訳）

> 本文書は `architecture.zh-CN.md` の訳文です。表現に相違または曖昧さがある場合は、中国語版を正とします。

> 第三段階の core 実装は完了しています。現在の Alpha で native preview
> の release 対象となる platform は Windows です。macOS/Linux の native
> preview backend は明示的に延期し、現段階の acceptance scope には含めません。
> 残る renderer 作業は、標準 B62 stroke、resource の完全描画、独立した
> 2K/8K・DPI・screenshot difference gate の品質収束です。zero-copy の
> WGL/D3D interop は現在の製品保証ではありません。

## 収束期の境界（2026-08-29）

この段階ではフロントエンドの技術スタックと Rust crate の構成を固定する。
Svelte、Tauri、既存の `arib-caption-worker` は変更しない。中央状態の整理は
feature session を優先する。Worker は入力、probe/demux、decode、Caption IR、
export、archive、evidence を担当し、Tauri は task history、queue、settings、
window lifecycle、native preview を担当する。`resubwinny-core` crate の分割は、
Caption IR、time model、transport API が安定し複数の consumer が現れた後に再評価する。

同じ収束期では、BD/DVD bitmap subtitle OCR、plugin system、AI translation、
macOS/Linux native preview を明示的に延期する。DRCS は local hash → Unicode
mapping の改善に限定し、汎用 OCR system へ拡張しない。

## 対象と非対象

日本の ISDB 録画ファイル向け、オープンソースかつクロスプラットフォームの字幕抽出・変換・保存・診断ツールです。伝送層では従来の MPEG-2 TS と BS4K/8K native TLV/MMT を区別します。`.ts`、`.m2ts`、`.tlv`、`.mmts` はファイル名の手掛かりに過ぎず、伝送形式の証拠にはしません。現在の release fixture は従来 TS と 192-byte MPEG-TS/TTML 録画です。native TLV/MMT は BS4K/8K の規範的 route ですが、合法な実 capture が得られるまで実装は実験的とします。対応できる route では ARIB 字幕の意味、レイアウト、特殊文字、出所情報を可能な限り保持します。録画管理、プレーヤー、映像/音声復号、EPG、CAS、汎用 MMT フレームワーク、ライブ受信は対象外です。旧ツール、`bs4kass.exe`、Caption2Ass は調査・比較用に限り、配布物へ同梱しません。

ResubWinny の Worker、Tauri service、Svelte frontend は MPL-2.0 で提供します。third-party library、binary、font、corpus material には、それぞれの license と provenance 要件が引き続き適用されます。

## 必須構成

worker の `main.rs` は `lib.rs` が公開する `run()` を呼ぶだけです。module
登録、共有 export、test entry は library 側に置き、conversion core を
process launcher から独立して再利用できるようにしています。

```text
Tauri 2 + Svelte 5 desktop GUI（WebView は表示のみ）
  -> background task、低頻度の進捗、取消、診断
GUI/CLI 共通の Rust conversion core
  -> 有界な順次 I/O、解析、時間軸、原子的公開
プロジェクト字幕モデルと exporter
  -> 小さく安定した C ABI
libaribcaption
```

GUI は唯一の入口ではなく、UI thread が録画バイトの読取り、パケット単位の受信、全時間軸の保持、demux、最終組版を行いません。conversion core は CLI からも呼び出せます。現在の GUI は同じ core を background thread で実行し、協調的な取消、進捗、atomic output を提供します。cross-process の crash isolation が必要になった時だけ sidecar を追加し、single-EXE delivery をそのために犠牲にしません。巨大なローカルファイルは Rust のブロッキング buffered I/O を既定とし、188 バイトの TS packet ごとに async task や channel message を作りません。

## ストリーミングと復旧の不変条件

- ファイルサイズは通常時メモリを決めてはならず、1 GB と 200 GB で同程度に保つ。
- 録画全体/全時間軸をロード、demux、細粒度 index、保持せず、frontend JavaScript で放送データを処理しない。
- 入力、resync、PES、MPU、活動 scene buffer に上限を設け、不可信の length から無制限に allocate しない。
- probe 後は対象 service/caption PID または asset だけを保持し、映像/音声の decode や完全な PES 再構成をしない。
- 可能な限り借用 slice を使い、packet/fragment をまたぐ場合または長期保持時のみ copy し、DRCS は hash で deduplicate する。
- 既定は順次 read であり whole-file mmap ではない。将来は file、stdin/pipe、分割ファイル、成長中の録画へ拡張可能にする。
- checkpoint はファイル識別（size、mtime、先頭/末尾 block hash）、byte offset、continuity、unwrap 済み PTS、B24 management/DRCS state、安全な出力位置を保存する。完全 state 復元が安全でなければ信頼できる sync point まで戻って短区間を再解析する。
- `.part`、一時 event body、DRCS asset、checkpoint を書き、成功時だけ原子的に公開する。取消/失敗時は未完了状態とログ/復旧情報を残す。スリープ抑止は手動・既定 off・実行中のみ有効とする。

## 入力 route

```text
MPEG-2 TS -> PAT/PMT -> subtitle PES -> ARIB STD-B24 data groups
TLV -> IPv6/圧縮 IP -> UDP -> MMTP -> signalling -> caption asset -> MPU
```

従来 route は service、PID、language、caption/superimpose type、PCR/PTS/DTS、source offset、不連続、warning を保持します。BS4K/8K の初期範囲は録画ファイルのみで、MMT package と caption asset の発見、関連 MPU の再構成、timestamp 復元、共通字幕コアへの投入に限定します。HEVC/音声 decode、完全 SI/EPG、CAS、ライブ、汎用 MMT は含めません。拡張子でなく内容により TS/TLV/MMTP/破損/途中入力を判定します。

### 信号別 ARIB 規格対応

これは規格レイヤーの対応表であり、filename だけで route を決める規則ではありません。version は 2026-07 に確認した ARIB 公開カタログのものです。実際には録画 stream 内の signalling、descriptor、payload を正とします。

| 信号 | 物理/transport layer | service/track 発見 | 字幕 coding/presentation | demux の入口 |
| --- | --- | --- | --- | --- |
| 地上波 2K（ISDB-T） | ARIB STD-B31、録画は通常 MPEG-2 TS | MPEG-2 PSI と ARIB STD-B10 SI | ARIB STD-B24 の字幕/文字スーパー data。B24 data group は subtitle PES で届く | PAT/PMT -> subtitle PES -> B24 data group |
| BS/広帯域 CS 2K | ARIB STD-B20、録画は通常 MPEG-2 TS | MPEG-2 PSI と ARIB STD-B10 SI | ARIB STD-B24。単一の `stream_type` や component tag heuristic を完全な規則としない | PAT/PMT -> subtitle PES -> B24 data group |
| BS4K/8K（高度広帯域衛星/ISDB-S3） | ARIB STD-B44 は TLV を含む ISDB-S3、ARIB STD-B60 は MMT media transport | MMT signalling、package/asset、descriptor | ARIB STD-B62 第一編第三部は ARIB-TTML 系を含む第二世代字幕/文字スーパー coding | TLV -> IP/UDP -> MMTP -> signalling -> caption asset/MPU -> descriptor が識別する字幕形式 |

重要な修正：BS4K/8K だからといって payload が必ず ARIB-TTML とは限りません。STD-B60 の後続資料は caption data format が caption-description method で識別されることを明確にしています。実際の signalling/descriptor を読み、ARIB-TTML、B24-compatible/other indicated format、unknown format を別 route にし、unknown は raw data を保存して報告します。192-byte の `*.m2ts` packetisation は recorder-file の表現であり、TS/TLV/MMT の content probe の代わりにはなりません。

STD-B24 は従来 digital broadcasting の data coding/transmission specification です。STD-B10 は MPEG-2 PSI を service information で補完するもので、glyph/layout 規格ではありません。STD-B62 は高度広帯域衛星放送に適用され、第一編第三部が字幕/文字スーパー coding を扱います。STD-B60 は MMT transport を扱います。physical/transport、service signalling、caption coding を別 layer として実装・試験します。

規格の入口（number、scope、link のみを記録し、著作権で保護された規格本文を転載しない）：[STD-B31](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b31.html)、[STD-B20](https://www.arib.or.jp/english/std_tr/broadcasting/std-b20.html)、[STD-B10](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b10.html)、[STD-B24](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b24.html)、[STD-B44](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b44.html)、[STD-B60](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b60.html)、[STD-B62](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b62.html)。

## 字幕の真実モデルと時間

ASS や一つの開始/終了/text cue は内部の真実ではありません。`TimedCaptionOperation`（clear、cursor、style、text、DRCS、ruby、definition 等）の忠実な時間軸を `CaptionPlaneState` に適用し、終了した `RegionInterval`/scene を生成します。独立 region は重なり、別々に更新・消滅できるため、一つの cue に寿命を統合しません。

raw/unwrapped/normalized PTS/DTS/PCR、source offset、management data、language/TCS、clear/repeat/roll-up、plane geometry、region、文字 style、writing direction、ruby、enclosure、DRCS、未対応 control、必要時の raw payload、warning を保持します。trim、PCR jump、不連続、wrap-around、packet loss、clear 欠落、多重 service、PTS reset、終了時刻欠落を扱います。strict/repair/zero-point/manual offset/end inference を明示し、次字幕の開始を無条件に前字幕の終了にしません。

region は overwrite、clear、終了時のみ閉じて export し、EOF で残りを閉じます。これにより有界メモリで交錯する ASS/TTML を増分出力できます。header が後続 style/DRCS に依存する場合は一時 body を許容しますが、放送入力の再読込や全時間軸の保持はしません。

## 形式と DRCS

保真 target は ASS、TTML、ARIB-TTML、プロジェクト archive です。ASS は互換性の高い視覚近似であり lossless ではありません。縦書き、ruby、flash、装飾、複雑な DRCS の制限を明示します。TTML は internal full、IMSC compatibility、ARIB-TTML compatibility を区別し、validation のために構造を黙って削除しません。

SRT、通常 WebVTT、TXT、CSV は lossy/text extraction のみで、正式な字幕変換または既定出力ではありません。GUI は region 統合、time split、style loss を表示します。

archive は operation/scene JSON、元の data group/PES/MMT caption asset、DRCS PNG/SVG、PID/asset ID/PTS、diagnostics を保存し、長期的に最も可逆性を目指す交換形式です。

DRCS は、証明可能な Unicode mapping、記録済みで user 承認された慣用代替、視覚 glyph export/reference、ASS の一時 font/vector/bitmap の順で扱います。破棄、推測、`[外:<hash>]` の出力は禁止です。glyph、代替、出現回数、初出時刻、user choice を持つ local dictionary/inspector を設けます。

## libaribcaption、preview、IPC、parser 安全性

初期段階で B24 を再実装しません。libaribcaption は decoding/control/DRCS/region-style と任意 renderer を担いますが、demux、プロジェクト model、全 timing、export、checkpoint、archive は担いません。Rust は広い C++ bindgen でなく project 所有の小さな C ABI を利用し、lifetime、pointer、UTF-8、exception isolation、allocator、portable build、ABI drift を監査します。

HTML/CSS は構造 preview 用です。保真 preview は native renderer の RGBA/PNG/WebP snapshot を時刻または字幕状態変化で要求し、video-rate で WebView に送信しません。IPC は有界・低頻度とし、初期の line-delimited JSON は progress/warning/track に十分です。

表示 plane 補正（2026-07-25）：root の `<tt>` が有効な pixel display extent を宣言した場合、B62/ARIB-TTML の viewer geometry を native renderer の論理 `1920×1080` plane に正規化します。これが無い場合は logical 2K convention を既定とし、完全な pixel `origin`/`extent` geometry が少なくとも一方の軸で 2K を超え、標準 3840×2160 または 7680×4320 plane に収まる場合だけ source plane を推定します。region geometry は横・縦ごとに scale し、pixel の font size、line height、letter spacing、直接 outline width は有界な共通 scale を用います。従って等価な 2K/4K/8K source layout は同じ視聴者相対の字幕面積を保ちます。曖昧または不正な input を 4K とみなすことはありません。raw PES/MMTP evidence は変更しません。

縦書き句読点の増分（2026-07-25）：native B62 preview は Unicode が明示する vertical presentation punctuation だけを対応させ、bundled ARIB font にその glyph がある場合だけ用います。無い場合は source character を維持します。archive から `render_at` までの決定的 PNG golden がこの経路を覆います。Latin rotation、縦中横、完全な orientation/punctuation rule、standard B62 stroke を実装済みとは主張しません。

native preview 同期の増分（2026-07-25）：`sync_preview_overlay` は mpv 再生時刻の取得、archive lookup、native RGBA composition、overlay の apply/clear、同じ caption plane の重複排除を Tauri backend に閉じ込めます。Svelte は low-frequency typed API を呼び結果を表示するだけで、media time の推定も caption layout も行いません。mpv 時刻が未準備なら backend は local clock を推測せず `awaiting-player-time` を返します。

libmpv runtime の増分（2026-07-29 更新）：Windows は bundled `libmpv` をプロセス内で読み込み、`mpv.exe` sidecar と JSON named pipe は使いません。完全な render API が利用できる場合は `libmpv-render` を既定とし、個別 source の WGL/render 初期化に失敗した場合だけ `libmpv-client-overlay` に fallback します。専用 WGL thread が OpenGL context、libmpv render loop、resize message、backend BGRA caption texture blend を所有し、capability/diagnostic API が利用可能 route と実際の選択を報告します。WebView は video frame も caption texture も受け取りません。macOS/Linux native preview backend は未実装です。

visual reference 補正（2026-07-25）：同梱の libaribcaption `screenshot0.png` を project の viewer-facing なテレビ字幕 reference とします。B24 は設定済みの ARIB font、ruby、background、stroke で libaribcaption が生成する RGBA を継続して用います。B62 は同じ viewer-facing relationship を目標にしますが、対応する B62 source payload と合法な reference capture が無い限り pixel-level 一致を主張しません。`docs/visual-reference.md` を参照してください。

横書き layout の増分（2026-07-25）：native B62 path は明示的な改行を保持し、有界な TTML region 内で `textAlign`、`displayAlign`、`lineHeight` を適用します。`start`/`end` は writing direction に従います。archive から `render_at` までの PNG golden は複数行・中央・bottom alignment を覆います。

reference implementation の監査（2026-07-25）：`makeding/aribb62.js` は reviewed commit `74304d40a5b8556be1148e123ae70d60f937ecf5` の package metadata で MIT を宣言していますが、repository と GitHub license endpoint に standalone LICENSE file はありません。semantic は Rust renderer に独立して port できますが、redistributable な license text と copyright notice を得るまで source は vendoring しません。最初の port は browser CSS を使わない native TTML named colour（`transparent` を含む）です。

主画面は file drop、service/caption track の選択、output format/mode、task control、preview を優先します。modern design とは既定操作を単純にしつつ内部状態をいつでも検査できることです。inspector は少なくとも container type、service ID、PID/asset ID、language、PTS range、DRCS count、CRC error、packet loss、不連続、未対応 command を示します。

TS 188/192/204 と PAT/PMT は小さな有界 parser を手書きしてよく、TLV/MMTP は winnow、nom、または有界 cursor を選べます。全 parser は panic、out-of-bounds、無限 loop、不可信 length による無制限 allocation を避け、offset を報告し、破損後に sync を回復できなければなりません。

## 検証、段階、現状

worker の entry point は意図的に小さく保ちます。設定定数は `config.rs`、
188-byte MPEG-TS は `transport/mpeg_ts.rs`、192-byte M2TS route は
`transport/m2ts.rs`、字幕文書の意味論は `caption/ttml.rs` が担当します。
Tauri の preview layer は backend catalog を返し、Windows の既定
`libmpv-render` と個別 source 用 `libmpv-client-overlay` fallback を区別します。

archive preview には B24 の RGBA evidence を合成する bounded native
caption-plane compositor を追加しました。plane size と active layer 数を
含む一枚の PNG を返します。text-only の TTML/B62 は resource-complete
font renderer が接続されるまで構造 preview のままです。

制限付き TLV route では、同一 MPU の resource を lossless な
`resource_evidence` archive record として保存します。desktop reader は
最大 64 record を保持し、構造的に検証済みの小さな PNG だけを一致する
active caption に低頻度 preview として公開します。font と非 PNG resource
は raw evidence のままです。

地上波、BS2K、caption/superimpose、縦書き、ruby、DRCS、色、位置変化、二言語、破損 TS、BS4K/8K を含む golden corpus を作ります。合法な原本/生成 sample、信頼できる screenshot、期待 event JSON、保真/lossy output、既知問題を保存します。旧ツール、FFmpeg/libaribcaption、本ツール、必要なら放送 screenshot を text、timing、clear、position、colour、DRCS、management change で差分比較します。TS sync、PSI、PES、B24、TLV、MMTP、signalling、MPU assembly を fuzz し、Windows/macOS/Linux CI で検証します。

実装順： (1) Rust core、CLI/API、caption model、B24 C ABI、従来 corpus、(2) ASS/TTML/archive と DRCS visual asset、(3) 限定 BS4K/8K route、(4) Tauri 2/Svelte 5 の task/track/log/inspector/複数タスク UI と native mpv preview。現在は Phase 3 を進行中です。B62 semantics、bounded resource evidence、worker の責務分割、archive 時点 preview、B24 native RGBA composition、横書き/縦書き ruby、保守的な B62 glyph orientation/punctuation、Windows `libmpv-render`、実録画を使う閾値付き 4K 長時間 performance gate、overlay composition test、fuzz target、cross-platform build matrix は実装済みです。resource-complete preview、standard B62 stroke の reference 検証、独立した 2K/8K・DPI・画像差分 gate、macOS/Linux native preview backend は未完了です。

Go/Wails prototype は調査証拠に過ぎません。18.6 GB の地上波 fixture では 13,653 caption PES と 2,230 libaribcaption caption object を得ましたが、最終 architecture でも ASS/DRCS/BS4K の完成品でもありません。local fixture は `ARIB_FIXTURE_DIR` で選択し、再現可能な opt-in regression check は `docs/corpus.md` を参照します。

Rust workspace には `crates/arib-caption-worker` を作成しました。有界な `inspect` command は 188-byte MPEG-TS、192-byte M2TS、raw TLV、unknown input を識別します。従来 B24 は project 所有の狭い C ABI bridge 経由で libaribcaption を呼びます。bridge は native object を解放する前に、plane、region、Unicode/PUA character、position、colour、style、DRCS code、alternative、raw pixel を Rust scene snapshot として copy します。unknown DRCS は対応する `.drcs` directory に raw pixel/metadata asset として保存し、ASS `\p1` vector drawing event としても描画します。`[外:<hash>]` は出力しません。地上波の完全変換では 13,653 PES、2,230 caption object、2,736 region、29,892 character、61 DRCS glyph、decoder error 0 を得ました。M2TS route は private data PID を発見し、有界 PES buffer で payload を再構成して UTF-8 ARIB-TTML document を取り出し、`div` から継承した timing と `region` position を ASS に書き出します。付属の 11.5 GB BS4K regression fixture は現在 422 TTML caption event、5,051 character、parser warning 0 で完了します。制限付き TLV route も、complete `stpp` payload が self-contained UTF-8 TTML で対応する MPT NTP metadata を持つ場合だけ変換し、他の asset は raw evidence route を維持します。Tauri/Svelte は presentation-only とし、typed request と low-frequency event を転送し、parse、export、diagnostic、preview data の準備は worker が担当します。B62 style、ruby、writing mode、resource scope、bounded PNG/font evidence は model に保持されています。backend は対応する TTML text、横書きと折り返し縦書き ruby、保守的な orientation/punctuation、direct opacity、限定された direct `tts:textOutline` を native rasterise します。Windows `libmpv-render` と native overlay composition は接続済みです。resource-complete preview、完全な B62 orientation/punctuation/stroke semantics、generic TLV/MMTP extraction、macOS/Linux native preview backend は Phase 3 の後続作業です。

現在の model 実装では、各 B24 scene を `RegionInterval` に分割します。有界の active-region map は、その region 自身が変化または消失したときだけ close するため、話者 label と本文は独立し重なった lifetime を保持します。close 済み interval は faithful ASS、optional TTML、JSONL archive record に共通して出力します。TTML は region timing、origin、extent、font size、colour と namespace 付きの未解決 DRCS reference を保持し、ASS は vector DRCS glyph による visual fallback です。完了済み task の timeline と diagnostic window は JSONL を streaming scan し、要求された page だけを保持します。live event view は backend の有界な最新 window のみを保持し、editor timeline は有界 prefetch range と append byte cursor を使うため、archive 全体の反復読込や WebView への全履歴転送を行いません。desktop と複数タスク処理は streaming parser の境界で協調的に pause、resume、cancel できます。中断後の `.checkpoint.json` は source size、mtime、先頭/末尾 64 KiB fingerprint、track、progress 上限を保存し、置換または切断された録画を拒否します。native B24 と partial artifact state はまだ serialize できないため、次回起動時は byte-exact resume を偽って主張せず、録画の trusted origin から完全再走査します。

raw TLV input は繰り返される 4-byte の `0x7F/type/length` header と有界 payload length で content-probe されます。現在は有界の diagnostic/raw-evidence MMTP route として direct IPv6/UDP、HCfB `0x60`/`0x61` context、MMTP packet ID/payload type、連続 signalling fragment の再構成（最大 16 stream、各 1 MiB）、MPT signalling table の asset type と descriptor tag（観測された `stpp` を含む）を報告します。MPT MPU timestamp descriptor は packet ID + MPU sequence を key に、正確な 64-bit NTP 原始値として保持しますが、normalised caption PTS としては扱いません。既知の `stpp` packet ID では MPU/MFU envelope を検証し、MFU を有界に再構成します（最大 8 MPU sequence、各 4 MiB）。最初の semantic route は、発見済み `stpp`、complete で self-contained な UTF-8 XML TTML payload、対応する MPT NTP metadata の三条件を満たす場合だけ受け入れます。最初の有効 MPU を零点として NTP 差分を既存 TTML caption model に渡します。sequence gap、invalid aggregation、cap breach、timestamp 欠落、その他の payload format は raw evidence のままとし、caption に推測変換しません。これは generic MMTP caption support の主張ではありません。desktop DRCS dictionary は user mapping を platform configuration directory に保存し、明示的な mapping mode のときだけ text に置換します。既定では未解決 glyph asset を保存し続けます。
archive を要求した場合、同じ有界 scan は発見した MPT asset の `asset_evidence` record（packet ID、type、descriptor tag、正確な NTP 原値）も出力します。`resource_reference` は元の `packet_id + mpu_sequence_number` scope を保持します。`subsampleNumber=0` は TTML payload、`1..lastSubsampleNumber` は同じ MPU の bounded resource evidence です。数値の `subt://` index を全体 packet ID として扱わず、証拠がない場合や不完全な場合は unresolved のままです。

`dump-tlv` はこの layer の最初の raw extraction route です。単一の sequential pass で動作し、発見済み `stpp` asset の complete closed-caption payload が得られた場合だけ JSONL record を書きます。各 record は TLV source offset、MMTP packet/sequence、MPU sequence、timed-MFU flag、lossless hex data を保持します。対応する MPT MPU timestamp descriptor が存在する場合は、正確な原始 `presentation_ntp` も保持します。shared timeline policy を実装するまで `pts_ms` は明示的に `null` のままとし、raw evidence に timeline を捏造しません。
同じ route は、完全かつ有界に再構成できた非 `stpp` MPU/MFU payload も `mmt_asset_payload` record として asset type、source offset、決定的な MPU scope key、lossless byte とともに出力します。resource record には bounded header 検証、PNG サイズ、小さな完全 PNG の制限付き preview data URI を含められますが、汎用的に decode 済みまたは renderable だと判定するものではありません。

実装訂正（2026-07-23）：M2TS の EOF PES flush regression は修正済みです。付属 BS4K fixture は現在 422 TTML caption event、5,051 character、parser warning 0 で完了し、raw export 有効時には 330 PES record を捕捉します。desktop client は Tauri 2 + Svelte 5 で、初期 eframe/Slint prototype ではありません。Home の task list は platform configuration directory に最新 20 件のローカル task summary を atomic に保存し、broadcast payload は保存しません。

local corpus の訂正（2026-07-23）：18.58 GB の地上波と 11.52 GB の BS4K fixture は `ARIB_FIXTURE_DIR` で選択する opt-in test になり、streamed byte/count の厳密な baseline を検証します。M2TS private PES envelope 全体を UTF-8 と仮定せず、有界 extractor は完全な `<tt>…</tt>` byte slice を見つけて、その XML slice だけを UTF-8 検証します。これにより BS4K fixture の 422 captions/5,051 characters を回復し、raw PES evidence は変更しません。

DRCS report の実装（2026-07-23）：任意の `--drcs-report` は従来 B24 conversion で glyph が見つかった場合だけ `<name>.drcs.json` を出力します。code、dimension、colour に依存しない glyph metadata、alternative、保存済み `.drcs` asset への path を index 化しますが、raw pixel byte は report に複製しません。native UI も同じ option を提供し、project archive は別の完全 caption timeline のままです。

TTML 継承の修正（2026-07-23）：限定 M2TS/TLV TTML parser は caption ごとに、単に直近の文字列上の `<div>` を取るのではなく、その時点で開いているすべての `div` を走査します。入れ子の `begin`/`end`/`dur` は正しい親 time base から累積され、継承された `style` と `region` は document order で適用されます。閉じた sibling が後続 caption の timing、writing mode、colour、placement に漏れることはありません。これは shared TTML/archive model と faithful TTML output を改善します。ASS の writing mode と ruby は引き続き近似表現です。

TTML style delivery（2026-07-23）：共有 caption style は、継承した foreground/background colour、family、size、weight、style、writing mode、text/display alignment、outline、line height、letter spacing、opacity を archive と TTML interchange の両方で保持します。ASS は font、bold/italic、spacing、foreground colour の定義済み対応だけを適用し、対応しない TTML layout や background semantics を再現したとは主張しません。

ARIB-TTML span-style の修正（2026-07-23）：放送 payload では有効な style が `p` ではなく `span style="…"` に置かれることが多いため、parser はその reference を解決します。二軸 font size、`arib-tt:letter-spacing`、TTML 八桁 RGBA colour も対象です。interchange output は safe な span reference を自己完結した inline TTML attribute に展開し、source 専用 style identifier への reference を出力しません。実 BS4K sample では archive/TTML の `丸ゴシック`、`144px 144px`、foreground/background colour、16px spacing と、定義済み ASS approximation を確認しています。

文字 encoding 修正（2026-07-23）：ARIB STD-B24 の character-coded caption は UTF-8 と仮定せず、引き続き libaribcaption で decode します。ARIB-TTML route は PES/MMTP envelope から XML を分離した後、BOM/XML declaration を尊重して UTF-8、UTF-16LE/BE、Shift_JIS、EUC-JP、ISO-2022-JP を strict decode します。malformed/unsupported XML は raw evidence に残して report し、replacement character で修復せず、invalid framing byte により後続の valid document を失わせません。

## 変更管理

三言語文書は同一変更で更新します。影響する route/model invariant、fixture と validation、ASS/archive/DRCS mapping の compatibility を記載してください。提案、prototype、一つの fixture だけで support を主張してはなりません。

証拠の優先順位：現在の release gate は 188-byte MPEG-TS/B24 と
192-byte MPEG-TS packetisation/private PES/ARIB-TTML です。どちらも実録画の
長時間 fixture と streaming count baseline で検証しています。native BS4K/8K
は規範上 `TLV -> IP/UDP -> MMTP -> MPT/MPU` ですが、現在は構成/unit 証拠と
狭い `stpp` route のみです。合法な実 TLV capture が得られるまで
`tlv_mmtp_experimental` は evidence-first とし、汎用対応とは表示しません。

MPEG-TS では B24 caption PID を検証済みの第一候補とします。PSI/PMT が
private data PID だけを示す場合、worker は有界 PES assembly から complete
ARIB-TTML XML を探索できます。document boundary と declared encoding を strict
検証した後だけ TTML model に入れ、private PID 単独を caption の証拠にはしません。
188-byte private-PES route は構成した end-to-end regression で検証済みであり、
合法な実録画 fixture は今後必要です。

MPEG-TS dynamic PMT 修正（2026-08-02）：B24 logical track は
`service_id + component_tag` で識別し、録画先頭で見つかった PID をファイル全体の
固定属性とは扱いません。`inspect` は先頭と全体に分散した固定数の 1 MiB window で
PAT/PMT を有界 sampling し、sequential decode は current PAT/PMT を継続追跡して
PID 遷移前に旧 PES を flush します。`component_tag 0x30..=0x37` は字幕、
`0x38..=0x3f` は文字スーパーであり、後者を通常字幕または TTML candidate route に
入れません。後続 PMT で PID `0x1201` が追加される 21,609,477,452-byte の実録画は、
18,722 PES、3,825 scene、70,853 character、decoder error 0 で完了しました。
ASS/archive/DRCS semantics は変更せず、raw evidence は各 PES の実 PID を記録します。

route code も同じ evidence 境界に従います。`mpeg_ts_b24_verified` は
descriptor で検証済み、`mpeg_ts_ttml_candidate` は 188-byte TS または
192-byte M2TS の private PES PID を示すだけで caption の証拠ではありません。
`mpeg_ts_192_ttml_verified` は strict validation が成功した 192-byte
M2TS/TTML conversion 専用、`tlv_mmtp_experimental` は evidence-first、
`unknown_unsupported` は対応 caption route がない状態です。いずれも拡張子から
推論しません。

縦書き ruby の折返し増分（2026-07-25）：明示的に関連付けられた ruby の base
text が自動的に column をまたぐ場合、backend は記録済みの character-cell の
reading path に沿って ruby glyph を配分し、writing mode ごとの側に 0.5 倍で
rasterise します。この bounded continuation は archive から `render_at` までの
PNG golden で覆いますが、一般的な B62 ruby grouping、source 固有 placement、
縦中横、完全な glyph orientation を実装済みとは主張しません。
# Current implementation note

現在のデスクトップ実装は Tauri 2 + Svelte 5 です。worker は `cli.rs`、`inspection.rs`、`jobs.rs`、`preview.rs`、`archive.rs`、`protocol.rs`、`resource.rs`、`transport/`、`caption/`、`timeline.rs`、`drcs.rs`、`exporters/` に責務分割し、`main.rs` は process entry point と test のみです。`render-at` command は archive の指定時刻 snapshot を CLI から返します。以下の Slint 記述は履歴であり、現在の構成を表しません。初期 cargo-fuzz target は content probe、strict TTML envelope、MMTP envelope を対象とし、CI matrix は Windows/macOS/Linux で core/desktop を build します。resource-to-preview の完全な composition、汎用 TLV/MMT 字幕変換、PSI/PES/B24/signalling/MPU の追加 fuzz coverage は将来の作業です。macOS/Linux preview backend は延期され、現在の Alpha acceptance scope には含まれません。

デスクトップ永続化の修正（2026-07-26）：settings、job record、task history、artifact manifest、checkpoint、DRCS mapping は同一 directory の atomic publisher を共用します。完全な `.part` を同期し、replacement が install されるまで既存 metadata を保持し、失敗時は復元します。これは Windows の置換 semantics の修正であり、caption payload、archive semantics、transport route は変更しません。

## B62 収束増分（2026-07-26）

native TTML/B62 preview は連続する `tts:ruby="base"` span を一つの base group として扱い、一つの `tts:ruby="text"` annotation を group 全体に配置します。`arib-tt:ruby` は引き続き `xml:id` で関連付けます。annotation 自身の colour、font size、letter spacing、opacity、限定された direct `tts:textOutline` は backend で保持し、明示されない場合は base font の 0.5 倍を既定とします。この bounded model は横書き、縦書き、自動改段する縦書き ruby に共通です。

縦書きでは利用可能な Unicode vertical-presentation glyph を優先し、CJK/全角 glyph は正立、ASCII/Latin glyph は backend の時計回り bitmap rotation を使います。明示的な 1–2 桁の `textCombine` は一つの縦書き cell 内で横書きのままです。worker は等価な 2K/4K/8K authored geometry を一つの論理 `1920×1080` plane に正規化し、視聴者に対する caption 面積を保ちます。

これは unit/visual-golden で検証した backend behaviour であり、放送局固有の B62 rule を実 capture で検証済みという主張ではありません。非連続 ruby、追加の Unicode orientation class、standard stroke semantics は、合法な source payload と reference capture による corpus 比較で受け入れます。

## Windows native preview 収束増分（2026-07-26）

Windows で完全な `libmpv` render API が検出された場合、既定で `libmpv-render` を選択します。backend が WGL context、libmpv render loop、resize、video viewport、backend BGRA caption texture と blend を所有します。個別 source で render worker の初期化に失敗した場合だけ、その preview は `libmpv-client-overlay` へ fallback し、backend diagnostics は route、fallback reason、surface dimensions と presentation cadence を報告します。実際の 3840×2160 HEVC `bs4k_test_2.ts` smoke は startup、video-frame present、1920×1080 の texture blend/readback、3840×2160 の resize/present を検証します。WebView は video frame も caption texture も受け取りません。

この WGL route は libmpv の `hwdec=auto-safe` を要求します。これは互換性のある copy-back acceleration を使用する場合がありますが、zero-copy の ANGLE/D3D interop を意味しません。`scripts/validate-preview.ps1 -Long` は startup、cadence、完全な字幕 plane upload、control、working set、shutdown に明示的な閾値を設けた 120 秒の実 4K gate を実行します。2026-07-30 の `bs4k_test_2.ts` では `d3d11va-copy` で 34.74 present/s、peak 1526.9 MiB、4K warm-up 後の増加 111.9 MiB を記録しました。独立した 2K/8K profiling、DPI review、reference screenshot differencing は未完了です。macOS/Linux は引き続き `preview.platform_not_implemented` を返します。

## ASS 忠実度の修正（2026-07-29）

B24 ASS exporter は、復号された source plane を ASS の 1920×1080 play resolution に正規化し、各表示文字の位置、サイズ、横方向比率、stroke、DRCS geometry を同じ比率で変換します。文字ごとの色、bold、italic、underline は保持し、ruby は変換後の broadcast character-cell 座標で layer 1 に出力します。ARIB-TTML route は安全な inline span style を保持し、明示的に関連付けられた ruby を別 layer に配置します。annotation に font size の指定がない場合は base の 0.5 倍を使います。TTML の文字レイアウト semantics と監査した reference implementation に合わせ、B62 の二軸 font size は第二成分だけを ASS の字高に使用し、letter spacing は ASS 標準の spacing command で一度だけ適用します。font の横方向 stretch や独自の一文字単位 grid renderer は使用しません。独立した ruby region は source geometry により base region と関連付け、ASS の `an8+pos` で被注音範囲の実描画 glyph 中央へ配置します。base 本文は分割も移動もせず、一つの Dialogue event のまま libass に渡します。補正するのは ruby の anchor だけで、同梱 font の libass-compatible advance と ink bounds を用います。単字と複数文字は同じ範囲中点規則を使い、上置・下置を認識します。複数行では水平範囲を求める前に ruby に最も近い source 行を選びます。FFmpeg/libass pixel test は単字上置と下段複数字の下置を描画し、rendered centre の差が 3px を超えると失敗します。また ruby の有無で base raster が変わらないことを比較します。同時刻 caption の group は timing が変わるまでしか保持せず、streaming memory の境界を維持します。

ASS の既定 font は同梱の `Rounded M+ 1m for ARIB` とし、broadcast の `丸ゴシック` も同じ実測 font に対応付けます。他の明示的な source family は変更しません。18.58 GB の地上波 fixture と 11.52 GB の M2TS fixture は decoder error 0 で完了し、FFmpeg/libass による `いかり`/`碇` と `捧` の中央に配置された `ささ` の frame で位置、前景色、font size、黒 stroke を確認しました。任意の TTML 半透明背景 rectangle は ASS の互換目標に含めず、TTML/archive に保持します。

## Ruby binding と export 専用 box layout（2026-07-30）

Ruby の対応付けは ASS exporter の一時 heuristic ではなく caption model の処理になりました。B24 の `RubyBinding` は exporter に到達する前に、base region/index range、base text と cell box、source ruby box、placement、writing mode、provenance を保持します。ARIB-TTML も base caption/run/grapheme range を保持し、独立 B62 ruby region は同じ timing の有界 group が揃った時点で archive、TTML、ASS の書き出し前に対応付けます。実 M2TS corpus では `ささ` → `捧` を含む 31 件の構造化 binding を得ました。曖昧または未対応の region は推測せず unbound のまま残します。

ASS だけが export 専用 box layout を使います。交換可能な glyph-metrics interface の現在の実装は同梱 Rounded M+ font を測定し、base の rendered ink range を総幅が一致する slot に分配します。glyph ink が重なる場合は整数 pixel 単位で font size を fallback し、最後に visible ruby ink 全体へ一回だけ有界な整数 pixel の中心補正を行います。base 本文は一つの shaped Dialogue event のままで、個別配置できるのは ruby glyph だけです。明示的な `rubyPosition` の上置・下置を保持し、縦書きは実 corpus 検証まで同じ algorithm の axis transpose として扱います。libmpv 内部の libass は glyph metrics API を公開しないため、FFmpeg/libass pixel test を runtime compatibility gate とします。この処理は native preview chain（`libaribcaption -> native RGBA -> libmpv surface`）へ入り込まず、変更もしません。

## Sequential ARIB-TTML document と private PES track（2026-08-02）

Namespace に準拠する TTML は read-only XML tree の local-name と ancestor 関係で解析し、literal な `<p>` 表記に依存しません。一部の 192-byte 録画では ARIB-TTML document に `begin`、`end`、`dur` がなく、同じ PID の次の complete document が前の document を閉じ、空の `<tt>` が clear を表します。Private PES が PTS flag を立てても MPEG marker/prefix 規則を満たさない zero-filled 値は拒否し、192-byte route は 30-bit wrap を処理する M2TS arrival timestamp を使います。PID ごとの document state は分離します。

PMT の `component_tag 0x30..0x37` と `0x38..0x3f` は caption と superimpose を分類しますが、それだけで B24/TTML とは確定しません。B24 は引き続き `data_component_id 0x0008`、TTML は complete XML と strict encoding validation を必要とします。既定の preview/export は caption track のみを選び、superimpose は明示的に選択できる独立 track として保持します。分類不能な stream は candidate のままとし、PID、filename、programme name から推測しません。

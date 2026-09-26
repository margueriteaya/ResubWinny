[简体中文](backend-contract.md) | [English](backend-contract.en.md) | [日本語](backend-contract.ja.md) | [繁體中文](backend-contract.zh-TW.md)

> **規範上の注意:** 簡体字中国語版のみが唯一の正式な情報源です。その他の言語版は同期された翻訳であり、表現に曖昧さや矛盾がある場合は簡体字中国語版を優先します。

# バックエンドコントラクト

> 実装注記（2026-09-02）：本文の logical 1920×1080 plane は有界の中間 texture であり、正しさを定義する目標解像度ではありません。
> Worker は optional `source_layout` に source plane、region、style、inline length を保持します。native renderer はそこから
> 中間 texture を導出し、texture 全体を libmpv の video-content viewport へ mapping します。正しさは黒帯を除いた
> video content に対する比率で評価します。

Tauri/Svelte UI は Rust バックエンドのクライアントです。TS/TLV データの解析、ARIB のデコード、高解像度ビデオのレンダリング、変換セマンティクスの決定は行いません。

永続化用の `.caption.jsonl` 形式は、[`contracts/archive.md`](contracts/archive.md) で定義しています。スキーマの明示的なバージョンと、ストリーミングリーダーの互換性規則も同文書に記載しています。

契約の詳細は、[`contracts/tauri-api.md`](contracts/tauri-api.md)、[`contracts/worker-protocol.md`](contracts/worker-protocol.md)、[`contracts/preview.md`](contracts/preview.md)、[`contracts/timeline.md`](contracts/timeline.md) に分けて説明しています。本書は引き続き、互換性を確認するための索引と詳細リファレンスとして使用します。

バックエンドは、範囲を限定した安定的なアプリケーション契約を公開します。現在の収束フェーズでは、単発のコマンドを増やすよりも、関連するクエリの統合を優先します。

|コマンド |責任 |
| --- | --- |
| `inspect_source` |範囲を限定した録画の検査と字幕トラックの検出 |
| `start_export` |ストリーミングワーカーを開始し、`task-event` の進行状況を出力します。オプションの検証済み `trackId` を受け入れます。 |
| `cancel_export` |現在のワーカープロセスを停止します。 |
| `pause_export` / `resume_export` |協調制御メッセージをワーカーに送信します。 |
| `create_job` / `list_jobs` / `get_job` / `remove_job` |メディアペイロードなしでタスクの概要を保持します。 |
| `start_job` / `pause_job` / `resume_job` / `cancel_job` |ワーカースーパーバイザを通じて永続化されたジョブを制御します。 |
| `get_job_diagnostics` |永続化されたジョブに対して収集された、制限された構造化された診断を返します。 |
| `get_job_diagnostics_window` | offset/limit を使用して、境界付きの診断ページを返します。 |
| `list_jobs_window` |最近のタスクの概要の境界付きページを返します。 |
| `get_job_artifacts` |タスクアーティファクトマニフェストと `.part` パスを返します。 |
| `get_job_checkpoint` |タスクの最新の制限された進行状況チェックポイントを返します。 |
| `pause_queue` / `resume_queue` / `queue_is_paused` |スーパーバイザキューを制御し、そのアクティブなワーカーを連携して一時停止/再開します。 |
| `load_drcs_report` |ワーカーが作成した DRCS レポートを読み取り、表示可能なグリフイメージを返します。 |
| `get_settings` / `update_settings` |app-data `settings.json` に保存された検証済みの UI 設定とエクスポートの既定値を読み取り、またはアトミックに更新します。 |
| `list_language_packs` |固定された app-data `language-packs/` ディレクトリから制限された JSON 言語ファイルを再スキャンします。ブラウザーが提供する任意のディレクトリは受け入れられません。 |
| `open_language_pack_directory` |必要に応じてその固定ディレクトリを作成し、プラットフォームファイルマネージャーで開きます。 |
| `start_preview` / `resize_preview` / `stop_preview` |現在のインプロセス libmpv ビデオサーフェスを制御します。 |
| `preview_command` |シーク/一時停止コマンドを libmpv に転送します。 |
| `get_preview_capabilities` |宣言されたビデオ/キャプション構成ルートと現在使用可能なルートのみをレポートします。 |
| `get_preview_runtime` |レンダーサーフェスの存在を主張せずに、検出された libmpv ランタイムとレンダー API シンボルの可用性を報告します。 |
| `get_preview_render_diagnostics` |アクティブなネイティブルートと制限されたレンダリングスレッドのカウンター/エラーを報告します。ワーカーが存在しない場合は、安定した非アクティブな結果が返されます。 |
| `render_at` | WebView 経由でビデオフレームを送信せずに、要求されたアーカイブ時間の制限付きキャプションプレーンスナップショットを返します。 |
| `sync_preview_overlay` |埋め込まれた libmpv 時間を読み取り、境界のあるネイティブプレーンをレンダリングし、WebView のタイミングやレイアウトを使用せずに Windows オーバーレイを適用、クリア、または重複排除します。 |
| `get_playback_time_mapping` / `update_playback_time_mapping` |ネイティブキャプションプレビューで使用される検証済みのメディア時間 → プロジェクト時間セグメントマッピングを取得または置換します。 |
| `get_timeline_window` / `get_timeline_window_filtered` |完了したタスクを参照するために、制限されたアーカイブページをストリーミングします。 |
| `get_timeline_recent_window_filtered` |追記された完全な JSONL レコードを順次読み取り、最新の境界付きライブイベントページのみを返します。 |
| `get_timeline_time_window` |エディターのタイムラインの制限されたプリフェッチ時間範囲を返し、追加されたレコードを増分的に読み取ります。 |

`render_at` は、アーカイブのエクスポートが完了するとタスクワークスペースに公開されます。UI は時間クエリを明示的かつ限定的に保ち、アーカイブに B24 レンダリングフレームが含まれている場合、実際の RGBA 派生 PNG を表示します。バックエンドは `planeWidth`、`planeHeight`、`composedPngBase64`、および `activeLayerCount` を返します。合成された画像は、CSS や WebView テキストレイアウトではなく、境界付きのネイティブキャプションプレーンコンポジターによって生成されます。境界付きレイアウトフィールドを持つ TTML 間隔は、バンドルされている ARIB フォントの Rounded M+ 1m を使用して、バックエンドでラスタライズされた 1920×1080 RGBA プレーンを返すことができます。有効に宣言された表示範囲は、ソースジオメトリとピクセル長をその論理プレーン上で正規化します。不在範囲のデフォルトは論理 2K であり、少なくとも 1 つの軸で論理 2K を超え、その平面に適合する完全なピクセル領域ジオメトリから正規の 4K/8K のみを推測します。同等の 2K/4K/8K レイアウトは、曖昧なソースを推測することなく、同じ視聴者相対サイズを維持します。境界付きリッチボディパーサーは、スパン/ルビタグの外側のテキストを保持し、明示的なスパンの色、サイズ、間隔、不透明度をマップします。ネイティブの水平パスは明示的な改行を保持し、解決された `textAlign`、`displayAlign`、および `lineHeight` を適用します。単純な水平方向の `tts:ruby` ベース/テキストペアが 0.5 スケールでラスタライズされ、ベーススパンの中央に配置されます。明示的に関連付けられた垂直ルビも同様に、自動列折り返しが発生する場合の有界継続を含め、基本セルの横に 0.5 スケールでラスタライズされます。どちらも `captionPlaneMode=ttml-vertical-ruby-basic-native` と `renderedRubyCount` を報告します。この継続では、一般的な B62 ルビのグループ化やソース固有の配置は実装されていません。垂直レンダラーは、バンドルされた ARIB フォントにマップされたグリフが含まれている場合にのみ、Unicode 垂直表示句読点を使用します。ラテン語の回転や縦中横を近似するものではありません。直接 `tts:textOutline` は、`none`、TTML 名前付き色、または完全な `#RRGGBB[AA]` と `px` 幅のみを受け入れ、その後、境界付きのネイティブアウトラインを適用します。`arib-tt:border` は意図的に変換されません。完全な B62 グリフの方向、標準の B62 ストローク動作、非 PNG リソース、およびレンダリング不可能または欠落しているグリフには、依然として明示的な制限があります。サポートされていないレコードは、捏造された画像ではなく構造プレビューのままになります。

TLV アーカイブには、サイズや件数に上限を設けた `asset_evidence` と `resource_evidence` レコードが含まれる場合があります。各 `resource_evidence` は、可逆な Base64 ペイロード、形式の検証結果、`subt://` 参照との照合に使う正確な `packet_id + mpu_sequence_number + subsample_number` キーを保持します。プレビューリーダーは最大 64 件を保持し、同じ MPU 内で一致したものだけを表示中の字幕に関連付けます。検証済みの小さな PNG の `preview_data_uri` は `resourcePreviews` として公開します。フォント、PNG 以外のリソース、欠落したリソース、不完全な対応関係は証拠として保持し、描画済みの字幕テキストとして扱いません。

`asset_evidence` は、入力で実際に確認した MPT シグナリングのみを記録します。記録するのは `packet_id`、ソース TLV オフセット、`asset_type`、記述子タグ、通知された MPU NTP 値です。これらは将来の `subt://` リソース対応付けに使う証拠であり、デコード済みの画像やフォントのバイト列ではありません。

`resource_reference` は、元の `packet_id + mpu_sequence_number` スコープを保持します。数値の `subt://` インデックスをグローバルな MPT パケット ID として扱うことはありません。同じ MPU に上限内のサブサンプルが存在する場合は、関連を `same-mpu-evidence` として元のリソースレコードに結び付けます。それ以外は明示的に `unresolved` のまま保持します。

`dump-tlv` は、完全でサイズ上限内の非 `stpp` MPU/MFU ペイロードも、決定的なスコープキーを持つ `mmt_asset_payload` として出力します。`format_hint` があっても、それは範囲を限定して調べたバイナリ署名やヘッダーの観測結果であり、デコードや描画が可能という意味ではありません。未知のアセットの意味は未解決のままです。PNG の寸法やフォントテーブル数も構造上のメタデータにすぎません。構造が完全な小さな PNG は、将来のネイティブプレビュー用にサイズ上限付きの `data:` 値を持つ場合がありますが、バックエンドが任意のリソース URL をデコードしたり信頼したりすることはありません。

スナップショットには `renderProfile` も含まれます。この契約は libaribcaption との互換性を意図しています。バンドルした `Rounded M+ 1m for ARIB` を使い、文字セルの形状と配置を保ち、ルビの相対サイズを 0.5 に維持します。背景のアルファ値と縁取り色は、デコード済みの元の文字データから取得します。公開済みの libaribcaption スクリーンショットを表示の視覚的な基準とし、固定したローカル基準とレビュー規則は `docs/visual-reference.md` に記載しています。

B24 の描画はデコーダーに基づきます。現在のネイティブ TTML 経路は、バンドルしたフォント、元の前景・背景 RGBA、span ごとのスタイル、単純な横書きルビ、明示的に関連付けた縦書きルビに対応しています。縦書きルビは、自動改段をまたぐ場合も上限を設けて継続します。複雑なルビのグループ化、縦書きの全字形方向、標準の縁取り動作は、ネイティブ実装のテストが完了するまでは宣言上のメタデータです。UI が任意の CSS シャドウや固定の黒い枠で代用してはいけません。

`captionOverlayModes` は、バックエンドの各経路の能力を `id`、`available`、`experimental`、`unavailableReasonCode` で表します。Windows では、検出したランタイムが完全な描画 API を公開していれば `libmpv-render` が利用可能になり、バックエンドはこれを既定で選択します。描画 Worker の起動に失敗した場合は、そのソースのプレビューを `libmpv-client-overlay` に切り替えます。UI はバックエンドが実際に使う経路を表示し、レンダラー自体は選択しません。

## ワーカーイベントエンベロープ

ワーカー JSONL イベントは、`protocolVersion`、`jobId`、`sequence`、および `payload` フィールドを使用します。互換性のため、従来のトップレベルのイベントフィールドは移行中もそのまま残ります。Tauri レイヤーは、イベントを Svelte に転送する前に、バージョンとシーケンスを検証する必要があります。

ワーカーは最初に `hello` を発行し、その後、必要に応じて、制限された `stage-changed`、`track-discovered`、進行状況、`diagnostic`、`drcs-discovered`、一時停止/再開、キャンセル、`artifact-created`、完了、または `failed` イベントを発行します。正常に公開されたすべてのアーティファクトは、完了前にその安定した種類と最終パスとともに報告されます。Tauri は、UI オプションから最終的なアーティファクトを推測するのではなく、そのイベントを使用してアトミック `app-data/jobs/{job-id}/artifacts.json` マニフェストを更新します。チェックポイントの永続性は Tauri に属します。`checkpoint.json` がアトミックに公開された後でのみ、`checkpoint-written` が転送されます。Tauri は、すべてのタスクイベントで安定した `code` および `parameters` シェイプを転送します。タイムラインと診断ページは JSONL ソースをストリーミングし、要求されたウィンドウのみを保持します。デスクトップは完全なアーカイブまたは診断履歴をメモリにキャッシュしません。ライブタイムウィンドウ API は、1 つの制限されたプリフェッチウィンドウを保持し、新しく完成した JSONL 行上でバイトカーソルを進め、要求された時間がそのウィンドウを離れるか、アーティファクトが置き換えられた場合にのみディスクから再構築します。プロトコルバージョンおよびシーケンス違反では、生のメッセージが証拠として保持されますが、`expected`、`actual`、`previous`、`current` などの名前付きパラメーターも保持されます。Svelte は、メッセージを解析せずにコードをローカライズします。ワーカーが提供する診断パラメーターは、JSON オブジェクトである場合、そのまま保存されます。キャンセルまたは失敗すると、ワーカーイベントとファイル証拠からアーティファクトステータスが調整されます。`completed` はワーカーが公開したことを意味し、`preserved` は既存のターゲットが変更されていないことを意味し、`incomplete` は `.part` ファイルが残ったことを意味します。`failed` または `cancelled` は、より強力なアーティファクトの証拠が存在しないことを意味します。アプリケーションの起動時に、永続的なアクティブ状態は `Interrupted` になり、永続的な `Queued` タスクは `Ready` になります。メモリ内キューが自動的に再開されることはありません。`resume_job` は、ジョブ ID、ソース、出力、トラック、ソースサイズ、進行状況の境界、および境界のある先頭/末尾のソースフィンガープリントを検証した後に、`Interrupted`、`Failed`、または `Cancelled` ジョブのみを再生します。タイムスタンプのみの変更は報告されますが、サイズとフィンガープリントがまだ一致している場合は受け入れられます。ネイティブデコーダと部分アーティファクト状態はシリアル化されていないため、リカバリでは現在、バイト正確な再開を要求するのではなく、信頼できる記録元からの完全な再生が実行されます。

ワーカーは独立して実行可能であり、UI 統合前にテストする必要があります。

```text
arib-caption-worker inspect recording.ts
arib-caption-worker convert recording.ts output.ass --overwrite --drcs-report
arib-caption-worker convert recording.m2ts output.ttml --ttml --overwrite
arib-caption-worker dump-tlv recording.tlv output.caption.mmtp.jsonl --overwrite
arib-caption-worker render-at output.caption.archive.jsonl 90000
```

既知の制限は製品の制約であり、隠れたフォールバックではありません。

- SRT は正式なロスレスターゲットではありません。
- 認識されていない TLV/MMTP アセットは生の証拠として保持され、推測されません。
- `inspect_source` は安定した `routeCode` を返します。`mpeg_ts_b24_verified` は B24 コンポーネント記述子によって直ちに検証されます。`mpeg_ts_ttml_candidate` は、188 バイト TS または 192 バイト M2TS にプライベート PES PID が見つかったことを示します。変換時には、さらに厳密な ARIB-TTML XML 検証が必要です。`mpeg_ts_192_ttml_verified` は、リリース時の検証対象となる、検証済みの 192 バイト M2TS/TTML 変換経路を示します。初回の範囲を限定した検査では、有効な TTML 文書を確認する前にこの経路を宣言してはいけません。`tlv_mmtp_experimental` は証拠の保持を優先します。実際の録画による検証なしに、汎用の BS4K/8K 対応として表示してはいけません。
- ネイティブ B24 および部分アーティファクト状態はシリアル化できないため、チェックポイントは現在、信頼できる記録オリジンからソース ID が検証された完全な再生を実行します。
- 現在の Windows ビデオサーフェスは、インプロセス `libmpv` によって所有されています。`mpv.exe` サイドカーまたは JSON 名前付きパイプは使用されません。ランタイムが完全なレンダリング API をエクスポートする場合、バックエンドは `libmpv-render` を選択し、WGL コンテキストと BGRA テクスチャブレンドパスを所有し、特定の起動が失敗した場合にのみクライアントオーバーレイにフォールバックします。`hwdec=auto-safe` を要求し、互換性のあるコピーバックアクセラレーションを許可しますが、ゼロコピー D3D/ANGLE の相互運用性は保証しません。`get_preview_render_diagnostics` は、選択されたルート、ライブサーフェスの寸法、1 秒あたりの提示数、テクスチャ操作数、アスペクト、要求されたデコーダーポリシー、およびロードされたソースがレポートする libmpv の実際の `hwdec-current` を返します。長時間の 2K/4K/8K プロファイリングは、暗黙の機能ではなく、リリース品質のゲートのままです。
- `get_preview_capabilities` は各経路を `{ id, available, experimental, unavailableReasonCode }` として報告します。これは表示用の契約であり、WebView から字幕ビットマップを送信することはできません。`render_preview_at` と `sync_preview_overlay` はバックエンド内部でサイズに上限のあるネイティブ字幕平面を合成し、libmpv に適用します。Windows 以外では `preview.platform_not_implemented` を報告します。これはネイティブプレビュー経路が実装済みという意味ではありません。
- `sync_preview_overlay` は、`mediaTimeMs` と `projectTimeMs` の両方をレポートします。`projectTimeMs` を使用してキャプションをクエリします。既定では両方の時刻が一致しますが、PTS 修復、プログラム境界、およびユーザーオフセットは、WebView に 2 番目のクロックを教えるのではなく、バックエンドマッピングを更新する必要があります。
- `trackId` は、検出した MPEG-TS B24 または M2TS データトラックの検証済み PID セレクターとして渡します。B24 では、選択した PID を論理トラック `service_id + component_tag` に対応付けます。順次デコードは PAT/PMT の更新に追従し、同じ論理トラックの PID が変更されても継続できます。検査結果は、代表の `caption_pid`、検査範囲内で見つかった `caption_pids`、コンポーネントタグ、PAT/PMT サービス ID、SDT サービス名、ISO-639 字幕言語を報告します。`broadcast` には、取得できた場合に NIT ネットワーク名、選択中のサービスの EIT 現在イベント名と説明、TDT/TOT の UTC 放送時刻も含めます。この SI 検査は内容に基づき、1 パケットの作業バッファで最大 64 MiB を読みます。選択したサービスに EIT がない場合に、別のサービスの番組で補うことはありません。欠落したフィールドは、検査範囲内に情報がなかったことを示します。広範な EPG 履歴、CAS、録画機のメタデータは製品契約の対象外です。キュー管理側が一時停止状態を保持し、実行中の Worker に協調的な一時停止・再開の指示を送ります。待機中に一時停止した場合も、次のジョブの開始を防ぎます。

プライベート PES トラックの検出では、`pids`、`caption_pids`、`superimpose_pids` を報告します。コンポーネントタグ `0x30..0x37` と `0x38..0x3f` は字幕と文字スーパーを分類しますが、それだけで B24 または TTML と確定することはできません。B24 にはデータコンポーネント記述子が必要です。TTML には、厳密なデコードを通過した完全な XML 文書が必要です。明示的な `trackId` がなければ、変換とプレビューは宣言済みの字幕コンポーネントを選び、文字スーパーは独立して扱います。PMT 記述子で分類されていないプライベートストリームは候補のまま保持し、PID から種類を推測しません。

TTML は XML のローカル名と祖先要素に基づいて解析し、既定の名前空間と接頭辞付きの名前空間の両方に対応します。連続する ARIB-TTML 文書では `begin`、`end`、`dur` が省略される場合があります。同じ PID 上の次の完全な文書が直前の文書を終了し、空の `<tt>` は字幕の消去を表します。192 バイト M2TS 経路では、PES PTS のマーカーまたは接頭辞の検証が失敗した場合、30 ビットの到着タイムスタンプから折り返しを考慮して文書時刻を求めます。`PTS_DTS_flags` が設定されているという理由だけで、ゼロ埋めされたプライベート PES フィールドを受け入れることはありません。別の PID に届いた文書で、ある PID の文書を終了することもありません。

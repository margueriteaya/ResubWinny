[简体中文](README.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [English](README.en.md)

> 簡体字中国語版が唯一の正式な情報源です。その他の言語版は同期された翻訳にすぎません。

# ResubWinny

> [!WARNING]
> 本プロジェクトは現在 alpha 初期段階にあり、利用可能性は保証されず、破壊的変更が行われる可能性があります！

ResubWinny は Windows 上で動作する、日本のコンテンツ全般の映像アーカイブ原本ファイルを対象とした字幕の抽出、検査、プレビュー、変換ツールです。モダンなユーザーインターフェースを備え、コマンドラインからも操作できます。

地上デジタルテレビ、BS/CS 2K、および一部の BS4K/8K 録画ファイルに含まれる ARIB 字幕を処理し、映像をリアルタイムでプレビューできます。外部プレーヤーでの再生向けには、字幕の位置、色、フォントサイズ、縁取り、Ruby（ルビ）、ARIB 外字、DRCS 字形、アクセシビリティ標識を可能な限り保持して出力できるほか、特殊タグを破棄し、後続作業やアーカイブに適した形式で出力することもできます。

現在の収束期間では日本の放送録画字幕に注力しています。BD/DVD グラフィック字幕 OCR、プラグインシステム、AI 翻訳、および macOS/Linux のネイティブプレビューは明確に延期されており、現在のロードマップまたは受け入れ範囲には含まれません。DRCS については、ローカルの hash → Unicode マッピングの改善のみを継続し、汎用 OCR システムへの拡張は行いません。

プロジェクトの現在のバージョンは `v0.2.2-α`（ソースコードバージョン `0.2.2-alpha.1`）です。現在も開発段階にあります。

## 特長

### 字幕の抽出と認識

- 拡張子 `.ts`、`.m2ts`、`.mmts` による判定に依存せず、ファイル内容に基づいて入力形式を検出します。
- 188-byte MPEG-TS 内の ARIB STD-B24 字幕をサポートします。
- 192-byte M2TS 形式の MPEG-TS、private PES、および厳密に検証された ARIB-TTML 字幕をサポートします。
- 複数サービスおよび複数字幕トラックの検出と選択をサポートします。
- 録画ファイル内で利用可能な放送ネットワーク、サービス、番組、放送時刻を解析し、再生位置に応じて該当する状態を照会できます。対応する SI/EIT/TOT の根拠がない場合、フィールドを捏造しません。
- UTF-8、UTF-16LE/BE、Shift_JIS、EUC-JP、ISO-2022-JP でエンコードされた TTML を厳密に処理します。
- 破損、切り詰め、または未サポートの入力に対して構造化された診断を提示します。

### 字幕のセマンティクスとレイアウト

- 字幕領域ごとの独立した表示時間と重複関係を保持します。
- 位置、フォントサイズ、色、縁取り、透明度、字間、書字方向を保持します。
- Ruby とルビが付与される対象との構造化された対応関係を認識して保持します。
- 横書き、基本的な縦書き、連続する Ruby のグループ化、縦書き文字の向きの処理をサポートします。
- ARIB 特殊記号、カスタム DRCS 字形、および一般的なアクセシビリティ標識を認識します。
- 2K、4K、8K ソースのジオメトリを論理字幕平面に正規化し、字幕が視聴者から見てほぼ同じ比率を保つようにします。

### エクスポートと検査

- ASS、TTML、SRT、WebVTT など複数の出力形式を一度に選択できます。
- 位置、色、Ruby、DRCS、ARIB 外字、アクセシビリティ標識を保持するかどうかを個別に指定できます。
- 字幕イベント一覧で特殊な字幕特性を絞り込み、マークできます。
- プロジェクトアーカイブ、元の PES/MMTP の根拠、および DRCS レポートを提供します。
- 途中での一時停止をサポートし、まず一時 `.part` ファイルに書き込み、成功後に完全なファイルを出力します。
- バックエンドが入力パスと出力パスを検証し、字幕出力によって元の録画が上書きされるのを防ぎます。

### デスクトップアプリケーション

- 多言語インターフェースで、簡体字中国語、繁体字中国語、日本語、英語を内蔵しています。
- ライト、ダーク、システム設定に従うテーマをサポートします。
- 複数タスクの作成、キューイング、一時停止、再開、キャンセル、履歴、診断をサポートします。
- 字幕イベント一覧、拡大縮小可能なタイムライン、クリック／ドラッグによるシーク、および DRCS 辞書を提供します。
- Windows では libmpv を使用して映像をネイティブレンダリングし、映像フレームを WebView に送りません。
- B24 字幕平面と ARIB-TTML 字幕画像は Rust バックエンドによって生成され、ネイティブプレビューにオーバーレイされます。

## 入力サポート状況

| 入力経路 | 状態 | 説明 |
| --- | --- | --- |
| 188-byte MPEG-TS + ARIB STD-B24 | 検証済み | 地上デジタルテレビおよび BS/CS 2K 録画ファイル向け |
| 192-byte M2TS 形式 MPEG-TS + private PES + ARIB-TTML | 検証済み | 既存の BS4K 録画サンプルによる回帰テスト済み。該当するかどうかは内容検出によって決定されます |
| MPEG-TS 内の private PES/TTML 候補 | 境界付き検出 | 完全な XML 境界、宣言されたエンコーディング、および TTML 文書のすべてが検証を通過した場合にのみ変換します |
| 生の TLV/IP/UDP/MMTP | 実験的 | 検出、診断、生の根拠を主目的とします。完全な `stpp`/TTML アセットは厳格な条件下でのみ処理します |
| 不明または未サポートの入力 | 明示的に拒否 | コンテナや字幕タイプを推測せず、安定したエラーと検出根拠を返します |

生の TLV/MMTP は、現在検証済みの汎用 BS4K/8K サポートには含まれません。ファイル拡張子はファイル選択ダイアログのヒントにのみ使用され、伝送形式の根拠にはなりません。

## 出力形式

| 形式 | 用途と制限 |
| --- | --- |
| ASS | 字幕制作および一般的なプレーヤー向け。ASS/libass で表現可能な位置、色、フォントサイズ、縁取り、Ruby を保持しますが、字幕背後の半透明背景矩形は復元できません |
| TTML | 独立した領域、時刻、スタイル、Ruby、書字方向、DRCS 参照、ソース情報を保持します |
| SRT | プレーンテキスト互換出力。放送上の位置、重複領域、Ruby レイアウト、DRCS グラフィックは表現できません |
| WebVTT | Web 互換のテキスト出力。SRT と同様に非可逆形式です |
| Caption archive (`.caption.jsonl`) | 統一字幕モデル、領域のライフサイクル、Ruby の対応関係、ソース情報を保存し、ページ分割されたタイムラインと再レンダリングに使用します |
| Raw evidence | ソースオフセット、シーケンス番号、時刻ソース、およびロスレスの PES/MMTP payload を保存します |
| DRCS report/assets | Unicode に直接マッピングできない字形、ピクセルリソース、マッピング候補、ユーザー選択を保存します |

## 技術スタック

| レイヤー | 技術 |
| --- | --- |
| デスクトップフロントエンド | Svelte 5、TypeScript、Vite、Lucide Svelte |
| デスクトップアプリケーション層 | Tauri 2、Rust 2024 Edition |
| 字幕 Worker | Rust、バージョン化された JSONL プロトコル、ストリーミング I/O |
| B24 デコードとレンダリング | libaribcaption、プロジェクト独自の限定的な C ABI、C++ ブリッジ |
| ネイティブ映像プレビュー | libmpv render API、Windows WGL/OpenGL コンポジット |
| 字幕モデル | `CaptionPlane -> RegionInterval -> exporters` |
| 永続化 | アプリケーションデータディレクトリ内のアトミックな JSON/JSONL ファイル |
| テストと品質 | Cargo test、Clippy、Rustfmt、Svelte Check、cargo-fuzz、GitHub Actions |

フロントエンドはバックエンドの状態を表示し、型付きリクエストを転送するだけであり、放送データを読み取らず、字幕時刻を決定せず、最終的な字幕レイアウトを計算せず、映像フレームも処理しません。メディアおよび字幕機能は、まず Worker または Tauri バックエンドで実装し、その後 GUI に接続します。GUI で実行できる中核操作には、対応する CLI、Worker、またはバックエンド API が存在するべきです。

## 技術アーキテクチャ

```text
Svelte 5 フロントエンド
    | typed Tauri API / 低頻度イベント
    v
Tauri 2 Rust アプリケーションサービス層
    | タスクスケジューリング、永続化、ネイティブプレビュー、Worker 管理
    v
arib-caption-worker
    | ストリーミング検出、解析、字幕モデル、タイムライン、エクスポート
    v
libaribcaption
    | プロジェクト独自の限定的な C ABI
    v
ARIB STD-B24 デコードとネイティブ字幕平面
```

Worker は固定サイズのストリーミングバッファと 64 ビットのファイルオフセットを使用します。ファイルサイズが増加しても、通常時のメモリはファイル長に比例して線形に増加するべきではありません。詳細な責務とインターフェースについては、[中国語アーキテクチャ文書](docs/architecture.zh-CN.md)および[バックエンドインターフェース契約](docs/backend-contract.md)を参照してください。

## 主な依存関係

| 依存関係 | 用途 | 統合方法 | ライセンス／状態 |
| --- | --- | --- | --- |
| [xqq/libaribcaption](https://github.com/xqq/libaribcaption) `v1.1.2` | ARIB STD-B24 デコードとネイティブレンダリング | 固定 commit の vendored ソースコード | MIT |
| [makeding/libaribtlv](https://github.com/makeding/libaribtlv) `0.6.1` | optional な実験的 TLV/MMTP → B62 TTML demux | `libaribtlv` feature 有効時に固定 commit の vendored ソースを静的 link。project-owned narrow C ABI | MIT |
| [Zlib](https://github.com/madler/zlib) `1.3.2` | libaribtlv の private compression dependency | 固定 commit の vendored ソースを静的 link。system Zlib は検出しない | Zlib License |
| [mpv](https://mpv.io/) / libmpv | Windows ネイティブ映像プレビュー | 動的リンク、交換可能な DLL。ビルド時にハッシュに基づいてダウンロード | LGPL-2.1-or-later |
| Rounded M+ 1m for ARIB `1.3` | ARIB 文字 fallback、字幕プレビュー、ASS フォントメトリクス | プロジェクトとともにフォントを配布 | M+ FONT LICENSE および WadaLab の許諾 |
| Tauri 2 | デスクトップウィンドウ、ネイティブ API、パッケージング | Cargo 依存関係 | 依存関係ライセンス一覧を参照 |
| Svelte 5 / Vite | フロントエンドインターフェースとビルド | npm lockfile で固定 | 依存関係ライセンス一覧を参照 |
| serde / serde_json | Worker プロトコル、モデル、永続化 | Cargo 依存関係 | 依存関係ライセンス一覧を参照 |
| encoding_rs | ARIB-TTML 文字エンコーディング | Cargo 依存関係 | 依存関係ライセンス一覧を参照 |
| roxmltree | namespace-aware な TTML/XML 構造解析 | Cargo 依存関係 | MIT / Apache-2.0 |
| fontdue / ttf-parser | バックエンド字幕および Ruby 字形メトリクス | Cargo 依存関係 | 依存関係ライセンス一覧を参照 |
| [makeding/aribb62.js](https://github.com/makeding/aribb62.js) | ARIB-TTML/B62 の動作研究における参考資料 | 参考のみ、ソースコードはバンドルしない | レビュー対象バージョンの package メタデータでは MIT と宣言。詳細は第三者記録を参照 |

完全なバージョン、固定された入手元、ハッシュ、ライセンスについては、[第三者に関する通知](THIRD_PARTY_NOTICES.md)、[依存関係バージョン記録](third_party/versions.json)、[依存関係ライセンス一覧](docs/dependency-licenses.md)、[依存関係更新方針](docs/dependency-updates.md)を参照してください。

## ビルド環境

現在の Alpha における完全なデスクトップ受け入れプラットフォームは Windows 11 x86-64 です。以下が必要です。

- `rust-toolchain.toml` で固定された Rust `1.97.1`（Rustfmt と Clippy を含む）。
- Node.js 22 LTS。
- npm 10 または 11。
- CMake。
- MSVC C/C++ ツールチェーンと Windows SDK を含む Visual Studio 2022 Build Tools。
- Microsoft Edge WebView2 Runtime。
- 固定バージョンの Windows libmpv 開発パッケージのインストールに使用する 7-Zip。

Worker、Tauri のコンパイルチェック、フロントエンドのビルドは Windows、macOS、Linux の CI 上で実行されます。現在もネイティブプレビューとインストーラーの製品受け入れプラットフォームは Windows です。

## ビルド方法

以下のコマンドはすべてリポジトリのルートディレクトリで実行します。

### 1. 1 コマンドでビルド

```powershell
./scripts/build.ps1
```

このコマンドは、ロックされたフロントエンド依存関係をインストールし、固定バージョンの Windows libmpv をダウンロードして検証し、Worker、フロントエンド、デスクトッププログラム、インストーラーをビルドします。再実行時には、検証済みの libmpv がそのまま再利用されます。ResubWinny は実行時に再生コンポーネントを自動でダウンロードまたは更新しません。

実行ファイルのみを生成し、インストーラーを生成しない場合：

```powershell
./scripts/build.ps1 -Target Executable
```

libmpv を同梱しないバージョンをビルドする場合：

```powershell
./scripts/build.ps1 -Libmpv External
```

このモードでは libmpv を配布しませんが、リアルタイムプレビューには `RESUBWINNY_LIBMPV` を通じて互換ランタイムライブラリを提供する必要があります。完全な品質チェックも同時に実行する必要がある場合は `-Check` を追加してください。libmpv を同梱したローカル成果物は、開発および非公開テスト専用です。公開リリースの前には、まったく同一の DLL に対応する検証済みソースパッケージと receipt を提供する必要があります。

### 2. 完全な品質チェックを実行

```powershell
./scripts/check.ps1
```

このスクリプトは、Worker とデスクトップバックエンドのテスト、Clippy、Rustfmt、Svelte チェック、フロントエンド／バックエンド間のインターフェース契約、fuzz target のコンパイル、ライセンス一覧、第三者ソースの検証を網羅します。

### 3. 開発時の実行

まず Worker をビルドし、その後 Tauri 開発プログラムを起動します。

```powershell
cargo build -p arib-caption-worker
npm run tauri --prefix studio-tauri -- dev
```

別の Worker を使用する必要がある場合は、`RESUBWINNY_WORKER` に実行ファイルの絶対パスを設定できます。

### 4. 基盤となるビルドコマンドを直接呼び出す

基本の Tauri 設定では libmpv の同梱を必須としていないため、以下のコマンドは開発および外部ランタイムライブラリのみを使用するビルドに適しています。デスクトップ実行ファイルのみを生成する場合：

```powershell
npm run tauri --prefix studio-tauri -- build --no-bundle
```

libmpv を同梱しない Tauri bundle を生成する場合：

```powershell
npm run tauri --prefix studio-tauri -- build
```

インストール済みでハッシュ検証済みの Windows libmpv を同梱する必要がある場合は、統一ビルドスクリプトを使用するか、設定を明示的に追加します。

```powershell
./scripts/setup-libmpv.ps1
npm run tauri --prefix studio-tauri -- build --config src-tauri/tauri.windows-libmpv.conf.json
```

成果物はすべて以下に配置されます。

```text
build/cargo/release/resubwinny-studio.exe
build/cargo/release/bundle/
```

ビルド成果物をクリーンアップする場合：

```powershell
./scripts/clean.ps1
```

`-Dependencies` を追加すると `node_modules` を削除できます。`-DownloadedRuntimes` を追加すると明示的にインストールした libmpv 開発ファイルを削除できます。`-TestOutputs` を追加するとローカルテスト出力を削除できます。

## ディレクトリ構成

```text
ResubWinny/
├── crates/
│   └── arib-caption-worker/       # ストリーム解析、パース、字幕モデル、CLI、エクスポーター
│       ├── src/caption/           # B24、TTML/B62、Ruby セマンティクス
│       ├── src/transport/         # MPEG-TS、M2TS、実験的な TLV/MMTP
│       ├── src/exporters/         # ASS、TTML、SRT、WebVTT、archive などの出力
│       └── src/tests/             # Worker のモジュール別回帰テスト
├── native/
│   └── aribcaption-bridge/        # libaribcaption の限定的な C ABI ブリッジ
├── shared/                        # Worker とデスクトップバックエンドで共有する識別ルール
├── studio-tauri/
│   ├── src/                       # Svelte フロントエンド
│   │   ├── backend/               # 型付き Tauri API とイベントエントリーポイント
│   │   ├── components/            # 共通 UI コンポーネント
│   │   ├── features/              # ホーム、タスク、複数タスク、DRCS、設定などの機能
│   │   └── locales/               # zh-CN、zh-TW、ja、en の文言
│   └── src-tauri/
│       └── src/                   # タスク、プレビュー、永続化、タイムライン、Worker 管理
├── fuzz/                          # TS、PES、B24、TTML、MMTP などの fuzz targets
├── scripts/                       # ビルド、チェック、クリーンアップ、コーパス、リリース用スクリプト
├── docs/                          # アーキテクチャ、インターフェース、コーパス、ライセンス、保守文書
├── third_party/                   # 出所を固定したサードパーティのソース、ヘッダー、フォント、ライセンス
├── .github/                       # CI、依存関係の更新、Issue、PR テンプレート
├── Cargo.toml                     # Worker workspace
├── CONTRIBUTING.md                # コントリビューション規則
├── THIRD_PARTY_NOTICES.md         # サードパーティに関する通知
└── LICENSE                        # MPL-2.0
```

生成されるすべてのファイルは、無視対象の `build/`、`node_modules/`、またはテスト出力ディレクトリに置く必要があり、ソースコードのコミットに混入させてはなりません。

## デスクトップアプリケーションの使用方法

1. `build/cargo/release/resubwinny-studio.exe` を起動します。
2. ホーム画面で録画ファイルを選択します。プログラムは内容に基づいてコンテナ、サービス、字幕トラックを検出し、ネイティブプレビューを自動的に準備します。プレビューはデフォルトで一時停止され、自動再生されません。
3. タスク画面に入り、放送サービス、字幕トラック、イベント一覧、タイムラインを確認します。番組と放送時刻は、録画ファイルに対応する放送テーブルが含まれ、それらの解析に成功した場合に表示されます。
4. プレビューウィンドウで、再生、一時停止、前後へのスキップ、タイムラインのドラッグ、音量調整を行います。下部の字幕タイムラインは、拡大縮小、クリック、ドラッグによる移動ができます。
5. 出力設定で1つ以上の形式を選択し、位置、色、Ruby、DRCS/ARIB 外字、アクセシビリティ識別子を保持するかどうかを選択します。対象形式では完全に表現できない内容がある場合、UI に警告が表示されます。
6. 出力ディレクトリを選択してからエクスポートを開始します。エクスポートを開始するまで、選択した出力ディレクトリに字幕成果物が作成されることはありません。
7. タスクログ、診断、成果物一覧で結果を確認します。未マッピングの DRCS がある場合は、DRCS 辞書で元の画像を確認し、マッピングを保存できます。
8. 録画ファイルを続けて追加して複数タスクのキューを構成できます。各タスクの状態はバックエンドによって個別に保存され、スケジューリングされます。

## CLI の使用方法

Worker のデフォルトパスは次のとおりです：

```text
build/cargo/release/arib-caption-worker.exe
```

機械可読イベントはすべて `stdout` に、人間可読ログは `stderr` に書き込まれます。現在のすべての CLI コマンドを以下に示します。

### `inspect`

字幕をエクスポートせずに、入力形式、サービス、字幕トラック、候補ルートを検出します。

```text
arib-caption-worker.exe inspect <recording>
```

### `broadcast-at`

ソースファイルのバイトオフセットに基づいて、MPEG-TS/M2TS 内の対応する放送ネットワーク、サービス、番組、放送時刻を照会します。`service-id` には10進数を使用します。

```text
arib-caption-worker.exe broadcast-at <recording> <byte_offset> [--service-id <id>]
```

### `decode-b24`

従来の B24 字幕トラックを検出して順番にデコードし、進捗および統計イベントを出力します。字幕ファイルは作成しません。

```text
arib-caption-worker.exe decode-b24 <recording>
```

### `convert`

内容に基づいてルートを自動検出し、字幕を変換します。出力パスを指定しない場合、デフォルトでは入力ファイル名の拡張子を `.ass` に変更したものを使用します。

```text
arib-caption-worker.exe convert <recording> [output] [options]
```

### `convert-b24`

従来の MPEG-TS/B24 ルートのみを使用して字幕を変換します。引数は `convert` と同じです。

```text
arib-caption-worker.exe convert-b24 <recording> [output] [options]
```

`convert` と `convert-b24` は、以下のすべてのオプションをサポートします：

| オプション | 機能 |
| --- | --- |
| `--ttml` | TTML も同時にエクスポートする |
| `--srt` | SRT 互換コピーも同時にエクスポートする |
| `--webvtt` | WebVTT 互換コピーも同時にエクスポートする |
| `--archive` | caption archive も同時にエクスポートする |
| `--archive-only` | caption archive のみを出力する。他の形式または `--no-ass` と組み合わせることはできない |
| `--raw` | 変換ルートに対応する生の PES/MMTP エビデンスをエクスポートする |
| `--no-ass` | デフォルトの ASS 出力を保持しない |
| `--drcs-report` | DRCS が検出された場合にレポートを生成する |
| `--drcs-map <json>` | 指定した JSON ファイル内の DRCS ユーザーマッピングを使用する |
| `--track-id <id>` | 字幕 PID/asset を選択する。10進数または `0x` 付き16進数を受け付ける |
| `--drop-position` | 字幕の位置を保持しない |
| `--drop-color` | 色を保持しない |
| `--drop-ruby` | Ruby を保持しない |
| `--drop-drcs` | DRCS グリフを保持しない |
| `--drop-gaiji` | ARIB 特殊外字を保持しない |
| `--drop-accessibility` | アクセシビリティ識別子を保持しない |
| `--overwrite` | 既存の出力成果物の上書きを許可する。入力録画の上書きは引き続き禁止される |

例：

```text
arib-caption-worker.exe convert recording.ts output.ass --ttml --archive --raw --drcs-report
arib-caption-worker.exe convert recording.m2ts output.ass --track-id 0x120 --srt --webvtt
arib-caption-worker.exe convert-b24 recording.ts output.ass --drop-position --drop-accessibility
arib-caption-worker.exe convert recording.ts output.caption.jsonl --archive-only
```

変換の実行中、`stdin` を介して行単位で協調制御メッセージを送信できます：

```json
{"type":"pause"}
{"type":"resume"}
{"type":"cancel","keepCheckpoint":true}
```

### `render-at`

caption archive から指定したミリ秒時点の字幕領域スナップショットを読み取り、JSONL イベントとして出力します。

```text
arib-caption-worker.exe render-at <archive.caption.jsonl> <time_ms>
```

### `dump-tlv`

実験的な TLV/MMTP の生エビデンスを抽出します。出力を指定しない場合、デフォルトで `.caption.mmtp.jsonl` を生成します。

```text
arib-caption-worker.exe dump-tlv <input> [output.caption.mmtp.jsonl] [--overwrite]
```

このコマンドは、検出された `stpp` asset 内の完全な closed-caption payload のみを出力し、TLV オフセット、MMTP/MPU シーケンス番号、元の NTP、可逆なバイト列を保持します。NTP を PTS と偽ることも、未知の asset を字幕であると推測することもありません。

## 開発と検証

Worker は単独でビルドおよびテストできます：

```powershell
cargo test -p arib-caption-worker
cargo build -p arib-caption-worker --release
```

トランスポート、プロトコル、字幕モデル、タイムライン、レンダリング、またはエクスポーターに関わる変更には、対応する回帰テストを追加する必要があります。合法ではあるものの再配布できない大容量の録画サンプルはローカルにのみ保持し、`ARIB_FIXTURE_DIR` を介して任意の長時間サンプル検証に使用します。詳細は[コーパスと回帰のドキュメント](docs/corpus.md)を参照してください。

開発に参加する前に、[コントリビューションガイド](CONTRIBUTING.md)、[中国語版アーキテクチャ文書](docs/architecture.zh-CN.md)、[バックエンドインターフェース契約](docs/backend-contract.md)、[ツールチェーンポリシー](docs/toolchain-policy.md)、[保守性に関する説明](docs/maintainability.md)をお読みください。

## 制限事項

- BS4K/8K の生の TLV/MMTP は、隔離された実験的機能であり、検証済みの汎用 BS4K/8K サポートには含まれません。
- 192-byte M2TS のサポートはパケットカプセル化ルートのみを示すものであり、BDMV/BDAV ディレクトリ、プレイリスト、CAS、またはメーカー独自の録画管理情報を完全にサポートすることを意味しません。
- ResubWinny は、録画管理ソフト、ライブ放送受信機、CAS 復号ツール、完全な EPG ブラウザーではありません。
- SRT と WebVTT では、重複する領域、放送上の位置、Ruby の組版、DRCS グラフィック、ARIB のすべての時間セマンティクスを正確に表現できません。
- BS4K/8K 信号に対応する標準 B62/ARIB-TTML の規則は、依然として研究段階にあります。

ソースコードのリリースと Windows バイナリのリリースには異なる基準が適用されます。具体的な項目は[リリースチェックリスト](docs/release-checklist.md)を参照してください。

## 余談

私にとって、日本のテレビには独特の魅力があります。テレビで放送される内容そのものはさておき、そのネットワーク体制や技術的な細部も同じように魅力的です。他の人にはこの熱中ぶりを理解できないかもしれませんが、自分が住む国のテレビ信号には、単一の音声トラック（バイリンガルすらないこともあります）と映像しかない、と言ったらどうでしょうか。初めて日本のデジタルテレビに触れたとき、オン・オフを切り替えられる字幕や、双方向のデータ放送を目にして、まるで別世界に来たような驚きを感じました。子どもの頃にテレビを見ていて、ある日、映像に焼き付けられていた字幕が突然消えたとき、分からなくなったと泣いて騒ぎ、両親に子ども向けチャンネルをいくつか追加契約してもらったことを思い返すと、自分の住む国のテレビ文化について語れるのはそれ自体の歴史だけで、その他はひたすら味気ないものだと感じます。

しかし、日本のテレビ信号は、独自に再発明された技術と「ガラパゴス化」の痕跡に満ちています。一般の人は、その専用の視聴環境を離れると、本来の姿を見にくくなります。長年にわたる開発者たちの情熱で信号源は徐々に解析可能になり、自由にテレビを見るという言葉も空文句ではなくなりました。それでも、一般の人が技術的なハードルを越えるための、便利で「理解できる」ツールは不足しています。文字を読むことは理解の始まりであり、字幕は文字を載せる媒体です。そこで私は、字幕からツールを作り始めようと思いました。

ResubWinny という名前に話を戻すと、元の Winny は2002年に公開された P2P ファイル共有ソフトウェアです。このソフトウェアの広範な利用には、著作権で保護されたコンテンツや不適切なコンテンツの流通が伴い、社会問題と見なされましたが、ソフトウェアの作者はユーザーの行為を理由に起訴されました。2023年には、この事件を題材とした同名映画が公開され、上海国際映画祭では「开发者有罪」（開発者は有罪）という題名で上映されました。

Resub は字幕の再加工を表し、Winny はこの名前の発明者である金子勇 a.k.a. 47氏への敬意を示すためのものです。また、次の基本的な常識に従うためでもあります：

**汎用技術の開発は、表現の自由の一部です。**

本プロジェクトは、P2P ネットワーク、ファイル共有、メディア探索、コンテンツ配信には関与しません。オープンソースのツールとして、メディア形式を読み取り、字幕を復元し、OCR を実行し、データを変換します。こうした技術の合法性と倫理性は、実現できる機能だけでなく、利用のされ方によって決まります。

ResubWinny を作ろうと思ったのは、日本のメディア処理に関するあまりにも多くの知識が、整理困難な 2ch の投稿、放棄された Windows ユーティリティ、可視性を階級で区分する ARIB 公式文書、そしてソースコードを提供しないソフトウェアの中に閉じ込められていたからです。こうした知識は、よりオープンで、監査可能で、移植可能で、保存可能であるべきです。既存のツールも、案内が不明瞭で理解のハードルが高いため、初心者を遠ざけてしまうことがよくあります。

したがって、このプロジェクトはオリジナル版 Winny の再現を意図したものではありません。これは一つの宣言です：**開発者には合法的なツールを構築する自由が、ユーザーには自らが所有するメディアを理解する自由が与えられるべきであり、技術そのものは簡単に使えるべきであり、知識は恐怖、訴追、または閉鎖的なコードによって失われるべきではありません。**

## 特別謝辞

- [xqq](https://github.com/xqq)：`libaribcaption` の作者であり、長年にわたり私の日本のテレビ放送研究に多大な支援を提供してくれました
- [huggy](https://github.com/makeding)：`aribb62.js` の作者であり、本プロジェクトにおける BS4K/8K 信号の字幕解析を支援してくれました
- [tsukumi](https://github.com/tsukumijima)：`KonomiTV` などのプロジェクトの作者であり、「自由にテレビを見る」文化に長年貢献しています
- Bunny：私のガールフレンドで、強力な技術的バックグラウンドを持ち、開発中に非常に難しい問題の解決を助けてくれました
- Codex：OpenAI が開発した大規模言語モデルベースのエージェントツールです。これがなければ、技術的基礎に欠ける私は自然言語でこのプロジェクトを推進することはできなかったでしょう

## ライセンス

ResubWinny 独自のソースコードには [Mozilla Public License 2.0](LICENSE) が適用されます。MPL の適用対象となるソースファイルを変更して配布する場合は、MPL-2.0 に従って対応するソースコードを提供する必要があります。ただし、ResubWinny と組み合わせたすべての独立モジュールに MPL の適用を自動的に要求するものではありません。

サードパーティのライブラリ、フォント、バイナリコンポーネント、テストコーパスには、引き続きそれぞれのライセンスと出所に関する要件が適用されます。Windows バイナリの配布では、libmpv の LGPL に基づく対応ソースコードおよび置換可能な動的ライブラリに関する要件も同時に満たす必要があります。

セキュリティ上の問題は、[セキュリティポリシー](SECURITY.md)に従って非公開で報告してください。コントリビューションされたコードにはデフォルトで MPL-2.0 が適用されます。具体的な要件は[コントリビューションガイド](CONTRIBUTING.md)を参照してください。

# ResubWinny への貢献

> 翻訳です。唯一の正本は[簡体字中国語版](CONTRIBUTING.md)です。ほかの言語: [English](CONTRIBUTING.en.md) · [繁體中文](CONTRIBUTING.zh-TW.md)

ResubWinny は、バックエンド優先のアーキテクチャを保つ、焦点の定まった修正や機能を歓迎します。大規模な transport、caption model、renderer、desktop workflow の変更を始める前に、入力 route、model invariant、期待する artifact、sample、既知の互換性上の制限を示す設計議論を開始してください。

## アーキテクチャの規則

- Svelte はバックエンドの state を表示し、型付けされた request を転送するだけです。media の parse、caption layout の計算、video decode、subtitle timing の所有はしません。
- Tauri は desktop lifecycle、永続化、native preview、Worker の監督を担います。media と caption の処理は `arib-caption-worker` に属します。
- 純粋に一時的な interface state 以外の GUI 操作には、同等の Worker/CLI または backend API が必要です。
- `CaptionPlane -> RegionInterval -> exporters` が唯一の caption-semantic path です。libaribcaption はプロジェクト所有の狭い C ABI の背後に置きます。
- input type は有界な content evidence から検出し、filename extension を信頼しません。
- TLV/MMTP は experimental で evidence-first です。verified と表現したり、unknown asset を caption と推測したりしないでください。

## ローカル環境

`rust-toolchain.toml` で固定した Rust toolchain、Node.js 22 LTS、`studio-tauri` 内での `npm ci` を使用します。生成物は `build/` の下に置き、source change には含めません。

Windows native-preview の開発には、7-Zip と、明示的に install し hash 検証した libmpv runtime も必要です。

```powershell
./scripts/setup-libmpv.ps1
```

アプリケーションがこの runtime を自ら download または update することはありません。

依存関係を install した後、完全な local quality gate を実行します。

```powershell
./scripts/check.ps1
```

`-SkipFrontend` と `-SkipFuzz` は焦点を絞った Rust のみの pass に利用できますが、pull request 前の完全な gate の代わりにはなりません。

```text
cargo test -p arib-caption-worker
cargo build -p arib-caption-worker --release
cargo test --manifest-path studio-tauri/src-tauri/Cargo.toml
npm ci --prefix studio-tauri
npm run build --prefix studio-tauri
cargo check --manifest-path fuzz/Cargo.toml
cargo fmt --check
cargo fmt --manifest-path studio-tauri/src-tauri/Cargo.toml --check
```

Rust の変更を提出する前に、warning を deny した Clippy を実行してください。transport、timeline、model、renderer、exporter の変更には、焦点を絞った regression test も必要です。合法な長時間 recording はローカルに留め、再配布が許可される場合だけ constructed または trimmed fixture を提出してください。

## 変更要件

- 公開 user text は locale file に置きます。組み込みの `en`、`ja`、`zh-CN`、`zh-TW` file は同じ key を含まなければなりません。
- Worker JSONL と型付けされた Tauri contract を versioned かつ backward-aware に保ちます。
- parse buffer と output dimension を有界にし、64-bit source offset を使います。
- unsupported source data は明示的な evidence として保持するか、stable code で reject してください。推測してはいけません。
- contract が変わるときは、README、backend contract、architecture document、corpus expectation、export limitation を更新します。
- recording、task output、log、build product、生成した dependency tree、credential、signing material を commit してはいけません。

## 依存関係とライセンス

ResubWinny source は MPL-2.0 です。新しい dependency は互換ライセンス、記録された目的、bundle する場合は固定された provenance と更新済みの license inventory が必要です。[依存関係の更新方針](docs/dependency-updates.md)に従ってください。libaribcaption、libmpv、Rounded M+ ARIB font、reference-only の aribb62.js には、それぞれ異なる更新・attribution 要件があります。

vendor source directory に nested `.git` metadata を含めてはいけません。clean で pin された libaribcaption update を review した後、`scripts/prepare-vendored-source.ps1` を実行し、この repository に置く source snapshot に変換します。

貢献することで、あなたの contribution が MPL-2.0 の下で提供されることに同意したものとします。

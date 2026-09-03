# 変更履歴

> 翻訳です。唯一の正本は[簡体字中国語版](CHANGELOG.md)です。ほかの言語: [English](CHANGELOG.en.md) · [繁體中文](CHANGELOG.zh-TW.md)

このプロジェクトは初期 Alpha 段階にあり、release には破壊的変更が含まれることがあります。

## [0.2.3-alpha.1] - 2026-09-03

### ワークスペースと初回案内

- メインワークスペースを見直し、録画の取り込み、プレビュー、よく使う操作を優先して配置しました。一般的なウィンドウの高さでもホーム画面の主な作業手順が見え、デスクトップ画面の文字位置も調整しています。
- Settings に About、ビルド情報、オフラインのライセンス一覧を追加しました。セグメントコントロールはキーボード操作に対応し、設定は自動保存されます。使われていなかったタイムライン設定は削除しました。
- 字幕オーバーレイ、ルビ、DRCS、XMB の波面を使った ARIB 風の初回案内を追加しました。アニメーションの負荷を抑え、ウィンドウ比率が変わっても 16:9 の XMB シーンを正しい比率で表示します。

### B62 / TLV 字幕処理

- ARIB-TTML の字幕処理から直接利用できるネイティブ B62 TLV バックエンドを統合しました。
- B62 のソースレイアウトの意味を保つようにしました。region と行内背景を別々に扱い、解像度に依存せず字幕を動画コンテンツの viewport に対応付けます。これにより、region の収容範囲を表示領域の境界として扱うことを避けます。

### 開発、文書、リリース

- 簡体字中国語、繁体字中国語、日本語、英語の開発者向け文書を追加し、簡体字中国語を唯一の正本として明記しました。
- Rust の依存関係、Vite、Svelte Vite プラグインを更新し、フロントエンド依存関係のライセンス一覧を更新しました。
- libmpv のビルドとキャッシュ処理を強化し、stable Cargo によるビルドとグラフィックス依存関係の設定を修正しました。Actions の artifact と cache のアクションも更新しています。
- クリーンなチェックアウトで Zlib の設定ヘッダーを生成する元ファイルを修正し、Windows のネイティブ TLV ビルドが追跡されていないファイルに依存しないようにしました。

### Windows Alpha リリース

- インストール可能な未署名 Windows x86_64 Alpha バイナリ、完全な対応ソース、ライセンス資料、ビルドレシート、SHA-256 チェックサムを初めて同梱します。
- Windows で「発行元が不明」という警告が表示されることがあります。未署名 Alpha では想定内の動作であり、コード署名による検証を意味するものではありません。

### 既知の制限

- 本バージョンは引き続きプレビュー版です。ネイティブ動画プレビューの主な受け入れ環境は Windows です。
- macOS と Linux では、ネイティブ動画プレビューをまだ提供していません。
- raw TLV/MMTP は実験的な機能であり、一般的な BS4K/8K 対応を意味しません。B62 の実放送での互換性は、再配布できない私有素材だけで検証しています。
- Windows パッケージは未署名です。私有の放送録画、それらから取り出した字幕、スクリーンショットはこのリリースに含めません。

## [0.2.2-alpha.1] - 2026-08-30

### Windows Alpha リリース

- public release を Source Release、Unsigned Windows Alpha、Signed Stable に明確に分けました。明示的に開示された public Alpha を code signing が妨げることはなくなります。
- Unsigned Windows Alpha は risk notice と dependency-license inventory を同梱し、正確な Git tag、commit、file size、SHA-256 を含む Release manifest を生成します。
- Windows candidate は、指定された compliant libmpv build が生成した同一の DLL、import library、complete corresponding source、`SOURCE-RECEIPT.json` を使用しなければなりません。hash、pin した provenance、complete source-package set は assembly 時に cross-check します。
- private な real-broadcast compatibility matrix を追加しました。installed application で complete subtitle workflow を検証し、recording、subtitle、programme metadata ではなく結果だけを公開します。

### 既知の制限

- 現在 pin している upstream libmpv development DLL はまだ public distribution できません。新たな compliant workflow により、対応する binary、complete corresponding source、build receipt を生成して永続的に公開する必要があります。
- Windows installer candidate は、public Unsigned Alpha Release を作成する前に、clean system への installation、real recording による complete workflow、uninstall acceptance を通過する必要があります。

## [0.2.1-alpha.1] - 2026-08-30

### UX と state 表示

- background preview indexing と export job を、利用者に理解できる二つの state に分離しました。indexing 中も export の設定と開始ができます。backend が作業を serialize する必要がある場合は、その待機関係を明確に説明します。
- output panel が折り畳まれていても操作失敗を確認できる、global で dismissible な persistent error banner を追加しました。
- Tasks page を開いても file picker を自動で開かず、既存の空の task page を表示するようにしました。
- Preview、Events、Diagnostics は通常の幅では text label を表示し、compact viewport でのみ icon-only を使用します。
- home page の Recent における false selected state と click target を修正しました。行全体を mouse と keyboard で開けるようにし、対応する history page のない「View all」action を削除しました。

### エンジニアリングとリリース

- cross-platform CI、Cargo dependency policy、fuzz check、Windows native dependency、lint flow を強化しました。
- source snapshot hash の cross-platform consistency を修正し、libmpv runtime provenance の pin と検証を続けています。
- source release、dependency license、repository integrity check を整備しました。

### 既知の制限

- これは引き続き preview release です。Windows は native video preview の主要な acceptance platform です。
- raw TLV/MMTP support は experimental のままであり、general BS4K/8K support と見なしてはいけません。
- この Release に public Windows binary は含まれません。signing と libmpv corresponding-source の release 要件は別途満たす必要があります。

[0.2.2-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.2-alpha.1
[0.2.1-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.1-alpha.1
[0.2.3-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.3-alpha.1

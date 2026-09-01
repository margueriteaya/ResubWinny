# 変更履歴

> 翻訳です。唯一の正本は[簡体字中国語版](CHANGELOG.md)です。ほかの言語: [English](CHANGELOG.en.md) · [繁體中文](CHANGELOG.zh-TW.md)

このプロジェクトは初期 Alpha 段階にあり、release には破壊的変更が含まれることがあります。

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

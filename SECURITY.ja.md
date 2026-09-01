# セキュリティポリシー

> 翻訳です。唯一の正本は[簡体字中国語版](SECURITY.md)です。ほかの言語: [English](SECURITY.en.md) · [繁體中文](SECURITY.zh-TW.md)

## サポート対象のバージョン

ResubWinny は現在 Alpha project です。security fix は最新の source revision と最新の `0.1.x` development package にのみ適用し、より古い development binary は保守しません。

native preview は Windows でのみ release test を行っています。macOS と Linux の desktop-preview backend は延期されており、現在は support 対象の security surface ではありません。cross-platform Worker と build check が native-preview support を意味するものではありません。

## 脆弱性の報告

exploitable な broadcast sample、private recording、credential、動作する proof of concept を含む issue を公開しないでください。利用できる場合は、repository host の private vulnerability-reporting feature を使用してください。次の情報を含めます。

- 影響を受ける revision と platform
- input route と、合法的に共有できる最小の reproducer
- expected behaviour と observed behaviour
- 問題が length、memory、filesystem、IPC、native-library、archive のいずれかの boundary をまたぐか
- personal path と programme data を取り除いた crash diagnostic

公開 repository に private reporting channel ができるまでは、sensitive material を送る前に project owner へ私的に連絡し、その開設を依頼してください。プロジェクトは完全な report を受領確認し、影響する version を評価して、fix と disclosure window を調整します。Alpha 期間中の response time は保証しません。

## セキュリティ境界

recording、TTML/XML、DRCS、archive、language pack、dependency artifact はすべて untrusted input です。parser は length と allocation の limit を強制しなければなりません。WebView は video frame を受け取らず、media の parse や caption layout の計算をしません。runtime component は libmpv、libaribcaption、font、language pack を download または暗黙に置換してはいけません。

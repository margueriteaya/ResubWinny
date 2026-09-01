[简体中文](worker-protocol.md) · [繁體中文](worker-protocol.zh-TW.md) · [日本語](worker-protocol.ja.md) · [English](worker-protocol.en.md)

> 本ページは翻訳です。簡体字中国語版のみを正式な情報源とします。

# Worker プロトコル契約

Worker メッセージは `protocolVersion`、`jobId`、`sequence`、`payload` を使用します。
移行期間中は従来のトップレベルフィールドも残します。Worker は最初に `hello` を送信し、
その後、必要に応じて上限のある stage、track、progress、diagnostic、artifact、completion、
failure の各イベントを送信します。

Tauri はイベントを転送する前にプロトコルバージョンとシーケンスを検証します。
検証に失敗した場合、元のメッセージを構造化された `expected`、`actual`、`previous`、
`current` パラメーターとともに証拠として保持します。成果物の状態は Worker イベントと
ファイルの証拠から導出し、UI が完了を推測することはありません。

probe/demux/decode、Caption IR、export、archive、evidence は Worker が担当します。
ジョブ履歴、キュー状態、チェックポイント、設定、ウィンドウのライフタイムは
Tauri アプリケーション層が引き続き担当します。

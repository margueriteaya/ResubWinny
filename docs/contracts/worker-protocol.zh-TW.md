[簡體中文](worker-protocol.md) · [繁體中文](worker-protocol.zh-TW.md) · [日本語](worker-protocol.ja.md) · [English](worker-protocol.en.md)

> 本頁是譯文。簡體中文版本是唯一權威來源。

# Worker 協定合約

Worker 訊息使用 `protocolVersion`、`jobId`、`sequence` 和 `payload`。
遷移期間保留舊版頂層欄位。Worker 會先發出 `hello`，之後視需要發出有界的
階段、軌道、進度、診斷、產物、完成或失敗事件。

Tauri 在轉送事件前驗證協定版本與序列。驗證失敗時，原始訊息會和結構化的
`expected`、`actual`、`previous` 或 `current` 引數一同保留為證據。
產物狀態由 Worker 事件與檔案證據推導；介面絕不猜測工作是否完成。

Worker 負責探測／解複用／解碼、Caption IR、匯出、archive 和證據。
工作歷史、佇列狀態、檢查點、設定與視窗生命週期仍由 Tauri 應用層負責。

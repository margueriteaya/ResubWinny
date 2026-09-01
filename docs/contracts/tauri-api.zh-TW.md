[簡體中文](tauri-api.md) · [繁體中文](tauri-api.zh-TW.md) · [日本語](tauri-api.ja.md) · [English](tauri-api.en.md)

> 本頁是譯文。簡體中文版本是唯一權威來源。

# Tauri API 合約

Svelte 應用程式是 Rust 應用層的使用者端。它不解析 TS/TLV、不解碼 ARIB、不算繪影片，也不決定轉換語意。

公開命令介面列於 [`../backend-contract.md`](../backend-contract.md)。本頁依職責將這些命令分組：

- 檢查與匯出：`inspect_source`、`start_export`、`cancel_export`、`pause_export`、`resume_export`；
- 持久化工作與復原：`create_job`、`list_jobs`、`get_job`，以及工作控制、診斷、產物、檢查點與佇列控制；
- 偏好設定與 DRCS：設定、語言套件與 DRCS 報告載入；
- 預覽與時間軸：原生預覽控制、archive 算繪、播放對映與有界時間軸視窗。

命令必須回傳有界資料與穩定錯誤程式碼。介面不得從選項推斷產物，也不得虛構後端未提供的能力。

## 介面凍結

目前命令介面處於收斂期。在底層模型仍在穩定時，不應繼續加入現有查詢的一次性變體。
時間軸查詢下次需要整合時，應優先使用一個帶引數的 `query_timeline` 請求（明確指定模式、
時間範圍與篩選器），而不是加入更多 `get_timeline_*` 命令。此類遷移必須維持回應有界、
archive 遊標語意、穩定錯誤程式碼，並同步更新前端合約。

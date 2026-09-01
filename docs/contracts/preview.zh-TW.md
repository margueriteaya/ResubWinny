[簡體中文](preview.md) · [繁體中文](preview.zh-TW.md) · [日本語](preview.ja.md) · [English](preview.en.md)

> 本頁是譯文。簡體中文版本是唯一權威來源。

# 預覽合約

原生預覽由 Tauri 後端與行程內 libmpv 負責。WebView 只提供命令並顯示有界狀態；
它絕不提交字幕點陣圖，也不執行字幕排版。

`render_at` 與 `sync_preview_overlay` 使用明確的專案時間對映，並同時回傳媒體時間與
專案時間。後端從 archive 合成字幕平面、回報所選 overlay 路線與能力中繼資料，
並以宣告方式保留不支援的 B62 功能，而不以 CSS 近似。
詳細算繪設定與路線限制見 [`backend-contract.md`](../backend-contract.md)。

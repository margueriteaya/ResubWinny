[簡體中文](preview.md) · [繁體中文](preview.zh-TW.md) · [日本語](preview.ja.md) · [English](preview.en.md)

> 本頁是譯文。簡體中文版本是唯一權威來源。

# 預覽合約

原生預覽由 Tauri 後端與行程內 libmpv 負責。WebView 只提供命令並顯示有界狀態；
它絕不提交字幕點陣圖，也不執行字幕排版。

`render_at` 與 `sync_preview_overlay` 使用明確的專案時間對映，並同時回傳媒體時間與
專案時間。後端從 archive 合成字幕平面、回報所選 overlay 路線與能力中繼資料，
並以宣告方式保留不支援的 B62 功能，而不以 CSS 近似。
詳細算繪設定與路線限制見 [`backend-contract.md`](../backend-contract.md)。

對於帶 `source_layout` 的 ARIB-TTML，後端先按來源顯示平面比例產生有界的中間字幕紋理，再把整張紋理映射至
libmpv 回報的實際視訊內容 viewport。letterbox/pillarbox、視窗大小、DPI 與全螢幕只改變最終變換，不改變字幕
相對於視訊內容的位置和面積。舊 archive 沒有 `source_layout` 時繼續按邏輯 1920×1080 相容路徑播放。

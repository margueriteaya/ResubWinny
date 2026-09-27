[简体中文](preview-composition.md) · [繁體中文](preview-composition.zh-TW.md) · [日本語](preview-composition.ja.md) · [English](preview-composition.en.md)

> **規範性說明：** 簡體中文版本是唯一權威來源。其他語言版本是同步譯文；如措辭存在歧義或衝突，以簡體中文版本為準。

# 預覽合成方案決定

Windows 預覽使用程序內 `libmpv-render`。ResubWinny 不啟動 `mpv.exe`，也不使用 JSON named pipe。macOS 與 Linux 沿用同一套字幕模型，但原生預覽後端尚未實作。

libmpv 與 Rust 字幕在專用渲染執行緒的離屏 FBO 中合成。一個從不顯示、沒有父視窗並位於螢幕外的 WGL host 只負責提供 OpenGL device context；它不承載影片畫面，也不參與應用程式視窗的層疊、移動或縮放。目前路徑不建立可見或子級 HWND，也沒有 `wid`、`overlay-add` 或按來源回退。

合成後的 RGBA 影格透過輪換的 PBO 讀回。每幀在提交後立即映射，避免影片固定落後 libmpv 音訊。三個 WebView2 SharedBuffer 像素槽把影格交給播放器元件內的 WebGL Canvas；控制槽只傳遞序號、尺寸與槽狀態。Canvas 只領取最新的就緒影格，並釋放已上傳或過期的槽，因此逐幀像素不經 Tauri 命令、事件或 COM 複製。

視窗移動、面板收合和 DPI 變化只改變 Canvas 的 DOM 版面。ResizeObserver 將元件的實體像素尺寸傳給後端，後端只調整離屏 FBO；畫面與播放器控制項始終位於同一個 WebView 內容層，不再追蹤原生視窗座標。共享表面限制為 1920×1080，較大的來源影片由 libmpv 縮放到元件實際尺寸。

`get_preview_capabilities` 只報告目前可用的 `libmpv-render` 路線。`get_preview_render_diagnostics` 報告渲染、呈現與丟棄影格數、表面尺寸、呈現節奏、字幕紋理操作、影片寬高比、解碼策略及最近錯誤。`render_at(time)` 繼續用於存檔時間點的有界字幕快照；`sync_preview_overlay` 仍負責依 libmpv 媒體時間查詢並更新 Rust 字幕紋理，不在 WebView 中建立第二套播放時鐘或字幕排版器。

渲染執行緒請求 `hwdec=auto-safe`，並在可用時報告 libmpv 實際選擇的 `hwdec-current`。這允許相容的回拷硬體解碼，不表示 D3D/ANGLE 零拷貝。應用程式退出和預覽停止都會先停止並 join 渲染執行緒，再於 WebView UI STA 釋放 SharedBuffer，最後銷毀隱藏的 WGL host。

`scripts/validate-preview.ps1` 使用 `ARIB_FIXTURE_DIR` 中合法持有的本機錄影執行 Windows 冒煙門禁。門禁涵蓋真實解碼首幀、seek、暫停/繼續、字幕紋理混合、尺寸變更、截圖和有界關閉。私有廣播素材不會提交到儲存庫；公開回歸依賴合成與協定測試。

# 變更紀錄

> 譯文。唯一權威來源為[簡體中文版本](CHANGELOG.md)。其他語言：[English](CHANGELOG.en.md) · [日本語](CHANGELOG.ja.md)

本專案仍處於早期 Alpha 階段，版本可能包含破壞性變更。

## [0.2.3-alpha.1] - 2026-09-03

### 工作區與首次引導

- 重新編排主工作區：優先呈現錄製入口、預覽和常用控制項；首頁主流程在一般視窗高度內保持可見，並微調桌面工作流程的文字基線。
- 新增 Settings 中的 About、建置來源資訊和離線授權瀏覽器；分段控制項補齊鍵盤操作，偏好設定會自動持續儲存，並移除了未使用的時間軸偏好項目。
- 新增 ARIB 風格的首次引導，透過字幕疊加、Ruby、DRCS 與 XMB 波面介紹工作流程；降低動畫開銷，並修正 16:9 XMB 場景在不同視窗比例下的顯示比例。

### B62 / TLV 字幕處理

- 整合原生 B62 TLV 後端，可由 ARIB-TTML 字幕工作流程直接使用。
- 保留 B62 原始版面語意：region 與行內背景分別處理，字幕以與解析度無關的方式映射到視訊內容 viewport，避免將 region 容量當作顯示平面的邊界。

### 工程、文件與發行

- 新增簡體中文、繁體中文、日語和英語開發者文件，並明確指定簡體中文為唯一權威來源。
- 更新 Rust 相依套件、Vite 與 Svelte Vite 外掛，並刷新前端相依套件授權清單。
- 強化 libmpv 建置與快取流程，修正 stable Cargo 建置和圖形相依設定，並升級 Actions 的 artifact 與 cache 操作。
- 修正乾淨 checkout 中 Zlib 設定標頭的生成來源，使 Windows 原生 TLV 建置不再依賴未追蹤檔案。

### Windows Alpha 發行

- 首次提供可安裝的未簽署 Windows x86_64 Alpha 版本；Release 附件僅包含不帶語言標籤的 NSIS setup 與 MSI 安裝程式。
- setup 採用全系統安裝模式，啟動安裝時會自動要求系統管理員權限；MSI 同樣按全系統範圍安裝。
- Windows 可能顯示「未知發行者」警告。這是未簽署 Alpha 的預期行為，並不代表已通過程式碼簽署驗證。

### 已知限制

- 目前仍為預覽版；Windows 是原生視訊預覽的主要驗收平臺。
- macOS 與 Linux 尚未提供原生視訊預覽。
- 原始 TLV/MMTP 支援仍為實驗性，不應視為通用 BS4K/8K 支援；B62 的真實廣播相容性僅以不可再散佈的私有素材驗證。
- Windows 套件未簽署。私有廣播錄製檔、從中產生的字幕和螢幕截圖不會隨本版散佈。

## [0.2.2-alpha.1] - 2026-08-30

### Windows Alpha 發行

- 將公開發行明確分為 Source Release、Unsigned Windows Alpha 與 Signed Stable；程式碼簽署不再阻礙已有明確揭露的公開 Alpha。
- 未簽署 Windows Alpha 現在隨套件提供風險說明與相依授權清單，並產生含有精確 Git tag、commit、檔案大小與 SHA-256 的 Release manifest。
- Windows candidate 必須使用指定合規 libmpv 建置產出的相同 DLL、import library、完整對應原始碼與 `SOURCE-RECEIPT.json`；雜湊、固定來源與完整原始碼套件集合會在組裝時交叉驗證。
- 新增私人的真實廣播相容性矩陣，規定使用已安裝的應用程式驗證完整字幕工作流程，同時僅公開結果、不公開錄製檔、字幕或節目中繼資料。

### 已知限制

- 目前固定的上游 libmpv 開發 DLL 尚不可公開散佈；必須先由新的合規 workflow 產生並長期發行相符的二進位檔、完整對應原始碼與建置回執。
- Windows 安裝程式 candidate 仍須透過乾淨系統安裝、真實錄製檔完整工作流程與解除安裝驗收，才可建立公開 Unsigned Alpha Release。

## [0.2.1-alpha.1] - 2026-08-30

### UX 與狀態表達

- 將背景預覽索引與匯出工作拆分為兩個使用者可理解的狀態。索引期間仍可設定並開始匯出；後端必須序列化工作時會清楚說明等待關係。
- 新增全域、可關閉的持久錯誤橫幅，即使輸出面板摺疊仍能看見操作失敗。
- 進入 Tasks 頁面時不再自動彈出檔案選擇器，而是顯示既有的空白任務頁面。
- Preview、Events 與 Diagnostics 在一般寬度顯示文字標籤，只在緊湊 viewport 使用純圖示。
- 修正首頁 Recent 的虛假選取狀態與點選範圍。整列支援滑鼠和鍵盤開啟，並移除沒有相對應歷史頁面的「View all」操作。

### 工程與發行

- 強化跨平臺 CI、Cargo 相依策略、fuzz 檢查、Windows 原生相依與 lint 流程。
- 修正原始碼快照雜湊的跨平臺一致性，並持續固定及驗證 libmpv 執行階段來源。
- 完善原始碼發行、相依授權與儲存庫完整性檢查。

### 已知限制

- 目前仍為預覽版；Windows 是原生影片預覽的主要驗收平臺。
- 原始 TLV/MMTP 支援仍為實驗性，不應視為通用 BS4K/8K 支援。
- 本 Release 不附公開 Windows 二進位檔；簽署與 libmpv 對應原始碼等發行門檻仍須分別滿足。

[0.2.2-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.2-alpha.1
[0.2.1-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.1-alpha.1
[0.2.3-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.3-alpha.1

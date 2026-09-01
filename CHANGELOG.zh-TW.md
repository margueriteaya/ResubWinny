# 變更紀錄

> 譯文。唯一權威來源為[簡體中文版本](CHANGELOG.md)。其他語言：[English](CHANGELOG.en.md) · [日本語](CHANGELOG.ja.md)

本專案仍處於早期 Alpha 階段，版本可能包含破壞性變更。

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

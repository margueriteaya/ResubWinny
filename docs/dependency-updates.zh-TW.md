# 第三方相依性更新政策

[簡體中文](dependency-updates.md) · [繁體中文](dependency-updates.zh-TW.md) · [日本語](dependency-updates.ja.md) · [English](dependency-updates.en.md)

> **規範性說明：** 簡體中文版本是唯一權威來源。其他語言版本僅為同步譯文；如有歧義或衝突，以簡體中文版本為準。

ResubWinny 使用固定且可審查的相依性更新。應用程式執行時絕不下載或替換剖析器、轉譯器、字型或播放元件。更新自動化可以提出建議，但不得合併或釋出。

## 相依性類別

| 類別 | 範例 | 固定與更新規則 |
| --- | --- | --- |
| 隨附原始碼 | libaribcaption | 在 `third_party/versions.json` 中固定上游標籤、完整提交和確定性的原始碼快照雜湊；審查原始碼差異及授權條款，隨後用 `scripts/prepare-vendored-source.ps1` 移除巢狀 Git 中繼資料。 |
| 下載的二進位執行階段 | Windows libmpv | 固定發行標籤提交、工作流程配方提交/執行、工具鏈提交、上游 mpv 提交、資產名稱、封存雜湊及解壓後雜湊。`scripts/setup-libmpv.ps1` 為開發和封裝明確安裝它；應用程式絕不下載它。絕不可只替換 DLL 而不連同其標頭、宣告和對應原始碼計畫。 |
| 僅供參考的原始碼 | aribb62.js | 固定已審查的提交。上游變更是研究輸入，不是可執行相依性，也不會自動移植。 |
| 套件管理的原始碼 | Cargo 與 npm 套件 | 鎖定檔是權威來源。Dependabot 可以提出更新；維護者負責審查和測試。 |
| 視覺資產 | 用於 ARIB 的 Rounded M+ 1m | 固定二進位雜湊、來源和授權條款。替換時必須進行字形涵蓋和視覺 golden 比較。 |

## 必需的更新記錄

每次相依性更新必須記錄：

1. 舊版和新版、提交、成品雜湊及上游 URL；
2. 上游發行說明及已審查的原始碼/ABI 差異；
3. 授權條款、版權、建置選項和傳遞相依性的變更；
4. 受影響的 ResubWinny 路由和模型不變數；
5. 為更新執行的測試和語料證據；
6. 對 archive、ASS、TTML、DRCS 和預覽的輸出相容性影響；
7. 復原提交或先前成品識別。

## 驗證門檻

所有更新均須執行一般專案門檻：

```text
cargo test -p arib-caption-worker
cargo check --manifest-path studio-tauri/src-tauri/Cargo.toml
npm run build --prefix studio-tauri
cargo check --manifest-path fuzz/Cargo.toml
cargo fmt --check
```

附加門檻取決於元件：

- **libaribcaption：** 橋接 ABI 編譯、B24 解碼語料、DRCS 對應、RegionInterval 時間以及 B24 視覺 golden 比較。即使 C ABI 未變，更改字元對應、控制碼、預設選項或轉譯器也屬於語義變更。
- **libmpv：** 匯出符號檢查、可替換性檢查、原生預覽冒煙測試、seek/pause/resume、疊加層時鐘同步、調整大小/DPI，以及 2K/4K/8K 效能樣本。驗證成品仍是 LGPL 建置，並用 `scripts/package-libmpv-source.ps1` 封裝其精確原始碼快取。
- **aribb62.js：** 手動檢查上游變更。只移植由 ARIB 檔案或語料證據支援、且已被獨立理解的行為。在其可再散佈授權條款明確前，絕不複製新增程式碼。
- **字型：** 字形涵蓋、缺字診斷、橫排/直排 ruby、標點方向、描邊/背景，以及邏輯 2K/4K/8K 視覺等價性。

剖析器或轉譯器更新在釋出前必須進行長樣本回歸。改變預期輸出的變更必須更新 golden 資料，並說明為何新結果更正確；禁止默默接受新輸出。

## 安全更新

高嚴重性安全更新可以採用加急審查，但仍需要授權條款驗證、受影響邊界的重點測試和明確的復原成品。只有在發行說明記錄該例外並排定被省略的門檻時，才可以跳過無關的長時間測試。

## 檢查上游

執行 `scripts/check-upstreams.ps1 -Online` 以驗證本機雜湊，並將固定的提交與目前上游頭部比較。當可用上游更新應產生失敗的維護訊號時，請在排程 CI 中加入 `-FailOnUpdate`。存在可用更新不代表獲得合併許可。

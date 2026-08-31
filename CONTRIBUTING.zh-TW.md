# 貢獻 ResubWinny

> 譯文。唯一權威來源為[簡體中文版本](CONTRIBUTING.md)。其他語言：[English](CONTRIBUTING.en.md) · [日本語](CONTRIBUTING.ja.md)

ResubWinny 歡迎能維持後端優先架構的聚焦修正與功能。開始涉及傳輸、字幕模型、渲染器或桌面工作流程的大型變更前，請先發起設計討論，說明輸入路線、模型不變數、預期產物、樣本與已知相容性限制。

## 架構規則

- Svelte 僅顯示後端狀態並轉送型別化請求；它不解析媒體、不計算字幕排版、不解碼影片，也不擁有字幕時間。
- Tauri 負責桌面生命週期、持久化、原生預覽及 Worker 監管。媒體與字幕處理屬於 `arib-caption-worker`。
- 除純暫態的介面狀態外，每項 GUI 操作都必須有等價的 Worker/CLI 或後端 API。
- `CaptionPlane -> RegionInterval -> exporters` 是唯一的字幕語意路徑。libaribcaption 始終位於專案維護的窄 C ABI 之後。
- 輸入型別應從有界的內容證據探測，絕不信任副檔名。
- TLV/MMTP 為實驗性、證據優先功能；不得稱其已驗證，也不得將未知 asset 推斷為字幕。

## 本機環境

使用 `rust-toolchain.toml` 固定的 Rust 工具鏈、Node.js 22 LTS，並在 `studio-tauri` 中執行 `npm ci`。產生的檔案應位於 `build/` 下，不屬於原始碼變更。

Windows 原生預覽開發還需要 7-Zip，以及明確安裝並透過雜湊驗證的 libmpv 執行階段：

```powershell
./scripts/setup-libmpv.ps1
```

應用程式絕不會自行下載或更新此執行階段。

安裝相依項後，執行完整本機品質門檻：

```powershell
./scripts/check.ps1
```

`-SkipFrontend` 與 `-SkipFuzz` 可用於聚焦的僅 Rust 檢查；它們不能取代提交 Pull Request 前的完整門檻。

```text
cargo test -p arib-caption-worker
cargo build -p arib-caption-worker --release
cargo test --manifest-path studio-tauri/src-tauri/Cargo.toml
npm ci --prefix studio-tauri
npm run build --prefix studio-tauri
cargo check --manifest-path fuzz/Cargo.toml
cargo fmt --check
cargo fmt --manifest-path studio-tauri/src-tauri/Cargo.toml --check
```

提交 Rust 修改前，以拒絕警告的方式執行 Clippy。涉及傳輸、時間軸、模型、渲染器或匯出器的變更還需針對性回歸測試。合法的長時錄製檔僅保留在本機；只有允許再散佈時，才提交建構的或裁剪過的 fixture。

## 變更要求

- 將面向使用者的公開文字保留在 locale 檔案中。內建的 `en`、`ja`、`zh-CN` 與 `zh-TW` 檔案必須包含相同鍵。
- 保持 Worker JSONL 與型別化 Tauri contract 已版本化，並顧及向後相容。
- 保持解析緩衝區與輸出尺寸有界；使用 64 位元來源檔偏移。
- 將不支援的來源資料保留為明確證據，或以穩定程式碼拒絕；不得猜測。
- 合約變動時更新 README、後端 contract、架構檔案、語料預期與匯出限制。
- 不得提交錄製檔、任務輸出、日誌、建置產物、產生的相依樹、憑證或簽章材料。

## 相依項與授權條款

ResubWinny 原始碼採用 MPL-2.0。新相依項必須使用相容授權、記錄用途；若隨專案封裝，還須固定來源並更新授權清單。請遵循[相依項更新策略](docs/dependency-updates.md)；libaribcaption、libmpv、Rounded M+ ARIB 字型和僅供參考的 aribb62.js 各有不同的更新及署名要求。

vendor 原始碼目錄不得含有巢狀 `.git` 中繼資料。審查乾淨、固定的 libaribcaption 更新後，執行 `scripts/prepare-vendored-source.ps1`，將其轉換為應放入此儲存庫的原始碼快照。

參與貢獻即表示你同意依 MPL-2.0 提供你的貢獻。

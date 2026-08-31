[English](maintainability.en.md) | [簡體中文](maintainability.md) | [日本語](maintainability.ja.md) | [繁體中文（臺灣）](maintainability.zh-TW.md)

> 翻譯宣告：簡體中文版本是唯一的權威來源。本繁體中文（臺灣）版本僅供參考。

# 可維護性審查

本檔案記錄目前的工程邊界，以及儲存庫適合公開發布原始碼之前仍需完成的工作。

## 已確立的邊界

- Svelte 應用程式負責呈現狀態並呼叫具型別的 Tauri 閘道。它不剖析傳輸串流、不解碼字幕，也不繪製視訊影格。
- Tauri 服務負責桌面生命週期、持久化、原生預覽和 Worker 監管。媒體剖析仍在 `arib-caption-worker` 中。
- Worker 將 `CaptionPlane -> RegionInterval -> exporters` 保持為唯一語意路徑，並且僅透過狹窄的 C 橋接層使用 libaribcaption。
- 產生的檔案隔離在 `build/` 下。原始碼建置不依賴預先存在的 `target/`、`dist/` 或已簽入的記錄檔。
- Tauri 套件建立會明確建置其發布版 Worker 資源。直接執行桌面端 `cargo check/test/clippy` 會略過僅限套件的資源驗證，因此貢獻者不需要只為檢查 Rust 程式碼而保留過時的發布版 Worker。當 Worker 或其他套件資源缺少時，發布建置仍會失敗。
- ResubWinny 原始碼採用 MPL-2.0 授權。Rust 和前端套件中繼資料宣告相同的 SPDX 識別碼，桌面套件包含根目錄的正式授權條款。

## 已完成的拆分

| 原熱點 | 目前結構 |
| --- | --- |
| 桌面字幕繪製器 | `caption_renderer.rs` 現在負責協調合成；`layout`、`rich_text`、`style`、`glyph`、`bitmap` 和 `tests` 分別負責聚焦的關注點。 |
| 桌面預覽 | `preview.rs` 負責能力探索和穩定的原生命令包裝函式；封存分頁、疊加層同步、Windows 原生播放、不支援平臺的虛設常式和測試均為獨立模組。 |
| 桌面工作 | `jobs.rs` 負責公開工作模型；JSON/JSONL 持久化和佇列監管器分別隔離在 `jobs/repository.rs` 和 `jobs/supervisor.rs` 中。 |
| Worker 匯出器 | 公開匯出器邊界仍位於 `exporters/mod.rs`；ASS、TTML、文字格式、B24 協調、證據和 Ruby 版面配置位於按格式聚焦的模組中。 |
| Worker TTML | B62 語意、嚴格 XML 檔案解碼和 TS/PES 掃描分別位於獨立的 `ttml`、`document` 和 `scan` 模組中。 |
| 實驗性 TLV/MMTP | 基礎封包/MPU 處理、訊號/MPT、證據寫入和受約束路徑分別位於獨立模組中。 |
| Worker 測試 | 語料庫、TS/M2TS、B24/時間軸、TTML、TLV、封存和合成通訊協定套件在獨立檔案中各自管理其測試資料；完整基準為 146 項測試。 |
| libmpv | 動態使用者端 ABI/播放與 Windows 繪製 Worker 已分離；繪製測試已隔離。 |
| 桌面時間軸 | 公開分頁/呈現保留在 `timeline.rs`；有界即時視窗和附加遊標狀態隔離在 `timeline/cache.rs` 中。 |
| Svelte 應用程式 | 佈景主題/地區設定偏好、多工作協調、DRCS 字典狀態、工作呈現和輸出格式中繼資料已移入功能控制器。多工作、DRCS 和設定檢視現在位於其所屬功能目錄下，而非原始碼根目錄。 |

剩餘的應用程式殼層是一個明確的組合根。`SourceSession` 負責來源準備、檢查世代、忙碌狀態生命週期和過時結果/錯誤的抑制，然後僅為目前來源套用工作設定並啟用預覽/索引。`ExportSession` 負責匯出/索引要求的有效性，包括過時的工作建立回呼、過時失敗和預覽索引取消，以及對應的開始/成功/失敗狀態投影。`PreviewSession` 負責原生預覽幾何、生命週期和受管理的開始/停止轉換、定位/拖曳協調，以及區分未知的第一個媒體樣本與實際零時間戳記。它也負責調整大小的合併及預覽頁面的產生/繼續狀態，因此過時的 WebView 主機和無型別繼續時間戳記不會洩漏到應用程式殼層。播放對應持久化和明確的媒體到專案遊標重新對應也保留在此預覽領域內，播放器命令及音量 IPC 錯誤/通知處理亦然。

成功檢查後的預設值由純工作設定轉換產生；殼層不再逐欄位重建輸出路徑、初始軌道/格式選擇或來源通知。批次控制器負責佇列生命週期和編輯專案軌道投影，而跨功能工作啟用仍在組合根中。
`HistorySession` 負責有界工作歷程持久化，`LayoutSession` 負責回應式殼層轉換。`runtime-session.ts` 集中管理工作執行階段重設；`feedback-session.ts` 集中管理有界通知和後端錯誤訊息；`selection-session.ts` 集中管理輸出格式、保留和軌道選擇轉換；`bootstrap-session.ts` 載入彼此獨立的桌面啟動資源；`application-lifecycle-session.ts` 負責桌面事件訂閱和清除；`recovery-session.ts` 負責檢查點資格判定和重播。這些工作階段將結果投影到 Svelte 值中，但不會成為第二個全域存放區。

目前最大的正式環境檔案是 Worker `exporters/ass.rs`（約 1,185 行）、`caption/ruby.rs`（約 1,080 行）、`App.svelte`（約 1,100 行）、Worker `caption/ttml.rs`（約 764 行）、桌面端 `jobs/repository.rs`（約 720 行）以及前端 `features/batch/BatchQueue.svelte`（約 632 行）。匯出器、工作和預覽進入模組現在是小型所有權邊界，而非實作收納容器。進一步拆分應遵循 ASS 事件建構、Ruby 關聯/版面配置、應用程式工作階段生命週期、儲存庫關注點以及多工作表格/預設關注點，而不是任意的行數門檻。

時間領域在其所有權邊界上均為明確。前端和桌面對應層區分媒體毫秒與專案毫秒，而 Worker 將 33 位元 MPEG PES 時鐘表示為 `Pts90k`，並僅在進入字幕 IR、證據或時間軸處理時將其轉換為毫秒。MMT 呈現 NTP 仍是獨立的傳輸概念。

字幕 IR 的匯聚發生在剖析之後，而非傳輸模型中。封閉的零複製 `CaptionCueRef` 為 B24 `RegionInterval` 和 ARIB-TTML `TtmlCaption` 公開共用的時間、區域、路徑、純文字、Ruby 數量和 DRCS 存在性語意，同時保留其完整的路徑特定 DRCS、Ruby、樣式和來源承載資料。封存寫入器使用此共用邊界，但保留 schema-v1 的 `region_interval` 和 `caption` 記錄形狀。

若干繪製器熱門路徑函式仍明確傳遞幾何資訊，以避免配置暫存內容物件。相容性 `start_export`、`create_job`、Worker 事件輔助函式和 libmpv 執行緒進入點也具有寬簽章。其 lint 例外均為區域性且有理由；新 API 應使用具型別的要求/狀態物件。現有 Tauri 引數名稱只能在協調完成前端合約移轉時變更。

## 建置和品質關卡

- Worker、桌面 crate 和模糊測試 crate 的 Cargo 輸出統一在 `build/cargo/` 下；Vite 輸出位於 `build/frontend/`。
- `scripts/clean.ps1` 會移除目前輸出以及過時的根目錄、模糊測試、Vite 和 Tauri 輸出位置。`-Dependencies` 還會移除 `node_modules`。
- Worker 和桌面 Clippy 在 CI 中使用 `-D warnings` 執行。
- 目前已驗證基準為 146 項 Worker 測試和 106 項透過的桌面測試。四項真實錄影/封存環境及效能測試仍為選擇性啟用，因為它們需要 Windows 桌面工作階段、合法錄影或封存路徑，以及路徑特定的效能門檻。
- 前端合約檢查目前涵蓋 58 個具型別命令、64 個原始檔和四個完整的內建地區設定檔；Svelte 建置無診斷訊息。
- `scripts/check.ps1` 是格式化、Worker 和桌面測試/lint、前端建置、模糊測試編譯及產生依賴授權清單的唯一本機進入點。
- `scripts/build.ps1` 是唯一封裝進入點。其 Windows 預設值為套件設定檔，該設定檔會明確安裝並驗證固定版本的執行階段；`-Libmpv External` 會產生不含 libmpv 的套件，並要求使用者提供相容執行階段。Tauri 基礎設定本身不會無提示地綑綁執行階段。
- 一般 CI 路徑有四個聚焦工作：一個共用靜態品質關卡、一個三平臺 Rust 測試矩陣、模糊測試目標編譯和依賴稽核。每週排程工作流程會對每個模糊測試目標執行有界的 30 秒運作；提取要求保留僅編譯的模糊測試涵蓋。`cargo-deny` 對 Worker、桌面端和模糊測試資訊清單強制執行已簽入的授權/來源原則。耗時較長的 LGPL libmpv 建置為手動執行，並與提取要求 CI 隔離。它直接在 GitHub Ubuntu 執行器上執行，並在對應原始碼封存旁記錄完整的工具/套件環境。
- `scripts/verify-repository.ps1` 拒絕產生/下載的成品、巢狀儲存庫、過大的追蹤檔案和發布版本偏移。`scripts/package-source.ps1` 從乾淨的 Git 修訂版建立按雜湊定址的原始碼封存；兩條路徑都已在暫儲存存庫中實際執行。
- GitHub 議題和提取要求範本記錄合法樣本邊界、受影響的傳輸路徑、模型不變數和驗證證據。

## 公開發布阻礙專案

- 每次依賴更新時，必須保持 `THIRD_PARTY_NOTICES.md` 與 `third_party/versions.json` 同步。現已記錄準確的 libaribcaption/libmpv 修訂版、雜湊、授權、來源位置和動態替換說明。
- 必須將大型 Windows libmpv 二進位檔排除在 Git 之外。`scripts/setup-libmpv.ps1` 會驗證其固定版本封存和解壓縮後雜湊；Windows CI 和封裝會呼叫該明確設定步驟。
- 必須保持納入版本庫的 libaribcaption 提交與來源快照雜湊同步。其巢狀 Git 中繼資料已移除；今後的更新在進入根儲存庫之前必須透過 `scripts/prepare-vendored-source.ps1`。
- 必須為確切綑綁的 Windows libmpv 建置映象一個持久、完整的對應原始碼封存及建置指令碼。適用的 LGPL 文字、建置來源、雜湊和替換機制現已記錄，但不能只將上游 URL 視為最終發布成品。
- 必須確保字型旁的 Rounded M+ 1m for ARIB 來源/授權檔包含在每個安裝程式和二進位封存中。已透過 SHA-256 將綑綁二進位檔與其記錄的上游檔案比對一致。
- `CONTRIBUTING.md`、`SECURITY.md` 和支援的工具鏈原則現已存在。Windows Alpha 候選工作流程會執行完整封裝關卡並寫入安裝程式雜湊，但不會建立公開發布。
- 必須記錄行為準則決定。Signed Stable 發布需要受保護的簽署身分，但明確揭露且滿足原始碼、雜湊、來源和授權關卡的 Unsigned Windows Alpha 不需要。
- 必須移除架構檔案中不再符合實際實作的宣告，並確保全部三種語言版本描述相同的已驗證及實驗效能力邊界。

## 建議順序

1. 建立可稽核的 Unsigned Windows Alpha 管線，發布精確的標籤和提交、完整成品雜湊、未簽署建置警告、通知以及綑綁 libmpv 的對應原始碼收據。
2. 針對來源選擇、原生預覽、動態廣播中繼資料、多工作控制、語言套件、輸出規劃和成品發布執行已封裝 Windows 端對端驗收。來源選擇、暫停的原生視訊、動態中繼資料、118 事件索引和最終封存時間軸復原已用 `bs4k_test_2.ts` 驗證；其餘工作流程仍需封裝驗收。
3. 維護私有的真實廣播相容性矩陣，並僅發布其結果。不得新增合成廣播產生來取代合法持有的錄影，並須將 TLV/MMTP 明確保持為實驗性功能。
4. 為純前端行為和產生的 Rust-to-TypeScript DTO 型別新增聚焦測試，且不得引入前端測試框架或 RPC 框架。
5. 為確切綑綁的 LGPL libmpv 建置產生固定、完整的對應原始碼套件；在此完成之前，目前開發 DLL 會阻礙公開二進位發布。
6. 保持 Cargo/npm 依賴稽核啟用。僅在 libmpv 對應原始碼關卡透過後發布未簽署 Alpha；將簽署作為建置提升到 Signed Stable 時的一項獨立要求。

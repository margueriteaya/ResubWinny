[簡體中文](backend-contract.md) | [English](backend-contract.en.md) | [日本語](backend-contract.ja.md) | [繁體中文](backend-contract.zh-TW.md)

> **規範性說明：** 簡體中文版本是唯一權威來源。其他語言版本是同步譯文；如措辭存在歧義或衝突，以簡體中文版本為準。

# 後端合約

> 2026-09-02 實作說明：本文的邏輯 1920×1080 平面只是有界中間紋理，不是定義正確性的目標解析度。
> Worker 在可選 `source_layout` 中保留來源平面、region、樣式和行內長度；原生算繪器由此明確計算
> 中間紋理，再將整張紋理映射至 libmpv 的視訊內容 viewport。正確性以排除黑邊後相對視訊內容的比例為準。
Tauri/Svelte UI 是 Rust 後端的客戶端。它不解析 TS/TLV 資料、解碼 ARIB、渲染高解析度影片或決定轉換語義。

持久的 `.caption.jsonl` 格式在 [`contracts/archive.md`](contracts/archive.md) 中單獨指定，包括其顯式模式版本和流式讀取器相容性規則。

該合同分為重點閱讀指南：[`contracts/tauri-api.md`](contracts/tauri-api.md)、[`contracts/worker-protocol.md`](contracts/worker-protocol.md)、[`contracts/preview.md`](contracts/preview.md) 和 [`contracts/timeline.md`](contracts/timeline.md)。該檔案保留了相容性索引和詳細參考。

後端表面是一個有界的、穩定的應用契約。在當前的收斂階段，更喜歡合併相關查詢而不是新增新的一次性命令變體：

| 命令 | 責任 |
| --- | --- |
| `inspect_source` | 錄音和字幕軌道發現的有界探測 |
| `start_export` | 啟動流工作器併發出 `task-event` 進度；接受可選的經過驗證的 `trackId` |
| `cancel_export` | 停止當前工作程序 |
| `pause_export` / `resume_export` | 向工作人員傳送協作控制訊息 |
| `create_job` / `list_jobs` / `get_job` / `remove_job` | 在沒有媒體負載的情況下保留任務摘要 |
| `start_job` / `pause_job` / `resume_job` / `cancel_job` | 透過工人主管控制持久化作業 |
| `get_job_diagnostics` | 返回為持久作業收集的有界結構化診斷資訊 |
| `get_job_diagnostics_window` | 使用偏移/限制返回有界診斷頁 |
| `list_jobs_window` | 返回最近任務摘要的有界頁面 |
| `get_job_artifacts` | 返回任務工件清單和 `.part` 路徑 |
| `get_job_checkpoint` | 返回任務的最新有界進度檢查點 |
| `pause_queue` / `resume_queue` / `queue_is_paused` | 控制 Supervisor 佇列並協作暫停/恢復其活動 Worker |
| `load_drcs_report` | 讀取工作人員生成的 DRCS 報告並返回可顯示的字形影象 |
| `get_settings` / `update_settings` | 讀取或自動更新經過驗證的 UI 並匯出應用程式資料 `settings.json` 中的預設值 |
| `list_language_packs` | 從固定的 app-data `language-packs/` 目錄中重新掃描有界的 JSON 語言檔案；不接受任意瀏覽器提供的目錄 |
| `open_language_pack_directory` | 在需要時建立該固定目錄並使用平臺檔案管理器開啟它 |
| `start_preview` / `resize_preview` / `stop_preview` | 控制當前程序內 libmpv 影片表面 |
| `preview_command` | 將查詢/暫停命令轉發到 libmpv |
| `get_preview_capabilities` | 報告宣告的影片/字幕合成路線以及僅當前可用的路線 |
| `get_preview_runtime` | 報告發現的 libmpv 執行時以及渲染 API 符號可用性，而不宣告渲染表面存在 |
| `get_preview_render_diagnostics` | 報告活動的本機路由和有界渲染執行緒計數器/錯誤；缺少工作人員會返回穩定的非活動結果 |
| `render_at` | 返回請求的存檔時間的有界字幕平面快照，而不透過 WebView 傳送影片幀 |
| `sync_preview_overlay` | 讀取嵌入的 libmpv 時間，渲染有界本機平面，並應用、清除或刪除 Windows 覆蓋層，無需 WebView 計時或佈局 |
| `get_playback_time_mapping` / `update_playback_time_mapping` | 獲取或替換本機字幕預覽使用的經過驗證的媒體時間→專案時間段對映 |
| `get_timeline_window` / `get_timeline_window_filtered` | 流式傳輸有界存檔頁面以供完成的任務瀏覽 |
| `get_timeline_recent_window_filtered` | 增量尾部完整的 JSONL 記錄並僅返回最新的有界實時事件頁面 |
| `get_timeline_time_window` | 返回編輯器時間線的有界預取時間範圍並增量讀取附加記錄 |

存檔匯出完成後，`render_at` 將在任務工作區中公開。當存檔包含 B24 渲染幀時，UI 保持時間查詢顯式且有界，並顯示真正的 RGBA 派生 PNG。後端返回`planeWidth`、`planeHeight`、`composedPngBase64`、`activeLayerCount`；合成影象是由有界本機字幕平面合成器生成的，而不是由 CSS 或 WebView 文字佈局生成的。具有有界佈局欄位的 TTML 間隔可以使用捆綁的 ARIB 字型的 Rounded M+ 1m 返回後端光柵化的 1920×1080 RGBA 平面。有效宣告的顯示範圍將源幾何圖形和畫素長度規範化到該邏輯平面上；缺失範圍預設為邏輯 2K，並且僅從至少一個軸上超過邏輯 2K 且適合該平面的完整畫素區域幾何形狀推斷規範的 4K/8K。等效的 2K/4K/8K 佈局保留相同的觀看者相對尺寸，而無需猜測不明確的來源。有界富體解析器保留 span/ruby 標籤外部的文字，並對映顯式的 span 顏色、大小、間距和不透明度。本機水平路徑保留顯式換行符並應用已解析的 `textAlign`、`displayAlign` 和 `lineHeight`。簡單的水平 `tts:ruby` 基本/文字對以 0.5 比例進行光柵化，並以其基本跨度為中心。明確關聯的垂直 ruby 同樣在其基本單元旁邊以 0.5 比例進行光柵化，包括髮生自動列換行時的有界延續；均報告 `captionPlaneMode=ttml-vertical-ruby-basic-native` 和 `renderedRubyCount`。此延續不實現一般的 B62 ruby 分組或特定於源的放置。僅當捆綁的 ARIB 字型包含對映的字形時，垂直渲染器才使用 Unicode 垂直表示標點符號；它從來不近似於拉丁旋轉或tate-chu-yoko。 Direct `tts:textOutline` 僅接受 `none`、TTML 命名顏色或完整 `#RRGGBB[AA]` 加上 `px` 寬度，然後應用有界的原生輪廓； `arib-tt:border`是故意不轉換的。完整的 B62 字形方向、標準 B62 筆劃行為、非 PNG 資源以及無法渲染/缺失的字形仍然存在明確的限制；不受支援的記錄仍然是結構預覽而不是捏造的影象。

TLV 歸檔匯出還可能包含有界 `asset_evidence` 和 `resource_evidence` 記錄。每個 `resource_evidence` 記錄都保留無損的 Base64 有效負載、格式驗證以及匹配的 `subt://` 引用所使用的確切 `packet_id + mpu_sequence_number + subsample_number` 記錄金鑰。存檔時預覽閱讀器最多保留 64 個此類記錄，僅將相同 MPU 匹配附加到活動字幕，並將經過驗證的小型 PNG `preview_data_uri` 公開為 `resourcePreviews`。字型資源、非 PNG 資源、缺失資源和不完整的地圖僅保留證據，不會宣告為渲染的標題文字。

單獨的有界 `asset_evidence` 記錄僅標識輸入中已觀察到的 MPT 信令（`packet_id`、源 TLV 偏移、`asset_type`、描述符標籤和通告的 MPU NTP 值）。它們是未來 `subt://` 資源加入的證據，而不是解碼的影象或字型位元組。 `resource_reference` 記錄攜帶原始 `packet_id + mpu_sequence_number` 範圍。數字 `subt://` 索引永遠不會被視為全域性 MPT 資料包 ID：如果存在有界的相同 MPU 子樣本，則關聯為 `same-mpu-evidence` 並指向其原始資源記錄；否則它仍顯式保留為 `unresolved`。 `dump-tlv` 另外還發出完整的有界非 `stpp` MPU/MFU 有效負載，作為具有確定性範圍金鑰的 `mmt_asset_payload` 原始證據。此類記錄可能包括 `format_hint`，但它只是有界二進位制簽名或有界標頭觀察（不是解碼或渲染宣告），而未知的資產語義仍未解決。 PNG 尺寸和字型表計數（如果存在）僅是結構後設資料。小型、結構完整的 PNG 資源還可能為未來的本機預覽表面攜帶有上限的 `data:` 預覽值；後端仍然不解碼或信任任意資源 URL。

該快照還帶有 `renderProfile`。它的合同故意與 libaribcaption 相容：使用捆綁的 `Rounded M+ 1m for ARIB` 系列，保留字元單元幾何形狀，將 ruby 保持在 0.5 相對比例，並從解碼的源字元資料中獲取背景 alpha 和描邊顏色。釋出的 libaribcaption 螢幕截圖是面向觀看者的視覺參考；其固定的本地基線和稽核規則位於`docs/visual-reference.md`中。該配置檔案的 B24 部分由解碼器支援。當前的本機 TTML 路徑使用捆綁字型、源前景/背景 RGBA、跨度樣式執行、簡單水平 ruby 和顯式關聯的垂直 ruby，包括跨自動列的有界延續。複雜的 ruby 分組、完整的垂直方向和標準筆劃行為在測試其本機實現之前仍然是宣告性後設資料； UI 不得使用任意 CSS 陰影或固定黑框來模仿它們。 `captionOverlayModes` 是一系列結構化後端路由功能：`id`、`available`、`experimental` 和 `unavailableReasonCode`。在 Windows 上，當發現的執行時匯出完整渲染 API 時，`libmpv-render` 變得可用；後端預設選擇它，如果渲染工作啟動失敗，則按源回退到 `libmpv-client-overlay`。 UI 呈現後端的實際路線，並且從不選擇渲染器本身。

## 工人活動信封

工作器 JSONL 事件使用 `protocolVersion`、`jobId`、`sequence` 和 `payload` 欄位。為了相容性，舊的頂級事件欄位在遷移期間仍然存在。 Tauri 層必須在將事件轉發到 Svelte 之前驗證版本和序列。

工作執行緒首先發出 `hello`，然後是有界 `stage-changed`、`track-discovered`、進度、`diagnostic`、`drcs-discovered`、暫停/恢復、取消、`artifact-created`、完成或 `failed` 事件（如果適用）。每個成功釋出的工件都會報告其穩定型別和完成前的最終路徑； Tauri 使用該事件來更新原子 `app-data/jobs/{job-id}/artifacts.json` 清單，而不是從 UI 選項推斷最終工件。檢查點永續性屬於 Tauri：只有在 `checkpoint.json` 原子釋出後，它才會轉發 `checkpoint-written`。 Tauri 在每次任務事件中轉發穩定的 `code` 和 `parameters` 形狀。時間線和診斷頁面流式傳輸其 JSONL 源並僅保留請求的視窗；桌面不會在記憶體中快取完整的存檔或診斷歷史記錄。實時時間視窗 API 保留一個有界的預取視窗，並將位元組游標移到新完成的 JSONL 行上，僅當請求的時間離開該視窗或工件被替換時才從磁碟重建。協議版本和序列違規保留其原始訊息作為證據，但也攜帶命名引數，例如 `expected`、`actual`、`previous` 和 `current`； Svelte 本地化程式碼而不解析該訊息。當工作人員提供的診斷引數是 JSON 物件時，將逐字保留。取消或失敗時，工件狀態將與 Worker 事件和檔案證據進行協調：`completed` 表示 Worker 釋出了它，`preserved` 表示預先存在的目標保持不變，`incomplete` 表示 `.part` 檔案保留。 `failed` 或 `cancelled` 表示不存在更強的偽影證據。應用程式啟動時，持久的活動狀態變為 `Interrupted`，持久的 `Queued` 任務變為 `Ready`；記憶體佇列永遠不會自行恢復。 `resume_job` 僅在驗證作業 ID、源、輸出、軌道、源大小、進度範圍和有界頭/尾源指紋後重播 `Interrupted`、`Failed` 或 `Cancelled` 作業。僅報告時間戳更改，但當大小和指紋仍然匹配時接受。本機解碼器和部分偽像狀態未序列化，因此恢復當前從可信記錄源執行完整重播，而不是宣告位元組精確恢復。

Worker 是獨立可執行的，必須在 UI 整合之前進行測試：

```text
arib-caption-worker inspect recording.ts
arib-caption-worker convert recording.ts output.ass --overwrite --drcs-report
arib-caption-worker convert recording.m2ts output.ttml --ttml --overwrite
arib-caption-worker dump-tlv recording.tlv output.caption.mmtp.jsonl --overwrite
arib-caption-worker render-at output.caption.archive.jsonl 90000
```

已知的限制是產品限制，而不是隱藏的後備方案：

- SRT 不是正式的無損目標。
- 未識別的 TLV/MMTP 資產將作為原始證據保留，不會被猜測。
- `inspect_source` 返回穩定的 `routeCode`：`mpeg_ts_b24_verified` 立即由 B24 元件描述符進行驗證。 `mpeg_ts_ttml_candidate` 表示在 188 位元組 TS 或 192 位元組 M2TS 中找到私有 PES PID，並且在轉換期間仍然需要嚴格的 ARIB-TTML XML 驗證。 `mpeg_ts_192_ttml_verified` 命名釋出門控、成功驗證的 192 位元組 M2TS/TTML 轉換路線；有界初始檢查在看到有效的 TTML 檔案之前不得宣告它。 `tlv_mmtp_experimental` 有意以證據為先，在沒有真實語料庫的情況下不得將其呈現為一般 BS4K/8K 支援。
- 檢查點當前從可信記錄源執行源身份驗證的完整重播，因為本機 B24 和部分工件狀態不可序列化。
- 當前的 Windows 影片表面由程序內 `libmpv` 擁有；不使用 `mpv.exe` sidecar 或 JSON 命名管道。在執行時匯出完整渲染 API 的情況下，後端選擇 `libmpv-render`，擁有 WGL 上下文和 BGRA 紋理混合路徑，並且僅在特定啟動失敗時才回退到客戶端覆蓋。它請求 `hwdec=auto-safe`，允許相容的回拷加速，但不承諾零複製 D3D/ANGLE 互操作性。當載入的源報告時，`get_preview_render_diagnostics` 返回選定的路線、實時表面尺寸、每秒呈現數、紋理操作計數、方面、請求的解碼器策略以及 libmpv 的實際 `hwdec-current`。長 2K/4K/8K 分析仍然是釋出質量的門控，而不是隱含的功能。
- `get_preview_capabilities` 將每條路由報告為 `{ id, available, experimental, unavailableReasonCode }`。它只是一個演示契約：WebView 無法提交標題點陣圖。 `render_preview_at`和`sync_preview_overlay`在後端組成有界的原生字幕平面，然後將其應用到libmpv。非 Windows 構建報告 `preview.platform_not_implemented`；它們並不意味著本機預覽路線。
- `sync_preview_overlay` 報告 `mediaTimeMs` 和 `projectTimeMs`。它使用 `projectTimeMs` 查詢字幕；預設對映是身份，但 PTS 修復、程式邊界和使用者偏移必須更新後端對映，而不是教導 WebView 第二個時鐘。
- `trackId` 作為所有發現的 MPEG-TS B24 或 M2TS 資料軌道的經過驗證的 PID 選擇器傳遞。對於B24，選定的PID解析為邏輯`service_id + component_tag`磁軌；順序解碼遵循當前 PAT/PMT 更新，並且可以在同一邏輯軌道的替換 PID 上繼續。檢查報告代表性 `caption_pid`、每個有界發現 `caption_pids`、元件標籤、PAT/PMT 服務 ID、SDT 服務名稱和 ISO-639 字幕語言。其 `broadcast` 物件還報告可選的 NIT 網路名稱、當前服務 EIT 當前事件名稱和描述以及 TDT/TOT UTC 廣播時間。此 SI 通行證是基於內容的，使用單資料包工作緩衝區最多傳輸 64 MiB，並且當所選服務沒有 EIT 時，絕不會替代另一服務的節目。缺少欄位意味著記錄沒有提供有界視窗中的資訊；它們不是解析器的猜測。廣泛的 EPG 歷史記錄、CAS 和記錄器後設資料仍然不包含在產品合同中。佇列管理器擁有暫停狀態並向其活動 Worker 傳送協作暫停/恢復控制；空閒暫停仍然會阻止下一個排隊作業的啟動。

專用 PES 軌跡發現報告 `pids`、`caption_pids` 和 `superimpose_pids`。元件標籤 `0x30..0x37` 和 `0x38..0x3f` 對兩種服務進行分類，但它們本身並不證明 B24 或 TTML：B24 仍然需要其資料元件描述符，而 TTML 仍然需要完整的、嚴格解碼的 XML 文件。在沒有顯式 `trackId` 的情況下，轉換和預覽會選擇宣告的字幕元件並保持疊加元件獨立。如果 PMT 描述符沒有對私有流進行分類，則它仍然是候選流，而不是從其 PID 中猜測。

符合名稱空間的 TTML 透過 XML 本地名稱和祖先進行解析，包括預設或帶字首的 TTML 名稱空間。連續的 ARIB-TTML 文件可能會省略 `begin`、`end` 和 `dur`；同一 PID 上的下一個完整文件關閉前一個文件，空的 `<tt>` 是清除操作。當 PES PTS 標記/字首驗證失敗時，192 位元組 M2TS 路由透過迴繞處理從 30 位到達時間戳匯出此文件時鐘。它永遠不會僅僅因為設定了 `PTS_DTS_flags` 就接受零填充的私有 PES 欄位，並且它永遠不會從到達另一個 PID 的文件中關閉一個 PID。

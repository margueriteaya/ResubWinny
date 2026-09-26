[簡體中文](backend-contract.md) | [English](backend-contract.en.md) | [日本語](backend-contract.ja.md) | [繁體中文](backend-contract.zh-TW.md)

> **規範性說明：** 簡體中文版本是唯一權威來源。其他語言版本是同步譯文；如措辭存在歧義或衝突，以簡體中文版本為準。

# 後端合約

> 2026-09-02 實作說明：本文的邏輯 1920×1080 平面只是有界中間紋理，不是定義正確性的目標解析度。
> Worker 在可選 `source_layout` 中保留來源平面、region、樣式和行內長度；原生算繪器由此明確計算
> 中間紋理，再將整張紋理映射至 libmpv 的視訊內容 viewport。正確性以排除黑邊後相對視訊內容的比例為準。
Tauri/Svelte UI 是 Rust 後端的客戶端。它不解析 TS/TLV 資料、解碼 ARIB、渲染高解析度影片或決定轉換語義。

持久化使用的 `.caption.jsonl` 格式在 [`contracts/archive.md`](contracts/archive.md) 中單獨指定，包括其明確的 schema 版本和流式讀取器相容性規則。

合約細節依主題分為以下指南：[`contracts/tauri-api.md`](contracts/tauri-api.md)、[`contracts/worker-protocol.md`](contracts/worker-protocol.md)、[`contracts/preview.md`](contracts/preview.md) 和 [`contracts/timeline.md`](contracts/timeline.md)。本文繼續作為相容性索引與詳細參考。

後端介面遵循範圍明確、穩定的應用合約。在目前的收斂階段，優先整合相關查詢，減少僅用於單一情境的命令變體：

| 命令 | 責任 |
| --- | --- |
| `inspect_source` | 在限定範圍內檢查錄影並探索字幕軌道 |
| `start_export` | 啟動串流 Worker 並發出 `task-event` 進度；接受可選的經過驗證的 `trackId` |
| `cancel_export` | 停止當前 Worker 程序 |
| `pause_export` / `resume_export` | 向 Worker 傳送協作控制訊息 |
| `create_job` / `list_jobs` / `get_job` / `remove_job` | 在沒有媒體負載的情況下保留任務摘要 |
| `start_job` / `pause_job` / `resume_job` / `cancel_job` | 透過 Worker Supervisor 控制持久化作業 |
| `get_job_diagnostics` | 返回為持久作業收集的有界結構化診斷資訊 |
| `get_job_diagnostics_window` | 使用偏移/限制返回有界診斷頁 |
| `list_jobs_window` | 返回最近任務摘要的有界頁面 |
| `get_job_artifacts` | 返回任務工件清單和 `.part` 路徑 |
| `get_job_checkpoint` | 返回任務的最新有界進度檢查點 |
| `pause_queue` / `resume_queue` / `queue_is_paused` | 控制 Supervisor 佇列並協作暫停/恢復其活動 Worker |
| `load_drcs_report` | 讀取 Worker 生成的 DRCS 報告並返回可顯示的字形影象 |
| `get_settings` / `update_settings` | 讀取或以不可分割的操作更新應用程式資料 `settings.json` 中經過驗證的介面設定與匯出預設值 |
| `list_language_packs` | 從固定的 app-data `language-packs/` 目錄中重新掃描有界的 JSON 語言檔案；不接受任意瀏覽器提供的目錄 |
| `open_language_pack_directory` | 在需要時建立該固定目錄並使用平臺檔案管理器開啟它 |
| `start_preview` / `resize_preview` / `stop_preview` | 控制當前程序內 libmpv 影片表面 |
| `preview_command` | 將跳轉/暫停命令轉發到 libmpv |
| `get_preview_capabilities` | 報告宣告的影片/字幕合成路徑以及僅當前可用的路線 |
| `get_preview_runtime` | 報告發現的 libmpv 執行時以及渲染 API 符號可用性，而不宣告渲染表面存在 |
| `get_preview_render_diagnostics` | 報告活動的原生路徑和有界渲染執行緒計數器/錯誤；缺少 Worker 時返回穩定的非活動結果 |
| `render_at` | 返回請求的存檔時間的有界字幕平面快照，而不透過 WebView 傳送影片幀 |
| `sync_preview_overlay` | 讀取嵌入的 libmpv 時間，渲染有界本機平面，並套用、清除 Windows 覆蓋層或略過重複更新，無需 WebView 計時或佈局 |
| `get_playback_time_mapping` / `update_playback_time_mapping` | 獲取或替換本機字幕預覽使用的經過驗證的媒體時間→專案時間段對映 |
| `get_timeline_window` / `get_timeline_window_filtered` | 流式傳輸有界存檔頁面以供完成的任務瀏覽 |
| `get_timeline_recent_window_filtered` | 從檔案尾端增量讀取完整的 JSONL 記錄並僅返回最新的有界實時事件頁面 |
| `get_timeline_time_window` | 返回編輯器時間線的有界預取時間範圍並增量讀取附加記錄 |

`render_at` 在檔案匯出完成後於任務工作區中暴露。UI 讓時間查詢保持顯式且有界，並在檔案含有 B24 渲染幀時顯示真正由 RGBA 派生出的 PNG。後端返回 `planeWidth`、`planeHeight`、`composedPngBase64` 與 `activeLayerCount`；合成影像由有界的本機字幕平面合成器產出，而不是由 CSS 或 WebView 文字佈局產出。帶有有界佈局欄位的 TTML 區間，可以使用捆綁的 Rounded M+ 1m for ARIB 字型，返回一個由後端點陣化的 1920×1080 RGBA 平面。有效的已宣告顯示 extent 會把源幾何與畫素長度歸一化到該邏輯平面上；缺少 extent 時預設按邏輯 2K 處理，且只在完整畫素 region 幾何至少在一個軸上超過邏輯 2K 並且能容納於該平面時，才推斷出規範的 4K/8K。等效的 2K/4K/8K 佈局保持相同的觀看者相對尺寸，而不去猜測含義不明的來源。有界的富文字正文解析器保留 span/ruby 標籤之外的文字，並對映顯式的 span 顏色、字號、字距與不透明度。本機水平路徑保留顯式換行，並應用解析後的 `textAlign`、`displayAlign` 與 `lineHeight`。簡單的水平 `tts:ruby` base/text 對以 0.5 比例點陣化，並在其 base span 上居中。顯式關聯的垂直 ruby 同樣以 0.5 比例點陣化在其 base 字元單元旁邊，在發生自動分欄換行時帶有有界延續；兩者都報告 `captionPlaneMode=ttml-vertical-ruby-basic-native` 與 `renderedRubyCount`。該延續並不實現通用的 B62 ruby 分組或特定來源的擺放規則。只有當捆綁的 ARIB 字型包含所對映的字形時，垂直渲染器才使用 Unicode 豎排呈現標點；它絕不近似拉丁字元旋轉或縱中橫。直接的 `tts:textOutline` 只接受 `none`、TTML 具名顏色，或完整的 `#RRGGBB[AA]` 加 `px` 寬度，然後應用有界的本機描邊；`arib-tt:border` 是刻意不做轉換的。完整的 B62 字形方向、標準 B62 描邊行為、非 PNG 資源，以及無法渲染或缺失的字形，仍是明確的限制；不受支援的記錄仍以結構化預覽呈現，而不是偽造出來的影像。

TLV 歸檔匯出還可能包含有界 `asset_evidence` 和 `resource_evidence` 記錄。每個 `resource_evidence` 記錄都保留無損的 Base64 有效負載、格式驗證以及匹配的 `subt://` 引用所使用的確切 `packet_id + mpu_sequence_number + subsample_number` 記錄鍵。封存預覽讀取器最多保留 64 個此類記錄，僅將同一 MPU 內相符的資源附加到活動字幕，並將經過驗證的小型 PNG `preview_data_uri` 公開為 `resourcePreviews`。字型資源、非 PNG 資源、缺失資源和不完整的資源對映僅保留證據，不會宣告為已算繪的字幕文字。

另有一類有界的 `asset_evidence` 記錄，只標識輸入中已經觀察到的 MPT 信令（`packet_id`、源 TLV 偏移、`asset_type`、描述符標籤，以及所通告的 MPU NTP 值）。它們是將來接入 `subt://` 資源的證據，而不是已解碼的影像或字型位元組。`resource_reference` 記錄攜帶其來源的 `packet_id + mpu_sequence_number` 作用域。數字形式的 `subt://` 索引絕不被當作全域 MPT 包 ID：若存在有界的同一 MPU 子樣本，則關聯標記為 `same-mpu-evidence` 並指向其原始資源記錄；否則顯式保持為 `unresolved`。`dump-tlv` 還會把完整而有界的非 `stpp` MPU/MFU 負載，以帶確定性作用域鍵的 `mmt_asset_payload` 原始證據形式輸出。此類記錄可能帶有 `format_hint`，但它只是有界的二進位制簽名觀察或有界的頭部觀察（不是解碼或渲染宣告），未知的資產語義仍然保持未解析。PNG 尺寸與字型表數量若存在，也僅是結構性元資料。結構完整的小體積 PNG 資源，還可能攜帶一個有上限的 `data:` 預覽值，供將來的本機預覽表面使用；後端仍然不會解碼或信任任意資源 URL。

該快照還帶有 `renderProfile`。它的合約刻意與 libaribcaption 保持相容：使用捆綁的 `Rounded M+ 1m for ARIB` 字族，保留字元單元幾何，把 ruby 維持在 0.5 的相對比例，並從解碼得到的源字元資料中取用背景 alpha 與描邊顏色。已釋出的 libaribcaption 截圖是面向觀看者的視覺參考；其固定的本地基線與審查規則見 `docs/visual-reference.md`。該 profile 的 B24 部分由解碼器支撐。當前的本機 TTML 路徑使用捆綁字型、源前景/背景 RGBA、span 樣式區段、簡單水平 ruby，以及顯式關聯的垂直 ruby，其中包含跨自動分欄的有界延續。複雜的 ruby 分組、完整的垂直排版方向與標準描邊行為，在其本機實現透過測試之前仍只是宣告性元資料；UI 不得用任意 CSS 陰影或固定黑框去模仿它們。`captionOverlayModes` 是一組結構化的後端路徑能力：`id`、`available`、`experimental` 與 `unavailableReasonCode`。在 Windows 上，當發現的執行時匯出完整渲染 API 時，`libmpv-render` 即變為可用；後端預設選擇它，若渲染 Worker 啟動失敗，則按來源回落到 `libmpv-client-overlay`。UI 呈現後端實際採用的路徑，絕不自行選擇渲染器。

## Worker 事件信封

Worker 的 JSONL 事件使用 `protocolVersion`、`jobId`、`sequence` 與 `payload` 欄位。為保持相容，遷移期間舊的頂層事件欄位依然保留。Tauri 層必須先驗證版本與序號，才能把事件轉發給 Svelte。

Worker 首先發出 `hello`，隨後按適用情況發出有界的 `stage-changed`、`track-discovered`、進度、`diagnostic`、`drcs-discovered`、暫停/恢復、取消、`artifact-created`、完成或 `failed` 事件。每個成功釋出的產物都會在完成之前報告其穩定型別與最終路徑；Tauri 依據該事件更新原子的 `app-data/jobs/{job-id}/artifacts.json` 清單，而不是從 UI 選項推斷最終產物。檢查點持久化歸 Tauri 負責：只有在 `checkpoint.json` 原子釋出之後，它才轉發 `checkpoint-written`。Tauri 在每個任務事件上都轉發穩定的 `code` 與 `parameters` 結構。時間軸與診斷頁面以流式讀取其 JSONL 來源，只保留所請求的視窗；桌面端不會把完整的檔案或診斷歷史快取在記憶體中。實時時間視窗 API 只保留一個有界的預取視窗，並在新寫完的 JSONL 行上推進位元組遊標，僅當請求時間離開該視窗或產物被替換時才從磁碟重建。協議版本與序號違規會保留其原始訊息作為證據，同時攜帶 `expected`、`actual`、`previous`、`current` 等具名引數；Svelte 本地化的是錯誤碼，而不解析該訊息。Worker 提供的診斷引數若為 JSON 物件，則逐字保留。取消或失敗時，產物狀態由 Worker 事件與檔案證據共同核對得出：`completed` 表示 Worker 已釋出，`preserved` 表示既有目標檔案未被觸動，`incomplete` 表示仍留有 `.part` 檔案。`failed` 或 `cancelled` 表示不存在更強的產物證據。應用啟動時，持久化的活動狀態變為 `Interrupted`，持久化的 `Queued` 任務變為 `Ready`；記憶體佇列絕不自行恢復。`resume_job` 只在驗證作業 ID、來源、輸出、軌道、來源大小、進度上界以及有界的首尾來源指紋之後，才重放 `Interrupted`、`Failed` 或 `Cancelled` 作業。僅時間戳發生變化會被報告，但在大小與指紋仍然一致時予以接受。本機解碼器與部分產物狀態不做序列化，因此當前的恢復是從可信的錄製來源完整重放，而不是聲稱可做位元組級精確續傳。

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
- `inspect_source` 返回穩定的 `routeCode`：`mpeg_ts_b24_verified` 由 B24 元件描述符立即驗證。`mpeg_ts_ttml_candidate` 表示在 188 位元組 TS 或 192 位元組 M2TS 中找到了私有 PES PID，並且在轉換期間仍需透過嚴格的 ARIB-TTML XML 校驗。`mpeg_ts_192_ttml_verified` 指的是受釋出門禁約束、已成功校驗的 192 位元組 M2TS/TTML 轉換路徑；有界的初次檢查在尚未見到有效 TTML 文件之前不得宣告該路徑。`tlv_mmtp_experimental` 有意以證據優先，在沒有真實語料之前，不得把它呈現為通用的 BS4K/8K 支援。
- 目前從檢查點恢復時，會先驗證來源身分，再從可信錄影來源完整重放處理流程，因為原生 B24 解碼器與部分產物狀態無法序列化。
- 當前的 Windows 影片表面由程序內 `libmpv` 擁有；不使用 `mpv.exe` sidecar 或 JSON 命名管道。在執行時匯出完整渲染 API 的情況下，後端選擇 `libmpv-render`，擁有 WGL 上下文和 BGRA 紋理混合路徑，並且僅在特定啟動失敗時才回退到客戶端覆蓋。它請求 `hwdec=auto-safe`，允許相容的回拷加速，但不承諾零複製 D3D/ANGLE 互操作性。`get_preview_render_diagnostics` 回傳所選路徑、目前表面尺寸、每秒呈現影格數、紋理操作計數、長寬比與要求的解碼器策略；若已載入的來源提供相關資訊，也會回傳 libmpv 實際使用的 `hwdec-current`。長時間 2K/4K/8K 效能測試仍屬於發布品質門檻。
- `get_preview_capabilities` 把每條路徑報告為 `{ id, available, experimental, unavailableReasonCode }`。它只是一份呈現層合約：WebView 無法提交字幕點陣圖。`render_preview_at` 與 `sync_preview_overlay` 在後端內部合成有界的本機字幕平面，再把它應用到 libmpv 上。非 Windows 構建報告 `preview.platform_not_implemented`；這並不意味著存在本機預覽路徑。
- `sync_preview_overlay` 報告 `mediaTimeMs` 和 `projectTimeMs`。它使用 `projectTimeMs` 查詢字幕；預設採用恆等對映，即媒體時間與專案時間一致。PTS 修復、節目邊界與使用者偏移必須透過更新後端對映處理；WebView 不維護第二套時鐘。
- `trackId` 作為所有發現的 MPEG-TS B24 或 M2TS 資料軌道的經過驗證的 PID 選擇器傳遞。對於 B24，選定的 PID 對應邏輯 `service_id + component_tag` 軌道；順序解碼遵循當前 PAT/PMT 更新，並且可以在同一邏輯軌道的替換 PID 上繼續。檢查報告代表性 `caption_pid`、檢查範圍內探索到的全部 `caption_pids`、元件標籤、PAT/PMT 服務 ID、SDT 服務名稱和 ISO-639 字幕語言。其 `broadcast` 物件還報告可選的 NIT 網路名稱、當前服務 EIT 當前事件名稱和描述以及 TDT/TOT UTC 廣播時間。這輪 SI 檢查以內容為依據，使用單一封包大小的工作緩衝區，最多讀取 64 MiB，並且當所選服務沒有 EIT 時，絕不會用另一服務的節目補齊。缺少欄位意味著錄影在限定檢查範圍內未提供相關資訊；它們不是解析器的猜測。廣泛的 EPG 歷史記錄、CAS 和錄影機後設資料仍然不包含在產品合約中。佇列管理器擁有暫停狀態並向其活動 Worker 傳送協作暫停/恢復控制；空閒暫停仍然會阻止下一個排隊作業的啟動。

私有 PES 軌道探索報告 `pids`、`caption_pids` 和 `superimpose_pids`。元件標籤 `0x30..0x37` 和 `0x38..0x3f` 對兩種服務進行分類，但它們本身並不證明 B24 或 TTML：B24 仍然需要其資料元件描述符，而 TTML 仍然需要完整的、嚴格解碼的 XML 文件。在沒有顯式 `trackId` 的情況下，轉換和預覽會選擇宣告的字幕元件並保持文字疊加元件獨立。如果 PMT 描述符沒有對私有流進行分類，則它仍然是候選流，而不是從其 PID 中猜測。

符合名稱空間的 TTML 透過 XML 區域名稱與祖先元素進行解析，包括預設或帶字首的 TTML 名稱空間。連續的 ARIB-TTML 文件可能會省略 `begin`、`end` 和 `dur`；同一 PID 上的下一個完整文件關閉前一個文件，空的 `<tt>` 是清除操作。當 PES PTS 標記/字首驗證失敗時，192 位元組 M2TS 路徑會處理時間戳迴繞，並從 30 位元到達時間戳推導文件時鐘。它永遠不會僅僅因為設定了 `PTS_DTS_flags` 就接受零填充的私有 PES 欄位，並且它不會用另一個 PID 上到達的文件結束目前 PID 的字幕文件。

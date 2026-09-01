# 架構基線（中文規範）

[簡體中文（唯一權威）](architecture.zh-CN.md) | [English](architecture.en.md) | [日本語](architecture.ja.md) | [繁體中文（臺灣）](architecture.zh-TW.md)

> 當前桌面實現為 Tauri 2 + Svelte 5；文中歷史版本記錄中的 Slint 僅作歷史說明，不代表當前架構。第三階段核心已經落地：B62/受限 TLV 資源證據、同 MPU PNG 資源到 archive/低頻預覽的接線、職責拆分、archive 時間點預覽、B24 原生字幕平面合成、TTML 橫排與豎排文字光柵化、連續 base ruby 分組、跨列豎排 ruby、保守的 CJK/全形正立與拉丁字元旋轉、TS/PSI/PES/B24 fuzz target、Windows `libmpv-render`、真實 4K 錄製樣本的閾值化長時效能門檻以及 Windows/macOS/Linux CI 檢查矩陣已完成。標準 B62 描邊、資源完整預覽、獨立 2K/8K 與 DPI/截圖差分仍屬於質量收斂項。Windows 是當前 Alpha 的原生預覽釋出平臺；macOS/Linux 原生預覽後端已明確延期，不屬於當前階段驗收範圍。WGL 零複製硬解互操作不是當前產品承諾。

> 本頁是同步譯文。簡體中文版本是專案唯一具權威性的規範架構檔案；任何歧義或衝突一律以簡體中文版本為準。

## 收斂期邊界（2026-08-29）

當前階段凍結前端技術棧與 Rust crate 拆分：Svelte、Tauri 與現有
`arib-caption-worker` 保持不變。前端優先透過 feature session 收攏中央狀態；
Worker 繼續負責輸入、探測/解複用、解碼、Caption IR、匯出、archive 與證據，
Tauri 繼續負責任務歷史、佇列、設定、視窗生命週期和原生預覽。只有在
Caption IR、時間模型或 transport API 穩定且出現多個消費者後，才重新評估
拆分 `resubwinny-core`。

同一收斂期明確延期 BD/DVD 圖形字幕 OCR、外掛系統、AI 翻譯以及
macOS/Linux 原生預覽。DRCS 只繼續完善本地 hash → Unicode 對映，
不擴建通用 OCR 系統。

## 1. 專案定位與邊界

本專案是面向日本 ISDB 廣播錄製檔案的開源、跨平臺字幕抽出與轉換工具。傳輸層主線必須區分傳統 MPEG-2 TS 與 BS4K/8K 原生 TLV/MMT；`.ts`、`.m2ts`、`.tlv`、`.mmts` 只作為檔名提示，最終一律按內容探測。當前可驗證語料包括傳統 TS 與 192-byte MPEG-TS/TTML 錄製；原生 TLV/MMT 保留為規範主線和實驗性輸入，直到獲得足夠真實樣本後再宣稱完整支援。所有路徑都儘可能保留 ARIB 字幕的語義、版式、特殊字元和診斷來源。

它不是錄影管理器、媒體播放器、影片/音訊解碼器、EPG 瀏覽器、CAS/加擾處理器、通用 MMT 媒體框架或網路直播接收器。重點始終是字幕抽出、恢復、轉換、存檔與診斷。

舊工具、`bs4kass.exe` 與 Caption2Ass 只可用於公開資料研究和黑箱比較；不得複製其非公開實現，也不得將它們打包到釋出產物中。

## 2. 已確認的架構

當前 worker 的 `main.rs` 僅呼叫 `lib.rs` 暴露的 `run()`；所有模組註冊、共享匯出和測試入口均位於 `lib.rs`，因此 CLI 入口與轉換核心已經可以獨立複用。

```text
Tauri 2 + Svelte 5 桌面介面（WebView 只負責展示）
  | 後臺任務、低頻進度、取消與診斷
  v
共享 Rust 轉換核心（GUI/CLI 同一實現）
  | 有界順序讀取、解析、時間軸、原子提交
  v
專案字幕模型與匯出器
  | 薄且穩定的 C ABI
  v
libaribcaption（第一代 ARIB STD-B24 解碼/可選渲染後端）
```

Worker 已按職責拆分為 `cli.rs`、`protocol.rs`、`inspection.rs`、`jobs.rs`、`preview.rs`、`archive.rs`、`resource.rs`、`config.rs`、`transport/mpeg_ts.rs`、`transport/m2ts.rs`、`transport/tlv_mmt.rs`、`caption/b24.rs`、`caption/ttml.rs`、`timeline.rs`、`drcs.rs` 和 `exporters/`；`main.rs` 只保留程式入口、模組註冊和測試入口，解析實現與配置常量不再堆疊於此。M2TS 的 192-byte packetisation、track discovery 和 route façade 已歸入 `transport/m2ts.rs`，TTML 檔案語義仍由 `caption/ttml.rs` 負責。`archive.rs` 提供 CLI 與桌面後端共用的有界 archive 時間點快照路徑，`resource.rs` 負責 B62 資源證據，`transport/tlv_mmt.rs` 負責 TLV/MMTP 基礎包與受限 stpp 路由。通用 TLV/MMT 字幕語義仍明確標記為未完成。

GUI 絕不能成為唯一入口，也不得在 UI 執行緒讀取錄製位元組、接收每個 TS 包、儲存完整字幕時間線、承擔解複用或最終排版。轉換核心必須可被單獨以 CLI 呼叫。當前介面在後臺執行緒執行同一核心，提供協作式取消、進度與原子輸出；需要跨程式崩潰隔離時再增加 sidecar，而不為此犧牲單一 EXE 釋出。

本地大檔案預設使用 Rust `File`、`BufReader`、`Read`、`Seek` 等阻塞順序 I/O；不要為每個 188 位元組 TS 包建立非同步任務或跨 channel 傳遞。Tokio 僅在 IPC、排程、網路輸入或並行獨立任務確有需要時引入。

## 3. 大檔案與恢復約束

- 檔案大小不得決定常態記憶體佔用：1 GB 與 200 GB 輸入應保持近似的資源規模。
- 預設不得整體讀入、整體解複用、快取全部 TS 包、累計全部字幕事件、建立細粒度全量索引，也不得由前端處理廣播檔案。
- 輸入緩衝、重同步視窗、每 PID 的 PES 緩衝、每 asset 的 MPU 緩衝和活動字幕場景均須有硬上限；不可信長度欄位不得直接導致任意分配。
- 目標路徑是：固定大小緩衝 -> 容器同步/探測 -> TS/TLV/MMTP 流式解析 -> 僅保留目標服務與字幕 PID/asset -> data group/MPU 重組 -> 解碼 -> 場景變化 -> 增量匯出。
- 首次掃描只識別容器、服務、軌道和必要時間基準；影片、音訊不解碼也不需完整 PES 重組。
- 優先借用輸入切片。僅在跨包 PES、data group、MMTP fragment 或需長期持有時複製；DRCS 按雜湊去重。不要為了消滅最後幾 KB 複製而寫複雜生命週期結構。
- 預設不用全檔案 mmap；可在未來作為平臺特定最佳化。讀取器應可擴充套件支援本地檔案、stdin/管道、分卷檔案與增長中的錄製檔案。
- 週期性 checkpoint 至少包含檔案身份（大小、mtime、首尾塊雜湊）、byte offset、continuity、PTS unwrap、當前 B24 management/DRCS 狀態和匯出安全位置。恢復優先回退到可靠同步點並重解析一小段，而不是假定可在任意位元組恢復解碼器狀態。
- 輸出先寫 `.part`、臨時 events、DRCS 目錄和 checkpoint；成功後原子釋出。失敗或取消時保留日誌、恢復資訊和明確的未完成標記。阻止睡眠為手動、預設關閉、僅在任務執行時生效。

## 4. 輸入路由

傳統地上波/BS/CS 的 MPEG-2 TS，以及被重新封裝或以 192-byte 形式儲存的 MPEG-TS 錄製：

```text
MPEG-2 TS -> PAT/PMT -> subtitle PES -> ARIB STD-B24 data groups
```

必須儲存服務、PID、語言、caption/superimpose 型別、PCR/PTS/DTS、原始檔偏移、不連續和解碼警告。192-byte packetisation 只說明錄製封裝形態，不代表 BS4K/8K 原生傳輸層；真實 route 仍由內容探測和 PSI/MMT 信令決定。

對 MPEG-TS，B24 caption PID 仍是優先的已驗證路線。若 PSI/PMT 僅發現 private data PID，worker 可用同一有界 PES 重組器尋找完整 ARIB-TTML XML；只有 XML 邊界、BOM/宣告編碼與 TTML 檔案均透過嚴格驗證時才進入 TTML 模型。private PID 本身不是字幕證明，未識別或不完整 payload 必須保留為原始證據或診斷，不能猜測轉換。

### 4.0 現實輸入優先順序

當前可執行計劃按證據強度排序：

1. **已驗證主線：** 188-byte MPEG-TS + ARIB STD-B24，以及 192-byte MPEG-TS packetisation + 私有 PES + ARIB-TTML。兩者均有本地長樣本、流式計數基線和匯出迴歸。
2. **規範主線、實驗性實現：** 原生 BS4K/8K `TLV -> IP/UDP -> MMTP -> MPT/MPU`。當前只有構造/單元證據和受限 `stpp` 路由；真實 TLV 樣本不足，只能提供探測、診斷、原始證據和明確條件下的轉換。
3. **禁止的判斷：** 不能從 `.ts`、`.m2ts` 或 `.tlv` 副檔名推斷傳輸格式，也不能把 192-byte MPEG-TS 檔案自動稱為原生 BS4K/8K TLV。

BS4K/8K：

```text
TLV -> IPv6/壓縮 IP -> UDP -> MMTP -> signalling -> caption asset -> MPU
```

首期 BS4K/8K 僅處理錄製檔案：找到 MMT package、識別字幕 asset、重組字幕相關 MPU、恢復時間戳、將有效載荷交給統一字幕核心。不得在此階段實現 HEVC/音訊解碼、完整 SI/EPG、CAS、直播或通用 MMT 框架。該模組按協議實現專案估算，不能視為“增加一個副檔名”。

輸入探測層必須區分 MPEG-2 TS、TLV、MMTP、損壞/截斷流；不得用副檔名替代檢測。

### 4.1 訊號與 ARIB 規範對照

下表是實現時引用的規範層次，不是“只要副檔名相同便可按同一路徑處理”的規則。版本號記錄的是 2026-07 查到的 ARIB 最新公開目錄；實際解析以錄製流內的信令、描述符和有效載荷為準。

| 訊號類別 | 物理/傳輸體系 | 服務與軌道發現 | 字幕編碼與呈現 | 本專案的解複用入口 |
| --- | --- | --- | --- | --- |
| 地上波 2K（ISDB-T） | ARIB STD-B31，地上數字電視傳送方式；錄製層通常為 MPEG-2 TS | MPEG-2 PSI 與 ARIB STD-B10 的 SI | ARIB STD-B24 的字幕/文字スーパー資料；B24 data group 由字幕 PES 送達 | PAT/PMT -> 目標 subtitle PES -> B24 data group |
| BS/廣帶 CS 2K | ARIB STD-B20，衛星數字放送傳送方式；錄製層通常為 MPEG-2 TS | MPEG-2 PSI 與 ARIB STD-B10 的 SI | ARIB STD-B24；同樣不能把 `stream_type` 或 component tag 的單一經驗規則當成完整規範 | PAT/PMT -> 目標 subtitle PES -> B24 data group |
| BS4K/8K（高度廣帶衛星數字放送/ISDB-S3） | ARIB STD-B44 定義 ISDB-S3 傳送方式，含 TLV；媒體傳送由 ARIB STD-B60 的 MMT 體系規定 | MMT signalling、package/asset 與描述符 | ARIB STD-B62 第一編第三部規定第二代字幕/文字スーパー編碼，包含 ARIB-TTML 體系 | TLV -> IP/UDP -> MMTP -> signalling -> caption asset/MPU -> 由描述符識別的字幕格式 |

關鍵修正：BS4K/8K 不能僅憑“4K/8K”就假定有效載荷必然為 ARIB-TTML。ARIB STD-B60 的後續說明明確字幕資料格式由 caption-description method 標識；實現必須讀取實際 signalling/descriptor，並把 ARIB-TTML、可能的 B24 相容/其他標識和未知格式分別路由、報告或保留原始資料。`*.m2ts` 的 192-byte 包封裝也只是錄製器常見檔案表示，不能替代對 TS/TLV/MMT 內容的判斷。

ARIB STD-B24 是傳統數字放送的資料編碼與傳送規範；ARIB STD-B10 是補充 MPEG-2 PSI 的服務資訊規範，不是字幕字形/排版規範。ARIB STD-B62 面向高度廣帶衛星數字放送，其第一編第三部負責字幕與文字スーパー編碼；ARIB STD-B60 則規定 MMT 媒體傳送。物理層/傳送層、服務信令層與字幕編碼層必須分開實現和測試。

規範入口（只記錄編號、範圍與連結，不轉載受版權保護的標準正文）：

- [ARIB STD-B31](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b31.html)：地上數字電視傳送；
- [ARIB STD-B20](https://www.arib.or.jp/english/std_tr/broadcasting/std-b20.html)：衛星數字放送傳送，覆蓋 BS 數字與廣帶 CS 數字；
- [ARIB STD-B10](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b10.html)：數字放送服務資訊；
- [ARIB STD-B24](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b24.html)：數字放送資料編碼與傳送；
- [ARIB STD-B44](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b44.html)：ISDB-S3/高度廣帶衛星數字放送傳送；
- [ARIB STD-B60](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b60.html)：MMT 媒體傳送；
- [ARIB STD-B62](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b62.html)：第二代多媒體編碼，第一編第三部為字幕/文字スーパー編碼。

## 5. 字幕真相模型

內部真相不是 ASS，也不是“開始、結束、一段文字”的單 cue 列表。ARIB 字幕是對字幕平面與獨立區域的時間操作。模型分兩層：

```text
TimedCaptionOperation { pts, operation }
  ClearScreen | ClearRegion | SetCursor | SetStyle | WriteText |
  WriteDrcs | BeginRuby | EndRuby | DefineDrcs | ...

CaptionPlaneState -> closed RegionInterval / CaptionScene
```

一個 `RegionInterval` 必須有確定的 begin/end、layer、幾何和樣式化內容。多個區域可並行出現、分別更新和消失；不得把不同生命週期粗暴合併成一條字幕。

模型至少儲存：原始與展開後的 PTS/DTS/PCR、normalized time、source packet offset、management data、language tag、TCS、clear-screen、repeat/roll-up、平面尺寸、區域與字元級樣式、橫豎排、ruby、enclosure、DRCS、無法表達的控制碼、原始 payload（存檔要求時）及全部警告。

時間軸不得只信 PTS。需要處理開頭裁切、PCR 跳變、discontinuity、wrap-around、丟包、清屏丟失、多服務混錄、節目切換後的 PTS 重置和無顯式結束時間。提供嚴格 PTS、自動修復、影片/字幕零點、手動全域性偏移及結束時間推斷等明確模式；不得無條件把下一條字幕開始時間作為上一條結束時間。

匯出器只在區域被覆蓋、清除或全屏清除時封閉並寫出；檔案結束時封閉剩餘區域。這樣輸入和記憶體保持流式，同時支援交錯時間軸。頭部依賴後續樣式/DRCS 的 ASS/TTML 可使用臨時 body/events 檔案後組裝，不得為此重讀廣播檔案或快取整個時間線。

## 6. 匯出與 DRCS

正式保真轉換目標：ASS、TTML、ARIB-TTML、專案原生存檔格式。ASS 是高相容的視覺近似，不是無損格式；它需要將狀態變化展開為重疊 Dialogue，並對豎排、ruby、閃爍、特殊裝飾和複雜 DRCS 明示近似限制。TTML 應區分內部完整表達、IMSC 相容模式和 ARIB-TTML 相容模式，不能為了校驗靜默刪除結構。

SRT、普通 WebVTT、TXT/CSV 只能位於“有損/文字提取”輸出，不得被稱為正式字幕轉換，也不應在預設輸出列表中。介面必須說明區域合併、時間切割和樣式丟失規則。

存檔輸出包含字幕操作/場景 JSON、原始 data group/PES/MMT caption asset、DRCS PNG/SVG、PID/asset ID/PTS 和診斷資訊；這是唯一承諾儘可能可逆的長期交換格式。

DRCS 策略按優先順序執行：

1. 使用可證明的標準 Unicode 對映；
2. 僅在記錄對映且經使用者認可時使用通行替代字；
3. 否則匯出字形資源並作為視覺元素引用；
4. ASS 可選臨時字型或向量/點陣圖策略。

不得靜默丟棄、猜測或輸出 `[外:<hash>]` 佔位。GUI 應提供特殊字元檢查器：原始字形、Unicode/替代、出現次數、首次時間和使用者選擇；使用者修訂寫入本地 DRCS 字典。

## 7. libaribcaption、FFI 與渲染

不在第一階段重寫 B24 狀態機。`libaribcaption` 負責字符集/控制碼解釋、DRCS 基礎、區域和字元樣式解析、現有行為參照及可選點陣圖渲染；它不負責 TS/TLV/MMT、專案模型、全部時間軸、匯出、checkpoint 或存檔。

Rust 只能依賴專案維護的小型穩定 C ABI，而不是整套 C++ API/bindgen。FFI 邊界重點審計物件生命週期、指標有效期、UTF-8、異常隔離、allocator 和跨平臺構建/ABI 漂移；FFI 呼叫次數不是主要效能風險。

HTML/CSS 結構預覽可顯示文字、區域、時間、樣式概況和 DRCS 佔位。保真預覽必須由 native renderer 輸出 RGBA/PNG/WebP 低頻快照；按 `render_at(time)` 請求或在字幕狀態變化時更新，不能按影片幀率將畫面送入 WebView。

主介面應優先完成檔案拖放、服務/字幕軌選擇、輸出格式與模式、任務控制和預覽；現代設計意味著預設操作簡單且底層資訊隨時可檢查。檢查器至少顯示容器型別、service ID、PID/asset ID、語言、PTS 範圍、DRCS 數量、CRC 錯誤、丟包數、不連續點與未支援命令。

## 8. IPC、解析安全與測試

GUI 與 worker 使用低頻、帶界限的訊息；首版逐行 JSON stdin/stdout 足夠，例如 progress、warning、track。禁止每個字元/包向前端發訊息。後續可評估 local socket、named pipe/Unix socket、protobuf 或 MessagePack。

TS 188/192/204 包頭和 PAT/PMT 宜手寫小型受限解析；TLV/MMTP 可選 winnow、nom 或受限 cursor。所有 parser 必須：不 panic、不越界、不無限迴圈、不按不可信長度無限分配、報告檔案偏移、可從損壞處重同步。

測試體系不可預設：建立 golden corpus（地上波、BS2K、caption/superimpose、豎排、ruby、DRCS、彩色、位置變化、雙語、損壞 TS、BS4K/8K），每項保留合法的原始/構造樣本、可靠畫面截圖、期望事件 JSON、期望 ASS/有損輸出與已知問題。對舊工具、FFmpeg/libaribcaption、新工具和必要的實際畫面做差分比較：字元、開始/清屏時間、位置、顏色、DRCS、management 切換。至少 fuzz TS sync、PSI length、PES、B24 group、TLV、MMTP、signalling、MPU assembly，並在 Windows/macOS/Linux CI 驗證。

公開專案必須包含構建指令碼、依賴版本、格式說明、測試方法、缺陷、樣本生成器和相容性結果；受版權約束的廣播片段只保留雜湊、截短資料或構造樣本。ResubWinny 的 Worker、Tauri 服務層與 Svelte 前端統一採用 MPL-2.0；第三方庫、二進位制、字型和測試語料仍遵循各自的許可證與來源要求。

## 9. 實施順序與當前證據

1. Rust worker、穩定 CLI/API、專案模型、B24 C ABI、傳統 TS 基準語料和迴歸；**已完成**；
2. ASS/TTML/存檔匯出及 DRCS 視覺資源；**已完成**；
3. 192-byte MPEG-TS packetisation 中的流式私有 PES、ARIB-TTML、時間軸和長樣本回歸；**已完成當前基線**；
3a. 188-byte MPEG-TS 中 private PES 的嚴格 ARIB-TTML 回退路由、ASS/TTML/archive/raw/即時預覽構造流回歸；**已完成構造流基線，真實樣本待補充**；
4. Tauri/Svelte 的任務、軌道、日誌、檢查器、多工處理和原生 mpv 預覽；**已完成當前基線**；
5. B62 原生字型/ruby/豎排/描邊渲染與 M2TS 多樣本差分驗證；**第三階段進行中**；
6. TLV/MMTP 的真實語料驗證與通用 asset 路由；**等待合法真實 TLV 樣本，當前僅實驗性**。

新的 Rust workspace 已建立 `crates/arib-caption-worker`。其 `inspect` 命令在有界讀取內識別 188-byte MPEG-TS、192-byte M2TS、原始 TLV 與未知輸入。傳統 B24 透過專案擁有的窄 C ABI 呼叫 libaribcaption；bridge 會在釋放 native 物件前把平面、區域、Unicode/PUA 字元、定位、顏色、樣式及 DRCS 程式碼、替代資訊、原始畫素複製為 Rust 場景快照。未知 DRCS 同時寫成同名 `.drcs` 原始畫素/後設資料資產，並以 ASS `\p1` 向量繪圖事件表現，不會輸出 `[外:<hash>]`。完整地上波轉換得到 13,653 個 PES、2,230 個字幕物件、2,736 個區域、29,892 個字元、61 個 DRCS 字形、0 個解碼錯誤。M2TS 路由會發現私有資料 PID、以有界 PES 緩衝重組有效載荷、提取 UTF-8 ARIB-TTML 檔案，並將繼承自 `div` 的時間與 `region` 位置寫入 ASS。隨附的 11.5 GB BS4K 迴歸樣本現得到 422 個 TTML 字幕事件、5,051 個字元、0 個解析警告。受限 TLV 路由同樣可轉換完整 `stpp` 載荷，但前提是它為自包含 UTF-8 TTML 且擁有匹配的 MPT NTP 後設資料；其他 asset 繼續走原始證據路徑。Tauri/Svelte GUI 僅展示狀態、預覽與診斷並轉發 typed API 請求，解析、匯出和預覽資料準備仍由 Worker/後端完成。B62 字幕樣式、ruby、writing mode、資源作用域和有界 PNG/字型證據已接入模型。後端已原生光柵化受支援的 TTML 文字、橫排與跨列豎排 Ruby、保守的字形方向/標點、透明度和受限的直接 `tts:textOutline`，並且不會把 `arib-tt:border` 猜作標準描邊。Windows `libmpv-render` 與原生 Overlay 合成已經接通。資源完整預覽、完整 B62 字形方向/標點/描邊語義、通用 TLV/MMTP 字幕抽出和 macOS/Linux 原生預覽仍是第三階段後續工作。

當前模型交付：每個 B24 場景都會拆分為 `RegionInterval`。有界活動區域表只在該區域自身發生變化或消失時關閉它，因此說話人標籤與正文可以擁有獨立、重疊的生命週期。已經關閉的同一區域會被同時寫入保真 ASS、可選 TTML 與 JSONL 存檔記錄。TTML 保留區域時間、位置、範圍、字號、顏色以及帶名稱空間的未解析 DRCS 引用；ASS 繼續以向量 DRCS 字形承擔視覺兜底。Tauri 的完成任務時間軸和診斷視窗直接流式掃描 JSONL，只保留請求頁；直播事件列表只保留後端最近視窗，編輯時間軸使用有界預取區間和追加位元組遊標，不再反覆讀取完整 archive，也不把完整事件歷史送進 WebView。單任務 Worker 可在流式解析邊界協作式暫停、繼續或取消。中斷後 `checkpoint.json` 會記錄檔案大小、mtime、首尾 64 KiB 指紋、軌道和進度上限；恢復會拒絕被替換或截斷的輸入。由於 native B24 與部分 artifact 狀態尚不能序列化，下次啟動仍從錄製檔案的可信起點完整重放，而不會虛假宣稱按位元組續跑。

顯示平面校正（2026-07-25）：根 `<tt>` 宣告有效畫素顯示範圍時，B62/ARIB-TTML 會歸一化到原生渲染器的邏輯 `1920×1080` 平面。缺失該範圍時仍預設邏輯 2K；只有完整的畫素 `origin`/`extent` 幾何至少在一個軸越過 2K 範圍、且可落入標準 3840×2160 或 7680×4320 平面，才會推斷源平面。區域幾何按橫縱軸分別縮放，畫素字號、行距、字距和直接描邊寬度採用有界的統一縮放。因此等價的 2K、4K、8K 源佈局會保持相同的觀眾相對字幕面積；模糊或無效資料絕不會被偷偷當作 4K。原始 PES/MMTP 證據保持不變。

豎排標點增量（2026-07-25）：原生 B62 預覽只對映 Unicode 明確定義的豎排標點形式，並且僅當捆綁 ARIB 字型含有該字形時使用；否則保留源字元。archive 到 `render_at` 的確定性 PNG 金樣覆蓋該路徑。這不表示已實現拉丁字元旋轉、縱中橫、完整朝向/標點規則或標準 B62 描邊。

原生預覽同步增量（2026-07-25）：`sync_preview_overlay` 將 mpv 播放時間讀取、archive 查詢、原生 RGBA 合成、overlay 寫入/清除與相同字幕平面去重全部保留在 Tauri 後端。Svelte 只低頻呼叫 typed API 並展示結果，不估算媒體時間、不排版字幕；mpv 尚未返回時間時後端明確返回 `awaiting-player-time`，不使用本地時鐘猜測。

播放時間軸增量（2026-07-25）：原生預覽現在持有經過校驗的 `PlaybackTimeMapping`，包括 segment 標識、媒體/專案錨點與有理速率。libmpv 只提供媒體時間，archive 渲染使用對映後的專案時間；PTS 修復、節目邊界與使用者偏移不會再偷偷落入 WebView 邏輯。

libmpv 執行時增量（2026-07-26）：Windows 現由專案程式內載入捆綁的 `libmpv`，不再啟動 `mpv.exe` 或使用 JSON named pipe。完整 render API 可用時，`libmpv-render` 是預設路線：專用 WGL 執行緒獨佔 OpenGL context、libmpv render loop、resize 訊息與後端 BGRA 字幕紋理混合；指定源初始化失敗時才回退到 `libmpv-client-overlay`。能力 API 對每條路由返回 `id`、`available`、`experimental` 與結構化不可用原因；macOS/Linux 會明確返回 `preview.platform_not_implemented`。WebView 不接收影片幀或字幕紋理；`render_preview_at` 與 `sync_preview_overlay` 在後端合成有界 native plane 後交給 libmpv。macOS/Linux 原生預覽後端已延期，不屬於當前 Alpha 驗收範圍。

視覺基線校正（2026-07-25）：隨附的 libaribcaption `screenshot0.png` 是專案面向觀眾的電視字幕參考圖。B24 繼續使用 libaribcaption 以已配置的 ARIB 字型、ruby、背景和描邊設定生成 RGBA。B62 以相同的觀眾可見關係為目標；但沒有匹配的 B62 源 payload 與合法參考截圖時，不宣稱畫素級一致，見 `docs/visual-reference.md`。

橫排佈局增量（2026-07-25）：原生 B62 路徑現在保留明確換行，並在有界 TTML 區域內應用 `textAlign`、`displayAlign` 與 `lineHeight`，其中 `start`/`end` 會遵循書寫方向。archive 到 `render_at` 的 PNG 金樣覆蓋多行、居中、底部對齊。

參考實現審計（2026-07-25）：`makeding/aribb62.js` 在審計 commit `74304d40a5b8556be1148e123ae70d60f937ecf5` 的 package 後設資料中宣告 MIT，但倉庫和 GitHub license endpoint 都沒有獨立 LICENSE 檔案。其語義可以獨立移植到 Rust renderer；在取得可再分發的許可證文字與版權通知前不 vendoring 原始碼。首個移植是原生命名 TTML 顏色（含 `transparent`），不依賴瀏覽器 CSS。

原始 TLV 輸入透過重複的 4-byte `0x7F/type/length` 頭、受限 payload 長度進行內容探測，並提供有界的診斷/原始證據 MMTP 路徑：直接 IPv6/UDP、HCfB `0x60`/`0x61` 上下文、MMTP packet ID/payload type、連續 signalling fragment 重組（最多 16 路、每路最多 1 MiB），以及 MPT signalling table 中的 asset type 與 descriptor tag（包括已觀察到的 `stpp`）都會報告。MPT MPU timestamp descriptor 會以 packet ID + MPU sequence 為鍵保留精確的 64 位 NTP 原始值，但不會冒充已歸一化的字幕 PTS。對於已知 `stpp` packet ID，會驗證 MPU/MFU 封裝，並有界重組 MFU（最多 8 個 MPU sequence、每個最多 4 MiB）。首個語義路徑只接受同時滿足三項條件的載荷：已發現的 `stpp`、完整且自包含的 UTF-8 XML TTML、以及匹配的 MPT NTP 後設資料；它以首個有效 MPU 為零點，把 NTP 差值送入既有 TTML 字幕模型。序號斷裂、非法聚合、超限、缺失時間戳或其他載荷格式仍只作為原始證據儲存，絕不猜測為字幕。這不是泛用 MMTP 字幕支援宣告。桌面 DRCS 字典會儲存使用者對映到平臺配置目錄；只有使用者明確選擇對映模式才替換為文字，預設仍保留未解析字形資源。
請求 archive 時，同一次有界掃描還會寫入已發現 MPT asset 的 `asset_evidence` 記錄（packet ID、型別、descriptor tag 和精確 NTP 原值）。`resource_reference` 會保留來源 `packet_id + mpu_sequence_number` 作用域；`subsampleNumber=0` 是 TTML payload，有限的 `1..lastSubsampleNumber` 單元組成同一 MPU 的資源證據。數字 `subt://` 索引不會被當作全域性 packet ID；證據缺失或不完整時仍明確保持未解析狀態。

`dump-tlv` 是該層首個原始抽出路徑：它只進行一次順序掃描，只有在已發現的 `stpp` asset 形成完整 closed-caption payload 後才寫入 JSONL。每條記錄保留 TLV 源偏移、MMTP packet/sequence、MPU sequence、timed-MFU 標誌和無損十六進位制資料；若對應 MPT MPU timestamp descriptor 存在，還會保留精確 `presentation_ntp` 原始值。`pts_ms` 仍必須明確為 `null`，直到實現共享時間軸策略；原始證據不得虛構時間軸。
同一路徑現在也會將已完成有界重組的非 `stpp` MPU/MFU payload 寫為 `mmt_asset_payload` 記錄，保留 asset type、源偏移、確定性的 MPU 作用域鍵和無損位元組。資源記錄可以包含有界頭部校驗、PNG 尺寸，以及小型完整 PNG 的受限預覽 data URI，但這仍只是抽出證據，不表示該 payload 已完成通用解碼或可直接渲染。

實現校正（2026-07-23）：M2TS 檔案結尾的 PES flush 迴歸已修復。隨附 BS4K 樣本現在得到 422 個 TTML 字幕事件、5,051 個字元、0 個解析警告；啟用原始匯出時會捕獲 330 條 PES 記錄。桌面端為 Tauri 2 + Svelte 5，而非歷史 Slint/eframe 原型。首頁任務列表會在平臺配置目錄中原子儲存最近 20 條本地任務摘要，不儲存廣播 payload。

本地語料校正（2026-07-23）：18.58 GB 地上波與 11.52 GB BS4K 樣本現在由 `ARIB_FIXTURE_DIR` 選擇，並作為 opt-in 測試驗證精確的 streamed byte/count baseline。M2TS 私有 PES envelope 不再被假定為 UTF-8：有界 extractor 會定位完整的 `<tt>…</tt>` 位元組切片，只對該 XML 切片做 UTF-8 驗證。這恢復了 BS4K 樣本的 422 條字幕/5,051 字元，同時不改變 raw PES evidence。

DRCS 報告交付（2026-07-23）：可選的 `--drcs-report` 只會在傳統 B24 轉換實際遇到 glyph 時生成 `<name>.drcs.json`。它索引程式碼、尺寸、與顏色無關的 glyph 後設資料、替代資訊及已儲存 `.drcs` 資產路徑，不會在報告中複製原始畫素位元組。原生 UI 暴露同一選項；專案 archive 仍是獨立的完整字幕時間線。

TTML 繼承校正（2026-07-23）：受限的 M2TS/TLV TTML parser 現在會在每條字幕前遍歷所有仍處於開啟狀態的 `div`，而不是隻取文字上最近的 `<div>`。巢狀 `begin`/`end`/`dur` 會從正確的父時間基準累積，繼承的 `style` 與 `region` 按 document order 應用，已經關閉的 sibling 不會把 timing、writing mode、colour 或 placement 洩漏到後續字幕。這會改善共享 TTML/archive 模型與保真 TTML 輸出；ASS 對 writing mode 和 ruby 仍是近似表達。

TTML 樣式交付（2026-07-23）：共享字幕樣式現會在 archive 與 TTML interchange 中保留繼承的前景/背景色、字型族、字號、粗細、斜體、書寫方向、文字/顯示對齊、輪廓、行距、字距和透明度。ASS 只對映其有明確定義的字型、粗斜體、字距與前景色；對於不受支援的 TTML 排版或背景語義，不會偽稱保真。

ARIB-TTML span 樣式校正（2026-07-23）：實際廣播 payload 常將有效樣式置於 `span style="…"` 而非 `p`。解析器現會解析該引用，包括雙軸字號、`arib-tt:letter-spacing` 與 TTML 八位 RGBA 色值。interchange 輸出會把安全的 span 引用展開為自包含的內聯 TTML 屬性，因此不會遺留僅在原始檔中存在的樣式 ID。真實 BS4K 樣本已驗證 archive/TTML 中的 `丸ゴシック`、`144px 144px`、前景/背景色與 16px 字距，以及相應的 ASS 近似對映。

字元編碼校正（2026-07-23）：ARIB STD-B24 的字元編碼字幕不會被當作 UTF-8 文字；它仍交給 libaribcaption 按 B24 規則解碼。對於 ARIB-TTML 路徑，提取器先從 PES/MMTP 外層中隔離 XML，再遵循 BOM/XML 宣告，嚴格解碼 UTF-8、UTF-16LE/BE、Shift_JIS、EUC-JP 或 ISO-2022-JP。畸形或不支援的 XML 會保留為原始證據並被報告，絕不會以替換字元“修復”；外層 framing 的非法位元組也不會再導致後續合法檔案被丟棄。

當前 worker 已按 `cli.rs`、`inspection.rs`、`jobs.rs`、`preview.rs`、`archive.rs`、`protocol.rs`、`resource.rs`、`transport/`、`caption/`、`timeline.rs`、`drcs.rs` 與 `exporters/` 拆分，`main.rs` 只保留程式入口和測試。`render-at` CLI 與 Tauri 的 `render_at` 都基於有界 archive 時間點快照；這不等同於已完成原生字幕平面渲染或通用 TLV/MMT 支援。

## 10. 變更紀律

任何架構變更必須在三語檔案同一變更中更新，並註明：影響的輸入 route/模型不變數、對應樣本與驗證、ASS/存檔/DRCS 對映相容性。未經這種記錄，不得把推測、臨時原型或單一樣本結果宣稱為支援。

現實輸入優先順序：當前 release gate 是 188-byte MPEG-TS/B24 與已成功嚴格驗證的 192-byte MPEG-TS packetisation/private PES/ARIB-TTML，兩者都有本地長樣本和流式計數基線。原生 BS4K/8K 的 TLV/MMTP 是規範主線，但目前只有構造/單元證據與受限 `stpp` 路由，能力碼為 `tlv_mmtp_experimental`；在獲得合法真實 TLV 語料前，它只提供探測、診斷、原始證據和明確條件下的轉換，不作為通用支援。inspection contract 使用 `mpeg_ts_b24_verified`、`mpeg_ts_ttml_candidate`、`tlv_mmtp_experimental` 與 `unknown_unsupported`：private PES PID 僅為 candidate，不能冒充 TTML 驗證結果；`mpeg_ts_192_ttml_verified` 只用於完成嚴格驗證後的 192-byte M2TS 轉換 route。這些能力碼來自內容探測，不來自副檔名。

MPEG-TS 動態 PMT 校正（2026-08-02）：B24 邏輯軌以 `service_id + component_tag` 標識，不把檔案開頭髮現的 PID 當作整份錄影的永久屬性。`inspect` 在檔案頭和全檔案固定數量的 1 MiB 視窗中有界取樣 PAT/PMT；順序解碼則持續跟蹤 current PAT/PMT，並在同一邏輯軌遷移 PID 時重新整理舊 PES 後切換。`component_tag 0x30..=0x37` 歸為字幕，`0x38..=0x3f` 歸為文字超級，後者不得進入普通字幕或 TTML candidate。一個 21,609,477,452-byte 實際錄影在 PMT version 更新後發現 PID `0x1201`，完整轉換得到 18,722 PES、3,825 scene、70,853 字元和 0 decoder error；ASS/archive/DRCS 語義不變，raw evidence 記錄每個 PES 的實際來源 PID。

換列豎排 ruby 增量（2026-07-25）：後端現在會在明確關聯的 ruby 正文自動換列時，按已記錄的正文字元格閱讀路徑分配 ruby 字形，並在對應書寫方向的側邊以 0.5 倍繪製。該受限 continuation 已有 archive 到 `render_at` 的 PNG 金樣覆蓋；它不表示已完成通用 B62 ruby 分組、來源特定定位、縱中橫或完整字形朝向。

桌面持久化校正（2026-07-26）：設定、任務記錄、任務歷史、artifact manifest、checkpoint 與 DRCS 對映現在統一使用同目錄原子釋出器：先同步完整 `.part`，保留舊後設資料直到新檔案安裝成功，替換失敗則恢復舊檔案。這修正了 Windows 的覆蓋語義；不改變字幕 payload、archive 語義或任何傳輸 route。

## B62 收斂增量（2026-07-26）

原生 TTML/B62 預覽現將連續的 `tts:ruby="base"` span 作為一個 base group，把一條 `tts:ruby="text"` 註釋放在整個 group 上方；`arib-tt:ruby` 仍按 `xml:id` 關聯。ruby 註釋自身的 colour、font size、letter spacing、opacity 和受限 direct `tts:textOutline` 會在後端保留；未明確指定時使用基字 0.5 倍的預設比例。該模型同時覆蓋橫排、豎排以及自動換列的豎排 ruby。

豎排 renderer 對具備 Unicode vertical-presentation glyph 的標點優先使用該 glyph，CJK 與全形字元保持正立，ASCII/Latin 字元使用後端順時針點陣圖旋轉；明確的 1–2 位 `textCombine` 繼續在單個豎排格內橫排。2K、4K、8K authored geometry 在 worker 中歸一化到邏輯 `1920×1080` 平面，等價佈局因此保持相同的觀眾相對面積。

這些是可重複的後端實現與 unit/visual-golden 覆蓋，不等於已用真實 B62 錄製流驗證所有 broadcaster-specific rule。下一步由 corpus 的合法 source payload 和參考截圖確定是否需要擴充套件非連續 ruby、額外的 Unicode orientation 類別或標準描邊語義。

## Windows 原生預覽收斂增量（2026-07-26）

Windows 在發現完整 `libmpv` render API 時預設選擇 `libmpv-render`。後端擁有 WGL context、libmpv render loop、resize、影片 viewport、後端 BGRA 字幕紋理和混合；若特定源無法初始化 render worker，則該次預覽回退到 `libmpv-client-overlay`，backend diagnostics 會報告實際路線、回退原因、surface 尺寸和呈現幀率。真實 3840×2160 HEVC `bs4k_test_2.ts` smoke 已驗證啟動、影片幀 present、1920×1080 紋理混合/readback，以及 3840×2160 resize/present。WebView 不接收影片幀或字幕紋理。

當前 WGL route 請求 libmpv 的 `hwdec=auto-safe` 策略，允許相容的 copy-back 加速，但不承諾 zero-copy 的 ANGLE/D3D 硬解互操作。`scripts/validate-preview.ps1 -Long` 現已執行帶明確啟動、幀率、完整字幕平面上傳、控制、工作集和退出閾值的 120 秒真實 4K 門檻。2026-07-30 的 `bs4k_test_2.ts` 實測為 `d3d11va-copy`、34.74 present/s、峰值 1526.9 MiB、4K 預熱後增長 111.9 MiB。獨立 2K/8K 效能、DPI 和參考截圖差分仍未完成；macOS/Linux 仍返回 `preview.platform_not_implemented`。

## ASS 保真校正（2026-07-29）

B24 ASS 匯出器現在先將解碼來源畫布歸一化到 ASS 的 1920×1080 play resolution，再同比變換每個可見字元的位置、字號、橫向比例、描邊和 DRCS 幾何；逐字元顏色、粗體、斜體和下劃線保持不變。Ruby 使用換算後的廣播字元格座標並置於 layer 1。ARIB-TTML 路線保留安全的行內 span 樣式，將明確關聯的 Ruby 分層輸出；註釋未指定字號時使用基字的 0.5 倍。依照 TTML 文字排版語義及審查過的參考實現，B62 雙維字號只取第二維作為 ASS 字高，letter spacing 僅透過 ASS 原生 spacing 指令應用一次。匯出器不再橫向拉伸字型，也不以專案自制的逐字元網格替代 libass shaping。獨立 Ruby region 根據來源幾何關係匹配基字 region，並以 ASS 標準 `an8+pos` 居中到被注音範圍的實際渲染字形中心。正文保持為一個完整 Dialogue event，既不拆分也不移動；僅用同捆字型的 libass-compatible advance 與 ink bounds 修正 Ruby 錨點。單字和多個漢字使用同一範圍中點規則，上置、下置均可識別；多行字幕會先選擇與 Ruby 垂直距離最近的來源行，再對映水平覆蓋範圍。FFmpeg/libass 畫素測試覆蓋單字上置以及下方一行的多字下置，最終水平中心誤差超過 3px 即失敗，並逐畫素比較加入 Ruby 前後的正文畫面不發生變化。相同時段字幕只緩衝到 timing 變化為止，仍滿足流式記憶體邊界。

ASS 預設使用隨專案提供的 `Rounded M+ 1m for ARIB`，廣播源的 `丸ゴシック` 也對映到這一經過測量的字型，使 Ruby 寬度計算與播放器實際渲染採用一致字形度量；其他明確指定的來源字型保持不變。18.58 GB 地上波與 11.52 GB M2TS 樣本均以 0 解碼錯誤完成，並透過 FFmpeg/libass 實際渲染的 `いかり`/`碇` 以及以 `捧` 字中線居中的 `ささ` 幀確認位置、前景色、字號與黑色描邊。任意 TTML 半透明背景矩形不屬於 ASS 相容目標，仍保留在 TTML/archive 資料中。

## Ruby 對應關係與匯出專用 Box Layout（2026-07-30）

Ruby 對應關係現已成為字幕模型階段的產物，而不是 ASS exporter 臨時執行的啟發式規則。B24 `RubyBinding` 會在 `RegionInterval` 進入匯出器之前記錄基準 region/index 範圍、基準文字與 cell box、Ruby 原始盒、上下位置、書寫方向和來源依據。ARIB-TTML 同樣記錄基準 caption/run/grapheme 範圍；同一時間組中的獨立 B62 Ruby region 會在有界分組完整後、archive/TTML/ASS 寫出前建立對應關係。真實 M2TS corpus 當前形成 31 條結構化 binding，其中 `ささ` 明確對應 `捧`；無法證明的 region 保持未繫結，不作猜測。

只有 ASS 離線匯出使用 Box Layout。佈局器透過可替換的 glyph-metrics 介面測量隨程式提供的 Rounded M+ 字型，把基準文字的實際 ink range 分配為總寬度嚴格相等的 slot；字形墨跡可能重疊時按整數畫素縮小字號，最後僅對整組可見 Ruby 墨跡做一次有界整數畫素回退校正。正文始終保持為一條由 libass shaping 的 Dialogue，只有 Ruby 字形允許分別定位。顯式 `rubyPosition` 的上置/下置會保留；豎排目前只提供同一演演算法的軸轉置資料路徑，等待真實豎排 corpus 驗證。由於 libmpv 內部使用的 libass 不公開字形度量 API，FFmpeg/libass 畫素測試是當前執行時相容門檻。該佈局不會進入或改動原生預覽鏈路（`libaribcaption -> native RGBA -> libmpv surface`）。

## 順序 ARIB-TTML 檔案與私有 PES 軌道（2026-08-02）

符合 namespace 規範的 TTML 現在透過只讀 XML 樹的 local-name 與祖先關係解析，不再要求標籤必須寫成字面 `<p>`。部分 192-byte 錄製檔案的 ARIB-TTML 檔案不含 `begin`、`end` 或 `dur`；同一 PID 的下一份完整檔案會關閉上一份檔案，空 `<tt>` 表示清屏。若 private PES 雖設定 PTS 標誌卻未滿足 MPEG marker/prefix 規則，零填充值會被拒絕，192-byte 路線改用處理 30-bit 迴繞的 M2TS arrival timestamp。各 PID 的檔案狀態完全隔離。

PMT 的 `component_tag 0x30..0x37` 與 `0x38..0x3f` 分別分類 caption 和 superimpose，但該標籤本身不證明 B24 或 TTML。B24 仍須具有 `data_component_id 0x0008`，TTML 仍須透過完整 XML 與嚴格編碼驗證。預設預覽和匯出只選擇宣告的 caption 軌，superimpose 作為獨立可選軌保留；描述符無法分類時保持 candidate，不以 PID、檔名或節目名稱猜測。

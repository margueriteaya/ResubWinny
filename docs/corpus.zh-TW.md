[簡體中文](corpus.md) | [English](corpus.en.md) | [日本語](corpus.ja.md) | [繁體中文](corpus.zh-TW.md)

> 本檔案是翻譯版本；簡體中文版本是唯一具有權威性的來源。

# 本地廣播迴歸語料庫

廣播錄影有意不提交或再分發。請將合法的本地樣本放在任意目錄中，並將 `ARIB_FIXTURE_DIR` 設定為該目錄。測試有意不回退到隱式的開發者路徑，因此長樣本執行總會明確指出即將讀取的語料庫。

```powershell
$env:ARIB_FIXTURE_DIR = 'C:\tvrecords_testfile'
$env:ARIB_LONG_FIXTURE = '1'
cargo test -p arib-caption-worker decodes_ -- --nocapture
```

這些選擇加入式檢查會流式處理完整輸入，並斷言以下當前基線。它們不釋出源位元組、字幕或螢幕截圖。

該語料庫有意優先採用使用者實際可以獲得且內容已經驗證的錄影：地面 MPEG-TS 樣本和 192 位元組 MPEG-TS/TTML 樣本是釋出門禁。後者是分組化的 MPEG-TS 錄影，不得用作已捕獲原生 BS4K TLV 的證據。目前 TLV/MMTP 沒有同等的本地釋出樣本；在取得合法真實捕獲之前，其解析器、信令限制和原始證據契約由有界構造測試覆蓋。

公開協議樣本可從 worker 的 `synthetic` 模組獲得：`make_ts_packet`、`make_pat`、`make_pmt`、`make_pes`、`make_b24_data_group` 和 `make_mmtp_packet` 為解析器測試構造確定性的分組和段邊界，而不嵌入廣播錄影，也不聲稱具有廣播機構特定語義。

若要在不完整掃描的情況下對釋出工件進行冒煙檢查，請執行：

```powershell
$env:ARIB_FIXTURE_DIR = 'C:\tvrecords_testfile'
.\scripts\validate-corpus.ps1
```

新增 `-Long` 可將兩項完整轉換執行到臨時驗證目錄中。該指令碼絕不會把輸出寫入語料庫目錄。

| 樣本 | 路由 | 釋出狀態／所需證據 |
| --- | --- | --- |
| `chijo_digital_test.ts` | ISDB-T MPEG-TS / ARIB STD-B24 | **釋出門禁。** 18,579,078,944 個輸入位元組；13,653 個 PES；2,230 個場景；2,736 個區域；29,892 個字元；61 個 DRCS 字形；0 個解碼器錯誤。NIT 網路名稱、當前 EIT 節目後設資料和 TDT/TOT 廣播時間必須全部存在。 |
| `bs4k_test.m2ts` | 192 位元組錄影機 M2TS / 私有 PES / ARIB-TTML | **釋出門禁。** 11,517,020,160 個輸入位元組；330 個 PES；422 條 TTML 字幕；5,051 個字元；0 個解析器錯誤。同時間區域關聯目前會在歸檔/ASS 輸出前記錄 31 個結構化 Ruby 繫結，其中包括從 `ささ` 到單個基礎字素 `捧` 的繫結。 |
| `bs4k_test_2.ts` | 188 位元組錄影機 MPEG-TS / ARIB STD-B24 | **釋出門禁。** 3,089,047,552 個輸入位元組；服務 101 從 ARIB SI 解碼為 `NHK　BSP4K`；NIT 網路名稱、當前 EIT 節目後設資料和 TDT/TOT 廣播時間必須全部存在；PID 0x0130 有 2,038 個 PES、118 條字幕、157 個區域、1,661 個字元及 0 個解碼器錯誤；單獨公佈的 PID 0x0138 沒有字幕事件，必須保持為空結果，不得偽造第二條軌道。 |
| 本地 38.07 GB 巴黎錄影（不再分發） | 192 位元組 M2TS / 私有 PES / 順序 ARIB-TTML | **通用路由迴歸。** 內容探測發現服務 101、PMT `0x0100`、字幕 PID `0x1C00`（`component_tag 0x30`）及獨立疊加字幕 PID `0x1C01`（`0x38`）。XML 具有完整 TTML 名稱空間，但省略元素計時；無效的全零 PES PTS 會被拒絕，並由可感知迴繞的 M2TS 到達時鐘在同一 PID 的下一個檔案處結束每個檔案。完整的預設字幕轉換讀取 38,065,729,536 位元組，並且必須保留 2,715 個字幕區域、28,618 個字元及 0 個解碼器錯誤，輸出須按單調順序持續至 03:11:48。它不得以檔名、服務 ID、節目名稱或固定 PID 值作為路由例外。 |
| 本地 20.12 GiB BS 錄影（不再分發） | 188 位元組 MPEG-TS / PMT 版本及字幕 PID 轉換 | **動態 PMT 迴歸。** 初始 PMT 僅公開疊加字幕 PID `0x1C12`（`component_tag 0x38`）；後續當前 PMT 新增字幕 PID `0x1201`（`component_tag 0x30`）。檢查必須僅報告 `0x1201`。完整轉換讀取 21,609,477,452 位元組，併產生 18,722 個選中 PES、3,825 個場景、6,679 個區域、70,853 個字元、7 個 DRCS 字形及 0 個解碼器錯誤。原始證據必須僅包含 PID `0x1201`。 |
| 構造的 PMT 版本轉換 TS | MPEG-TS / B24 字幕與疊加字幕 | 固定大小發現視窗必須在初始僅疊加字幕的 PMT 之後找到較晚的字幕元件；順序解碼必須僅路由選中的邏輯 `service_id + component_tag`，並拒絕疊加字幕 PES。 |
| 構造的 188 位元組私有 PES TS | MPEG-TS / PMT 私有 PID / 嚴格 ARIB-TTML | B24 發現保持為空；私有 PID 被發現；轉換、ASS、TTML、歸檔、原始 PES 證據及有界預覽均產生一條經過驗證的 TTML 字幕。 |
| `testdata/golden/b62-layout.xml` | 構造的 ARIB-TTML 語義樣本 | 穩定 JSON 摘要驗證巢狀計時、百分比區域、橫排 Ruby 證據、豎排書寫模式、字號及顏色，而不再分發廣播材料。單元迴歸還驗證：等效宣告的 1920×1080、3840×2160 和 7680×4320 畫素佈局會歸一化為相同的邏輯檢視器幾何形狀和文字長度。 |
| 構造的 TLV/MMTP `stpp` 樣本 | ISDB-S3 TLV → MMTP → MPT/MPU | **僅限實驗。** 驗證邊界、片段丟失、來源及證據優先的 `stpp` 路由；它不能將 TLV/MMTP 提升為具有釋出門禁的路由。 |

解析器模糊測試儲存在釋出工作區之外的 `fuzz/` 中。初始目標覆蓋基於內容的 TS/TLV 探測、嚴格 TTML 信封解碼、有界 ARIB SI 服務名稱文字解碼、188/192 位元組 TS PSI/PES 後設資料解析及 MMTP/TLV 有效載荷信封。`cargo check --manifest-path fuzz/Cargo.toml` 提供穩定工具鏈編譯檢查；CI 還會在 Linux nightly 上使用 `cargo-fuzz` 構建所有目標。PSI/PES/B24 狀態機以及更深層的信令/MPU 語義模糊測試目標仍是未來的語料庫工作；每週工作流會在有界時間間隔內執行每個已宣告目標。

對於視覺或格式更改，請在被忽略的驗證目錄中建立輸出，並比較專案歸檔、ASS、TTML、原始 PES 證據以及未解析的 DRCS 資產目錄。完整命令為：

```powershell
.\build\cargo\release\arib-caption-worker.exe convert `
  "$env:ARIB_FIXTURE_DIR\chijo_digital_test.ts" `
  artifacts\validation\chijo_digital_test.ass --ttml --archive --raw

.\build\cargo\release\arib-caption-worker.exe convert `
  "$env:ARIB_FIXTURE_DIR\bs4k_test.m2ts" `
  artifacts\validation\bs4k_test.ass --ttml --archive --raw
```

M2TS 樣本尤其重要：其私有 PES 信封在有效 TTML 檔案之前含有非 UTF-8 位元組。迴歸不得僅僅因為其傳輸成幀不是 UTF-8 就拒絕整個 PES；XML 文字本身須嚴格依照其宣告編碼或 BOM 解碼。

Windows 原生預覽冒煙門禁使用 B24 `bs4k_test_2.ts` 樣本，且不分發該錄影：

```powershell
$env:ARIB_FIXTURE_DIR = 'C:\tvrecords_testfile'
.\scripts\validate-preview.ps1 -FixtureDirectory $env:ARIB_FIXTURE_DIR
```

它驗證 WGL 宿主建立、程式內 libmpv 載入、渲染 worker 啟動、錄影開啟及乾淨關閉。它有意與視覺螢幕截圖驗收分離，不得將其解釋為硬體解碼或畫素保真度宣告。

新增 `-Long` 可執行帶閾值的 120 秒 Windows 4K 門禁：

```powershell
.\scripts\validate-preview.ps1 `
  -FixtureDirectory $env:ARIB_FIXTURE_DIR `
  -Long
```

該門禁保持 3840x2160 原生表面活動，三次替換完整的 1920x1080 後端字幕平面，並執行暫停、恢復、精確定位及關閉。若低於 20 presents/s，或啟動超過 10 s、控制或字幕上傳超過 1 s、關閉超過 3 s、工作集超過 2048 MiB，或 4K 預熱後工作集增長超過 512 MiB，則失敗。帶模式版本的結果寫入 `build/validation/preview-performance-windows-4k.json`。

2026-07-30 的真實語料庫基線使用 `d3d11va-copy` 持續 120 秒達到 34.74 presents/s；峰值工作集為 1526.9 MiB，預熱後增長為 111.9 MiB。這僅完成 Windows 4K 長門禁。它不是 8K、跨平臺、DPI 或視覺保真度驗收結果。測試框架本身使用 Cargo 的測試配置檔案，同時載入與應用程式相同的捆綁 libmpv DLL 和原生 WGL 路由；打包釋出驗收另行跟蹤。

2026-07-31 的打包 Windows 驗收使用最終釋出可執行檔案和 `bs4k_test_2.ts`。內容探測選擇了 188 位元組 MPEG-TS/B24；原生 libmpv 在初始暫停時呈現影片；可見由 EIT/NIT/TOT 派生的頻道、網路、節目、描述及廣播時間；PID `0x0130` 產生 118 個解碼事件。在流式歸檔從 `.jsonl.part` 更改為其釋出的 `.jsonl` 路徑後，任務時間線重新填充了真實字幕條，且沒有“找不到歸檔”的診斷。這是該路由的打包迴歸結果，並非對每個桌面工作流的驗收或 macOS/Linux 預覽宣告。

## 流式記憶體釋出門禁

有界解析器常量是大型錄影的必要證據，但並不充分。釋出候選版本還必須完成至少一次 1 GiB 或更大 TS/M2TS 轉換，且 Worker 峰值工作集不超過 **384 MiB**：

```powershell
.\scripts\validate-memory.ps1 `
  -Source "$env:ARIB_FIXTURE_DIR\chijo_digital_test.ts" `
  -TrackId 276
```

該指令碼報告絕對峰值及每輸入 GiB 對應的峰值 MiB。絕對門禁會捕獲意外保留整個時間線/PES 的情況；比較 3 GiB、11 GiB 和 18 GiB 樣本的比率可檢查記憶體是否隨錄影時長線性增長。生成的輸出保留在隔離臨時目錄中，並在測量後刪除。

2026-07-27 測得的 Windows x86-64 釋出基線：

| 樣本 | 輸入 | 峰值工作集 | 峰值/輸入比率 |
| --- | ---: | ---: | ---: |
| `bs4k_test_2.ts`，PID 0x0130 | 2.877 GiB | 22.5 MiB | 7.83 MiB/GiB |
| `chijo_digital_test.ts`，PID 0x0114 | 17.303 GiB | 35.7 MiB | 2.06 MiB/GiB |

輸入大小增加六倍時，絕對峰值僅增加 13.2 MiB，這與有界流式處理一致，而不是保留整個錄影。這些數字是此機器的迴歸基線，並不保證每個解碼器/執行時構建都具有相同的分配器開銷。

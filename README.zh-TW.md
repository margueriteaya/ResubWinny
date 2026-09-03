[簡體中文](README.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [English](README.en.md)

> 本檔案為翻譯版本。簡體中文版本是唯一權威來源，其他語言版本僅為同步譯文。

# ResubWinny

> [!WARNING]
> 本專案目前處於 alpha 前期，無法保證可用性，且可能存在破壞性變更！

ResubWinny 是一款在 Windows 上執行、針對泛日本內容影片檔源檔案的字幕擷取、檢查、預覽與轉換工具，具備現代化的使用介面，也可使用命令列進行操作。

它能夠處理地面數位電視、BS/CS 2K 以及部分 BS4K/8K 錄製檔案中的 ARIB 字幕、即時預覽影片，為外部播放器播放盡可能輸出保留字幕位置、色彩、字級、描邊、Ruby（注音）、ARIB 外字、DRCS 字形和無障礙標識的字幕，也可捨棄特殊標籤，輸出為適合後續工作或封存的格式。

目前收斂期專注日本廣播錄製字幕；BD/DVD 圖形字幕 OCR、外掛系統、AI 翻譯以及 macOS/Linux 原生預覽均已明確延後，不屬於目前路線或驗收範圍。DRCS 僅繼續完善本機 hash → Unicode 對應，不擴建通用 OCR 系統。

專案目前版本為 `v0.2.3-α`（原始碼版本 `0.2.3-alpha.1`）。目前仍處於開發階段。

## 功能特色

### 字幕擷取與辨識

- 依檔案內容偵測輸入格式，不依賴 `.ts`、`.m2ts` 或 `.mmts` 副檔名進行判斷；
- 支援 188-byte MPEG-TS 中的 ARIB STD-B24 字幕；
- 支援 192-byte M2TS 風格 MPEG-TS、私有 PES 與嚴格驗證的 ARIB-TTML 字幕；
- 支援多服務、多字幕軌探索與選擇；
- 解析錄製檔案中可用的廣播網路、服務、節目及播出時間，並可依播放位置查詢相應狀態；缺少對應 SI/EIT/TOT 證據時不會偽造欄位；
- 嚴格處理 UTF-8、UTF-16LE/BE、Shift_JIS、EUC-JP 與 ISO-2022-JP 編碼的 TTML；
- 對損壞、截斷或未支援的輸入提供結構化診斷。

### 字幕語意與排版

- 保留字幕區域各自獨立的顯示時間和重疊關係；
- 保留位置、字級、色彩、描邊、透明度、字距和書寫方向；
- 辨識並保留 Ruby 與被標音物件的結構化對應關係；
- 支援橫排、基礎直排、連續 Ruby 分組和直排字元方向處理；
- 辨識 ARIB 特殊符號、自訂 DRCS 字形及常見無障礙標識；
- 將 2K、4K、8K 來源幾何正規化至邏輯字幕平面，使字幕維持相近的觀眾可見比例。

### 匯出與檢查

- 可一次選擇 ASS、TTML、SRT、WebVTT 等多種輸出格式；
- 可分別決定是否保留位置、色彩、Ruby、DRCS、ARIB 外字和無障礙標識；
- 在字幕事件清單中篩選並標記特殊字幕特徵；
- 提供專案封存、原始 PES/MMTP 證據和 DRCS 報告；
- 支援中途暫停，先寫入暫存 `.part` 檔案，成功後輸出完整檔案；
- 後端驗證輸入與輸出路徑，防止字幕輸出覆寫原始錄影。

### 桌面應用程式

- 多語言介面，內建簡體中文、繁體中文、日文和英文；
- 支援淺色、深色和跟隨系統佈景主題；
- 支援多工作建立、排隊、暫停、繼續、取消、歷史記錄和診斷；
- 提供字幕事件清單、可縮放時間軸、點選/拖曳跳轉和 DRCS 字典；
- Windows 使用 libmpv 原生算繪影片，不將影片影格送入 WebView；
- B24 字幕平面和 ARIB-TTML 字幕影像由 Rust 後端產生並疊加至原生預覽。

## 輸入支援狀態

| 輸入路線 | 狀態 | 說明 |
| --- | --- | --- |
| 188-byte MPEG-TS + ARIB STD-B24 | 已驗證 | 針對地面數位電視及 BS/CS 2K 錄製檔案 |
| 192-byte M2TS 風格 MPEG-TS + 私有 PES + ARIB-TTML | 已驗證 | 已透過現有 BS4K 錄製樣本迴歸；是否成立由內容偵測決定 |
| MPEG-TS 中的私有 PES/TTML 候選 | 有界偵測 | 僅在完整 XML 邊界、宣告編碼和 TTML 檔案皆透過驗證後才轉換 |
| 原始 TLV/IP/UDP/MMTP | 實驗性 | 以偵測、診斷和原始證據為主；僅在嚴格條件下處理完整 `stpp`/TTML 資產 |
| 未知或不受支援的輸入 | 明確拒絕 | 傳回穩定錯誤和偵測證據，不猜測容器或字幕型別 |

原始 TLV/MMTP 不屬於目前已驗證的通用 BS4K/8K 支援。副檔名僅用於檔案選擇器提示，不能作為傳輸格式的證據。

## 輸出格式

| 格式 | 用途與限制 |
| --- | --- |
| ASS | 針對字幕製作和常見播放器；保留 ASS/libass 能表達的位置、色彩、字級、描邊和 Ruby，但無法還原字幕後方的半透明背景矩形 |
| TTML | 保留獨立區域、時間、樣式、Ruby、書寫方向、DRCS 參照和來源資訊 |
| SRT | 純文字相容輸出；無法表達廣播位置、重疊區域、Ruby 排版和 DRCS 圖形 |
| WebVTT | Web 相容文字輸出；與 SRT 一樣屬於失真格式 |
| Caption archive (`.caption.jsonl`) | 儲存統一字幕模型、區域生命週期、Ruby 對應關係和來源資訊，用於分頁時間軸和再次算繪 |
| Raw evidence | 儲存來源位移、序號、時間來源及無損 PES/MMTP payload |
| DRCS report/assets | 儲存無法直接對應至 Unicode 的字形、畫素資源、候選對應與使用者選擇 |

## 技術堆疊

| 層級 | 技術 |
| --- | --- |
| 桌面前端 | Svelte 5、TypeScript、Vite、Lucide Svelte |
| 桌面應用層 | Tauri 2、Rust 2024 Edition |
| 字幕 Worker | Rust、版本化 JSONL 協定、串流 I/O |
| B24 解碼與算繪 | libaribcaption、專案自有窄幅 C ABI、C++ 橋接 |
| 原生影片預覽 | libmpv render API；Windows WGL/OpenGL 合成 |
| 字幕模型 | `CaptionPlane -> RegionInterval -> exporters` |
| 持久化 | 應用程式資料目錄中的不可分割 JSON/JSONL 檔案 |
| 測試與品質 | Cargo test、Clippy、Rustfmt、Svelte Check、cargo-fuzz、GitHub Actions |

前端僅顯示後端狀態並轉送型別化請求，不讀取廣播資料、不決定字幕時間、不計算最終字幕排版，也不處理影片影格。媒體與字幕能力由 Worker 或 Tauri 後端先實作，再接入 GUI；能夠在 GUI 中執行的核心操作應有對應的 CLI、Worker 或後端 API。

## 技術架構

```text
Svelte 5 前端
    | typed Tauri API / 低頻事件
    v
Tauri 2 Rust 應用服務層
    | 工作排程、持久化、原生預覽、Worker 管理
    v
arib-caption-worker
    | 串流偵測、解析、字幕模型、時間軸與匯出
    v
libaribcaption
    | 專案自有窄幅 C ABI
    v
ARIB STD-B24 解碼與原生字幕平面
```

Worker 使用固定大小的串流緩衝區和 64 位元檔案位移。檔案體積增加時，常態記憶體不應隨檔案長度線性成長。詳細職責與介面請參閱[中文架構檔案](docs/architecture.zh-CN.md)和[後端介面合約](docs/backend-contract.md)。

## 主要相依套件

| 相依套件 | 用途 | 整合方式 | 授權條款/狀態 |
| --- | --- | --- | --- |
| [xqq/libaribcaption](https://github.com/xqq/libaribcaption) `v1.1.2` | ARIB STD-B24 解碼與原生算繪 | 固定 commit 的 vendored 原始碼 | MIT |
| [makeding/libaribtlv](https://github.com/makeding/libaribtlv) `0.6.1` | 可選的實驗性 TLV/MMTP → B62 TTML 解複用 | `libaribtlv` feature 啟用時以固定 commit 的 vendored 原始碼靜態連結；專案自有窄 C ABI | MIT |
| [Zlib](https://github.com/madler/zlib) `1.3.2` | libaribtlv 的私有壓縮相依性 | 以固定 commit 的 vendored 原始碼靜態連結；不偵測系統 Zlib | Zlib License |
| [mpv](https://mpv.io/) / libmpv | Windows 原生影片預覽 | 動態連結、可替換 DLL；建置時依雜湊下載 | LGPL-2.1-or-later |
| Rounded M+ 1m for ARIB `1.3` | ARIB 字元 fallback、字幕預覽與 ASS 字型度量 | 隨專案散佈字型 | M+ FONT LICENSE 與 WadaLab 授權 |
| Tauri 2 | 桌面視窗、原生 API 與封裝 | Cargo 相依套件 | 請參閱相依套件授權條款清單 |
| Svelte 5 / Vite | 前端介面與建置 | npm lockfile 鎖定 | 請參閱相依套件授權條款清單 |
| serde / serde_json | Worker 協定、模型和持久化 | Cargo 相依套件 | 請參閱相依套件授權條款清單 |
| encoding_rs | ARIB-TTML 字元編碼 | Cargo 相依套件 | 請參閱相依套件授權條款清單 |
| roxmltree | namespace-aware TTML/XML 結構解析 | Cargo 相依套件 | MIT / Apache-2.0 |
| fontdue / ttf-parser | 後端字幕及 Ruby 字形度量 | Cargo 相依套件 | 請參閱相依套件授權條款清單 |
| [makeding/aribb62.js](https://github.com/makeding/aribb62.js) | ARIB-TTML/B62 行為研究參考 | 僅供參考，不繫結原始碼 | 審查版本的 package 中繼資料宣告 MIT；詳見第三方記錄 |

完整版本、固定來源、雜湊和授權條款請參閱[第三方宣告](THIRD_PARTY_NOTICES.md)、[相依套件版本記錄](third_party/versions.json)、[相依套件授權條款清單](docs/dependency-licenses.md)與[相依套件更新政策](docs/dependency-updates.md)。

## 建置環境

目前 Alpha 的完整桌面驗收平臺是 Windows 11 x86-64。需要：

- `rust-toolchain.toml` 鎖定的 Rust `1.97.1`，包括 Rustfmt 與 Clippy；
- Node.js 22 LTS；
- npm 10 或 11；
- CMake；
- Visual Studio 2022 Build Tools，包含 MSVC C/C++ 工具鏈和 Windows SDK；
- Microsoft Edge WebView2 Runtime；
- 7-Zip，用於安裝固定版本的 Windows libmpv 開發套件。

Worker、Tauri 編譯檢查和前端建置會在 Windows、macOS 與 Linux CI 上執行；目前原生預覽和安裝套件的產品驗收平臺仍是 Windows。

## 建置方式

以下命令均在儲存庫根目錄執行。

### 1. 一行命令建置

```powershell
./scripts/build.ps1
```

此命令會安裝鎖定的前端相依套件，下載並核對固定版本的 Windows libmpv，建置 Worker、前端、桌面程式和安裝套件。重複執行時，已驗證的 libmpv 會直接重複使用。ResubWinny 執行時不會自行下載或更新播放元件。

僅產生執行檔、不產生安裝套件：

```powershell
./scripts/build.ps1 -Target Executable
```

建置不附帶 libmpv 的版本：

```powershell
./scripts/build.ps1 -Libmpv External
```

此模式不會散佈 libmpv，但即時預覽需要透過 `RESUBWINNY_LIBMPV` 提供相容執行階段程式庫。需要同時執行完整品質檢查時附加 `-Check`。附帶 libmpv 的本機產出物僅供開發與私下測試；公開發布前仍須為完全相同的 DLL 提供透過驗證的對應原始碼套件和 receipt。

### 2. 執行完整品質檢查

```powershell
./scripts/check.ps1
```

此指令碼涵蓋 Worker 與桌面後端測試、Clippy、Rustfmt、Svelte 檢查、前後端介面合約、fuzz target 編譯、授權條款清單和第三方來源驗證。

### 3. 開發執行

先建置 Worker，再啟動 Tauri 開發程式：

```powershell
cargo build -p arib-caption-worker
npm run tauri --prefix studio-tauri -- dev
```

如需使用其他 Worker，可將 `RESUBWINNY_WORKER` 設定為執行檔的絕對路徑。

### 4. 直接呼叫底層建置命令

基礎 Tauri 設定不強制附帶 libmpv，因此下列命令適合開發和僅使用外部執行階段程式庫的建置。僅產生桌面執行檔：

```powershell
npm run tauri --prefix studio-tauri -- build --no-bundle
```

產生不附帶 libmpv 的 Tauri bundle：

```powershell
npm run tauri --prefix studio-tauri -- build
```

需要附帶已安裝並經過雜湊驗證的 Windows libmpv 時，請使用統一建置指令碼，或明確加入設定：

```powershell
./scripts/setup-libmpv.ps1
npm run tauri --prefix studio-tauri -- build --config src-tauri/tauri.windows-libmpv.conf.json
```

產出物統一位於：

```text
build/cargo/release/resubwinny-studio.exe
build/cargo/release/bundle/
```

清理建置產出物：

```powershell
./scripts/clean.ps1
```

附加 `-Dependencies` 可刪除 `node_modules`；附加 `-DownloadedRuntimes` 可刪除明確安裝的 libmpv 開發檔案；附加 `-TestOutputs` 可刪除本機測試輸出。

## 目錄結構

```text
ResubWinny/
├── crates/
│   └── arib-caption-worker/       # 串流偵測、解析、字幕模型、CLI 與匯出器
│       ├── src/caption/           # B24、TTML/B62 與 Ruby 語意
│       ├── src/transport/         # MPEG-TS、M2TS 與實驗性 TLV/MMTP
│       ├── src/exporters/         # ASS、TTML、SRT、WebVTT、archive 等輸出
│       └── src/tests/             # Worker 分模組迴歸測試
├── native/
│   └── aribcaption-bridge/        # libaribcaption 的窄幅 C ABI 橋接
├── shared/                        # Worker 與桌面後端共用的辨識規則
├── studio-tauri/
│   ├── src/                       # Svelte 前端
│   │   ├── backend/               # typed Tauri API 與事件進入點
│   │   ├── components/            # 通用介面元件
│   │   ├── features/              # 首頁、工作、多工作、DRCS、設定等功能
│   │   └── locales/               # zh-CN、zh-TW、ja、en 文案
│   └── src-tauri/
│       └── src/                   # 工作、預覽、持久化、時間軸與 Worker 管理
├── fuzz/                          # TS、PES、B24、TTML、MMTP 等 fuzz targets
├── scripts/                       # 建置、檢查、清理、語料庫和發布指令碼
├── docs/                          # 架構、介面、語料庫、授權條款與維護檔案
├── third_party/                   # 固定來源的第三方原始碼、標頭檔、字型與授權條款
├── .github/                       # CI、相依套件更新、Issue 與 PR 範本
├── Cargo.toml                     # Worker workspace
├── CONTRIBUTING.md                # 貢獻規則
├── THIRD_PARTY_NOTICES.md         # 第三方宣告
└── LICENSE                        # MPL-2.0
```

所有產生的檔案都應位於被忽略的 `build/`、`node_modules/` 或測試輸出目錄，不應混入原始碼提交。

## 桌面程式使用方式

1. 啟動 `build/cargo/release/resubwinny-studio.exe`。
2. 在首頁選擇一個錄製檔案。程式會依內容偵測容器、服務和字幕軌，並自動準備原生預覽；預覽預設暫停，不會自動播放。
3. 進入工作頁面，檢視廣播服務、字幕軌、事件清單和時間軸；節目與播出時間會在錄製檔案包含並成功解析相應廣播表時顯示。
4. 在預覽視窗中播放、暫停、前後跳轉、拖曳時間軸或調整音量。下方字幕時間軸可以縮放、點選和拖曳跳轉。
5. 在輸出設定中選擇一種或多種格式，並選擇是否保留位置、色彩、Ruby、DRCS/ARIB 外字及無障礙標識。介面會提示目標格式無法完整表達的內容。
6. 選擇輸出目錄後開始匯出。尚未開始匯出前，程式不會在所選輸出目錄建立字幕產出物。
7. 在工作記錄、診斷和產出物清單中檢查結果。遇到未對應 DRCS 時，可在 DRCS 字典中檢視原始影像並儲存對應。
8. 可繼續新增錄製檔案形成多工作佇列，各工作由後端獨立儲存狀態並排程。

## CLI 使用方式

Worker 預設路徑為：

```text
build/cargo/release/arib-caption-worker.exe
```

所有機器可讀事件寫入 `stdout`，人類可讀記錄寫入 `stderr`。下方列出目前全部 CLI 命令。

### `inspect`

偵測輸入格式、服務、字幕軌和候選路由，不進行字幕匯出。

```text
arib-caption-worker.exe inspect <recording>
```

### `broadcast-at`

依來源檔案位元組位移查詢 MPEG-TS/M2TS 中對應的廣播網路、服務、節目與播出時間。`service-id` 使用十進位。

```text
arib-caption-worker.exe broadcast-at <recording> <byte_offset> [--service-id <id>]
```

### `decode-b24`

探索並依序解碼傳統 B24 字幕軌，輸出進度與統計事件，不建立字幕檔案。

```text
arib-caption-worker.exe decode-b24 <recording>
```

### `convert`

依內容自動偵測路線並轉換字幕。未指定輸出路徑時，預設使用輸入檔名並改為 `.ass`。

```text
arib-caption-worker.exe convert <recording> [output] [options]
```

### `convert-b24`

僅使用傳統 MPEG-TS/B24 路線轉換字幕，引數與 `convert` 相同。

```text
arib-caption-worker.exe convert-b24 <recording> [output] [options]
```

`convert` 與 `convert-b24` 支援下列全部選項：

| 選項 | 作用 |
| --- | --- |
| `--ttml` | 同時匯出 TTML |
| `--srt` | 同時匯出 SRT 相容副本 |
| `--webvtt` | 同時匯出 WebVTT 相容副本 |
| `--archive` | 同時匯出 caption archive |
| `--archive-only` | 僅發布 caption archive；不能與其他格式或 `--no-ass` 組合 |
| `--raw` | 匯出路線對應的原始 PES/MMTP 證據 |
| `--no-ass` | 不保留預設 ASS 輸出 |
| `--drcs-report` | 發現 DRCS 時產生報告 |
| `--drcs-map <json>` | 使用指定 JSON 檔案中的 DRCS 使用者對應 |
| `--track-id <id>` | 選擇字幕 PID/asset；接受十進位或 `0x` 十六進位 |
| `--drop-position` | 不保留字幕位置 |
| `--drop-color` | 不保留色彩 |
| `--drop-ruby` | 不保留 Ruby |
| `--drop-drcs` | 不保留 DRCS 字形 |
| `--drop-gaiji` | 不保留 ARIB 特殊外字 |
| `--drop-accessibility` | 不保留無障礙標識 |
| `--overwrite` | 允許覆寫已存在的輸出產出物；仍禁止覆寫輸入錄影 |

範例：

```text
arib-caption-worker.exe convert recording.ts output.ass --ttml --archive --raw --drcs-report
arib-caption-worker.exe convert recording.m2ts output.ass --track-id 0x120 --srt --webvtt
arib-caption-worker.exe convert-b24 recording.ts output.ass --drop-position --drop-accessibility
arib-caption-worker.exe convert recording.ts output.caption.jsonl --archive-only
```

轉換執行期間可透過 `stdin` 逐行傳送協作式控制訊息：

```json
{"type":"pause"}
{"type":"resume"}
{"type":"cancel","keepCheckpoint":true}
```

### `render-at`

從 caption archive 讀取指定毫秒時點的字幕區域快照，並以 JSONL 事件輸出。

```text
arib-caption-worker.exe render-at <archive.caption.jsonl> <time_ms>
```

### `dump-tlv`

實驗性 TLV/MMTP 原始證據擷取。未指定輸出時預設產生 `.caption.mmtp.jsonl`。

```text
arib-caption-worker.exe dump-tlv <input> [output.caption.mmtp.jsonl] [--overwrite]
```

此命令僅輸出已探索 `stpp` asset 中完整的 closed-caption payload，並保留 TLV 位移、MMTP/MPU 序號、原始 NTP 和無損位元組。它不會將 NTP 冒充為 PTS，也不會將未知 asset 猜測為字幕。

## 開發與驗證

Worker 可以獨立建置和測試：

```powershell
cargo test -p arib-caption-worker
cargo build -p arib-caption-worker --release
```

涉及傳輸、協定、字幕模型、時間軸、算繪或匯出器的修改，需要增加對應迴歸測試。合法但無法再散佈的大型錄製樣本僅保留於本機，透過 `ARIB_FIXTURE_DIR` 參與選用的長樣本驗證。詳細說明請參閱[語料庫與迴歸檔案](docs/corpus.md)。

參與開發前請閱讀[貢獻指南](CONTRIBUTING.md)、[中文架構檔案](docs/architecture.zh-CN.md)、[後端介面合約](docs/backend-contract.md)、[工具鏈政策](docs/toolchain-policy.md)和[可維護性說明](docs/maintainability.md)。

## 限制

- BS4K/8K 的原始 TLV/MMTP 是隔離的實驗能力，不屬於已驗證的通用 BS4K/8K 支援；
- 192-byte M2TS 支援僅說明封包封裝路線，不代表完整支援 BDMV/BDAV 目錄、播放清單、CAS 或廠商私有錄影管理資訊；
- ResubWinny 不是錄影管理器、直播接收器、CAS 解密工具或完整 EPG 瀏覽器；
- SRT 和 WebVTT 無法準確表達重疊區域、廣播位置、Ruby 排版、DRCS 圖形和全部 ARIB 時間語意；
- BS4K/8K 訊號所對應標準 B62/ARIB-TTML 的規則仍處於研究狀態。

原始碼發布與 Windows 二進位發布採用不同門檻，具體專案請參閱[發布檢查清單](docs/release-checklist.md)。

## 題外話

對我而言，日本的電視文化有其獨特魅力。略過電視上播出的內容本身，它的聯播體制、技術細節同樣令人著迷。旁人可能無法理解我的這種痴迷，但如果我說，在我居住的國家，電視訊號只有單音軌（甚至沒有原聲重現）和畫面呢？當我第一次接觸日本數位電視時，看到它有可以開關的字幕，看到有可以互動的資料廣播，有一種劉姥姥進了大觀園的感覺。回想起小時候看電視時，某一天燒在畫面上的字幕突然消失時，哭著鬧著說看不懂了，讓父母加訂了幾個兒童頻道的這件事情，會感覺我住的國家的電視文化能說道的只有它本身的歷史，其餘的除了蒼白還是蒼白。

但日本的電視訊號，滿滿都是自己造的輪子和「加拉巴哥現象」的痕跡。一般人，離了那一套特供的收視環境，便難以看見它真實的面貌。得益於多年來開發者們的熱情，訊號源慢慢變得可以被解析，自由地看電視不再是一句空話，但一般人仍然缺乏便利與「可以被理解」的工具來跨過技術門檻。讀懂文字是理解的開始，字幕是文字的載體，於是我想從字幕開始做一個工具。

說回 ResubWinny 這個名字，原本的 Winny，是在 2002 年發布的 P2P 檔案分享軟體，該軟體的廣泛使用伴隨著著作權內容與不當內容的傳播，被視為社會問題，但軟體作者卻因為使用者的行為遭到檢控。2023 年，以該事件為題材的同名電影上映，在上海國際電影節展映的片名叫做「開發者有罪」。

Resub 代表對字幕的再加工，而 Winny 是為了向這個名字的發明者金子勇 a.k.a. 47氏致敬，也是為了遵循一個基本常識：

**開發通用技術並非犯罪，這是表達自由的一部分。**

本專案不涉及 P2P 網路、檔案分享、媒體探索或內容散佈。它是一個開放原始碼工具，讀取媒體格式、還原字幕、執行 OCR 或轉換資料的工具本身並不構成侵權。它們的合法性和倫理性取決於它們的使用方式，而不僅僅是它們在技術上能夠實現的功能。

製作 ResubWinny 的願望，緣起於太多關於日本媒體處理的知識被困在難以整理的 2ch 貼文、廢棄的 Windows 公用程式、ARIB 官方以階級來劃分可見性的檔案，以及不提供原始碼的軟體中。這些知識應該更加開放、可稽核、可移植且可儲存。現有的工具也常常因為指引不明確、理解有門檻而勸退初學者。

因此，這個專案並非意在重現原版 Winny。它是一種宣言：**開發者應享有建構合法工具的自由，使用者應享有理解其所擁有媒體的自由，技術本身應簡單易用，知識不應因恐懼、起訴或封閉程式碼而消失。**

## 特別感謝

- [xqq](https://github.com/xqq)，`libaribcaption` 的作者，長久以來為我研究日本電視廣播提供了巨大的幫助
- [huggy](https://github.com/makeding)，`aribb62.js` 的作者，為本專案解析 BS4K/8K 訊號字幕提供了支援
- [tsukumi](https://github.com/tsukumijima)，`KonomiTV` 等專案的作者，為「自由看電視」的文化長期貢獻
- Bunny，我的女友，具有強大的技術背景，在開發過程中幫我解決了十分棘手的問題
- Codex，OpenAI 開發的基於大型語言模型的代理工具，沒有它，缺乏技術基礎的我就不可能用自然語言推動這個專案

## 授權條款

ResubWinny 自有原始碼採用 [Mozilla Public License 2.0](LICENSE)。修改 MPL 涵蓋的原始檔並散佈時，需要依照 MPL-2.0 提供相應原始碼；這不會自動要求與 ResubWinny 組合的所有獨立模組採用 MPL。

第三方程式庫、字型、二進位元件和測試語料庫繼續遵循各自的授權條款與來源要求。Windows 二進位散佈必須同時滿足 libmpv 的 LGPL 對應原始碼與可替換動態程式庫要求。

安全性問題請依照[安全性政策](SECURITY.md)私下回報。貢獻的程式碼預設以 MPL-2.0 提供，具體要求請參閱[貢獻指南](CONTRIBUTING.md)。

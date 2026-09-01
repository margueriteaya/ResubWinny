[簡體中文（權威）](b62-compatibility.md) | [English](b62-compatibility.en.md) | [日本語](b62-compatibility.ja.md) | [繁體中文](b62-compatibility.zh-TW.md)

> 此為翻譯。簡體中文版本是唯一的權威來源。

# ARIB STD-B62 / ARIB-TTML 相容性

ResubWinny 將 ARIB-TTML 視為字幕資料格式，而不是瀏覽器 CSS。傳輸和 XML 解碼器與渲染器保持獨立，未知資產保留為原始證據，而不會被猜測為字幕。

面向觀看者的視覺基準是 [`libaribcaption` screenshot0](visual-reference.md)：B24 仍由 libaribcaption 渲染為 RGBA，而 B62 工作必須在不使用瀏覽器版面的情況下，收斂到相同的邏輯平面以及字型/注音/背景/描邊關係。

本專案將 `makeding/aribb62.js` 作為公開行為參考進行審查。所審查的 `74304d40a5b8556be1148e123ae70d60f937ecf5` 套件中繼資料宣告為 MIT，但該儲存庫和 GitHub 授權端點目前都未提供獨立的 `LICENSE` 檔案。因此，在可再散佈的著作權宣告和授權文字可用之前，ResubWinny 會把經獨立驗證的語意移植到 Rust 後端，並且不將其原始碼納入專案。尤其是，其面向瀏覽器的描邊渲染不被視為規範性的 ARIB 實作，不得被悄然提升為歸檔模型。

## 目前語意對應

| ARIB-TTML 關注項 | ResubWinny 行為 |
| --- | --- |
| `lrtb`, `rltb` | 正規化為 TTML `horizontal-tb`，並保留衍生的 `ltr`/`rtl` 方向，除非來源 `tts:direction` 明確覆寫它；原生預覽使用有界字元單元 RTL 放置，而非通用 Unicode 雙向文字塑形 |
| `tblr` | 正規化為 `vertical-lr` |
| `tbrl` | 正規化為 `vertical-rl` |
| `arib-tt:ruby` / `ruby` / `rt` | 保留在安全的行內 TTML 本文和歸檔記錄中；基本的水平原生預覽會將 `arib-tt:ruby` 注音 span 解析到其 `xml:id` 基文 span，並從行內本文渲染中移除該注音 |
| 繼承的 `div` 時間和樣式 | 在發出字幕區間之前解析 |
| 標準命名 TTML 色彩 | 除現有 `#RRGGBB[AA]` 支援外，還以不區分大小寫的方式原生解析 `black`、`white`、`red`、`green`、`blue`、`yellow`、`cyan`、`magenta` 和 `transparent`；不使用瀏覽器 CSS 色彩解析器 |
| 水平 `br`/換行、`textAlign`、`displayAlign`、`lineHeight` | 原生預覽保留明確換行，使用 `start`/`end`/`left`/`right`/`center` 配置每一條有界行，並使用 `before`/`center`/`after` 定位行區塊。`start` 和 `end` 遵循解析後的 LTR/RTL 方向。這是原生 RGBA 版面，不是瀏覽器後援方案 |
| 宣告的或有證據支援的顯示平面 | 有效的根 `tts:extent` 具有權威性，並被正規化到後端的邏輯 `1920×1080` 字幕平面。若無該值，邏輯 2K 仍為預設值；僅當完整畫素 `origin`/`extent` 幾何在至少一個軸上超過邏輯 2K 且仍處於相應平面範圍內時，解析器才推斷標準的 3840×2160 或 7680×4320。區域 origin/extent 使用獨立的水平/垂直縮放比例；畫素字型大小、行高、字母間距和直接輪廓寬度使用有界的統一縮放比例。因此，以等效 2K、4K 和 8K 製作的版面會佔據相同的觀看者相對區域；絕不猜測有歧義的輸入 |
| `subt://` 影象/字型和 `smpte:image` | 數字 `subt://<index>` 參照僅針對相同的 `packet_id + mpu_sequence_number` 資源狀態解析。當存在有界的 `subsampleNumber` 資源時，歸檔會寫入無損的 `resource_evidence` 記錄，該記錄以此範圍加子樣本編號為鍵，並保留資料型別、位元組長度、有界格式驗證和 base64 承載資料。歸檔預覽讀取器僅將相符的、小型且結構完整的 PNG 公開為低頻資源預覽；字型和非 PNG 資源仍是證據，而非渲染文字。缺失或不完整的對應仍明確標記為 `unresolved`。發現的 MPT 資產作為有界 `asset_evidence` 記錄發出，完整的非 `stpp` MPU/MFU 承載資料可由 `dump-tlv` 擷取為 `mmt_asset_payload` 原始證據，並帶有相符的範圍鍵 |
| 帶明確 `origin`/`extent` 的水平文字 | 後端可使用隨附的 Rounded M+ 1m ARIB 字型和來源前景/背景 RGBA，將其點陣化到有界的 1920×1080 RGBA 平面中。隨附字型缺失的字元會被計數並留空，而不會替換為豆腐塊或通用字元。這是一條初始原生預覽路徑，並非完整的 B62 渲染器 |
| `vertical-lr` / `vertical-rl` | 後端具有有界的原生直排模式：它垂直推進字元單元，在區域溢位時開啟新欄，並遵循左/右欄方向。當隨附 ARIB 字型中存在明確的 Unicode 直排呈現形式時，它會將標點對應到該形式。CJK/全形字元保持直立；ASCII 和拉丁字元使用原生順時針點陣圖旋轉，而未分類文字系統保持直立，不進行猜測。明確關聯的注音在其基文單元旁點陣化，包括跨自動換欄的有界延續（`ttml-vertical-ruby-basic-native`）。注音預設為基文字型大小的一半，但會保留其明確 `tts:color`、`tts:fontSize`、`tts:letterSpacing`、直接 opacity 以及支援的直接 `tts:textOutline`。包含一或兩個 ASCII 數字且直接設定 `tts:textCombine="all"` 或 `digits` 的 span 會在一個直排單元內水平點陣化；更長的序列仍保持直排。完整的 B62 方向表和特定於來源的注音放置仍有待合法語料庫比對。 |
| 安全的 `rich_body` span 樣式 | 有界 token 擷取會保留標籤之間的普通本文，並把每個來源 span 的明確前景色、字型大小、字母間距和直接 opacity 套用於原生文字預覽。明確關聯的注音文字（`tts:ruby="text"` 或 `arib-tt:ruby`）保持結構化而非行內化，並帶有其本身受支援的注音呈現屬性。 |
| 水平 `ruby` 基文/注音對 | 原生預覽將 `tts:ruby="text"` span 與緊鄰其前且連續的 `tts:ruby="base"` 群組關聯，或將 `arib-tt:ruby` 注音 span 與其 `xml:id` 基文 span 關聯；一條注音會在整個已解析的基文群組上置中。注音字型大小預設為基文字型大小的 0.5，而明確支援的注音色彩、字型大小、字母間距、opacity 和直接輪廓優先。快照報告 `ttml-horizontal-ruby-basic-native` 以及已渲染注音計數。非連續/重疊且特定於來源的 B62 注音放置仍保留為中繼資料，直到語料庫比對證明某種放置規則。 |
| 直接 TTML `tts:textOutline` | 保守的原生預覽對應接受直接 TTML 命名色彩或 `#RRGGBB`/`#RRGGBBAA` 加一個 `px` 寬度，接受 `none`，將半徑限制為 1–4 畫素，並套用繼承的 opacity。未重複宣告輪廓的 Rounded M+/`丸ゴシック` 字幕使用接收器基準的 2 px 黑色描邊，並由原生 PNG golden 保護；明確 `none` 會停用該描邊。不支援的語法仍為中繼資料，而不會變成虛構的輪廓 |
| `arib-tt:border` 和瀏覽器描邊 CSS | 不自動轉換為 `tts:textOutline`；這避免宣稱非標準輪廓等價性 |
| 未知書寫模式或擴充 | 保留為來源樣式中繼資料，並透過診斷/原始路徑報告 |

ASS 仍是一種近似表示。它可以保留位置、色彩、字型大小和部分文字樣式，但並不是 B62 書寫、注音、動畫、點陣圖資源或廣播描邊語意的無損表示。

## 計畫增量

1. 將已實作的有界注音分組和保守直排方向路徑與合法 B62 擷取進行比較；只擴充語料庫所證明的規則。
2. 在將已實作的接收器基準描邊 golden 擴充到任何其他字型系列或語法之前，將其與使用者驗證的 ARIB 擷取進行比較；絕不從瀏覽器 `text-shadow` 或 `-webkit-text-stroke` 推斷這些擴充。
3. 為目前 B24 RGBA 合成器和基本水平注音 TTML 平面保留原生視覺 golden；只有在能夠與合法參考擷取進行比較時，才新增包含巢狀時間、直排注音、資源 URL 和不支援擴充的 B62 fixture。

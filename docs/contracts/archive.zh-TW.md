[簡體中文](archive.md) · [繁體中文](archive.zh-TW.md) · [日本語](archive.ja.md) · [English](archive.en.md)

> 本頁是譯文。簡體中文版本是唯一權威來源。

# 字幕 archive 合約

字幕 archive 採用 UTF-8 JSON Lines（`.caption.jsonl`）格式，是專案的持久中間表示；
可從中衍生 ASS、TTML 與預覽輸出，而不把這些呈現格式視為無損格式。

## 檔頭與 schema 版本

第一行完整記錄是 archive 檔頭：

```json
{"type":"arib_caption_studio_archive","schemaVersion":1,"version":1,"source":"recording.ts","route":"arib_std_b24","format":"jsonl"}
```

`schemaVersion` 是權威的 archive 相容性版本。版本 1 也會把原有 `version` 欄位寫成相容別名；
兩個值必須一致。新的寫入器不得在未遞增 `schemaVersion` 時，默默改變既有記錄的語意或結構。

只需要有界時間軸或預覽記錄的讀取器可以忽略未知記錄型別。需要完整語意保真度的讀取器必須拒絕
不支援的 `schemaVersion`，不能自行猜測。明確 `schemaVersion` 欄位引入前產生的檔案使用
`version: 1`，仍屬於版本 1 archive。

## 記錄

之後每一個完整行都是帶有穩定 `type` 的獨立 JSON 物件。字幕 payload 記錄使用
`{"type":"caption","value":{...}}` 形式的 envelope；其他現有型別包括 `region_interval`、
`scene`、`resource_reference`、`resource_evidence`、`asset_evidence` 與 `summary`。

轉換執行期間，寫入器會 flush 完整字幕記錄，讓桌面端可以追蹤讀取檔案。讀取器必須忽略不完整的
最後一行，直到後續附加使其完整。B24 與 B62 的傳輸專屬證據保持分離；共通語意透過字幕記錄表達，
而不假裝兩種傳輸共用同一個解碼器模型。

在 Worker 內，兩條路線都要先跨越封閉、零複製的 `CaptionCueRef` 語意邊界，再釋出到 archive。
它統一時間、區域、路線識別、純文字、ruby 數量與 DRCS 存在性，同時保留每條路線的忠實 payload。
樣式、字形畫素與 TTML 資源證據仍為路線專屬。因此 schema v1 繼續把 B24 釋出為
`region_interval`、把 ARIB-TTML 釋出為 `caption`；共享的內部邊界不會重新命名或複製記錄。

ARIB-TTML `caption.value` 可帶可選的 `source_layout`。它保留來源顯示平面的寬高與判定依據
（`declared`、`inferred` 或 `legacy_logical2k`）、來源 region 幾何、未縮放樣式和安全的行內 TTML；
既有的 `x`、`y`、`width`、`height`、`style` 與 `rich_body` 繼續保留為邏輯 1920×1080 相容檢視。
新讀取器優先使用 `source_layout` 映射至實際視訊內容 viewport；沒有該欄位的舊 archive 按
`LegacyLogical1920x1080` 解釋。schema v1 讀取器必須忽略此可選欄位，因此不要求遷移舊檔案；
曾被錯誤縮放且未保存來源語意的 archive 無法可靠反推，只能從合法來源錄製重新擷取。

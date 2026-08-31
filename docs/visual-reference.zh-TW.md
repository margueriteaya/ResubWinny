# 視覺參考基準

[简体中文](visual-reference.md) · [繁體中文](visual-reference.zh-TW.md) · [日本語](visual-reference.ja.md) · [English](visual-reference.en.md)

> **規範性聲明：** 簡體中文版本是唯一的權威來源。其他語言版本均為同步翻譯；若措辭有歧義或衝突，以簡體中文版本為準。

ResubWinny 使用 libaribcaption 發布的字幕螢幕截圖，作為共用 B24/B62 預覽設定檔面向觀眾的主要參考：

- 來源：<https://github.com/xqq/libaribcaption/raw/master/screenshots/screenshot0.png>
- 儲存庫內建參考：`third_party/libaribcaption/screenshots/screenshot0.png`
- 尺寸：`1920×1080`
- SHA-256：`3115B9B125AFA7CDF6F41D3D0155476CD18134021CDD05A55C8C65E749A403F6`

該圖確立了預期的電視端呈現結果：1920×1080 邏輯字幕平面、支援 ARIB 的字型選擇、獨立定位的文字區域、來源中的前景色／背景色／筆畫色彩，以及在視覺上持續與其基底文字相連的注音。它不是 B62 傳輸測試資料，也不構成對該圖中未呈現的 B62 功能進行猜測的依據。

## 實作契約

B24 路徑具有權威性：專案自有的 C ABI 要求 libaribcaption 使用 `Rounded M+ 1m for ARIB` 直接產生 RGBA，並啟用注音與背景、停用 DRCS 取代、合併區域，同時使用 `2.0` 的算繪器筆畫寬度。ResubWinny 在其封存與原生預覽合成器中保持該影像不變；Svelte UI 與瀏覽器文字引擎都不會重新繪製該影像。

B62/ARIB-TTML 原生算繪器必須以相同的觀眾視覺關係為目標，而不是採用另一套視覺語言。其 2K/4K/8K 座標會正規化至同一個 1920×1080 邏輯平面。它針對橫排注音、直排注音、直排標點，以及播送內容未重複提供直接輪廓宣告時所使用的 Rounded M+ 接收器基準黑色筆畫，設有視覺黃金樣本。明確的 `tts:textOutline="none"` 仍具有權威性。刻意不將與這張 B24 螢幕截圖逐像素比較作為驗收測試：該截圖沒有對應的 B62 來源 TTML、時間資訊、區域中繼資料或樣式酬載。新的 B62 語意必須具備合法來源的樣本與參考擷取畫面，才能標記為已驗證。

## 審查規則

變更 B24 橋接設定、字幕平面合成、字型資源或 B62 文字／注音／筆畫版面配置時，應將此影像與受影響的 PNG 黃金樣本一併審查。不得以 WebView CSS 補償、替換成通用字型，或僅憑合成範例宣稱視覺一致。

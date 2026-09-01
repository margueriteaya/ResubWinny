# ResubWinny App 圖示

[简体中文](app-icon.md) · [繁體中文](app-icon.zh-TW.md) · [日本語](app-icon.ja.md) · [English](app-icon.en.md)

> **規範性聲明：** 簡體中文版本是唯一的權威來源。其他語言版本均為同步翻譯；若措辭有歧義或衝突，以簡體中文版本為準。

Apple 平台的標準來源是供 Icon Composer 使用的分層圖稿。其中不得包含平台遮罩、模擬的 Liquid Glass、斜面、光暈、模糊邊緣、鏡面高光或圖層間陰影。

## 來源圖層

所有來源圖稿均為 1024 x 1024 的正方形，位於 `studio-tauri/src-tauri/icons/source/apple/`：

- `01-background.svg`：滿版不透明背景。
- `02-broadcast-plane.svg`：16:9 播放平面的前景圖層。
- `03-captions.svg`：字幕區域、注音儲存格，以及獨立的側邊區域。
- `04-composite-preview.svg`：僅供無遮罩預覽；請勿將其匯入為 Icon Composer 的分層來源。

依上述順序將前三個圖層匯入 Icon Composer。保持兩個前景 SVG 完全不透明，並在 Icon Composer 中調整半透明度、折射、鏡面高光與陰影。系統會為 Default、Dark、Clear 與 Tinted 外觀提供平台遮罩及動態效果。

Tauri 的平面備援圖示是 `icons/source/flat-app-icon.svg`。其圓角背景是特別為不會套用 Apple 系統遮罩的平台所設計。它不是 Icon Composer 的來源圖層。

## 幾何規格

- 畫布：1024 x 1024。
- 播放平面：620 x 348.75，比例精確為 16:9，置於畫布中央。
- 主要內容保持在 watchOS 的圓形裁切範圍內。
- 前景邊緣為實邊，不作羽化處理。
- 不包含文字、平台硬體、螢幕截圖或複製的應用程式 UI。

## 重新產生

從 `studio-tauri/` 重新產生跨平台 Tauri 備援圖示：

```powershell
npm run tauri -- icon src-tauri/icons/source/flat-app-icon.svg
```

## 參考資料

- Apple Human Interface Guidelines：App icons
  <https://developer.apple.com/design/human-interface-guidelines/app-icons/>
- Apple Icon Composer
  <https://developer.apple.com/icon-composer/>
- Apple Design Resources license
  <https://developer.apple.com/apple-design-resources-license/>

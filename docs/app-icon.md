# ResubWinny 应用图标

[简体中文](app-icon.md) · [繁體中文](app-icon.zh-TW.md) · [日本語](app-icon.ja.md) · [English](app-icon.en.md)

> **规范性声明：**简体中文版本是唯一的权威来源。其他语言版本均为同步翻译；若措辞存在歧义或冲突，以简体中文版本为准。

Apple 平台的规范源文件是供 Icon Composer 使用的分层图稿。其中不得包含平台蒙版、模拟的 Liquid Glass、斜面、辉光、模糊边缘、镜面高光或图层间阴影。

## 源图层

所有源图稿均为 1024 x 1024 的正方形，位于 `studio-tauri/src-tauri/icons/source/apple/`：

- `01-background.svg`：全出血不透明背景。
- `02-broadcast-plane.svg`：16:9 播放平面的前景图层。
- `03-captions.svg`：字幕区域、注音单元格和独立侧边区域。
- `04-composite-preview.svg`：仅供无蒙版预览；请勿将其作为 Icon Composer 分层源文件导入。

按上述顺序将前三个图层导入 Icon Composer。保持两个前景 SVG 完全不透明，并在 Icon Composer 中调整半透明度、折射、镜面高光和阴影。系统会为 Default、Dark、Clear 和 Tinted 外观提供平台蒙版与动态效果。

Tauri 的扁平后备图标是 `icons/source/flat-app-icon.svg`。其圆角背景是为不会应用 Apple 系统蒙版的平台特意设计的。它不是 Icon Composer 源图层。

## 几何规格

- 画布：1024 x 1024。
- 播放平面：620 x 348.75，比例精确为 16:9，位于画布中央。
- 主要内容保持在 watchOS 的圆形裁剪范围内。
- 前景边缘为实边，不作羽化处理。
- 不包含文字、平台硬件、屏幕截图或复刻的应用程序 UI。

## 重新生成

在 `studio-tauri/` 中重新生成跨平台 Tauri 后备图标：

```powershell
npm run tauri -- icon src-tauri/icons/source/flat-app-icon.svg
```

## 参考资料

- Apple Human Interface Guidelines：App icons
  <https://developer.apple.com/design/human-interface-guidelines/app-icons/>
- Apple Icon Composer
  <https://developer.apple.com/icon-composer/>
- Apple Design Resources license
  <https://developer.apple.com/apple-design-resources-license/>

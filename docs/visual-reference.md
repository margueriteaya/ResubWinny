# 视觉参考基线

[简体中文](visual-reference.md) · [繁體中文](visual-reference.zh-TW.md) · [日本語](visual-reference.ja.md) · [English](visual-reference.en.md)

> **规范性声明：**简体中文版本是唯一的权威来源。其他语言版本均为同步翻译；若措辞存在歧义或冲突，以简体中文版本为准。

ResubWinny 使用 libaribcaption 发布的字幕屏幕截图，作为共享 B24/B62 预览配置面向观看者的主要参考：

- 来源：<https://github.com/xqq/libaribcaption/raw/master/screenshots/screenshot0.png>
- 仓库内置参考：`third_party/libaribcaption/screenshots/screenshot0.png`
- 尺寸：`1920×1080`
- SHA-256：`3115B9B125AFA7CDF6F41D3D0155476CD18134021CDD05A55C8C65E749A403F6`

该图确立了预期的电视端呈现效果：1920×1080 逻辑字幕平面、支持 ARIB 的字体选择、独立定位的文本区域、来源中的前景色/背景色/描边色，以及在视觉上始终与基准文本关联的注音。它不是 B62 传输夹具，也不构成对该图中未出现的 B62 功能进行猜测的依据。

## 实现契约

B24 路径具有权威性：项目自有的 C ABI 要求 libaribcaption 使用 `Rounded M+ 1m for ARIB` 直接生成 RGBA，并启用注音和背景、禁用 DRCS 替换、合并区域，同时使用 `2.0` 的渲染器描边宽度。ResubWinny 在其存档和原生预览合成器中保持该图像不变；Svelte UI 和浏览器文本引擎均不会重新绘制该图像。

B62/ARIB-TTML 原生渲染器必须以相同的观看者视觉关系为目标，而不是采用另一套视觉语言。其 2K/4K/8K 坐标会归一化到同一个 1920×1080 逻辑平面。它为横排注音、竖排注音、竖排标点，以及广播未重复提供直接轮廓声明时所使用的 Rounded M+ 接收机基线黑色描边设置了视觉黄金样本。显式的 `tts:textOutline="none"` 仍具有权威性。特意不将与这张 B24 屏幕截图逐像素对比作为验收测试：该截图没有对应的 B62 源 TTML、定时、区域元数据或样式载荷。新的 B62 语义必须具备合法来源的样本和参考捕获，方可标记为已验证。

## 审查规则

更改 B24 桥接设置、字幕平面合成、字体资源或 B62 文本/注音/描边布局时，应将此图像与受影响的 PNG 黄金样本一并审查。不得通过 WebView CSS 进行补偿，不得替换为通用字体，也不得仅凭合成示例宣称视觉一致。

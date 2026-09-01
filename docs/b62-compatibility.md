[简体中文（权威）](b62-compatibility.md) | [English](b62-compatibility.en.md) | [日本語](b62-compatibility.ja.md) | [繁體中文](b62-compatibility.zh-TW.md)

> 本简体中文版本是唯一的权威来源。其他语言版本均为翻译。

# ARIB STD-B62 / ARIB-TTML 兼容性

ResubWinny 将 ARIB-TTML 视为字幕数据格式，而不是浏览器 CSS。传输和 XML 解码器与渲染器保持独立，未知资源保留为原始证据，而不会被猜测为字幕。

面向观看者的视觉基准是 [`libaribcaption` screenshot0](visual-reference.md)：B24 仍由 libaribcaption 渲染为 RGBA，而 B62 工作必须在不使用浏览器布局的情况下，收敛到相同的逻辑平面以及字体/注音/背景/描边关系。

本项目将 `makeding/aribb62.js` 作为公开行为参考进行审查。所审查的 `74304d40a5b8556be1148e123ae70d60f937ecf5` 软件包元数据声明为 MIT，但该仓库和 GitHub 许可证端点目前都未提供独立的 `LICENSE` 文件。因此，在可再分发的版权声明和许可证文本可用之前，ResubWinny 会把经独立验证的语义移植到 Rust 后端，并且不将其源代码纳入项目。尤其是，其面向浏览器的描边渲染不被视为规范性的 ARIB 实现，不得被悄然提升为归档模型。

## 当前语义映射

| ARIB-TTML 关注项 | ResubWinny 行为 |
| --- | --- |
| `lrtb`, `rltb` | 规范化为 TTML `horizontal-tb`，并保留派生的 `ltr`/`rtl` 方向，除非源 `tts:direction` 明确覆盖它；原生预览使用有界字符单元 RTL 放置，而非通用 Unicode 双向文本塑形 |
| `tblr` | 规范化为 `vertical-lr` |
| `tbrl` | 规范化为 `vertical-rl` |
| `arib-tt:ruby` / `ruby` / `rt` | 保留在安全的内联 TTML 正文和归档记录中；基本的水平原生预览会将 `arib-tt:ruby` 注音 span 解析到其 `xml:id` 基文 span，并从内联正文渲染中移除该注音 |
| 继承的 `div` 时间和样式 | 在发出字幕区间之前解析 |
| 标准命名 TTML 颜色 | 除现有 `#RRGGBB[AA]` 支持外，还以不区分大小写的方式原生解析 `black`、`white`、`red`、`green`、`blue`、`yellow`、`cyan`、`magenta` 和 `transparent`；不使用浏览器 CSS 颜色解析器 |
| 水平 `br`/换行、`textAlign`、`displayAlign`、`lineHeight` | 原生预览保留显式换行，使用 `start`/`end`/`left`/`right`/`center` 布局每一条有界行，并使用 `before`/`center`/`after` 定位行块。`start` 和 `end` 遵循解析后的 LTR/RTL 方向。这是原生 RGBA 布局，不是浏览器回退方案 |
| 声明的或有证据支持的显示平面 | 有效的根 `tts:extent` 具有权威性，并被归一化到后端的逻辑 `1920×1080` 字幕平面。若无该值，逻辑 2K 仍为默认值；仅当完整像素 `origin`/`extent` 几何在至少一个轴上超过逻辑 2K 且仍处于相应平面范围内时，解析器才推断规范的 3840×2160 或 7680×4320。区域 origin/extent 使用独立的水平/垂直缩放比例；像素字体大小、行高、字母间距和直接轮廓宽度使用有界的统一缩放比例。因此，以等效 2K、4K 和 8K 创作的布局会占据相同的观看者相对区域；绝不猜测有歧义的输入 |
| `subt://` 图像/字体和 `smpte:image` | 数字 `subt://<index>` 引用仅针对相同的 `packet_id + mpu_sequence_number` 资源状态解析。当存在有界的 `subsampleNumber` 资源时，归档会写入无损的 `resource_evidence` 记录，该记录以此作用域加子样本编号为键，并保留数据类型、字节长度、有界格式验证和 base64 载荷。归档预览读取器仅将匹配的、小型且结构完整的 PNG 公开为低频资源预览；字体和非 PNG 资源仍是证据，而非渲染文本。缺失或不完整的映射仍明确标记为 `unresolved`。发现的 MPT 资源作为有界 `asset_evidence` 记录发出，完整的非 `stpp` MPU/MFU 载荷可由 `dump-tlv` 提取为 `mmt_asset_payload` 原始证据，并带有匹配的作用域键 |
| 带显式 `origin`/`extent` 的水平文本 | 后端可使用捆绑的 Rounded M+ 1m ARIB 字体和源前景/背景 RGBA，将其光栅化到有界的 1920×1080 RGBA 平面中。捆绑字体缺失的字形会被计数并留空，而不会替换为豆腐块或通用字形。这是一条初始原生预览路径，并非完整的 B62 渲染器 |
| `vertical-lr` / `vertical-rl` | 后端具有有界的原生竖排模式：它垂直推进字符单元，在区域溢出时开启新列，并遵循左/右列方向。当捆绑 ARIB 字体中存在明确的 Unicode 竖排展示形式时，它会将标点映射到该形式。CJK/全角字形保持直立；ASCII 和拉丁字形使用原生顺时针位图旋转，而未分类文字体系保持直立，不进行猜测。明确关联的注音在其基文单元旁光栅化，包括跨自动换列的有界延续（`ttml-vertical-ruby-basic-native`）。注音默认为基文字体大小的一半，但会保留其显式 `tts:color`、`tts:fontSize`、`tts:letterSpacing`、直接 opacity 以及受支持的直接 `tts:textOutline`。包含一或两个 ASCII 数字且直接设置 `tts:textCombine="all"` 或 `digits` 的 span 会在一个竖排单元内水平光栅化；更长的序列仍保持竖排。完整的 B62 方向表和特定于来源的注音放置仍有待合法语料库比对。 |
| 安全的 `rich_body` span 样式 | 有界 token 提取会保留标签之间的普通正文，并把每个源 span 的显式前景色、字体大小、字母间距和直接 opacity 应用于原生文本预览。明确关联的注音文本（`tts:ruby="text"` 或 `arib-tt:ruby`）保持结构化而非内联，并携带其自身受支持的注音呈现属性。 |
| 水平 `ruby` 基文/注音对 | 原生预览将 `tts:ruby="text"` span 与紧邻其前且连续的 `tts:ruby="base"` 组关联，或将 `arib-tt:ruby` 注音 span 与其 `xml:id` 基文 span 关联；一条注音会在整个已解析的基文组上居中。注音字体大小默认为基文字体大小的 0.5，而显式支持的注音颜色、字体大小、字母间距、opacity 和直接轮廓优先。快照报告 `ttml-horizontal-ruby-basic-native` 以及已渲染注音计数。非连续/重叠且特定于来源的 B62 注音放置仍保留为元数据，直到语料库比对证明某种放置规则。 |
| 直接 TTML `tts:textOutline` | 保守的原生预览映射接受直接 TTML 命名颜色或 `#RRGGBB`/`#RRGGBBAA` 加一个 `px` 宽度，接受 `none`，将半径限制为 1–4 像素，并应用继承的 opacity。未重复声明轮廓的 Rounded M+/`丸ゴシック` 字幕使用接收器基准的 2 px 黑色描边，并由原生 PNG golden 保护；显式 `none` 会禁用该描边。不受支持的语法仍为元数据，而不会变成虚构的轮廓 |
| `arib-tt:border` 和浏览器描边 CSS | 不自动转换为 `tts:textOutline`；这避免宣称非标准轮廓等价性 |
| 未知书写模式或扩展 | 保留为源样式元数据，并通过诊断/原始路径报告 |

ASS 仍是一种近似表示。它可以保留位置、颜色、字体大小和部分文本样式，但并不是 B62 书写、注音、动画、位图资源或广播描边语义的无损表示。

## 计划增量

1. 将已实现的有界注音分组和保守竖排方向路径与合法 B62 捕获进行比较；只扩展语料库所证明的规则。
2. 在将已实现的接收器基准描边 golden 扩展到任何其他字体系列或语法之前，将其与用户验证的 ARIB 捕获进行比较；绝不从浏览器 `text-shadow` 或 `-webkit-text-stroke` 推断这些扩展。
3. 为当前 B24 RGBA 合成器和基本水平注音 TTML 平面保留原生视觉 golden；只有在能够与合法参考捕获进行比较时，才添加包含嵌套时间、竖排注音、资源 URL 和不受支持扩展的 B62 fixture。

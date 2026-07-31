# 架构基线（中文规范）

> 当前桌面实现为 Tauri 2 + Svelte 5；文中历史版本记录中的 Slint 仅作历史说明，不代表当前架构。第三阶段核心已经落地：B62/受限 TLV 资源证据、同 MPU PNG 资源到 archive/低频预览的接线、职责拆分、archive 时间点预览、B24 原生字幕平面合成、TTML 横排与竖排文字光栅化、连续 base ruby 分组、跨列竖排 ruby、保守的 CJK/全角正立与拉丁字符旋转、TS/PSI/PES/B24 fuzz target、Windows `libmpv-render`、真实 4K 录制样本的阈值化长时性能门槛以及 Windows/macOS/Linux CI 检查矩阵已完成。标准 B62 描边、资源完整预览、独立 2K/8K 与 DPI/截图差分仍属于质量收敛项。Windows 是当前 Alpha 的原生预览发布平台；macOS/Linux 原生预览后端已明确延期，不属于当前阶段验收范围。WGL 零拷贝硬解互操作不是当前产品承诺。

> 本文件是项目的唯一规范性架构文档。英语与日语版本是同步译文；歧义或冲突一律以本文件为准。

## 1. 项目定位与边界

本项目是面向日本 ISDB 广播录制文件的开源、跨平台字幕抽出与转换工具。传输层主线必须区分传统 MPEG-2 TS 与 BS4K/8K 原生 TLV/MMT；`.ts`、`.m2ts`、`.tlv`、`.mmts` 只作为文件名提示，最终一律按内容探测。当前可验证语料包括传统 TS 与 192-byte MPEG-TS/TTML 录制；原生 TLV/MMT 保留为规范主线和实验性输入，直到获得足够真实样本后再宣称完整支持。所有路径都尽可能保留 ARIB 字幕的语义、版式、特殊字符和诊断来源。

它不是录像管理器、媒体播放器、视频/音频解码器、EPG 浏览器、CAS/加扰处理器、通用 MMT 媒体框架或网络直播接收器。重点始终是字幕抽出、恢复、转换、存档与诊断。

旧工具、`bs4kass.exe` 与 Caption2Ass 只可用于公开资料研究和黑箱比较；不得复制其非公开实现，也不得将它们打包到发布产物中。

## 2. 已确认的架构

当前 worker 的 `main.rs` 仅调用 `lib.rs` 暴露的 `run()`；所有模块注册、共享导出和测试入口均位于 `lib.rs`，因此 CLI 入口与转换核心已经可以独立复用。

```text
Tauri 2 + Svelte 5 桌面界面（WebView 只负责展示）
  | 后台任务、低频进度、取消与诊断
  v
共享 Rust 转换核心（GUI/CLI 同一实现）
  | 有界顺序读取、解析、时间轴、原子提交
  v
项目字幕模型与导出器
  | 薄且稳定的 C ABI
  v
libaribcaption（第一代 ARIB STD-B24 解码/可选渲染后端）
```

Worker 已按职责拆分为 `cli.rs`、`protocol.rs`、`inspection.rs`、`jobs.rs`、`preview.rs`、`archive.rs`、`resource.rs`、`config.rs`、`transport/mpeg_ts.rs`、`transport/m2ts.rs`、`transport/tlv_mmt.rs`、`caption/b24.rs`、`caption/ttml.rs`、`timeline.rs`、`drcs.rs` 和 `exporters/`；`main.rs` 只保留进程入口、模块注册和测试入口，解析实现与配置常量不再堆叠于此。M2TS 的 192-byte packetisation、track discovery 和 route façade 已归入 `transport/m2ts.rs`，TTML 文档语义仍由 `caption/ttml.rs` 负责。`archive.rs` 提供 CLI 与桌面后端共用的有界 archive 时间点快照路径，`resource.rs` 负责 B62 资源证据，`transport/tlv_mmt.rs` 负责 TLV/MMTP 基础包与受限 stpp 路由。通用 TLV/MMT 字幕语义仍明确标记为未完成。

GUI 绝不能成为唯一入口，也不得在 UI 线程读取录制字节、接收每个 TS 包、保存完整字幕时间线、承担解复用或最终排版。转换核心必须可被单独以 CLI 调用。当前界面在后台线程运行同一核心，提供协作式取消、进度与原子输出；需要跨进程崩溃隔离时再增加 sidecar，而不为此牺牲单一 EXE 发布。

本地大文件默认使用 Rust `File`、`BufReader`、`Read`、`Seek` 等阻塞顺序 I/O；不要为每个 188 字节 TS 包建立异步任务或跨 channel 传递。Tokio 仅在 IPC、调度、网络输入或并行独立任务确有需要时引入。

## 3. 大文件与恢复约束

- 文件大小不得决定常态内存占用：1 GB 与 200 GB 输入应保持近似的资源规模。
- 默认不得整体读入、整体解复用、缓存全部 TS 包、累计全部字幕事件、建立细粒度全量索引，也不得由前端处理广播文件。
- 输入缓冲、重同步窗口、每 PID 的 PES 缓冲、每 asset 的 MPU 缓冲和活动字幕场景均须有硬上限；不可信长度字段不得直接导致任意分配。
- 目标路径是：固定大小缓冲 -> 容器同步/探测 -> TS/TLV/MMTP 流式解析 -> 仅保留目标服务与字幕 PID/asset -> data group/MPU 重组 -> 解码 -> 场景变化 -> 增量导出。
- 首次扫描只识别容器、服务、轨道和必要时间基准；视频、音频不解码也不需完整 PES 重组。
- 优先借用输入切片。仅在跨包 PES、data group、MMTP fragment 或需长期持有时复制；DRCS 按哈希去重。不要为了消灭最后几 KB 复制而写复杂生命周期结构。
- 默认不用全文件 mmap；可在未来作为平台特定优化。读取器应可扩展支持本地文件、stdin/管道、分卷文件与增长中的录制文件。
- 周期性 checkpoint 至少包含文件身份（大小、mtime、首尾块哈希）、byte offset、continuity、PTS unwrap、当前 B24 management/DRCS 状态和导出安全位置。恢复优先回退到可靠同步点并重解析一小段，而不是假定可在任意字节恢复解码器状态。
- 输出先写 `.part`、临时 events、DRCS 目录和 checkpoint；成功后原子发布。失败或取消时保留日志、恢复信息和明确的未完成标记。阻止睡眠为手动、默认关闭、仅在任务运行时生效。

## 4. 输入路由

传统地上波/BS/CS 的 MPEG-2 TS，以及被重新封装或以 192-byte 形式保存的 MPEG-TS 录制：

```text
MPEG-2 TS -> PAT/PMT -> subtitle PES -> ARIB STD-B24 data groups
```

必须保存服务、PID、语言、caption/superimpose 类型、PCR/PTS/DTS、源文件偏移、不连续和解码警告。192-byte packetisation 只说明录制封装形态，不代表 BS4K/8K 原生传输层；真实 route 仍由内容探测和 PSI/MMT 信令决定。

对 MPEG-TS，B24 caption PID 仍是优先的已验证路线。若 PSI/PMT 仅发现 private data PID，worker 可用同一有界 PES 重组器寻找完整 ARIB-TTML XML；只有 XML 边界、BOM/声明编码与 TTML 文档均通过严格验证时才进入 TTML 模型。private PID 本身不是字幕证明，未识别或不完整 payload 必须保留为原始证据或诊断，不能猜测转换。

### 4.0 现实输入优先级

当前可执行计划按证据强度排序：

1. **已验证主线：** 188-byte MPEG-TS + ARIB STD-B24，以及 192-byte MPEG-TS packetisation + 私有 PES + ARIB-TTML。两者均有本地长样本、流式计数基线和导出回归。
2. **规范主线、实验性实现：** 原生 BS4K/8K `TLV -> IP/UDP -> MMTP -> MPT/MPU`。当前只有构造/单元证据和受限 `stpp` 路由；真实 TLV 样本不足，只能提供探测、诊断、原始证据和明确条件下的转换。
3. **禁止的判断：** 不能从 `.ts`、`.m2ts` 或 `.tlv` 扩展名推断传输格式，也不能把 192-byte MPEG-TS 文件自动称为原生 BS4K/8K TLV。

BS4K/8K：

```text
TLV -> IPv6/压缩 IP -> UDP -> MMTP -> signalling -> caption asset -> MPU
```

首期 BS4K/8K 仅处理录制文件：找到 MMT package、识别字幕 asset、重组字幕相关 MPU、恢复时间戳、将有效载荷交给统一字幕核心。不得在此阶段实现 HEVC/音频解码、完整 SI/EPG、CAS、直播或通用 MMT 框架。该模块按协议实现项目估算，不能视为“增加一个扩展名”。

输入探测层必须区分 MPEG-2 TS、TLV、MMTP、损坏/截断流；不得用文件扩展名替代检测。

### 4.1 信号与 ARIB 规范对照

下表是实现时引用的规范层次，不是“只要文件扩展名相同便可按同一路径处理”的规则。版本号记录的是 2026-07 查到的 ARIB 最新公开目录；实际解析以录制流内的信令、描述符和有效载荷为准。

| 信号类别 | 物理/传输体系 | 服务与轨道发现 | 字幕编码与呈现 | 本项目的解复用入口 |
| --- | --- | --- | --- | --- |
| 地上波 2K（ISDB-T） | ARIB STD-B31，地上数字电视传送方式；录制层通常为 MPEG-2 TS | MPEG-2 PSI 与 ARIB STD-B10 的 SI | ARIB STD-B24 的字幕/文字スーパー数据；B24 data group 由字幕 PES 送达 | PAT/PMT -> 目标 subtitle PES -> B24 data group |
| BS/广带 CS 2K | ARIB STD-B20，卫星数字放送传送方式；录制层通常为 MPEG-2 TS | MPEG-2 PSI 与 ARIB STD-B10 的 SI | ARIB STD-B24；同样不能把 `stream_type` 或 component tag 的单一经验规则当成完整规范 | PAT/PMT -> 目标 subtitle PES -> B24 data group |
| BS4K/8K（高度广带卫星数字放送/ISDB-S3） | ARIB STD-B44 定义 ISDB-S3 传送方式，含 TLV；媒体传送由 ARIB STD-B60 的 MMT 体系规定 | MMT signalling、package/asset 与描述符 | ARIB STD-B62 第一编第三部规定第二代字幕/文字スーパー编码，包含 ARIB-TTML 体系 | TLV -> IP/UDP -> MMTP -> signalling -> caption asset/MPU -> 由描述符识别的字幕格式 |

关键修正：BS4K/8K 不能仅凭“4K/8K”就假定有效载荷必然为 ARIB-TTML。ARIB STD-B60 的后续说明明确字幕数据格式由 caption-description method 标识；实现必须读取实际 signalling/descriptor，并把 ARIB-TTML、可能的 B24 兼容/其他标识和未知格式分别路由、报告或保留原始数据。`*.m2ts` 的 192-byte 包封装也只是录制器常见文件表示，不能替代对 TS/TLV/MMT 内容的判断。

ARIB STD-B24 是传统数字放送的数据编码与传送规范；ARIB STD-B10 是补充 MPEG-2 PSI 的服务信息规范，不是字幕字形/排版规范。ARIB STD-B62 面向高度广带卫星数字放送，其第一编第三部负责字幕与文字スーパー编码；ARIB STD-B60 则规定 MMT 媒体传送。物理层/传送层、服务信令层与字幕编码层必须分开实现和测试。

规范入口（只记录编号、范围与链接，不转载受版权保护的标准正文）：

- [ARIB STD-B31](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b31.html)：地上数字电视传送；
- [ARIB STD-B20](https://www.arib.or.jp/english/std_tr/broadcasting/std-b20.html)：卫星数字放送传送，覆盖 BS 数字与广带 CS 数字；
- [ARIB STD-B10](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b10.html)：数字放送服务信息；
- [ARIB STD-B24](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b24.html)：数字放送数据编码与传送；
- [ARIB STD-B44](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b44.html)：ISDB-S3/高度广带卫星数字放送传送；
- [ARIB STD-B60](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b60.html)：MMT 媒体传送；
- [ARIB STD-B62](https://www.arib.or.jp/english/std_tr/broadcasting/desc/std-b62.html)：第二代多媒体编码，第一编第三部为字幕/文字スーパー编码。

## 5. 字幕真相模型

内部真相不是 ASS，也不是“开始、结束、一段文本”的单 cue 列表。ARIB 字幕是对字幕平面与独立区域的时间操作。模型分两层：

```text
TimedCaptionOperation { pts, operation }
  ClearScreen | ClearRegion | SetCursor | SetStyle | WriteText |
  WriteDrcs | BeginRuby | EndRuby | DefineDrcs | ...

CaptionPlaneState -> closed RegionInterval / CaptionScene
```

一个 `RegionInterval` 必须有确定的 begin/end、layer、几何和样式化内容。多个区域可并行出现、分别更新和消失；不得把不同生命周期粗暴合并成一条字幕。

模型至少保存：原始与展开后的 PTS/DTS/PCR、normalized time、source packet offset、management data、language tag、TCS、clear-screen、repeat/roll-up、平面尺寸、区域与字符级样式、横竖排、ruby、enclosure、DRCS、无法表达的控制码、原始 payload（存档要求时）及全部警告。

时间轴不得只信 PTS。需要处理开头裁切、PCR 跳变、discontinuity、wrap-around、丢包、清屏丢失、多服务混录、节目切换后的 PTS 重置和无显式结束时间。提供严格 PTS、自动修复、视频/字幕零点、手动全局偏移及结束时间推断等明确模式；不得无条件把下一条字幕开始时间作为上一条结束时间。

导出器只在区域被覆盖、清除或全屏清除时封闭并写出；文件结束时封闭剩余区域。这样输入和内存保持流式，同时支持交错时间轴。头部依赖后续样式/DRCS 的 ASS/TTML 可使用临时 body/events 文件后组装，不得为此重读广播文件或缓存整个时间线。

## 6. 导出与 DRCS

正式保真转换目标：ASS、TTML、ARIB-TTML、项目原生存档格式。ASS 是高兼容的视觉近似，不是无损格式；它需要将状态变化展开为重叠 Dialogue，并对竖排、ruby、闪烁、特殊装饰和复杂 DRCS 明示近似限制。TTML 应区分内部完整表达、IMSC 兼容模式和 ARIB-TTML 兼容模式，不能为了校验静默删除结构。

SRT、普通 WebVTT、TXT/CSV 只能位于“有损/文本提取”输出，不得被称为正式字幕转换，也不应在默认输出列表中。界面必须说明区域合并、时间切割和样式丢失规则。

存档输出包含字幕操作/场景 JSON、原始 data group/PES/MMT caption asset、DRCS PNG/SVG、PID/asset ID/PTS 和诊断信息；这是唯一承诺尽可能可逆的长期交换格式。

DRCS 策略按优先级执行：

1. 使用可证明的标准 Unicode 映射；
2. 仅在记录映射且经用户认可时使用通行替代字；
3. 否则导出字形资源并作为视觉元素引用；
4. ASS 可选临时字体或矢量/位图策略。

不得静默丢弃、猜测或输出 `[外:<hash>]` 占位。GUI 应提供特殊字符检查器：原始字形、Unicode/替代、出现次数、首次时间和用户选择；用户修订写入本地 DRCS 字典。

## 7. libaribcaption、FFI 与渲染

不在第一阶段重写 B24 状态机。`libaribcaption` 负责字符集/控制码解释、DRCS 基础、区域和字符样式解析、现有行为参照及可选位图渲染；它不负责 TS/TLV/MMT、项目模型、全部时间轴、导出、checkpoint 或存档。

Rust 只能依赖项目维护的小型稳定 C ABI，而不是整套 C++ API/bindgen。FFI 边界重点审计对象生命周期、指针有效期、UTF-8、异常隔离、allocator 和跨平台构建/ABI 漂移；FFI 调用次数不是主要性能风险。

HTML/CSS 结构预览可显示文字、区域、时间、样式概况和 DRCS 占位。保真预览必须由 native renderer 输出 RGBA/PNG/WebP 低频快照；按 `render_at(time)` 请求或在字幕状态变化时更新，不能按视频帧率将画面送入 WebView。

主界面应优先完成文件拖放、服务/字幕轨选择、输出格式与模式、任务控制和预览；现代设计意味着默认操作简单且底层信息随时可检查。检查器至少显示容器类型、service ID、PID/asset ID、语言、PTS 范围、DRCS 数量、CRC 错误、丢包数、不连续点与未支持命令。

## 8. IPC、解析安全与测试

GUI 与 worker 使用低频、带界限的消息；首版逐行 JSON stdin/stdout 足够，例如 progress、warning、track。禁止每个字符/包向前端发消息。后续可评估 local socket、named pipe/Unix socket、protobuf 或 MessagePack。

TS 188/192/204 包头和 PAT/PMT 宜手写小型受限解析；TLV/MMTP 可选 winnow、nom 或受限 cursor。所有 parser 必须：不 panic、不越界、不无限循环、不按不可信长度无限分配、报告文件偏移、可从损坏处重同步。

测试体系不可缺省：建立 golden corpus（地上波、BS2K、caption/superimpose、竖排、ruby、DRCS、彩色、位置变化、双语、损坏 TS、BS4K/8K），每项保留合法的原始/构造样本、可靠画面截图、期望事件 JSON、期望 ASS/有损输出与已知问题。对旧工具、FFmpeg/libaribcaption、新工具和必要的实际画面做差分比较：字符、开始/清屏时间、位置、颜色、DRCS、management 切换。至少 fuzz TS sync、PSI length、PES、B24 group、TLV、MMTP、signalling、MPU assembly，并在 Windows/macOS/Linux CI 验证。

公开项目必须包含构建脚本、依赖版本、格式说明、测试方法、缺陷、样本生成器和兼容性结果；受版权约束的广播片段只保留哈希、截短数据或构造样本。ResubWinny 的 Worker、Tauri 服务层与 Svelte 前端统一采用 MPL-2.0；第三方库、二进制、字体和测试语料仍遵循各自的许可证与来源要求。

## 9. 实施顺序与当前证据

1. Rust worker、稳定 CLI/API、项目模型、B24 C ABI、传统 TS 基准语料和回归；**已完成**；
2. ASS/TTML/存档导出及 DRCS 视觉资源；**已完成**；
3. 192-byte MPEG-TS packetisation 中的流式私有 PES、ARIB-TTML、时间轴和长样本回归；**已完成当前基线**；
3a. 188-byte MPEG-TS 中 private PES 的严格 ARIB-TTML 回退路由、ASS/TTML/archive/raw/即时预览构造流回归；**已完成构造流基线，真实样本待补充**；
4. Tauri/Svelte 的任务、轨道、日志、检查器、多任务处理和原生 mpv 预览；**已完成当前基线**；
5. B62 原生字体/ruby/竖排/描边渲染与 M2TS 多样本差分验证；**第三阶段进行中**；
6. TLV/MMTP 的真实语料验证与通用 asset 路由；**等待合法真实 TLV 样本，当前仅实验性**。

新的 Rust workspace 已创建 `crates/arib-caption-worker`。其 `inspect` 命令在有界读取内识别 188-byte MPEG-TS、192-byte M2TS、原始 TLV 与未知输入。传统 B24 通过项目拥有的窄 C ABI 调用 libaribcaption；bridge 会在释放 native 对象前把平面、区域、Unicode/PUA 字符、定位、颜色、样式及 DRCS 代码、替代信息、原始像素复制为 Rust 场景快照。未知 DRCS 同时写成同名 `.drcs` 原始像素/元数据资产，并以 ASS `\p1` 矢量绘图事件表现，不会输出 `[外:<hash>]`。完整地上波转换得到 13,653 个 PES、2,230 个字幕对象、2,736 个区域、29,892 个字符、61 个 DRCS 字形、0 个解码错误。M2TS 路由会发现私有数据 PID、以有界 PES 缓冲重组有效载荷、提取 UTF-8 ARIB-TTML 文档，并将继承自 `div` 的时间与 `region` 位置写入 ASS。随附的 11.5 GB BS4K 回归样本现得到 422 个 TTML 字幕事件、5,051 个字符、0 个解析警告。受限 TLV 路由同样可转换完整 `stpp` 载荷，但前提是它为自包含 UTF-8 TTML 且拥有匹配的 MPT NTP 元数据；其他 asset 继续走原始证据路径。Tauri/Svelte GUI 仅展示状态、预览与诊断并转发 typed API 请求，解析、导出和预览数据准备仍由 Worker/后端完成。B62 字幕样式、ruby、writing mode、资源作用域和有界 PNG/字体证据已接入模型。后端已原生光栅化受支持的 TTML 文本、横排与跨列竖排 Ruby、保守的字形方向/标点、透明度和受限的直接 `tts:textOutline`，并且不会把 `arib-tt:border` 猜作标准描边。Windows `libmpv-render` 与原生 Overlay 合成已经接通。资源完整预览、完整 B62 字形方向/标点/描边语义、通用 TLV/MMTP 字幕抽出和 macOS/Linux 原生预览仍是第三阶段后续工作。

当前模型交付：每个 B24 场景都会拆分为 `RegionInterval`。有界活动区域表只在该区域自身发生变化或消失时关闭它，因此说话人标签与正文可以拥有独立、重叠的生命周期。已经关闭的同一区域会被同时写入保真 ASS、可选 TTML 与 JSONL 存档记录。TTML 保留区域时间、位置、范围、字号、颜色以及带命名空间的未解析 DRCS 引用；ASS 继续以矢量 DRCS 字形承担视觉兜底。Tauri 的完成任务时间轴和诊断窗口直接流式扫描 JSONL，只保留请求页；直播事件列表只保留后端最近窗口，编辑时间轴使用有界预取区间和追加字节游标，不再反复读取完整 archive，也不把完整事件历史送进 WebView。单任务 Worker 可在流式解析边界协作式暂停、继续或取消。中断后 `checkpoint.json` 会记录文件大小、mtime、首尾 64 KiB 指纹、轨道和进度上限；恢复会拒绝被替换或截断的输入。由于 native B24 与部分 artifact 状态尚不能序列化，下次启动仍从录制文件的可信起点完整重放，而不会虚假宣称按字节续跑。

显示平面校正（2026-07-25）：根 `<tt>` 声明有效像素显示范围时，B62/ARIB-TTML 会归一化到原生渲染器的逻辑 `1920×1080` 平面。缺失该范围时仍默认逻辑 2K；只有完整的像素 `origin`/`extent` 几何至少在一个轴越过 2K 范围、且可落入标准 3840×2160 或 7680×4320 平面，才会推断源平面。区域几何按横纵轴分别缩放，像素字号、行距、字距和直接描边宽度采用有界的统一缩放。因此等价的 2K、4K、8K 源布局会保持相同的观众相对字幕面积；模糊或无效数据绝不会被偷偷当作 4K。原始 PES/MMTP 证据保持不变。

竖排标点增量（2026-07-25）：原生 B62 预览只映射 Unicode 明确定义的竖排标点形式，并且仅当捆绑 ARIB 字体含有该字形时使用；否则保留源字符。archive 到 `render_at` 的确定性 PNG 金样覆盖该路径。这不表示已实现拉丁字符旋转、纵中横、完整朝向/标点规则或标准 B62 描边。

原生预览同步增量（2026-07-25）：`sync_preview_overlay` 将 mpv 播放时间读取、archive 查询、原生 RGBA 合成、overlay 写入/清除与相同字幕平面去重全部保留在 Tauri 后端。Svelte 只低频调用 typed API 并展示结果，不估算媒体时间、不排版字幕；mpv 尚未返回时间时后端明确返回 `awaiting-player-time`，不使用本地时钟猜测。

播放时间轴增量（2026-07-25）：原生预览现在持有经过校验的 `PlaybackTimeMapping`，包括 segment 标识、媒体/项目锚点与有理速率。libmpv 只提供媒体时间，archive 渲染使用映射后的项目时间；PTS 修复、节目边界与用户偏移不会再偷偷落入 WebView 逻辑。

libmpv 运行时增量（2026-07-26）：Windows 现由项目进程内加载捆绑的 `libmpv`，不再启动 `mpv.exe` 或使用 JSON named pipe。完整 render API 可用时，`libmpv-render` 是默认路线：专用 WGL 线程独占 OpenGL context、libmpv render loop、resize 消息与后端 BGRA 字幕纹理混合；指定源初始化失败时才回退到 `libmpv-client-overlay`。能力 API 对每条路由返回 `id`、`available`、`experimental` 与结构化不可用原因；macOS/Linux 会明确返回 `preview.platform_not_implemented`。WebView 不接收视频帧或字幕纹理；`render_preview_at` 与 `sync_preview_overlay` 在后端合成有界 native plane 后交给 libmpv。macOS/Linux 原生预览后端已延期，不属于当前 Alpha 验收范围。

视觉基线校正（2026-07-25）：随附的 libaribcaption `screenshot0.png` 是项目面向观众的电视字幕参考图。B24 继续使用 libaribcaption 以已配置的 ARIB 字体、ruby、背景和描边设置生成 RGBA。B62 以相同的观众可见关系为目标；但没有匹配的 B62 源 payload 与合法参考截图时，不宣称像素级一致，见 `docs/visual-reference.md`。

横排布局增量（2026-07-25）：原生 B62 路径现在保留明确换行，并在有界 TTML 区域内应用 `textAlign`、`displayAlign` 与 `lineHeight`，其中 `start`/`end` 会遵循书写方向。archive 到 `render_at` 的 PNG 金样覆盖多行、居中、底部对齐。

参考实现审计（2026-07-25）：`makeding/aribb62.js` 在审计 commit `74304d40a5b8556be1148e123ae70d60f937ecf5` 的 package 元数据中声明 MIT，但仓库和 GitHub license endpoint 都没有独立 LICENSE 文件。其语义可以独立移植到 Rust renderer；在取得可再分发的许可证文本与版权通知前不 vendoring 源码。首个移植是原生命名 TTML 颜色（含 `transparent`），不依赖浏览器 CSS。

原始 TLV 输入通过重复的 4-byte `0x7F/type/length` 头、受限 payload 长度进行内容探测，并提供有界的诊断/原始证据 MMTP 路径：直接 IPv6/UDP、HCfB `0x60`/`0x61` 上下文、MMTP packet ID/payload type、连续 signalling fragment 重组（最多 16 路、每路最多 1 MiB），以及 MPT signalling table 中的 asset type 与 descriptor tag（包括已观察到的 `stpp`）都会报告。MPT MPU timestamp descriptor 会以 packet ID + MPU sequence 为键保留精确的 64 位 NTP 原始值，但不会冒充已归一化的字幕 PTS。对于已知 `stpp` packet ID，会验证 MPU/MFU 封装，并有界重组 MFU（最多 8 个 MPU sequence、每个最多 4 MiB）。首个语义路径只接受同时满足三项条件的载荷：已发现的 `stpp`、完整且自包含的 UTF-8 XML TTML、以及匹配的 MPT NTP 元数据；它以首个有效 MPU 为零点，把 NTP 差值送入既有 TTML 字幕模型。序号断裂、非法聚合、超限、缺失时间戳或其他载荷格式仍只作为原始证据保存，绝不猜测为字幕。这不是泛用 MMTP 字幕支持声明。桌面 DRCS 字典会保存用户映射到平台配置目录；只有用户明确选择映射模式才替换为文本，默认仍保留未解析字形资源。
请求 archive 时，同一次有界扫描还会写入已发现 MPT asset 的 `asset_evidence` 记录（packet ID、类型、descriptor tag 和精确 NTP 原值）。`resource_reference` 会保留来源 `packet_id + mpu_sequence_number` 作用域；`subsampleNumber=0` 是 TTML payload，有限的 `1..lastSubsampleNumber` 单元组成同一 MPU 的资源证据。数字 `subt://` 索引不会被当作全局 packet ID；证据缺失或不完整时仍明确保持未解析状态。

`dump-tlv` 是该层首个原始抽出路径：它只进行一次顺序扫描，只有在已发现的 `stpp` asset 形成完整 closed-caption payload 后才写入 JSONL。每条记录保留 TLV 源偏移、MMTP packet/sequence、MPU sequence、timed-MFU 标志和无损十六进制数据；若对应 MPT MPU timestamp descriptor 存在，还会保留精确 `presentation_ntp` 原始值。`pts_ms` 仍必须明确为 `null`，直到实现共享时间轴策略；原始证据不得虚构时间轴。
同一路径现在也会将已完成有界重组的非 `stpp` MPU/MFU payload 写为 `mmt_asset_payload` 记录，保留 asset type、源偏移、确定性的 MPU 作用域键和无损字节。资源记录可以包含有界头部校验、PNG 尺寸，以及小型完整 PNG 的受限预览 data URI，但这仍只是抽出证据，不表示该 payload 已完成通用解码或可直接渲染。

实现校正（2026-07-23）：M2TS 文件结尾的 PES flush 回归已修复。随附 BS4K 样本现在得到 422 个 TTML 字幕事件、5,051 个字符、0 个解析警告；启用原始导出时会捕获 330 条 PES 记录。桌面端为 Tauri 2 + Svelte 5，而非历史 Slint/eframe 原型。首页任务列表会在平台配置目录中原子保存最近 20 条本地任务摘要，不保存广播 payload。

本地语料校正（2026-07-23）：18.58 GB 地上波与 11.52 GB BS4K 样本现在由 `ARIB_FIXTURE_DIR` 选择，并作为 opt-in 测试验证精确的 streamed byte/count baseline。M2TS 私有 PES envelope 不再被假定为 UTF-8：有界 extractor 会定位完整的 `<tt>…</tt>` 字节切片，只对该 XML 切片做 UTF-8 验证。这恢复了 BS4K 样本的 422 条字幕/5,051 字符，同时不改变 raw PES evidence。

DRCS 报告交付（2026-07-23）：可选的 `--drcs-report` 只会在传统 B24 转换实际遇到 glyph 时生成 `<name>.drcs.json`。它索引代码、尺寸、与颜色无关的 glyph 元数据、替代信息及已保存 `.drcs` 资产路径，不会在报告中复制原始像素字节。原生 UI 暴露同一选项；项目 archive 仍是独立的完整字幕时间线。

TTML 继承校正（2026-07-23）：受限的 M2TS/TLV TTML parser 现在会在每条字幕前遍历所有仍处于打开状态的 `div`，而不是只取文本上最近的 `<div>`。嵌套 `begin`/`end`/`dur` 会从正确的父时间基准累积，继承的 `style` 与 `region` 按 document order 应用，已经关闭的 sibling 不会把 timing、writing mode、colour 或 placement 泄漏到后续字幕。这会改善共享 TTML/archive 模型与保真 TTML 输出；ASS 对 writing mode 和 ruby 仍是近似表达。

TTML 样式交付（2026-07-23）：共享字幕样式现会在 archive 与 TTML interchange 中保留继承的前景/背景色、字体族、字号、粗细、斜体、书写方向、文本/显示对齐、轮廓、行距、字距和透明度。ASS 只映射其有明确定义的字体、粗斜体、字距与前景色；对于不受支持的 TTML 排版或背景语义，不会伪称保真。

ARIB-TTML span 样式校正（2026-07-23）：实际广播 payload 常将有效样式置于 `span style="…"` 而非 `p`。解析器现会解析该引用，包括双轴字号、`arib-tt:letter-spacing` 与 TTML 八位 RGBA 色值。interchange 输出会把安全的 span 引用展开为自包含的内联 TTML 属性，因此不会遗留仅在源文档中存在的样式 ID。真实 BS4K 样本已验证 archive/TTML 中的 `丸ゴシック`、`144px 144px`、前景/背景色与 16px 字距，以及相应的 ASS 近似映射。

字符编码校正（2026-07-23）：ARIB STD-B24 的字符编码字幕不会被当作 UTF-8 文本；它仍交给 libaribcaption 按 B24 规则解码。对于 ARIB-TTML 路径，提取器先从 PES/MMTP 外层中隔离 XML，再遵循 BOM/XML 声明，严格解码 UTF-8、UTF-16LE/BE、Shift_JIS、EUC-JP 或 ISO-2022-JP。畸形或不支持的 XML 会保留为原始证据并被报告，绝不会以替换字符“修复”；外层 framing 的非法字节也不会再导致后续合法文档被丢弃。

当前 worker 已按 `cli.rs`、`inspection.rs`、`jobs.rs`、`preview.rs`、`archive.rs`、`protocol.rs`、`resource.rs`、`transport/`、`caption/`、`timeline.rs`、`drcs.rs` 与 `exporters/` 拆分，`main.rs` 只保留进程入口和测试。`render-at` CLI 与 Tauri 的 `render_at` 都基于有界 archive 时间点快照；这不等同于已完成原生字幕平面渲染或通用 TLV/MMT 支持。

## 10. 变更纪律

任何架构变更必须在三语文档同一变更中更新，并注明：影响的输入 route/模型不变量、对应样本与验证、ASS/存档/DRCS 映射兼容性。未经这种记录，不得把推测、临时原型或单一样本结果宣称为支持。

现实输入优先级：当前 release gate 是 188-byte MPEG-TS/B24 与已成功严格验证的 192-byte MPEG-TS packetisation/private PES/ARIB-TTML，两者都有本地长样本和流式计数基线。原生 BS4K/8K 的 TLV/MMTP 是规范主线，但目前只有构造/单元证据与受限 `stpp` 路由，能力码为 `tlv_mmtp_experimental`；在获得合法真实 TLV 语料前，它只提供探测、诊断、原始证据和明确条件下的转换，不作为通用支持。inspection contract 使用 `mpeg_ts_b24_verified`、`mpeg_ts_ttml_candidate`、`tlv_mmtp_experimental` 与 `unknown_unsupported`：private PES PID 仅为 candidate，不能冒充 TTML 验证结果；`mpeg_ts_192_ttml_verified` 只用于完成严格验证后的 192-byte M2TS 转换 route。这些能力码来自内容探测，不来自扩展名。

换列竖排 ruby 增量（2026-07-25）：后端现在会在明确关联的 ruby 正文自动换列时，按已记录的正文字符格阅读路径分配 ruby 字形，并在对应书写方向的侧边以 0.5 倍绘制。该受限 continuation 已有 archive 到 `render_at` 的 PNG 金样覆盖；它不表示已完成通用 B62 ruby 分组、来源特定定位、纵中横或完整字形朝向。

桌面持久化校正（2026-07-26）：设置、任务记录、任务历史、artifact manifest、checkpoint 与 DRCS 映射现在统一使用同目录原子发布器：先同步完整 `.part`，保留旧元数据直到新文件安装成功，替换失败则恢复旧文件。这修正了 Windows 的覆盖语义；不改变字幕 payload、archive 语义或任何传输 route。

## B62 收敛增量（2026-07-26）

原生 TTML/B62 预览现将连续的 `tts:ruby="base"` span 作为一个 base group，把一条 `tts:ruby="text"` 注释放在整个 group 上方；`arib-tt:ruby` 仍按 `xml:id` 关联。ruby 注释自身的 colour、font size、letter spacing、opacity 和受限 direct `tts:textOutline` 会在后端保留；未明确指定时使用基字 0.5 倍的默认比例。该模型同时覆盖横排、竖排以及自动换列的竖排 ruby。

竖排 renderer 对具备 Unicode vertical-presentation glyph 的标点优先使用该 glyph，CJK 与全角字符保持正立，ASCII/Latin 字符使用后端顺时针位图旋转；明确的 1–2 位 `textCombine` 继续在单个竖排格内横排。2K、4K、8K authored geometry 在 worker 中归一化到逻辑 `1920×1080` 平面，等价布局因此保持相同的观众相对面积。

这些是可重复的后端实现与 unit/visual-golden 覆盖，不等于已用真实 B62 录制流验证所有 broadcaster-specific rule。下一步由 corpus 的合法 source payload 和参考截图确定是否需要扩展非连续 ruby、额外的 Unicode orientation 类别或标准描边语义。

## Windows 原生预览收敛增量（2026-07-26）

Windows 在发现完整 `libmpv` render API 时默认选择 `libmpv-render`。后端拥有 WGL context、libmpv render loop、resize、视频 viewport、后端 BGRA 字幕纹理和混合；若特定源无法初始化 render worker，则该次预览回退到 `libmpv-client-overlay`，backend diagnostics 会报告实际路线、回退原因、surface 尺寸和呈现帧率。真实 3840×2160 HEVC `bs4k_test_2.ts` smoke 已验证启动、视频帧 present、1920×1080 纹理混合/readback，以及 3840×2160 resize/present。WebView 不接收视频帧或字幕纹理。

当前 WGL route 请求 libmpv 的 `hwdec=auto-safe` 策略，允许兼容的 copy-back 加速，但不承诺 zero-copy 的 ANGLE/D3D 硬解互操作。`scripts/validate-preview.ps1 -Long` 现已执行带明确启动、帧率、完整字幕平面上传、控制、工作集和退出阈值的 120 秒真实 4K 门槛。2026-07-30 的 `bs4k_test_2.ts` 实测为 `d3d11va-copy`、34.74 present/s、峰值 1526.9 MiB、4K 预热后增长 111.9 MiB。独立 2K/8K 性能、DPI 和参考截图差分仍未完成；macOS/Linux 仍返回 `preview.platform_not_implemented`。

## ASS 保真校正（2026-07-29）

B24 ASS 导出器现在先将解码来源画布归一化到 ASS 的 1920×1080 play resolution，再同比变换每个可见字符的位置、字号、横向比例、描边和 DRCS 几何；逐字符颜色、粗体、斜体和下划线保持不变。Ruby 使用换算后的广播字符格坐标并置于 layer 1。ARIB-TTML 路线保留安全的行内 span 样式，将明确关联的 Ruby 分层输出；注释未指定字号时使用基字的 0.5 倍。依照 TTML 文字排版语义及审查过的参考实现，B62 双维字号只取第二维作为 ASS 字高，letter spacing 仅通过 ASS 原生 spacing 指令应用一次。导出器不再横向拉伸字体，也不以项目自制的逐字符网格替代 libass shaping。独立 Ruby region 根据来源几何关系匹配基字 region，并以 ASS 标准 `an8+pos` 居中到被注音范围的实际渲染字形中心。正文保持为一个完整 Dialogue event，既不拆分也不移动；仅用同捆字体的 libass-compatible advance 与 ink bounds 修正 Ruby 锚点。单字和多个汉字使用同一范围中点规则，上置、下置均可识别；多行字幕会先选择与 Ruby 垂直距离最近的来源行，再映射水平覆盖范围。FFmpeg/libass 像素测试覆盖单字上置以及下方一行的多字下置，最终水平中心误差超过 3px 即失败，并逐像素比较加入 Ruby 前后的正文画面不发生变化。相同时段字幕只缓冲到 timing 变化为止，仍满足流式内存边界。

ASS 默认使用随项目提供的 `Rounded M+ 1m for ARIB`，广播源的 `丸ゴシック` 也映射到这一经过测量的字体，使 Ruby 宽度计算与播放器实际渲染采用一致字形度量；其他明确指定的来源字体保持不变。18.58 GB 地上波与 11.52 GB M2TS 样本均以 0 解码错误完成，并通过 FFmpeg/libass 实际渲染的 `いかり`/`碇` 以及以 `捧` 字中线居中的 `ささ` 帧确认位置、前景色、字号与黑色描边。任意 TTML 半透明背景矩形不属于 ASS 兼容目标，仍保留在 TTML/archive 数据中。

## Ruby 对应关系与导出专用 Box Layout（2026-07-30）

Ruby 对应关系现已成为字幕模型阶段的产物，而不是 ASS exporter 临时执行的启发式规则。B24 `RubyBinding` 会在 `RegionInterval` 进入导出器之前记录基准 region/index 范围、基准文本与 cell box、Ruby 原始盒、上下位置、书写方向和来源依据。ARIB-TTML 同样记录基准 caption/run/grapheme 范围；同一时间组中的独立 B62 Ruby region 会在有界分组完整后、archive/TTML/ASS 写出前建立对应关系。真实 M2TS corpus 当前形成 31 条结构化 binding，其中 `ささ` 明确对应 `捧`；无法证明的 region 保持未绑定，不作猜测。

只有 ASS 离线导出使用 Box Layout。布局器通过可替换的 glyph-metrics 接口测量随程序提供的 Rounded M+ 字体，把基准文字的实际 ink range 分配为总宽度严格相等的 slot；字形墨迹可能重叠时按整数像素缩小字号，最后仅对整组可见 Ruby 墨迹做一次有界整数像素回退校正。正文始终保持为一条由 libass shaping 的 Dialogue，只有 Ruby 字形允许分别定位。显式 `rubyPosition` 的上置/下置会保留；竖排目前只提供同一算法的轴转置数据路径，等待真实竖排 corpus 验证。由于 libmpv 内部使用的 libass 不公开字形度量 API，FFmpeg/libass 像素测试是当前运行时兼容门槛。该布局不会进入或改动原生预览链路（`libaribcaption -> native RGBA -> libmpv surface`）。

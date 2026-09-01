[English](maintainability.en.md) | [简体中文](maintainability.md) | [日本語](maintainability.ja.md) | [繁體中文（台灣）](maintainability.zh-TW.md)

> 翻译声明：简体中文版本是唯一的权威来源。其他语言版本仅供参考。

# 可维护性审查

本文档记录当前的工程边界，以及仓库适合公开发布源代码之前仍需完成的工作。

## 已确立的边界

- Svelte 应用负责呈现状态并调用有类型的 Tauri 网关。它不解析传输流、不解码字幕，也不渲染视频帧。
- Tauri 服务负责桌面生命周期、持久化、原生预览和 Worker 监管。媒体解析仍在 `arib-caption-worker` 中。
- Worker 将 `CaptionPlane -> RegionInterval -> exporters` 保持为唯一语义路径，并且仅通过狭窄的 C 桥接层使用 libaribcaption。
- 生成的文件隔离在 `build/` 下。源码构建不依赖预先存在的 `target/`、`dist/` 或已签入的日志。
- Tauri 捆绑包创建会显式构建其发布版 Worker 资源。直接运行桌面端 `cargo check/test/clippy` 会跳过仅捆绑时的资源验证，因此贡献者无需仅为检查 Rust 代码而保留过时的发布版 Worker。当 Worker 或其他捆绑资源缺失时，发布构建仍会失败。
- ResubWinny 源代码采用 MPL-2.0 许可。Rust 和前端包元数据声明相同的 SPDX 标识符，桌面捆绑包包含规范的根许可证。

## 已完成的拆分

| 原热点 | 当前结构 |
| --- | --- |
| 桌面字幕渲染器 | `caption_renderer.rs` 现在负责协调合成；`layout`、`rich_text`、`style`、`glyph`、`bitmap` 和 `tests` 分别负责聚焦的关注点。 |
| 桌面预览 | `preview.rs` 负责能力发现和稳定的原生命令包装器；归档分页、叠加层同步、Windows 原生播放、不支持平台的桩实现和测试均为独立模块。 |
| 桌面任务 | `jobs.rs` 负责公共任务模型；JSON/JSONL 持久化和队列监管器分别隔离在 `jobs/repository.rs` 和 `jobs/supervisor.rs` 中。 |
| Worker 导出器 | 公共导出器边界仍位于 `exporters/mod.rs`；ASS、TTML、文本格式、B24 编排、证据和 Ruby 布局位于按格式聚焦的模块中。 |
| Worker TTML | B62 语义、严格 XML 文档解码和 TS/PES 扫描分别位于独立的 `ttml`、`document` 和 `scan` 模块中。 |
| 实验性 TLV/MMTP | 基础数据包/MPU 处理、信令/MPT、证据写入和受约束路径分别位于独立模块中。 |
| Worker 测试 | 语料库、TS/M2TS、B24/时间线、TTML、TLV、归档和合成协议套件在独立文件中各自管理其夹具；完整基线为 146 项测试。 |
| libmpv | 动态客户端 ABI/播放与 Windows 渲染 Worker 已分离；渲染测试已隔离。 |
| 桌面时间线 | 公共分页/呈现保留在 `timeline.rs`；有界实时窗口和追加游标状态隔离在 `timeline/cache.rs` 中。 |
| Svelte 应用 | 主题/区域设置偏好、多任务协调、DRCS 字典状态、任务呈现和输出格式元数据已移入功能控制器。多任务、DRCS 和设置视图现在位于其所属功能目录下，而非源码根目录。 |

剩余的应用外壳是一个显式组合根。`SourceSession` 负责源准备、检查代次、忙碌状态生命周期和过时结果/错误的抑制，然后仅为当前源应用任务设置并激活预览/索引。`ExportSession` 负责导出/索引请求的有效性，包括过时的任务创建回调、过时失败和预览索引取消，以及对应的开始/成功/失败状态投影。`PreviewSession` 负责原生预览几何、生命周期和受管理的启动/停止转换、定位/拖动协调，以及区分未知的首个媒体样本与实际零时间戳。它还负责调整大小的合并及预览页的生成/恢复状态，因此过时的 WebView 宿主和无类型恢复时间戳不会泄漏到应用外壳。播放映射持久化和显式的媒体到项目游标重映射也保留在此预览域内，播放器命令及音量 IPC 错误/通知处理亦然。

成功检查后的默认值由纯任务设置转换产生；外壳不再逐字段重建输出路径、初始轨道/格式选择或源通知。批处理控制器负责队列生命周期和编辑项轨道投影，而跨功能任务激活仍在组合根中。
`HistorySession` 负责有界任务历史持久化，`LayoutSession` 负责响应式外壳转换。`runtime-session.ts` 集中管理任务运行时重置；`feedback-session.ts` 集中管理有界通知和后端错误消息；`selection-session.ts` 集中管理输出格式、保留和轨道选择转换；`bootstrap-session.ts` 加载相互独立的桌面启动资源；`application-lifecycle-session.ts` 负责桌面事件订阅和清理；`recovery-session.ts` 负责检查点资格判定和重放。这些会话将结果投影到 Svelte 值中，但不会成为第二个全局存储。

目前最大的生产文件是 Worker `exporters/ass.rs`（约 1,185 行）、`caption/ruby.rs`（约 1,080 行）、`App.svelte`（约 1,100 行）、Worker `caption/ttml.rs`（约 764 行）、桌面端 `jobs/repository.rs`（约 720 行）以及前端 `features/batch/BatchQueue.svelte`（约 632 行）。导出器、任务和预览入口模块现在是小型所有权边界，而非实现收纳桶。进一步拆分应遵循 ASS 事件构造、Ruby 关联/布局、应用会话生命周期、仓库关注点以及多任务表格/预设关注点，而不是任意的行数阈值。

时间域在其所有权边界上均为显式。前端和桌面映射层区分媒体毫秒与项目毫秒，而 Worker 将 33 位 MPEG PES 时钟表示为 `Pts90k`，并仅在进入字幕 IR、证据或时间线处理时将其转换为毫秒。MMT 呈现 NTP 仍是独立的传输概念。

字幕 IR 的汇聚发生在解析之后，而非传输模型中。封闭的零拷贝 `CaptionCueRef` 为 B24 `RegionInterval` 和 ARIB-TTML `TtmlCaption` 暴露共享的时间、区域、路径、纯文本、Ruby 数量和 DRCS 存在性语义，同时保留其完整的路径特定 DRCS、Ruby、样式和来源载荷。归档写入器使用此公共边界，但保留 schema-v1 的 `region_interval` 和 `caption` 记录形状。

若干渲染器热路径函数仍显式传递几何信息，以避免分配临时上下文对象。兼容性 `start_export`、`create_job`、Worker 事件辅助函数和 libmpv 线程入口点也具有宽签名。其 lint 例外均为局部且有理由；新 API 应使用有类型的请求/状态对象。现有 Tauri 参数名称只能在协调完成前端契约迁移时更改。

## 构建和质量门槛

- Worker、桌面 crate 和模糊测试 crate 的 Cargo 输出统一在 `build/cargo/` 下；Vite 输出位于 `build/frontend/`。
- `scripts/clean.ps1` 会移除当前输出以及过时的根目录、模糊测试、Vite 和 Tauri 输出位置。`-Dependencies` 还会移除 `node_modules`。
- Worker 和桌面 Clippy 在 CI 中使用 `-D warnings` 运行。
- 当前已验证基线为 146 项 Worker 测试和 106 项通过的桌面测试。四项真实录像/归档环境及性能测试仍为选择性启用，因为它们需要 Windows 桌面会话、合法录像或归档路径，以及路径特定的性能阈值。
- 前端契约检查目前覆盖 58 个有类型命令、64 个源文件和四个完整的内置区域设置文件；Svelte 构建无诊断信息。
- `scripts/check.ps1` 是格式化、Worker 和桌面测试/lint、前端构建、模糊测试编译及生成依赖许可证清单的唯一本地入口点。
- `scripts/build.ps1` 是唯一打包入口点。其 Windows 默认值为捆绑配置，该配置会显式安装并验证固定版本的运行时；`-Libmpv External` 会生成不含 libmpv 的包，并要求用户提供兼容运行时。Tauri 基础配置本身不会静默捆绑运行时。
- 常规 CI 路径有四个聚焦作业：一个共享静态质量门槛、一个三平台 Rust 测试矩阵、模糊测试目标编译和依赖审计。每周计划工作流会对每个模糊测试目标执行有界的 30 秒运行；拉取请求保留仅编译的模糊测试覆盖。`cargo-deny` 对 Worker、桌面端和模糊测试清单强制执行已签入的许可证/来源策略。耗时较长的 LGPL libmpv 构建为手动执行，并与拉取请求 CI 隔离。它直接在 GitHub Ubuntu 运行器上运行，并在相应源代码归档旁记录完整的工具/包环境。
- `scripts/verify-repository.ps1` 拒绝生成/下载的工件、嵌套仓库、超大跟踪文件和发布版本漂移。`scripts/package-source.ps1` 从干净的 Git 修订版创建按哈希寻址的源代码归档；两条路径都已在临时仓库中实际运行。
- GitHub 议题和拉取请求模板记录合法样本边界、受影响的传输路径、模型不变量和验证证据。

## 公开发布阻碍项

- 每次依赖更新时，必须保持 `THIRD_PARTY_NOTICES.md` 与 `third_party/versions.json` 同步。现已记录准确的 libaribcaption/libmpv 修订版、哈希、许可证、源位置和动态替换说明。
- 必须将大型 Windows libmpv 二进制文件排除在 Git 之外。`scripts/setup-libmpv.ps1` 会验证其固定版本归档和解压后哈希；Windows CI 和打包会调用该显式设置步骤。
- 必须保持已供应的 libaribcaption 提交与源快照哈希同步。其嵌套 Git 元数据已移除；今后的更新在进入根仓库之前必须通过 `scripts/prepare-vendored-source.ps1`。
- 必须为确切捆绑的 Windows libmpv 构建镜像一个持久、完整的对应源代码归档及构建脚本。适用的 LGPL 文本、构建来源、哈希和替换机制现已记录，但不能只将上游 URL 视为最终发布工件。
- 必须确保字体旁的 Rounded M+ 1m for ARIB 来源/许可证文件包含在每个安装程序和二进制归档中。已通过 SHA-256 将捆绑二进制文件与其记录的上游文件匹配。
- `CONTRIBUTING.md`、`SECURITY.md` 和受支持的工具链策略现已存在。Windows Alpha 候选工作流会运行完整打包门槛并写入安装程序哈希，但不会创建公开发布。
- 必须记录行为准则决定。Signed Stable 发布需要受保护的签名身份，但明确披露且满足源代码、哈希、来源和许可证门槛的 Unsigned Windows Alpha 不需要。
- 必须移除架构文档中不再符合实际实现的声明，并确保全部三种语言版本描述相同的已验证及实验性能力边界。

## 建议顺序

1. 构建可审计的 Unsigned Windows Alpha 流水线，发布准确的标签和提交、完整工件哈希、未签名构建警告、通知以及捆绑 libmpv 的对应源代码回执。
2. 针对源选择、原生预览、动态广播元数据、多任务控制、语言包、输出规划和工件发布执行已打包 Windows 端到端验收。源选择、暂停的原生视频、动态元数据、118 事件索引和最终归档时间线恢复已用 `bs4k_test_2.ts` 验证；其余工作流仍需打包验收。
3. 维护私有的真实广播兼容性矩阵，并仅发布其结果。不得添加合成广播生成来替代合法持有的录像，并须将 TLV/MMTP 明确保持为实验性功能。
4. 为纯前端行为和生成的 Rust 到 TypeScript DTO 类型添加聚焦测试，且不得引入前端测试框架或 RPC 框架。
5. 为确切捆绑的 LGPL libmpv 构建生成固定、完整的对应源代码包；在此完成之前，当前开发 DLL 会阻碍公开二进制分发。
6. 保持 Cargo/npm 依赖审计启用。仅在 libmpv 对应源代码门槛通过后发布未签名 Alpha；将签名作为构建提升到 Signed Stable 时的一项独立要求。

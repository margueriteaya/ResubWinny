# Changelog

> **唯一权威原文（简体中文）**。其他语言： [English](CHANGELOG.en.md) · [繁體中文](CHANGELOG.zh-TW.md) · [日本語](CHANGELOG.ja.md)

本项目仍处于早期 Alpha 阶段，版本可能包含破坏性变更。

## [0.2.3-alpha.1] - 2026-09-03

### 工作台与首次引导

- 重新编排主工作台：录制入口、预览和常用控制优先呈现，首页主流程在常见窗口高度内保持可见；同时微调桌面工作流文字基线。
- 新增设置页的 About、构建来源信息和离线许可证浏览器；分段控件补齐键盘操作，偏好设置会自动持续保存，并移除了未使用的时间线偏好项。
- 新增 ARIB 风格的首次引导体验，以字幕叠加、Ruby、DRCS 与 XMB 波面介绍工作流；降低动画开销，并修正 16:9 XMB 场景在不同窗口比例下的显示比例。

### B62 / TLV 字幕处理

- 集成原生 B62 TLV 后端，供 ARIB‑TTML 字幕工作流直接使用。
- 保留 B62 源布局语义：region 与行内背景分别处理，字幕以与分辨率无关的方式映射到视频内容 viewport，避免将 region 容量误作显示平面边界。

### 工程、文档与发布

- 新增简体中文、繁体中文、日语和英语开发者文档，并明确简体中文为唯一权威来源。
- 更新 Rust 依赖、Vite 与 Svelte Vite 插件；刷新前端依赖许可证信息。
- 加固 libmpv 构建与缓存流程，修正稳定 Cargo 构建和相关图形依赖配置；升级 Actions 的 artifact 与 cache 操作。
- 修正干净检出环境中 Zlib 配置头的生成来源，使 Windows 原生 TLV 构建不再依赖未跟踪文件。

### Windows Alpha 发布

- 首次附带可安装的未签名 Windows x86_64 Alpha 二进制、完整对应源码、许可证材料、构建回执和 SHA-256 校验和。
- Windows 可能显示“未知发布者”提示；该提示是未签名 Alpha 的预期行为，并不表示程序已获得代码签名验证。

### 已知限制

- 当前仍为预览版；Windows 是原生视频预览的主要验收平台。
- macOS 与 Linux 尚未提供原生视频预览。
- 原始 TLV/MMTP 支持仍为实验性，不应视为通用 BS4K/8K 支持；B62 的真实广播兼容性验证仅在不可再分发的私有素材上执行。
- 本版 Windows 包未签名。私有真实广播素材、从中导出的字幕和截图均不会随发布分发。

## [0.2.2-alpha.1] - 2026-08-30

### Windows Alpha 发布

- 将公开发布明确分为 Source Release、Unsigned Windows Alpha 和 Signed Stable；代码签名不再阻塞明确披露的公开 Alpha。
- 未签名 Windows Alpha 现在随包提供风险说明和依赖许可证清单，并生成包含精确 Git tag、commit、文件大小与 SHA-256 的 Release manifest。
- Windows candidate 必须使用指定 libmpv 合规构建产出的同一 DLL、import library、完整对应源码与 `SOURCE-RECEIPT.json`；哈希、固定来源和完整源码包集合会在汇聚时交叉校验。
- 增加私有真实广播兼容性矩阵，规定使用安装后的程序验证完整字幕工作流，同时只公开结果、不公开录像、字幕或节目元数据。

### 已知限制

- 当前固定的上游 libmpv 开发 DLL 仍不可公开分发；必须先由新的合规 workflow 生成并长期发布相匹配的二进制、完整对应源码和构建回执。
- Windows 安装包候选仍需通过干净系统安装、真实录制完整工作流和卸载验收，才可创建公开 Unsigned Alpha Release。

## [0.2.1-alpha.1] - 2026-08-30

### UX 与状态表达

- 将后台预览索引与导出任务拆分为两个用户可理解的状态；索引期间仍可配置并开始导出，后端需要串行时会明确说明等待关系。
- 增加全局、可关闭的持久错误横幅，即使输出面板折叠也能看到操作失败。
- 进入 Tasks 页面时不再自动弹出文件选择器，而是展示已有的空白任务页面。
- Preview、Events 与 Diagnostics 在普通宽度显示文字标签，仅在紧凑视口使用纯图标。
- 修正首页 Recent 的假选中状态和点击范围；整行支持鼠标与键盘打开，并移除没有对应历史页面的 “View all” 操作。

### 工程与发布

- 加固跨平台 CI、Cargo 依赖策略、fuzz 检查、Windows 原生依赖与 lint 流程。
- 修正源码快照哈希的跨平台一致性，并继续固定、验证 libmpv 运行时来源。
- 完善源码发布、依赖许可证与仓库完整性检查。

### 已知限制

- 当前仍为预览版；Windows 是原生视频预览的主要验收平台。
- 原始 TLV/MMTP 支持仍为实验性，不应视为通用 BS4K/8K 支持。
- 本 Release 不附公共 Windows 二进制；签名和 libmpv 对应源码等发布门槛仍需单独满足。

[0.2.2-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.2-alpha.1
[0.2.1-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.1-alpha.1
[0.2.3-alpha.1]: https://github.com/margueriteaya/ResubWinny/releases/tag/v0.2.3-alpha.1

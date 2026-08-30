# Changelog

本项目仍处于早期 Alpha 阶段，版本可能包含破坏性变更。

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

# 第三方依赖更新政策

[简体中文](dependency-updates.md) · [繁體中文](dependency-updates.zh-TW.md) · [日本語](dependency-updates.ja.md) · [English](dependency-updates.en.md)

> **规范性说明：** 简体中文版本是唯一权威来源。其他语言版本仅为同步译文；如有歧义或冲突，以简体中文版本为准。

ResubWinny 使用固定且可审查的依赖更新。应用程序运行时绝不下载或替换解析器、渲染器、字体或播放组件。更新自动化可以提出建议，但不得合并或发布。

## 依赖类别

| 类别 | 示例 | 固定与更新规则 |
| --- | --- | --- |
| 随附源代码 | libaribcaption | 在 `third_party/versions.json` 中固定上游标签、完整提交和确定性的源代码快照哈希；审查源代码差异及许可证，随后用 `scripts/prepare-vendored-source.ps1` 移除嵌套的 Git 元数据。 |
| 下载的二进制运行时 | Windows libmpv | 固定发布标签提交、工作流配方提交/运行、工具链提交、上游 mpv 提交、资源名称、归档哈希及提取后哈希。`scripts/setup-libmpv.ps1` 为开发和打包显式安装它；应用程序绝不下载它。绝不可仅替换 DLL，而不同时提供其头文件、声明和对应源代码计划。 |
| 仅供参考的源代码 | aribb62.js | 固定已审查的提交。上游变更是研究输入，不是可执行依赖，也不会被自动移植。 |
| 包管理的源代码 | Cargo 和 npm 包 | 锁定文件是权威来源。Dependabot 可以提出更新；维护者负责审查和测试。 |
| 视觉资源 | 用于 ARIB 的 Rounded M+ 1m | 固定二进制哈希、来源和许可证。替换时必须进行字形覆盖和视觉 golden 对比。 |

## 必需的更新记录

每次依赖更新必须记录：

1. 旧版和新版、提交、制品哈希及上游 URL；
2. 上游发布说明及已审查的源代码/ABI 差异；
3. 许可证、版权、构建选项和传递依赖的变更；
4. 受影响的 ResubWinny 路由和模型不变量；
5. 为更新运行的测试和语料证据；
6. 对 archive、ASS、TTML、DRCS 和预览的输出兼容性影响；
7. 回滚提交或先前制品标识。

## 验证门槛

所有更新均须运行常规项目门槛：

```text
cargo test -p arib-caption-worker
cargo check --manifest-path studio-tauri/src-tauri/Cargo.toml
npm run build --prefix studio-tauri
cargo check --manifest-path fuzz/Cargo.toml
cargo fmt --check
```

附加门槛取决于组件：

- **libaribcaption：** 桥接 ABI 编译、B24 解码语料、DRCS 映射、RegionInterval 时间以及 B24 视觉 golden 对比。即使 C ABI 未变，更改字符映射、控制代码、默认选项或渲染器也属于语义变更。
- **libmpv：** 导出符号检查、可替换性检查、原生预览冒烟测试、seek/pause/resume、叠加层时钟同步、调整大小/DPI，以及 2K/4K/8K 性能样本。验证制品仍是 LGPL 构建，并用 `scripts/package-libmpv-source.ps1` 打包其精确源代码缓存。
- **aribb62.js：** 手动检查上游变更。只移植由 ARIB 文档或语料证据支持、且已被独立理解的行为。在其可再分发许可证明确前，绝不复制新增代码。
- **字体：** 字形覆盖、缺字诊断、横排/竖排 ruby、标点方向、描边/背景，以及逻辑 2K/4K/8K 视觉等价性。

解析器或渲染器更新在发布前必须进行长样本回归。改变预期输出的变更必须更新 golden 数据，并解释为何新结果更正确；禁止静默接受新输出。

## 安全更新

高严重性安全更新可以采用加急审查，但仍需要许可证验证、受影响边界的重点测试和明确的回滚制品。只有在发布说明记录该例外并安排被省略的门槛时，才可以跳过无关的长时间测试。

## 检查上游

运行 `scripts/check-upstreams.ps1 -Online` 以验证本地哈希，并将固定的提交与当前上游头部比较。当可用上游更新应产生失败的维护信号时，请在计划 CI 中加入 `-FailOnUpdate`。存在可用更新不代表获得合并许可。

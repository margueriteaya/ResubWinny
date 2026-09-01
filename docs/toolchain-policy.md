# 支持的工具链

[简体中文](toolchain-policy.md) · [繁體中文](toolchain-policy.zh-TW.md) · [日本語](toolchain-policy.ja.md) · [English](toolchain-policy.en.md)

> **规范性声明：**简体中文版本是唯一的权威来源。其他语言版本均为同步翻译；若措辞存在歧义或冲突，以简体中文版本为准。

本仓库通过 `rust-toolchain.toml` 将 Rust 固定为 `1.97.1`。CI 和本地候选发布版本构建必须使用该文件，而非未限定版本的 `stable`。同一工具链中的 Rustfmt 和 Clippy 是门禁的一部分。

桌面前端支持 Node.js 22 LTS，并使用已提交的 npm 锁定文件。验证和打包必须使用 `npm ci`，不得进行不受约束的依赖项刷新。较新的 Node 版本可能可在本地运行，但并非发布基线。

Windows 11 x86-64 是 Alpha 软件包及原生预览的验收平台。Worker、Tauri 编译和前端检查仍会在 Windows、macOS 和 Linux 上进行，但 macOS/Linux 原生预览后端暂缓实现。

工具链升级属于有意进行的依赖项变更，必须满足：

1. 审查发布说明和兼容性；
2. 通过 Worker、桌面端、前端和模糊测试的编译门禁；
3. 审查锁定文件，且不得夹带无关的软件包变动；
4. 完成 Windows 打包预览和长样本回归测试；以及
5. 在同一次变更中更新 CI、本文件和贡献者说明。

任何应用程序组件均不得在运行时安装编译器、软件包管理器或构建工具。

# 参与贡献 ResubWinny

> **唯一权威原文（简体中文）**。其他语言： [English](CONTRIBUTING.en.md) · [繁體中文](CONTRIBUTING.zh-TW.md) · [日本語](CONTRIBUTING.ja.md)

ResubWinny 欢迎能保持后端优先架构的聚焦修复和功能。开始涉及传输、字幕模型、渲染器或桌面工作流的大型变更前，请先发起设计讨论，说明输入路线、模型不变量、预期产物、样本和已知兼容性限制。

## 架构规则

- Svelte 只展示后端状态并转发类型化请求；它不解析媒体、不计算字幕排版、不解码视频，也不拥有字幕时间。
- Tauri 负责桌面生命周期、持久化、原生预览和 Worker 监管。媒体和字幕处理属于 `arib-caption-worker`。
- 除纯瞬时的界面状态外，每项 GUI 操作都必须有等价的 Worker/CLI 或后端 API。
- `CaptionPlane -> RegionInterval -> exporters` 是唯一的字幕语义路径。libaribcaption 始终置于项目维护的窄 C ABI 之后。
- 输入类型从有界的内容证据中探测，绝不信任文件扩展名。
- TLV/MMTP 是实验性、证据优先的功能；不得称其已验证，也不得把未知 asset 推断为字幕。

## 本地环境

使用 `rust-toolchain.toml` 固定的 Rust 工具链、Node.js 22 LTS，并在 `studio-tauri` 中执行 `npm ci`。生成文件应位于 `build/` 下，不属于源码变更。

Windows 原生预览开发还需要 7-Zip，以及显式安装并经过哈希校验的 libmpv 运行库：

```powershell
./scripts/setup-libmpv.ps1
```

应用程序绝不会自行下载或更新此运行库。

安装依赖后，执行完整本地质量门禁：

```powershell
./scripts/check.ps1
```

`-SkipFrontend` 和 `-SkipFuzz` 可用于聚焦的仅 Rust 检查；它们不能替代提交 Pull Request 前的完整门禁。

```text
cargo test -p arib-caption-worker
cargo build -p arib-caption-worker --release
cargo test --manifest-path studio-tauri/src-tauri/Cargo.toml
npm ci --prefix studio-tauri
npm run build --prefix studio-tauri
cargo check --manifest-path fuzz/Cargo.toml
cargo fmt --check
cargo fmt --manifest-path studio-tauri/src-tauri/Cargo.toml --check
```

提交 Rust 修改前，以拒绝警告的方式运行 Clippy。涉及传输、时间线、模型、渲染器或导出器的改动还需要针对性的回归测试。合法的长时录制文件仅保存在本地；只有在允许再分发时，才提交构造的或裁剪过的 fixture。

## 变更要求

- 将面向用户的公开文字保留在 locale 文件中。内置的 `en`、`ja`、`zh-CN` 和 `zh-TW` 文件必须含有相同的键。
- 保持 Worker JSONL 和类型化 Tauri 合同带有版本，并顾及向后兼容。
- 保持解析缓冲区和输出尺寸有界；使用 64 位源文件偏移。
- 将不支持的源数据保留为明确证据，或使用稳定代码拒绝；不得猜测。
- 合同变动时更新 README、后端合同、架构文档、语料预期和导出限制。
- 不得提交录制文件、任务输出、日志、构建产物、生成的依赖树、凭据或签名材料。

## 依赖与许可证

ResubWinny 源码采用 MPL-2.0。新依赖必须使用兼容许可证、记录用途；若随项目捆绑，还须固定来源并更新许可证清单。请遵循 [依赖更新策略](docs/dependency-updates.md)；libaribcaption、libmpv、Rounded M+ ARIB 字体和仅作参考的 aribb62.js 各有不同的更新与署名要求。

vendor 源码目录不得含有嵌套的 `.git` 元数据。审查干净、固定的 libaribcaption 更新后，运行 `scripts/prepare-vendored-source.ps1`，将其转换成应放入本仓库的源码快照。

参与贡献即表示你同意依照 MPL-2.0 提供你的贡献。

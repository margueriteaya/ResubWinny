# ResubWinny

⚠️本项目现在处于alpha前期，无法保证可用性，且可能存在破坏性变更！⚠️

⚠️This project is currently in the early alpha stage; availability cannot be guaranteed, and there may be breaking changes! English documents is preparing!⚠️

⚠️このプロジェクトは現在α版の初期段階にあり、安定した動作を保証できず、また破壊的な変更が行われる可能性があります。日本語の開発ドキュメントはまだまだ準備しています。⚠️

ResubWinny 是一款在 Windows 上运行的面向泛日本内容视频档源文件的字幕抽取、检查、预览与转换工具，具有现代化的使用界面，也可使用命令行来进行操作。

它能够处理地面数字电视、BS/CS 2K 以及部分 BS4K/8K 录制文件中的 ARIB 字幕，实时预览视频，为外部播放器播放尽可能输出保留字幕位置、颜色、字号、描边、Ruby（注音）、ARIB 外字、DRCS 字形和无障碍标识的字幕，也可丢弃特殊标签，输出为适合后续工作或存档的格式。

未来会加入 BD/DVD 的图形化字幕提取与 OCR 识别、对照修正与转换功能，并且实现多操作系统的支持。

项目当前版本为 `v0.1.0α`（源码版本 `0.1.0-alpha.1`）。目前仍处于开发阶段。

## 特性

### 字幕抽取与识别

- 按文件内容探测输入格式，不依赖 `.ts`、`.m2ts` 或 `.mmts` 扩展名作判断；
- 支持 188-byte MPEG-TS 中的 ARIB STD-B24 字幕；
- 支持 192-byte M2TS 风格 MPEG-TS、私有 PES 与严格验证的 ARIB-TTML 字幕；
- 支持多服务、多字幕轨道发现与选择；
- 解析录制文件中可用的广播网络、服务、节目及播出时间，并可按播放位置查询相应状态；缺少对应 SI/EIT/TOT 证据时不会伪造字段；
- 严格处理 UTF-8、UTF-16LE/BE、Shift_JIS、EUC-JP 与 ISO-2022-JP 编码的 TTML；
- 对损坏、截断或未支持的输入给出结构化诊断。

### 字幕语义与排版

- 保留字幕区域独立的显示时间和重叠关系；
- 保留位置、字号、颜色、描边、透明度、字距和书写方向；
- 识别并保留 Ruby 与被标音对象的结构化对应关系；
- 支持横排、基础竖排、连续 Ruby 分组和竖排字符方向处理；
- 识别 ARIB 特殊符号、自定义 DRCS 字形及常见无障碍标识；
- 将 2K、4K、8K 来源几何归一化到逻辑字幕平面，使字幕保持相近的观众可见比例。

### 导出与检查

- 可一次选择 ASS、TTML、SRT、WebVTT 等多个输出格式；
- 可分别决定是否保留位置、颜色、Ruby、DRCS、ARIB 外字和无障碍标识；
- 在字幕事件列表中筛选并标记特殊字幕特征；
- 提供项目存档、原始 PES/MMTP 证据和 DRCS 报告；
- 支持中途暂停，先写入临时 `.part` 文件，成功后输出完整文件；
- 后端校验输入与输出路径，防止字幕输出覆盖原始录像。

### 桌面应用

- 多语言界面，内置简体中文、繁体中文、日语和英语；
- 支持浅色、深色和跟随系统主题；
- 支持多任务创建、排队、暂停、继续、取消、历史记录和诊断；
- 提供字幕事件列表、可缩放时间轴、点击/拖动跳转和 DRCS 字典；
- Windows 使用 libmpv 原生渲染视频，不把视频帧送入 WebView；
- B24 字幕平面和 ARIB-TTML 字幕图像由 Rust 后端生成并叠加到原生预览。

## 输入支持状态

| 输入路线 | 状态 | 说明 |
| --- | --- | --- |
| 188-byte MPEG-TS + ARIB STD-B24 | 已验证 | 面向地面数字电视及 BS/CS 2K 录制文件 |
| 192-byte M2TS 风格 MPEG-TS + 私有 PES + ARIB-TTML | 已验证 | 已通过现有 BS4K 录制样本回归；是否成立由内容探测决定 |
| MPEG-TS 中的私有 PES/TTML 候选 | 有界探测 | 只有完整 XML 边界、声明编码和 TTML 文档均通过验证后才转换 |
| 原始 TLV/IP/UDP/MMTP | 实验性 | 以探测、诊断和原始证据为主；只在严格条件下处理完整 `stpp`/TTML 资产 |
| 未知或不受支持的输入 | 明确拒绝 | 返回稳定错误和探测证据，不猜测容器或字幕类型 |

原始 TLV/MMTP 不属于当前已验证的通用 BS4K/8K 支持。文件扩展名只用于文件选择器提示，不能作为传输格式的证据。

## 输出格式

| 格式 | 用途与限制 |
| --- | --- |
| ASS | 面向字幕制作和常见播放器；保留 ASS/libass 能表达的位置、颜色、字号、描边和 Ruby，但无法还原字幕后的半透明背景矩形 |
| TTML | 保留独立区域、时间、样式、Ruby、书写方向、DRCS 引用和来源信息 |
| SRT | 纯文本兼容输出；无法表达广播位置、重叠区域、Ruby 排版和 DRCS 图形 |
| WebVTT | Web 兼容文本输出；与 SRT 一样属于有损格式 |
| Caption archive (`.caption.jsonl`) | 保存统一字幕模型、区域生命周期、Ruby 对应关系和来源信息，用于分页时间线和再次渲染 |
| Raw evidence | 保存来源偏移、序号、时间来源及无损 PES/MMTP payload |
| DRCS report/assets | 保存无法直接映射到 Unicode 的字形、像素资源、候选映射与用户选择 |

## 技术栈

| 层级 | 技术 |
| --- | --- |
| 桌面前端 | Svelte 5、TypeScript、Vite、Lucide Svelte |
| 桌面应用层 | Tauri 2、Rust 2024 Edition |
| 字幕 Worker | Rust、版本化 JSONL 协议、流式 I/O |
| B24 解码与渲染 | libaribcaption、项目自有狭窄 C ABI、C++ 桥接 |
| 原生视频预览 | libmpv render API；Windows WGL/OpenGL 合成 |
| 字幕模型 | `CaptionPlane -> RegionInterval -> exporters` |
| 持久化 | 应用数据目录中的原子 JSON/JSONL 文件 |
| 测试与质量 | Cargo test、Clippy、Rustfmt、Svelte Check、cargo-fuzz、GitHub Actions |

前端只展示后端状态并转发类型化请求，不读取广播数据、不决定字幕时间、不计算最终字幕排版，也不处理视频帧。媒体与字幕能力由 Worker 或 Tauri 后端先实现，再接入 GUI；能够在 GUI 中执行的核心操作应有对应的 CLI、Worker 或后端 API。

## 技术架构

```text
Svelte 5 前端
    | typed Tauri API / 低频事件
    v
Tauri 2 Rust 应用服务层
    | 任务调度、持久化、原生预览、Worker 管理
    v
arib-caption-worker
    | 流式探测、解析、字幕模型、时间轴与导出
    v
libaribcaption
    | 项目自有狭窄 C ABI
    v
ARIB STD-B24 解码与原生字幕平面
```

Worker 使用固定大小的流式缓冲和 64 位文件偏移。文件体积增加时，常态内存不应随文件长度线性增长。详细职责与接口见 [中文架构文档](docs/architecture.zh-CN.md) 和 [后端接口合同](docs/backend-contract.md)。

## 主要依赖

| 依赖 | 用途 | 集成方式 | 许可证/状态 |
| --- | --- | --- | --- |
| [xqq/libaribcaption](https://github.com/xqq/libaribcaption) `v1.1.2` | ARIB STD-B24 解码与原生渲染 | 固定 commit 的 vendored 源码 | MIT |
| [mpv](https://mpv.io/) / libmpv | Windows 原生视频预览 | 动态链接、可替换 DLL；构建时按哈希下载 | LGPL-2.1-or-later |
| Rounded M+ 1m for ARIB `1.3` | ARIB 字符 fallback、字幕预览与 ASS 字体度量 | 随项目分发字体 | M+ FONT LICENSE 与 WadaLab 授权 |
| Tauri 2 | 桌面窗口、原生 API 与打包 | Cargo 依赖 | 见依赖许可证清单 |
| Svelte 5 / Vite | 前端界面与构建 | npm lockfile 固定 | 见依赖许可证清单 |
| serde / serde_json | Worker 协议、模型和持久化 | Cargo 依赖 | 见依赖许可证清单 |
| encoding_rs | ARIB-TTML 字符编码 | Cargo 依赖 | 见依赖许可证清单 |
| roxmltree | namespace-aware TTML/XML 结构解析 | Cargo 依赖 | MIT / Apache-2.0 |
| fontdue / ttf-parser | 后端字幕及 Ruby 字形度量 | Cargo 依赖 | 见依赖许可证清单 |
| [makeding/aribb62.js](https://github.com/makeding/aribb62.js) | ARIB-TTML/B62 行为研究参考 | 仅参考，不捆绑源码 | 审查版本的 package 元数据声明 MIT；详见第三方记录 |

完整版本、固定来源、哈希和许可证见 [第三方声明](THIRD_PARTY_NOTICES.md)、[依赖版本记录](third_party/versions.json)、[依赖许可证清单](docs/dependency-licenses.md) 与 [依赖更新策略](docs/dependency-updates.md)。

## 构建环境

当前 Alpha 的完整桌面验收平台是 Windows 11 x86-64。需要：

- `rust-toolchain.toml` 固定的 Rust `1.97.1`，包括 Rustfmt 与 Clippy；
- Node.js 22 LTS；
- npm 10 或 11；
- CMake；
- Visual Studio 2022 Build Tools，包含 MSVC C/C++ 工具链和 Windows SDK；
- Microsoft Edge WebView2 Runtime；
- 7-Zip，用于安装固定版本的 Windows libmpv 开发包。

Worker、Tauri 编译检查和前端构建会在 Windows、macOS 与 Linux CI 上运行；当前原生预览和安装包的产品验收平台仍是 Windows。

## 构建方法

以下命令均在仓库根目录执行。

### 1. 一条命令构建

```powershell
./scripts/build.ps1
```

该命令会安装锁定的前端依赖，下载并核对固定版本的 Windows libmpv，构建 Worker、前端、桌面程序和安装包。重复执行时已验证的 libmpv 会直接复用。ResubWinny 运行时不会自行下载或更新播放组件。

只生成可执行文件、不生成安装包：

```powershell
./scripts/build.ps1 -Target Executable
```

构建不携带 libmpv 的版本：

```powershell
./scripts/build.ps1 -Libmpv External
```

此模式不会分发 libmpv，但实时预览需要通过 `RESUBWINNY_LIBMPV` 提供兼容运行库。需要同时执行完整质量检查时附加 `-Check`。携带 libmpv 的本地产物仅供开发与私下测试；公开发布前仍须为完全相同的 DLL 提供通过校验的对应源码包和 receipt。

### 2. 运行完整质量检查

```powershell
./scripts/check.ps1
```

该脚本覆盖 Worker 与桌面后端测试、Clippy、Rustfmt、Svelte 检查、前后端接口合同、fuzz target 编译、许可证清单和第三方来源校验。

### 3. 开发运行

先构建 Worker，再启动 Tauri 开发程序：

```powershell
cargo build -p arib-caption-worker
npm run tauri --prefix studio-tauri -- dev
```

如需使用其他 Worker，可设置 `RESUBWINNY_WORKER` 为可执行文件的绝对路径。

### 4. 直接调用底层构建命令

基础 Tauri 配置不强制携带 libmpv，因此下面的命令适合开发和仅使用外部运行库的构建。只生成桌面可执行文件：

```powershell
npm run tauri --prefix studio-tauri -- build --no-bundle
```

生成不携带 libmpv 的 Tauri bundle：

```powershell
npm run tauri --prefix studio-tauri -- build
```

需要携带已安装并经过哈希验证的 Windows libmpv 时，使用统一构建脚本，或显式添加配置：

```powershell
./scripts/setup-libmpv.ps1
npm run tauri --prefix studio-tauri -- build --config src-tauri/tauri.windows-libmpv.conf.json
```

产物统一位于：

```text
build/cargo/release/resubwinny-studio.exe
build/cargo/release/bundle/
```

清理构建产物：

```powershell
./scripts/clean.ps1
```

附加 `-Dependencies` 可删除 `node_modules`；附加 `-DownloadedRuntimes` 可删除显式安装的 libmpv 开发文件；附加 `-TestOutputs` 可删除本地测试输出。

## 目录结构

```text
ResubWinny/
├── crates/
│   └── arib-caption-worker/       # 流式探测、解析、字幕模型、CLI 与导出器
│       ├── src/caption/           # B24、TTML/B62 与 Ruby 语义
│       ├── src/transport/         # MPEG-TS、M2TS 与实验性 TLV/MMTP
│       ├── src/exporters/         # ASS、TTML、SRT、WebVTT、archive 等输出
│       └── src/tests/             # Worker 分模块回归测试
├── native/
│   └── aribcaption-bridge/        # libaribcaption 的狭窄 C ABI 桥接
├── shared/                        # Worker 与桌面后端共享的识别规则
├── studio-tauri/
│   ├── src/                       # Svelte 前端
│   │   ├── backend/               # typed Tauri API 与事件入口
│   │   ├── components/            # 通用界面组件
│   │   ├── features/              # 首页、任务、多任务、DRCS、设置等功能
│   │   └── locales/               # zh-CN、zh-TW、ja、en 文案
│   └── src-tauri/
│       └── src/                   # 任务、预览、持久化、时间线与 Worker 管理
├── fuzz/                          # TS、PES、B24、TTML、MMTP 等 fuzz targets
├── scripts/                       # 构建、检查、清理、语料和发布脚本
├── docs/                          # 架构、接口、语料、许可证与维护文档
├── third_party/                   # 固定来源的第三方源码、头文件、字体与许可证
├── .github/                       # CI、依赖更新、Issue 与 PR 模板
├── Cargo.toml                     # Worker workspace
├── CONTRIBUTING.md                # 贡献规则
├── THIRD_PARTY_NOTICES.md         # 第三方声明
└── LICENSE                        # MPL-2.0
```

所有生成文件都应位于被忽略的 `build/`、`node_modules/` 或测试输出目录，不应混入源码提交。

## 桌面程序使用方法

1. 启动 `build/cargo/release/resubwinny-studio.exe`。
2. 在首页选择一个录制文件。程序会按内容探测容器、服务和字幕轨道，并自动准备原生预览；预览默认暂停，不会自动播放。
3. 进入任务页面，查看广播服务、字幕轨道、事件列表和时间轴；节目与播出时间会在录制文件包含并成功解析相应广播表时显示。
4. 在预览窗口中播放、暂停、前后跳转、拖动时间轴或调整音量。下方字幕时间轴可以缩放、点击和拖动跳转。
5. 在输出设置中选择一个或多个格式，并选择是否保留位置、颜色、Ruby、DRCS/ARIB 外字及无障碍标识。界面会提示目标格式不能完整表达的内容。
6. 选择输出目录后开始导出。未开始导出前，程序不会在所选输出目录创建字幕产物。
7. 在任务日志、诊断和产物列表中检查结果。遇到未映射 DRCS 时，可在 DRCS 字典中查看原始图像并保存映射。
8. 可继续添加录制文件形成多任务队列，各任务由后端独立保存状态并调度。

## CLI 使用方法

Worker 默认路径为：

```text
build/cargo/release/arib-caption-worker.exe
```

所有机器可读事件写入 `stdout`，人类可读日志写入 `stderr`。下面列出当前全部 CLI 命令。

### `inspect`

探测输入格式、服务、字幕轨道和候选路由，不进行字幕导出。

```text
arib-caption-worker.exe inspect <recording>
```

### `broadcast-at`

按源文件字节偏移查询 MPEG-TS/M2TS 中对应的广播网络、服务、节目与播出时间。`service-id` 使用十进制。

```text
arib-caption-worker.exe broadcast-at <recording> <byte_offset> [--service-id <id>]
```

### `decode-b24`

发现并顺序解码传统 B24 字幕轨道，输出进度与统计事件，不创建字幕文件。

```text
arib-caption-worker.exe decode-b24 <recording>
```

### `convert`

按内容自动探测路线并转换字幕。未指定输出路径时，默认使用输入文件名并改为 `.ass`。

```text
arib-caption-worker.exe convert <recording> [output] [options]
```

### `convert-b24`

只使用传统 MPEG-TS/B24 路线转换字幕，参数与 `convert` 相同。

```text
arib-caption-worker.exe convert-b24 <recording> [output] [options]
```

`convert` 与 `convert-b24` 支持以下全部选项：

| 选项 | 作用 |
| --- | --- |
| `--ttml` | 同时导出 TTML |
| `--srt` | 同时导出 SRT 兼容副本 |
| `--webvtt` | 同时导出 WebVTT 兼容副本 |
| `--archive` | 同时导出 caption archive |
| `--archive-only` | 只发布 caption archive；不能与其他格式或 `--no-ass` 组合 |
| `--raw` | 导出路线对应的原始 PES/MMTP 证据 |
| `--no-ass` | 不保留默认 ASS 输出 |
| `--drcs-report` | 在发现 DRCS 时生成报告 |
| `--drcs-map <json>` | 使用指定 JSON 文件中的 DRCS 用户映射 |
| `--track-id <id>` | 选择字幕 PID/asset；接受十进制或 `0x` 十六进制 |
| `--drop-position` | 不保留字幕位置 |
| `--drop-color` | 不保留颜色 |
| `--drop-ruby` | 不保留 Ruby |
| `--drop-drcs` | 不保留 DRCS 字形 |
| `--drop-gaiji` | 不保留 ARIB 特殊外字 |
| `--drop-accessibility` | 不保留无障碍标识 |
| `--overwrite` | 允许覆盖已存在的输出产物；仍禁止覆盖输入录像 |

示例：

```text
arib-caption-worker.exe convert recording.ts output.ass --ttml --archive --raw --drcs-report
arib-caption-worker.exe convert recording.m2ts output.ass --track-id 0x120 --srt --webvtt
arib-caption-worker.exe convert-b24 recording.ts output.ass --drop-position --drop-accessibility
arib-caption-worker.exe convert recording.ts output.caption.jsonl --archive-only
```

转换运行期间可通过 `stdin` 逐行发送协作式控制消息：

```json
{"type":"pause"}
{"type":"resume"}
{"type":"cancel","keepCheckpoint":true}
```

### `render-at`

从 caption archive 读取指定毫秒时点的字幕区域快照，并以 JSONL 事件输出。

```text
arib-caption-worker.exe render-at <archive.caption.jsonl> <time_ms>
```

### `dump-tlv`

实验性 TLV/MMTP 原始证据抽出。未指定输出时默认生成 `.caption.mmtp.jsonl`。

```text
arib-caption-worker.exe dump-tlv <input> [output.caption.mmtp.jsonl] [--overwrite]
```

该命令只输出已发现 `stpp` asset 中完整的 closed-caption payload，并保留 TLV 偏移、MMTP/MPU 序号、原始 NTP 和无损字节。它不会把 NTP 冒充为 PTS，也不会将未知 asset 猜测为字幕。

## 开发与验证

Worker 可以单独构建和测试：

```powershell
cargo test -p arib-caption-worker
cargo build -p arib-caption-worker --release
```

涉及传输、协议、字幕模型、时间轴、渲染或导出器的修改，需要增加对应回归测试。合法但无法再分发的大型录制样本只保留在本地，通过 `ARIB_FIXTURE_DIR` 参与可选长样本验证。详细说明见 [语料与回归文档](docs/corpus.md)。

参与开发前请阅读 [贡献指南](CONTRIBUTING.md)、[中文架构文档](docs/architecture.zh-CN.md)、[后端接口合同](docs/backend-contract.md)、[工具链策略](docs/toolchain-policy.md) 和 [可维护性说明](docs/maintainability.md)。

## 限制

- BS4K/8K 的原始 TLV/MMTP 是隔离的实验能力，不属于已验证的通用 BS4K/8K 支持；
- 192-byte M2TS 支持仅说明包封装路线，不代表完整支持 BDMV/BDAV 目录、播放列表、CAS 或厂商私有录像管理信息；
- ResubWinny 不是录像管理器、直播接收器、CAS 解密工具或完整 EPG 浏览器；
- SRT 和 WebVTT 无法准确表达重叠区域、广播位置、Ruby 排版、DRCS 图形和全部 ARIB 时间语义；
- BS4K/8K 信号所对应标准 B62/ARIB-TTML 的规则仍处于研究状态。

源码发布与 Windows 二进制发布采用不同门槛，具体项目见 [发布检查清单](docs/release-checklist.md)。

## 余谈

对我而言，日本的电视文化有其独特魅力。略过电视上播出的内容本身，它的联播体制、技术细节同样令人着迷。旁人可能无法理解我的这种痴迷，但如果我说，在我居住的国家，电视信号只有单音轨（甚至没有原声重现）和画面呢？当我第一次接触日本数字电视时，看到它有可以开关的字幕，看到有可以互动的数据广播，有一种刘姥姥进了大观园的感觉。回想起小时候看电视时，某一天烧在画面上的字幕突然消失时，哭着闹着说看不懂了，让父母加开了几个儿童频道订阅的这件事情，会感觉我住的国家的电视文化能说道的只有它本身的历史，其余的除了苍白还是苍白。

但日本的电视信号，满满都是自己造的轮子和「加拉帕戈斯现象」的痕迹。一般人，离了那一套特供的收视环境，便难以看见它真实的面貌。得益于多年来开发者们的热情，信号源慢慢变得可以被解析，自由地看电视不再是一句空话，但一般人仍然缺乏便利与「可以被理解」的工具来迈过技术门槛。读懂文字是理解的开始，字幕是文字的承载体，于是我想从字幕开始做一个工具。

说回 ResubWinny 这个名字，原本的 Winny，是在 2002 年发布的 P2P 文件共享软件，该软件的广泛使用伴随着著作权内容与不当内容的传播，被视作社会问题，但软件作者却因为用户的行为遭到检控。2023 年，以该事件为题材的同名电影上映，在上海国际电影节展映的片名叫做「开发者有罪」。

Resub 代表对字幕的再加工，而 Winny 是为了向这个名字的发明者金子勇 a.k.a. 47氏致敬，也是为了遵循一个基本常识：

**开发通用技术并非犯罪，这是表达自由的一部分。**

本项目不涉及 P2P 网络、文件共享、媒体发现或内容分发。它是一个开源工具，读取媒体格式、恢复字幕、执行 OCR 或转换数据的工具本身并不构成侵权。它们的合法性和伦理性取决于它们的使用方式，而不仅仅是它们在技术上能够实现的功能。

制作 ResubWinny 的愿望，缘起于太多关于日本媒体处理的知识被困在难以整理的 2ch 帖子、废弃的 Windows 实用程序、ARIB 官方以阶级来划分可见性的文档，以及不提供源代码的软件中。这些知识应该更加开放、可审计、可移植且可保存。现有的工具也常常因为指引不明晰、理解有门槛将初学者劝退。

因此，这个项目并非意在重现原版 Winny。它是一种宣言：**开发者应享有构建合法工具的自由，用户应享有理解其所拥有媒体的自由，技术本身应简单易用，知识不应因恐惧、起诉或封闭代码而消失。**

## 特别鸣谢

- [xqq](https://github.com/xqq)，`libaribcaption` 的作者，长久以来为我研究日本电视广播提供了巨大的帮助
- [huggy](https://github.com/makeding)， `aribb62.js` 的作者，为本项目解析 BS4K/8K 信号字幕提供了支持
- [tsukumi](https://github.com/tsukumijima)， `KonomiTV` 等项目的作者，为「自由看电视」的文化长期贡献
- Bunny，我的女友，具有强大的技术背景，在开发过程中帮我解决了十分的难题
- Codex，OpenAI 开发的基于大语言模型的代理工具，没有它，缺乏技术基础的我就不可能用自然语言推动这个项目

## 许可证

ResubWinny 自有源代码采用 [Mozilla Public License 2.0](LICENSE)。修改 MPL 覆盖的源文件并分发时，需要按照 MPL-2.0 提供相应源代码；这不会自动要求与 ResubWinny 组合的所有独立模块采用 MPL。

第三方库、字体、二进制组件和测试语料继续遵循各自的许可证与来源要求。Windows 二进制分发必须同时满足 libmpv 的 LGPL 对应源码与可替换动态库要求。

安全问题请按照 [安全策略](SECURITY.md) 私下报告。贡献代码默认以 MPL-2.0 提供，具体要求见 [贡献指南](CONTRIBUTING.md)。

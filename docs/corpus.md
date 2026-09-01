[简体中文](corpus.md) | [English](corpus.en.md) | [日本語](corpus.ja.md) | [繁體中文](corpus.zh-TW.md)

> 本简体中文版本是唯一具有权威性的来源。

# 本地广播回归语料库

广播录像有意不提交或再分发。请将合法的本地样本放在任意目录中，并将 `ARIB_FIXTURE_DIR` 设置为该目录。测试有意不回退到隐式的开发者路径，因此长样本运行总会明确指出即将读取的语料库。

```powershell
$env:ARIB_FIXTURE_DIR = 'C:\tvrecords_testfile'
$env:ARIB_LONG_FIXTURE = '1'
cargo test -p arib-caption-worker decodes_ -- --nocapture
```

这些选择加入式检查会流式处理完整输入，并断言以下当前基线。它们不发布源字节、字幕或屏幕截图。

该语料库有意优先采用用户实际可以获得且内容已经验证的录像：地面 MPEG-TS 样本和 192 字节 MPEG-TS/TTML 样本是发布门禁。后者是分组化的 MPEG-TS 录像，不得用作已捕获原生 BS4K TLV 的证据。目前 TLV/MMTP 没有同等的本地发布样本；在取得合法真实捕获之前，其解析器、信令限制和原始证据契约由有界构造测试覆盖。

公开协议样本可从 worker 的 `synthetic` 模块获得：`make_ts_packet`、`make_pat`、`make_pmt`、`make_pes`、`make_b24_data_group` 和 `make_mmtp_packet` 为解析器测试构造确定性的分组和段边界，而不嵌入广播录像，也不声称具有广播机构特定语义。

若要在不完整扫描的情况下对发布工件进行冒烟检查，请运行：

```powershell
$env:ARIB_FIXTURE_DIR = 'C:\tvrecords_testfile'
.\scripts\validate-corpus.ps1
```

添加 `-Long` 可将两项完整转换运行到临时验证目录中。该脚本绝不会把输出写入语料库目录。

| 样本 | 路由 | 发布状态／所需证据 |
| --- | --- | --- |
| `chijo_digital_test.ts` | ISDB-T MPEG-TS / ARIB STD-B24 | **发布门禁。** 18,579,078,944 个输入字节；13,653 个 PES；2,230 个场景；2,736 个区域；29,892 个字符；61 个 DRCS 字形；0 个解码器错误。NIT 网络名称、当前 EIT 节目元数据和 TDT/TOT 广播时间必须全部存在。 |
| `bs4k_test.m2ts` | 192 字节录像机 M2TS / 私有 PES / ARIB-TTML | **发布门禁。** 11,517,020,160 个输入字节；330 个 PES；422 条 TTML 字幕；5,051 个字符；0 个解析器错误。同时间区域关联目前会在归档/ASS 输出前记录 31 个结构化 Ruby 绑定，其中包括从 `ささ` 到单个基础字素 `捧` 的绑定。 |
| `bs4k_test_2.ts` | 188 字节录像机 MPEG-TS / ARIB STD-B24 | **发布门禁。** 3,089,047,552 个输入字节；服务 101 从 ARIB SI 解码为 `NHK　BSP4K`；NIT 网络名称、当前 EIT 节目元数据和 TDT/TOT 广播时间必须全部存在；PID 0x0130 有 2,038 个 PES、118 条字幕、157 个区域、1,661 个字符及 0 个解码器错误；单独公布的 PID 0x0138 没有字幕事件，必须保持为空结果，不得伪造第二条轨道。 |
| 本地 38.07 GB 巴黎录像（不再分发） | 192 字节 M2TS / 私有 PES / 顺序 ARIB-TTML | **通用路由回归。** 内容探测发现服务 101、PMT `0x0100`、字幕 PID `0x1C00`（`component_tag 0x30`）及独立叠加字幕 PID `0x1C01`（`0x38`）。XML 具有完整 TTML 命名空间，但省略元素计时；无效的全零 PES PTS 会被拒绝，并由可感知回绕的 M2TS 到达时钟在同一 PID 的下一个文档处结束每个文档。完整的默认字幕转换读取 38,065,729,536 字节，并且必须保留 2,715 个字幕区域、28,618 个字符及 0 个解码器错误，输出须按单调顺序持续至 03:11:48。它不得以文件名、服务 ID、节目名称或固定 PID 值作为路由例外。 |
| 本地 20.12 GiB BS 录像（不再分发） | 188 字节 MPEG-TS / PMT 版本及字幕 PID 转换 | **动态 PMT 回归。** 初始 PMT 仅公开叠加字幕 PID `0x1C12`（`component_tag 0x38`）；后续当前 PMT 添加字幕 PID `0x1201`（`component_tag 0x30`）。检查必须仅报告 `0x1201`。完整转换读取 21,609,477,452 字节，并产生 18,722 个选中 PES、3,825 个场景、6,679 个区域、70,853 个字符、7 个 DRCS 字形及 0 个解码器错误。原始证据必须仅包含 PID `0x1201`。 |
| 构造的 PMT 版本转换 TS | MPEG-TS / B24 字幕与叠加字幕 | 固定大小发现窗口必须在初始仅叠加字幕的 PMT 之后找到较晚的字幕组件；顺序解码必须仅路由选中的逻辑 `service_id + component_tag`，并拒绝叠加字幕 PES。 |
| 构造的 188 字节私有 PES TS | MPEG-TS / PMT 私有 PID / 严格 ARIB-TTML | B24 发现保持为空；私有 PID 被发现；转换、ASS、TTML、归档、原始 PES 证据及有界预览均产生一条经过验证的 TTML 字幕。 |
| `testdata/golden/b62-layout.xml` | 构造的 ARIB-TTML 语义样本 | 稳定 JSON 摘要验证嵌套计时、百分比区域、横排 Ruby 证据、竖排书写模式、字号及颜色，而不再分发广播材料。单元回归还验证：等效声明的 1920×1080、3840×2160 和 7680×4320 像素布局会保留各自源平面与源值，并在映射到视频内容 viewport 后保持相同的观看者相对几何和文本长度。 |
| 构造的 TLV/MMTP `stpp` 样本 | ISDB-S3 TLV → MMTP → MPT/MPU | **仅限实验。** 验证边界、片段丢失、来源及证据优先的 `stpp` 路由；它不能将 TLV/MMTP 提升为具有发布门禁的路由。 |

解析器模糊测试保存在发布工作区之外的 `fuzz/` 中。初始目标覆盖基于内容的 TS/TLV 探测、严格 TTML 信封解码、有界 ARIB SI 服务名称文本解码、188/192 字节 TS PSI/PES 元数据解析及 MMTP/TLV 有效载荷信封。`cargo check --manifest-path fuzz/Cargo.toml` 提供稳定工具链编译检查；CI 还会在 Linux nightly 上使用 `cargo-fuzz` 构建所有目标。PSI/PES/B24 状态机以及更深层的信令/MPU 语义模糊测试目标仍是未来的语料库工作；每周工作流会在有界时间间隔内运行每个已声明目标。

对于视觉或格式更改，请在被忽略的验证目录中创建输出，并比较项目归档、ASS、TTML、原始 PES 证据以及未解析的 DRCS 资产目录。完整命令为：

```powershell
.\build\cargo\release\arib-caption-worker.exe convert `
  "$env:ARIB_FIXTURE_DIR\chijo_digital_test.ts" `
  artifacts\validation\chijo_digital_test.ass --ttml --archive --raw

.\build\cargo\release\arib-caption-worker.exe convert `
  "$env:ARIB_FIXTURE_DIR\bs4k_test.m2ts" `
  artifacts\validation\bs4k_test.ass --ttml --archive --raw
```

M2TS 样本尤其重要：其私有 PES 信封在有效 TTML 文档之前含有非 UTF-8 字节。回归不得仅仅因为其传输成帧不是 UTF-8 就拒绝整个 PES；XML 文本本身须严格依照其声明编码或 BOM 解码。

Windows 原生预览冒烟门禁使用 B24 `bs4k_test_2.ts` 样本，且不分发该录像：

```powershell
$env:ARIB_FIXTURE_DIR = 'C:\tvrecords_testfile'
.\scripts\validate-preview.ps1 -FixtureDirectory $env:ARIB_FIXTURE_DIR
```

它验证 WGL 宿主创建、进程内 libmpv 加载、渲染 worker 启动、录像打开及干净关闭。它有意与视觉屏幕截图验收分离，不得将其解释为硬件解码或像素保真度声明。

添加 `-Long` 可运行带阈值的 120 秒 Windows 4K 门禁：

```powershell
.\scripts\validate-preview.ps1 `
  -FixtureDirectory $env:ARIB_FIXTURE_DIR `
  -Long
```

该门禁保持 3840x2160 原生表面活动，三次替换完整的 1920x1080 后端字幕平面，并执行暂停、恢复、精确定位及关闭。若低于 20 presents/s，或启动超过 10 s、控制或字幕上传超过 1 s、关闭超过 3 s、工作集超过 2048 MiB，或 4K 预热后工作集增长超过 512 MiB，则失败。带模式版本的结果写入 `build/validation/preview-performance-windows-4k.json`。

2026-07-30 的真实语料库基线使用 `d3d11va-copy` 持续 120 秒达到 34.74 presents/s；峰值工作集为 1526.9 MiB，预热后增长为 111.9 MiB。这仅完成 Windows 4K 长门禁。它不是 8K、跨平台、DPI 或视觉保真度验收结果。测试框架本身使用 Cargo 的测试配置文件，同时加载与应用程序相同的捆绑 libmpv DLL 和原生 WGL 路由；打包发布验收另行跟踪。

2026-07-31 的打包 Windows 验收使用最终发布可执行文件和 `bs4k_test_2.ts`。内容探测选择了 188 字节 MPEG-TS/B24；原生 libmpv 在初始暂停时呈现视频；可见由 EIT/NIT/TOT 派生的频道、网络、节目、描述及广播时间；PID `0x0130` 产生 118 个解码事件。在流式归档从 `.jsonl.part` 更改为其发布的 `.jsonl` 路径后，任务时间线重新填充了真实字幕条，且没有“找不到归档”的诊断。这是该路由的打包回归结果，并非对每个桌面工作流的验收或 macOS/Linux 预览声明。

## 流式内存发布门禁

有界解析器常量是大型录像的必要证据，但并不充分。发布候选版本还必须完成至少一次 1 GiB 或更大 TS/M2TS 转换，且 Worker 峰值工作集不超过 **384 MiB**：

```powershell
.\scripts\validate-memory.ps1 `
  -Source "$env:ARIB_FIXTURE_DIR\chijo_digital_test.ts" `
  -TrackId 276
```

该脚本报告绝对峰值及每输入 GiB 对应的峰值 MiB。绝对门禁会捕获意外保留整个时间线/PES 的情况；比较 3 GiB、11 GiB 和 18 GiB 样本的比率可检查内存是否随录像时长线性增长。生成的输出保留在隔离临时目录中，并在测量后删除。

2026-07-27 测得的 Windows x86-64 发布基线：

| 样本 | 输入 | 峰值工作集 | 峰值/输入比率 |
| --- | ---: | ---: | ---: |
| `bs4k_test_2.ts`，PID 0x0130 | 2.877 GiB | 22.5 MiB | 7.83 MiB/GiB |
| `chijo_digital_test.ts`，PID 0x0114 | 17.303 GiB | 35.7 MiB | 2.06 MiB/GiB |

输入大小增加六倍时，绝对峰值仅增加 13.2 MiB，这与有界流式处理一致，而不是保留整个录像。这些数字是此机器的回归基线，并不保证每个解码器/运行时构建都具有相同的分配器开销。

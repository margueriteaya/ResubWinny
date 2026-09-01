[简体中文](backend-contract.md) | [English](backend-contract.en.md) | [日本語](backend-contract.ja.md) | [繁體中文](backend-contract.zh-TW.md)

> **规范性说明：** 简体中文版本是唯一权威来源。其他语言版本是同步译文；如措辞存在歧义或冲突，以简体中文版本为准。

# 后端合约

> 2026-09-02 实现说明：本文的逻辑 1920×1080 平面只是有界中间纹理，不是正确性的目标分辨率。
> Worker 在可选 `source_layout` 中保留源平面、region、样式和行内长度；原生渲染器由此显式计算
> 中间纹理，再将整张纹理映射到 libmpv 的视频内容 viewport。正确性以排除黑边后相对视频内容的比例为准。
Tauri/Svelte UI 是 Rust 后端的客户端。它不解析 TS/TLV 数据、解码 ARIB、渲染高分辨率视频或决定转换语义。

持久的 `.caption.jsonl` 格式在 [`contracts/archive.md`](contracts/archive.md) 中单独指定，包括其显式模式版本和流式读取器兼容性规则。

该合同分为重点阅读指南：[`contracts/tauri-api.md`](contracts/tauri-api.md)、[`contracts/worker-protocol.md`](contracts/worker-protocol.md)、[`contracts/preview.md`](contracts/preview.md) 和 [`contracts/timeline.md`](contracts/timeline.md)。该文件保留了兼容性索引和详细参考。

后端表面是一个有界的、稳定的应用契约。在当前的收敛阶段，更喜欢合并相关查询而不是添加新的一次性命令变体：

| 命令 | 责任 |
| --- | --- |
| `inspect_source` | 录音和字幕轨道发现的有界探测 |
| `start_export` | 启动流工作器并发出 `task-event` 进度；接受可选的经过验证的 `trackId` |
| `cancel_export` | 停止当前工作进程 |
| `pause_export` / `resume_export` | 向工作人员发送协作控制消息 |
| `create_job` / `list_jobs` / `get_job` / `remove_job` | 在没有媒体负载的情况下保留任务摘要 |
| `start_job` / `pause_job` / `resume_job` / `cancel_job` | 通过工人主管控制持久化作业 |
| `get_job_diagnostics` | 返回为持久作业收集的有界结构化诊断信息 |
| `get_job_diagnostics_window` | 使用偏移/限制返回有界诊断页 |
| `list_jobs_window` | 返回最近任务摘要的有界页面 |
| `get_job_artifacts` | 返回任务工件清单和 `.part` 路径 |
| `get_job_checkpoint` | 返回任务的最新有界进度检查点 |
| `pause_queue` / `resume_queue` / `queue_is_paused` | 控制 Supervisor 队列并协作暂停/恢复其活动 Worker |
| `load_drcs_report` | 读取工作人员生成的 DRCS 报告并返回可显示的字形图像 |
| `get_settings` / `update_settings` | 读取或自动更新经过验证的 UI 并导出应用程序数据 `settings.json` 中的默认值 |
| `list_language_packs` | 从固定的 app-data `language-packs/` 目录中重新扫描有界的 JSON 语言文件；不接受任意浏览器提供的目录 |
| `open_language_pack_directory` | 在需要时创建该固定目录并使用平台文件管理器打开它 |
| `start_preview` / `resize_preview` / `stop_preview` | 控制当前进程内 libmpv 视频表面 |
| `preview_command` | 将查找/暂停命令转发到 libmpv |
| `get_preview_capabilities` | 报告声明的视频/字幕合成路线以及仅当前可用的路线 |
| `get_preview_runtime` | 报告发现的 libmpv 运行时以及渲染 API 符号可用性，而不声明渲染表面存在 |
| `get_preview_render_diagnostics` | 报告活动的本机路由和有界渲染线程计数器/错误；缺少工作人员会返回稳定的非活动结果 |
| `render_at` | 返回请求的存档时间的有界字幕平面快照，而不通过 WebView 发送视频帧 |
| `sync_preview_overlay` | 读取嵌入的 libmpv 时间，渲染有界本机平面，并应用、清除或删除 Windows 覆盖层，无需 WebView 计时或布局 |
| `get_playback_time_mapping` / `update_playback_time_mapping` | 获取或替换本机字幕预览使用的经过验证的媒体时间→项目时间段映射 |
| `get_timeline_window` / `get_timeline_window_filtered` | 流式传输有界存档页面以供完成的任务浏览 |
| `get_timeline_recent_window_filtered` | 增量尾部完整的 JSONL 记录并仅返回最新的有界实时事件页面 |
| `get_timeline_time_window` | 返回编辑器时间线的有界预取时间范围并增量读取附加记录 |

存档导出完成后，`render_at` 将在任务工作区中公开。当存档包含 B24 渲染帧时，UI 保持时间查询显式且有界，并显示真正的 RGBA 派生 PNG。后端返回`planeWidth`、`planeHeight`、`composedPngBase64`、`activeLayerCount`；合成图像是由有界本机字幕平面合成器生成的，而不是由 CSS 或 WebView 文本布局生成的。具有有界布局字段的 TTML 间隔可以使用捆绑的 ARIB 字体的 Rounded M+ 1m 返回后端光栅化的 1920×1080 RGBA 平面。有效声明的显示范围将源几何图形和像素长度规范化到该逻辑平面上；缺失范围默认为逻辑 2K，并且仅从至少一个轴上超过逻辑 2K 且适合该平面的完整像素区域几何形状推断规范的 4K/8K。等效的 2K/4K/8K 布局保留相同的观看者相对尺寸，而无需猜测不明确的来源。有界富体解析器保留 span/ruby 标签外部的文本，并映射显式的 span 颜色、大小、间距和不透明度。本机水平路径保留显式换行符并应用已解析的 `textAlign`、`displayAlign` 和 `lineHeight`。简单的水平 `tts:ruby` 基本/文本对以 0.5 比例进行光栅化，并以其基本跨度为中心。明确关联的垂直 ruby 同样在其基本单元旁边以 0.5 比例进行光栅化，包括发生自动列换行时的有界延续；均报告 `captionPlaneMode=ttml-vertical-ruby-basic-native` 和 `renderedRubyCount`。此延续不实现一般的 B62 ruby 分组或特定于源的放置。仅当捆绑的 ARIB 字体包含映射的字形时，垂直渲染器才使用 Unicode 垂直表示标点符号；它从来不近似于拉丁旋转或tate-chu-yoko。 Direct `tts:textOutline` 仅接受 `none`、TTML 命名颜色或完整 `#RRGGBB[AA]` 加上 `px` 宽度，然后应用有界的原生轮廓； `arib-tt:border`是故意不转换的。完整的 B62 字形方向、标准 B62 笔划行为、非 PNG 资源以及无法渲染/缺失的字形仍然存在明确的限制；不受支持的记录仍然是结构预览而不是捏造的图像。

TLV 归档导出还可能包含有界 `asset_evidence` 和 `resource_evidence` 记录。每个 `resource_evidence` 记录都保留无损的 Base64 有效负载、格式验证以及匹配的 `subt://` 引用所使用的确切 `packet_id + mpu_sequence_number + subsample_number` 记录密钥。存档时预览阅读器最多保留 64 个此类记录，仅将相同 MPU 匹配附加到活动字幕，并将经过验证的小型 PNG `preview_data_uri` 公开为 `resourcePreviews`。字体资源、非 PNG 资源、缺失资源和不完整的地图仅保留证据，不会声明为渲染的标题文本。

单独的有界 `asset_evidence` 记录仅标识输入中已观察到的 MPT 信令（`packet_id`、源 TLV 偏移、`asset_type`、描述符标签和通告的 MPU NTP 值）。它们是未来 `subt://` 资源加入的证据，而不是解码的图像或字体字节。 `resource_reference` 记录携带原始 `packet_id + mpu_sequence_number` 范围。数字 `subt://` 索引永远不会被视为全局 MPT 数据包 ID：如果存在有界的相同 MPU 子样本，则关联为 `same-mpu-evidence` 并指向其原始资源记录；否则它仍显式保留为 `unresolved`。 `dump-tlv` 另外还发出完整的有界非 `stpp` MPU/MFU 有效负载，作为具有确定性范围密钥的 `mmt_asset_payload` 原始证据。此类记录可能包括 `format_hint`，但它只是有界二进制签名或有界标头观察（不是解码或渲染声明），而未知的资产语义仍未解决。 PNG 尺寸和字体表计数（如果存在）仅是结构元数据。小型、结构完整的 PNG 资源还可能为未来的本机预览表面携带有上限的 `data:` 预览值；后端仍然不解码或信任任意资源 URL。

该快照还带有 `renderProfile`。它的合同故意与 libaribcaption 兼容：使用捆绑的 `Rounded M+ 1m for ARIB` 系列，保留字符单元几何形状，将 ruby 保持在 0.5 相对比例，并从解码的源字符数据中获取背景 alpha 和描边颜色。发布的 libaribcaption 屏幕截图是面向观看者的视觉参考；其固定的本地基线和审核规则位于`docs/visual-reference.md`中。该配置文件的 B24 部分由解码器支持。当前的本机 TTML 路径使用捆绑字体、源前景/背景 RGBA、跨度样式运行、简单水平 ruby 和显式关联的垂直 ruby，包括跨自动列的有界延续。复杂的 ruby 分组、完整的垂直方向和标准笔划行为在测试其本机实现之前仍然是声明性元数据； UI 不得使用任意 CSS 阴影或固定黑框来模仿它们。 `captionOverlayModes` 是一系列结构化后端路由功能：`id`、`available`、`experimental` 和 `unavailableReasonCode`。在 Windows 上，当发现的运行时导出完整渲染 API 时，`libmpv-render` 变得可用；后端默认选择它，如果渲染工作启动失败，则按源回退到 `libmpv-client-overlay`。 UI 呈现后端的实际路线，并且从不选择渲染器本身。

## 工人活动信封

工作器 JSONL 事件使用 `protocolVersion`、`jobId`、`sequence` 和 `payload` 字段。为了兼容性，旧的顶级事件字段在迁移期间仍然存在。 Tauri 层必须在将事件转发到 Svelte 之前验证版本和序列。

工作线程首先发出 `hello`，然后是有界 `stage-changed`、`track-discovered`、进度、`diagnostic`、`drcs-discovered`、暂停/恢复、取消、`artifact-created`、完成或 `failed` 事件（如果适用）。每个成功发布的工件都会报告其稳定类型和完成前的最终路径； Tauri 使用该事件来更新原子 `app-data/jobs/{job-id}/artifacts.json` 清单，而不是从 UI 选项推断最终工件。检查点持久性属于 Tauri：只有在 `checkpoint.json` 原子发布后，它才会转发 `checkpoint-written`。 Tauri 在每次任务事件中转发稳定的 `code` 和 `parameters` 形状。时间线和诊断页面流式传输其 JSONL 源并仅保留请求的窗口；桌面不会在内存中缓存完整的存档或诊断历史记录。实时时间窗口 API 保留一个有界的预取窗口，并将字节光标移到新完成的 JSONL 行上，仅当请求的时间离开该窗口或工件被替换时才从磁盘重建。协议版本和序列违规保留其原始消息作为证据，但也携带命名参数，例如 `expected`、`actual`、`previous` 和 `current`； Svelte 本地化代码而不解析该消息。当工作人员提供的诊断参数是 JSON 对象时，将逐字保留。取消或失败时，工件状态将与 Worker 事件和文件证据进行协调：`completed` 表示 Worker 发布了它，`preserved` 表示预先存在的目标保持不变，`incomplete` 表示 `.part` 文件保留。 `failed` 或 `cancelled` 表示不存在更强的伪影证据。应用程序启动时，持久的活动状态变为 `Interrupted`，持久的 `Queued` 任务变为 `Ready`；内存队列永远不会自行恢复。 `resume_job` 仅在验证作业 ID、源、输出、轨道、源大小、进度范围和有界头/尾源指纹后重播 `Interrupted`、`Failed` 或 `Cancelled` 作业。仅报告时间戳更改，但当大小和指纹仍然匹配时接受。本机解码器和部分伪像状态未序列化，因此恢复当前从可信记录源执行完整重播，而不是声明字节精确恢复。

Worker 是独立可执行的，必须在 UI 集成之前进行测试：

```text
arib-caption-worker inspect recording.ts
arib-caption-worker convert recording.ts output.ass --overwrite --drcs-report
arib-caption-worker convert recording.m2ts output.ttml --ttml --overwrite
arib-caption-worker dump-tlv recording.tlv output.caption.mmtp.jsonl --overwrite
arib-caption-worker render-at output.caption.archive.jsonl 90000
```

已知的限制是产品限制，而不是隐藏的后备方案：

- SRT 不是正式的无损目标。
- 未识别的 TLV/MMTP 资产将作为原始证据保留，不会被猜测。
- `inspect_source` 返回稳定的 `routeCode`：`mpeg_ts_b24_verified` 立即由 B24 组件描述符进行验证。 `mpeg_ts_ttml_candidate` 表示在 188 字节 TS 或 192 字节 M2TS 中找到私有 PES PID，并且在转换期间仍然需要严格的 ARIB-TTML XML 验证。 `mpeg_ts_192_ttml_verified` 命名发布门控、成功验证的 192 字节 M2TS/TTML 转换路线；有界初始检查在看到有效的 TTML 文件之前不得声明它。 `tlv_mmtp_experimental` 有意以证据为先，在没有真实语料库的情况下不得将其呈现为一般 BS4K/8K 支持。
- 检查点当前从可信记录源执行源身份验证的完整重播，因为本机 B24 和部分工件状态不可序列化。
- 当前的 Windows 视频表面由进程内 `libmpv` 拥有；不使用 `mpv.exe` sidecar 或 JSON 命名管道。在运行时导出完整渲染 API 的情况下，后端选择 `libmpv-render`，拥有 WGL 上下文和 BGRA 纹理混合路径，并且仅在特定启动失败时才回退到客户端覆盖。它请求 `hwdec=auto-safe`，允许兼容的回拷加速，但不承诺零拷贝 D3D/ANGLE 互操作性。当加载的源报告时，`get_preview_render_diagnostics` 返回选定的路线、实时表面尺寸、每秒呈现数、纹理操作计数、方面、请求的解码器策略以及 libmpv 的实际 `hwdec-current`。长 2K/4K/8K 分析仍然是发布质量的门控，而不是隐含的功能。
- `get_preview_capabilities` 将每条路由报告为 `{ id, available, experimental, unavailableReasonCode }`。它只是一个演示契约：WebView 无法提交标题位图。 `render_preview_at`和`sync_preview_overlay`在后端组成有界的原生字幕平面，然后将其应用到libmpv。非 Windows 构建报告 `preview.platform_not_implemented`；它们并不意味着本机预览路线。
- `sync_preview_overlay` 报告 `mediaTimeMs` 和 `projectTimeMs`。它使用 `projectTimeMs` 查询字幕；默认映射是身份，但 PTS 修复、程序边界和用户偏移必须更新后端映射，而不是教导 WebView 第二个时钟。
- `trackId` 作为所有发现的 MPEG-TS B24 或 M2TS 数据轨道的经过验证的 PID 选择器传递。对于B24，选定的PID解析为逻辑`service_id + component_tag`磁道；顺序解码遵循当前 PAT/PMT 更新，并且可以在同一逻辑轨道的替换 PID 上继续。检查报告代表性 `caption_pid`、每个有界发现 `caption_pids`、组件标签、PAT/PMT 服务 ID、SDT 服务名称和 ISO-639 字幕语言。其 `broadcast` 对象还报告可选的 NIT 网络名称、当前服务 EIT 当前事件名称和描述以及 TDT/TOT UTC 广播时间。此 SI 通行证是基于内容的，使用单数据包工作缓冲区最多传输 64 MiB，并且当所选服务没有 EIT 时，绝不会替代另一服务的节目。缺少字段意味着记录没有提供有界窗口中的信息；它们不是解析器的猜测。广泛的 EPG 历史记录、CAS 和记录器元数据仍然不包含在产品合同中。队列管理器拥有暂停状态并向其活动 Worker 发送协作暂停/恢复控制；空闲暂停仍然会阻止下一个排队作业的启动。

专用 PES 轨迹发现报告 `pids`、`caption_pids` 和 `superimpose_pids`。组件标签 `0x30..0x37` 和 `0x38..0x3f` 对两种服务进行分类，但它们本身并不证明 B24 或 TTML：B24 仍然需要其数据组件描述符，而 TTML 仍然需要完整的、严格解码的 XML 文档。在没有显式 `trackId` 的情况下，转换和预览会选择声明的字幕组件并保持叠加组件独立。如果 PMT 描述符没有对私有流进行分类，则它仍然是候选流，而不是从其 PID 中猜测。

符合命名空间的 TTML 通过 XML 本地名称和祖先进行解析，包括默认或带前缀的 TTML 命名空间。连续的 ARIB-TTML 文档可能会省略 `begin`、`end` 和 `dur`；同一 PID 上的下一个完整文档关闭前一个文档，空的 `<tt>` 是清除操作。当 PES PTS 标记/前缀验证失败时，192 字节 M2TS 路由通过回绕处理从 30 位到达时间戳导出此文档时钟。它永远不会仅仅因为设置了 `PTS_DTS_flags` 就接受零填充的私有 PES 字段，并且它永远不会从到达另一个 PID 的文档中关闭一个 PID。

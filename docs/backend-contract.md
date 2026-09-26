[简体中文](backend-contract.md) | [English](backend-contract.en.md) | [日本語](backend-contract.ja.md) | [繁體中文](backend-contract.zh-TW.md)

> **规范性说明：** 简体中文版本是唯一权威来源。其他语言版本是同步译文；如措辞存在歧义或冲突，以简体中文版本为准。

# 后端合约

> 2026-09-02 实现说明：本文的逻辑 1920×1080 平面只是有界中间纹理，不是正确性的目标分辨率。
> Worker 在可选 `source_layout` 中保留源平面、region、样式和行内长度；原生渲染器由此显式计算
> 中间纹理，再将整张纹理映射到 libmpv 的视频内容 viewport。正确性以排除黑边后相对视频内容的比例为准。
Tauri/Svelte UI 是 Rust 后端的客户端。它不解析 TS/TLV 数据、解码 ARIB、渲染高分辨率视频或决定转换语义。

持久化使用的 `.caption.jsonl` 格式在 [`contracts/archive.md`](contracts/archive.md) 中单独指定，包括其显式 schema 版本和流式读取器兼容性规则。

合约细节按主题分为以下指南：[`contracts/tauri-api.md`](contracts/tauri-api.md)、[`contracts/worker-protocol.md`](contracts/worker-protocol.md)、[`contracts/preview.md`](contracts/preview.md) 和 [`contracts/timeline.md`](contracts/timeline.md)。本文继续作为兼容性索引和详细参考。

后端接口遵循范围明确、稳定的应用合约。在当前收敛阶段，优先整合相关查询，减少仅用于单一场景的命令变体：

| 命令 | 责任 |
| --- | --- |
| `inspect_source` | 在限定范围内检查录像并发现字幕轨道 |
| `start_export` | 启动流式 Worker 并发出 `task-event` 进度；接受可选的经过验证的 `trackId` |
| `cancel_export` | 停止当前 Worker 进程 |
| `pause_export` / `resume_export` | 向 Worker 发送协作控制消息 |
| `create_job` / `list_jobs` / `get_job` / `remove_job` | 在没有媒体负载的情况下保留任务摘要 |
| `start_job` / `pause_job` / `resume_job` / `cancel_job` | 通过 Worker 监管器控制持久化作业 |
| `get_job_diagnostics` | 返回为持久作业收集的有界结构化诊断信息 |
| `get_job_diagnostics_window` | 使用偏移/限制返回有界诊断页 |
| `list_jobs_window` | 返回最近任务摘要的有界页面 |
| `get_job_artifacts` | 返回任务工件清单和 `.part` 路径 |
| `get_job_checkpoint` | 返回任务的最新有界进度检查点 |
| `pause_queue` / `resume_queue` / `queue_is_paused` | 控制 Supervisor 队列并协作暂停/恢复其活动 Worker |
| `load_drcs_report` | 读取 Worker 生成的 DRCS 报告并返回可显示的字形图像 |
| `get_settings` / `update_settings` | 读取或原子更新应用数据 `settings.json` 中经过验证的界面设置与导出默认值 |
| `list_language_packs` | 从固定的 app-data `language-packs/` 目录中重新扫描有界的 JSON 语言文件；不接受任意浏览器提供的目录 |
| `open_language_pack_directory` | 在需要时创建该固定目录并使用平台文件管理器打开它 |
| `start_preview` / `resize_preview` / `stop_preview` | 控制当前进程内 libmpv 视频表面 |
| `preview_command` | 将跳转/暂停命令转发到 libmpv |
| `get_preview_capabilities` | 报告声明的视频/字幕合成路径以及仅当前可用的路线 |
| `get_preview_runtime` | 报告发现的 libmpv 运行时以及渲染 API 符号可用性，而不声明渲染表面存在 |
| `get_preview_render_diagnostics` | 报告活动的原生路径和有界渲染线程计数器/错误；缺少 Worker 会返回稳定的非活动结果 |
| `render_at` | 返回请求的存档时间的有界字幕平面快照，而不通过 WebView 发送视频帧 |
| `sync_preview_overlay` | 读取嵌入的 libmpv 时间，渲染有界本机平面，并应用、清除 Windows 覆盖层或跳过重复更新，无需 WebView 计时或布局 |
| `get_playback_time_mapping` / `update_playback_time_mapping` | 获取或替换本机字幕预览使用的经过验证的媒体时间→项目时间段映射 |
| `get_timeline_window` / `get_timeline_window_filtered` | 流式传输有界存档页面以供完成的任务浏览 |
| `get_timeline_recent_window_filtered` | 从文件末尾增量读取完整的 JSONL 记录并仅返回最新的有界实时事件页面 |
| `get_timeline_time_window` | 返回编辑器时间线的有界预取时间范围并增量读取附加记录 |

`render_at` 在档案导出完成后于任务工作区中暴露。UI 让时间查询保持显式且有界，并在档案含有 B24 渲染帧时显示真正由 RGBA 派生出的 PNG。后端返回 `planeWidth`、`planeHeight`、`composedPngBase64` 与 `activeLayerCount`；合成图像由有界的本机字幕平面合成器产出，而不是由 CSS 或 WebView 文本布局产出。带有有界布局字段的 TTML 区间，可以使用捆绑的 Rounded M+ 1m for ARIB 字体，返回一个由后端光栅化的 1920×1080 RGBA 平面。有效的已声明显示 extent 会把源几何与像素长度归一化到该逻辑平面上；缺少 extent 时默认按逻辑 2K 处理，且只在完整像素 region 几何至少在一个轴上超过逻辑 2K 并且能容纳于该平面时，才推断出规范的 4K/8K。等效的 2K/4K/8K 布局保持相同的观看者相对尺寸，而不去猜测含义不明的来源。有界的富文本正文解析器保留 span/ruby 标签之外的文本，并映射显式的 span 颜色、字号、字距与不透明度。本机水平路径保留显式换行，并应用解析后的 `textAlign`、`displayAlign` 与 `lineHeight`。简单的水平 `tts:ruby` base/text 对以 0.5 比例光栅化，并在其 base span 上居中。显式关联的垂直 ruby 同样以 0.5 比例光栅化在其 base 字符单元旁边，在发生自动分栏换行时带有有界延续；两者都报告 `captionPlaneMode=ttml-vertical-ruby-basic-native` 与 `renderedRubyCount`。该延续并不实现通用的 B62 ruby 分组或特定来源的摆放规则。只有当捆绑的 ARIB 字体包含所映射的字形时，垂直渲染器才使用 Unicode 竖排呈现标点；它绝不近似拉丁字符旋转或纵中横。直接的 `tts:textOutline` 只接受 `none`、TTML 具名颜色，或完整的 `#RRGGBB[AA]` 加 `px` 宽度，然后应用有界的本机描边；`arib-tt:border` 是刻意不做转换的。完整的 B62 字形方向、标准 B62 描边行为、非 PNG 资源，以及无法渲染或缺失的字形，仍是明确的限制；不受支持的记录仍以结构化预览呈现，而不是伪造出来的图像。

TLV 归档导出还可能包含有界 `asset_evidence` 和 `resource_evidence` 记录。每个 `resource_evidence` 记录都保留无损的 Base64 有效负载、格式验证以及匹配的 `subt://` 引用所使用的确切 `packet_id + mpu_sequence_number + subsample_number` 记录键。归档预览读取器最多保留 64 个此类记录，仅将同一 MPU 内匹配的资源附加到活动字幕，并将经过验证的小型 PNG `preview_data_uri` 公开为 `resourcePreviews`。字体资源、非 PNG 资源、缺失资源和不完整的资源映射仅保留证据，不会声明为已渲染的字幕文本。

另有一类有界的 `asset_evidence` 记录，只标识输入中已经观察到的 MPT 信令（`packet_id`、源 TLV 偏移、`asset_type`、描述符标签，以及所通告的 MPU NTP 值）。它们是将来接入 `subt://` 资源的证据，而不是已解码的图像或字体字节。`resource_reference` 记录携带其来源的 `packet_id + mpu_sequence_number` 作用域。数字形式的 `subt://` 索引绝不被当作全局 MPT 包 ID：若存在有界的同一 MPU 子样本，则关联标记为 `same-mpu-evidence` 并指向其原始资源记录；否则显式保持为 `unresolved`。`dump-tlv` 还会把完整而有界的非 `stpp` MPU/MFU 负载，以带确定性作用域键的 `mmt_asset_payload` 原始证据形式输出。此类记录可能带有 `format_hint`，但它只是有界的二进制签名观察或有界的头部观察（不是解码或渲染声明），未知的资产语义仍然保持未解析。PNG 尺寸与字体表数量若存在，也仅是结构性元数据。结构完整的小体积 PNG 资源，还可能携带一个有上限的 `data:` 预览值，供将来的本机预览表面使用；后端仍然不会解码或信任任意资源 URL。

该快照还带有 `renderProfile`。它的合约刻意与 libaribcaption 保持兼容：使用捆绑的 `Rounded M+ 1m for ARIB` 字族，保留字符单元几何，把 ruby 维持在 0.5 的相对比例，并从解码得到的源字符数据中取用背景 alpha 与描边颜色。已发布的 libaribcaption 截图是面向观看者的视觉参考；其固定的本地基线与审查规则见 `docs/visual-reference.md`。该 profile 的 B24 部分由解码器支撑。当前的本机 TTML 路径使用捆绑字体、源前景/背景 RGBA、span 样式区段、简单水平 ruby，以及显式关联的垂直 ruby，其中包含跨自动分栏的有界延续。复杂的 ruby 分组、完整的垂直排版方向与标准描边行为，在其本机实现通过测试之前仍只是声明性元数据；UI 不得用任意 CSS 阴影或固定黑框去模仿它们。`captionOverlayModes` 是一组结构化的后端路径能力：`id`、`available`、`experimental` 与 `unavailableReasonCode`。在 Windows 上，当发现的运行时导出完整渲染 API 时，`libmpv-render` 即变为可用；后端默认选择它，若渲染 Worker 启动失败，则按来源回落到 `libmpv-client-overlay`。UI 呈现后端实际采用的路径，绝不自行选择渲染器。

## Worker 事件信封

Worker 的 JSONL 事件使用 `protocolVersion`、`jobId`、`sequence` 与 `payload` 字段。为保持兼容，迁移期间旧的顶层事件字段依然保留。Tauri 层必须先校验版本与序号，才能把事件转发给 Svelte。

Worker 首先发出 `hello`，随后按适用情况发出有界的 `stage-changed`、`track-discovered`、进度、`diagnostic`、`drcs-discovered`、暂停/恢复、取消、`artifact-created`、完成或 `failed` 事件。每个成功发布的产物都会在完成之前报告其稳定类型与最终路径；Tauri 依据该事件更新原子的 `app-data/jobs/{job-id}/artifacts.json` 清单，而不是从 UI 选项推断最终产物。检查点持久化归 Tauri 负责：只有在 `checkpoint.json` 原子发布之后，它才转发 `checkpoint-written`。Tauri 在每个任务事件上都转发稳定的 `code` 与 `parameters` 结构。时间轴与诊断页面以流式读取其 JSONL 来源，只保留所请求的窗口；桌面端不会把完整的档案或诊断历史缓存在内存中。实时时间窗口 API 只保留一个有界的预取窗口，并在新写完的 JSONL 行上推进字节游标，仅当请求时间离开该窗口或产物被替换时才从磁盘重建。协议版本与序号违规会保留其原始消息作为证据，同时携带 `expected`、`actual`、`previous`、`current` 等具名参数；Svelte 本地化的是错误码，而不解析该消息。Worker 提供的诊断参数若为 JSON 对象，则逐字保留。取消或失败时，产物状态由 Worker 事件与文件证据共同核对得出：`completed` 表示 Worker 已发布，`preserved` 表示既有目标文件未被触动，`incomplete` 表示仍留有 `.part` 文件。`failed` 或 `cancelled` 表示不存在更强的产物证据。应用启动时，持久化的活动状态变为 `Interrupted`，持久化的 `Queued` 任务变为 `Ready`；内存队列绝不自行恢复。`resume_job` 只在校验作业 ID、来源、输出、轨道、来源大小、进度上界以及有界的首尾来源指纹之后，才重放 `Interrupted`、`Failed` 或 `Cancelled` 作业。仅时间戳发生变化会被报告，但在大小与指纹仍然一致时予以接受。本机解码器与部分产物状态不做序列化，因此当前的恢复是从可信的录制来源完整重放，而不是声称可做字节级精确续传。

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
- `inspect_source` 返回稳定的 `routeCode`：`mpeg_ts_b24_verified` 由 B24 组件描述符立即验证。`mpeg_ts_ttml_candidate` 表示在 188 字节 TS 或 192 字节 M2TS 中找到了私有 PES PID，并且在转换期间仍需通过严格的 ARIB-TTML XML 校验。`mpeg_ts_192_ttml_verified` 指的是受发布门禁约束、已成功校验的 192 字节 M2TS/TTML 转换路径；有界的初次检查在尚未见到有效 TTML 文档之前不得声明该路径。`tlv_mmtp_experimental` 有意以证据优先，在没有真实语料之前，不得把它呈现为通用的 BS4K/8K 支持。
- 当前从检查点恢复时，会先验证来源身份，再从可信录像源完整重放处理流程，因为原生 B24 解码器和部分产物状态无法序列化。
- 当前的 Windows 视频表面由进程内 `libmpv` 拥有；不使用 `mpv.exe` sidecar 或 JSON 命名管道。在运行时导出完整渲染 API 的情况下，后端选择 `libmpv-render`，拥有 WGL 上下文和 BGRA 纹理混合路径，并且仅在特定启动失败时才回退到客户端覆盖。它请求 `hwdec=auto-safe`，允许兼容的回拷加速，但不承诺零拷贝 D3D/ANGLE 互操作性。`get_preview_render_diagnostics` 返回所选路径、当前表面尺寸、每秒呈现帧数、纹理操作计数、宽高比和请求的解码器策略；如果已加载来源提供相应信息，还会返回 libmpv 实际使用的 `hwdec-current`。长时间 2K/4K/8K 性能测试仍属于发布质量门槛。
- `get_preview_capabilities` 把每条路径报告为 `{ id, available, experimental, unavailableReasonCode }`。它只是一份呈现层合约：WebView 无法提交字幕位图。`render_preview_at` 与 `sync_preview_overlay` 在后端内部合成有界的本机字幕平面，再把它应用到 libmpv 上。非 Windows 构建报告 `preview.platform_not_implemented`；这并不意味着存在本机预览路径。
- `sync_preview_overlay` 报告 `mediaTimeMs` 和 `projectTimeMs`。它使用 `projectTimeMs` 查询字幕；默认采用恒等映射，即媒体时间与项目时间一致。PTS 修复、节目边界和用户偏移必须通过更新后端映射处理；WebView 不维护第二套时钟。
- `trackId` 作为所有发现的 MPEG-TS B24 或 M2TS 数据轨道的经过验证的 PID 选择器传递。对于 B24，选定的 PID 对应逻辑 `service_id + component_tag` 轨道；顺序解码遵循当前 PAT/PMT 更新，并且可以在同一逻辑轨道的替换 PID 上继续。检查报告代表性 `caption_pid`、检查范围内发现的全部 `caption_pids`、组件标签、PAT/PMT 服务 ID、SDT 服务名称和 ISO-639 字幕语言。其 `broadcast` 对象还报告可选的 NIT 网络名称、当前服务 EIT 当前事件名称和描述以及 TDT/TOT UTC 广播时间。这轮 SI 检查以内容为依据，使用单个数据包大小的工作缓冲区，最多读取 64 MiB，并且当所选服务没有 EIT 时，绝不会用另一服务的节目补齐。缺少字段意味着录像在限定检查范围内未提供相应信息；它们不是解析器的猜测。广泛的 EPG 历史记录、CAS 和录像机元数据仍然不包含在产品合约中。队列管理器拥有暂停状态并向其活动 Worker 发送协作暂停/恢复控制；空闲暂停仍然会阻止下一个排队作业的启动。

私有 PES 轨道发现报告 `pids`、`caption_pids` 和 `superimpose_pids`。组件标签 `0x30..0x37` 和 `0x38..0x3f` 对两种服务进行分类，但它们本身并不证明 B24 或 TTML：B24 仍然需要其数据组件描述符，而 TTML 仍然需要完整的、严格解码的 XML 文档。在没有显式 `trackId` 的情况下，转换和预览会选择声明的字幕组件并保持文字叠加组件独立。如果 PMT 描述符没有对私有流进行分类，则它仍然是候选流，而不是从其 PID 中猜测。

符合命名空间的 TTML 通过 XML 局部名称和祖先元素进行解析，包括默认或带前缀的 TTML 命名空间。连续的 ARIB-TTML 文档可能会省略 `begin`、`end` 和 `dur`；同一 PID 上的下一个完整文档关闭前一个文档，空的 `<tt>` 是清除操作。当 PES PTS 标记/前缀验证失败时，192 字节 M2TS 路径会处理时间戳回绕，并从 30 位到达时间戳推导文档时钟。它永远不会仅仅因为设置了 `PTS_DTS_flags` 就接受零填充的私有 PES 字段，并且它不会用另一个 PID 上到达的文档结束当前 PID 的字幕文档。

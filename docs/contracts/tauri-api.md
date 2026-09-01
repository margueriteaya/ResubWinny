[简体中文](tauri-api.md) · [繁體中文](tauri-api.zh-TW.md) · [日本語](tauri-api.ja.md) · [English](tauri-api.en.md)

> 简体中文版本是唯一权威来源。其他语言版本仅为同步译文。

# Tauri API 合同

Svelte 应用是 Rust 应用层的客户端。它不解析 TS/TLV、不解码 ARIB、不渲染视频，也不决定转换语义。

公开命令面列于 [`../backend-contract.md`](../backend-contract.md)。本页按职责对这些命令分组：

- 检查与导出：`inspect_source`、`start_export`、`cancel_export`、`pause_export`、`resume_export`；
- 持久化任务与恢复：`create_job`、`list_jobs`、`get_job`，以及任务控制、诊断、产物、检查点与队列控制；
- 偏好设置与 DRCS：设置、语言包及 DRCS 报告加载；
- 预览与时间轴：原生预览控制、archive 渲染、播放映射与有界时间轴窗口。

命令必须返回有界数据和稳定错误代码。界面不得根据选项推断产物，也不得虚构后端没有提供的能力。

## 接口面冻结

当前命令面处于收敛期。在底层模型仍在稳定时，不应继续增加现有查询的一次性变体。
时间轴查询下次需要整合时，应优先使用一个带参数的 `query_timeline` 请求（显式指定模式、
时间范围和过滤器），而不是增加更多 `get_timeline_*` 命令。此类迁移必须保持响应有界、
archive 游标语义、稳定错误代码，并同步更新前端合同。

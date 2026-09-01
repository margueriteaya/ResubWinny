[简体中文](worker-protocol.md) · [繁體中文](worker-protocol.zh-TW.md) · [日本語](worker-protocol.ja.md) · [English](worker-protocol.en.md)

> 简体中文版本是唯一权威来源。其他语言版本仅为同步译文。

# Worker 协议合同

Worker 消息使用 `protocolVersion`、`jobId`、`sequence` 和 `payload`。
迁移期间保留旧版顶层字段。Worker 首先发出 `hello`，随后按需发出有界的
阶段、轨道、进度、诊断、产物、完成或失败事件。

Tauri 在转发事件前验证协议版本和序列。验证失败时，原始消息会与结构化的
`expected`、`actual`、`previous` 或 `current` 参数一同作为证据保留。
产物状态由 Worker 事件和文件证据推导；界面绝不猜测任务是否完成。

Worker 负责探测/解复用/解码、Caption IR、导出、archive 和证据。
任务历史、队列状态、检查点、设置及窗口生命周期仍归 Tauri 应用层所有。

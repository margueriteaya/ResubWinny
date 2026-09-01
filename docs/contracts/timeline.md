[简体中文](timeline.md) · [繁體中文](timeline.zh-TW.md) · [日本語](timeline.ja.md) · [English](timeline.en.md)

> 简体中文版本是唯一权威来源。其他语言版本仅为同步译文。

# 时间轴合同

时间轴 API 流式返回有界的 archive 窗口，而不是在桌面界面中缓存完整 archive：

- `get_timeline_window` 及其过滤变体对已完成的 archive 分页；
- `get_timeline_recent_window_filtered` 跟随读取完整的 JSONL 记录；
- `get_timeline_time_window` 返回有界时间范围，并在新增记录中推进字节游标。

最后一行 JSONL 尚不完整时，读取器会忽略它，直到后续追加使其完整。
时间轴记录使用项目时间的毫秒字段；预览的媒体时钟必须显式映射，不得以含义不明的
时间值泄漏到接口中。Archive 格式与 schema 规则见 [`archive.md`](archive.md)。

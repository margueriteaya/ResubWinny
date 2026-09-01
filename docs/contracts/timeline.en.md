[简体中文](timeline.md) · [繁體中文](timeline.zh-TW.md) · [日本語](timeline.ja.md) · [English](timeline.en.md)

> This is a translation. The Simplified Chinese version is the sole authoritative source.

# Timeline contract

Timeline APIs stream bounded archive windows rather than caching complete
archives in the desktop UI:

- `get_timeline_window` and filtered variants page completed archives;
- `get_timeline_recent_window_filtered` tails complete JSONL records;
- `get_timeline_time_window` returns a bounded time range and advances a byte
  cursor over appended records.

Readers ignore incomplete final JSONL lines until a later append completes
them. Timeline records use project-time millisecond fields; preview's media
clock is mapped explicitly and must not leak through as an ambiguous time
value. Archive format and schema rules are in [`archive.md`](archive.md).

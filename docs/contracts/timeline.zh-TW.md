[簡體中文](timeline.md) · [繁體中文](timeline.zh-TW.md) · [日本語](timeline.ja.md) · [English](timeline.en.md)

> 本頁是譯文。簡體中文版本是唯一權威來源。

# 時間軸合約

時間軸 API 以串流方式回傳有界的 archive 視窗，而不在桌面介面中快取完整 archive：

- `get_timeline_window` 及其篩選變體對已完成的 archive 分頁；
- `get_timeline_recent_window_filtered` 追蹤讀取完整的 JSONL 記錄；
- `get_timeline_time_window` 回傳有界時間範圍，並在新增記錄中推進位元組遊標。

最後一行 JSONL 尚未完整時，讀取器會忽略它，直到後續附加使其完整。
時間軸記錄使用專案時間的毫秒欄位；預覽的媒體時鐘必須明確對映，不得以語意不明的
時間值洩漏至介面。Archive 格式與 schema 規則見 [`archive.md`](archive.md)。

[简体中文](archive.md) · [繁體中文](archive.zh-TW.md) · [日本語](archive.ja.md) · [English](archive.en.md)

> 简体中文版本是唯一权威来源。其他语言版本仅为同步译文。

# 字幕 archive 合同

字幕 archive 是 UTF-8 JSON Lines（`.caption.jsonl`）格式。它是项目的持久中间表示；
可从中派生 ASS、TTML 和预览输出，而不把这些呈现格式视为无损格式。

## 文件头与 schema 版本

第一行完整记录是 archive 文件头：

```json
{"type":"arib_caption_studio_archive","schemaVersion":1,"version":1,"source":"recording.ts","route":"arib_std_b24","format":"jsonl"}
```

`schemaVersion` 是权威的 archive 兼容性版本。版本 1 还把原有的 `version` 字段写作兼容别名；
两个值必须一致。新的写入器不得在不递增 `schemaVersion` 的情况下，静默改变现有记录的含义或结构。

只需要有界时间轴或预览记录的读取器可以忽略未知记录类型。需要完整语义保真度的读取器必须拒绝
不受支持的 `schemaVersion`，而不能猜测。显式 `schemaVersion` 字段引入前生成的文件使用
`version: 1`，仍属于版本 1 archive。

## 记录

之后的每一完整行都是带稳定 `type` 的独立 JSON 对象。字幕 payload 记录使用
`{"type":"caption","value":{...}}` 形式的 envelope；其他现有类型包括 `region_interval`、
`scene`、`resource_reference`、`resource_evidence`、`asset_evidence` 和 `summary`。

转换运行期间，写入器会 flush 完整字幕记录，使桌面端能够跟随读取文件。读取器必须忽略不完整的
最后一行，直到后续追加使其完整。B24 与 B62 的传输专属证据保持分离；公共语义通过字幕记录表达，
而不是假装两种传输共用同一个解码器模型。

在 Worker 内，两条路线都要先跨越封闭、零拷贝的 `CaptionCueRef` 语义边界，再发布到 archive。
它统一时间、区域、路线标识、纯文本、ruby 数量和 DRCS 存在性，同时保留每条路线的忠实 payload。
样式、字形像素与 TTML 资源证据仍为路线专属。因此 schema v1 继续把 B24 发布为
`region_interval`、把 ARIB-TTML 发布为 `caption`；共享的内部边界不会重命名或复制记录。

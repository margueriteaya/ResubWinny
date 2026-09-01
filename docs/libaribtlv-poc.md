# libaribtlv B62 抽取后端

[简体中文](libaribtlv-poc.md) · [繁體中文](libaribtlv-poc.zh-TW.md) · [日本語](libaribtlv-poc.ja.md) · [English](libaribtlv-poc.en.md)

> **规范性说明：**简体中文版本是唯一权威来源；如同步译文存在冲突，以本文件为准。

可选的 Worker `libaribtlv` feature 为 ARIB STD-B62 字幕提供有界的原生 TLV/MMTP 解复用路径。它只是实验性、证据优先 TLV 路线的一项实现增量，不构成通用 BS4K/8K 支持声明，也不包含播放器或 MSE 集成。

已审查的依赖为 `makeding/libaribtlv` 0.6.1（C API 版本 6，commit `a84e5b62bf9230d3fcea21c66e62f7cc5d50a3c2`）及 Zlib 1.3.2（commit `da607da739fa6047df13e66a2af6b8bec7c2a498`）。两份完整源码快照均位于 `third_party/`，由 `third_party/versions.json` 固定，并记录于 `THIRD_PARTY_NOTICES.md`。运行时和 feature 构建过程均不会下载依赖。

## 构建与测试

项目自有桥接会从 vendored 快照静态构建 libaribtlv 及其私有 Zlib；无需 `CMAKE_PREFIX_PATH`、外部 checkout 或系统 Zlib：

```powershell
cargo test -p arib-caption-worker --features libaribtlv
```

窄 C ABI 只暴露字幕轨、access unit、同 MPU 字幕资源、归一化时间戳、random-access/discontinuity 后设数据与解析错误。Rust 会在 callback 返回前复制所有短生命周期字符串和字节视图；不收集 ARIB-HTML5 application resource，也不接收音视频 access unit。

## 路由与证据规则

启用 feature 后，原生后端接管 TLV→B62 TTML 扫描，并以有界分块流式读取。archive 分开保留 packet/track 身份、可用的 MPU/MMTP sequence、归一化有理数 PTS、时间原点、discontinuity 与实际 MPT presentation NTP。缺失值保持缺失；绝不把 PTS 写成 NTP，也不把 NTP 猜作 PTS。

只有 compression type 0 会进入现有的严格、自包含 XML TTML 解码器。compression type 1/2（EXI）、未知压缩/格式/data type、非自包含 XML、畸形文档及不完整资源只保留原始证据与诊断。同 MPU 资源只有在 demuxer 提供 MPU scope 时才可标为完整。

在合法真实流语料和可信参考画面通过验证前，不得宣称通用 BS4K/8K 支持。公开测试只使用构造的协议夹具；私有广播录像不得再分发。

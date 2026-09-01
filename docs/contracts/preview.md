[简体中文](preview.md) · [繁體中文](preview.zh-TW.md) · [日本語](preview.ja.md) · [English](preview.en.md)

> 简体中文版本是唯一权威来源。其他语言版本仅为同步译文。

# 预览合同

原生预览由 Tauri 后端和进程内 libmpv 所有。WebView 只提供命令并显示有界状态；
它绝不提交字幕位图，也不执行字幕排版。

`render_at` 与 `sync_preview_overlay` 使用显式的项目时间映射，并同时返回媒体时间与
项目时间。后端从 archive 合成字幕平面，报告所选 overlay 路线和能力元数据，
并以声明方式保留不受支持的 B62 特性，而不是用 CSS 近似。
详细渲染配置与路线限制见 [`backend-contract.md`](../backend-contract.md)。

对于带 `source_layout` 的 ARIB-TTML，后端先按源显示平面比例生成有界的中间字幕纹理，再把整张纹理映射到
libmpv 报告的实际视频内容 viewport。letterbox/pillarbox、窗口大小、DPI 与全屏只改变最终变换，不改变字幕
相对于视频内容的位置和面积。旧 archive 没有 `source_layout` 时继续按逻辑 1920×1080 兼容路径播放。

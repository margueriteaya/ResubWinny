[简体中文](preview-composition.md) · [繁體中文](preview-composition.zh-TW.md) · [日本語](preview-composition.ja.md) · [English](preview-composition.en.md)

> **规范性说明：** 简体中文版本是唯一权威来源。其他语言版本是同步译文；如措辞存在歧义或冲突，以简体中文版本为准。

# 预览合成方案决定

Windows 预览使用进程内 `libmpv-render`。ResubWinny 不启动 `mpv.exe`，也不使用 JSON 命名管道。macOS 与 Linux 沿用同一套字幕模型，但原生预览后端尚未实现。

libmpv 与 Rust 字幕在专用渲染线程的离屏 FBO 中合成。一个从不显示、没有父窗口并位于屏幕外的 WGL host 只负责提供 OpenGL device context；它不承载视频画面，也不参与应用窗口的层叠、移动或缩放。当前路径不创建可见或子级 HWND，也没有 `wid`、`overlay-add` 或按来源回退。

合成后的 RGBA 帧通过轮换的 PBO 读回。每帧在提交后立即映射，避免让视频固定落后 libmpv 音频。三个 WebView2 SharedBuffer 像素槽把帧交给播放器组件内的 WebGL Canvas；控制槽只传递序号、尺寸与槽状态。Canvas 只领取最新的就绪帧，并释放已上传或过期的槽，因此逐帧像素不经过 Tauri 命令、事件或 COM 复制。

窗口移动、面板折叠和 DPI 变化只改变 Canvas 的 DOM 布局。ResizeObserver 将组件的物理像素尺寸发送给后端，后端只调整离屏 FBO；画面与播放器控件始终处于同一个 WebView 内容层，不再追踪原生窗口坐标。共享表面限定为 1920×1080，较大的源视频由 libmpv 缩放到组件实际尺寸。

`get_preview_capabilities` 只报告当前可用的 `libmpv-render` 路线。`get_preview_render_diagnostics` 报告渲染、呈现与丢弃帧数、表面尺寸、呈现节奏、字幕纹理操作、视频宽高比、解码策略及最近错误。`render_at(time)` 继续用于存档时间点的有界字幕快照；`sync_preview_overlay` 仍负责按 libmpv 媒体时间查询并更新 Rust 字幕纹理，不在 WebView 中建立第二套播放时钟或字幕排版器。

渲染线程请求 `hwdec=auto-safe`，并在可用时报告 libmpv 实际选择的 `hwdec-current`。这允许兼容的回拷硬件解码，不表示 D3D/ANGLE 零拷贝。应用退出和预览停止都会先停止并 join 渲染线程，再在 WebView UI STA 释放 SharedBuffer，最后销毁隐藏的 WGL host。

`scripts/validate-preview.ps1` 使用 `ARIB_FIXTURE_DIR` 中合法持有的本地录像执行 Windows 冒烟门禁。门禁覆盖真实解码首帧、seek、暂停/继续、字幕纹理混合、尺寸变化、截图和有界关闭。私有广播素材不会提交到仓库；公开回归依赖合成与协议测试。

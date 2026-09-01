# 支援的工具鏈

[简体中文](toolchain-policy.md) · [繁體中文](toolchain-policy.zh-TW.md) · [日本語](toolchain-policy.ja.md) · [English](toolchain-policy.en.md)

> **規範性聲明：** 簡體中文版本是唯一的權威來源。其他語言版本均為同步翻譯；若措辭有歧義或衝突，以簡體中文版本為準。

本儲存庫透過 `rust-toolchain.toml` 將 Rust 固定為 `1.97.1`。CI 與本機候選發行版本建置必須使用該檔案，不得使用未限定版本的 `stable`。同一工具鏈中的 Rustfmt 與 Clippy 也是門檻的一部分。

桌面前端支援 Node.js 22 LTS，並使用已提交的 npm 鎖定檔。驗證與封裝必須使用 `npm ci`，不得進行不受限制的相依套件更新。較新的 Node 版本可能可在本機運作，但並非發行基準。

Windows 11 x86-64 是 Alpha 套件及原生預覽的驗收平台。Worker、Tauri 編譯與前端檢查仍會在 Windows、macOS 與 Linux 上進行，但 macOS/Linux 原生預覽後端則延後實作。

工具鏈升級屬於刻意進行的相依套件變更，必須滿足：

1. 審查發行說明與相容性；
2. 通過 Worker、桌面端、前端與模糊測試的編譯門檻；
3. 審查鎖定檔，且不得夾帶無關的套件變動；
4. 完成 Windows 封裝預覽與長樣本迴歸測試；以及
5. 在同一次變更中更新 CI、本文件與貢獻者說明。

任何應用程式元件都不得在執行階段安裝編譯器、套件管理員或建置工具。

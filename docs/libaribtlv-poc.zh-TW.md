# libaribtlv B62 擷取後端

[简体中文](libaribtlv-poc.md) · [繁體中文](libaribtlv-poc.zh-TW.md) · [日本語](libaribtlv-poc.ja.md) · [English](libaribtlv-poc.en.md)

> **規範性說明：**簡體中文版本是唯一權威來源；若同步譯文有衝突，以簡體中文文件為準。

選用的 Worker `libaribtlv` feature 為 ARIB STD-B62 字幕提供有界的原生 TLV/MMTP 解多工路徑。它只是實驗性、證據優先 TLV 路線的一項實作增量，不構成通用 BS4K/8K 支援聲明，也不包含播放器或 MSE 整合。

已審查的相依項目為 `makeding/libaribtlv` 0.6.1（C API 版本 6，commit `a84e5b62bf9230d3fcea21c66e62f7cc5d50a3c2`）及 Zlib 1.3.2（commit `da607da739fa6047df13e66a2af6b8bec7c2a498`）。兩份完整原始碼快照均位於 `third_party/`，由 `third_party/versions.json` 固定，並記錄於 `THIRD_PARTY_NOTICES.md`。執行時與 feature 建置過程均不會下載相依項目。

## 建置與測試

專案自有 bridge 會從 vendored 快照靜態建置 libaribtlv 及其私有 Zlib；不需要 `CMAKE_PREFIX_PATH`、外部 checkout 或系統 Zlib：

```powershell
cargo test -p arib-caption-worker --features libaribtlv
```

窄 C ABI 只公開字幕軌、access unit、相同 MPU 字幕資源、正規化時間戳、random-access/discontinuity 後設資料與解析錯誤。Rust 會在 callback 返回前複製所有短生命週期字串與位元組 view；不收集 ARIB-HTML5 application resource，也不接收音訊／視訊 access unit。

## 路由與證據規則

啟用 feature 後，原生後端接管 TLV→B62 TTML 掃描，並以有界分塊串流讀取。archive 分別保留 packet/track 身分、可用的 MPU/MMTP sequence、正規化有理數 PTS、時間原點、discontinuity 與實際 MPT presentation NTP。缺少值維持缺少；絕不把 PTS 寫成 NTP，也不把 NTP 猜成 PTS。

只有 compression type 0 會進入現有的嚴格、自包含 XML TTML decoder。compression type 1/2（EXI）、未知 compression/format/data type、非自包含 XML、畸形文件及不完整資源只保留原始證據與診斷。相同 MPU 資源只有在 demuxer 提供 MPU scope 時才可標記為完整。

在合法真實串流語料與可信參考畫面通過驗證前，不得宣稱通用 BS4K/8K 支援。公開測試只使用構造的協定 fixture；私人廣播錄影不得再次散布。

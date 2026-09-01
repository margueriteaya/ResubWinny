# libmpv 原始碼與發行合規性

[簡體中文](libmpv-source-compliance.md) · [繁體中文](libmpv-source-compliance.zh-TW.md) · [日本語](libmpv-source-compliance.ja.md) · [English](libmpv-source-compliance.en.md)

> **規範性說明：** 簡體中文版本是唯一權威來源。其他語言版本僅為同步譯文；如有歧義或衝突，以簡體中文版本為準。

ResubWinny 在 Windows 上動態載入可替換的 LGPL 建置 libmpv。隨附的開發 DLL 已固定並經雜湊檢查，但其上游建置使用會變動的相依性分支，且未將完整原始碼快取作為發行成品公開。因此，目前 DLL 僅獲準用於開發和私有測試，不得用於 ResubWinny 的首次公開二進位發行。

## 建置設定

基礎 Tauri 設定不隨附 libmpv 二進位檔。因此，它可以在不下載或再散佈該函式庫的情況下建置應用程式和安裝套件；此時預覽需要使用者提供相容執行階段。一般的一鍵建置使用明確 Windows libmpv 設定，以產生可用於開發/私有測試的套件：

```powershell
./scripts/build.ps1
```

外部執行階段套件同樣明確：

```powershell
./scripts/build.ps1 -Libmpv External
```

兩種設定均不得削弱公開發行規則。含有 `libmpv-2.dll` 的套件必須待與之相符的完整對應原始碼、原始碼收據、宣告和二進位雜湊一併釋出後，方可釋出。只有隨附 DLL 的雜湊保持完全一致時，同一經驗證的對應原始碼成品才可供後續 ResubWinny 建置重用。

## 必需的公開發行建置

發行建置必須從 `third_party/versions.json` 中記錄的提交執行，並且在下載或編譯套件之前套用已審查的僅 LGPL 修補。同一個建置工作必須保留：

1. 精確的建置配方取出；
2. 已套用修補的 `mpv-winbuild-cmake` 取出；
3. 下載和更新後完整的 `src_packages` 目錄；
4. 產生的 DLL 與匯入程式庫雜湊；
5. 套件提交/狀態收據；
6. 重建所需的全部建置選項、修補、授權條款文字和指令碼；以及
7. 已記錄的 runner、工具鏈和原生套件環境。

針對這三個原始碼目錄以及包含新建置 DLL/匯入程式庫的解壓目錄執行 `scripts/package-libmpv-source.ps1`，並傳入同一工作產生的建置環境記錄。該指令碼拒絕缺少核心套件的原始碼快取，驗證固定配方與工具鏈祖先、LGPL 設定，記錄每個原始碼套件，並建立以雜湊定址的對應原始碼封存。

該封存和 `SOURCE-RECEIPT.json` 必須上傳至每個含有該 libmpv DLL 的公開二進位檔旁。可變的儲存庫分支、GitHub Actions 執行 URL，或只包含 mpv 本身的原始碼封存均不足夠。

## 更新 libmpv

未來的 libmpv 更新只可作為一個不可分割的變更接受，其中包括：

- 新的二進位檔、匯入程式庫、標頭和雜湊；
- 由同一建置產生的精確原始碼封存與收據；
- 更新後的宣告和 `third_party/versions.json`；
- 匯出符號和可替換性檢查；
- 預覽、seek、疊加時鐘、DPI 及 2K/4K/8K 效能回歸；以及
- 明確的復原成品。

任何執行階段元件均不得下載或默默替換 libmpv。

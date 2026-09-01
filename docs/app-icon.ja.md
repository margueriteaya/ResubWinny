# ResubWinny アプリアイコン

[简体中文](app-icon.md) · [繁體中文](app-icon.zh-TW.md) · [日本語](app-icon.ja.md) · [English](app-icon.en.md)

> **規範に関する注記：** 簡体字中国語版が唯一の正式な情報源です。他言語版は同期された翻訳であり、文言に曖昧さや矛盾がある場合は、簡体字中国語版が優先されます。

Apple プラットフォーム向けの正規ソースは、Icon Composer 用のレイヤー化されたアートワークです。プラットフォームマスク、Liquid Glass の模倣、ベベル、グロー、ぼかしたエッジ、スペキュラハイライト、レイヤー間のシャドウを含めてはなりません。

## ソースレイヤー

すべてのソースアートワークは 1024 x 1024 の正方形で、`studio-tauri/src-tauri/icons/source/apple/` にあります。

- `01-background.svg`：全面を覆う不透明な背景。
- `02-broadcast-plane.svg`：16:9 の放送プレーンの前景レイヤー。
- `03-captions.svg`：字幕領域、ルビセル、および独立した側部領域。
- `04-composite-preview.svg`：マスクなしのプレビュー専用。レイヤー化された Icon Composer のソースとしてインポートしないでください。

最初の 3 レイヤーをこの順序で Icon Composer にインポートします。2 つの前景 SVG は完全に不透明なままにし、半透明、屈折、スペキュラハイライト、シャドウは Icon Composer で調整します。Default、Dark、Clear、Tinted の各アピアランスに対するプラットフォームマスクと動的効果はシステムが提供します。

フラットな Tauri フォールバックは `icons/source/flat-app-icon.svg` です。角丸の背景は、Apple のシステムマスクを適用しないプラットフォームのために意図されたものです。これは Icon Composer のソースレイヤーではありません。

## ジオメトリ

- キャンバス：1024 x 1024。
- 放送プレーン：620 x 348.75。正確な 16:9 で、キャンバスの中央に配置。
- 主要コンテンツは watchOS の円形クロップ内に収める。
- 前景のエッジはソリッドで、フェザリングしない。
- テキスト、プラットフォームのハードウェア、スクリーンショット、複製されたアプリケーション UI は含めない。

## 再生成

`studio-tauri/` から、クロスプラットフォームの Tauri フォールバックアイコンを再生成します。

```powershell
npm run tauri -- icon src-tauri/icons/source/flat-app-icon.svg
```

## 参考資料

- Apple Human Interface Guidelines：App icons
  <https://developer.apple.com/design/human-interface-guidelines/app-icons/>
- Apple Icon Composer
  <https://developer.apple.com/icon-composer/>
- Apple Design Resources license
  <https://developer.apple.com/apple-design-resources-license/>

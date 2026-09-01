[简体中文](preview.md) · [繁體中文](preview.zh-TW.md) · [日本語](preview.ja.md) · [English](preview.en.md)

> 本ページは翻訳です。簡体字中国語版のみを正式な情報源とします。

# プレビュー契約

ネイティブプレビューは Tauri バックエンドとプロセス内 libmpv が担当します。
WebView はコマンドを送信し、上限のある状態を表示するだけで、字幕ビットマップの
送信や字幕レイアウトは行いません。

`render_at` と `sync_preview_overlay` は明示的なプロジェクト時間マッピングを使用し、
メディア時間とプロジェクト時間の両方を返します。バックエンドは archive から字幕平面を
合成し、選択した overlay ルートと capability metadata を報告します。未対応の B62 機能は
CSS で近似せず、宣言的なまま保持します。詳細な render profile とルート制限は
[`backend-contract.md`](../backend-contract.md) を参照してください。

`source_layout` を持つ ARIB-TTML では、backend が source display-plane の比率から有界の中間 caption texture を生成し、
texture 全体を libmpv が報告する実際の video-content viewport へ mapping します。letterbox/pillarbox、window size、DPI、
fullscreen は最終 transform だけを変更し、video content に対する字幕の位置と面積は変えません。`source_layout` の無い
旧 archive は logical 1920×1080 compatibility path で引き続き再生します。

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
